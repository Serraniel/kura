// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! What a snapper setup has and what it still needs.
//!
//! kura never replaces an existing setup. It only adds what is missing and
//! never adds a second tool that takes snapshots before every update.

use std::collections::BTreeSet;

use crate::action::Action;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tool {
    Snapper,
    Timeshift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bootloader {
    Grub,
    SystemdBoot,
    Limine,
}

/// A snapper configuration from `/etc/snapper/configs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub name: String,
    /// `SUBVOLUME=` of the configuration; `None` when the file cannot be
    /// read (it is only readable by root on many systems).
    pub subvolume: Option<String>,
}

/// What the caller found on the system.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// File system of `/`.
    pub root_fs: String,
    /// `/.snapshots` is a mount of its own (a separate subvolume like `@snapshots`).
    pub snapshots_mounted: bool,
    /// Installed packages; only the names below matter.
    pub packages: BTreeSet<String>,
    pub configs: Vec<Config>,
    /// Tools that a pacman hook runs before transactions.
    pub hooks: BTreeSet<Tool>,
    pub bootloader: Option<Bootloader>,
    /// Enabled systemd units.
    pub units: BTreeSet<String>,
}

/// One thing kura would add.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Install(String),
    /// `snapper create-config` for `/`. With an existing `/.snapshots` mount
    /// the mount is kept and snapper's own subvolume is not created.
    CreateRootConfig {
        keep_mount: bool,
    },
    Enable(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Report {
    /// Snapshots need btrfs.
    #[default]
    Unsupported,
    /// Another tool already takes snapshots before updates; kura adds nothing.
    OtherTool(Tool),
    /// Everything is in place.
    Complete,
    /// These steps are missing, in the order they run.
    Missing(Vec<Step>),
}

/// The tool a pacman hook file runs, from its `Exec =` line.
pub fn hook_tool(hook: &str) -> Option<Tool> {
    let exec = hook.lines().find_map(|l| {
        let (key, value) = l.split_once('=')?;
        (key.trim() == "Exec").then_some(value.trim())
    })?;
    let program = exec.split_whitespace().next()?.rsplit('/').next()?;
    if program.starts_with("snap-pac") || program == "snapper" {
        Some(Tool::Snapper)
    } else if program.starts_with("timeshift") {
        Some(Tool::Timeshift)
    } else {
        None
    }
}

/// Parses a snapper configuration file for its `SUBVOLUME`.
pub fn config_subvolume(config: &str) -> Option<String> {
    config.lines().find_map(|l| {
        let value = l.trim().strip_prefix("SUBVOLUME=")?;
        Some(value.trim_matches('"').to_owned())
    })
}

/// Configuration names from `SNAPPER_CONFIGS` in `/etc/conf.d/snapper`.
pub fn config_names(conf: &str) -> Vec<String> {
    conf.lines()
        .find_map(|l| l.trim().strip_prefix("SNAPPER_CONFIGS="))
        .map(|v| {
            v.trim_matches('"')
                .split_whitespace()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Whether `/.snapshots` is mounted, from `/proc/mounts`.
pub fn snapshots_mounted(mounts: &str) -> bool {
    mounts
        .lines()
        .any(|l| l.split_whitespace().nth(1) == Some("/.snapshots"))
}

/// A line of `/proc/mounts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub device: String,
    pub fs: String,
    pub options: String,
}

/// The last mount on `target`, as the kernel sees it.
pub fn mount_entry(mounts: &str, target: &str) -> Option<Mount> {
    mounts.lines().rev().find_map(|l| {
        let mut cols = l.split_whitespace();
        let device = cols.next()?;
        if cols.next()? != target {
            return None;
        }
        Some(Mount {
            device: device.to_owned(),
            fs: cols.next()?.to_owned(),
            options: cols.next()?.to_owned(),
        })
    })
}

/// Splits missing steps into packages to install and actions to run.
/// Packages no repository offers are left out, like `limine-snapper-sync`
/// on plain Arch, where it is only in the AUR.
pub fn split(steps: &[Step], available: impl Fn(&str) -> bool) -> (Vec<String>, Vec<Action>) {
    let mut packages = Vec::new();
    let mut actions = Vec::new();
    for step in steps {
        match step {
            Step::Install(p) if available(p) => packages.push(p.clone()),
            Step::Install(_) => {}
            Step::CreateRootConfig { keep_mount } => actions.push(Action::SnapperRootConfig {
                keep_mount: *keep_mount,
            }),
            Step::Enable(unit) => actions.push(Action::Enable { unit: unit.clone() }),
        }
    }
    (packages, actions)
}

/// Compares the system with a complete setup.
pub fn check(facts: &Facts) -> Report {
    if facts.root_fs != "btrfs" {
        return Report::Unsupported;
    }
    let has = |p: &str| facts.packages.contains(p);
    // A configuration kura cannot read may cover `/`; never risk a second one.
    let root_config = facts
        .configs
        .iter()
        .any(|c| c.subvolume.as_deref().is_none_or(|s| s == "/"));
    if facts.hooks.contains(&Tool::Timeshift) && !root_config {
        return Report::OtherTool(Tool::Timeshift);
    }

    let mut steps = Vec::new();
    let install = |p: &str, steps: &mut Vec<Step>| {
        if !has(p) {
            steps.push(Step::Install(p.to_owned()));
        }
    };
    install("snapper", &mut steps);
    if facts.hooks.is_empty() {
        install("snap-pac", &mut steps);
    }
    match facts.bootloader {
        Some(Bootloader::Grub) => install("grub-btrfs", &mut steps),
        Some(Bootloader::Limine) => install("limine-snapper-sync", &mut steps),
        Some(Bootloader::SystemdBoot) | None => {}
    }
    if !root_config {
        steps.push(Step::CreateRootConfig {
            keep_mount: facts.snapshots_mounted,
        });
    }
    let mut enable = |unit: &str| {
        if !facts.units.contains(unit) {
            steps.push(Step::Enable(unit.to_owned()));
        }
    };
    enable("snapper-cleanup.timer");
    if facts.bootloader == Some(Bootloader::Grub) {
        enable("grub-btrfsd.service");
    }

    if steps.is_empty() {
        Report::Complete
    } else {
        Report::Missing(steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    fn btrfs() -> Facts {
        Facts {
            root_fs: "btrfs".into(),
            ..Facts::default()
        }
    }

    #[test]
    fn fresh_btrfs_with_grub_gets_everything() {
        let facts = Facts {
            bootloader: Some(Bootloader::Grub),
            snapshots_mounted: true,
            ..btrfs()
        };
        assert_eq!(
            check(&facts),
            Report::Missing(vec![
                Step::Install("snapper".into()),
                Step::Install("snap-pac".into()),
                Step::Install("grub-btrfs".into()),
                Step::CreateRootConfig { keep_mount: true },
                Step::Enable("snapper-cleanup.timer".into()),
                Step::Enable("grub-btrfsd.service".into()),
            ])
        );
    }

    #[test]
    fn complete_setup_needs_nothing() {
        // A CachyOS-like setup with limine.
        let facts = Facts {
            packages: set(&["snapper", "snap-pac", "limine-snapper-sync"]),
            configs: vec![Config {
                name: "root".into(),
                subvolume: Some("/".into()),
            }],
            hooks: [Tool::Snapper].into(),
            bootloader: Some(Bootloader::Limine),
            units: set(&["snapper-cleanup.timer"]),
            ..btrfs()
        };
        assert_eq!(check(&facts), Report::Complete);
    }

    #[test]
    fn only_adds_what_is_missing() {
        // snapper with a root config but no hook and no cleanup.
        let facts = Facts {
            packages: set(&["snapper"]),
            configs: vec![Config {
                name: "root".into(),
                subvolume: Some("/".into()),
            }],
            bootloader: Some(Bootloader::SystemdBoot),
            ..btrfs()
        };
        assert_eq!(
            check(&facts),
            Report::Missing(vec![
                Step::Install("snap-pac".into()),
                Step::Enable("snapper-cleanup.timer".into()),
            ])
        );
    }

    #[test]
    fn unreadable_configs_count_as_root() {
        let facts = Facts {
            packages: set(&["snapper", "snap-pac"]),
            configs: vec![Config {
                name: "default".into(),
                subvolume: None,
            }],
            hooks: [Tool::Snapper].into(),
            units: set(&["snapper-cleanup.timer"]),
            ..btrfs()
        };
        assert_eq!(check(&facts), Report::Complete);
        let home_only = Facts {
            configs: vec![Config {
                name: "home".into(),
                subvolume: Some("/home".into()),
            }],
            ..facts
        };
        assert_eq!(
            check(&home_only),
            Report::Missing(vec![Step::CreateRootConfig { keep_mount: false }])
        );
    }

    #[test]
    fn respects_other_tools() {
        let timeshift = Facts {
            hooks: [Tool::Timeshift].into(),
            ..btrfs()
        };
        assert_eq!(check(&timeshift), Report::OtherTool(Tool::Timeshift));
        // A snapper root config next to timeshift-autosnap: no second hook.
        let both = Facts {
            packages: set(&["snapper"]),
            configs: vec![Config {
                name: "root".into(),
                subvolume: Some("/".into()),
            }],
            units: set(&["snapper-cleanup.timer"]),
            ..timeshift
        };
        assert_eq!(check(&both), Report::Complete);
        let ext4 = Facts {
            root_fs: "ext4".into(),
            ..Facts::default()
        };
        assert_eq!(check(&ext4), Report::Unsupported);
    }

    #[test]
    fn splits_packages_and_actions() {
        let steps = [
            Step::Install("snapper".into()),
            Step::Install("limine-snapper-sync".into()),
            Step::CreateRootConfig { keep_mount: true },
            Step::Enable("snapper-cleanup.timer".into()),
        ];
        let (packages, actions) = split(&steps, |p| p != "limine-snapper-sync");
        assert_eq!(packages, ["snapper"]);
        assert_eq!(
            actions,
            [
                Action::SnapperRootConfig { keep_mount: true },
                Action::Enable {
                    unit: "snapper-cleanup.timer".into()
                }
            ]
        );
    }

    #[test]
    fn finds_mounts() {
        let mounts = "/dev/sda2 /.snapshots btrfs rw,subvol=/@old 0 0\n/dev/sda2 /.snapshots btrfs rw,subvolid=257,subvol=/@snapshots 0 0\n";
        assert_eq!(
            mount_entry(mounts, "/.snapshots"),
            Some(Mount {
                device: "/dev/sda2".into(),
                fs: "btrfs".into(),
                options: "rw,subvolid=257,subvol=/@snapshots".into(),
            })
        );
        assert_eq!(mount_entry(mounts, "/home"), None);
    }

    #[test]
    fn parses_system_files() {
        let hook = "[Trigger]\nOperation = Upgrade\n\n[Action]\nWhen = PreTransaction\nExec = /usr/share/libalpm/scripts/snap-pac pre\n";
        assert_eq!(hook_tool(hook), Some(Tool::Snapper));
        assert_eq!(
            hook_tool("[Action]\nExec=/usr/bin/timeshift-autosnap\n"),
            Some(Tool::Timeshift)
        );
        assert_eq!(hook_tool("[Action]\nExec = /usr/bin/kura-helper record\n"), None);
        assert_eq!(hook_tool("[Action]\n"), None);
        assert_eq!(
            config_subvolume("# comment\nSUBVOLUME=\"/home\"\nFSTYPE=\"btrfs\"\n").as_deref(),
            Some("/home")
        );
        assert_eq!(config_subvolume("FSTYPE=\"btrfs\"\n"), None);
        assert_eq!(
            config_names("# list\nSNAPPER_CONFIGS=\"root home\"\n"),
            ["root", "home"]
        );
        assert_eq!(config_names(""), Vec::<String>::new());
        let mounts =
            "/dev/sda2 / btrfs rw,subvol=/@ 0 0\n/dev/sda2 /.snapshots btrfs rw,subvol=/@snapshots 0 0\n";
        assert!(snapshots_mounted(mounts));
        assert!(!snapshots_mounted("/dev/sda2 / btrfs rw 0 0\n"));
    }
}
