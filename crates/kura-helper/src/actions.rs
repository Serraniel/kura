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
use kura_core::plan::{Plan, PlanKind};

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

/// Runs one action on the real system.
pub fn run(action: &Action) -> Result<(), String> {
    match action {
        Action::Enable { unit } => run_program("systemctl", &["enable", "--now", unit]),
        Action::SnapperRootConfig { keep_mount } => snapper_config("root", Path::new("/"), *keep_mount),
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
        run(action).map_err(ApplyError::Failed)?;
    }
    Ok(())
}
