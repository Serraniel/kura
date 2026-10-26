// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the background check found. Written by kura-helper, read by the tray
//! agent and the window.

use serde::{Deserialize, Serialize};

use crate::security::Finding;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Status {
    /// Unix time of the check.
    pub checked: i64,
    /// Number of packages with a newer version.
    pub updates: usize,
    /// Whether all packages of the update are already in the cache.
    pub downloaded: bool,
    /// The update cannot be prepared; the window explains why.
    #[serde(default)]
    pub blocked: bool,
    /// Titles of Arch news items that concern the update.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub news: Vec<String>,
    /// Known vulnerabilities in installed packages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<Finding>,
    /// AUR updates past their cooling period.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub aur_updates: usize,
    /// AUR updates still in their cooling period.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub aur_waiting: usize,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes a reference
fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// What the tray and the overview show, most important first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// An installed app or system part has a known vulnerability.
    Security,
    /// The update needs attention: news to read or a problem to solve.
    NeedsAttention,
    UpdatesReady,
    UpToDate,
    /// No check has run yet.
    Unknown,
}

impl Status {
    pub fn state(&self) -> State {
        if self.checked == 0 {
            State::Unknown
        } else if !self.security.is_empty() {
            State::Security
        } else if self.blocked || !self.news.is_empty() {
            State::NeedsAttention
        } else if self.updates > 0 || self.aur_updates > 0 {
            State::UpdatesReady
        } else {
            State::UpToDate
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_follows_priority() {
        assert_eq!(Status::default().state(), State::Unknown);
        let mut s = Status {
            checked: 1,
            ..Status::default()
        };
        assert_eq!(s.state(), State::UpToDate);
        s.aur_waiting = 2;
        assert_eq!(s.state(), State::UpToDate);
        s.aur_updates = 1;
        assert_eq!(s.state(), State::UpdatesReady);
        s.aur_updates = 0;
        s.updates = 3;
        assert_eq!(s.state(), State::UpdatesReady);
        s.news.push("linux needs a manual step".into());
        assert_eq!(s.state(), State::NeedsAttention);
        s.news.clear();
        s.blocked = true;
        assert_eq!(s.state(), State::NeedsAttention);
        s.security.push(crate::security::Finding {
            advisory: "AVG-1".into(),
            package: "ffmpeg".into(),
            installed: "1".into(),
            fixed: Some("2".into()),
            severity: crate::security::Severity::High,
            kind: "x".into(),
            cves: vec![],
            apps: vec!["kdenlive".into()],
        });
        assert_eq!(s.state(), State::Security);
    }

    #[test]
    fn reads_older_files_without_new_fields() {
        let s: Status = serde_json::from_str(r#"{"checked":5,"updates":2,"downloaded":true}"#).unwrap();
        assert_eq!(s.state(), State::UpdatesReady);
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            r#"{"checked":5,"updates":2,"downloaded":true,"blocked":false}"#
        );
    }
}
