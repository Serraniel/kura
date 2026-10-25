// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reads pacman's databases through libalpm and turns them into kura-core types.
//!
//! Never parse pacman's text output: it is translated and changes between
//! versions. Everything goes through libalpm.

use std::path::Path;

use alpm::{Alpm, PackageReason, SigLevel};
use kura_core::package::{InstallReason, InstalledPackage, PackageSet};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("libalpm: {0}")]
    Alpm(#[from] alpm::Error),
    #[error("cannot read {path}: {source}")]
    Io { path: String, source: std::io::Error },
}

/// Where pacman keeps its data.
#[derive(Debug, Clone)]
pub struct Paths<'a> {
    pub root: &'a str,
    pub dbpath: &'a str,
    pub config: &'a str,
}

impl Default for Paths<'_> {
    fn default() -> Self {
        Self {
            root: "/",
            dbpath: "/var/lib/pacman/",
            config: "/etc/pacman.conf",
        }
    }
}

/// Repository names in the order pacman.conf lists them.
///
/// Only section headers are read. `Include` lines and options do not change
/// which repositories exist.
pub fn repositories(pacman_conf: &str) -> Vec<String> {
    pacman_conf
        .lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix('[')?.strip_suffix(']'))
        .filter(|name| *name != "options")
        .map(str::to_owned)
        .collect()
}

/// Reads all installed packages and the repository each one belongs to.
pub fn installed(paths: &Paths<'_>) -> Result<PackageSet, Error> {
    let conf = std::fs::read_to_string(paths.config).map_err(|source| Error::Io {
        path: paths.config.to_owned(),
        source,
    })?;
    let handle = Alpm::new(paths.root, paths.dbpath)?;
    for repo in repositories(&conf) {
        // A repository whose database was never downloaded is simply empty.
        if Path::new(paths.dbpath)
            .join("sync")
            .join(format!("{repo}.db"))
            .exists()
        {
            handle.register_syncdb(repo, SigLevel::USE_DEFAULT)?;
        }
    }

    let packages = handle.localdb().pkgs().iter().map(|p| {
        let repo = handle
            .syncdbs()
            .iter()
            .find(|db| db.pkg(p.name()).is_ok())
            .map(|db| db.name().to_owned());
        InstalledPackage {
            name: p.name().to_owned(),
            version: p.version().as_str().to_owned(),
            repo,
            reason: match p.reason() {
                PackageReason::Explicit => InstallReason::Explicit,
                PackageReason::Depend => InstallReason::Dependency,
            },
            depends: p.depends().iter().map(|d| d.name().to_owned()).collect(),
            optdepends: p.optdepends().iter().map(|d| d.name().to_owned()).collect(),
            provides: p.provides().iter().map(|d| d.name().to_owned()).collect(),
            licenses: p.licenses().iter().map(str::to_owned).collect(),
            arch: p.arch().map(str::to_owned),
        }
    });
    Ok(PackageSet::new(packages.collect::<Vec<_>>()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repositories_in_order() {
        let conf = "[options]\nHoldPkg = pacman\n\n[endeavouros]\nInclude = /etc/pacman.d/endeavouros-mirrorlist\n\n[core]\nInclude = x\n# [testing]\n[extra]\n  [multilib]  \n";
        assert_eq!(repositories(conf), ["endeavouros", "core", "extra", "multilib"]);
    }

    /// Reads the real system database. Run with `cargo test -- --ignored` on Arch.
    #[test]
    #[ignore = "needs a real pacman database"]
    fn reads_this_system() {
        let set = installed(&Paths::default()).unwrap();
        assert!(set.get("pacman").is_some());
        assert_eq!(set.get("pacman").unwrap().repo.as_deref(), Some("core"));
    }
}
