// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Kernels: which ones kura knows, which may be removed, and whether the
//! modules that DKMS builds exist for a kernel.

use crate::health::is_kernel;

/// Kernels kura offers, in the order it shows them. The text for each is
/// the Fluent message `kernel-<name>`.
pub const KNOWN: &[&str] = &[
    "linux",
    "linux-lts",
    "linux-zen",
    "linux-hardened",
    "linux-cachyos",
    "linux-cachyos-lts",
];

/// Why a kernel cannot be removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    /// The computer runs it right now.
    Running,
    /// Nothing could start without it.
    Last,
}

/// Whether `kernel` may be removed. `installed` are the installed kernel
/// packages, `running` the package of the running kernel.
pub fn can_remove(kernel: &str, installed: &[String], running: Option<&str>) -> Result<(), Keep> {
    if running == Some(kernel) {
        return Err(Keep::Running);
    }
    if installed.iter().filter(|k| is_kernel(k) && *k != kernel).count() == 0 {
        return Err(Keep::Last);
    }
    Ok(())
}

/// The kernel release a kernel package installs, from the `pkgbase` files
/// in `/usr/lib/modules/*/pkgbase`: `(release, package)` pairs.
pub fn release_of<'a>(package: &str, pkgbases: &'a [(String, String)]) -> Option<&'a str> {
    pkgbases
        .iter()
        .find(|(_, p)| p == package)
        .map(|(release, _)| release.as_str())
}

/// DKMS modules missing for a kernel release. `modules` are the module
/// names DKMS manages (directories in `/var/lib/dkms`); `built` checks
/// whether a module file exists for the release.
pub fn missing_dkms(modules: &[String], release: &str, built: impl Fn(&str, &str) -> bool) -> Vec<String> {
    modules.iter().filter(|m| !built(m, release)).cloned().collect()
}

/// Whether a pacman hook rewrites GRUB's menu itself.
pub fn hook_updates_grub(hook: &str) -> bool {
    hook.lines().any(|l| {
        l.split_once('=').is_some_and(|(k, v)| {
            k.trim() == "Exec" && (v.contains("grub-mkconfig") || v.contains("update-grub"))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn keeps_the_running_and_the_last_kernel() {
        let installed = names(&["linux", "linux-lts", "linux-firmware"]);
        assert_eq!(can_remove("linux", &installed, Some("linux")), Err(Keep::Running));
        assert_eq!(can_remove("linux-lts", &installed, Some("linux")), Ok(()));
        let one = names(&["linux-zen", "linux-firmware"]);
        assert_eq!(can_remove("linux-zen", &one, None), Err(Keep::Last));
        assert!(KNOWN.iter().all(|k| is_kernel(k)));
    }

    #[test]
    fn grub_hooks() {
        assert!(hook_updates_grub(
            "[Action]\nExec = /usr/bin/grub-mkconfig -o /boot/grub/grub.cfg\n"
        ));
        assert!(hook_updates_grub("[Action]\nExec=/usr/bin/update-grub\n"));
        assert!(!hook_updates_grub(
            "[Action]\nExec = /usr/bin/kura-helper record\n"
        ));
    }

    #[test]
    fn finds_releases_and_missing_modules() {
        let bases = vec![
            ("6.18.1-arch1-1".to_owned(), "linux".to_owned()),
            ("6.12.40-1-lts".to_owned(), "linux-lts".to_owned()),
        ];
        assert_eq!(release_of("linux-lts", &bases), Some("6.12.40-1-lts"));
        assert_eq!(release_of("linux-zen", &bases), None);
        let modules = names(&["nvidia", "vboxhost"]);
        let built = |m: &str, r: &str| m == "nvidia" && r.ends_with("lts");
        assert_eq!(missing_dkms(&modules, "6.12.40-1-lts", built), ["vboxhost"]);
        assert_eq!(
            missing_dkms(&modules, "6.18.1-arch1-1", built),
            ["nvidia", "vboxhost"]
        );
    }
}
