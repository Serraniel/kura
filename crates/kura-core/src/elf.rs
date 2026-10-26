// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Just enough ELF to find which shared libraries a program needs
//! (`DT_NEEDED`) and where it looks for them (`DT_RUNPATH`, `DT_RPATH`).
//! Used to find AUR packages that broke because a library changed.

/// What a binary links against.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Linking {
    /// 32-bit binaries look in `/usr/lib32`.
    pub is_32bit: bool,
    pub needed: Vec<String>,
    /// Extra library directories, `$ORIGIN` not yet replaced.
    pub runpath: Vec<String>,
}

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const DT_NULL: u64 = 0;
const DT_NEEDED: u64 = 1;
const DT_STRTAB: u64 = 5;
const DT_RPATH: u64 = 15;
const DT_RUNPATH: u64 = 29;

struct Reader<'a> {
    data: &'a [u8],
    wide: bool,
}

impl Reader<'_> {
    fn u16(&self, at: usize) -> Option<u16> {
        Some(u16::from_le_bytes(self.data.get(at..at + 2)?.try_into().ok()?))
    }
    fn u32(&self, at: usize) -> Option<u32> {
        Some(u32::from_le_bytes(self.data.get(at..at + 4)?.try_into().ok()?))
    }
    fn u64(&self, at: usize) -> Option<u64> {
        Some(u64::from_le_bytes(self.data.get(at..at + 8)?.try_into().ok()?))
    }
    /// An address-sized word.
    fn word(&self, at: usize) -> Option<u64> {
        if self.wide {
            self.u64(at)
        } else {
            self.u32(at).map(u64::from)
        }
    }
    fn string(&self, at: usize) -> Option<String> {
        let rest = self.data.get(at..)?;
        let end = rest.iter().position(|b| *b == 0)?;
        Some(String::from_utf8_lossy(&rest[..end]).into_owned())
    }
}

/// Library directories from the files in `/etc/ld.so.conf.d`.
pub fn ld_conf_dirs(conf: &str) -> Vec<String> {
    conf.lines()
        .map(str::trim)
        .filter(|l| l.starts_with('/'))
        .map(|l| l.trim_end_matches('/').to_owned())
        .collect()
}

/// Whether the bytes start like an ELF file.
pub fn is_elf(data: &[u8]) -> bool {
    data.starts_with(b"\x7fELF")
}

/// Reads the dynamic linking information of a little-endian ELF file.
/// `None` for anything else, including static binaries.
pub fn linking(data: &[u8]) -> Option<Linking> {
    if !is_elf(data) || data.get(5) != Some(&1) {
        return None;
    }
    let wide = match data.get(4)? {
        1 => false,
        2 => true,
        _ => return None,
    };
    let r = Reader { data, wide };
    let (phoff, phentsize, phnum) = if wide {
        (r.u64(0x20)?, r.u16(0x36)?, r.u16(0x38)?)
    } else {
        (u64::from(r.u32(0x1c)?), r.u16(0x2a)?, r.u16(0x2c)?)
    };
    // Program headers: the loadable segments map addresses to file offsets.
    let mut loads: Vec<(u64, u64, u64)> = Vec::new(); // (vaddr, offset, filesz)
    let mut dynamic: Option<(u64, u64)> = None; // (offset, size)
    for i in 0..usize::from(phnum) {
        let at = usize::try_from(phoff).ok()? + i * usize::from(phentsize);
        let kind = r.u32(at)?;
        let (offset, vaddr, filesz) = if wide {
            (r.u64(at + 8)?, r.u64(at + 16)?, r.u64(at + 32)?)
        } else {
            (
                u64::from(r.u32(at + 4)?),
                u64::from(r.u32(at + 8)?),
                u64::from(r.u32(at + 16)?),
            )
        };
        match kind {
            PT_LOAD => loads.push((vaddr, offset, filesz)),
            PT_DYNAMIC => dynamic = Some((offset, filesz)),
            _ => {}
        }
    }
    let (dyn_off, dyn_size) = dynamic?;
    let entry = if wide { 16 } else { 8 };
    let mut needed = Vec::new();
    let mut paths = Vec::new();
    let mut strtab_addr = None;
    let mut at = usize::try_from(dyn_off).ok()?;
    let end = at + usize::try_from(dyn_size).ok()?;
    while at + entry <= end {
        let tag = r.word(at)?;
        let value = r.word(at + entry / 2)?;
        match tag {
            DT_NULL => break,
            DT_NEEDED => needed.push(value),
            DT_RPATH | DT_RUNPATH => paths.push(value),
            DT_STRTAB => strtab_addr = Some(value),
            _ => {}
        }
        at += entry;
    }
    let strtab_addr = strtab_addr?;
    let strtab = loads
        .iter()
        .find(|(vaddr, _, size)| strtab_addr >= *vaddr && strtab_addr < vaddr + size)
        .map(|(vaddr, offset, _)| strtab_addr - vaddr + offset)?;
    let read = |off: u64| r.string(usize::try_from(strtab + off).ok()?);
    Some(Linking {
        is_32bit: !wide,
        needed: needed.into_iter().filter_map(read).collect(),
        runpath: paths
            .into_iter()
            .filter_map(read)
            .flat_map(|p| p.split(':').map(str::to_owned).collect::<Vec<_>>())
            .filter(|p| !p.is_empty())
            .collect(),
    })
}

/// Libraries `linking` needs that are in none of the directories.
/// `exists` checks a full path; `origin` is the binary's own directory;
/// `extra` are directories from `/etc/ld.so.conf.d`.
pub fn missing(
    linking: &Linking,
    origin: &str,
    extra: &[String],
    exists: impl Fn(&str) -> bool,
) -> Vec<String> {
    let system: &[&str] = if linking.is_32bit {
        &["/usr/lib32", "/lib32"]
    } else {
        &["/usr/lib", "/lib"]
    };
    let dirs: Vec<String> = linking
        .runpath
        .iter()
        .map(|p| p.replace("$ORIGIN", origin).replace("${ORIGIN}", origin))
        .chain(system.iter().map(|s| (*s).to_owned()))
        .chain(extra.iter().cloned())
        .collect();
    linking
        .needed
        .iter()
        .filter(|lib| !dirs.iter().any(|d| exists(&format!("{d}/{lib}"))))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal 64-bit ELF: one loadable segment covering the file at
    // address 0x400000 and a dynamic section with NEEDED, RUNPATH and STRTAB.
    fn elf64() -> Vec<u8> {
        let mut d = vec![0u8; 0x200];
        d[..4].copy_from_slice(b"\x7fELF");
        d[4] = 2; // 64-bit
        d[5] = 1; // little endian
        d[0x20..0x28].copy_from_slice(&0x40u64.to_le_bytes()); // phoff
        d[0x36..0x38].copy_from_slice(&56u16.to_le_bytes()); // phentsize
        d[0x38..0x3a].copy_from_slice(&2u16.to_le_bytes()); // phnum
        let ph = |d: &mut Vec<u8>, at: usize, kind: u32, offset: u64, vaddr: u64, size: u64| {
            d[at..at + 4].copy_from_slice(&kind.to_le_bytes());
            d[at + 8..at + 16].copy_from_slice(&offset.to_le_bytes());
            d[at + 16..at + 24].copy_from_slice(&vaddr.to_le_bytes());
            d[at + 32..at + 40].copy_from_slice(&size.to_le_bytes());
        };
        ph(&mut d, 0x40, PT_LOAD, 0, 0x40_0000, 0x200);
        ph(&mut d, 0x78, PT_DYNAMIC, 0x100, 0x40_0100, 0x50);
        // String table at file offset 0x180.
        let strings = b"\0libfoo.so.1\0libc.so.6\0$ORIGIN/../lib\0";
        d[0x180..0x180 + strings.len()].copy_from_slice(strings);
        let dynamic: [(u64, u64); 5] = [
            (DT_NEEDED, 1),
            (DT_NEEDED, 13),
            (DT_RUNPATH, 23),
            (DT_STRTAB, 0x40_0180),
            (DT_NULL, 0),
        ];
        for (i, (tag, value)) in dynamic.iter().enumerate() {
            let at = 0x100 + i * 16;
            d[at..at + 8].copy_from_slice(&tag.to_le_bytes());
            d[at + 8..at + 16].copy_from_slice(&value.to_le_bytes());
        }
        d
    }

    #[test]
    fn reads_needed_libraries() {
        let l = linking(&elf64()).unwrap();
        assert_eq!(
            l,
            Linking {
                is_32bit: false,
                needed: vec!["libfoo.so.1".into(), "libc.so.6".into()],
                runpath: vec!["$ORIGIN/../lib".into()],
            }
        );
        assert_eq!(linking(b"#!/bin/sh\n"), None);
        assert_eq!(linking(&elf64()[..0x30]), None);
        let mut big_endian = elf64();
        big_endian[5] = 2;
        assert_eq!(linking(&big_endian), None);
    }

    #[test]
    fn finds_missing_libraries() {
        let l = linking(&elf64()).unwrap();
        let present = ["/usr/lib/libc.so.6", "/opt/app/bin/../lib/libfoo.so.1"];
        assert_eq!(
            missing(&l, "/opt/app/bin", &[], |p| present.contains(&p)),
            Vec::<String>::new()
        );
        assert_eq!(
            missing(&l, "/usr/bin", &[], |p| present.contains(&p)),
            ["libfoo.so.1"]
        );
        let extra = ["/opt/app/bin/../lib".to_owned()];
        assert_eq!(
            missing(&l, "/usr/bin", &extra, |p| present.contains(&p)),
            Vec::<String>::new()
        );
        let l32 = Linking {
            is_32bit: true,
            needed: vec!["libc.so.6".into()],
            runpath: Vec::new(),
        };
        assert_eq!(
            missing(&l32, "/usr/bin", &[], |p| present.contains(&p)),
            ["libc.so.6"]
        );
        assert!(is_elf(b"\x7fELF...."));
        assert_eq!(
            ld_conf_dirs("# comment\n/usr/lib/openmpi/\ninclude x\n"),
            ["/usr/lib/openmpi"]
        );
    }

    #[test]
    fn reads_real_binaries_when_present() {
        // Arch has these; skip elsewhere.
        for (path, lib) in [
            ("/usr/bin/bash", "libc.so.6"),
            ("/usr/lib32/libz.so.1", "libc.so.6"),
        ] {
            let Ok(data) = std::fs::read(path) else { continue };
            let l = linking(&data).unwrap();
            assert!(l.needed.iter().any(|n| n == lib), "{path}: {:?}", l.needed);
            assert_eq!(l.is_32bit, path.contains("lib32"));
        }
    }
}
