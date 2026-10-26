// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The AUR through its RPC interface and git's HTTP refs, both with curl.
//! Reads only.

use std::path::Path;
use std::process::Command;

use std::collections::BTreeMap;

use kura_core::aur::{self, Info, Maintainer, Resolution, Satisfied, Update};
use kura_core::package::InstallReason;
use kura_core::plan::{AurBuild, Plan, PlanKind, Step};
use kura_core::settings::Settings;

use crate::Config;
use crate::prepare::{Outcome, Problem, Request};

pub const BASE_URL: &str = "https://aur.archlinux.org";
/// When kura first saw each AUR package's current maintainer.
pub const MAINTAINERS: &str = "/var/lib/kura/aur-maintainers.json";

/// The system-wide settings; defaults when the file is missing.
pub fn settings() -> Settings {
    Settings::parse(&std::fs::read_to_string(kura_core::settings::PATH).unwrap_or_default())
}

/// Names per info request; the AUR accepts long lists, this keeps requests small.
const BATCH: usize = 100;

fn curl(args: &[&str]) -> Result<String, String> {
    let out = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--max-time",
            "30",
        ])
        .args(args)
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
    }
}

/// Asks the RPC interface (`/rpc/?v=5&type=…`).
fn rpc(kind: &str, args: &[String]) -> Result<Vec<Info>, String> {
    let url = format!("{BASE_URL}/rpc/");
    let mut all: Vec<String> = vec!["--get".into()];
    for a in ["v=5".to_owned(), format!("type={kind}")].iter().chain(args) {
        all.push("--data-urlencode".into());
        all.push(a.clone());
    }
    all.push(url);
    let all: Vec<&str> = all.iter().map(String::as_str).collect();
    aur::parse(&curl(&all)?)
}

/// Metadata for packages by exact name. Unknown names are left out.
pub fn info(names: &[String]) -> Result<Vec<Info>, String> {
    let mut out = Vec::new();
    for chunk in names.chunks(BATCH) {
        let args: Vec<String> = chunk.iter().map(|n| format!("arg[]={n}")).collect();
        out.extend(rpc("info", &args)?);
    }
    Ok(out)
}

/// Searches names and descriptions (`by = "name-desc"`) or what packages
/// provide (`by = "provides"`).
pub fn search(term: &str, by: &str) -> Result<Vec<Info>, String> {
    rpc("search", &[format!("by={by}"), format!("arg={term}")])
}

/// The commit the package base's `HEAD` points to right now.
pub fn head_commit(base: &str) -> Result<String, String> {
    let url = format!("{BASE_URL}/{base}.git/info/refs?service=git-upload-pack");
    aur::head_commit(&curl(&[&url])?).ok_or_else(|| format!("{base}: no commit found"))
}

/// Infos for `names`. Names that are no package of their own are looked up
/// as something another package provides; the most voted provider wins.
fn fetch_with_providers(names: &[String]) -> Vec<Info> {
    let mut found = info(names).unwrap_or_default();
    let missing: Vec<&String> = names
        .iter()
        .filter(|n| !found.iter().any(|i| &i.name == *n))
        .collect();
    for name in missing {
        let best = search(name, "provides")
            .unwrap_or_default()
            .into_iter()
            .max_by_key(|i| i.num_votes);
        if let Some(best) = best {
            // Search results carry no dependencies; ask again for the full entry.
            found.extend(info(std::slice::from_ref(&best.name)).unwrap_or_default());
        }
    }
    found
}

/// Resolves AUR targets against the installed packages and the repositories.
pub fn resolve(conf: &Config, targets: &[String]) -> Result<Resolution, crate::Error> {
    let handle = crate::handle(conf)?;
    let satisfied = |dep: &str| {
        if handle.localdb().pkgs().find_satisfier(dep).is_some() {
            Satisfied::Installed
        } else if handle.syncdbs().find_satisfier(dep).is_some() {
            Satisfied::Repository
        } else {
            Satisfied::No
        }
    };
    Ok(aur::resolve(targets, fetch_with_providers, satisfied))
}

/// Bases in an order where each comes after everything in its `after`.
fn ordered(res: &Resolution) -> Vec<String> {
    let mut done = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    loop {
        let next: Vec<String> = aur::ready(&res.builds, &done, &done)
            .into_iter()
            .map(str::to_owned)
            .collect();
        if next.is_empty() {
            return out;
        }
        done.extend(next.iter().cloned());
        out.extend(next);
    }
}

/// Plans installing packages from the AUR: the repository dependencies
/// together with the full update, then the AUR package bases, each pinned
/// to the commit it has right now.
///
/// `base-devel` is added when it is missing, because every build needs it.
pub fn prepare_install(
    conf: &Config,
    workdir: Option<&Path>,
    targets: &[String],
    refresh: bool,
) -> Result<Outcome, crate::Error> {
    let blocked = |problems| Outcome {
        plan: Err(problems),
        available: Vec::new(),
        download_size: 0,
        installed_delta: 0,
    };
    let res = resolve(conf, targets)?;
    if !res.missing.is_empty() {
        return Ok(blocked(
            res.missing
                .iter()
                .map(|(by, dep)| {
                    if by.is_empty() {
                        Problem::NotFound { name: dep.clone() }
                    } else {
                        Problem::UnsatisfiedDependency {
                            target: by.clone(),
                            dependency: dep.clone(),
                            causing: None,
                        }
                    }
                })
                .collect(),
        ));
    }
    let circle = aur::cycle(&res.builds);
    if !circle.is_empty() {
        return Ok(blocked(vec![Problem::Other {
            message: format!("AUR packages need each other: {}", circle.join(", ")),
        }]));
    }
    let mut builds = Vec::new();
    for base in ordered(&res) {
        let build = &res.builds[&base];
        let commit = head_commit(&base).map_err(crate::Error::Config)?;
        builds.push(AurBuild {
            base: base.clone(),
            commit,
            version: build.version.clone(),
            packages: build.packages.iter().cloned().collect(),
            after: build.after.iter().cloned().collect(),
        });
    }

    let mut deps: Vec<String> = res.repo_deps.iter().cloned().collect();
    let handle = crate::handle(conf)?;
    if handle.localdb().pkg("base-devel").is_err() {
        deps.push("base-devel".into());
    }
    drop(handle);
    let mut outcome = if deps.is_empty() {
        Outcome {
            plan: Ok(Plan::new(PlanKind::AurInstall)),
            available: Vec::new(),
            download_size: 0,
            installed_delta: 0,
        }
    } else {
        crate::prepare::prepare(conf, workdir, &Request::Dependencies(deps), refresh, &[])?
    };
    if let Ok(plan) = &mut outcome.plan {
        let installed = crate::installed(conf)?;
        for b in &builds {
            for name in &b.packages {
                plan.steps.push(match installed.get(name) {
                    Some(old) => Step::Upgrade {
                        name: name.clone(),
                        from: old.version.clone(),
                        to: b.version.clone(),
                    },
                    None => Step::Install {
                        name: name.clone(),
                        version: b.version.clone(),
                        reason: if targets.contains(name) {
                            InstallReason::Explicit
                        } else {
                            InstallReason::Dependency
                        },
                    },
                });
            }
        }
        plan.builds = builds;
    }
    Ok(outcome)
}

/// Installed packages that no repository has, with their versions.
fn foreign(conf: &Config) -> Result<Vec<(String, String)>, crate::Error> {
    Ok(crate::installed(conf)?
        .iter()
        .filter(|p| p.repo.is_none())
        .map(|p| (p.name.clone(), p.version.clone()))
        .collect())
}

/// AUR updates for installed packages, with their cooling period.
/// With `remember`, maintainer changes are saved (root only).
pub fn updates(conf: &Config, settings: &Settings, now: i64, remember: bool) -> Result<Vec<Update>, String> {
    let installed = foreign(conf).map_err(|e| e.to_string())?;
    if installed.is_empty() {
        return Ok(Vec::new());
    }
    let names: Vec<String> = installed.iter().map(|(n, _)| n.clone()).collect();
    let infos = info(&names)?;
    let known: BTreeMap<String, Maintainer> = std::fs::read_to_string(MAINTAINERS)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let tracked = aur::track_maintainers(&known, &infos, now);
    if remember && let Ok(json) = serde_json::to_string(&tracked) {
        let _ = std::fs::write(MAINTAINERS, json);
    }
    Ok(aur::updates(
        &installed,
        &infos,
        now,
        settings.aur_cooling_days,
        |n| tracked.get(n).map(|m| m.since),
    ))
}

/// Programs and libraries of AUR packages that need a library which is gone,
/// usually because a repository update changed its version. These packages
/// must be built again. Returns package names with the missing libraries.
pub fn needs_rebuild(conf: &Config) -> Result<Vec<(String, Vec<String>)>, crate::Error> {
    use kura_core::elf;
    use std::io::Read as _;

    // The parts kura needs sit at the start of a file; huge files are cut.
    const READ_LIMIT: u64 = 4 * 1024 * 1024;
    let extra: Vec<String> = std::fs::read_dir("/etc/ld.so.conf.d")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .flat_map(|c| elf::ld_conf_dirs(&c))
        .collect();
    let handle = crate::handle(conf)?;
    let foreign: Vec<_> = handle
        .localdb()
        .pkgs()
        .into_iter()
        .filter(|p| !handle.syncdbs().iter().any(|db| db.pkg(p.name()).is_ok()))
        .collect();
    let mut out = Vec::new();
    for pkg in foreign {
        let mut missing: Vec<String> = Vec::new();
        for file in pkg.files().files() {
            let name = String::from_utf8_lossy(file.name()).into_owned();
            let checked = ["usr/bin/", "usr/lib/", "usr/lib32/"].iter().any(|dir| {
                name.strip_prefix(dir)
                    .is_some_and(|rest| !rest.contains('/') && !rest.is_empty())
            });
            if !checked {
                continue;
            }
            let path = format!("/{name}");
            let is_file = std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_file());
            if !is_file {
                continue;
            }
            let mut data = Vec::new();
            let Ok(f) = std::fs::File::open(&path) else {
                continue;
            };
            if f.take(READ_LIMIT).read_to_end(&mut data).is_err() {
                continue;
            }
            let Some(linking) = elf::linking(&data) else {
                continue;
            };
            let origin = path.rsplit_once('/').map_or("/", |(dir, _)| dir);
            for lib in elf::missing(&linking, origin, &extra, |p| Path::new(p).exists()) {
                if !missing.contains(&lib) {
                    missing.push(lib);
                }
            }
        }
        if !missing.is_empty() {
            out.push((pkg.name().to_owned(), missing));
        }
    }
    Ok(out)
}

/// AUR packages to build again, limited to those the AUR still has.
pub fn rebuild_targets(conf: &Config) -> Result<Vec<String>, String> {
    let broken: Vec<String> = needs_rebuild(conf)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    if broken.is_empty() {
        return Ok(broken);
    }
    let known = info(&broken)?;
    Ok(broken
        .into_iter()
        .filter(|n| known.iter().any(|i| &i.name == n))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "reads the real package database"]
    fn rebuild_scan_runs() {
        let conf = crate::load_config().unwrap();
        let found = needs_rebuild(&conf).unwrap();
        eprintln!("{found:?}");
    }
}
