// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Generations: the list of installed packages after a transaction.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::package::{InstallReason, InstalledPackage};
use crate::version::vercmp;

/// What a generation stores per package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    pub reason: InstallReason,
}

/// A snapshot of installed package versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Generation {
    pub id: u64,
    /// Unix time in seconds.
    pub created: i64,
    pub packages: BTreeMap<String, Entry>,
}

impl Generation {
    pub fn from_installed<'a>(
        id: u64,
        created: i64,
        installed: impl IntoIterator<Item = &'a InstalledPackage>,
    ) -> Self {
        let packages = installed
            .into_iter()
            .map(|p| {
                (
                    p.name.clone(),
                    Entry {
                        version: p.version.clone(),
                        repo: p.repo.clone(),
                        reason: p.reason,
                    },
                )
            })
            .collect();
        Self {
            id,
            created,
            packages,
        }
    }

    /// Changes needed to get from `self` to `target`.
    pub fn diff(&self, target: &Self) -> Vec<Change> {
        let mut changes = Vec::new();
        for (name, now) in &self.packages {
            match target.packages.get(name) {
                None => changes.push(Change::Remove {
                    name: name.clone(),
                    version: now.version.clone(),
                }),
                Some(then) => {
                    match vercmp(&now.version, &then.version) {
                        Ordering::Less => changes.push(Change::Upgrade {
                            name: name.clone(),
                            from: now.version.clone(),
                            to: then.version.clone(),
                        }),
                        Ordering::Greater => changes.push(Change::Downgrade {
                            name: name.clone(),
                            from: now.version.clone(),
                            to: then.version.clone(),
                        }),
                        Ordering::Equal => {}
                    }
                    if now.reason != then.reason {
                        changes.push(Change::Reason {
                            name: name.clone(),
                            reason: then.reason,
                        });
                    }
                }
            }
        }
        for (name, then) in &target.packages {
            if !self.packages.contains_key(name) {
                changes.push(Change::Install {
                    name: name.clone(),
                    version: then.version.clone(),
                    reason: then.reason,
                });
            }
        }
        changes
    }
}

/// One difference between two generations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Change {
    Install {
        name: String,
        version: String,
        reason: InstallReason,
    },
    Remove {
        name: String,
        version: String,
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
    Reason {
        name: String,
        reason: InstallReason,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use InstallReason::{Dependency, Explicit};

    fn generation(id: u64, pkgs: &[(&str, &str, InstallReason)]) -> Generation {
        let installed: Vec<_> = pkgs
            .iter()
            .map(|(n, v, r)| InstalledPackage::new(*n, *v, *r))
            .collect();
        Generation::from_installed(id, 1_761_400_000, &installed)
    }

    #[test]
    fn diff_lists_every_kind_of_change() {
        let now = generation(
            42,
            &[
                ("firefox", "152.0-1", Explicit),
                ("linux", "7.2.13-1", Explicit),
                ("gimp", "3.0.6-1", Explicit),
                ("hunspell", "1.7.2-1", Dependency),
            ],
        );
        let then = generation(
            41,
            &[
                ("firefox", "151.0.1-1", Explicit),
                ("linux", "7.2.13-1", Explicit),
                ("hunspell", "1.7.2-1", Explicit),
                ("electron36", "36.9.0-1", Dependency),
                ("mesa", "1:26.2.2-1", Dependency),
            ],
        );
        let mut changes = now.diff(&then);
        changes.sort_by_key(|c| format!("{c:?}"));
        assert_eq!(
            changes,
            vec![
                Change::Downgrade {
                    name: "firefox".into(),
                    from: "152.0-1".into(),
                    to: "151.0.1-1".into()
                },
                Change::Install {
                    name: "electron36".into(),
                    version: "36.9.0-1".into(),
                    reason: Dependency
                },
                Change::Install {
                    name: "mesa".into(),
                    version: "1:26.2.2-1".into(),
                    reason: Dependency
                },
                Change::Reason {
                    name: "hunspell".into(),
                    reason: Explicit
                },
                Change::Remove {
                    name: "gimp".into(),
                    version: "3.0.6-1".into()
                },
            ]
        );
        let back = then.diff(&now);
        assert!(back.contains(&Change::Upgrade {
            name: "firefox".into(),
            from: "151.0.1-1".into(),
            to: "152.0-1".into()
        }));
    }

    #[test]
    fn identical_generations_have_no_changes() {
        let g = generation(1, &[("a", "1-1", Explicit)]);
        assert_eq!(g.diff(&g.clone()), Vec::<Change>::new());
    }

    #[test]
    fn serialises_round_trip() {
        let g = generation(7, &[("a", "1-1", Explicit), ("b", "2-1", Dependency)]);
        let json = serde_json::to_string(&g).unwrap();
        assert_eq!(serde_json::from_str::<Generation>(&json).unwrap(), g);
        let c = Change::Remove {
            name: "a".into(),
            version: "1".into(),
        };
        assert_eq!(
            serde_json::to_string(&c).unwrap(),
            r#"{"kind":"remove","name":"a","version":"1"}"#
        );
    }
}
