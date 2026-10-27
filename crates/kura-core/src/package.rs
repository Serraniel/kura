// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Installed packages and the dependency graph between them.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

/// Why a package is installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallReason {
    /// The user asked for it.
    Explicit,
    /// Pulled in by another package.
    Dependency,
}

/// One installed package as kura sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    /// Sync repository the package came from, `None` for foreign packages (AUR, local builds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    pub reason: InstallReason,
    /// Dependency names with version constraints removed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optdepends: Vec<String>,
    /// Names this package provides, without versions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provides: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub licenses: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
}

impl InstalledPackage {
    pub fn new(name: impl Into<String>, version: impl Into<String>, reason: InstallReason) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            repo: None,
            reason,
            depends: Vec::new(),
            optdepends: Vec::new(),
            provides: Vec::new(),
            licenses: Vec::new(),
            arch: None,
        }
    }

    #[must_use]
    pub fn with_depends<I, S>(mut self, deps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.depends = deps.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn with_optdepends<I, S>(mut self, deps: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.optdepends = deps.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn with_provides<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.provides = names.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn with_repo(mut self, repo: impl Into<String>) -> Self {
        self.repo = Some(repo.into());
        self
    }
}

/// Strips a version constraint from a dependency string: `glibc>=2.40` -> `glibc`.
pub fn dep_name(dep: &str) -> &str {
    let end = dep.find(['<', '>', '=']).unwrap_or(dep.len());
    dep[..end].trim()
}

/// The set of installed packages with lookups along dependencies.
#[derive(Debug, Clone, Default)]
pub struct PackageSet {
    packages: BTreeMap<String, InstalledPackage>,
    /// Maps every provided name (including the package's own name) to package names.
    providers: BTreeMap<String, BTreeSet<String>>,
}

impl PackageSet {
    pub fn new(packages: impl IntoIterator<Item = InstalledPackage>) -> Self {
        let mut set = Self::default();
        for pkg in packages {
            set.insert(pkg);
        }
        set
    }

    fn insert(&mut self, pkg: InstalledPackage) {
        self.providers
            .entry(pkg.name.clone())
            .or_default()
            .insert(pkg.name.clone());
        for p in &pkg.provides {
            self.providers
                .entry(dep_name(p).to_owned())
                .or_default()
                .insert(pkg.name.clone());
        }
        self.packages.insert(pkg.name.clone(), pkg);
    }

    pub fn get(&self, name: &str) -> Option<&InstalledPackage> {
        self.packages.get(name)
    }

    pub fn len(&self) -> usize {
        self.packages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &InstalledPackage> {
        self.packages.values()
    }

    /// Installed packages that satisfy a dependency string.
    pub fn resolve(&self, dep: &str) -> impl Iterator<Item = &str> {
        self.providers
            .get(dep_name(dep))
            .into_iter()
            .flatten()
            .map(String::as_str)
    }

    /// Packages that directly depend on `name` (through any name it provides).
    pub fn required_by(&self, name: &str) -> BTreeSet<&str> {
        let Some(target) = self.packages.get(name) else {
            return BTreeSet::new();
        };
        let mut names: BTreeSet<&str> = target.provides.iter().map(|p| dep_name(p)).collect();
        names.insert(&target.name);
        self.packages
            .values()
            .filter(|p| p.name != name)
            .filter(|p| p.depends.iter().any(|d| names.contains(dep_name(d))))
            .map(|p| p.name.as_str())
            .collect()
    }

    /// Dependencies that nothing needs any more, including packages that only
    /// such orphans needed. Optional dependencies of installed packages are kept.
    pub fn orphans(&self) -> BTreeSet<&str> {
        // Who needs each package, built once: asking `dependants` for every
        // package in every round takes minutes on a full desktop.
        let mut needed_by: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for p in self.packages.values() {
            let deps = p.depends.iter().map(|d| dep_name(d));
            let opt = p.optdepends.iter().map(|d| optdep_name(d));
            for dep in deps.chain(opt) {
                for target in self.providers.get(dep).into_iter().flatten() {
                    if *target != p.name {
                        needed_by.entry(target).or_default().insert(&p.name);
                    }
                }
            }
        }
        let unneeded = |name: &str, removed: &BTreeSet<&str>| {
            self.packages
                .get(name)
                .is_some_and(|p| p.reason == InstallReason::Dependency)
                && needed_by
                    .get(name)
                    .is_none_or(|by| by.iter().all(|d| removed.contains(d)))
        };
        // Removing a package can only free what it depends on: look at those again.
        let mut removed: BTreeSet<&str> = BTreeSet::new();
        let mut queue: VecDeque<&str> = self.packages.keys().map(String::as_str).collect();
        while let Some(name) = queue.pop_front() {
            if removed.contains(name) || !unneeded(name, &removed) {
                continue;
            }
            removed.insert(name);
            let p = &self.packages[name];
            let deps = p.depends.iter().map(|d| dep_name(d));
            let opt = p.optdepends.iter().map(|d| optdep_name(d));
            for dep in deps.chain(opt) {
                queue.extend(self.providers.get(dep).into_iter().flatten().map(String::as_str));
            }
        }
        removed
    }

    /// Explicitly installed packages that need `name`, directly or indirectly.
    ///
    /// Used to report a vulnerable library by the apps the user knows. Returns
    /// `name` itself when it is explicitly installed.
    pub fn explicit_users(&self, name: &str) -> BTreeSet<&str> {
        let mut found = BTreeSet::new();
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::from([name]);
        while let Some(current) = queue.pop_front() {
            if !seen.insert(current) {
                continue;
            }
            let Some(pkg) = self.packages.get(current) else {
                continue;
            };
            if pkg.reason == InstallReason::Explicit {
                found.insert(pkg.name.as_str());
            }
            queue.extend(self.required_by(current));
        }
        found
    }
}

fn optdep_name(dep: &str) -> &str {
    // Optional dependencies look like `name: description`.
    dep_name(dep.split(':').next().unwrap_or(dep))
}

#[cfg(test)]
mod tests {
    use super::*;
    use InstallReason::{Dependency, Explicit};

    fn set() -> PackageSet {
        PackageSet::new([
            InstalledPackage::new("kdenlive", "26.08.1-1", Explicit)
                .with_depends(["ffmpeg>=2:8", "qt6-base"]),
            InstalledPackage::new("vlc", "3.0.23-1", Explicit).with_depends(["ffmpeg"]),
            InstalledPackage::new("ffmpeg", "2:8.0.1-3", Dependency).with_depends(["libdav1d.so=7-64"]),
            InstalledPackage::new("dav1d", "1.5.0-1", Dependency).with_provides(["libdav1d.so=7-64"]),
            InstalledPackage::new("qt6-base", "6.12.0-3", Dependency),
            InstalledPackage::new("electron36", "36.9.0-1", Dependency).with_depends(["c-ares"]),
            InstalledPackage::new("c-ares", "1.34.0-1", Dependency),
            InstalledPackage::new("hunspell", "1.7.2-1", Dependency),
            InstalledPackage::new("libreoffice-fresh", "26.8.1-1", Explicit)
                .with_optdepends(["hunspell: spell checking"]),
        ])
    }

    #[test]
    fn dep_name_strips_constraints() {
        assert_eq!(dep_name("glibc>=2.40"), "glibc");
        assert_eq!(dep_name("libfoo.so=1-64"), "libfoo.so");
        assert_eq!(dep_name("bash"), "bash");
        assert_eq!(dep_name("python<3.14"), "python");
    }

    #[test]
    fn resolves_provides() {
        let s = set();
        assert_eq!(s.resolve("libdav1d.so=7-64").collect::<Vec<_>>(), ["dav1d"]);
        assert_eq!(s.resolve("ffmpeg>=1").collect::<Vec<_>>(), ["ffmpeg"]);
        assert_eq!(s.resolve("missing").count(), 0);
    }

    #[test]
    fn required_by_follows_provides() {
        let s = set();
        assert_eq!(s.required_by("dav1d").into_iter().collect::<Vec<_>>(), ["ffmpeg"]);
        assert_eq!(
            s.required_by("ffmpeg").into_iter().collect::<Vec<_>>(),
            ["kdenlive", "vlc"]
        );
        assert!(s.required_by("unknown").is_empty());
    }

    #[test]
    fn orphans_are_found_recursively_and_optdepends_are_kept() {
        let s = set();
        assert_eq!(
            s.orphans().into_iter().collect::<Vec<_>>(),
            ["c-ares", "electron36"]
        );
    }

    #[test]
    fn orphans_of_a_full_desktop_are_quick() {
        // A long chain of unneeded dependencies: the old way took minutes.
        let list: Vec<_> = (0..1000)
            .map(|i| {
                InstalledPackage::new(format!("lib{i}"), "1-1", InstallReason::Dependency)
                    .with_depends([format!("lib{}", i + 1)])
            })
            .collect();
        let start = std::time::Instant::now();
        assert_eq!(PackageSet::new(list).orphans().len(), 1000);
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
    }

    #[test]
    fn explicit_users_name_the_apps() {
        let s = set();
        assert_eq!(
            s.explicit_users("dav1d").into_iter().collect::<Vec<_>>(),
            ["kdenlive", "vlc"]
        );
        assert_eq!(s.explicit_users("vlc").into_iter().collect::<Vec<_>>(), ["vlc"]);
        assert!(s.explicit_users("electron36").is_empty());
    }

    #[test]
    fn basic_accessors() {
        let s = set();
        assert_eq!(s.len(), 9);
        assert!(!s.is_empty());
        assert!(PackageSet::default().is_empty());
        assert_eq!(s.get("vlc").map(|p| p.version.as_str()), Some("3.0.23-1"));
        assert_eq!(s.iter().count(), 9);
        let p = InstalledPackage::new("x", "1", Explicit).with_repo("extra");
        assert_eq!(p.repo.as_deref(), Some("extra"));
    }
}
