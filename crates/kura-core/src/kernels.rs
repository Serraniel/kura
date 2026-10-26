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

/// The id `grub-set-default` takes for the normal (not fallback, not
/// recovery) entry that starts `kernel`, from `grub.cfg`. Entries inside a
/// submenu get the submenu id in front: `submenu>entry`.
pub fn grub_entry_id(cfg: &str, kernel: &str) -> Option<String> {
    let image = format!("/vmlinuz-{kernel}");
    let id_of = |line: &str| {
        let rest = line.split("$menuentry_id_option '").nth(1)?;
        Some(rest.split('\'').next()?.to_owned())
    };
    let indented = |line: &str| line.starts_with([' ', '\t']);
    let mut submenu: Option<String> = None;
    // The current entry: its full id, and whether it is a fallback or recovery one.
    let mut entry: Option<(String, bool)> = None;
    for line in cfg.lines() {
        let t = line.trim_start();
        let mut words = t.split_whitespace();
        match words.next() {
            Some("submenu") => submenu = id_of(t),
            Some("menuentry") => {
                let title = t.to_lowercase();
                let unwanted = title.contains("fallback") || title.contains("recovery");
                let inside = indented(line).then_some(submenu.as_ref()).flatten();
                entry = id_of(t).map(|id| {
                    let full = inside.map_or_else(|| id.clone(), |s| format!("{s}>{id}"));
                    (full, unwanted)
                });
            }
            Some("linux") if words.next().is_some_and(|p| p.ends_with(&image)) => {
                if let Some((id, false)) = &entry {
                    return Some(id.clone());
                }
            }
            _ => {}
        }
    }
    None
}

/// The systemd-boot entry (file name without `.conf`) that starts `kernel`
/// with its normal initramfs. `entries` are `(file name, content)` pairs.
pub fn sdboot_entry(entries: &[(String, String)], kernel: &str) -> Option<String> {
    let image = format!("/vmlinuz-{kernel}");
    entries
        .iter()
        .filter(|(_, c)| {
            c.lines().any(|l| {
                let mut w = l.split_whitespace();
                w.next() == Some("linux") && w.next().is_some_and(|p| p.ends_with(&image))
            }) && !c.contains("fallback")
        })
        .map(|(name, _)| name.trim_end_matches(".conf").to_owned())
        .next()
}

/// `GRUB_DEFAULT=saved` in `/etc/default/grub`: only then kura may choose
/// the default entry.
pub fn grub_uses_saved_default(defaults: &str) -> bool {
    defaults.lines().any(|l| {
        l.trim()
            .strip_prefix("GRUB_DEFAULT=")
            .is_some_and(|v| v.trim_matches('"') == "saved")
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
    fn finds_boot_entries() {
        let cfg = "menuentry 'Arch Linux' --class arch $menuentry_id_option 'gnulinux-simple-abc' {\n\tlinux\t/vmlinuz-linux root=UUID=abc rw\n}\nsubmenu 'Advanced options for Arch Linux' $menuentry_id_option 'gnulinux-advanced-abc' {\n\tmenuentry 'Arch Linux, with Linux linux-lts' --class arch $menuentry_id_option 'gnulinux-linux-lts-advanced-abc' {\n\t\tlinux\t/vmlinuz-linux-lts root=UUID=abc rw\n\t}\n\tmenuentry 'Arch Linux, with Linux linux-lts (fallback initramfs)' --class arch $menuentry_id_option 'gnulinux-linux-lts-fallback-abc' {\n\t\tlinux\t/vmlinuz-linux-lts root=UUID=abc rw\n\t}\n}\n";
        assert_eq!(
            grub_entry_id(cfg, "linux").as_deref(),
            Some("gnulinux-simple-abc")
        );
        assert_eq!(
            grub_entry_id(cfg, "linux-lts").as_deref(),
            Some("gnulinux-advanced-abc>gnulinux-linux-lts-advanced-abc")
        );
        assert_eq!(grub_entry_id(cfg, "linux-zen"), None);

        let entries = vec![
            (
                "arch-fallback.conf".to_owned(),
                "title Arch (fallback)\nlinux /vmlinuz-linux\ninitrd /initramfs-linux-fallback.img\n"
                    .to_owned(),
            ),
            (
                "arch-zen.conf".to_owned(),
                "title Arch zen\nlinux /vmlinuz-linux-zen\ninitrd /initramfs-linux-zen.img\n".to_owned(),
            ),
            (
                "arch.conf".to_owned(),
                "title Arch\nlinux /vmlinuz-linux\ninitrd /initramfs-linux.img\n".to_owned(),
            ),
        ];
        assert_eq!(sdboot_entry(&entries, "linux").as_deref(), Some("arch"));
        assert_eq!(sdboot_entry(&entries, "linux-zen").as_deref(), Some("arch-zen"));
        assert_eq!(sdboot_entry(&entries, "linux-lts"), None);

        assert!(grub_uses_saved_default(
            "GRUB_DEFAULT=saved\nGRUB_SAVEDEFAULT=true\n"
        ));
        assert!(grub_uses_saved_default("GRUB_DEFAULT=\"saved\"\n"));
        assert!(!grub_uses_saved_default("GRUB_DEFAULT=0\n"));
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
