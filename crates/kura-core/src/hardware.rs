// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading hardware facts from the text files Linux provides. The caller
//! reads the files; these functions only parse them.

/// `model name` from `/proc/cpuinfo`.
pub fn cpu_model(cpuinfo: &str) -> Option<String> {
    cpuinfo.lines().find_map(|l| {
        let (key, value) = l.split_once(':')?;
        (key.trim() == "model name").then(|| value.trim().to_owned())
    })
}

/// `MemTotal` from `/proc/meminfo`, in bytes.
pub fn mem_total(meminfo: &str) -> Option<u64> {
    meminfo.lines().find_map(|l| {
        let rest = l.strip_prefix("MemTotal:")?;
        let kib: u64 = rest.split_whitespace().next()?.parse().ok()?;
        Some(kib * 1024)
    })
}

/// File system type of `/` from `/proc/mounts`. The last mount wins, as in the kernel.
pub fn root_fs(mounts: &str) -> Option<String> {
    mounts.lines().rev().find_map(|l| {
        let mut cols = l.split_whitespace();
        let _dev = cols.next()?;
        let mount = cols.next()?;
        let fs = cols.next()?;
        (mount == "/").then(|| fs.to_owned())
    })
}

/// Used and total bytes from `df --block-size=1 --output=used,size /`.
pub fn df_usage(df: &str) -> Option<(u64, u64)> {
    let line = df.lines().nth(1)?;
    let mut cols = line.split_whitespace();
    Some((cols.next()?.parse().ok()?, cols.next()?.parse().ok()?))
}

fn bracketed(name: &str) -> &str {
    name.find('[')
        .and_then(|s| name[s + 1..].find(']').map(|e| &name[s + 1..s + 1 + e]))
        .unwrap_or(name)
}

/// A readable name for a PCI device from `pci.ids`, preferring the marketing
/// names in brackets: `AMD/ATI Radeon RX 7900 XT/7900 XTX/7900M`.
pub fn pci_name(pci_ids: &str, vendor: u16, device: u16) -> Option<String> {
    let vendor_hex = format!("{vendor:04x}");
    let device_hex = format!("{device:04x}");
    let mut vendor_name: Option<&str> = None;
    for line in pci_ids.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(name) = vendor_name {
            if line.starts_with("\t\t") {
                continue;
            }
            if let Some(rest) = line.strip_prefix('\t') {
                if let Some(dev) = rest.strip_prefix(&device_hex) {
                    return Some(format!("{} {}", bracketed(name), bracketed(dev.trim())));
                }
                continue;
            }
            // Next vendor: the device is unknown.
            return Some(bracketed(name).to_owned());
        }
        if let Some(name) = line.strip_prefix(&vendor_hex) {
            vendor_name = Some(name.trim());
        }
    }
    vendor_name.map(|n| bracketed(n).to_owned())
}

/// The x86-64 microarchitecture level (1 to 4) from the `flags` line of
/// `/proc/cpuinfo`, as glibc and CachyOS define the levels.
pub fn x86_64_level(cpuinfo: &str) -> u8 {
    let Some(flags) = cpuinfo.lines().find_map(|l| {
        let (key, value) = l.split_once(':')?;
        (key.trim() == "flags").then_some(value)
    }) else {
        return 0;
    };
    let has: Vec<&str> = flags.split_whitespace().collect();
    let all = |list: &[&str]| list.iter().all(|f| has.contains(f));
    let v2 = ["cx16", "lahf_lm", "popcnt", "sse4_1", "sse4_2", "ssse3"];
    let v3 = [
        "avx", "avx2", "bmi1", "bmi2", "f16c", "fma", "abm", "movbe", "xsave",
    ];
    let v4 = ["avx512f", "avx512bw", "avx512cd", "avx512dq", "avx512vl"];
    if !all(&["lm"]) {
        0
    } else if !all(&v2) {
        1
    } else if !all(&v3) {
        2
    } else if !all(&v4) {
        3
    } else {
        4
    }
}

/// Parses a sysfs hex id like `0x1002`.
pub fn hex_id(s: &str) -> Option<u16> {
    u16::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_and_memory() {
        let cpuinfo =
            "processor\t: 0\nvendor_id\t: AuthenticAMD\nmodel name\t: AMD Ryzen 7 7800X3D 8-Core Processor\n";
        assert_eq!(
            cpu_model(cpuinfo).as_deref(),
            Some("AMD Ryzen 7 7800X3D 8-Core Processor")
        );
        assert_eq!(cpu_model("nothing"), None);
        assert_eq!(
            mem_total("MemTotal:       32768000 kB\nMemFree: 1 kB\n"),
            Some(32_768_000 * 1024)
        );
        assert_eq!(mem_total("MemFree: 1 kB\n"), None);
    }

    #[test]
    fn mounts_and_disk() {
        let mounts =
            "proc /proc proc rw 0 0\n/dev/nvme0n1p2 / ext4 rw 0 0\n/dev/nvme0n1p2 / btrfs rw,subvol=/@ 0 0\n";
        assert_eq!(root_fs(mounts).as_deref(), Some("btrfs"));
        assert_eq!(root_fs("proc /proc proc rw 0 0\n"), None);
        assert_eq!(
            df_usage("     Used        1B-blocks\n412000000000 1000000000000\n"),
            Some((412_000_000_000, 1_000_000_000_000))
        );
        assert_eq!(df_usage("header only\n"), None);
    }

    #[test]
    fn x86_64_levels() {
        let v2 = "lm cx16 lahf_lm popcnt sse4_1 sse4_2 ssse3";
        let v3 = format!("{v2} avx avx2 bmi1 bmi2 f16c fma abm movbe xsave");
        let v4 = format!("{v3} avx512f avx512bw avx512cd avx512dq avx512vl");
        assert_eq!(x86_64_level(&format!("flags\t: {v2}\n")), 2);
        assert_eq!(x86_64_level(&format!("flags\t: {v3}\n")), 3);
        assert_eq!(x86_64_level(&format!("processor : 0\nflags : {v4}\n")), 4);
        assert_eq!(x86_64_level("flags : lm\n"), 1);
        assert_eq!(x86_64_level("flags : fpu\n"), 0);
        assert_eq!(x86_64_level("Features : neon\n"), 0);
    }

    #[test]
    fn pci_names() {
        let ids = "# comment\n1002  Advanced Micro Devices, Inc. [AMD/ATI]\n\t744c  Navi 31 [Radeon RX 7900 XT/7900 XTX/7900M]\n\t\t1002 0e3b  Subsystem\n\t7480  Navi 33\n10de  NVIDIA Corporation\n\t1b80  GP104 [GeForce GTX 1080]\n8086  Intel Corporation\n";
        assert_eq!(
            pci_name(ids, 0x1002, 0x744c).as_deref(),
            Some("AMD/ATI Radeon RX 7900 XT/7900 XTX/7900M")
        );
        assert_eq!(pci_name(ids, 0x1002, 0x7480).as_deref(), Some("AMD/ATI Navi 33"));
        assert_eq!(
            pci_name(ids, 0x10de, 0x1b80).as_deref(),
            Some("NVIDIA Corporation GeForce GTX 1080")
        );
        assert_eq!(
            pci_name(ids, 0x10de, 0x9999).as_deref(),
            Some("NVIDIA Corporation")
        );
        assert_eq!(
            pci_name(ids, 0x8086, 0x1234).as_deref(),
            Some("Intel Corporation")
        );
        assert_eq!(pci_name(ids, 0xffff, 0x0001), None);
        assert_eq!(hex_id("0x1002\n"), Some(0x1002));
        assert_eq!(hex_id("zz"), None);
    }
}
