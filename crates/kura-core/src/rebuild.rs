// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! AUR packages whose rebuild the user turned down, stored in
//! `/var/lib/kura/rebuild-skipped.json` with the version that was installed
//! then. Another installed version (an update or a successful rebuild) ends
//! the choice, so a new AUR version is offered as usual.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const PATH: &str = "/var/lib/kura/rebuild-skipped.json";

/// Package name to the installed version at the time of skipping.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Skipped(pub BTreeMap<String, String>);

impl Skipped {
    /// Reads the file's content; anything unreadable means nothing is skipped.
    pub fn parse(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_default()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default() + "\n"
    }

    /// Whether `name` at `installed` should not be offered for a rebuild.
    pub fn skips(&self, name: &str, installed: &str) -> bool {
        self.0.get(name).is_some_and(|v| v == installed)
    }

    /// Stops offering `name` at `version`, or offers it again with `None`.
    /// Refuses names that are not package names.
    pub fn set(&mut self, name: &str, version: Option<&str>) -> Result<(), String> {
        if !valid_name(name) {
            return Err(format!("not a package name: {name}"));
        }
        match version {
            Some(v) => self.0.insert(name.to_owned(), v.to_owned()),
            None => self.0.remove(name),
        };
        Ok(())
    }

    /// The skipped packages that are still installed at the skipped version.
    pub fn current(&self, installed: impl Fn(&str) -> Option<String>) -> Vec<String> {
        self.0
            .iter()
            .filter(|(name, version)| installed(name).as_ref() == Some(*version))
            .map(|(name, _)| name.clone())
            .collect()
    }
}

/// Package names as pacman allows them.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(['-', '.'])
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"@._+-".contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_only_the_version_it_was_set_for() {
        let mut s = Skipped::default();
        s.set("citra", Some("2:r10112.608383e-1")).unwrap();
        assert!(s.skips("citra", "2:r10112.608383e-1"));
        assert!(!s.skips("citra", "2:r10200.1234567-1"));
        assert!(!s.skips("dee", "1.0-1"));

        let back = Skipped::parse(&s.to_json());
        assert_eq!(back, s);

        s.set("citra", None).unwrap();
        assert!(!s.skips("citra", "2:r10112.608383e-1"));
    }

    #[test]
    fn refuses_odd_names_and_reads_broken_files_as_empty() {
        let mut s = Skipped::default();
        assert!(s.set("../etc/passwd", Some("1")).is_err());
        assert!(s.set("-rf", Some("1")).is_err());
        assert!(s.set("", Some("1")).is_err());
        assert!(s.set("Citra", Some("1")).is_err());
        assert!(s.set("lib32-gcc13+x", Some("1")).is_ok());
        assert_eq!(Skipped::parse("not json"), Skipped::default());
        assert_eq!(Skipped::parse(""), Skipped::default());
    }

    #[test]
    fn current_leaves_out_changed_and_removed_packages() {
        let s = Skipped::parse(r#"{"citra": "1-1", "dee": "2-1", "gone": "3-1"}"#);
        let installed = |n: &str| match n {
            "citra" => Some("1-1".to_owned()),
            "dee" => Some("2-2".to_owned()),
            _ => None,
        };
        assert_eq!(s.current(installed), ["citra"]);
    }
}
