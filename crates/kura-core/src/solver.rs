// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Rules that turn a blocked update into a repair proposal.
//!
//! Rules work on classes of problems, not on single cases. Each proposal is
//! simulated with libalpm by the caller before the user sees it.

use std::collections::{BTreeSet, VecDeque};

use crate::package::{InstallReason, PackageSet, dep_name};

/// A dependency that cannot be satisfied, as libalpm reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsatisfied {
    /// The package that has the dependency.
    pub target: String,
    /// The dependency, possibly with a version (`audit=4.1.1`).
    pub dependency: String,
}

/// What a rule proposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repair {
    /// Packages to remove together with the update.
    pub remove: Vec<String>,
    /// The rule or flow that found it, for the details view and problem reports.
    pub rule: String,
}

/// `target` and every installed package that needs it, directly or indirectly.
fn chain<'a>(set: &'a PackageSet, target: &'a str) -> BTreeSet<&'a str> {
    let mut found = BTreeSet::new();
    let mut queue = VecDeque::from([target]);
    while let Some(name) = queue.pop_front() {
        if set.get(name).is_none() || !found.insert(name) {
            continue;
        }
        queue.extend(set.required_by(name));
    }
    found
}

/// Rule "stale chain": an installed package holds another one at an old
/// version and blocks the update. If nothing in its chain was installed on
/// purpose, the chain is removed and everything else is updated.
///
/// Real case: `lib32-audit` required `audit=4.1.1` on `CachyOS`; removing
/// `lib32-audit`, `lib32-pam` and `lib32-libcap` let the update through while
/// `audit`, `pam` and `libcap` stayed.
pub fn stale_chain(problems: &[Unsatisfied], set: &PackageSet) -> Option<Repair> {
    let mut remove: BTreeSet<&str> = BTreeSet::new();
    for p in problems {
        // The blocked dependency must itself be installed: something newer is replacing it.
        set.resolve(dep_name(&p.dependency)).next()?;
        let c = chain(set, &p.target);
        if c.is_empty()
            || c.iter().any(|n| {
                set.get(n)
                    .is_some_and(|pkg| pkg.reason == InstallReason::Explicit)
            })
        {
            return None;
        }
        remove.extend(c);
    }
    if remove.is_empty() {
        return None;
    }
    Some(Repair {
        remove: remove.into_iter().map(str::to_owned).collect(),
        rule: "stale-chain".into(),
    })
}

/// Flows for known cases first, then the built-in rules. The first
/// proposal wins.
pub fn propose(problems: &[Unsatisfied], set: &PackageSet, flows: &[crate::flows::Flow]) -> Option<Repair> {
    flows
        .iter()
        .find_map(|f| crate::flows::propose(f, problems, set))
        .or_else(|| stale_chain(problems, set))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::InstalledPackage;
    use InstallReason::{Dependency, Explicit};

    /// The CachyOS case from the requirements discussion.
    fn cachyos() -> PackageSet {
        PackageSet::new([
            InstalledPackage::new("audit", "4.1.1-1", Dependency),
            InstalledPackage::new("pam", "1.7.0-1", Dependency).with_depends(["audit"]),
            InstalledPackage::new("libcap", "2.76-1", Dependency),
            InstalledPackage::new("lib32-audit", "4.1.1-1", Dependency).with_depends(["audit=4.1.1"]),
            InstalledPackage::new("lib32-pam", "1.7.0-1", Dependency).with_depends(["lib32-audit"]),
            InstalledPackage::new("lib32-libcap", "2.76-1", Dependency).with_depends(["lib32-pam"]),
            InstalledPackage::new("steam", "1.0-1", Explicit).with_depends(["lib32-libcap"]),
        ])
    }

    fn problem() -> Vec<Unsatisfied> {
        vec![Unsatisfied {
            target: "lib32-audit".into(),
            dependency: "audit=4.1.1".into(),
        }]
    }

    #[test]
    fn removes_the_chain_when_nothing_explicit_needs_it() {
        let mut set = cachyos();
        // Without steam, nothing explicit needs the lib32 chain.
        set = PackageSet::new(
            set.iter()
                .filter(|p| p.name != "steam")
                .cloned()
                .collect::<Vec<_>>(),
        );
        let repair = propose(&problem(), &set, &[]).unwrap();
        assert_eq!(repair.remove, ["lib32-audit", "lib32-libcap", "lib32-pam"]);
        assert_eq!(repair.rule, "stale-chain");
    }

    #[test]
    fn keeps_chains_an_app_needs() {
        // steam needs lib32-libcap, so removing the chain would break an app.
        assert_eq!(propose(&problem(), &cachyos(), &[]), None);
    }

    #[test]
    fn needs_an_installed_dependency_and_known_target() {
        let set = PackageSet::new(
            cachyos()
                .iter()
                .filter(|p| p.name != "steam")
                .cloned()
                .collect::<Vec<_>>(),
        );
        let unknown_dep = vec![Unsatisfied {
            target: "lib32-audit".into(),
            dependency: "nothing>=1".into(),
        }];
        assert_eq!(propose(&unknown_dep, &set, &[]), None);
        let unknown_target = vec![Unsatisfied {
            target: "ghost".into(),
            dependency: "audit".into(),
        }];
        assert_eq!(propose(&unknown_target, &set, &[]), None);
        assert_eq!(propose(&[], &set, &[]), None);
    }
}
