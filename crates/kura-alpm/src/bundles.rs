// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Bundles shipped with kura, and plans that install them.

use std::path::Path;

use kura_core::bundles::{self, Bundle};
use kura_core::plan::PlanKind;

use crate::prepare::{Outcome, Request};
use crate::{Config, Error};

/// Where the package installs the bundles.
pub const DIR: &str = "/usr/share/kura/bundles";

/// Every valid bundle in `dir`, sorted by id. Broken files are skipped.
pub fn load(dir: &Path) -> Vec<Bundle> {
    let mut out: Vec<Bundle> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "toml"))
        .filter_map(|e| bundles::parse(&std::fs::read_to_string(e.path()).ok()?).ok())
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

pub fn find(id: &str) -> Option<Bundle> {
    load(Path::new(DIR)).into_iter().find(|b| b.id == id)
}

/// The packages to install: required and recommended ones, plus the chosen
/// optional ones.
pub fn packages(bundle: &Bundle, optional: &[String]) -> Vec<String> {
    bundle
        .packages
        .required
        .iter()
        .chain(&bundle.packages.recommended)
        .chain(
            bundle
                .packages
                .optional
                .iter()
                .map(|o| &o.name)
                .filter(|n| optional.contains(n)),
        )
        .cloned()
        .collect()
}

/// The plan for installing `bundle` for `user`. Repositories it needs are
/// simulated as already on, so the preview shows every package.
pub fn prepare(
    conf: &Config,
    workdir: Option<&Path>,
    bundle: &Bundle,
    user: &str,
    optional: &[String],
) -> Result<Outcome, Error> {
    let active: Vec<String> = conf.repos.iter().map(|r| r.name.clone()).collect();
    let (before, after) = bundles::actions(bundle, &active, user);
    let mut sim = conf.clone();
    for action in &before {
        if let kura_core::action::Action::AddRepo { name, .. } = action {
            let mut repo = conf
                .repos
                .iter()
                .find(|r| r.name == "core")
                .cloned()
                .unwrap_or_default();
            // pacman-conf expanded $repo already: core's servers end in /core/os/….
            repo.servers = repo
                .servers
                .iter()
                .map(|s| s.replace("/core/", &format!("/{name}/")))
                .collect();
            repo.name.clone_from(name);
            sim.repos.push(repo);
        }
    }
    let mut outcome = crate::prepare::prepare(
        &sim,
        workdir,
        &Request::Install(packages(bundle, optional)),
        true,
        &[],
    )?;
    if let Ok(plan) = &mut outcome.plan {
        plan.kind = PlanKind::Bundle;
        plan.actions = before.into_iter().chain(after).collect();
    }
    Ok(outcome)
}
