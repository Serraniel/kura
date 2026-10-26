// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Bundles: sets of apps plus the configuration they need, described in
//! TOML files that ship with kura. A bundle can only use the declarative
//! actions below; it never runs commands.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::action::Action;

/// Text in several languages; English is required.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct Texts(pub BTreeMap<String, String>);

impl Texts {
    /// The text for the first of `languages` it has, otherwise English.
    pub fn get(&self, languages: &[&str]) -> &str {
        languages
            .iter()
            .find_map(|l| self.0.get(*l))
            .or_else(|| self.0.get("en"))
            .map_or("", String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Optional {
    pub name: String,
    #[serde(default)]
    pub reason: Texts,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Packages {
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub recommended: Vec<String>,
    #[serde(default)]
    pub optional: Vec<Optional>,
}

/// What a bundle may change besides packages.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum BundleAction {
    /// Turn on an official repository that is off by default. Only
    /// `multilib` (32-bit libraries for games) is allowed.
    EnableRepo(String),
    /// Add the person installing the bundle to a group (user scope).
    UserGroup(String),
    /// Enable and start a system service.
    EnableService(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub id: String,
    pub version: u32,
    pub name: Texts,
    pub description: Texts,
    #[serde(default)]
    pub packages: Packages,
    #[serde(default)]
    pub actions: Vec<BundleAction>,
}

/// Repositories a bundle may turn on, with their `Include` line.
const REPOS: &[(&str, &str)] = &[("multilib", "/etc/pacman.d/mirrorlist")];

/// Reads a bundle file and checks what it may do.
pub fn parse(text: &str) -> Result<Bundle, String> {
    let bundle: Bundle = toml::from_str(text).map_err(|e| e.to_string())?;
    if !bundle.name.0.contains_key("en") || !bundle.description.0.contains_key("en") {
        return Err(format!(
            "{}: English name and description are required",
            bundle.id
        ));
    }
    let simple = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"-_.@".contains(&b))
    };
    for action in &bundle.actions {
        let ok = match action {
            BundleAction::EnableRepo(r) => REPOS.iter().any(|(name, _)| name == r),
            BundleAction::UserGroup(g) => simple(g),
            BundleAction::EnableService(u) => {
                simple(u)
                    && std::path::Path::new(u)
                        .extension()
                        .is_some_and(|e| e == "service" || e == "timer")
            }
        };
        if !ok {
            return Err(format!("{}: action not allowed: {action:?}", bundle.id));
        }
    }
    Ok(bundle)
}

/// Actions that must run before the packages can be installed (repositories
/// that are not on yet), and the ones that run after, for `user`.
pub fn actions(bundle: &Bundle, active_repos: &[String], user: &str) -> (Vec<Action>, Vec<Action>) {
    let mut before = Vec::new();
    let mut after = Vec::new();
    for action in &bundle.actions {
        match action {
            BundleAction::EnableRepo(r) if !active_repos.contains(r) => {
                if let Some((name, include)) = REPOS.iter().find(|(name, _)| name == r) {
                    before.push(Action::AddRepo {
                        name: (*name).to_owned(),
                        include: (*include).to_owned(),
                        before: None,
                    });
                }
            }
            BundleAction::EnableRepo(_) => {}
            BundleAction::UserGroup(group) => after.push(Action::AddUserToGroup {
                user: user.to_owned(),
                group: group.clone(),
            }),
            BundleAction::EnableService(unit) => after.push(Action::Enable { unit: unit.clone() }),
        }
    }
    (before, after)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAMING: &str = r#"
id = "gaming"
version = 1

[name]
en = "Gaming with Steam"
de = "Spielen mit Steam"

[description]
en = "Steam and tools."

[packages]
required = ["steam"]
recommended = ["gamemode"]

[[packages.optional]]
name = "mangohud"
reason = { en = "Shows frame rates." }

[[actions]]
enable-repo = "multilib"

[[actions]]
user-group = "gamemode"
"#;

    #[test]
    fn reads_bundles() {
        let b = parse(GAMING).unwrap();
        assert_eq!(b.id, "gaming");
        assert_eq!(b.name.get(&["de"]), "Spielen mit Steam");
        assert_eq!(b.name.get(&["ja"]), "Gaming with Steam");
        assert_eq!(b.packages.required, ["steam"]);
        assert_eq!(b.packages.optional[0].reason.get(&["en"]), "Shows frame rates.");

        let (before, after) = actions(&b, &["core".into()], "anna");
        assert_eq!(
            before,
            [Action::AddRepo {
                name: "multilib".into(),
                include: "/etc/pacman.d/mirrorlist".into(),
                before: None
            }]
        );
        assert_eq!(
            after,
            [Action::AddUserToGroup {
                user: "anna".into(),
                group: "gamemode".into()
            }]
        );
        let (before, _) = actions(&b, &["multilib".into()], "anna");
        assert_eq!(before, Vec::<Action>::new());
    }

    #[test]
    fn refuses_what_bundles_may_not_do() {
        let other_repo = GAMING.replace("enable-repo = \"multilib\"", "enable-repo = \"evil\"");
        assert!(parse(&other_repo).is_err());
        let shell = GAMING.replace("user-group = \"gamemode\"", "run = \"rm -rf /\"");
        assert!(parse(&shell).is_err());
        let odd_group = GAMING.replace("user-group = \"gamemode\"", "user-group = \"wheel; x\"");
        assert!(parse(&odd_group).is_err());
        let no_english = GAMING.replace("en = \"Gaming with Steam\"", "");
        assert!(parse(&no_english).is_err());
        let service = GAMING.replace("user-group = \"gamemode\"", "enable-service = \"bluetooth\"");
        assert!(parse(&service).is_err());
    }

    #[test]
    fn shipped_bundles_are_valid() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/bundles");
        let mut count = 0;
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let text = std::fs::read_to_string(entry.path()).unwrap();
            let bundle = parse(&text).unwrap_or_else(|e| panic!("{}: {e}", entry.path().display()));
            assert_eq!(
                entry.path().file_stem().unwrap().to_str(),
                Some(bundle.id.as_str())
            );
            count += 1;
        }
        assert!(count >= 2);
    }
}
