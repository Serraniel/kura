// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Runs actions: fixed programs with fixed arguments, nothing from data files.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::Sender;

use kura_alpm::Config;
use kura_alpm::apply::{ApplyError, Event};
use kura_core::action::Action;
use kura_core::plan::{Plan, PlanKind, Step};

fn run_program(program: &str, args: &[&str]) -> Result<(), String> {
    let out = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("{program}: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{program} {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

const PACMAN_CONF: &str = "/etc/pacman.conf";

/// Makes the boot loader start `kernel` by default.
fn set_default_kernel(kernel: &str) -> Result<(), String> {
    let grub_cfg = fs::read_to_string("/boot/grub/grub.cfg").unwrap_or_default();
    if let Some(id) = kura_core::kernels::grub_entry_id(&grub_cfg, kernel) {
        return run_program("grub-set-default", &[&id]);
    }
    let entries: Vec<(String, String)> = ["/boot", "/efi", "/boot/efi"]
        .iter()
        .flat_map(|esp| {
            fs::read_dir(Path::new(esp).join("loader/entries"))
                .into_iter()
                .flatten()
                .flatten()
        })
        .filter_map(|e| {
            Some((
                e.file_name().to_string_lossy().into_owned(),
                fs::read_to_string(e.path()).ok()?,
            ))
        })
        .collect();
    let entry = kura_core::kernels::sdboot_entry(&entries, kernel)
        .ok_or_else(|| format!("no boot entry for {kernel}"))?;
    run_program("bootctl", &["set-default", &format!("{entry}.conf")])
}

/// Changes `pacman.conf` after keeping a copy of the old file next to it.
fn edit_pacman_conf(change: impl FnOnce(&str) -> String) -> Result<(), String> {
    let path = Path::new(PACMAN_CONF);
    let old = fs::read_to_string(path).map_err(|e| format!("{PACMAN_CONF}: {e}"))?;
    let new = change(&old);
    if new == old {
        return Ok(());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    fs::copy(path, format!("{PACMAN_CONF}.kura-{stamp}")).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("conf.kura-new");
    fs::write(&tmp, new).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// Runs one action on the real system.
pub fn run(action: &Action, conf: &Config, events: &Sender<Event>) -> Result<(), String> {
    match action {
        Action::Enable { unit } => run_program("systemctl", &["enable", "--now", unit]),
        Action::SnapperRootConfig { keep_mount } => snapper_config("root", Path::new("/"), *keep_mount),
        Action::TrustKey { key, keyserver } => {
            run_program("pacman-key", &["--recv-keys", key, "--keyserver", keyserver])?;
            run_program("pacman-key", &["--lsign-key", key])
        }
        Action::ForgetKey { key } => run_program("pacman-key", &["--delete", key]),
        Action::InstallFrom { urls } => {
            kura_alpm::apply::install_urls(conf, urls, events).map_err(|e| e.to_string())
        }
        Action::AddRepo {
            name,
            include,
            before,
        } => edit_pacman_conf(|c| kura_core::pacman_conf::add_repo(c, name, include, before.as_deref())),
        Action::RemoveRepo { name } => edit_pacman_conf(|c| kura_core::pacman_conf::remove_repo(c, name)),
        Action::AddRepoServer { name, server, before } => edit_pacman_conf(|c| {
            kura_core::pacman_conf::add_repo_line(c, name, &format!("Server = {server}"), before.as_deref())
        }),
        Action::InstallFromRepo { repo, packages } => {
            kura_alpm::apply::install_from_repo(repo, packages, events).map_err(|e| e.to_string())
        }
        Action::RegenerateGrub => run_program("grub-mkconfig", &["-o", "/boot/grub/grub.cfg"]),
        Action::SetDefaultKernel { kernel } => set_default_kernel(kernel),
        Action::AddUserToGroup { user, group } => run_program("gpasswd", &["-a", user, group]),
        Action::SetArchitecture { value } => {
            edit_pacman_conf(|c| kura_core::pacman_conf::set_option(c, "Architecture", value))
        }
    }
}

/// `snapper create-config` for `path`.
///
/// snapper always creates its own `.snapshots` subvolume. When `.snapshots`
/// is already a mount (a separate subvolume like `@snapshots`), the mount is
/// taken away for a moment, snapper's subvolume is replaced by the empty
/// mount point again and the same mount comes back. The mount is restored
/// even when snapper fails.
pub fn snapper_config(name: &str, path: &Path, keep_mount: bool) -> Result<(), String> {
    let path_str = path.to_str().ok_or("path is not UTF-8")?;
    let create = || run_program("snapper", &["--no-dbus", "-c", name, "create-config", path_str]);
    if !keep_mount {
        return create();
    }
    let dir = path.join(".snapshots");
    let dir_str = dir.to_str().ok_or("path is not UTF-8")?;
    let mounts = fs::read_to_string("/proc/mounts").map_err(|e| e.to_string())?;
    let mount = kura_core::snapshots::mount_entry(&mounts, dir_str)
        .ok_or_else(|| format!("{dir_str} is not mounted"))?;

    run_program("umount", &[dir_str])?;
    let result = (|| {
        // Only an empty mount point can be removed; anything else stops here.
        fs::remove_dir(&dir).map_err(|e| format!("{dir_str}: {e}"))?;
        create()?;
        run_program("btrfs", &["subvolume", "delete", dir_str])?;
        Ok::<(), String>(())
    })();
    if !dir.exists() {
        fs::create_dir(&dir).map_err(|e| format!("{dir_str}: {e}"))?;
    }
    run_program(
        "mount",
        &["-t", &mount.fs, "-o", &mount.options, &mount.device, dir_str],
    )?;
    result?;
    // snapper's own subvolume has these permissions; the mount takes them over.
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o750)).map_err(|e| e.to_string())
}

/// Sets up snapshots. Like any plan, the helper first derives the plan
/// again from the system and stops if it differs from the approved one.
/// Only actions derived here ever run.
pub fn apply_snapshots(conf: &Config, approved: &Plan, events: &Sender<Event>) -> Result<(), ApplyError> {
    let fresh = kura_alpm::snapshots::prepare(conf, None, false)?;
    let Some(fresh) = fresh else {
        return Ok(());
    };
    let fresh = fresh.plan.map_err(ApplyError::Blocked)?;
    if fresh.actions != approved.actions || fresh.steps.is_empty() != approved.steps.is_empty() {
        return Err(ApplyError::Changed(Box::new(fresh)));
    }
    if !approved.steps.is_empty() {
        let packages = Plan {
            kind: PlanKind::Install,
            steps: approved.steps.clone(),
            actions: Vec::new(),
            builds: Vec::new(),
        };
        kura_alpm::apply::apply(conf, &packages, events).map_err(|e| match e {
            ApplyError::Changed(mut plan) => {
                plan.kind = PlanKind::Snapshots;
                plan.actions.clone_from(&fresh.actions);
                ApplyError::Changed(plan)
            }
            other => other,
        })?;
    }
    for action in &fresh.actions {
        let _ = events.send(Event::Configuring {
            action: action.clone(),
        });
        run(action, conf, events).map_err(ApplyError::Failed)?;
    }
    Ok(())
}

/// Adds or takes away a repository profile. Only the exact actions kura
/// derives for one of its built-in profiles run; package steps may only
/// remove that profile's keyring and mirror list packages.
pub fn apply_profile(conf: &Config, approved: &Plan, events: &Sender<Event>) -> Result<(), ApplyError> {
    let (profile, _, add) = kura_core::profiles::known(&approved.actions)
        .ok_or_else(|| ApplyError::Failed("unknown repository profile change".into()))?;
    // Adding changes no packages here; taking away may only remove the
    // profile's keyring and mirror list packages.
    let removals_ok = if add {
        approved.steps.is_empty()
    } else {
        approved.steps.iter().all(
            |s| matches!(s, Step::Remove { name, .. } if profile.bootstrap_packages.contains(&name.as_str())),
        )
    };
    if !removals_ok {
        return Err(ApplyError::Failed(
            "unexpected package change in a profile plan".into(),
        ));
    }
    let run_all = |actions: &[Action]| -> Result<(), ApplyError> {
        for action in actions {
            let _ = events.send(Event::Configuring {
                action: action.clone(),
            });
            run(action, conf, events).map_err(ApplyError::Failed)?;
        }
        Ok(())
    };
    if add {
        return run_all(&approved.actions);
    }
    // Taking away: repositories and settings first, then the keyring and
    // mirror list packages, the key last.
    let (before, after) = approved.actions.split_at(approved.actions.len() - 1);
    run_all(before)?;
    if !approved.steps.is_empty() {
        let packages = Plan {
            kind: PlanKind::Remove,
            steps: approved.steps.clone(),
            actions: Vec::new(),
            builds: Vec::new(),
        };
        kura_alpm::apply::apply(conf, &packages, events)?;
    }
    run_all(after)
}

/// Installs or removes a kernel. The boot menu work must be what the helper
/// derives itself; the running kernel and the last kernel are never removed.
/// After an install, DKMS modules that were not built are reported.
pub fn apply_kernel(conf: &Config, approved: &Plan, events: &Sender<Event>) -> Result<(), ApplyError> {
    let kernel = approved
        .steps
        .iter()
        .find_map(|s| match s {
            Step::Install { name, .. } | Step::Remove { name, .. } if kura_core::health::is_kernel(name) => {
                Some(name.clone())
            }
            _ => None,
        })
        .ok_or_else(|| ApplyError::Failed("no kernel in a kernel plan".into()))?;
    let adding = approved.steps.iter().any(|s| matches!(s, Step::Install { .. }));
    if approved.actions != kura_alpm::kernels::boot_actions(conf, &kernel, adding) {
        return Err(ApplyError::Failed(
            "unexpected boot menu change in a kernel plan".into(),
        ));
    }
    let previous = kura_alpm::kernels::running_package();
    let added: Vec<&str> = approved
        .steps
        .iter()
        .filter_map(|s| match s {
            Step::Install { name, .. } if kura_core::health::is_kernel(name) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let removed: Vec<&str> = approved
        .steps
        .iter()
        .filter_map(|s| match s {
            Step::Remove { name, .. } if kura_core::health::is_kernel(name) => Some(name.as_str()),
            _ => None,
        })
        .collect();
    if !removed.is_empty() {
        let installed: Vec<String> = kura_alpm::installed(conf)?
            .iter()
            .map(|p| p.name.clone())
            .collect();
        let running = kura_alpm::kernels::running_package();
        for kernel in &removed {
            if kura_core::kernels::can_remove(kernel, &installed, running.as_deref()).is_err() {
                return Err(ApplyError::Failed(format!("{kernel} must stay")));
            }
        }
    }
    let packages = Plan {
        kind: if added.is_empty() {
            PlanKind::Remove
        } else {
            PlanKind::Install
        },
        steps: approved.steps.clone(),
        actions: Vec::new(),
        builds: Vec::new(),
    };
    kura_alpm::apply::apply(conf, &packages, events).map_err(|e| match e {
        ApplyError::Changed(mut plan) => {
            plan.kind = PlanKind::Kernel;
            plan.actions.clone_from(&approved.actions);
            ApplyError::Changed(plan)
        }
        other => other,
    })?;
    for action in &approved.actions {
        let _ = events.send(Event::Configuring {
            action: action.clone(),
        });
        run(action, conf, events).map_err(ApplyError::Failed)?;
    }
    if adding {
        // Remember it, to offer removing the old kernel once the new one started.
        let record = serde_json::json!({"added": kernel, "previous": previous});
        let _ = fs::write(kura_alpm::kernels::ADDED, record.to_string());
    } else if kura_alpm::kernels::replaced_kernel().is_some_and(|(_, old)| old == kernel) {
        let _ = fs::remove_file(kura_alpm::kernels::ADDED);
    }
    for kernel in added {
        let missing = kura_alpm::kernels::missing_dkms(kernel);
        if !missing.is_empty() {
            return Err(ApplyError::Failed(format!(
                "{kernel}: DKMS did not build {}; start the computer with your old kernel",
                missing.join(", ")
            )));
        }
    }
    Ok(())
}

/// Installs a bundle for `caller`. The bundle is found by what the plan
/// installs and does; actions run only when they are exactly what the
/// shipped bundle file gives, and user-scope actions only for the caller.
pub fn apply_bundle(
    conf: &Config,
    approved: &Plan,
    events: &Sender<Event>,
    caller: Option<&str>,
) -> Result<(), ApplyError> {
    let caller = caller.ok_or_else(|| ApplyError::Failed("unknown caller".into()))?;
    let explicit: Vec<String> = approved
        .steps
        .iter()
        .filter_map(|s| match s {
            Step::Install {
                name,
                reason: kura_core::package::InstallReason::Explicit,
                ..
            } => Some(name.clone()),
            _ => None,
        })
        .collect();
    let active: Vec<String> = conf.repos.iter().map(|r| r.name.clone()).collect();
    let bundle = kura_alpm::bundles::load(Path::new(kura_alpm::bundles::DIR))
        .into_iter()
        .find(|b| {
            let (before, after) = kura_core::bundles::actions(b, &active, caller);
            let expected: Vec<Action> = before.into_iter().chain(after).collect();
            let allowed = kura_alpm::bundles::packages(
                b,
                &b.packages
                    .optional
                    .iter()
                    .map(|o| o.name.clone())
                    .collect::<Vec<_>>(),
            );
            expected == approved.actions && explicit.iter().all(|n| allowed.contains(n))
        })
        .ok_or_else(|| ApplyError::Failed("not a bundle kura knows".into()))?;
    let (before, after) = kura_core::bundles::actions(&bundle, &active, caller);
    let run_all = |actions: &[Action], conf: &Config| -> Result<(), ApplyError> {
        for action in actions {
            let _ = events.send(Event::Configuring {
                action: action.clone(),
            });
            run(action, conf, events).map_err(ApplyError::Failed)?;
        }
        Ok(())
    };
    run_all(&before, conf)?;
    // Repositories may have changed: read the configuration again.
    let conf = kura_alpm::load_config()?;
    let packages = Plan {
        kind: PlanKind::Install,
        steps: approved.steps.clone(),
        actions: Vec::new(),
        builds: Vec::new(),
    };
    kura_alpm::apply::apply(&conf, &packages, events).map_err(|e| match e {
        ApplyError::Changed(mut plan) => {
            plan.kind = PlanKind::Bundle;
            plan.actions.clone_from(&approved.actions);
            ApplyError::Changed(plan)
        }
        other => other,
    })?;
    run_all(&after, &conf)
}
