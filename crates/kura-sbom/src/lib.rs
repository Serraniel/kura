// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `CycloneDX` 1.6 SBOM of the installed packages.
//!
//! The local package database already lists every installed component with
//! version, licence and dependencies; this crate only writes it in a standard
//! format so vulnerability scanners can read it.

use std::fmt::Write as _;

use kura_core::package::PackageSet;
use serde_json::{Value, json};

/// The system the SBOM describes.
#[derive(Debug, Clone)]
pub struct System<'a> {
    /// `ID` from os-release, for example `endeavouros`.
    pub distro_id: &'a str,
    pub distro_name: &'a str,
}

/// Values that differ per run and are supplied by the caller so output is reproducible in tests.
#[derive(Debug, Clone)]
pub struct RunInfo<'a> {
    /// `urn:uuid:...`
    pub serial: &'a str,
    /// RFC 3339 timestamp.
    pub timestamp: &'a str,
    pub tool_version: &'a str,
}

/// purl namespace for a package, based on the repository it came from.
pub fn namespace(repo: Option<&str>) -> &'static str {
    match repo {
        None => "aur",
        Some(r) if r.starts_with("cachyos") => "cachyos",
        Some("endeavouros") => "endeavouros",
        Some("chaotic-aur") => "chaotic-aur",
        Some(_) => "arch",
    }
}

/// Package URL as defined by the purl specification for the `alpm` type.
pub fn purl(namespace: &str, name: &str, version: &str, arch: Option<&str>) -> String {
    let mut p = format!("pkg:alpm/{namespace}/{}@{}", encode(name), encode(version));
    if let Some(a) = arch {
        let _ = write!(p, "?arch={}", encode(a));
    }
    p
}

/// Arch writes SPDX identifiers and expressions; older packages use free text
/// such as `custom:Foo`, which goes into a named licence instead.
fn is_spdx_expression(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with("custom")
        && s.split_whitespace().all(|tok| {
            let tok = tok.trim_matches(['(', ')']);
            matches!(tok, "AND" | "OR" | "WITH")
                || (!tok.is_empty()
                    && tok
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+' | b':')))
        })
}

fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// Builds the SBOM document.
pub fn build(packages: &PackageSet, system: &System<'_>, run: &RunInfo<'_>) -> Value {
    let components: Vec<Value> = packages
        .iter()
        .map(|p| {
            let ns = namespace(p.repo.as_deref());
            let licenses: Vec<Value> = p
                .licenses
                .iter()
                .map(|l| {
                    if is_spdx_expression(l) {
                        json!({ "expression": l })
                    } else {
                        json!({ "license": { "name": l } })
                    }
                })
                .collect();
            let mut c = json!({
                "type": "library",
                "bom-ref": p.name,
                "name": p.name,
                "version": p.version,
                "purl": purl(ns, &p.name, &p.version, p.arch.as_deref()),
            });
            if !licenses.is_empty() {
                c["licenses"] = Value::Array(licenses);
            }
            c
        })
        .collect();

    let dependencies: Vec<Value> = packages
        .iter()
        .map(|p| {
            let mut deps: Vec<&str> = p.depends.iter().flat_map(|d| packages.resolve(d)).collect();
            deps.sort_unstable();
            deps.dedup();
            json!({ "ref": p.name, "dependsOn": deps })
        })
        .collect();

    let root = json!({
        "type": "operating-system",
        "bom-ref": "system",
        "name": system.distro_name,
        "purl": format!("pkg:generic/{}", encode(system.distro_id)),
    });

    json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "serialNumber": run.serial,
        "version": 1,
        "metadata": {
            "timestamp": run.timestamp,
            "tools": { "components": [{ "type": "application", "name": "kura", "version": run.tool_version }] },
            "component": root,
        },
        "components": components,
        "dependencies": dependencies,
    })
}

#[cfg(test)]
mod tests {
    use kura_core::package::{InstallReason, InstalledPackage};

    use super::*;

    fn sample() -> Value {
        let mut ffmpeg = InstalledPackage::new("ffmpeg", "2:8.0.1-3", InstallReason::Dependency)
            .with_repo("extra")
            .with_depends(["libdav1d.so=7-64", "glibc"]);
        ffmpeg.licenses = vec!["GPL-3.0-only".into()];
        ffmpeg.arch = Some("x86_64".into());
        let set = PackageSet::new([
            ffmpeg,
            InstalledPackage::new("dav1d", "1.5.0-1", InstallReason::Dependency)
                .with_repo("extra")
                .with_provides(["libdav1d.so=7-64"]),
            InstalledPackage::new("yay-bin", "13.0.1-1", InstallReason::Explicit),
        ]);
        build(
            &set,
            &System {
                distro_id: "endeavouros",
                distro_name: "EndeavourOS",
            },
            &RunInfo {
                serial: "urn:uuid:00000000-0000-4000-8000-000000000000",
                timestamp: "2026-10-25T12:00:00Z",
                tool_version: "0.0.0",
            },
        )
    }

    #[test]
    fn document_shape() {
        let bom = sample();
        assert_eq!(bom["bomFormat"], "CycloneDX");
        assert_eq!(bom["specVersion"], "1.6");
        assert_eq!(bom["metadata"]["component"]["name"], "EndeavourOS");
        assert_eq!(bom["components"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn components_have_purls_and_licences() {
        let bom = sample();
        let ffmpeg = bom["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "ffmpeg")
            .unwrap();
        assert_eq!(ffmpeg["purl"], "pkg:alpm/arch/ffmpeg@2%3A8.0.1-3?arch=x86_64");
        assert_eq!(ffmpeg["licenses"][0]["expression"], "GPL-3.0-only");
        let yay = bom["components"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "yay-bin")
            .unwrap();
        assert_eq!(yay["purl"], "pkg:alpm/aur/yay-bin@13.0.1-1");
        assert!(yay.get("licenses").is_none());
    }

    #[test]
    fn dependencies_resolve_provides() {
        let bom = sample();
        let dep = bom["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["ref"] == "ffmpeg")
            .unwrap();
        assert_eq!(dep["dependsOn"], json!(["dav1d"]));
    }

    #[test]
    fn namespaces() {
        assert_eq!(namespace(Some("core")), "arch");
        assert_eq!(namespace(Some("cachyos-v3")), "cachyos");
        assert_eq!(namespace(Some("endeavouros")), "endeavouros");
        assert_eq!(namespace(Some("chaotic-aur")), "chaotic-aur");
        assert_eq!(namespace(None), "aur");
        assert_eq!(purl("arch", "gtk+", "1.0", None), "pkg:alpm/arch/gtk%2B@1.0");
    }

    #[test]
    fn licence_kinds() {
        assert!(is_spdx_expression("GPL-3.0-or-later"));
        assert!(is_spdx_expression("(MIT OR Apache-2.0) AND LicenseRef-UnRAR"));
        assert!(is_spdx_expression("GPL-2.0-only WITH Linux-syscall-note"));
        assert!(!is_spdx_expression("custom:Foo"));
        assert!(!is_spdx_expression("Some free text, really"));
        assert!(!is_spdx_expression(""));
    }

    #[test]
    fn empty_system() {
        let set = PackageSet::default();
        let bom = build(
            &set,
            &System {
                distro_id: "arch",
                distro_name: "Arch Linux",
            },
            &RunInfo {
                serial: "urn:uuid:x",
                timestamp: "t",
                tool_version: "v",
            },
        );
        assert_eq!(bom["components"], json!([]));
    }
}

/// Formats Unix seconds as an RFC 3339 UTC timestamp.
pub fn rfc3339(unix: u64) -> String {
    let days = unix / 86_400;
    let secs = unix % 86_400;
    // Civil-from-days, Howard Hinnant's algorithm.
    let z = days.cast_signed() + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

#[cfg(test)]
mod time_tests {
    #[test]
    fn rfc3339_known_values() {
        assert_eq!(super::rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(super::rfc3339(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(super::rfc3339(1_792_931_045), "2026-10-25T12:24:05Z");
    }
}
