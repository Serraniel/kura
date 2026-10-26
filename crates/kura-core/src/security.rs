// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Known vulnerabilities from the Arch Linux Security Tracker, matched against
//! the installed packages and reported by the apps the user knows.
//!
//! A vulnerable library is reported as the explicitly installed apps that need
//! it ("Kdenlive"), or as "System" when only system parts do. The library
//! itself stays available in the details.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::package::PackageSet;
use crate::version::vercmp;

/// One advisory group (AVG) from <https://security.archlinux.org/issues/all.json>.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Advisory {
    pub name: String,
    pub packages: Vec<String>,
    pub status: String,
    pub severity: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub affected: String,
    pub fixed: Option<String>,
    #[serde(default)]
    pub issues: Vec<String>,
}

pub fn parse_tracker(json: &str) -> Result<Vec<Advisory>, serde_json::Error> {
    serde_json::from_str(json)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "critical" => Severity::Critical,
            "high" => Severity::High,
            "medium" => Severity::Medium,
            _ => Severity::Low,
        }
    }
}

/// A vulnerability that affects this computer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub advisory: String,
    /// The installed package that is vulnerable.
    pub package: String,
    pub installed: String,
    /// Version that fixes it; `None` while no fix exists.
    pub fixed: Option<String>,
    pub severity: Severity,
    /// What an attacker could do, as the tracker words it.
    pub kind: String,
    pub cves: Vec<String>,
    /// Explicitly installed packages that need the vulnerable one.
    pub apps: Vec<String>,
}

impl Finding {
    /// A fix is released and the update installs it.
    pub fn has_fix(&self) -> bool {
        self.fixed.is_some()
    }
}

/// Whether an installed version is affected by an advisory.
fn affects(advisory: &Advisory, installed: &str) -> bool {
    match advisory.status.as_str() {
        "Vulnerable" => vercmp(installed, &advisory.affected) != Ordering::Less,
        "Fixed" => advisory
            .fixed
            .as_deref()
            .is_some_and(|f| vercmp(installed, f) == Ordering::Less),
        _ => false,
    }
}

/// Matches advisories against installed packages. Most severe first.
pub fn scan(advisories: &[Advisory], installed: &PackageSet) -> Vec<Finding> {
    let mut findings = Vec::new();
    for adv in advisories {
        for name in &adv.packages {
            let Some(pkg) = installed.get(name) else { continue };
            // Packages from other sources (AUR, other distributions) are not tracked.
            if !matches!(pkg.repo.as_deref(), Some("core" | "extra" | "multilib")) {
                continue;
            }
            if !affects(adv, &pkg.version) {
                continue;
            }
            let fixed = if adv.status == "Fixed" {
                adv.fixed.clone()
            } else {
                None
            };
            findings.push(Finding {
                advisory: adv.name.clone(),
                package: name.clone(),
                installed: pkg.version.clone(),
                fixed,
                severity: Severity::parse(&adv.severity),
                kind: adv.kind.clone(),
                cves: adv.issues.clone(),
                apps: installed
                    .explicit_users(name)
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            });
        }
    }
    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| a.package.cmp(&b.package))
    });
    findings
}

/// The apps named in findings, for one short sentence. "System" findings have no apps.
pub fn affected_apps(findings: &[Finding]) -> BTreeSet<&str> {
    findings
        .iter()
        .flat_map(|f| f.apps.iter().map(String::as_str))
        .collect()
}

/// Findings the plan fixes: it upgrades the package to the fixed version
/// or a newer one.
pub fn fixed_by<'a>(findings: &'a [Finding], plan: &crate::plan::Plan) -> Vec<&'a Finding> {
    findings
        .iter()
        .filter(|f| {
            let Some(fixed) = &f.fixed else { return false };
            plan.steps.iter().any(|s| match s {
                crate::plan::Step::Upgrade { name, to, .. } => {
                    name == &f.package && crate::version::vercmp(to, fixed) != std::cmp::Ordering::Less
                }
                _ => false,
            })
        })
        .collect()
}

/// Whether kura may install the update on its own: the user turned it on,
/// the update fixes a known vulnerability, the simulation found no problem
/// and no Arch news item concerns it.
pub fn auto_install(enabled: bool, status: &crate::status::Status, fixes: usize) -> bool {
    enabled && fixes > 0 && !status.blocked && status.news.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::{InstallReason, InstalledPackage};

    const TRACKER: &str = r#"[
      {"name": "AVG-1", "packages": ["ffmpeg"], "status": "Fixed", "severity": "High",
       "type": "arbitrary code execution", "affected": "2:8.0.1-1", "fixed": "2:8.0.2-1",
       "ticket": null, "issues": ["CVE-2026-0001"]},
      {"name": "AVG-2", "packages": ["openssl"], "status": "Vulnerable", "severity": "Critical",
       "type": "denial of service", "affected": "3.5.0-1", "fixed": null, "issues": ["CVE-2026-0002", "CVE-2026-0003"]},
      {"name": "AVG-3", "packages": ["gimp"], "status": "Fixed", "severity": "Medium",
       "type": "information disclosure", "affected": "2.0-1", "fixed": "2.1-1", "issues": []},
      {"name": "AVG-4", "packages": ["vlc"], "status": "Not affected", "severity": "Low",
       "type": "unknown", "affected": "1-1", "fixed": null, "issues": []},
      {"name": "AVG-5", "packages": ["yay-bin"], "status": "Vulnerable", "severity": "High",
       "type": "unknown", "affected": "1-1", "fixed": null, "issues": []}
    ]"#;

    fn system() -> PackageSet {
        PackageSet::new([
            InstalledPackage::new("kdenlive", "26.08.1-1", InstallReason::Explicit)
                .with_repo("extra")
                .with_depends(["ffmpeg"]),
            InstalledPackage::new("vlc", "3.0.23-1", InstallReason::Explicit)
                .with_repo("extra")
                .with_depends(["ffmpeg"]),
            InstalledPackage::new("ffmpeg", "2:8.0.1-3", InstallReason::Dependency).with_repo("extra"),
            InstalledPackage::new("openssl", "3.5.1-1", InstallReason::Dependency).with_repo("core"),
            InstalledPackage::new("gimp", "3.0.6-1", InstallReason::Explicit).with_repo("extra"),
            InstalledPackage::new("yay-bin", "13.0.1-1", InstallReason::Explicit),
        ])
    }

    #[test]
    fn finds_vulnerable_packages_and_names_the_apps() {
        let advisories = parse_tracker(TRACKER).unwrap();
        let findings = scan(&advisories, &system());
        let names: Vec<_> = findings
            .iter()
            .map(|f| (f.advisory.as_str(), f.package.as_str()))
            .collect();
        // Critical first; gimp is newer than the fix; vlc is not affected; AUR is not tracked.
        assert_eq!(names, [("AVG-2", "openssl"), ("AVG-1", "ffmpeg")]);

        let ffmpeg = &findings[1];
        assert_eq!(ffmpeg.apps, ["kdenlive", "vlc"]);
        assert_eq!(ffmpeg.fixed.as_deref(), Some("2:8.0.2-1"));
        assert!(ffmpeg.has_fix());
        assert_eq!(ffmpeg.severity, Severity::High);

        let openssl = &findings[0];
        assert!(openssl.apps.is_empty(), "only system parts need openssl");
        assert!(!openssl.has_fix());
        assert_eq!(openssl.cves.len(), 2);

        assert_eq!(
            affected_apps(&findings).into_iter().collect::<Vec<_>>(),
            ["kdenlive", "vlc"]
        );
    }

    #[test]
    fn versions_older_than_affected_are_safe() {
        let mut advisories = parse_tracker(TRACKER).unwrap();
        advisories[1].affected = "3.6.0-1".into();
        let findings = scan(&advisories, &system());
        assert!(findings.iter().all(|f| f.package != "openssl"));
    }

    #[test]
    fn severity_order() {
        assert!(Severity::Critical > Severity::High);
        assert_eq!(Severity::parse("Unknown"), Severity::Low);
        assert_eq!(Severity::parse("MEDIUM"), Severity::Medium);
        assert_eq!(serde_json::to_string(&Severity::High).unwrap(), "\"high\"");
    }

    #[test]
    fn broken_json_is_an_error() {
        assert!(parse_tracker("[{").is_err());
    }

    #[test]
    fn automatic_installs_need_a_fix_and_a_clean_update() {
        use crate::plan::{Plan, PlanKind, Step};
        let finding = |package: &str, fixed: Option<&str>| Finding {
            advisory: "AVG-1".into(),
            package: package.into(),
            installed: "1.0-1".into(),
            fixed: fixed.map(Into::into),
            severity: Severity::High,
            kind: "arbitrary code execution".into(),
            cves: Vec::new(),
            apps: Vec::new(),
        };
        let mut plan = Plan::new(PlanKind::SystemUpdate);
        plan.steps.push(Step::Upgrade {
            name: "openssl".into(),
            from: "3.0-1".into(),
            to: "3.1-1".into(),
        });
        let findings = [
            finding("openssl", Some("3.1-1")),
            finding("openssl", Some("3.2-1")),
            finding("curl", Some("8.0-1")),
            finding("openssl", None),
        ];
        let fixes = fixed_by(&findings, &plan);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].fixed.as_deref(), Some("3.1-1"));

        let clean = crate::status::Status {
            checked: 1,
            ..crate::status::Status::default()
        };
        assert!(auto_install(true, &clean, 1));
        assert!(!auto_install(false, &clean, 1));
        assert!(!auto_install(true, &clean, 0));
        let news = crate::status::Status {
            news: vec!["manual step".into()],
            ..clean.clone()
        };
        assert!(!auto_install(true, &news, 1));
        let blocked = crate::status::Status {
            blocked: true,
            ..clean
        };
        assert!(!auto_install(true, &blocked, 1));
    }
}
