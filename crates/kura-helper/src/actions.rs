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
    let profile = kura_core::profiles::ALL
        .iter()
        .find(|p| {
            kura_core::profiles::add_actions(p) == approved.actions
                || kura_core::profiles::remove_actions(p) == approved.actions
        })
        .ok_or_else(|| ApplyError::Failed("unknown repository profile change".into()))?;
    let removals_ok = approved.steps.iter().all(
        |s| matches!(s, Step::Remove { name, .. } if profile.bootstrap_packages.contains(&name.as_str())),
    );
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
    // Taking away: the repository goes first, then its packages, then the key.
    let (before, after) = approved.actions.split_at(if approved.steps.is_empty() {
        approved.actions.len()
    } else {
        profile.repos.len()
    });
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
