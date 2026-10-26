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

/// The boot menu work for adding (`add`) or removing `kernel`: GRUB's menu
/// when no pacman hook writes it, and after adding, the new kernel as the
/// default where kura may choose it.
pub fn boot_actions(conf: &Config, kernel: &str, add: bool) -> Vec<Action> {
    let loader = crate::system::bootloader();
    let mut out = Vec::new();
    if loader == Some(Bootloader::Grub)
        && !crate::system::hook_texts(conf)
            .iter()
            .any(|h| kura_core::kernels::hook_updates_grub(h))
    {
        out.push(Action::RegenerateGrub);
    }
    let saved = kura_core::kernels::grub_uses_saved_default(
        &fs::read_to_string("/etc/default/grub").unwrap_or_default(),
    );
    if add && (loader == Some(Bootloader::SystemdBoot) || loader == Some(Bootloader::Grub) && saved) {
        out.push(Action::SetDefaultKernel {
            kernel: kernel.to_owned(),
        });
    }
    out
}

/// Where kura remembers a kernel it added and the one that ran before, to
/// offer removing the old one after the new one started.
pub const ADDED: &str = "/var/lib/kura/kernel-added.json";

/// `(added, previous)` once the added kernel is the running one.
pub fn replaced_kernel() -> Option<(String, String)> {
    let text = fs::read_to_string(ADDED).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let added = v.get("added")?.as_str()?.to_owned();
    let previous = v.get("previous")?.as_str()?.to_owned();
    (running_package().as_deref() == Some(added.as_str()) && added != previous).then_some((added, previous))
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
        plan.actions = boot_actions(conf, kernel, add);
    }
    Ok(outcome)
}
