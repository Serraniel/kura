// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Trial runs: before an update changes the system, it runs in a throwaway
//! btrfs snapshot of `/` inside a container. Only a transaction that fails
//! there stops the update; when the trial itself cannot be set up, the
//! update goes ahead as before.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use std::sync::mpsc::Sender;

use kura_alpm::apply::Event;
use kura_core::plan::Plan;

const SNAPSHOT: &str = "/var/lib/kura/trial";
const HELPER: &str = "/usr/lib/kura/kura-helper";
/// Exit code of `kura-helper trial` when the transaction itself failed.
pub const FAILED: i32 = 3;

/// What happened.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Passed,
    /// The update failed in the snapshot, with the reason.
    Failed(String),
    /// The trial could not be set up or did not finish.
    Skipped(String),
}

/// Trial runs need a snapshot of `/`, which only btrfs can make.
pub fn possible() -> bool {
    kura_core::hardware::root_fs(&std::fs::read_to_string("/proc/mounts").unwrap_or_default()).as_deref()
        == Some("btrfs")
}

fn quiet(events: &Sender<Event>, program: &str, args: &[&str]) -> Result<(), String> {
    let out = crate::console::run(Command::new(program).args(args), "", events)
        .map_err(|e| format!("{program}: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{program}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// Runs `plan` in a snapshot of `/` and throws the snapshot away.
pub fn run(plan: &Plan, events: &Sender<Event>) -> Outcome {
    if Path::new(SNAPSHOT).exists() {
        let _ = quiet(events, "btrfs", &["subvolume", "delete", SNAPSHOT]);
    }
    if let Err(e) = quiet(events, "btrfs", &["subvolume", "snapshot", "/", SNAPSHOT]) {
        return Outcome::Skipped(e);
    }
    let result = in_container(plan, events);
    if let Err(e) = quiet(events, "btrfs", &["subvolume", "delete", SNAPSHOT]) {
        eprintln!("kura: trial snapshot not removed: {e}");
    }
    result
}

fn in_container(plan: &Plan, events: &Sender<Event>) -> Outcome {
    let Ok(json) = serde_json::to_string(plan) else {
        return Outcome::Skipped("plan".into());
    };
    let mut cmd = Command::new("systemd-nspawn");
    cmd.args([
        "--quiet",
        "--register=no",
        // The snapshot has the machine id of the host, so its journal stays out.
        "--link-journal=no",
        // Package scripts run as they do on the real system. Without this,
        // a future systemd limits them, and prints a note about it until then.
        "--restrict-address-families=",
        "--as-pid2",
        "--pipe",
        &format!("--directory={SNAPSHOT}"),
        // Packages downloaded for the trial are reused by the real update.
        "--bind=/var/cache/pacman/pkg",
        "--",
        HELPER,
        "trial",
    ]);
    let _ = events.send(Event::Output {
        line: format!("$ {}", crate::console::command_line(&cmd)),
    });
    let child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return Outcome::Skipped(format!("systemd-nspawn: {e}")),
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(json.as_bytes());
    }
    // The update inside prints its output on stderr; the last line is the reason.
    let stderr = child
        .stderr
        .take()
        .map(|s| crate::console::forward(s, "[trial] ", events));
    let Ok(status) = child.wait() else {
        return Outcome::Skipped("systemd-nspawn did not finish".into());
    };
    let reason = String::from_utf8_lossy(stderr.as_deref().unwrap_or_default()).into_owned();
    let last = reason
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or_default()
        .to_owned();
    match status.code() {
        Some(0) => Outcome::Passed,
        Some(FAILED) => Outcome::Failed(last),
        _ => Outcome::Skipped(last),
    }
}

/// Inside the container: applies the plan from stdin. Exits with
/// [`FAILED`] when the transaction fails.
pub fn inside() -> i32 {
    let mut json = String::new();
    if std::io::Read::read_to_string(&mut std::io::stdin(), &mut json).is_err() {
        return 1;
    }
    let Ok(plan) = serde_json::from_str::<Plan>(&json) else {
        return 1;
    };
    let Ok(conf) = kura_alpm::load_config() else {
        return 1;
    };
    // Output lines go to stderr, where the helper outside shows them.
    let (events, received) = std::sync::mpsc::channel();
    let printer = std::thread::spawn(move || {
        let l = kura_i18n::Localizer::from_env();
        let mut seen = std::collections::BTreeSet::new();
        for event in received {
            let line = crate::text::event(&l, &event);
            if let Event::Output { .. } = event {
                eprintln!("{line}");
            } else if seen.insert(line.clone()) {
                eprintln!("# {line}");
            }
        }
    });
    let result = kura_alpm::apply::apply(&conf, &plan, &events);
    drop(events);
    let _ = printer.join();
    match result {
        Ok(()) => 0,
        // A changed plan is no failure of the update itself. The reason goes
        // last, because the helper outside reports the last line.
        Err(e @ (kura_alpm::apply::ApplyError::Changed(_) | kura_alpm::apply::ApplyError::Locked)) => {
            eprintln!("{e}");
            1
        }
        Err(e) => {
            eprintln!("{e}");
            FAILED
        }
    }
}
