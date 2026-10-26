// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The AUR through its RPC interface and git's HTTP refs, both with curl.
//! Reads only.

use std::path::Path;
use std::process::Command;

use kura_core::aur::{self, Info, Resolution, Satisfied};
use kura_core::package::InstallReason;
use kura_core::plan::{AurBuild, Plan, PlanKind, Step};

use crate::Config;
use crate::prepare::{Outcome, Problem, Request};

pub const BASE_URL: &str = "https://aur.archlinux.org";

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
        for b in &builds {
            for name in &b.packages {
                plan.steps.push(Step::Install {
                    name: name.clone(),
                    version: b.version.clone(),
                    reason: if targets.contains(name) {
                        InstallReason::Explicit
                    } else {
                        InstallReason::Dependency
                    },
                });
            }
        }
        plan.builds = builds;
    }
    Ok(outcome)
}
