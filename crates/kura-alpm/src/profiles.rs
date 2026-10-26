// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Plans that add or take away a repository profile, and the switch of
//! installed packages that comes with it.

use std::path::Path;

use kura_core::plan::{Plan, PlanKind, RemovalReason, Step};
use kura_core::profiles::{self, Level, Profile};

use crate::prepare::{Outcome, Request};
use crate::{Config, Error};

const PACMAN_CONF: &str = "/etc/pacman.conf";

fn pacman_conf() -> String {
    std::fs::read_to_string(PACMAN_CONF).unwrap_or_default()
}

/// The repository set of the profile that is in `pacman.conf` right now.
pub fn active_level(profile: &Profile) -> Option<&'static Level> {
    let conf = pacman_conf();
    profiles::ALL
        .iter()
        .find(|p| p.id == profile.id)
        .and_then(|p| profiles::active_level(p, &conf))
}

pub fn is_active(profile: &Profile) -> bool {
    active_level(profile).is_some()
}

/// The CPU's x86-64 level.
pub fn cpu_level() -> u8 {
    kura_core::hardware::x86_64_level(&std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default())
}

/// The configuration as it will be once the profile's repositories are in
/// place (`add`) or gone. Used to simulate the switch before anything changes.
fn simulated(conf: &Config, profile: &Profile, level: &Level, add: bool) -> Config {
    let mut c = conf.clone();
    let names: Vec<&str> = level.repos.iter().map(|(n, _)| *n).collect();
    c.repos.retain(|r| !names.contains(&r.name.as_str()));
    if add {
        let at = profile
            .before
            .and_then(|b| c.repos.iter().position(|r| r.name == b))
            .unwrap_or(c.repos.len());
        let new: Vec<pacmanconf::Repository> = names
            .iter()
            .map(|n| {
                let mut r = pacmanconf::Repository::default();
                (*n).clone_into(&mut r.name);
                r.servers = profiles::server_for(profile, n).into_iter().collect();
                r
            })
            .collect();
        c.repos.splice(at..at, new);
        if let Some(arch) = level.architecture {
            c.architecture = arch.split_whitespace().map(str::to_owned).collect();
        }
    } else if level.architecture.is_some() {
        c.architecture = vec![std::env::consts::ARCH.to_owned()];
    }
    c
}

/// The plans that add the profile or take it away: first the change of
/// repositories and keys, then, if wanted, an estimate of the switch of
/// installed packages, simulated before anything changes. The estimate is
/// `None` when it cannot be made; the real switch is prepared afterwards.
/// Taking a profile away always switches back.
pub fn prepare(
    conf: &Config,
    workdir: Option<&Path>,
    profile: &'static Profile,
    switch_now: bool,
) -> Result<(Plan, Option<Outcome>), Error> {
    let mut change = Plan::new(PlanKind::Profile);
    if let Some(level) = active_level(profile) {
        change.actions = profiles::remove_actions(profile, level);
        let handle = crate::handle(conf)?;
        for name in profile.bootstrap_packages {
            if let Ok(pkg) = handle.localdb().pkg(*name) {
                change.steps.push(Step::Remove {
                    name: (*name).to_owned(),
                    version: pkg.version().to_string(),
                    why: RemovalReason::Requested,
                });
            }
        }
        drop(handle);
        let switch = if level.architecture.is_some() {
            let sim = simulated(conf, profile, level, false);
            Some(crate::prepare::prepare(
                &sim,
                workdir,
                &Request::Switch(None),
                true,
                &[],
            )?)
        } else {
            None
        };
        return Ok((change, switch));
    }
    let level = profiles::level(profile, cpu_level())
        .ok_or_else(|| Error::Config(format!("{}: this processor is too old", profile.id)))?;
    change.actions = profiles::add_actions(profile, level);
    let switch = if switch_now && level.architecture.is_some() {
        let sim = simulated(conf, profile, level, true);
        let repos = level.repos.iter().map(|(n, _)| (*n).to_owned()).collect();
        Some(crate::prepare::prepare(
            &sim,
            workdir,
            &Request::Switch(Some(repos)),
            true,
            &[],
        )?)
    } else {
        None
    };
    Ok((change, switch))
}
