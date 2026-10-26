// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! pacman-style arguments (`kura -Syu`, `kura -S firefox`) mapped to
//! kura's own commands, so every change still gets a preview. Things kura
//! never does, such as a partial upgrade, are refused with a reason.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Update,
    Install(Vec<String>),
    Remove(Vec<String>),
    Search(String),
    Info(String),
    /// Installed packages, or only the named ones.
    List(Vec<String>),
    /// Not done by kura; the Fluent id of the reason.
    Refused(&'static str),
}

/// Reads pacman-style arguments, or `None` when they are not pacman style.
pub fn parse(args: &[String]) -> Option<Op> {
    let first = args.first()?;
    let op = first
        .strip_prefix('-')
        .filter(|f| !f.starts_with('-'))?
        .chars()
        .next()?;
    if !matches!(op, 'S' | 'R' | 'Q') {
        return None;
    }
    let flags: String = args
        .iter()
        .filter(|a| a.starts_with('-') && !a.starts_with("--"))
        .flat_map(|a| a.chars().skip(1))
        .collect();
    let targets: Vec<String> = args.iter().filter(|a| !a.starts_with('-')).cloned().collect();
    let has = |c: char| flags.contains(c);
    Some(match op {
        'S' if has('c') => Op::Refused("pacman-clean"),
        'S' if has('s') => Op::Search(targets.join(" ")),
        'S' if has('i') => targets
            .first()
            .map_or(Op::Refused("pacman-usage"), |t| Op::Info(t.clone())),
        'S' if !targets.is_empty() => Op::Install(targets),
        'S' if has('u') => Op::Update,
        // `-Sy` alone refreshes without upgrading: a partial upgrade later.
        'S' if has('y') => Op::Refused("pacman-partial"),
        'R' if !targets.is_empty() => Op::Remove(targets),
        'Q' if has('i') => targets
            .first()
            .map_or(Op::Refused("pacman-usage"), |t| Op::Info(t.clone())),
        'Q' if has('s') => Op::Search(targets.join(" ")),
        'Q' => Op::List(targets),
        _ => Op::Refused("pacman-usage"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Option<Op> {
        parse(&args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())
    }

    #[test]
    fn maps_common_commands() {
        assert_eq!(p(&["-Syu"]), Some(Op::Update));
        assert_eq!(p(&["-Su"]), Some(Op::Update));
        assert_eq!(p(&["-S", "-y", "-u"]), Some(Op::Update));
        assert_eq!(
            p(&["-S", "firefox", "vlc"]),
            Some(Op::Install(vec!["firefox".into(), "vlc".into()]))
        );
        assert_eq!(p(&["-Syu", "firefox"]), Some(Op::Install(vec!["firefox".into()])));
        assert_eq!(
            p(&["-S", "--needed", "git"]),
            Some(Op::Install(vec!["git".into()]))
        );
        assert_eq!(p(&["-Rns", "vlc"]), Some(Op::Remove(vec!["vlc".into()])));
        assert_eq!(
            p(&["-Ss", "photo", "editor"]),
            Some(Op::Search("photo editor".into()))
        );
        assert_eq!(p(&["-Si", "gimp"]), Some(Op::Info("gimp".into())));
        assert_eq!(p(&["-Qi", "gimp"]), Some(Op::Info("gimp".into())));
        assert_eq!(p(&["-Q"]), Some(Op::List(vec![])));
        assert_eq!(p(&["-Qs", "gtk"]), Some(Op::Search("gtk".into())));
    }

    #[test]
    fn refuses_what_kura_does_not_do() {
        assert_eq!(p(&["-Sy"]), Some(Op::Refused("pacman-partial")));
        assert_eq!(p(&["-Scc"]), Some(Op::Refused("pacman-clean")));
        assert_eq!(p(&["-R"]), Some(Op::Refused("pacman-usage")));
        assert_eq!(p(&["-S"]), Some(Op::Refused("pacman-usage")));
        assert_eq!(p(&["update"]), None);
        assert_eq!(p(&["--help"]), None);
        assert_eq!(p(&["-h"]), None);
        assert_eq!(p(&[]), None);
    }
}
