// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Creates a snapper configuration next to an existing `.snapshots` mount.
//! Changes the system: runs only in the e2e container, which prepares a
//! btrfs image and sets `KURA_TEST_BTRFS` to its mount point.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[test]
#[ignore = "needs root and a prepared btrfs mount (e2e container only)"]
fn keeps_the_existing_snapshots_mount() {
    // The coverage job runs ignored tests too; only the e2e job sets this.
    let Ok(root) = std::env::var("KURA_TEST_BTRFS") else {
        eprintln!("KURA_TEST_BTRFS not set, skipping");
        return;
    };
    let root = Path::new(&root);
    let dir = root.join(".snapshots");
    let target = dir.to_str().unwrap();
    let before = std::fs::read_to_string("/proc/mounts").unwrap();
    let before = kura_core::snapshots::mount_entry(&before, target).expect("mounted before");

    kura_helper::actions::snapper_config("kuratest", root, true, &std::sync::mpsc::channel().0).unwrap();

    let after = std::fs::read_to_string("/proc/mounts").unwrap();
    let after = kura_core::snapshots::mount_entry(&after, target).expect("mounted after");
    assert_eq!(after.options, before.options);
    assert!(Path::new("/etc/snapper/configs/kuratest").exists());
    let mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o750);
}
