// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Plans: every change kura makes, described as data before it runs.
//!
//! The preview shows a plan, and the privileged helper accepts nothing else.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::action::Action;
use crate::generation::Change;
use crate::package::InstallReason;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanKind {
    /// Upgrade everything that has an update.
    SystemUpdate,
    Install,
    Remove,
    /// Return to an earlier generation.
    Restore {
        generation: u64,
    },
    /// Steps chosen by the problem solver.
    Repair,
    /// Set up snapshots before updates: packages plus actions.
    Snapshots,
    /// Build packages from the AUR and install them.
    AurInstall,
    /// Add or take away a repository profile.
    Profile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RemovalReason {
    /// The user asked for it.
    Requested,
    /// Nothing needs the package any more.
    Orphan,
    /// Part of a repair strategy.
    Repair,
    /// Replaced by another package during an update (pacman's `replaces`).
    Replaced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case")]
pub enum Step {
    Install {
        name: String,
        version: String,
        reason: InstallReason,
    },
    Upgrade {
        name: String,
        from: String,
        to: String,
    },
    Downgrade {
        name: String,
        from: String,
        to: String,
    },
    Remove {
        name: String,
        version: String,
        why: RemovalReason,
    },
    MarkReason {
        name: String,
        reason: InstallReason,
    },
}

impl Step {
    pub fn package(&self) -> &str {
        match self {
            Step::Install { name, .. }
            | Step::Upgrade { name, .. }
            | Step::Downgrade { name, .. }
            | Step::Remove { name, .. }
            | Step::MarkReason { name, .. } => name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub kind: PlanKind,
    pub steps: Vec<Step>,
    /// Changes that are not package changes; they run after the packages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
    /// AUR package bases to build, in an order that respects `after`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub builds: Vec<AurBuild>,
}

/// One AUR package base, pinned to the commit the preview showed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AurBuild {
    pub base: String,
    pub commit: String,
    pub version: String,
    /// Packages of this base to install.
    pub packages: Vec<String>,
    /// Bases that are built and installed first.
    pub after: Vec<String>,
}

/// Facts about the system that a plan is checked against.
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// Every package that has a newer version in the sync databases.
    pub available_upgrades: BTreeSet<String>,
    /// Packages a repair strategy holds back on purpose.
    pub held: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlanError {
    #[error("the plan is empty")]
    Empty,
    #[error("package {0} appears more than once")]
    Duplicate(String),
    #[error("partial upgrade: {} would stay outdated", .0.join(", "))]
    PartialUpgrade(Vec<String>),
    #[error("{0} must never be held back")]
    CannotHold(String),
}

/// Packages that must never be held back, because the rest of the system
/// cannot run against an old version of them.
pub fn can_hold(name: &str) -> bool {
    const NEVER: &[&str] = &[
        "glibc",
        "systemd",
        "systemd-libs",
        "pacman",
        "filesystem",
        "gcc-libs",
    ];
    !(NEVER.contains(&name) || name.ends_with("-keyring") || name == "linux" || name.starts_with("linux-"))
}

impl Plan {
    pub fn new(kind: PlanKind) -> Self {
        Self {
            kind,
            steps: Vec::new(),
            actions: Vec::new(),
            builds: Vec::new(),
        }
    }

    /// Builds a plan that applies generation changes.
    pub fn from_changes(kind: PlanKind, changes: &[Change]) -> Self {
        let steps = changes
            .iter()
            .map(|c| match c.clone() {
                Change::Install {
                    name,
                    version,
                    reason,
                } => Step::Install {
                    name,
                    version,
                    reason,
                },
                Change::Remove { name, version } => Step::Remove {
                    name,
                    version,
                    why: RemovalReason::Requested,
                },
                Change::Upgrade { name, from, to } => Step::Upgrade { name, from, to },
                Change::Downgrade { name, from, to } => Step::Downgrade { name, from, to },
                Change::Reason { name, reason } => Step::MarkReason { name, reason },
            })
            .collect();
        Self {
            kind,
            steps,
            actions: Vec::new(),
            builds: Vec::new(),
        }
    }

    /// Adds removal of packages nothing needs any more.
    pub fn add_cleanup<'a>(&mut self, orphans: impl IntoIterator<Item = (&'a str, &'a str)>) {
        for (name, version) in orphans {
            self.steps.push(Step::Remove {
                name: name.to_owned(),
                version: version.to_owned(),
                why: RemovalReason::Orphan,
            });
        }
    }

    /// Checks the rules every plan must follow.
    pub fn validate(&self, ctx: &Context) -> Result<(), PlanError> {
        if self.steps.is_empty() && self.actions.is_empty() && self.builds.is_empty() {
            return Err(PlanError::Empty);
        }
        let mut seen = BTreeSet::new();
        for step in &self.steps {
            if !matches!(step, Step::MarkReason { .. }) && !seen.insert(step.package()) {
                return Err(PlanError::Duplicate(step.package().to_owned()));
            }
        }
        if let Some(name) = ctx.held.iter().find(|n| !can_hold(n)) {
            return Err(PlanError::CannotHold(name.clone()));
        }
        // Installing or updating against newer sync databases without
        // upgrading everything is a partial upgrade.
        if matches!(
            self.kind,
            PlanKind::SystemUpdate
                | PlanKind::Install
                | PlanKind::Repair
                | PlanKind::Snapshots
                | PlanKind::AurInstall
        ) {
            let upgraded: BTreeSet<&str> = self
                .steps
                .iter()
                .filter(|s| matches!(s, Step::Upgrade { .. }))
                .map(Step::package)
                .collect();
            let removed: BTreeSet<&str> = self
                .steps
                .iter()
                .filter(|s| matches!(s, Step::Remove { .. }))
                .map(Step::package)
                .collect();
            let missing: Vec<String> = ctx
                .available_upgrades
                .iter()
                .filter(|n| {
                    !upgraded.contains(n.as_str()) && !removed.contains(n.as_str()) && !ctx.held.contains(*n)
                })
                .cloned()
                .collect();
            if !missing.is_empty() {
                return Err(PlanError::PartialUpgrade(missing));
            }
        }
        Ok(())
    }

    /// Splits the steps into the groups the preview shows.
    pub fn groups(&self, catalog: &impl AppCatalog) -> Groups<'_> {
        let mut groups = Groups::default();
        for step in &self.steps {
            let bucket = match step {
                Step::Remove {
                    why: RemovalReason::Orphan,
                    ..
                } => &mut groups.cleanup,
                _ if catalog.is_app(step.package()) => &mut groups.apps,
                _ => &mut groups.system,
            };
            bucket.push(step);
        }
        groups
    }
}

/// Tells apps (things with a window the user starts) from other packages.
pub trait AppCatalog {
    fn is_app(&self, package: &str) -> bool;
}

impl<F: Fn(&str) -> bool> AppCatalog for F {
    fn is_app(&self, package: &str) -> bool {
        self(package)
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Groups<'a> {
    pub apps: Vec<&'a Step>,
    pub system: Vec<&'a Step>,
    pub cleanup: Vec<&'a Step>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up(name: &str) -> Step {
        Step::Upgrade {
            name: name.into(),
            from: "1-1".into(),
            to: "2-1".into(),
        }
    }

    fn ctx(upgrades: &[&str]) -> Context {
        Context {
            available_upgrades: upgrades.iter().map(|s| (*s).to_owned()).collect(),
            held: BTreeSet::new(),
        }
    }

    #[test]
    fn full_update_is_valid() {
        let plan = Plan {
            kind: PlanKind::SystemUpdate,
            steps: vec![up("firefox"), up("mesa")],
            actions: Vec::new(),
            builds: Vec::new(),
        };
        assert_eq!(plan.validate(&ctx(&["firefox", "mesa"])), Ok(()));
    }

    #[test]
    fn partial_update_is_rejected() {
        let plan = Plan {
            kind: PlanKind::SystemUpdate,
            steps: vec![up("firefox")],
            actions: Vec::new(),
            builds: Vec::new(),
        };
        assert_eq!(
            plan.validate(&ctx(&["firefox", "mesa"])),
            Err(PlanError::PartialUpgrade(vec!["mesa".into()]))
        );
        let install = Plan {
            kind: PlanKind::Install,
            steps: vec![Step::Install {
                name: "gimp".into(),
                version: "3".into(),
                reason: InstallReason::Explicit,
            }],
            actions: Vec::new(),
            builds: Vec::new(),
        };
        assert!(matches!(
            install.validate(&ctx(&["mesa"])),
            Err(PlanError::PartialUpgrade(_))
        ));
    }

    #[test]
    fn held_and_removed_packages_count_as_handled() {
        let mut c = ctx(&["firefox", "some-aur-tool", "lib32-audit"]);
        c.held.insert("some-aur-tool".into());
        let plan = Plan {
            kind: PlanKind::Repair,
            steps: vec![
                up("firefox"),
                Step::Remove {
                    name: "lib32-audit".into(),
                    version: "4.1.1-1".into(),
                    why: RemovalReason::Repair,
                },
            ],
            actions: Vec::new(),
            builds: Vec::new(),
        };
        assert_eq!(plan.validate(&c), Ok(()));
    }

    #[test]
    fn critical_packages_cannot_be_held() {
        for name in ["glibc", "linux", "linux-lts", "archlinux-keyring", "pacman"] {
            assert!(!can_hold(name), "{name}");
            let mut c = ctx(&[]);
            c.held.insert(name.into());
            let plan = Plan {
                kind: PlanKind::SystemUpdate,
                steps: vec![up("x")],
                actions: Vec::new(),
                builds: Vec::new(),
            };
            assert_eq!(plan.validate(&c), Err(PlanError::CannotHold(name.into())));
        }
        assert!(can_hold("some-aur-tool"));
    }

    #[test]
    fn restore_and_remove_skip_the_partial_upgrade_rule() {
        let restore = Plan::from_changes(
            PlanKind::Restore { generation: 41 },
            &[Change::Downgrade {
                name: "firefox".into(),
                from: "2".into(),
                to: "1".into(),
            }],
        );
        assert_eq!(restore.validate(&ctx(&["mesa"])), Ok(()));
    }

    #[test]
    fn empty_and_duplicate_plans_are_rejected() {
        assert_eq!(
            Plan::new(PlanKind::Remove).validate(&ctx(&[])),
            Err(PlanError::Empty)
        );
        let plan = Plan {
            kind: PlanKind::SystemUpdate,
            steps: vec![up("a"), up("a")],
            actions: Vec::new(),
            builds: Vec::new(),
        };
        assert_eq!(plan.validate(&ctx(&["a"])), Err(PlanError::Duplicate("a".into())));
    }

    #[test]
    fn from_changes_maps_every_change() {
        let changes = [
            Change::Install {
                name: "a".into(),
                version: "1".into(),
                reason: InstallReason::Dependency,
            },
            Change::Remove {
                name: "b".into(),
                version: "1".into(),
            },
            Change::Upgrade {
                name: "c".into(),
                from: "1".into(),
                to: "2".into(),
            },
            Change::Downgrade {
                name: "d".into(),
                from: "2".into(),
                to: "1".into(),
            },
            Change::Reason {
                name: "e".into(),
                reason: InstallReason::Explicit,
            },
        ];
        let plan = Plan::from_changes(PlanKind::Restore { generation: 1 }, &changes);
        let names: Vec<_> = plan.steps.iter().map(Step::package).collect();
        assert_eq!(names, ["a", "b", "c", "d", "e"]);
        assert!(matches!(
            plan.steps[1],
            Step::Remove {
                why: RemovalReason::Requested,
                ..
            }
        ));
    }

    #[test]
    fn groups_for_the_preview() {
        let mut plan = Plan {
            kind: PlanKind::SystemUpdate,
            steps: vec![up("firefox"), up("mesa")],
            actions: Vec::new(),
            builds: Vec::new(),
        };
        plan.add_cleanup([("electron36", "36.9.0-1")]);
        let is_app = |p: &str| p == "firefox";
        let g = plan.groups(&is_app);
        assert_eq!(
            g.apps.iter().map(|s| s.package()).collect::<Vec<_>>(),
            ["firefox"]
        );
        assert_eq!(g.system.iter().map(|s| s.package()).collect::<Vec<_>>(), ["mesa"]);
        assert_eq!(
            g.cleanup.iter().map(|s| s.package()).collect::<Vec<_>>(),
            ["electron36"]
        );
    }

    #[test]
    fn errors_read_well() {
        assert_eq!(
            PlanError::PartialUpgrade(vec!["a".into(), "b".into()]).to_string(),
            "partial upgrade: a, b would stay outdated"
        );
        let json = serde_json::to_string(&PlanKind::Restore { generation: 3 }).unwrap();
        assert_eq!(json, r#"{"restore":{"generation":3}}"#);
    }
}
