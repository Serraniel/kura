// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Repository profiles: third-party repositories kura can add and take away
//! again. Each profile is fixed here, signing key included; data files
//! never name keys or addresses.

use crate::action::Action;

#[derive(Debug)]
pub struct Profile {
    pub id: &'static str,
    /// Repository sections with their `Include` files.
    pub repos: &'static [(&'static str, &'static str)],
    /// Added before this section, or at the end.
    pub before: Option<&'static str>,
    /// Full fingerprint of the signing key.
    pub key: &'static str,
    pub keyserver: &'static str,
    /// Keyring and mirror list packages, installed from these addresses
    /// before the repository exists.
    pub bootstrap: &'static [&'static str],
    /// Their package names, removed again when the profile goes.
    pub bootstrap_packages: &'static [&'static str],
}

/// Ready-built AUR packages, run by volunteers.
pub static CHAOTIC_AUR: Profile = Profile {
    id: "chaotic-aur",
    repos: &[("chaotic-aur", "/etc/pacman.d/chaotic-mirrorlist")],
    before: None,
    key: "EF925EA60F33D0CB85C44AD13056513887B78AEB",
    keyserver: "keyserver.ubuntu.com",
    bootstrap: &[
        "https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-keyring.pkg.tar.zst",
        "https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-mirrorlist.pkg.tar.zst",
    ],
    bootstrap_packages: &["chaotic-keyring", "chaotic-mirrorlist"],
};

pub static ALL: &[&Profile] = &[&CHAOTIC_AUR];

pub fn find(id: &str) -> Option<&'static Profile> {
    ALL.iter().copied().find(|p| p.id == id)
}

/// Whether the profile's repositories are all in `pacman.conf`.
pub fn is_active(profile: &Profile, pacman_conf: &str) -> bool {
    profile
        .repos
        .iter()
        .all(|(name, _)| crate::pacman_conf::has_repo(pacman_conf, name))
}

/// What adding the profile does, in order.
pub fn add_actions(profile: &Profile) -> Vec<Action> {
    let mut out = vec![
        Action::TrustKey {
            key: profile.key.to_owned(),
            keyserver: profile.keyserver.to_owned(),
        },
        Action::InstallFrom {
            urls: profile.bootstrap.iter().map(|u| (*u).to_owned()).collect(),
        },
    ];
    out.extend(profile.repos.iter().map(|(name, include)| Action::AddRepo {
        name: (*name).to_owned(),
        include: (*include).to_owned(),
        before: profile.before.map(str::to_owned),
    }));
    out
}

/// What taking the profile away does. Its keyring and mirror list packages
/// are removed in the same plan; apps installed from it stay.
pub fn remove_actions(profile: &Profile) -> Vec<Action> {
    let mut out: Vec<Action> = profile
        .repos
        .iter()
        .map(|(name, _)| Action::RemoveRepo {
            name: (*name).to_owned(),
        })
        .collect();
    out.push(Action::ForgetKey {
        key: profile.key.to_owned(),
    });
    out
}

/// Whether `actions` is exactly what kura derives for one of its profiles.
/// The helper runs nothing else.
pub fn is_known(actions: &[Action]) -> bool {
    ALL.iter()
        .any(|p| add_actions(p) == actions || remove_actions(p) == actions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaotic_aur_comes_and_goes() {
        let p = find("chaotic-aur").unwrap();
        let add = add_actions(p);
        assert!(matches!(&add[0], Action::TrustKey { key, .. } if key.len() == 40));
        assert!(matches!(&add[2], Action::AddRepo { name, before: None, .. } if name == "chaotic-aur"));
        assert!(is_known(&add));
        let remove = remove_actions(p);
        assert_eq!(
            remove,
            [
                Action::RemoveRepo {
                    name: "chaotic-aur".into()
                },
                Action::ForgetKey { key: p.key.into() }
            ]
        );
        assert!(is_known(&remove));
        assert!(!is_known(&[Action::ForgetKey { key: p.key.into() }]));
        assert!(find("nope").is_none());

        assert!(!is_active(p, "[options]\n[core]\n"));
        assert!(is_active(p, "[core]\n[chaotic-aur]\nInclude = x\n"));
    }
}
