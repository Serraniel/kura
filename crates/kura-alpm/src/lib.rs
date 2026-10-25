// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reads pacman's databases through libalpm and turns them into kura-core types.
//!
//! Never parse pacman's text output: it is translated and changes between
//! versions. Everything goes through libalpm. The configuration comes from
//! `pacman-conf`, which resolves pacman.conf exactly as pacman does.

pub mod apply;
pub mod history;
pub mod prepare;

use std::collections::BTreeSet;

use alpm::{Alpm, PackageReason};
use kura_core::package::{InstallReason, InstalledPackage, PackageSet};
pub use pacmanconf::Config;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("libalpm: {0}")]
    Alpm(#[from] alpm::Error),
    #[error("cannot read the pacman configuration: {0}")]
    Config(String),
    #[error("cannot use {path}: {source}")]
    Io { path: String, source: std::io::Error },
}

/// Reads the system's pacman configuration.
pub fn load_config() -> Result<Config, Error> {
    Config::new().map_err(|e| Error::Config(e.to_string()))
}

/// A libalpm handle set up like pacman would set it up.
pub fn handle(conf: &Config) -> Result<Alpm, Error> {
    let mut alpm = alpm_utils::alpm_with_conf(conf)?;
    // alpm-utils does not apply IgnoreGroup.
    alpm.set_ignoregroups(conf.ignore_group.iter())?;
    alpm.set_hookdirs(hook_dirs(conf).iter())?;
    Ok(alpm)
}

/// Where pacman looks for hooks. pacman always adds its system directory
/// first and the configured `HookDir`s after it; `pacman-conf` lists only the
/// configured ones. Without the system directory, hooks for the initramfs,
/// the boot menu, DKMS and systemd would not run.
pub fn hook_dirs(conf: &Config) -> Vec<String> {
    const SYSTEM_HOOKS: &str = "/usr/share/libalpm/hooks/";
    let mut dirs = vec![SYSTEM_HOOKS.to_owned()];
    dirs.extend(
        conf.hook_dir
            .iter()
            .filter(|d| d.trim_end_matches('/') != SYSTEM_HOOKS.trim_end_matches('/'))
            .cloned(),
    );
    dirs
}

/// Whether this process runs as root.
pub fn is_root() -> bool {
    std::fs::read_to_string("/proc/self/status").is_ok_and(|s| {
        s.lines()
            .find_map(|l| l.strip_prefix("Uid:"))
            .and_then(|v| v.split_whitespace().nth(1))
            == Some("0")
    })
}

/// Reads all installed packages and the repository each one belongs to.
pub fn installed(conf: &Config) -> Result<PackageSet, Error> {
    let handle = handle(conf)?;
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

/// Packages that ship a menu entry (`usr/share/applications/*.desktop`).
/// The preview lists these as apps and everything else as system parts.
pub fn apps(conf: &Config) -> Result<BTreeSet<String>, Error> {
    let handle = handle(conf)?;
    Ok(handle
        .localdb()
        .pkgs()
        .iter()
        .filter(|p| {
            p.files().files().iter().any(|f| {
                let name = f.name();
                name.starts_with(b"usr/share/applications/") && name.ends_with(b".desktop")
            })
        })
        .map(|p| p.name().to_owned())
        .collect())
}

/// Orphans to add to an update as cleanup, with their versions.
pub fn cleanup(set: &PackageSet) -> Vec<(String, String)> {
    set.orphans()
        .into_iter()
        .filter_map(|n| set.get(n).map(|p| (p.name.clone(), p.version.clone())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_lists_orphans_with_versions() {
        let set = PackageSet::new([
            InstalledPackage::new("app", "1-1", InstallReason::Explicit),
            InstalledPackage::new("electron36", "36.9.0-1", InstallReason::Dependency),
        ]);
        assert_eq!(cleanup(&set), [("electron36".to_owned(), "36.9.0-1".to_owned())]);
    }

    #[test]
    fn system_hook_directory_comes_first() {
        let conf = Config {
            hook_dir: vec!["/etc/pacman.d/hooks/".into()],
            ..Config::default()
        };
        assert_eq!(
            hook_dirs(&conf),
            ["/usr/share/libalpm/hooks/", "/etc/pacman.d/hooks/"]
        );
        let twice = Config {
            hook_dir: vec!["/usr/share/libalpm/hooks".into()],
            ..Config::default()
        };
        assert_eq!(hook_dirs(&twice), ["/usr/share/libalpm/hooks/"]);
    }

    #[test]
    fn root_detection_reads_proc() {
        // Only checks that it does not panic; the answer depends on who runs the tests.
        let _ = is_root();
    }

    /// Reads the real system database. Runs in CI's Arch container.
    #[test]
    #[ignore = "needs a real pacman installation"]
    fn reads_this_system() {
        let conf = load_config().unwrap();
        let set = installed(&conf).unwrap();
        assert_eq!(set.get("pacman").unwrap().repo.as_deref(), Some("core"));
        apps(&conf).unwrap();
    }
}
