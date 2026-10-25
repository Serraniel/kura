// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Suggestions for the system health page, from facts the caller collects.

/// What kura knows about the system.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// Installed kernel packages (`linux`, `linux-lts`, ...).
    pub kernels: Vec<String>,
    /// File system of `/`, for example `btrfs`.
    pub root_fs: String,
    /// Snapper is installed and has a configuration for `/`.
    pub snapper: bool,
    /// Packages nothing needs any more.
    pub orphans: usize,
    /// kura chose the download servers.
    pub mirrors_ranked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suggestion {
    /// A second, rarely changing kernel to start from if an update breaks booting.
    BackupKernel,
    /// The disk supports snapshots but nothing takes them.
    Snapshots,
    /// Packages nothing needs; removed with the next update.
    Cleanup { count: usize },
    /// The download servers were never chosen by speed.
    Mirrors,
}

/// Kernel package names kura knows.
pub fn is_kernel(package: &str) -> bool {
    matches!(
        package,
        "linux" | "linux-lts" | "linux-zen" | "linux-hardened" | "linux-rt" | "linux-rt-lts"
    ) || (package.starts_with("linux-cachyos") && !package.ends_with("-headers"))
}

/// Whether a kernel package version is the running kernel (`uname -r`).
/// `7.2.11.zen1-1` runs as `7.2.11-zen1-1-zen`, `6.18.1.arch1-1` as `6.18.1-arch1-1`.
pub fn is_running(version: &str, uname: &str) -> bool {
    let without_release = version.rsplit_once('-').map_or(version, |(v, _)| v);
    !uname.is_empty() && uname.replace('-', ".").starts_with(without_release)
}

/// Suggestions, most important first.
pub fn suggestions(facts: &Facts) -> Vec<Suggestion> {
    let mut out = Vec::new();
    if !facts
        .kernels
        .iter()
        .any(|k| k == "linux-lts" || k == "linux-cachyos-lts")
    {
        out.push(Suggestion::BackupKernel);
    }
    if facts.root_fs == "btrfs" && !facts.snapper {
        out.push(Suggestion::Snapshots);
    }
    if !facts.mirrors_ranked {
        out.push(Suggestion::Mirrors);
    }
    if facts.orphans > 0 {
        out.push(Suggestion::Cleanup { count: facts.orphans });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_what_is_missing() {
        let facts = Facts {
            kernels: vec!["linux".into(), "linux-zen".into()],
            root_fs: "btrfs".into(),
            snapper: false,
            orphans: 39,
            mirrors_ranked: false,
        };
        assert_eq!(
            suggestions(&facts),
            [
                Suggestion::BackupKernel,
                Suggestion::Snapshots,
                Suggestion::Mirrors,
                Suggestion::Cleanup { count: 39 }
            ]
        );
    }

    #[test]
    fn nothing_to_suggest_on_a_well_kept_system() {
        let facts = Facts {
            kernels: vec!["linux".into(), "linux-lts".into()],
            root_fs: "ext4".into(),
            snapper: false,
            orphans: 0,
            mirrors_ranked: true,
        };
        assert_eq!(suggestions(&facts), Vec::<Suggestion>::new());
        let snap = Facts {
            root_fs: "btrfs".into(),
            snapper: true,
            ..facts
        };
        assert_eq!(suggestions(&snap), Vec::<Suggestion>::new());
    }

    #[test]
    fn running_kernel() {
        assert!(is_running("7.2.11.zen1-1", "7.2.11-zen1-1-zen"));
        assert!(is_running("6.18.1.arch1-1", "6.18.1-arch1-1"));
        assert!(!is_running("7.2.11.arch1-1", "7.2.11-zen1-1-zen"));
        assert!(!is_running("6.18.1.arch1-1", ""));
    }

    #[test]
    fn kernel_names() {
        for k in [
            "linux",
            "linux-lts",
            "linux-zen",
            "linux-cachyos",
            "linux-cachyos-lts",
        ] {
            assert!(is_kernel(k), "{k}");
        }
        for k in [
            "linux-firmware",
            "linux-headers",
            "linux-cachyos-headers",
            "linux-api-headers",
        ] {
            assert!(!is_kernel(k), "{k}");
        }
    }
}
