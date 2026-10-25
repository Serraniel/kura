// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The periodic check: find updates on a private database copy, download them
//! into pacman's cache without installing anything, and write the status.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use alpm::TransFlag;
use kura_core::news::Item;
use kura_core::status::Status;

use crate::prepare::{Request, available_upgrades, build, open};
use crate::{Config, Error};

pub const STATUS: &str = "/var/lib/kura/status.json";
pub const CHECK_DB: &str = "/var/lib/kura/checkdb";
const NEWS_FEED: &str = "https://archlinux.org/feeds/news/";

/// Reads the Arch news feed with curl. `None` when it cannot be loaded.
pub fn fetch_news() -> Option<Vec<Item>> {
    let out = std::process::Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--max-time",
            "20",
            NEWS_FEED,
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    Some(kura_core::news::parse_rss(&String::from_utf8_lossy(&out.stdout)))
}

/// Start of the period whose news matter: the last recorded update, or 30 days ago.
pub fn news_since(journal: &Path, now: i64) -> i64 {
    crate::history::load(journal)
        .ok()
        .and_then(|j| {
            j.records()
                .iter()
                .rev()
                .find(|r| r.summary().upgraded > 0)
                .map(|r| r.created)
        })
        .unwrap_or(now - 30 * 86_400)
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// Runs the check. With `download`, the packages of the update are fetched
/// into the cache so installing later is quick.
pub fn check(
    conf: &Config,
    workdir: &Path,
    download: bool,
    news: Option<&[Item]>,
    since: i64,
) -> Result<Status, Error> {
    let mut handle = open(conf, Some(workdir))?;
    handle.syncdbs_mut().update(false)?;
    let available = available_upgrades(&handle);
    let flags = if download {
        TransFlag::DOWNLOAD_ONLY
    } else {
        TransFlag::NONE
    };
    handle.trans_init(flags)?;
    let built = build(&mut handle, &Request::Update, &available);
    let mut downloaded = available.is_empty();
    let blocked = match &built {
        Ok(b) => b.plan.is_err(),
        Err(_) => true,
    };
    if download && !blocked && !available.is_empty() {
        downloaded = handle.trans_commit().is_ok();
    }
    handle.trans_release()?;

    let packages: BTreeSet<String> = available.iter().cloned().collect();
    let news = news
        .map(|items| {
            kura_core::news::affecting(items, since, &packages)
                .into_iter()
                .map(|i| i.title.clone())
                .collect()
        })
        .unwrap_or_default();
    Ok(Status {
        checked: now(),
        updates: available.len(),
        downloaded,
        blocked,
        news,
    })
}

/// Writes the status atomically so readers never see half a file.
pub fn write_status(path: &Path, status: &Status) -> Result<(), Error> {
    let io = |source| Error::Io {
        path: path.display().to_string(),
        source,
    };
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(io)?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string(status).map_err(|e| Error::Config(e.to_string()))?;
    fs::write(&tmp, json).map_err(io)?;
    fs::rename(&tmp, path).map_err(io)
}

/// Reads the status. A missing or unreadable file means no check ran yet.
pub fn read_status(path: &Path) -> Status {
    fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_round_trip_and_missing_file() {
        let dir = std::env::temp_dir().join(format!("kura-status-{}", std::process::id()));
        let path = dir.join("status.json");
        assert_eq!(read_status(&path), Status::default());
        let s = Status {
            checked: 9,
            updates: 2,
            downloaded: true,
            blocked: false,
            news: vec!["x".into()],
        };
        write_status(&path, &s).unwrap();
        assert_eq!(read_status(&path), s);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn news_period_without_history_is_thirty_days() {
        assert_eq!(
            news_since(Path::new("/nonexistent/history.jsonl"), 100 * 86_400),
            70 * 86_400
        );
    }

    /// Downloads databases and packages. CI container only.
    #[test]
    #[ignore = "network and root; CI container only"]
    fn checks_in_a_container() {
        let conf = crate::load_config().unwrap();
        let dir = std::env::temp_dir().join(format!("kura-bg-{}", std::process::id()));
        let status = check(&conf, &dir, true, None, 0).unwrap();
        assert!(status.checked > 0);
        assert!(!status.blocked);
        let _ = fs::remove_dir_all(dir);
    }
}
