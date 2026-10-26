// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Kernels on this system: which one runs, DKMS modules, the boot menu, and
//! plans that add or remove a kernel.

use std::fs;
use std::path::Path;

use kura_core::action::Action;
use kura_core::health::kernel_packages;
use kura_core::plan::PlanKind;
use kura_core::snapshots::Bootloader;

use crate::prepare::{Outcome, Request};
use crate::{Config, Error};

/// `(release, package)` for every installed kernel, from `/usr/lib/modules`.
pub fn pkgbases() -> Vec<(String, String)> {
    fs::read_dir("/usr/lib/modules")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let package = fs::read_to_string(e.path().join("pkgbase")).ok()?;
            Some((
                e.file_name().to_string_lossy().into_owned(),
                package.trim().to_owned(),
            ))
        })
        .collect()
}

/// The package of the running kernel.
pub fn running_package() -> Option<String> {
    let release = fs::read_to_string("/proc/sys/kernel/osrelease").ok()?;
    let path = Path::new("/usr/lib/modules").join(release.trim()).join("pkgbase");
    Some(fs::read_to_string(path).ok()?.trim().to_owned())
}

/// Modules DKMS builds for every kernel.
pub fn dkms_modules() -> Vec<String> {
    fs::read_dir("/var/lib/dkms")
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Whether DKMS built `module` for the kernel `release`.
pub fn dkms_built(module: &str, release: &str) -> bool {
    fs::read_dir(Path::new("/var/lib/dkms").join(module))
        .into_iter()
        .flatten()
        .flatten()
        .any(|version| version.path().join(release).is_dir())
}

/// DKMS modules missing for the kernel package, once it is installed.
pub fn missing_dkms(package: &str) -> Vec<String> {
    let bases = pkgbases();
    let Some(release) = kura_core::kernels::release_of(package, &bases) else {
        return Vec::new();
    };
    kura_core::kernels::missing_dkms(&dkms_modules(), release, dkms_built)
}

/// The boot menu work kura must do itself after kernels change.
pub fn boot_actions(conf: &Config) -> Vec<Action> {
    let grub = crate::system::bootloader() == Some(Bootloader::Grub);
    let hooked = crate::system::hook_texts(conf)
        .iter()
        .any(|h| kura_core::kernels::hook_updates_grub(h));
    if grub && !hooked {
        vec![Action::RegenerateGrub]
    } else {
        Vec::new()
    }
}

/// The plan that installs `kernel` (with headers when DKMS needs them) or
/// removes it with its headers.
pub fn prepare(conf: &Config, workdir: Option<&Path>, kernel: &str, add: bool) -> Result<Outcome, Error> {
    let set = crate::installed(conf)?;
    let packages: Vec<String> = kernel_packages(kernel, set.get("dkms").is_some())
        .into_iter()
        .filter(|p| add || set.get(p).is_some())
        .collect();
    let request = if add {
        Request::Install(packages)
    } else {
        Request::Remove(packages)
    };
    let mut outcome = crate::prepare::prepare(conf, workdir, &request, add, &[])?;
    if let Ok(plan) = &mut outcome.plan {
        plan.kind = PlanKind::Kernel;
        plan.actions = boot_actions(conf);
    }
    Ok(outcome)
}
