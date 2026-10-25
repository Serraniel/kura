// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The history of generations as a journal.
//!
//! Each line of the journal holds the changes since the previous generation;
//! the first line holds the full package list. Any generation is rebuilt by
//! replaying the lines up to it. This keeps the file small even after years
//! of updates.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::generation::{Change, Entry, Generation};
use crate::package::InstallReason;

/// One journal line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub id: u64,
    /// Unix time in seconds.
    pub created: i64,
    /// What caused the change, for example `kura:system-update` or `pacman`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    pub changes: Vec<RecordChange>,
}

/// A change with everything needed to replay it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum RecordChange {
    /// Package added or its version, repository or reason changed.
    Set {
        name: String,
        version: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        repo: Option<String>,
        reason: InstallReason,
        /// Version before, if the package was installed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<String>,
    },
    Unset {
        name: String,
        version: String,
    },
}

/// How many packages a record installed, upgraded, downgraded and removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    pub installed: usize,
    pub upgraded: usize,
    pub downgraded: usize,
    pub removed: usize,
}

impl Record {
    pub fn summary(&self) -> Summary {
        let mut s = Summary::default();
        for c in &self.changes {
            match c {
                RecordChange::Set { from: None, .. } => s.installed += 1,
                RecordChange::Set {
                    from: Some(from),
                    version,
                    ..
                } => match crate::version::vercmp(from, version) {
                    std::cmp::Ordering::Less => s.upgraded += 1,
                    std::cmp::Ordering::Greater => s.downgraded += 1,
                    std::cmp::Ordering::Equal => {}
                },
                RecordChange::Unset { .. } => s.removed += 1,
            }
        }
        s
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JournalError {
    #[error("line {line}: {message}")]
    Parse { line: usize, message: String },
    #[error("line {line}: generation {id} is not newer than the one before")]
    Order { line: usize, id: u64 },
}

/// All records, oldest first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Journal {
    records: Vec<Record>,
}

fn apply(packages: &mut BTreeMap<String, Entry>, changes: &[RecordChange]) {
    for c in changes {
        match c {
            RecordChange::Set {
                name,
                version,
                repo,
                reason,
                ..
            } => {
                packages.insert(
                    name.clone(),
                    Entry {
                        version: version.clone(),
                        repo: repo.clone(),
                        reason: *reason,
                    },
                );
            }
            RecordChange::Unset { name, .. } => {
                packages.remove(name);
            }
        }
    }
}

impl Journal {
    /// Reads journal lines. Empty lines are skipped.
    pub fn parse<'a>(lines: impl IntoIterator<Item = &'a str>) -> Result<Self, JournalError> {
        let mut records: Vec<Record> = Vec::new();
        for (i, line) in lines.into_iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let record: Record = serde_json::from_str(line).map_err(|e| JournalError::Parse {
                line: i + 1,
                message: e.to_string(),
            })?;
            if records.last().is_some_and(|last| last.id >= record.id) {
                return Err(JournalError::Order {
                    line: i + 1,
                    id: record.id,
                });
            }
            records.push(record);
        }
        Ok(Self { records })
    }

    pub fn records(&self) -> &[Record] {
        &self.records
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Rebuilds a generation by replaying the journal up to `id`.
    pub fn generation(&self, id: u64) -> Option<Generation> {
        let end = self.records.iter().position(|r| r.id == id)?;
        let mut packages = BTreeMap::new();
        for r in &self.records[..=end] {
            apply(&mut packages, &r.changes);
        }
        let r = &self.records[end];
        Some(Generation {
            id: r.id,
            created: r.created,
            packages,
        })
    }

    /// The newest generation, or an empty one if nothing was recorded yet.
    pub fn latest(&self) -> Generation {
        self.records
            .last()
            .and_then(|r| self.generation(r.id))
            .unwrap_or(Generation {
                id: 0,
                created: 0,
                packages: BTreeMap::new(),
            })
    }

    /// Creates the record that brings the journal to `current`, or `None`
    /// if nothing changed. The caller appends `record_line` to the file.
    pub fn record(&self, current: &Generation, created: i64, cause: Option<String>) -> Option<Record> {
        let latest = self.latest();
        let mut changes = Vec::new();
        for (name, now) in &current.packages {
            let before = latest.packages.get(name);
            if before != Some(now) {
                changes.push(RecordChange::Set {
                    name: name.clone(),
                    version: now.version.clone(),
                    repo: now.repo.clone(),
                    reason: now.reason,
                    from: before.map(|b| b.version.clone()),
                });
            }
        }
        for (name, before) in &latest.packages {
            if !current.packages.contains_key(name) {
                changes.push(RecordChange::Unset {
                    name: name.clone(),
                    version: before.version.clone(),
                });
            }
        }
        if changes.is_empty() {
            return None;
        }
        Some(Record {
            id: latest.id + 1,
            created,
            cause,
            changes,
        })
    }

    /// Adds a record created by [`Journal::record`].
    pub fn push(&mut self, record: Record) {
        self.records.push(record);
    }

    /// Changes needed to go from the newest generation back to `id`.
    pub fn changes_to(&self, id: u64) -> Option<Vec<Change>> {
        let target = self.generation(id)?;
        Some(self.latest().diff(&target))
    }
}

/// One journal line for a record, without the trailing newline.
///
/// # Panics
///
/// Never: records only contain strings, numbers and enums.
pub fn record_line(record: &Record) -> String {
    serde_json::to_string(record).expect("records always serialise")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::InstalledPackage;
    use InstallReason::{Dependency, Explicit};

    fn generation(pkgs: &[(&str, &str, InstallReason)]) -> Generation {
        let installed: Vec<_> = pkgs
            .iter()
            .map(|(n, v, r)| InstalledPackage::new(*n, *v, *r))
            .collect();
        Generation::from_installed(0, 0, &installed)
    }

    fn journal_with(states: &[&[(&str, &str, InstallReason)]]) -> (Journal, String) {
        let mut j = Journal::default();
        let mut text = String::new();
        for (i, s) in states.iter().enumerate() {
            let created = 1_000 + i64::try_from(i).unwrap();
            if let Some(r) = j.record(&generation(s), created, Some("pacman".into())) {
                text.push_str(&record_line(&r));
                text.push('\n');
                j.push(r);
            }
        }
        (j, text)
    }

    const S1: &[(&str, &str, InstallReason)] = &[
        ("firefox", "151.0-1", Explicit),
        ("mesa", "1:26.2.1-1", Dependency),
    ];
    const S2: &[(&str, &str, InstallReason)] = &[
        ("firefox", "152.0-1", Explicit),
        ("mesa", "1:26.2.1-1", Dependency),
        ("gimp", "3.0.6-1", Explicit),
    ];
    const S3: &[(&str, &str, InstallReason)] =
        &[("firefox", "152.0-1", Explicit), ("mesa", "1:26.2.0-1", Explicit)];

    #[test]
    fn records_only_differences_and_replays_them() {
        let (j, text) = journal_with(&[S1, S1, S2, S3]);
        // The unchanged second state does not create a record.
        assert_eq!(j.records().len(), 3);
        assert_eq!(text.lines().count(), 3);

        let parsed = Journal::parse(text.lines()).unwrap();
        assert_eq!(parsed, j);
        assert_eq!(parsed.generation(1).unwrap().packages, generation(S1).packages);
        assert_eq!(parsed.generation(2).unwrap().packages, generation(S2).packages);
        assert_eq!(parsed.latest().packages, generation(S3).packages);
        assert_eq!(parsed.latest().id, 3);
        assert_eq!(parsed.generation(9), None);
    }

    #[test]
    fn summaries_count_kinds_of_change() {
        let (j, _) = journal_with(&[S1, S2, S3]);
        assert_eq!(
            j.records()[0].summary(),
            Summary {
                installed: 2,
                ..Summary::default()
            }
        );
        assert_eq!(
            j.records()[1].summary(),
            Summary {
                installed: 1,
                upgraded: 1,
                ..Summary::default()
            }
        );
        // mesa went back a version and changed its reason; gimp was removed.
        assert_eq!(
            j.records()[2].summary(),
            Summary {
                downgraded: 1,
                removed: 1,
                ..Summary::default()
            }
        );
    }

    #[test]
    fn reason_only_change_counts_as_no_version_change() {
        let a: &[(&str, &str, InstallReason)] = &[("x", "1-1", Dependency)];
        let b: &[(&str, &str, InstallReason)] = &[("x", "1-1", Explicit)];
        let (j, _) = journal_with(&[a, b]);
        assert_eq!(j.records().len(), 2);
        assert_eq!(j.records()[1].summary(), Summary::default());
    }

    #[test]
    fn changes_to_an_earlier_generation() {
        let (j, _) = journal_with(&[S1, S2]);
        let changes = j.changes_to(1).unwrap();
        assert!(changes.contains(&Change::Remove {
            name: "gimp".into(),
            version: "3.0.6-1".into()
        }));
        assert!(changes.contains(&Change::Downgrade {
            name: "firefox".into(),
            from: "152.0-1".into(),
            to: "151.0-1".into()
        }));
        assert_eq!(j.changes_to(5), None);
    }

    #[test]
    fn empty_journal() {
        let j = Journal::parse(["", "  "]).unwrap();
        assert!(j.is_empty());
        assert_eq!(j.latest().id, 0);
        assert!(j.latest().packages.is_empty());
    }

    #[test]
    fn broken_lines_are_reported() {
        assert!(matches!(
            Journal::parse(["{not json"]),
            Err(JournalError::Parse { line: 1, .. })
        ));
        let line = r#"{"id":1,"created":0,"changes":[]}"#;
        assert_eq!(
            Journal::parse([line, line]),
            Err(JournalError::Order { line: 2, id: 1 })
        );
        assert_eq!(
            JournalError::Order { line: 2, id: 1 }.to_string(),
            "line 2: generation 1 is not newer than the one before"
        );
    }
}
