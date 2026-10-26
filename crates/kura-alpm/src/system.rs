// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Facts for the system health page and the package search. Only reads.

use std::fs;
use std::path::Path;
use std::process::Command;

use kura_core::hardware;
use kura_core::health::{Facts, is_kernel};

use crate::{Config, Error};

/// What the health page shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Info {
    pub cpu: Option<String>,
    pub memory: Option<u64>,
    pub graphics: Vec<String>,
    /// Used and total bytes of `/`.
    pub disk: Option<(u64, u64)>,
    /// Bytes of old package files in pacman's cache.
    pub cache: u64,
    /// Installed kernels with their versions.
    pub kernels: Vec<(String, String)>,
    /// `uname -r` of the running kernel.
    pub running: String,
    pub facts: Facts,
}

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn graphics() -> Vec<String> {
    let pci_ids = read("/usr/share/hwdata/pci.ids");
    let mut out: Vec<String> = Vec::new();
    let Ok(cards) = fs::read_dir("/sys/class/drm") else {
        return out;
    };
    for card in cards.flatten() {
        let dev = card.path().join("device");
        let class = fs::read_to_string(dev.join("class")).unwrap_or_default();
        if !class.trim().starts_with("0x03") {
            continue;
        }
        let id = |f: &str| {
            fs::read_to_string(dev.join(f))
                .ok()
                .and_then(|s| hardware::hex_id(&s))
        };
        if let (Some(v), Some(d)) = (id("vendor"), id("device")) {
            let name = hardware::pci_name(&pci_ids, v, d).unwrap_or_else(|| format!("{v:04x}:{d:04x}"));
            if !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out
}

fn dir_size(dir: &Path) -> u64 {
    fs::read_dir(dir).map_or(0, |entries| {
        entries
            .flatten()
            .filter_map(|e| e.metadata().ok())
            .filter(fs::Metadata::is_file)
            .map(|m| m.len())
            .sum()
    })
}

/// Collects everything the health page needs.
pub fn collect(conf: &Config) -> Result<Info, Error> {
    let set = crate::installed(conf)?;
    let kernels: Vec<(String, String)> = set
        .iter()
        .filter(|p| is_kernel(&p.name))
        .map(|p| (p.name.clone(), p.version.clone()))
        .collect();
    let disk = Command::new("df")
        .args(["--block-size=1", "--output=used,size", "/"])
        .output()
        .ok()
        .and_then(|o| hardware::df_usage(&String::from_utf8_lossy(&o.stdout)));
    let facts = Facts {
        kernels: kernels.iter().map(|(n, _)| n.clone()).collect(),
        root_fs: hardware::root_fs(&read("/proc/mounts")).unwrap_or_default(),
        snapper: Path::new("/etc/snapper/configs/root").exists(),
        orphans: set.orphans().len(),
        mirrors_ranked: Path::new("/var/lib/kura/mirrors-ranked").exists(),
    };
    Ok(Info {
        cpu: hardware::cpu_model(&read("/proc/cpuinfo")),
        memory: hardware::mem_total(&read("/proc/meminfo")),
        graphics: graphics(),
        disk,
        cache: conf.cache_dir.iter().map(|d| dir_size(Path::new(d))).sum(),
        kernels,
        running: read("/proc/sys/kernel/osrelease").trim().to_owned(),
        facts,
    })
}

/// One search result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub name: String,
    pub version: String,
    pub description: String,
    pub repo: String,
    pub installed: bool,
}

/// Searches the repositories by name and description. Name matches first.
pub fn search(conf: &Config, query: &str, limit: usize) -> Result<Vec<Hit>, Error> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let handle = crate::handle(conf)?;
    let mut hits: Vec<(u8, Hit)> = Vec::new();
    for db in handle.syncdbs() {
        for p in db.pkgs() {
            let name = p.name().to_lowercase();
            let desc = p.desc().unwrap_or_default();
            let rank = if name == q {
                0
            } else if name.starts_with(&q) {
                1
            } else if name.contains(&q) {
                2
            } else if desc.to_lowercase().contains(&q) {
                3
            } else {
                continue;
            };
            hits.push((
                rank,
                Hit {
                    name: p.name().to_owned(),
                    version: p.version().as_str().to_owned(),
                    description: desc.to_owned(),
                    repo: db.name().to_owned(),
                    installed: handle.localdb().pkg(p.name()).is_ok(),
                },
            ));
        }
    }
    hits.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.name.cmp(&b.1.name)));
    hits.dedup_by(|a, b| a.1.name == b.1.name);
    Ok(hits.into_iter().take(limit).map(|(_, h)| h).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_size_of_missing_dir_is_zero() {
        assert_eq!(dir_size(Path::new("/nonexistent/kura")), 0);
    }

    /// Reads the real system. CI container only.
    #[test]
    #[ignore = "needs a real pacman installation"]
    fn collects_and_searches_in_a_container() {
        let conf = crate::load_config().unwrap();
        let info = collect(&conf).unwrap();
        assert!(info.kernels.is_empty() || !info.running.is_empty());
        let hits = search(&conf, "pacman", 10).unwrap();
        assert_eq!(hits[0].name, "pacman");
        assert!(hits[0].installed);
        assert_eq!(search(&conf, "  ", 10).unwrap().len(), 0);
    }
}
