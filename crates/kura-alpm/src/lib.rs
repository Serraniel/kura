// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reads pacman's databases through libalpm and turns them into kura-core types.
//!
//! Never parse pacman's text output: it is translated and changes between
//! versions. Everything goes through libalpm.

pub mod check;
pub mod conf;

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

/// Reads pacman.conf and the files it includes.
pub fn config(paths: &Paths<'_>) -> Result<conf::Config, Error> {
    conf::parse(&conf::Disk, Path::new(paths.config)).ok_or_else(|| Error::Io {
        path: paths.config.to_owned(),
        source: std::io::Error::from(std::io::ErrorKind::NotFound),
    })
}

/// Reads all installed packages and the repository each one belongs to.
pub fn installed(paths: &Paths<'_>) -> Result<PackageSet, Error> {
    let conf = config(paths)?;
    let handle = Alpm::new(paths.root, paths.dbpath)?;
    for repo in conf.repos.iter().map(|r| r.name.as_str()) {
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
    fn missing_config_is_an_error() {
        let paths = Paths {
            config: "/nonexistent/pacman.conf",
            ..Paths::default()
        };
        assert!(matches!(config(&paths), Err(Error::Io { .. })));
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
