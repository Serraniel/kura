// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading and appending the generation journal on disk.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kura_core::generation::Generation;
use kura_core::history::{Journal, Record, record_line};

use crate::{Config, Error};

/// Where the journal lives. Readable by everyone, written only by root.
pub const JOURNAL: &str = "/var/lib/kura/history.jsonl";
/// Written by kura-helper before a transaction so the record knows its cause.
pub const CAUSE: &str = "/run/kura/cause";

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> Error + '_ {
    move |source| Error::Io {
        path: path.display().to_string(),
        source,
    }
}

/// Reads the journal. A missing file is an empty journal.
pub fn load(path: &Path) -> Result<Journal, Error> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(io(path)(e)),
    };
    Journal::parse(text.lines()).map_err(|e| Error::Config(format!("{}: {e}", path.display())))
}

/// The cause kura-helper left behind, if it is recent. Anything older than an
/// hour is stale (a crashed run) and ignored.
pub fn read_cause(path: &Path, now: SystemTime) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    let age = now.duration_since(meta.modified().ok()?).unwrap_or_default();
    if age > Duration::from_secs(3600) {
        return None;
    }
    let cause = fs::read_to_string(path).ok()?;
    let cause = cause.trim();
    (!cause.is_empty()).then(|| cause.to_owned())
}

/// Appends a record for the current state of the system, if anything changed.
pub fn record(conf: &Config, journal: &Path, cause: &Path) -> Result<Option<Record>, Error> {
    let now = SystemTime::now();
    let created = now
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
    let set = crate::installed(conf)?;
    let current = Generation::from_installed(0, created, set.iter());
    let existing = load(journal)?;
    let cause = read_cause(cause, now).or_else(|| Some("pacman".to_owned()));
    let Some(record) = existing.record(&current, created, cause) else {
        return Ok(None);
    };
    append(journal, &record)?;
    Ok(Some(record))
}

/// Appends one record as a line. Runs inside pacman's transaction lock, so
/// there is only ever one writer.
pub fn append(journal: &Path, record: &Record) -> Result<(), Error> {
    if let Some(dir) = journal.parent() {
        fs::create_dir_all(dir).map_err(io(dir))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(journal)
        .map_err(io(journal))?;
    writeln!(file, "{}", record_line(record)).map_err(io(journal))
}

#[cfg(test)]
mod tests {
    use kura_core::generation::Generation;
    use kura_core::package::{InstallReason, InstalledPackage};

    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("kura-history-{name}-{}", std::process::id()))
    }

    #[test]
    fn append_and_load_round_trip() {
        let dir = tmp("journal");
        let path = dir.join("history.jsonl");
        assert!(load(&path).unwrap().is_empty());
        let pkgs = [InstalledPackage::new("a", "1-1", InstallReason::Explicit)];
        let r = Journal::default()
            .record(&Generation::from_installed(0, 5, &pkgs), 5, None)
            .unwrap();
        append(&path, &r).unwrap();
        let j = load(&path).unwrap();
        assert_eq!(j.records(), [r]);
        fs::write(&path, "garbage\n").unwrap();
        assert!(matches!(load(&path), Err(Error::Config(_))));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cause_is_read_only_when_fresh() {
        let path = tmp("cause");
        assert_eq!(read_cause(&path, SystemTime::now()), None);
        fs::write(&path, "kura:system-update\n").unwrap();
        assert_eq!(
            read_cause(&path, SystemTime::now()).as_deref(),
            Some("kura:system-update")
        );
        let later = SystemTime::now() + Duration::from_secs(7200);
        assert_eq!(read_cause(&path, later), None);
        fs::write(&path, "  \n").unwrap();
        assert_eq!(read_cause(&path, SystemTime::now()), None);
        fs::remove_file(path).unwrap();
    }

    /// Records the real system. CI container only.
    #[test]
    #[ignore = "reads the real pacman database"]
    fn records_this_system() {
        let conf = crate::load_config().unwrap();
        let dir = tmp("real");
        let path = dir.join("history.jsonl");
        let first = record(&conf, &path, &dir.join("cause")).unwrap().unwrap();
        assert_eq!(first.id, 1);
        assert_eq!(first.cause.as_deref(), Some("pacman"));
        // Nothing changed, so nothing new is recorded.
        assert!(record(&conf, &path, &dir.join("cause")).unwrap().is_none());
        fs::remove_dir_all(dir).unwrap();
    }
}
