// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The AUR: metadata, trust signals, the cooling period for updates, the
//! dependency graph and the order in which packages can be built in parallel.
//!
//! Fetching and building happen elsewhere; everything here works on data.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::package::dep_name;
use crate::version::vercmp;

const DAY: i64 = 86_400;

/// One package as the AUR RPC (version 5) describes it.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Info {
    pub name: String,
    pub package_base: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "URL")]
    pub url: Option<String>,
    #[serde(default)]
    pub num_votes: u64,
    #[serde(default)]
    pub popularity: f64,
    /// When the package was flagged out of date.
    #[serde(default)]
    pub out_of_date: Option<i64>,
    /// `None` for orphaned packages.
    #[serde(default)]
    pub maintainer: Option<String>,
    pub first_submitted: i64,
    pub last_modified: i64,
    #[serde(default)]
    pub depends: Vec<String>,
    #[serde(default)]
    pub make_depends: Vec<String>,
    #[serde(default)]
    pub check_depends: Vec<String>,
    #[serde(default)]
    pub provides: Vec<String>,
}

#[derive(Deserialize)]
struct Response {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    results: Vec<Info>,
}

/// Parses an RPC answer (`info` or `search`).
pub fn parse(json: &str) -> Result<Vec<Info>, String> {
    let r: Response = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if r.kind == "error" {
        return Err(r.error.unwrap_or_else(|| "unknown error".into()));
    }
    Ok(r.results)
}

/// The commit `HEAD` points to, from git's smart HTTP answer to
/// `<base>.git/info/refs?service=git-upload-pack`. kura pins this commit in
/// the plan, so the build uses exactly what the preview showed.
pub fn head_commit(refs: &str) -> Option<String> {
    refs.split(['\n', '\0']).find_map(|line| {
        // pkt-line: 4 hex digits of length, then "<sha> <ref>". A flush
        // packet "0000" can come right before it.
        let mut line = line;
        while let Some(rest) = line.strip_prefix("0000") {
            line = rest;
        }
        let line = line.get(4..).filter(|_| line.len() > 45)?;
        let (sha, name) = line.split_once(' ')?;
        let valid = sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit());
        (valid && name == "HEAD").then(|| sha.to_owned())
    })
}

/// Whether `base` is a valid AUR package base name. Names end up in paths
/// and URLs, so nothing else is accepted.
pub fn valid_base(base: &str) -> bool {
    !base.is_empty()
        && !base.starts_with(['-', '.'])
        && base
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"@._+-".contains(&b))
}

/// Whether `commit` is a full git commit id.
pub fn valid_commit(commit: &str) -> bool {
    commit.len() == 40 && commit.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// What kura needs from a `.SRCINFO` file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SrcInfo {
    pub pkgbase: String,
    pub pkgnames: Vec<String>,
    /// Keys that sign the sources; imported before the build.
    pub validpgpkeys: Vec<String>,
}

/// Parses `.SRCINFO` (`key = value` lines).
pub fn parse_srcinfo(text: &str) -> SrcInfo {
    let mut out = SrcInfo::default();
    for line in text.lines() {
        let Some((key, value)) = line.trim().split_once(" = ") else {
            continue;
        };
        match key {
            "pkgbase" => value.clone_into(&mut out.pkgbase),
            "pkgname" => out.pkgnames.push(value.to_owned()),
            "validpgpkeys" => out.validpgpkeys.push(value.to_owned()),
            _ => {}
        }
    }
    out
}

/// Things worth knowing before installing a package from the AUR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warning {
    /// First uploaded less than 30 days ago.
    New,
    /// Nobody maintains it.
    Orphaned,
    /// Flagged as out of date since this time.
    OutOfDate { since: i64 },
}

pub fn warnings(info: &Info, now: i64) -> Vec<Warning> {
    let mut out = Vec::new();
    if now - info.first_submitted < 30 * DAY {
        out.push(Warning::New);
    }
    if info.maintainer.is_none() {
        out.push(Warning::Orphaned);
    }
    if let Some(since) = info.out_of_date {
        out.push(Warning::OutOfDate { since });
    }
    out
}

/// When an AUR update may enter the normal update: `days` after the last
/// change, and `days` after kura first saw the current maintainer.
pub fn ready_at(info: &Info, days: u32, maintainer_since: Option<i64>) -> i64 {
    info.last_modified.max(maintainer_since.unwrap_or(i64::MIN)) + i64::from(days) * DAY
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub name: String,
    pub from: String,
    pub to: String,
    /// `None` when the update is ready, otherwise when it will be.
    pub waits_until: Option<i64>,
}

/// Updates for installed foreign packages. `maintainer_since` says when kura
/// first saw a package's current maintainer.
pub fn updates(
    installed: &[(String, String)],
    infos: &[Info],
    now: i64,
    days: u32,
    maintainer_since: impl Fn(&str) -> Option<i64>,
) -> Vec<Update> {
    installed
        .iter()
        .filter_map(|(name, version)| {
            let info = infos.iter().find(|i| &i.name == name)?;
            if vercmp(&info.version, version) != Ordering::Greater {
                return None;
            }
            let at = ready_at(info, days, maintainer_since(name));
            Some(Update {
                name: name.clone(),
                from: version.clone(),
                to: info.version.clone(),
                waits_until: (now < at).then_some(at),
            })
        })
        .collect()
}

/// How a dependency is satisfied without building anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Satisfied {
    Installed,
    /// A repository has it; it is installed before the builds start.
    Repository,
    No,
}

/// One package base to build.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Build {
    pub version: String,
    /// Packages of this base to install afterwards.
    pub packages: BTreeSet<String>,
    /// Bases that must be built and installed first.
    pub after: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolution {
    pub builds: BTreeMap<String, Build>,
    /// Dependencies from the repositories, installed in one transaction first.
    pub repo_deps: BTreeSet<String>,
    /// `(needed by, dependency)` that nothing provides.
    pub missing: Vec<(String, String)>,
}

fn provides(info: &Info, name: &str) -> bool {
    info.name == name || info.provides.iter().any(|p| dep_name(p) == name)
}

/// Resolves the whole dependency graph before anything is built.
///
/// `fetch` asks the AUR for packages by name, in batches; `satisfied` checks
/// the installed packages and the repositories.
pub fn resolve(
    targets: &[String],
    mut fetch: impl FnMut(&[String]) -> Vec<Info>,
    satisfied: impl Fn(&str) -> Satisfied,
) -> Resolution {
    let mut known: Vec<Info> = Vec::new();
    let mut asked: BTreeSet<String> = BTreeSet::new();
    let mut res = Resolution::default();
    // (package name, needed by) still to look at.
    let mut todo: Vec<(String, Option<String>)> = targets.iter().map(|t| (t.clone(), None)).collect();

    while !todo.is_empty() {
        let unknown: Vec<String> = todo
            .iter()
            .map(|(n, _)| n.clone())
            .filter(|n| !known.iter().any(|i| provides(i, n)) && !asked.contains(n))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if !unknown.is_empty() {
            asked.extend(unknown.iter().cloned());
            known.extend(fetch(&unknown));
        }
        let mut next = Vec::new();
        for (name, needed_by) in std::mem::take(&mut todo) {
            let Some(info) = known.iter().find(|i| provides(i, &name)).cloned() else {
                res.missing.push((needed_by.unwrap_or_default(), name));
                continue;
            };
            if let Some(parent) = &needed_by
                && let Some(b) = res.builds.get_mut(parent)
                && *parent != info.package_base
            {
                b.after.insert(info.package_base.clone());
            }
            let first = !res.builds.contains_key(&info.package_base);
            let build = res.builds.entry(info.package_base.clone()).or_default();
            build.version.clone_from(&info.version);
            let added = build.packages.insert(info.name.clone());
            if !first && !added {
                continue;
            }
            let deps = info
                .depends
                .iter()
                .chain(&info.make_depends)
                .chain(&info.check_depends);
            for dep in deps {
                match satisfied(dep) {
                    Satisfied::Installed => {}
                    Satisfied::Repository => {
                        res.repo_deps.insert(dep.clone());
                    }
                    Satisfied::No => next.push((dep_name(dep).to_owned(), Some(info.package_base.clone()))),
                }
            }
        }
        todo = next;
    }
    res
}

/// Bases that can start now: everything they wait for is done.
pub fn ready<'a>(
    builds: &'a BTreeMap<String, Build>,
    done: &BTreeSet<String>,
    started: &BTreeSet<String>,
) -> Vec<&'a str> {
    builds
        .iter()
        .filter(|(base, b)| !started.contains(*base) && b.after.iter().all(|a| done.contains(a)))
        .map(|(base, _)| base.as_str())
        .collect()
}

/// Bases that wait for each other in a circle and can never be built.
pub fn cycle(builds: &BTreeMap<String, Build>) -> Vec<String> {
    let mut done = BTreeSet::new();
    loop {
        let now: Vec<String> = ready(builds, &done, &done)
            .into_iter()
            .map(str::to_owned)
            .collect();
        if now.is_empty() {
            break;
        }
        done.extend(now);
    }
    builds.keys().filter(|b| !done.contains(*b)).cloned().collect()
}

/// How many builds run at the same time and how many threads each gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Jobs {
    pub parallel: usize,
    /// For `MAKEFLAGS=-j…`.
    pub threads_each: usize,
}

/// Memory a single build is assumed to need.
const BUILD_MEMORY: u64 = 2 * 1024 * 1024 * 1024;

pub fn jobs(cpus: usize, memory_available: u64, builds: usize) -> Jobs {
    let cpus = cpus.max(1);
    let by_memory = usize::try_from(memory_available / BUILD_MEMORY)
        .unwrap_or(usize::MAX)
        .max(1);
    let parallel = builds.clamp(1, cpus.min(by_memory));
    Jobs {
        parallel,
        threads_each: (cpus / parallel).max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(name: &str, base: &str, depends: &[&str], make: &[&str]) -> Info {
        Info {
            name: name.into(),
            package_base: base.into(),
            version: "1.0-1".into(),
            maintainer: Some("someone".into()),
            depends: depends.iter().map(|s| (*s).to_owned()).collect(),
            make_depends: make.iter().map(|s| (*s).to_owned()).collect(),
            ..Info::default()
        }
    }

    #[test]
    fn parses_rpc_answers() {
        let json = r#"{"resultcount":1,"results":[{"Depends":["pacman>6.1","git"],"Description":"AUR helper","FirstSubmitted":1475000000,"ID":1,"LastModified":1760000000,"Maintainer":"jguer","MakeDepends":["go"],"Name":"yay","NumVotes":2500,"OutOfDate":null,"PackageBase":"yay","PackageBaseID":2,"Popularity":30.5,"URL":"https://github.com/Jguer/yay","URLPath":"/cgit/aur.git/snapshot/yay.tar.gz","Version":"12.5.0-1"}],"type":"multiinfo","version":5}"#;
        let got = parse(json).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "yay");
        assert_eq!(got[0].depends, ["pacman>6.1", "git"]);
        assert_eq!(got[0].make_depends, ["go"]);
        assert_eq!(got[0].url.as_deref(), Some("https://github.com/Jguer/yay"));
        assert_eq!(got[0].maintainer.as_deref(), Some("jguer"));
        assert_eq!(got[0].check_depends, Vec::<String>::new());

        let err = r#"{"error":"Incorrect request type specified.","resultcount":0,"results":[],"type":"error","version":5}"#;
        assert_eq!(parse(err), Err("Incorrect request type specified.".into()));
        assert!(parse("not json").is_err());
    }

    #[test]
    fn reads_the_head_commit() {
        let refs = "001e# service=git-upload-pack\n0000015523a5d6a1d2e2b1b9a3f0f6d8b7e3f2c1a0b9c8d7 HEAD\0multi_ack thin-pack side-band symref=HEAD:refs/heads/master\n003f23a5d6a1d2e2b1b9a3f0f6d8b7e3f2c1a0b9c8d7 refs/heads/master\n0000";
        assert_eq!(
            head_commit(refs).as_deref(),
            Some("23a5d6a1d2e2b1b9a3f0f6d8b7e3f2c1a0b9c8d7")
        );
        assert_eq!(head_commit("001e# service=git-upload-pack\n0000"), None);
        assert_eq!(head_commit("0000zz HEAD"), None);
    }

    #[test]
    fn validates_names_and_commits() {
        for ok in ["pfetch", "lib32-foo", "python-foo.bar", "c++utils", "foo@bar"] {
            assert!(valid_base(ok), "{ok}");
        }
        for bad in ["", "-x", ".x", "../etc", "a/b", "Foo", "a b"] {
            assert!(!valid_base(bad), "{bad}");
        }
        assert!(valid_commit("23a5d6a1d2e2b1b9a3f0f6d8b7e3f2c1a0b9c8d7"));
        assert!(!valid_commit("23A5d6a1d2e2b1b9a3f0f6d8b7e3f2c1a0b9c8d7"));
        assert!(!valid_commit("23a5d6"));
    }

    #[test]
    fn parses_srcinfo() {
        let text = "pkgbase = tools\n\tpkgdesc = Tools\n\tpkgver = 1.0\n\tvalidpgpkeys = ABCDEF0123456789ABCDEF0123456789ABCDEF01\n\npkgname = tool-a\n\npkgname = tool-b\n\tdepends = tool-a\n";
        assert_eq!(
            parse_srcinfo(text),
            SrcInfo {
                pkgbase: "tools".into(),
                pkgnames: vec!["tool-a".into(), "tool-b".into()],
                validpgpkeys: vec!["ABCDEF0123456789ABCDEF0123456789ABCDEF01".into()],
            }
        );
    }

    #[test]
    fn trust_warnings() {
        let now = 1_000 * DAY;
        let mut i = info("a", "a", &[], &[]);
        i.first_submitted = now - 40 * DAY;
        assert_eq!(warnings(&i, now), []);
        i.first_submitted = now - 2 * DAY;
        i.maintainer = None;
        i.out_of_date = Some(now - DAY);
        assert_eq!(
            warnings(&i, now),
            [
                Warning::New,
                Warning::Orphaned,
                Warning::OutOfDate { since: now - DAY }
            ]
        );
    }

    #[test]
    fn cooling_period() {
        let now = 1_000 * DAY;
        let mut a = info("a", "a", &[], &[]);
        a.version = "2.0-1".into();
        a.last_modified = now - DAY;
        let mut b = info("b", "b", &[], &[]);
        b.version = "1.5-1".into();
        b.last_modified = now - 10 * DAY;
        let mut c = info("c", "c", &[], &[]);
        c.version = "1.0-1".into();
        let installed = [
            ("a".to_owned(), "1.0-1".to_owned()),
            ("b".to_owned(), "1.0-1".to_owned()),
            ("c".to_owned(), "1.0-1".to_owned()),
            ("gone".to_owned(), "1.0-1".to_owned()),
        ];
        let infos = [a, b, c];
        let got = updates(&installed, &infos, now, 3, |_| None);
        assert_eq!(
            got,
            [
                Update {
                    name: "a".into(),
                    from: "1.0-1".into(),
                    to: "2.0-1".into(),
                    waits_until: Some(now + 2 * DAY),
                },
                Update {
                    name: "b".into(),
                    from: "1.0-1".into(),
                    to: "1.5-1".into(),
                    waits_until: None,
                },
            ]
        );
        // A new maintainer restarts the period.
        let got = updates(&installed, &infos, now, 3, |n| (n == "b").then_some(now));
        assert_eq!(got[1].waits_until, Some(now + 3 * DAY));
        // Zero days: everything is ready.
        assert!(
            updates(&installed, &infos, now, 0, |_| None)
                .iter()
                .all(|u| u.waits_until.is_none())
        );
    }

    #[test]
    fn resolves_the_whole_graph_in_batches() {
        // app needs lib (AUR) and gtk3 (repo); lib needs cmake to build (repo)
        // and helper (AUR, a split package of base "tools").
        let aur = [
            info("app", "app", &["lib", "gtk3"], &["cargo"]),
            info("lib", "lib", &["tool-a>=2"], &["cmake"]),
            info("tool-a", "tools", &[], &[]),
            info("tool-b", "tools", &["tool-a"], &[]),
        ];
        let mut batches: Vec<Vec<String>> = Vec::new();
        let fetch = |names: &[String]| {
            batches.push(names.to_vec());
            aur.iter().filter(|i| names.contains(&i.name)).cloned().collect()
        };
        let satisfied = |dep: &str| match dep_name(dep) {
            "cargo" => Satisfied::Installed,
            "gtk3" | "cmake" => Satisfied::Repository,
            _ => Satisfied::No,
        };
        let res = resolve(&["app".into(), "tool-b".into()], fetch, satisfied);
        assert_eq!(batches, [vec!["app", "tool-b"], vec!["lib", "tool-a"]]);
        assert_eq!(res.repo_deps, BTreeSet::from(["gtk3".into(), "cmake".into()]));
        assert_eq!(res.missing, Vec::<(String, String)>::new());
        assert_eq!(res.builds.len(), 3);
        assert_eq!(res.builds["app"].after, BTreeSet::from(["lib".into()]));
        assert_eq!(res.builds["lib"].after, BTreeSet::from(["tools".into()]));
        // Both split packages of "tools" are installed, and the base does
        // not wait for itself.
        assert_eq!(
            res.builds["tools"].packages,
            BTreeSet::from(["tool-a".into(), "tool-b".into()])
        );
        assert!(res.builds["tools"].after.is_empty());
    }

    #[test]
    fn reports_what_nobody_provides() {
        let aur = [
            info("app", "app", &["ghost"], &[]),
            info("x", "x", &["java-runtime"], &[]),
        ];
        let mut provider = info("jre-bin", "jre-bin", &[], &[]);
        provider.provides = vec!["java-runtime=21".into()];
        let all: Vec<Info> = aur.iter().cloned().chain([provider]).collect();
        let fetch = |names: &[String]| {
            all.iter()
                .filter(|i| names.iter().any(|n| provides(i, n)))
                .cloned()
                .collect()
        };
        let res = resolve(&["app".into(), "x".into()], fetch, |_| Satisfied::No);
        assert_eq!(res.missing, [("app".to_owned(), "ghost".to_owned())]);
        assert_eq!(res.builds["x"].after, BTreeSet::from(["jre-bin".into()]));
        let none = resolve(&["nothing".into()], |_| Vec::new(), |_| Satisfied::No);
        assert_eq!(none.missing, [(String::new(), "nothing".to_owned())]);
    }

    #[test]
    fn schedules_independent_builds_together() {
        let build = |after: &[&str]| Build {
            after: after.iter().map(|s| (*s).to_owned()).collect(),
            ..Build::default()
        };
        let builds = BTreeMap::from([
            ("app".to_owned(), build(&["lib"])),
            ("lib".to_owned(), build(&[])),
            ("other".to_owned(), build(&[])),
        ]);
        let none = BTreeSet::new();
        assert_eq!(ready(&builds, &none, &none), ["lib", "other"]);
        let started = BTreeSet::from(["lib".to_owned(), "other".to_owned()]);
        assert_eq!(ready(&builds, &none, &started), Vec::<&str>::new());
        // "app" starts as soon as "lib" is done, even while "other" still builds.
        let done = BTreeSet::from(["lib".to_owned()]);
        assert_eq!(ready(&builds, &done, &started), ["app"]);
        assert_eq!(cycle(&builds), Vec::<String>::new());

        let circle = BTreeMap::from([
            ("a".to_owned(), build(&["b"])),
            ("b".to_owned(), build(&["a"])),
            ("c".to_owned(), build(&[])),
        ]);
        assert_eq!(cycle(&circle), ["a", "b"]);
    }

    #[test]
    fn job_limits() {
        const GIB: u64 = 1024 * 1024 * 1024;
        assert_eq!(
            jobs(16, 32 * GIB, 3),
            Jobs {
                parallel: 3,
                threads_each: 5
            }
        );
        // Memory limits parallel builds.
        assert_eq!(
            jobs(16, 5 * GIB, 8),
            Jobs {
                parallel: 2,
                threads_each: 8
            }
        );
        assert_eq!(
            jobs(4, 64 * GIB, 10),
            Jobs {
                parallel: 4,
                threads_each: 1
            }
        );
        assert_eq!(
            jobs(0, 0, 0),
            Jobs {
                parallel: 1,
                threads_each: 1
            }
        );
    }
}
