// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Driver suggestions from the hardware kura finds. The rules below are the
//! built-in defaults; the `kura-bundles` repository will extend them.

use std::collections::BTreeSet;

use crate::health::kernel_packages;

pub const INTEL: u16 = 0x8086;
pub const AMD: u16 = 0x1002;
pub const NVIDIA: u16 = 0x10de;

/// The first NVIDIA device id of the Turing generation; the open kernel
/// modules support Turing and everything newer.
const NVIDIA_TURING: u16 = 0x1e00;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVendor {
    Intel,
    Amd,
    Other,
}

/// `vendor_id` from `/proc/cpuinfo`.
pub fn cpu_vendor(cpuinfo: &str) -> CpuVendor {
    let id = cpuinfo.lines().find_map(|l| {
        let (key, value) = l.split_once(':')?;
        (key.trim() == "vendor_id").then(|| value.trim())
    });
    match id {
        Some("GenuineIntel") => CpuVendor::Intel,
        Some("AuthenticAMD") => CpuVendor::Amd,
        _ => CpuVendor::Other,
    }
}

/// A graphics card, by its PCI ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gpu {
    pub vendor: u16,
    pub device: u16,
}

#[derive(Debug, Clone, Default)]
pub struct Facts {
    pub cpu: Option<CpuVendor>,
    pub gpus: Vec<Gpu>,
    pub installed: BTreeSet<String>,
    /// The multilib repository is enabled, so 32-bit drivers for games work.
    pub multilib: bool,
    /// Installed kernel packages; DKMS drivers need their headers.
    pub kernels: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Processor fixes loaded at every start.
    Microcode,
    Graphics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub kind: Kind,
    /// The maker, as people know it.
    pub vendor: &'static str,
    pub packages: Vec<String>,
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

fn graphics(gpu: Gpu, facts: &Facts) -> Option<(&'static str, Vec<String>)> {
    let (vendor, mut packages, lib32) = match gpu.vendor {
        AMD => (
            "AMD",
            names(&["mesa", "vulkan-radeon"]),
            names(&["lib32-mesa", "lib32-vulkan-radeon"]),
        ),
        INTEL => (
            "Intel",
            names(&["mesa", "vulkan-intel", "intel-media-driver"]),
            names(&["lib32-mesa", "lib32-vulkan-intel"]),
        ),
        NVIDIA if gpu.device >= NVIDIA_TURING => {
            // Someone chose a driver already (perhaps a legacy one); leave it.
            if facts
                .installed
                .iter()
                .any(|p| p.starts_with("nvidia") && p.ends_with("utils"))
            {
                return None;
            }
            let mut p = names(&["nvidia-open-dkms", "nvidia-utils"]);
            for kernel in &facts.kernels {
                p.extend(kernel_packages(kernel, true).into_iter().skip(1));
            }
            ("NVIDIA", p, names(&["lib32-nvidia-utils"]))
        }
        _ => return None,
    };
    if facts.multilib {
        packages.extend(lib32);
    }
    Some((vendor, packages))
}

/// What to install for this hardware, most important first. Only packages
/// that are missing and that a repository offers are listed.
pub fn suggest(facts: &Facts, available: impl Fn(&str) -> bool) -> Vec<Suggestion> {
    let missing = |list: Vec<String>| -> Vec<String> {
        list.into_iter()
            .filter(|p| !facts.installed.contains(p) && available(p))
            .collect()
    };
    let mut out = Vec::new();
    let microcode = match facts.cpu {
        Some(CpuVendor::Intel) => Some(("Intel", "intel-ucode")),
        Some(CpuVendor::Amd) => Some(("AMD", "amd-ucode")),
        _ => None,
    };
    if let Some((vendor, package)) = microcode {
        let packages = missing(names(&[package]));
        if !packages.is_empty() {
            out.push(Suggestion {
                kind: Kind::Microcode,
                vendor,
                packages,
            });
        }
    }
    let mut seen = BTreeSet::new();
    for gpu in &facts.gpus {
        if !seen.insert(gpu.vendor) {
            continue;
        }
        if let Some((vendor, list)) = graphics(*gpu, facts) {
            let packages = missing(list);
            if !packages.is_empty() {
                out.push(Suggestion {
                    kind: Kind::Graphics,
                    vendor,
                    packages,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn reads_the_cpu_vendor() {
        assert_eq!(
            cpu_vendor("processor : 0\nvendor_id\t: AuthenticAMD\n"),
            CpuVendor::Amd
        );
        assert_eq!(cpu_vendor("vendor_id : GenuineIntel\n"), CpuVendor::Intel);
        assert_eq!(cpu_vendor("CPU implementer : 0x41\n"), CpuVendor::Other);
    }

    #[test]
    fn suggests_microcode_and_graphics() {
        let facts = Facts {
            cpu: Some(CpuVendor::Amd),
            gpus: vec![Gpu {
                vendor: AMD,
                device: 0x744c,
            }],
            installed: set(&["mesa", "lib32-mesa"]),
            multilib: true,
            kernels: vec!["linux".into()],
        };
        assert_eq!(
            suggest(&facts, |_| true),
            [
                Suggestion {
                    kind: Kind::Microcode,
                    vendor: "AMD",
                    packages: names(&["amd-ucode"]),
                },
                Suggestion {
                    kind: Kind::Graphics,
                    vendor: "AMD",
                    packages: names(&["vulkan-radeon", "lib32-vulkan-radeon"]),
                },
            ]
        );
        // Without multilib no 32-bit drivers; nothing missing, nothing said.
        let done = Facts {
            installed: set(&["amd-ucode", "mesa", "vulkan-radeon"]),
            multilib: false,
            ..facts
        };
        assert_eq!(suggest(&done, |_| true), []);
    }

    #[test]
    fn nvidia_gets_the_open_driver_for_every_kernel() {
        let facts = Facts {
            cpu: Some(CpuVendor::Intel),
            gpus: vec![
                Gpu {
                    vendor: INTEL,
                    device: 0x9a49,
                },
                Gpu {
                    vendor: NVIDIA,
                    device: 0x2484,
                },
            ],
            installed: set(&["intel-ucode", "mesa", "vulkan-intel", "intel-media-driver"]),
            multilib: false,
            kernels: vec!["linux".into(), "linux-lts".into()],
        };
        assert_eq!(
            suggest(&facts, |_| true),
            [Suggestion {
                kind: Kind::Graphics,
                vendor: "NVIDIA",
                packages: names(&[
                    "nvidia-open-dkms",
                    "nvidia-utils",
                    "linux-headers",
                    "linux-lts-headers"
                ]),
            }]
        );
        // A driver the user chose stays; old cards get no automatic suggestion.
        let chosen = Facts {
            installed: set(&[
                "intel-ucode",
                "mesa",
                "vulkan-intel",
                "intel-media-driver",
                "nvidia-580xx-utils",
            ]),
            ..facts.clone()
        };
        assert_eq!(suggest(&chosen, |_| true), []);
        let pascal = Facts {
            gpus: vec![Gpu {
                vendor: NVIDIA,
                device: 0x1b80,
            }],
            ..facts.clone()
        };
        assert_eq!(suggest(&pascal, |_| true), []);
        // Packages no repository offers are left out.
        assert_eq!(
            suggest(&facts, |p| p != "nvidia-open-dkms")[0].packages[0],
            "nvidia-utils"
        );
    }
}
