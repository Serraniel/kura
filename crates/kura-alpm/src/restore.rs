// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Going back to an earlier generation.
//!
//! Old package files come from pacman's cache or, for packages from the Arch
//! repositories, from the Arch Linux Archive. Settings, personal files and
//! data formats are not reverted, and the next update brings the newer
//! versions back; the window and the CLI say so before anything runs.

use std::path::{Path, PathBuf};

use kura_core::generation::Generation;
use kura_core::history::Journal;
use kura_core::plan::{Plan, PlanKind, Step};

use crate::prepare::Problem;

const ARCHIVE: &str = "https://archive.archlinux.org/packages";
const EXTENSIONS: [&str; 2] = [".pkg.tar.zst", ".pkg.tar.xz"];
const ARCHES: [&str; 2] = ["x86_64", "any"];

/// Where the file for one package version comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A file in pacman's cache.
    Cache(PathBuf),
    /// Candidate archive URLs, tried in order.
    Archive(Vec<String>),
}

/// The plan that returns from the newest generation to `target`.
pub fn plan(journal: &Journal, target: u64) -> Option<Plan> {
    let changes = journal.changes_to(target)?;
    Some(Plan::from_changes(
        PlanKind::Restore { generation: target },
        &changes,
    ))
}

/// Archive URLs for a package version, one per possible architecture.
pub fn archive_urls(name: &str, version: &str) -> Vec<String> {
    let first = name.chars().next().map_or('_', |c| c.to_ascii_lowercase());
    ARCHES
        .iter()
        .map(|arch| format!("{ARCHIVE}/{first}/{name}/{name}-{version}-{arch}.pkg.tar.zst"))
        .collect()
}

/// Finds a cached package file for a version.
pub fn cached(cache_dirs: &[String], name: &str, version: &str) -> Option<PathBuf> {
    cache_dirs.iter().find_map(|dir| {
        ARCHES.iter().find_map(|arch| {
            EXTENSIONS.iter().find_map(|ext| {
                let path = Path::new(dir).join(format!("{name}-{version}-{arch}{ext}"));
                path.is_file().then_some(path)
            })
        })
    })
}

/// Where each package that has to be installed comes from. Packages that are
/// neither cached nor from the Arch repositories cannot be brought back.
pub fn sources(
    cache_dirs: &[String],
    plan: &Plan,
    target: &Generation,
) -> Result<Vec<(String, Source)>, Vec<Problem>> {
    let mut found = Vec::new();
    let mut missing = Vec::new();
    for step in &plan.steps {
        let (name, version) = match step {
            Step::Install { name, version, .. } => (name, version),
            Step::Upgrade { name, to, .. } | Step::Downgrade { name, to, .. } => (name, to),
            Step::Remove { .. } | Step::MarkReason { .. } => continue,
        };
        if let Some(path) = cached(cache_dirs, name, version) {
            found.push((name.clone(), Source::Cache(path)));
            continue;
        }
        let from_arch = target
            .packages
            .get(name)
            .and_then(|e| e.repo.as_deref())
            .is_some_and(|r| matches!(r, "core" | "extra" | "multilib"));
        if from_arch {
            found.push((name.clone(), Source::Archive(archive_urls(name, version))));
        } else {
            missing.push(Problem::NotRestorable {
                name: name.clone(),
                version: version.clone(),
            });
        }
    }
    if missing.is_empty() {
        Ok(found)
    } else {
        Err(missing)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use kura_core::generation::Generation;
    use kura_core::history::Journal;
    use kura_core::package::{InstallReason, InstalledPackage};

    use super::*;

    fn generation(pkgs: &[(&str, &str, Option<&str>)]) -> Generation {
        let installed: Vec<_> = pkgs
            .iter()
            .map(|(n, v, r)| {
                let p = InstalledPackage::new(*n, *v, InstallReason::Explicit);
                match r {
                    Some(repo) => p.with_repo(*repo),
                    None => p,
                }
            })
            .collect();
        Generation::from_installed(0, 0, &installed)
    }

    #[test]
    fn plan_and_sources() {
        let then = generation(&[("firefox", "151.0-1", Some("extra")), ("yay-bin", "12.0-1", None)]);
        let now = generation(&[
            ("firefox", "152.0-1", Some("extra")),
            ("yay-bin", "13.0-1", None),
            ("gimp", "3-1", Some("extra")),
        ]);
        let mut j = Journal::default();
        let r1 = j.record(&then, 1, None).unwrap();
        j.push(r1);
        let r2 = j.record(&now, 2, None).unwrap();
        j.push(r2);

        let plan = plan(&j, 1).unwrap();
        assert_eq!(plan.kind, PlanKind::Restore { generation: 1 });
        assert_eq!(plan.steps.len(), 3);
        assert!(super::plan(&j, 9).is_none());

        let dir = std::env::temp_dir().join(format!("kura-restore-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let target = j.generation(1).unwrap();
        let cache = vec![dir.to_string_lossy().into_owned()];

        // yay-bin came from the AUR and is not cached: it cannot come back.
        let problems = sources(&cache, &plan, &target).unwrap_err();
        assert_eq!(
            problems,
            vec![Problem::NotRestorable {
                name: "yay-bin".into(),
                version: "12.0-1".into()
            }]
        );

        // With the old AUR build in the cache, everything can be restored.
        fs::write(dir.join("yay-bin-12.0-1-x86_64.pkg.tar.zst"), b"").unwrap();
        let found = sources(&cache, &plan, &target).unwrap();
        assert_eq!(found.len(), 2);
        assert!(matches!(
            &found.iter().find(|(n, _)| n == "yay-bin").unwrap().1,
            Source::Cache(_)
        ));
        assert_eq!(
            found.iter().find(|(n, _)| n == "firefox").unwrap().1,
            Source::Archive(vec![
                "https://archive.archlinux.org/packages/f/firefox/firefox-151.0-1-x86_64.pkg.tar.zst".into(),
                "https://archive.archlinux.org/packages/f/firefox/firefox-151.0-1-any.pkg.tar.zst".into(),
            ])
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cache_lookup_tries_arches_and_extensions() {
        let dir = std::env::temp_dir().join(format!("kura-cache-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("python-x-1.0-1-any.pkg.tar.xz"), b"").unwrap();
        let cache = vec!["/nonexistent".to_owned(), dir.to_string_lossy().into_owned()];
        assert_eq!(
            cached(&cache, "python-x", "1.0-1"),
            Some(dir.join("python-x-1.0-1-any.pkg.tar.xz"))
        );
        assert_eq!(cached(&cache, "python-x", "2.0-1"), None);
        fs::remove_dir_all(dir).unwrap();
    }
}
