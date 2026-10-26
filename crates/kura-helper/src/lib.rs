// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The D-Bus protocol between kura's user programs and `kura-helper`, and the
//! client the CLI and GUI use.
//!
//! The helper accepts exactly one kind of request: an approved plan. Before
//! running it, it asks polkit whether the caller may do this kind of change.

pub mod actions;
pub mod aur;
pub mod report;
pub mod text;
pub mod trial;

use std::sync::mpsc;
use std::time::Duration;

use kura_alpm::apply::Event;
use kura_alpm::prepare::Problem;
use kura_core::plan::{Plan, PlanKind};
use serde::{Deserialize, Serialize};

pub const BUS_NAME: &str = "org.kura.Helper1";
pub const OBJECT_PATH: &str = "/org/kura/Helper1";

/// What happened to a plan sent to the helper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "kebab-case")]
pub enum Reply {
    /// All changes were made.
    Done,
    /// The system moved on since the preview; this is the new plan to review.
    Changed {
        plan: Plan,
    },
    /// The change cannot be prepared any more.
    Blocked {
        problems: Vec<Problem>,
    },
    /// pacman or another program holds the database lock.
    Locked,
    /// polkit did not authorise the caller.
    NotAuthorized,
    /// The helper is already running another plan.
    Busy,
    Failed {
        message: String,
    },
    /// The update failed in a throwaway snapshot; nothing was changed.
    TrialFailed {
        reason: String,
    },
    /// New download servers are in use; the old list was kept as `backup`.
    Mirrors {
        count: usize,
        backup: String,
    },
}

/// The polkit action that guards a kind of plan.
pub fn action_for(kind: PlanKind) -> &'static str {
    match kind {
        PlanKind::SystemUpdate | PlanKind::Repair | PlanKind::Switch { .. } => "org.kura.update",
        PlanKind::Install | PlanKind::Bundle => "org.kura.install",
        PlanKind::Remove => "org.kura.remove",
        PlanKind::Restore { .. } => "org.kura.restore",
        PlanKind::Snapshots => "org.kura.snapshots",
        PlanKind::AurInstall => "org.kura.aur",
        PlanKind::Profile => "org.kura.profiles",
        PlanKind::Kernel => "org.kura.kernels",
    }
}

/// What the helper writes to the cause file before a transaction.
pub fn cause_for(kind: PlanKind) -> String {
    let kind = match kind {
        PlanKind::SystemUpdate => "system-update",
        PlanKind::Install => "install",
        PlanKind::Remove => "remove",
        PlanKind::Restore { .. } => "restore",
        PlanKind::Repair => "repair",
        PlanKind::Snapshots => "snapshots",
        PlanKind::AurInstall => "aur-install",
        PlanKind::Profile => "profile",
        PlanKind::Kernel => "kernel",
        PlanKind::Bundle => "bundle",
        PlanKind::Switch { .. } => "switch",
    };
    format!("kura:{kind}")
}

#[zbus::proxy(
    interface = "org.kura.Helper1",
    default_service = "org.kura.Helper1",
    default_path = "/org/kura/Helper1"
)]
pub trait Helper {
    /// Runs an approved plan (JSON) and returns a [`Reply`] (JSON).
    fn apply(&self, plan: &str) -> zbus::Result<String>;

    /// Chooses fast, up-to-date download servers and returns a [`Reply`] (JSON).
    fn rank_mirrors(&self) -> zbus::Result<String>;

    /// Saves the system-wide settings (JSON) and returns a [`Reply`] (JSON).
    fn set_settings(&self, settings: &str) -> zbus::Result<String>;

    /// Progress of the running plan, as a JSON [`Event`].
    #[zbus(signal)]
    fn progress(&self, event: String) -> zbus::Result<()>;
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("cannot reach kura-helper: {0}")]
    Bus(#[from] zbus::Error),
    #[error("unexpected answer from kura-helper: {0}")]
    Protocol(String),
}

/// Sends a plan to the helper and calls `on_event` for each progress event
/// until the helper answers.
pub fn apply(plan: &Plan, mut on_event: impl FnMut(Event)) -> Result<Reply, ClientError> {
    let conn = zbus::blocking::connection::Builder::system()?
        // Large updates take a while; the helper reports progress meanwhile.
        .method_timeout(Duration::from_secs(6 * 3600))
        .build()?;
    let proxy = HelperProxyBlocking::new(&conn)?;
    let signals = proxy.receive_progress()?;
    let json = serde_json::to_string(plan).map_err(|e| ClientError::Protocol(e.to_string()))?;

    let (tx, rx) = mpsc::channel::<Result<String, zbus::Error>>();
    let caller = proxy.clone();
    std::thread::spawn(move || {
        let _ = tx.send(caller.apply(&json));
    });
    let (events_tx, events_rx) = mpsc::channel::<Event>();
    std::thread::spawn(move || {
        for signal in signals {
            let Ok(args) = signal.args() else { continue };
            if let Ok(event) = serde_json::from_str::<Event>(args.event())
                && events_tx.send(event).is_err()
            {
                break;
            }
        }
    });

    loop {
        while let Ok(event) = events_rx.try_recv() {
            on_event(event);
        }
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(answer) => {
                while let Ok(event) = events_rx.try_recv() {
                    on_event(event);
                }
                let text = answer?;
                return serde_json::from_str(&text).map_err(|e| ClientError::Protocol(e.to_string()));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(ClientError::Protocol(
                    "the request ended without an answer".into(),
                ));
            }
        }
    }
}

/// Asks the helper to rank the download servers.
pub fn rank_mirrors() -> Result<Reply, ClientError> {
    let conn = zbus::blocking::connection::Builder::system()?
        .method_timeout(Duration::from_secs(600))
        .build()?;
    let text = HelperProxyBlocking::new(&conn)?.rank_mirrors()?;
    serde_json::from_str(&text).map_err(|e| ClientError::Protocol(e.to_string()))
}

/// Asks the helper to save the system-wide settings.
pub fn set_settings(settings: &kura_core::settings::Settings) -> Result<Reply, ClientError> {
    let conn = zbus::blocking::connection::Builder::system()?.build()?;
    let json = serde_json::to_string(settings).map_err(|e| ClientError::Protocol(e.to_string()))?;
    let text = HelperProxyBlocking::new(&conn)?.set_settings(&json)?;
    serde_json::from_str(&text).map_err(|e| ClientError::Protocol(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_and_causes() {
        assert_eq!(action_for(PlanKind::SystemUpdate), "org.kura.update");
        assert_eq!(action_for(PlanKind::Repair), "org.kura.update");
        assert_eq!(action_for(PlanKind::Install), "org.kura.install");
        assert_eq!(action_for(PlanKind::Remove), "org.kura.remove");
        assert_eq!(action_for(PlanKind::Snapshots), "org.kura.snapshots");
        assert_eq!(action_for(PlanKind::AurInstall), "org.kura.aur");
        assert_eq!(
            action_for(PlanKind::Restore { generation: 1 }),
            "org.kura.restore"
        );
        assert_eq!(cause_for(PlanKind::SystemUpdate), "kura:system-update");
        assert_eq!(cause_for(PlanKind::Restore { generation: 2 }), "kura:restore");
    }

    #[test]
    fn replies_serialise() {
        let r = Reply::Blocked {
            problems: vec![Problem::NotFound { name: "x".into() }],
        };
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(serde_json::from_str::<Reply>(&json).unwrap(), r);
        assert_eq!(
            serde_json::to_string(&Reply::Done).unwrap(),
            r#"{"result":"done"}"#
        );
    }
}
