// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What the background check found. Written by kura-helper, read by the tray
//! agent and the window.

use serde::{Deserialize, Serialize};

use crate::package::PackageSet;
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
    /// Open reports about older versions than the installed ones. Listed in
    /// System health, no warning. See [`crate::security::Scan`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub older_reports: Vec<Finding>,
    /// AUR updates past their cooling period.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub aur_updates: usize,
    /// AUR updates still in their cooling period.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub aur_waiting: usize,
    /// AUR packages that need a library which is gone and must be built again.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub aur_rebuild: usize,
    /// When kura last installed security updates on its own (Unix time).
    #[serde(default, skip_serializing_if = "is_zero_time")]
    pub auto_installed: i64,
    /// How many known vulnerabilities that update fixed.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub auto_fixed: usize,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes a reference
fn is_zero(n: &usize) -> bool {
    *n == 0
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes a reference
fn is_zero_time(t: &i64) -> bool {
    *t == 0
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
    /// Brings the status up to date after packages changed, without asking
    /// any server. A finding goes away when its version is no longer
    /// installed. The next check finds it again if the new version has the
    /// same problem.
    pub fn after_change(&mut self, installed: &PackageSet, updates: usize) {
        let current = |f: &Finding| {
            installed
                .get(&f.package)
                .is_some_and(|p| p.version == f.installed)
        };
        self.security.retain(current);
        self.older_reports.retain(current);
        self.updates = updates;
        if updates == 0 {
            self.downloaded = true;
            self.blocked = false;
            self.news.clear();
        }
    }

    pub fn state(&self) -> State {
        if self.checked == 0 {
            State::Unknown
        } else if self.security.iter().any(Finding::urgent) {
            State::Security
        } else if self.blocked || !self.news.is_empty() || self.aur_rebuild > 0 {
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
    fn a_finished_update_clears_what_it_solved() {
        use crate::package::{InstallReason, InstalledPackage};
        use crate::security::Severity;

        let finding = |package: &str, installed: &str| Finding {
            advisory: "AVG-1".into(),
            package: package.into(),
            installed: installed.into(),
            fixed: None,
            severity: Severity::High,
            kind: "kind".into(),
            cves: Vec::new(),
            apps: Vec::new(),
        };
        let mut status = Status {
            checked: 1,
            updates: 8,
            blocked: true,
            news: vec!["news".into()],
            security: vec![
                finding("same", "1-1"),
                finding("newer", "1-1"),
                finding("gone", "1-1"),
            ],
            older_reports: vec![finding("gone", "1-1")],
            ..Status::default()
        };
        let installed = PackageSet::new([
            InstalledPackage::new("same", "1-1", InstallReason::Explicit),
            InstalledPackage::new("newer", "2-1", InstallReason::Explicit),
        ]);

        status.after_change(&installed, 0);

        assert_eq!(status.updates, 0);
        assert!(status.downloaded);
        assert!(!status.blocked);
        assert_eq!(status.news, [] as [String; 0]);
        assert_eq!(status.security, [finding("same", "1-1")]);
        assert_eq!(status.older_reports, []);
        assert_eq!(status.state(), State::Security);
    }

    #[test]
    fn updates_still_waiting_keep_their_news() {
        let mut status = Status {
            checked: 1,
            updates: 8,
            news: vec!["news".into()],
            ..Status::default()
        };
        status.after_change(&PackageSet::new([]), 2);
        assert_eq!(status.updates, 2);
        assert_eq!(status.news, ["news"]);
    }

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
        s.aur_rebuild = 1;
        assert_eq!(s.state(), State::NeedsAttention);
        s.aur_rebuild = 0;
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
        // Low risk and no fix: no warning.
        s.security[0].fixed = None;
        s.security[0].severity = crate::security::Severity::Medium;
        assert_eq!(s.state(), State::NeedsAttention);
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
