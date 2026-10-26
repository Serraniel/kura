// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Flows: fixes for known update problems, described in TOML files that
//! ship with kura. A flow matches a typed libalpm problem and the installed
//! versions, and proposes packages to remove. Like every repair, the
//! proposal is simulated before anyone sees it.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::Deserialize;

use crate::bundles::Texts;
use crate::package::{InstallReason, PackageSet, dep_name};
use crate::solver::{Repair, Unsatisfied};
use crate::version::vercmp;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Match {
    /// Only `unsatisfied-dependency` exists so far.
    pub error: String,
    /// Dependencies that cannot be satisfied (names without version).
    #[serde(default)]
    pub blocked: Vec<String>,
    /// Installed packages with a version condition: `*`, `<1.2-1`, `>=2`…
    #[serde(default)]
    pub installed: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FlowRepair {
    pub remove: Vec<String>,
    /// Packages that must stay; a flow removing one of them is invalid.
    #[serde(default)]
    pub keep: Vec<String>,
    /// Stop when something installed on purpose needs a removed package.
    #[serde(default)]
    pub only_if_unneeded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flow {
    pub id: String,
    pub version: u32,
    pub name: Texts,
    pub description: Texts,
    #[serde(rename = "match")]
    pub matches: Match,
    pub repair: FlowRepair,
}

/// Reads a flow file and checks it.
pub fn parse(text: &str) -> Result<Flow, String> {
    let flow: Flow = toml::from_str(text).map_err(|e| e.to_string())?;
    if flow.matches.error != "unsatisfied-dependency" {
        return Err(format!("{}: unknown error type {}", flow.id, flow.matches.error));
    }
    if let Some(p) = flow.repair.remove.iter().find(|p| flow.repair.keep.contains(p)) {
        return Err(format!("{}: {p} is both removed and kept", flow.id));
    }
    if !flow.name.0.contains_key("en") || !flow.description.0.contains_key("en") {
        return Err(format!("{}: English name and description are required", flow.id));
    }
    Ok(flow)
}

/// Whether `version` meets a condition such as `*`, `<4.1.4-2` or `>=1`.
pub fn version_matches(version: &str, condition: &str) -> bool {
    let condition = condition.trim();
    if condition == "*" {
        return true;
    }
    let (op, wanted) = ["<=", ">=", "<", ">", "="]
        .iter()
        .find_map(|op| condition.strip_prefix(op).map(|v| (*op, v.trim())))
        .unwrap_or(("=", condition));
    let order = vercmp(version, wanted);
    match op {
        "<=" => order != Ordering::Greater,
        ">=" => order != Ordering::Less,
        "<" => order == Ordering::Less,
        ">" => order == Ordering::Greater,
        _ => order == Ordering::Equal,
    }
}

/// The repair a flow proposes for these problems, or `None` when it does
/// not match or must not run here.
pub fn propose(flow: &Flow, problems: &[Unsatisfied], set: &PackageSet) -> Option<Repair> {
    if problems.is_empty() {
        return None;
    }
    let blocked_ok = flow
        .matches
        .blocked
        .iter()
        .all(|b| problems.iter().any(|p| dep_name(&p.dependency) == b));
    let installed_ok = flow
        .matches
        .installed
        .iter()
        .all(|(name, cond)| set.get(name).is_some_and(|p| version_matches(&p.version, cond)));
    if !blocked_ok || !installed_ok {
        return None;
    }
    let remove: Vec<String> = flow
        .repair
        .remove
        .iter()
        .filter(|p| set.get(p).is_some())
        .cloned()
        .collect();
    if remove.is_empty() || flow.repair.keep.iter().any(|k| set.get(k).is_none()) {
        return None;
    }
    if flow.repair.only_if_unneeded {
        let needed = remove.iter().any(|name| {
            set.get(name).is_some_and(|p| p.reason == InstallReason::Explicit)
                || set
                    .required_by(name)
                    .iter()
                    .any(|by| !remove.iter().any(|r| r == by))
        });
        if needed {
            return None;
        }
    }
    Some(Repair {
        remove,
        rule: flow.id.clone(),
        reinstall: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::InstalledPackage;
    use InstallReason::{Dependency, Explicit};

    const FLOW: &str = r#"
id = "lib32-audit-blocks-audit-update"
version = 1
name = { en = "Remove outdated 32-bit system libraries" }
description = { en = "They block the update." }

[match]
error = "unsatisfied-dependency"
blocked = ["audit"]
installed = { lib32-audit = "<4.1.4-2" }

[repair]
remove = ["lib32-audit", "lib32-pam", "lib32-libcap"]
keep = ["audit", "pam", "libcap"]
only-if-unneeded = true
"#;

    fn system(with_steam: bool) -> PackageSet {
        let mut list = vec![
            InstalledPackage::new("audit", "4.1.1-1", Dependency),
            InstalledPackage::new("pam", "1.7.0-1", Dependency).with_depends(["audit"]),
            InstalledPackage::new("libcap", "2.76-1", Dependency),
            InstalledPackage::new("lib32-audit", "4.1.1-1", Dependency).with_depends(["audit=4.1.1"]),
            InstalledPackage::new("lib32-pam", "1.7.0-1", Dependency).with_depends(["lib32-audit"]),
            InstalledPackage::new("lib32-libcap", "2.76-1", Dependency).with_depends(["lib32-pam"]),
        ];
        if with_steam {
            list.push(InstalledPackage::new("steam", "1.0-1", Explicit).with_depends(["lib32-libcap"]));
        }
        PackageSet::new(list)
    }

    fn problem() -> Vec<Unsatisfied> {
        vec![Unsatisfied {
            target: "lib32-audit".into(),
            dependency: "audit=4.1.1".into(),
        }]
    }

    #[test]
    fn matches_the_known_case() {
        let flow = parse(FLOW).unwrap();
        let repair = propose(&flow, &problem(), &system(false)).unwrap();
        assert_eq!(repair.remove, ["lib32-audit", "lib32-pam", "lib32-libcap"]);
        assert_eq!(repair.rule, "lib32-audit-blocks-audit-update");
        // Steam needs the 32-bit libraries: the flow stays out.
        assert_eq!(propose(&flow, &problem(), &system(true)), None);
        // Other problems or newer versions do not match.
        let other = vec![Unsatisfied {
            target: "x".into(),
            dependency: "glibc>=3".into(),
        }];
        assert_eq!(propose(&flow, &other, &system(false)), None);
        let newer = PackageSet::new(
            system(false)
                .iter()
                .map(|p| {
                    let mut p = p.clone();
                    if p.name == "lib32-audit" {
                        p.version = "4.1.4-2".into();
                    }
                    p
                })
                .collect::<Vec<_>>(),
        );
        assert_eq!(propose(&flow, &problem(), &newer), None);
        assert_eq!(propose(&flow, &[], &system(false)), None);
    }

    #[test]
    fn checks_flow_files() {
        assert!(parse(&FLOW.replace("unsatisfied-dependency", "conflict")).is_err());
        assert!(parse(&FLOW.replace("keep = [\"audit\"", "keep = [\"lib32-pam\"")).is_err());
        assert!(parse(&FLOW.replace("only-if-unneeded = true", "run = \"x\"")).is_err());
    }

    #[test]
    fn version_conditions() {
        assert!(version_matches("1.0-1", "*"));
        assert!(version_matches("4.1.1-1", "<4.1.4-2"));
        assert!(!version_matches("4.1.4-2", "<4.1.4-2"));
        assert!(version_matches("4.1.4-2", "<=4.1.4-2"));
        assert!(version_matches("2", ">=2"));
        assert!(version_matches("3", ">2"));
        assert!(version_matches("2", "=2"));
        assert!(version_matches("2", "2"));
    }

    #[test]
    fn shipped_flows_are_valid() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/flows");
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let text = std::fs::read_to_string(entry.path()).unwrap();
            let flow = parse(&text).unwrap_or_else(|e| panic!("{}: {e}", entry.path().display()));
            assert_eq!(entry.path().file_stem().unwrap().to_str(), Some(flow.id.as_str()));
        }
    }
}
