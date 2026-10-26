// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Changes to the system that are not package changes. Each action is a
//! fixed, declarative operation; `kura-helper` only runs actions it derived
//! itself from the system's state, never ones it received.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum Action {
    /// Enable and start a systemd unit.
    Enable {
        unit: String,
    },
    /// Create the snapper configuration `root` for `/`. With an existing
    /// `/.snapshots` mount the mount is kept instead of snapper's own subvolume.
    SnapperRootConfig {
        keep_mount: bool,
    },
    /// Trust a repository's signing key in pacman's keyring.
    TrustKey {
        key: String,
        keyserver: String,
    },
    /// Stop trusting a key kura added.
    ForgetKey {
        key: String,
    },
    /// Install package files from these addresses (a repository's keyring
    /// and mirror list, before the repository itself exists).
    InstallFrom {
        urls: Vec<String>,
    },
    /// Add a repository section to `pacman.conf`, before `before` or at the end.
    AddRepo {
        name: String,
        include: String,
        before: Option<String>,
    },
    RemoveRepo {
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialises_as_tagged_objects() {
        let a = Action::Enable {
            unit: "snapper-cleanup.timer".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, r#"{"action":"enable","unit":"snapper-cleanup.timer"}"#);
        assert_eq!(serde_json::from_str::<Action>(&json).unwrap(), a);
        let c = Action::SnapperRootConfig { keep_mount: true };
        assert_eq!(
            serde_json::to_string(&c).unwrap(),
            r#"{"action":"snapper-root-config","keep_mount":true}"#
        );
    }
}
