// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Turns a request (update, install, remove) into a plan without changing
//! anything (L1 simulation).
//!
//! For previews this works on a private copy of the sync databases, like
//! pacman-contrib's `checkupdates`: the local database is linked, sync
//! databases are refreshed in kura's own directory. The system's sync
//! databases are never touched, so a later `pacman -S` cannot turn into a
//! partial upgrade because of kura.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use alpm::{Alpm, PrepareData, TransFlag};
use kura_core::locale::Locale;
use kura_core::package::InstallReason;
use kura_core::plan::{Context, Plan, PlanKind, RemovalReason, Step};

use crate::{Config, Error};

/// What the user wants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Upgrade everything.
    Update,
    /// Install these packages. Pending upgrades are included, because
    /// installing against newer databases without upgrading is a partial upgrade.
    Install(Vec<String>),
    /// Remove these packages and what only they needed.
    Remove(Vec<String>),
    /// Upgrade everything and remove these packages, as a repair proposed
    /// by the problem solver.
    Repair(Vec<String>),
    /// Upgrade everything and install what satisfies these dependencies,
    /// marked as dependencies: the repository part of an AUR install.
    Dependencies(Vec<String>),
    /// Upgrade everything and reinstall packages from other builds: with
    /// repositories, generic builds those repositories have (for example
    /// CachyOS's builds for newer processors); without, every package built
    /// for a newer processor goes back to the generic build.
    Switch(Option<Vec<String>>),
}

impl Request {
    pub fn kind(&self) -> PlanKind {
        match self {
            Request::Update => PlanKind::SystemUpdate,
            Request::Install(_) => PlanKind::Install,
            Request::Remove(_) => PlanKind::Remove,
            Request::Repair(_) => PlanKind::Repair,
            Request::Dependencies(_) => PlanKind::AurInstall,
            Request::Switch(target) => PlanKind::Switch {
                back: target.is_none(),
            },
        }
    }
}

/// Why libalpm could not prepare the change.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
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
    InvalidArchitecture { packages: Vec<String> },
    /// A requested package does not exist.
    NotFound { name: String },
    /// An old version is neither cached nor available from the Arch archive.
    NotRestorable { name: String, version: String },
    /// Any other libalpm error, as its message.
    Other { message: String },
}

/// Result of preparing a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The prepared plan, or why it could not be prepared.
    pub plan: Result<Plan, Vec<Problem>>,
    /// Packages that have a newer version.
    pub available: Vec<String>,
    /// Bytes still to download (packages already in the cache count as zero).
    pub download_size: u64,
    /// Change of installed size in bytes.
    pub installed_delta: i64,
}

/// Links the real local database into `workdir` and seeds the sync
/// databases from the system's copies when ours are older.
fn prepare_workdir(real_dbpath: &str, workdir: &Path) -> Result<(), Error> {
    let io = |path: &Path| {
        let path = path.display().to_string();
        move |source| Error::Io { path, source }
    };
    let sync = workdir.join("sync");
    fs::create_dir_all(&sync).map_err(io(&sync))?;
    let local = workdir.join("local");
    let target = Path::new(real_dbpath).join("local");
    if fs::read_link(&local).ok().as_deref() != Some(target.as_path()) {
        let _ = fs::remove_file(&local);
        std::os::unix::fs::symlink(&target, &local).map_err(io(&local))?;
    }
    if let Ok(entries) = fs::read_dir(Path::new(real_dbpath).join("sync")) {
        for entry in entries.flatten() {
            let from = entry.path();
            if from.extension().and_then(|e| e.to_str()) != Some("db") {
                continue;
            }
            let to = sync.join(entry.file_name());
            let modified = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok();
            if modified(&to) < modified(&from) {
                fs::copy(&from, &to).map_err(io(&to))?;
            }
        }
    }
    Ok(())
}

/// Opens a handle for a preview. With `workdir`, a private database copy is
/// used; without, the real databases (only the helper does that).
pub(crate) fn open(conf: &Config, workdir: Option<&Path>) -> Result<Alpm, Error> {
    let Some(workdir) = workdir else {
        return crate::handle(conf);
    };
    prepare_workdir(&conf.db_path, workdir)?;
    let mut private = conf.clone();
    private.db_path = workdir.to_str().map(str::to_owned).ok_or_else(|| Error::Io {
        path: workdir.display().to_string(),
        source: std::io::Error::other("path is not valid UTF-8"),
    })?;
    if !crate::is_root() {
        // Switching to the download user needs root.
        private.download_user = None;
    }
    crate::handle(&private)
}

/// Prepares a request: optionally refreshes the databases, then builds the plan.
///
/// `locales` are the user's languages: installs get matching language packs,
/// and where several packages could satisfy a dependency, the one for the
/// user's language is chosen. The helper passes none, because an approved
/// plan already names every package.
pub fn prepare(
    conf: &Config,
    workdir: Option<&Path>,
    request: &Request,
    refresh: bool,
    locales: &[Locale],
) -> Result<Outcome, Error> {
    let mut handle = open(conf, workdir)?;
    if refresh {
        // Downloads all databases in parallel.
        handle.syncdbs_mut().update(false)?;
    }
    let available = available_upgrades(&handle);
    let request = with_language_packs(&handle, request, locales);
    let chosen = locales.to_vec();
    handle.set_question_cb(chosen, |q, locales: &mut Vec<Locale>| {
        if let alpm::Question::SelectProvider(mut s) = q.question() {
            let names: Vec<&str> = s.providers().iter().map(|p| p.name()).collect();
            if let Some(pick) = kura_core::langpacks::pick_provider(&names, locales)
                && let Some(i) = names.iter().position(|n| *n == pick)
            {
                s.set_index(i32::try_from(i).unwrap_or(0));
            }
        }
    });
    handle.trans_init(flags(&request))?;
    let built = build(&mut handle, &request, &available);
    handle.trans_release()?;
    let built = built?;
    Ok(Outcome {
        plan: built.plan,
        available: available.into_iter().collect(),
        download_size: built.download_size,
        installed_delta: built.installed_delta,
    })
}

/// Prepares a request; when an update is blocked, asks the problem solver for
/// a repair and simulates it. Returns the outcome to show and the repair, if
/// one was used.
pub fn prepare_with_repair(
    conf: &Config,
    workdir: Option<&Path>,
    request: &Request,
    refresh: bool,
    locales: &[Locale],
) -> Result<(Outcome, Option<kura_core::solver::Repair>), Error> {
    let outcome = prepare(conf, workdir, request, refresh, locales)?;
    let Err(problems) = &outcome.plan else {
        return Ok((outcome, None));
    };
    if !matches!(request, Request::Update) {
        return Ok((outcome, None));
    }
    let unsatisfied: Vec<kura_core::solver::Unsatisfied> = problems
        .iter()
        .filter_map(|p| match p {
            Problem::UnsatisfiedDependency {
                target, dependency, ..
            } => Some(kura_core::solver::Unsatisfied {
                target: target.clone(),
                dependency: dependency.clone(),
            }),
            _ => None,
        })
        .collect();
    if unsatisfied.len() != problems.len() {
        return Ok((outcome, None));
    }
    let installed = crate::installed(conf)?;
    let handle = open(conf, workdir)?;
    let available = |n: &str| handle.syncdbs().iter().any(|db| db.pkg(n).is_ok());
    let proposal = kura_core::solver::propose(&unsatisfied, &installed, &load_flows(), available);
    drop(handle);
    let Some(repair) = proposal else {
        return Ok((outcome, None));
    };
    // L1 simulation of the proposal; if it does not work, the original problem stands.
    let mut repaired = prepare(
        conf,
        workdir,
        &Request::Repair(repair.remove.clone()),
        false,
        locales,
    )?;
    if let Ok(plan) = &mut repaired.plan {
        for step in &mut plan.steps {
            if let Step::Remove { name, why, .. } = step
                && repair.reinstall.contains(name)
            {
                *why = RemovalReason::Reinstall;
            }
        }
    }
    if repaired.plan.is_ok() {
        Ok((repaired, Some(repair)))
    } else {
        Ok((outcome, None))
    }
}

/// Adds the language pack for each requested package that has one in the
/// repositories and is not installed yet.
fn with_language_packs(handle: &Alpm, request: &Request, locales: &[Locale]) -> Request {
    let Request::Install(names) = request else {
        return request.clone();
    };
    let exists =
        |n: &str| handle.localdb().pkg(n).is_err() && handle.syncdbs().iter().any(|db| db.pkg(n).is_ok());
    let mut all = names.clone();
    for name in names {
        if let Some(pack) = kura_core::langpacks::for_package(name, locales, exists)
            && !all.contains(&pack)
        {
            all.push(pack);
        }
    }
    Request::Install(all)
}

/// A build for any x86-64 processor, as Arch builds them.
fn is_generic(pkg: &alpm::Package) -> bool {
    matches!(pkg.arch(), Some("x86_64" | "any")) && !pkg.packager().unwrap_or_default().contains("CachyOS")
}

/// Adds the reinstalls of a switch to the transaction (see [`Request::Switch`]).
fn add_switches(handle: &Alpm, target: Option<&[String]>) {
    for old in handle.localdb().pkgs() {
        let reinstall = match target {
            Some(repos) => {
                is_generic(old)
                    && handle
                        .syncdbs()
                        .iter()
                        .find(|db| db.pkg(old.name()).is_ok())
                        .is_some_and(|db| repos.iter().any(|r| r == db.name()))
            }
            None => !is_generic(old),
        };
        if !reinstall || handle.trans_add().iter().any(|p| p.name() == old.name()) {
            continue;
        }
        if let Some(new) = handle.syncdbs().iter().find_map(|db| db.pkg(old.name()).ok()) {
            // A package that cannot be added keeps its build; the switch goes on.
            let _ = handle.trans_add_pkg(new);
        }
    }
}

/// Adds packages from the repositories, by name or, with `satisfy`, by
/// dependency (`pacman>6.1`). Returns the problem when one cannot be added.
fn add_installs(handle: &Alpm, names: &[String], satisfy: bool) -> Option<Problem> {
    for name in names {
        let found = if satisfy {
            handle.syncdbs().find_satisfier(name.as_str())
        } else {
            handle.syncdbs().iter().find_map(|db| db.pkg(name.as_str()).ok())
        };
        let Some(pkg) = found else {
            return Some(Problem::NotFound { name: name.clone() });
        };
        // Already part of the update, or asked for twice.
        if satisfy && handle.trans_add().iter().any(|p| p.name() == pkg.name()) {
            continue;
        }
        if let Err(e) = handle.trans_add_pkg(pkg) {
            return Some(Problem::Other {
                message: e.to_string(),
            });
        }
    }
    None
}

/// Marks `names` for removal. Returns the problem if one is not installed.
fn add_removals<'a>(
    handle: &Alpm,
    names: &'a [String],
    explicit: &mut BTreeSet<&'a str>,
) -> Result<Option<Problem>, Error> {
    for name in names {
        let Ok(pkg) = handle.localdb().pkg(name.as_str()) else {
            return Ok(Some(Problem::NotFound { name: name.clone() }));
        };
        handle.trans_remove_pkg(pkg)?;
        explicit.insert(name);
    }
    Ok(None)
}

/// Installed language packs that depend on one of `names`.
fn packs_of<'h>(handle: &'h Alpm, names: &[String]) -> Vec<&'h alpm::Package> {
    handle
        .localdb()
        .pkgs()
        .into_iter()
        .filter(|p| !names.iter().any(|n| n == p.name()))
        .filter(|p| {
            p.depends().into_iter().any(|d| {
                names
                    .iter()
                    .any(|n| n == d.name() && kura_core::langpacks::is_pack_of(n, p.name()))
            })
        })
        .collect()
}

/// Where the package installs the flows.
pub const FLOWS: &str = "/usr/share/kura/flows";

/// Every valid flow shipped with kura. Broken files are skipped.
pub fn load_flows() -> Vec<kura_core::flows::Flow> {
    std::fs::read_dir(FLOWS)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "toml"))
        .filter_map(|e| kura_core::flows::parse(&std::fs::read_to_string(e.path()).ok()?).ok())
        .collect()
}

pub(crate) fn flags(request: &Request) -> TransFlag {
    match request {
        // Like `pacman -Rs`: also remove dependencies nothing else needs.
        Request::Remove(_) => TransFlag::RECURSE,
        // Like `pacman -S --asdeps`.
        Request::Dependencies(_) => TransFlag::ALL_DEPS,
        _ => TransFlag::NONE,
    }
}

pub(crate) fn available_upgrades(handle: &Alpm) -> BTreeSet<String> {
    handle
        .localdb()
        .pkgs()
        .iter()
        .filter(|p| p.sync_new_version(handle.syncdbs()).is_some())
        .map(|p| p.name().to_owned())
        .collect()
}

pub(crate) struct Built {
    pub plan: Result<Plan, Vec<Problem>>,
    pub download_size: u64,
    pub installed_delta: i64,
}

/// Adds the request to an initialised transaction, prepares it and reads the
/// result back as a plan. The transaction stays open for the caller.
pub(crate) fn build(
    handle: &mut Alpm,
    request: &Request,
    available: &BTreeSet<String>,
) -> Result<Built, Error> {
    let blocked = |problems| Built {
        plan: Err(problems),
        download_size: 0,
        installed_delta: 0,
    };
    let mut explicit: BTreeSet<&str> = BTreeSet::new();
    match request {
        Request::Update => handle.sync_sysupgrade(false)?,
        Request::Install(names) => {
            handle.sync_sysupgrade(false)?;
            if let Some(problem) = add_installs(handle, names, false) {
                return Ok(blocked(vec![problem]));
            }
            explicit.extend(names.iter().map(String::as_str));
        }
        Request::Switch(target) => {
            // Going back may mean older versions than the optimised builds.
            handle.sync_sysupgrade(target.is_none())?;
            add_switches(handle, target.as_deref());
        }
        Request::Dependencies(deps) => {
            handle.sync_sysupgrade(false)?;
            if let Some(problem) = add_installs(handle, deps, true) {
                return Ok(blocked(vec![problem]));
            }
        }
        Request::Repair(names) => {
            handle.sync_sysupgrade(false)?;
            if let Some(problem) = add_removals(handle, names, &mut explicit)? {
                return Ok(blocked(vec![problem]));
            }
        }
        Request::Remove(names) => {
            if let Some(problem) = add_removals(handle, names, &mut explicit)? {
                return Ok(blocked(vec![problem]));
            }
            // Language packs that need a removed package go with it.
            for pkg in packs_of(handle, names) {
                handle.trans_remove_pkg(pkg)?;
            }
        }
    }
    if let Err(e) = handle.trans_prepare() {
        return Ok(blocked(problems(&e)));
    }

    let mut plan = Plan::new(request.kind());
    let (mut download, mut delta) = (0u64, 0i64);
    for pkg in handle.trans_add() {
        download += u64::try_from(pkg.download_size().max(0)).unwrap_or(0);
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
                reason: if explicit.contains(pkg.name()) {
                    InstallReason::Explicit
                } else {
                    InstallReason::Dependency
                },
            }),
        }
    }
    for pkg in handle.trans_remove() {
        delta -= pkg.isize();
        let why = match request {
            Request::Remove(_) if explicit.contains(pkg.name()) => RemovalReason::Requested,
            Request::Remove(_) => RemovalReason::Orphan,
            Request::Repair(_) if explicit.contains(pkg.name()) => RemovalReason::Repair,
            _ => RemovalReason::Replaced,
        };
        plan.steps.push(Step::Remove {
            name: pkg.name().to_owned(),
            version: pkg.version().as_str().to_owned(),
            why,
        });
    }

    // The rule every plan follows; a violation here would be a bug in kura.
    let ctx = Context {
        available_upgrades: available.clone(),
        ..Context::default()
    };
    let plan = match plan.validate(&ctx) {
        Ok(()) | Err(kura_core::plan::PlanError::Empty) => Ok(plan),
        Err(e) => Err(vec![Problem::Other {
            message: e.to_string(),
        }]),
    };
    Ok(Built {
        plan,
        download_size: download,
        installed_delta: delta,
    })
}

pub(crate) fn problems(e: &alpm::PrepareError<'_>) -> Vec<Problem> {
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
        // libalpm fills this list with strings, while the alpm crate types it
        // as packages; reading it as packages crashes (issue #96).
        Some(PrepareData::PkgInvalidArch(_)) => {
            vec![Problem::InvalidArchitecture { packages: Vec::new() }]
        }
        None => vec![Problem::Other {
            message: e.to_string(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_kinds_and_flags() {
        assert_eq!(Request::Update.kind(), PlanKind::SystemUpdate);
        assert_eq!(Request::Install(vec![]).kind(), PlanKind::Install);
        assert_eq!(Request::Remove(vec![]).kind(), PlanKind::Remove);
        assert_eq!(flags(&Request::Remove(vec![])), TransFlag::RECURSE);
        assert_eq!(flags(&Request::Update), TransFlag::NONE);
    }

    #[test]
    fn workdir_links_local_and_copies_sync() {
        let base = std::env::temp_dir().join(format!("kura-prepare-test-{}", std::process::id()));
        let real = base.join("real");
        fs::create_dir_all(real.join("local")).unwrap();
        fs::create_dir_all(real.join("sync")).unwrap();
        fs::write(real.join("sync/core.db"), b"db").unwrap();
        fs::write(real.join("sync/core.db.sig"), b"sig").unwrap();
        let work = base.join("work");
        prepare_workdir(real.to_str().unwrap(), &work).unwrap();
        assert_eq!(fs::read_link(work.join("local")).unwrap(), real.join("local"));
        assert_eq!(fs::read(work.join("sync/core.db")).unwrap(), b"db");
        assert!(!work.join("sync/core.db.sig").exists());
        // A second run keeps the link and does not fail.
        prepare_workdir(real.to_str().unwrap(), &work).unwrap();
        fs::remove_dir_all(base).unwrap();
    }
}
