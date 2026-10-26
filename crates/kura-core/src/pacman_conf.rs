// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Adding and removing repository sections in `pacman.conf`. Everything
//! else in the file, comments included, stays exactly as it was.

/// Section names in order, `options` included.
pub fn sections(conf: &str) -> Vec<String> {
    conf.lines()
        .filter_map(|l| {
            let l = l.trim();
            l.strip_prefix('[')?.strip_suffix(']').map(str::to_owned)
        })
        .collect()
}

pub fn has_repo(conf: &str, name: &str) -> bool {
    sections(conf).iter().any(|s| s == name)
}

/// Adds `[name]` with `Include = include`, before the section `before` or
/// at the end. Nothing changes when the repository is already there.
pub fn add_repo(conf: &str, name: &str, include: &str, before: Option<&str>) -> String {
    add_repo_line(conf, name, &format!("Include = {include}"), before)
}

/// Like [`add_repo`], with any setting line such as `Server = …`.
pub fn add_repo_line(conf: &str, name: &str, line: &str, before: Option<&str>) -> String {
    if has_repo(conf, name) {
        return conf.to_owned();
    }
    let block = format!("[{name}]\n{line}\n");
    let lines: Vec<&str> = conf.lines().collect();
    let at = before.and_then(|b| lines.iter().position(|l| l.trim() == format!("[{b}]")));
    let Some(i) = at else {
        return format!("{}\n\n{block}", conf.trim_end_matches('\n'));
    };
    let mut out = String::new();
    for l in &lines[..i] {
        out.push_str(l);
        out.push('\n');
    }
    out.push_str(&block);
    out.push('\n');
    for l in &lines[i..] {
        out.push_str(l);
        out.push('\n');
    }
    out
}

/// Removes the section `[name]` with its settings and the blank lines after
/// them. The first comment or section ends the block, so comments that
/// belong to the next section stay.
pub fn remove_repo(conf: &str, name: &str) -> String {
    let header = format!("[{name}]");
    let mut out = String::new();
    let mut skipping = false;
    for line in conf.lines() {
        let t = line.trim();
        if t == header {
            skipping = true;
            continue;
        }
        if skipping {
            if !(t.starts_with('#') || t.starts_with('[')) {
                continue;
            }
            skipping = false;
        }
        out.push_str(line);
        out.push('\n');
    }
    // A removed block leaves at most one blank line behind, none at the end.
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

/// The value of an option in `[options]`, such as `Architecture`.
pub fn option(conf: &str, key: &str) -> Option<String> {
    let mut in_options = false;
    for line in conf.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_options = t == "[options]";
            continue;
        }
        if in_options
            && let Some((k, v)) = t.split_once('=')
            && k.trim() == key
        {
            return Some(v.trim().to_owned());
        }
    }
    None
}

/// Sets an option in `[options]`, replacing the active line or adding one
/// right after the section header.
pub fn set_option(conf: &str, key: &str, value: &str) -> String {
    let setting = format!("{key} = {value}\n");
    let mut out = String::new();
    let mut in_options = false;
    let mut done = false;
    for line in conf.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_options = t == "[options]";
            out.push_str(line);
            out.push('\n');
            if in_options && option(conf, key).is_none() && !done {
                out.push_str(&setting);
                done = true;
            }
            continue;
        }
        if in_options && !done && t.split_once('=').is_some_and(|(k, _)| k.trim() == key) {
            out.push_str(&setting);
            done = true;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARCH: &str = "# pacman.conf\n[options]\nHoldPkg = pacman glibc\nArchitecture = auto\n\n# The testing repositories are disabled.\n#[core-testing]\n#Include = /etc/pacman.d/mirrorlist\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n\n[extra]\nInclude = /etc/pacman.d/mirrorlist\n\n[multilib]\nInclude = /etc/pacman.d/mirrorlist\n";

    #[test]
    fn lists_sections() {
        assert_eq!(sections(ARCH), ["options", "core", "extra", "multilib"]);
        assert!(has_repo(ARCH, "extra"));
        assert!(!has_repo(ARCH, "core-testing"));
    }

    #[test]
    fn adds_at_the_end_or_before_a_section() {
        let end = add_repo(ARCH, "chaotic-aur", "/etc/pacman.d/chaotic-mirrorlist", None);
        assert!(end.starts_with(ARCH.trim_end()));
        assert!(end.ends_with("[multilib]\nInclude = /etc/pacman.d/mirrorlist\n\n[chaotic-aur]\nInclude = /etc/pacman.d/chaotic-mirrorlist\n"));
        assert_eq!(add_repo(&end, "chaotic-aur", "x", None), end);

        let first = add_repo(ARCH, "cachyos", "/etc/pacman.d/cachyos-mirrorlist", Some("core"));
        assert_eq!(
            sections(&first),
            ["options", "cachyos", "core", "extra", "multilib"]
        );
        assert!(first.contains("#[core-testing]\n#Include = /etc/pacman.d/mirrorlist\n\n[cachyos]\n"));
    }

    #[test]
    fn removing_restores_the_original() {
        let end = add_repo(ARCH, "chaotic-aur", "/etc/pacman.d/chaotic-mirrorlist", None);
        assert_eq!(remove_repo(&end, "chaotic-aur"), ARCH);
        let first = add_repo(ARCH, "cachyos", "/etc/pacman.d/cachyos-mirrorlist", Some("core"));
        assert_eq!(remove_repo(&first, "cachyos"), ARCH);
        // Removing what is not there changes nothing.
        assert_eq!(remove_repo(ARCH, "chaotic-aur"), ARCH);
    }

    #[test]
    fn options() {
        assert_eq!(option(ARCH, "Architecture").as_deref(), Some("auto"));
        assert_eq!(option(ARCH, "Missing"), None);
        let v3 = set_option(ARCH, "Architecture", "x86_64 x86_64_v3");
        assert_eq!(option(&v3, "Architecture").as_deref(), Some("x86_64 x86_64_v3"));
        assert_eq!(set_option(&v3, "Architecture", "auto"), ARCH);
        let added = set_option("[options]\nHoldPkg = pacman\n[core]\n", "Architecture", "auto");
        assert_eq!(
            added,
            "[options]\nArchitecture = auto\nHoldPkg = pacman\n[core]\n"
        );
        // A commented-out line stays as it is.
        let commented = set_option("[options]\n#Architecture = x\n", "Architecture", "auto");
        assert_eq!(commented, "[options]\nArchitecture = auto\n#Architecture = x\n");
    }

    #[test]
    fn keeps_comments_of_the_next_section() {
        let conf = "[a]\nInclude = x\n# about b\n#[b-testing]\n[b]\nServer = y\n";
        assert_eq!(
            remove_repo(conf, "a"),
            "# about b\n#[b-testing]\n[b]\nServer = y\n"
        );
    }
}
