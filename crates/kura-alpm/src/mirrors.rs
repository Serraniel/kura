// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Ranking Arch mirrors: status download, speed test and writing the list.
//! Only kura-helper runs this; it decides on the mirrors itself so no caller
//! can put its own servers into pacman's configuration.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use kura_core::mirrors::{candidates, country_of_zone, mirrorlist, parse_status};

use crate::Error;

pub const MIRRORLIST: &str = "/etc/pacman.d/mirrorlist";
const STATUS_URL: &str = "https://archlinux.org/mirrors/status/json/";
/// How many candidates are speed-tested and how many end up in the list.
const TESTED: usize = 12;
const KEPT: usize = 10;

fn curl(args: &[&str]) -> Option<String> {
    let out = Command::new("curl")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The user's country, from the system timezone. Works offline.
pub fn country() -> Option<String> {
    let zone = fs::read_link("/etc/localtime").ok()?;
    let zone = zone.to_string_lossy();
    let zone = zone.split("zoneinfo/").nth(1)?;
    let tab = fs::read_to_string("/usr/share/zoneinfo/zone.tab").ok()?;
    country_of_zone(&tab, zone)
}

/// Download speed in bytes per second for a small file on the mirror.
fn speed(base: &str) -> f64 {
    let url = format!("{}/core/os/x86_64/core.db", base.trim_end_matches('/'));
    curl(&[
        "--silent",
        "--output",
        "/dev/null",
        "--max-time",
        "5",
        "--write-out",
        "%{speed_download}",
        &url,
    ])
    .and_then(|s| s.trim().parse().ok())
    .unwrap_or(0.0)
}

/// The result of a ranking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranked {
    pub count: usize,
    pub backup: PathBuf,
}

/// Ranks the mirrors and writes the new list; the old one is kept as a backup.
pub fn rank(list: &Path, generated: &str) -> Result<Ranked, Error> {
    let status = curl(&[
        "--fail",
        "--silent",
        "--show-error",
        "--location",
        "--max-time",
        "30",
        STATUS_URL,
    ])
    .ok_or_else(|| Error::Config("the mirror status could not be downloaded".into()))?;
    let mirrors = parse_status(&status).map_err(|e| Error::Config(e.to_string()))?;
    let country = country();
    let picked = candidates(&mirrors, country.as_deref(), TESTED);

    let handles: Vec<_> = picked
        .iter()
        .map(|m| {
            let url = m.url.clone();
            std::thread::spawn(move || (speed(&url), url))
        })
        .collect();
    let mut measured: Vec<(f64, String)> = handles.into_iter().filter_map(|h| h.join().ok()).collect();
    measured.retain(|(s, _)| *s > 0.0);
    measured.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    measured.truncate(KEPT);
    if measured.is_empty() {
        return Err(Error::Config("no mirror answered the speed test".into()));
    }
    let urls: Vec<&str> = measured.iter().map(|(_, u)| u.as_str()).collect();
    write(list, &mirrorlist(&urls, generated), generated).map(|backup| Ranked {
        count: urls.len(),
        backup,
    })
}

/// Keeps the current list as `<list>.kura-<stamp>` and writes the new one atomically.
pub fn write(list: &Path, text: &str, stamp: &str) -> Result<PathBuf, Error> {
    let io = |source| Error::Io {
        path: list.display().to_string(),
        source,
    };
    let backup = list.with_file_name(format!(
        "{}.kura-{stamp}",
        list.file_name().and_then(|n| n.to_str()).unwrap_or("mirrorlist")
    ));
    if list.exists() {
        fs::copy(list, &backup).map_err(io)?;
    }
    let tmp = list.with_extension("kura-new");
    fs::write(&tmp, text).map_err(io)?;
    fs::rename(&tmp, list).map_err(io)?;
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_keeps_a_backup() {
        let dir = std::env::temp_dir().join(format!("kura-mirrors-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let list = dir.join("mirrorlist");
        fs::write(&list, "Server = old\n").unwrap();
        let backup = write(&list, "Server = new\n", "2026-10-25").unwrap();
        assert_eq!(fs::read_to_string(&list).unwrap(), "Server = new\n");
        assert_eq!(fs::read_to_string(&backup).unwrap(), "Server = old\n");
        assert_eq!(backup, dir.join("mirrorlist.kura-2026-10-25"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn country_does_not_panic() {
        let _ = country();
    }

    /// Downloads the mirror status and tests speeds. CI container only.
    #[test]
    #[ignore = "network; CI container only"]
    fn ranks_in_a_container() {
        let dir = std::env::temp_dir().join(format!("kura-rank-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let ranked = rank(&dir.join("mirrorlist"), "test").unwrap();
        assert!(ranked.count > 0);
        let _ = fs::remove_dir_all(dir);
    }
}
