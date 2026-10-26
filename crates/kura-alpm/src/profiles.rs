// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Plans that add or take away a repository profile.

use kura_core::plan::{Plan, PlanKind, RemovalReason, Step};
use kura_core::profiles::{self, Profile};

use crate::{Config, Error};

const PACMAN_CONF: &str = "/etc/pacman.conf";

/// Whether the profile is in `pacman.conf` right now.
pub fn is_active(profile: &Profile) -> bool {
    profiles::is_active(profile, &std::fs::read_to_string(PACMAN_CONF).unwrap_or_default())
}

/// The plan that adds the profile, or takes it away when it is there.
pub fn prepare(conf: &Config, profile: &Profile) -> Result<Plan, Error> {
    let mut plan = Plan::new(PlanKind::Profile);
    if is_active(profile) {
        plan.actions = profiles::remove_actions(profile);
        let handle = crate::handle(conf)?;
        for name in profile.bootstrap_packages {
            if let Ok(pkg) = handle.localdb().pkg(*name) {
                plan.steps.push(Step::Remove {
                    name: (*name).to_owned(),
                    version: pkg.version().to_string(),
                    why: RemovalReason::Requested,
                });
            }
        }
    } else {
        plan.actions = profiles::add_actions(profile);
    }
    Ok(plan)
}
