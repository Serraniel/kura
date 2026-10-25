// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The parts of pacman.conf kura needs: repositories with their servers,
//! the architecture and the cache directories.

use std::path::{Path, PathBuf};

/// One `[repository]` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    pub name: String,
    /// Server URLs with `$repo` and `$arch` already filled in.
    pub servers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub architecture: String,
    pub cache_dirs: Vec<String>,
    pub repos: Vec<Repo>,
}

/// Reads a file; used so tests can supply their own files.
pub trait Files {
    fn read(&self, path: &Path) -> Option<String>;
    /// Files in a directory, for `Include` lines with a `*`.
    fn list(&self, dir: &Path) -> Vec<PathBuf>;
}

/// The real file system.
#[derive(Debug, Clone, Copy, Default)]
pub struct Disk;

impl Files for Disk {
    fn read(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }

    fn list(&self, dir: &Path) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
            .unwrap_or_default();
        out.sort();
        out
    }
}

/// Matches a file name against a pattern with `*` wildcards.
fn glob_match(pattern: &str, name: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == name;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !name.starts_with(first) || !name[first.len()..].ends_with(last) {
        return false;
    }
    let mut rest = &name[first.len()..name.len() - last.len()];
    for mid in &parts[1..parts.len() - 1] {
        match rest.find(mid) {
            Some(i) => rest = &rest[i + mid.len()..],
            None => return false,
        }
    }
    true
}

fn expand_include(files: &impl Files, value: &str) -> Vec<PathBuf> {
    let path = Path::new(value);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    if !name.contains('*') {
        return vec![path.to_path_buf()];
    }
    let dir = path.parent().unwrap_or(Path::new("/"));
    files
        .list(dir)
        .into_iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| glob_match(name, n))
        })
        .collect()
}

#[derive(Default)]
struct Raw {
    architecture: Option<String>,
    cache_dirs: Vec<String>,
    /// Repository name with raw server URLs.
    repos: Vec<(String, Vec<String>)>,
}

fn parse_into(files: &impl Files, text: &str, section: &mut Option<String>, raw: &mut Raw, depth: u8) {
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            *section = Some(name.to_owned());
            if name != "options" && !raw.repos.iter().any(|(n, _)| n == name) {
                raw.repos.push((name.to_owned(), Vec::new()));
            }
            continue;
        }
        let (key, value) = match line.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => (line, ""),
        };
        match (section.as_deref(), key) {
            (_, "Include") if depth < 8 => {
                for path in expand_include(files, value) {
                    if let Some(included) = files.read(&path) {
                        parse_into(files, &included, section, raw, depth + 1);
                    }
                }
            }
            (Some("options"), "Architecture") => {
                raw.architecture = value.split_whitespace().next().map(str::to_owned);
            }
            (Some("options"), "CacheDir") => {
                raw.cache_dirs.extend(value.split_whitespace().map(str::to_owned));
            }
            (Some(repo), "Server") if repo != "options" => {
                if let Some((_, servers)) = raw.repos.iter_mut().find(|(n, _)| n == repo) {
                    servers.push(value.to_owned());
                }
            }
            _ => {}
        }
    }
}

/// Parses pacman.conf and the files it includes.
pub fn parse(files: &impl Files, path: &Path) -> Option<Config> {
    let text = files.read(path)?;
    let mut raw = Raw::default();
    parse_into(files, &text, &mut None, &mut raw, 0);
    let architecture = match raw.architecture.as_deref() {
        None | Some("auto") => std::env::consts::ARCH.to_owned(),
        Some(a) => a.to_owned(),
    };
    let repos = raw
        .repos
        .into_iter()
        .map(|(name, servers)| Repo {
            servers: servers
                .iter()
                .map(|s| s.replace("$repo", &name).replace("$arch", &architecture))
                .collect(),
            name,
        })
        .collect();
    let cache_dirs = if raw.cache_dirs.is_empty() {
        vec!["/var/cache/pacman/pkg/".to_owned()]
    } else {
        raw.cache_dirs
    };
    Some(Config {
        architecture,
        cache_dirs,
        repos,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    struct Mem(BTreeMap<&'static str, &'static str>);

    impl Files for Mem {
        fn read(&self, path: &Path) -> Option<String> {
            self.0.get(path.to_str()?).map(|s| (*s).to_owned())
        }
        fn list(&self, dir: &Path) -> Vec<PathBuf> {
            self.0
                .keys()
                .map(PathBuf::from)
                .filter(|p| p.parent() == Some(dir))
                .collect()
        }
    }

    #[test]
    fn reads_repos_servers_and_includes() {
        let files = Mem(BTreeMap::from([
            (
                "/etc/pacman.conf",
                "[options]\nArchitecture = auto\nCacheDir = /var/cache/pacman/pkg/ /mnt/cache/\nInclude = /etc/pacman.d/conf.d/*.conf\n\n[endeavouros]\nSigLevel = PackageRequired\nInclude = /etc/pacman.d/endeavouros-mirrorlist\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n\n[extra]\nInclude = /etc/pacman.d/mirrorlist\n# [testing]\n",
            ),
            (
                "/etc/pacman.d/mirrorlist",
                "## Germany\nServer = https://mirror.example.de/archlinux/$repo/os/$arch\n#Server = https://disabled.example/$repo/os/$arch\nServer = https://other.example/$repo/os/$arch\n",
            ),
            (
                "/etc/pacman.d/endeavouros-mirrorlist",
                "Server = https://eos.example/$repo/$arch\n",
            ),
            ("/etc/pacman.d/conf.d/10-extra.conf", "ParallelDownloads = 5\n"),
            ("/etc/pacman.d/conf.d/README", "Server = ignored\n"),
        ]));
        let c = parse(&files, Path::new("/etc/pacman.conf")).unwrap();
        assert_eq!(c.architecture, std::env::consts::ARCH);
        assert_eq!(c.cache_dirs, ["/var/cache/pacman/pkg/", "/mnt/cache/"]);
        let names: Vec<_> = c.repos.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["endeavouros", "core", "extra"]);
        let arch = std::env::consts::ARCH;
        assert_eq!(
            c.repos[0].servers,
            [format!("https://eos.example/endeavouros/{arch}")]
        );
        assert_eq!(
            c.repos[1].servers,
            [
                format!("https://mirror.example.de/archlinux/core/os/{arch}"),
                format!("https://other.example/core/os/{arch}"),
            ]
        );
    }

    #[test]
    fn explicit_architecture_and_default_cache() {
        let files = Mem(BTreeMap::from([(
            "/c",
            "[options]\nArchitecture = x86_64_v3 x86_64\n[core]\nServer = https://m/$repo/$arch\n",
        )]));
        let c = parse(&files, Path::new("/c")).unwrap();
        assert_eq!(c.architecture, "x86_64_v3");
        assert_eq!(c.cache_dirs, ["/var/cache/pacman/pkg/"]);
        assert_eq!(c.repos[0].servers, ["https://m/core/x86_64_v3"]);
    }

    #[test]
    fn missing_file_gives_none() {
        assert_eq!(parse(&Mem(BTreeMap::new()), Path::new("/nope")), None);
    }

    #[test]
    fn globs() {
        assert!(glob_match("*.conf", "10-a.conf"));
        assert!(!glob_match("*.conf", "README"));
        assert!(glob_match("a*b*c", "axxbyyc"));
        assert!(!glob_match("a*b*c", "axxc"));
        assert!(glob_match("exact", "exact"));
        assert!(!glob_match("exact", "other"));
    }

    #[test]
    fn reads_this_system() {
        if let Some(c) = parse(&Disk, Path::new("/etc/pacman.conf")) {
            assert!(c.repos.iter().any(|r| r.name == "core"));
        }
    }
}
