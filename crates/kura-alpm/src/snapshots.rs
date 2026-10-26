// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Plans that set up snapshots before updates.

use std::path::Path;

use kura_core::plan::{Plan, PlanKind};
use kura_core::snapshots::{self, Report};

use crate::prepare::{Outcome, Request};
use crate::{Config, Error};

/// The plan that adds what the snapshot setup lacks, or `None` when nothing
/// is missing or snapshots are not possible.
///
/// Packages are installed like any other install, with the full system
/// update that comes with it. The actions run afterwards.
pub fn prepare(conf: &Config, workdir: Option<&Path>, refresh: bool) -> Result<Option<Outcome>, Error> {
    let set = crate::installed(conf)?;
    let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
    let Report::Missing(steps) = crate::system::snapshot_report(conf, &set, &mounts) else {
        return Ok(None);
    };
    let handle = crate::handle(conf)?;
    let (packages, actions) =
        snapshots::split(&steps, |p| handle.syncdbs().iter().any(|db| db.pkg(p).is_ok()));
    if packages.is_empty() && actions.is_empty() {
        return Ok(None);
    }
    let mut outcome = if packages.is_empty() {
        Outcome {
            plan: Ok(Plan::new(PlanKind::Snapshots)),
            available: Vec::new(),
            download_size: 0,
            installed_delta: 0,
        }
    } else {
        crate::prepare::prepare(conf, workdir, &Request::Install(packages), refresh, &[])?
    };
    if let Ok(plan) = &mut outcome.plan {
        plan.kind = PlanKind::Snapshots;
        plan.actions = actions;
    }
    Ok(Some(outcome))
}
