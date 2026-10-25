// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Looks for updates and prepares the full system update without running it.
//!
//! Works on a private copy of the sync databases, like pacman-contrib's
//! `checkupdates`: the local database is linked, sync databases are refreshed
//! in kura's own directory. The system's sync databases are never touched, so
//! a later `pacman -S` cannot turn into a partial upgrade because of kura.

use std::fs;
use std::path::Path;

use alpm::{Alpm, PrepareData, SigLevel, TransFlag};
use kura_core::package::PackageSet;
use kura_core::plan::{Context, Plan, PlanKind, RemovalReason, Step};

use crate::conf::Config;
use crate::{Error, Paths};

/// Why libalpm could not prepare the update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// `target` needs `dependency`, which nothing provides any more.
    UnsatisfiedDependency {
        target: String,
        dependency: String,
        causing: Option<String>,
    },
    /// Two packages cannot be installed together.
    Conflict {
        first: String,
        second: String,
        reason: String,
    },
    /// Packages built for another architecture.
    InvalidArchitecture(Vec<String>),
    /// Any other libalpm error, as its message.
    Other(String),
}

/// Result of an update check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The prepared update, or why it could not be prepared.
    pub plan: Result<Plan, Vec<Problem>>,
    /// Packages that have a newer version.
    pub available: Vec<String>,
    /// Bytes still to download (packages already in the cache count as zero).
    pub download_size: u64,
    /// Change of installed size in bytes.
    pub installed_delta: i64,
}

/// Prepares the private database directory: links the real local database
/// and seeds the sync databases from the system's copies when ours are older.
fn prepare_workdir(paths: &Paths<'_>, workdir: &Path) -> Result<(), Error> {
    let io = |path: &Path| {
        let path = path.display().to_string();
        move |source| Error::Io { path, source }
    };
    let sync = workdir.join("sync");
    fs::create_dir_all(&sync).map_err(io(&sync))?;
    let local = workdir.join("local");
    let target = Path::new(paths.dbpath).join("local");
    if fs::read_link(&local).ok().as_deref() != Some(target.as_path()) {
        let _ = fs::remove_file(&local);
        std::os::unix::fs::symlink(&target, &local).map_err(io(&local))?;
    }
    let system_sync = Path::new(paths.dbpath).join("sync");
    if let Ok(entries) = fs::read_dir(&system_sync) {
        for entry in entries.flatten() {
            let from = entry.path();
            if from.extension().and_then(|e| e.to_str()) != Some("db") {
                continue;
            }
            let to = sync.join(entry.file_name());
            let newer = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok();
            if newer(&to) < newer(&from) {
                fs::copy(&from, &to).map_err(io(&to))?;
            }
        }
    }
    Ok(())
}

fn size(n: i64) -> i64 {
    n.max(0)
}

/// Checks for updates. With `refresh`, the private sync databases are
/// downloaded from the configured mirrors first.
pub fn check(paths: &Paths<'_>, config: &Config, workdir: &Path, refresh: bool) -> Result<Outcome, Error> {
    prepare_workdir(paths, workdir)?;
    let dbpath = workdir.to_str().ok_or_else(|| Error::Io {
        path: workdir.display().to_string(),
        source: std::io::Error::other("path is not valid UTF-8"),
    })?;
    let mut handle = Alpm::new(paths.root, dbpath)?;
    for dir in &config.cache_dirs {
        handle.add_cachedir(dir.as_str())?;
    }
    for repo in &config.repos {
        let db = handle.register_syncdb_mut(repo.name.as_str(), SigLevel::USE_DEFAULT)?;
        for server in &repo.servers {
            db.add_server(server.as_str())?;
        }
    }
    if refresh {
        // Downloads all databases in parallel.
        handle.syncdbs_mut().update(false)?;
    }

    let mut available: Vec<String> = handle
        .localdb()
        .pkgs()
        .iter()
        .filter(|p| p.sync_new_version(handle.syncdbs()).is_some())
        .map(|p| p.name().to_owned())
        .collect();
    available.sort();

    handle.trans_init(TransFlag::NONE)?;
    let result = (|| -> Result<Outcome, Error> {
        handle.sync_sysupgrade(false)?;
        if let Err(e) = handle.trans_prepare() {
            let problems = problems(&e);
            return Ok(Outcome {
                plan: Err(problems),
                available: available.clone(),
                download_size: 0,
                installed_delta: 0,
            });
        }
        let mut plan = Plan::new(PlanKind::SystemUpdate);
        let (mut download, mut delta) = (0u64, 0i64);
        for pkg in handle.trans_add() {
            download += u64::try_from(size(pkg.download_size())).unwrap_or(0);
            delta += pkg.isize();
            match handle.localdb().pkg(pkg.name()) {
                Ok(old) => {
                    delta -= old.isize();
                    plan.steps.push(Step::Upgrade {
                        name: pkg.name().to_owned(),
                        from: old.version().as_str().to_owned(),
                        to: pkg.version().as_str().to_owned(),
                    });
                }
                Err(_) => plan.steps.push(Step::Install {
                    name: pkg.name().to_owned(),
                    version: pkg.version().as_str().to_owned(),
                    reason: kura_core::package::InstallReason::Dependency,
                }),
            }
        }
        for pkg in handle.trans_remove() {
            delta -= pkg.isize();
            plan.steps.push(Step::Remove {
                name: pkg.name().to_owned(),
                version: pkg.version().as_str().to_owned(),
                why: RemovalReason::Replaced,
            });
        }
        Ok(Outcome {
            plan: Ok(plan),
            available: available.clone(),
            download_size: download,
            installed_delta: delta,
        })
    })();
    handle.trans_release()?;
    let mut outcome = result?;
    // The rule every plan follows; a violation here would be a bug in kura.
    if let Ok(plan) = &outcome.plan {
        let ctx = Context {
            available_upgrades: outcome.available.iter().cloned().collect(),
            ..Context::default()
        };
        if !plan.steps.is_empty()
            && let Err(e) = plan.validate(&ctx)
        {
            outcome.plan = Err(vec![Problem::Other(e.to_string())]);
        }
    }
    Ok(outcome)
}

fn problems(e: &alpm::PrepareError<'_>) -> Vec<Problem> {
    match e.data() {
        Some(PrepareData::UnsatisfiedDeps(list)) => list
            .iter()
            .map(|m| Problem::UnsatisfiedDependency {
                target: m.target().to_owned(),
                dependency: m.depend().to_string(),
                causing: m.causing_pkg().map(str::to_owned),
            })
            .collect(),
        Some(PrepareData::ConflictingDeps(list)) => list
            .iter()
            .map(|c| Problem::Conflict {
                first: c.package1().name().to_owned(),
                second: c.package2().name().to_owned(),
                reason: c.reason().to_string(),
            })
            .collect(),
        Some(PrepareData::PkgInvalidArch(list)) => {
            vec![Problem::InvalidArchitecture(
                list.iter().map(|p| p.name().to_owned()).collect(),
            )]
        }
        None => vec![Problem::Other(e.to_string())],
    }
}

/// Packages that ship a menu entry (`usr/share/applications/*.desktop`).
/// The preview lists these as apps and everything else as system parts.
pub fn apps(paths: &Paths<'_>) -> Result<std::collections::BTreeSet<String>, Error> {
    let handle = Alpm::new(paths.root, paths.dbpath)?;
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

/// Orphans to add to the update as cleanup, with their versions.
pub fn cleanup(set: &PackageSet) -> Vec<(String, String)> {
    set.orphans()
        .into_iter()
        .filter_map(|n| set.get(n).map(|p| (p.name.clone(), p.version.clone())))
        .collect()
}

#[cfg(test)]
mod tests {
    use kura_core::package::{InstallReason, InstalledPackage};

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
    fn negative_sizes_count_as_zero() {
        assert_eq!(size(-5), 0);
        assert_eq!(size(7), 7);
    }

    /// Downloads databases from the configured mirrors. Runs in CI's Arch
    /// container (`-- --ignored`); do not run it on a machine you care about.
    #[test]
    #[ignore = "needs network and a real pacman installation"]
    fn checks_this_system() {
        let paths = Paths::default();
        let config = crate::conf::parse(&crate::conf::Disk, Path::new(paths.config)).unwrap();
        let dir = std::env::temp_dir().join(format!("kura-check-test-{}", std::process::id()));
        let outcome = check(&paths, &config, &dir, true).unwrap();
        if let Ok(plan) = &outcome.plan {
            let upgrades = plan
                .steps
                .iter()
                .filter(|s| matches!(s, Step::Upgrade { .. }))
                .count();
            assert!(upgrades >= outcome.available.len().saturating_sub(5));
        }
        // A container has no desktop apps; the lookup only has to work.
        apps(&paths).unwrap();
        let _ = fs::remove_dir_all(dir);
    }
}
