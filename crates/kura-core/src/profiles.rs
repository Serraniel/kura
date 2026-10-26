// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Repository profiles: third-party repositories kura can add and take away
//! again. Each profile is fixed here, signing key included; data files
//! never name keys or addresses.

use crate::action::Action;

/// How the keyring and mirror list get onto the system before the
/// repository is usable.
#[derive(Debug)]
pub enum Bootstrap {
    /// Package files at fixed addresses.
    Urls(&'static [&'static str]),
    /// A repository section with a fixed server, replaced afterwards.
    Server {
        repo: &'static str,
        server: &'static str,
    },
}

/// Repositories and the `Architecture` value for one CPU level.
#[derive(Debug)]
pub struct Level {
    pub level: u8,
    pub repos: &'static [(&'static str, &'static str)],
    pub architecture: Option<&'static str>,
    /// Mirror list packages only this level needs.
    pub packages: &'static [&'static str],
}

#[derive(Debug)]
pub struct Profile {
    pub id: &'static str,
    /// Repository sets by CPU level, best first; the first one the CPU
    /// supports is used.
    pub levels: &'static [Level],
    /// Added before this section, or at the end.
    pub before: Option<&'static str>,
    /// Full fingerprint of the signing key.
    pub key: &'static str,
    pub keyserver: &'static str,
    pub bootstrap: Bootstrap,
    /// Keyring and mirror list packages, removed again with the profile.
    pub bootstrap_packages: &'static [&'static str],
    /// Packages the bootstrap replaces and the switch back restores, such
    /// as CachyOS's pacman, which understands its mirror lists.
    pub bootstrap_replaces: &'static [&'static str],
}

/// Ready-built AUR packages, run by volunteers.
pub static CHAOTIC_AUR: Profile = Profile {
    id: "chaotic-aur",
    levels: &[Level {
        level: 1,
        repos: &[("chaotic-aur", "/etc/pacman.d/chaotic-mirrorlist")],
        architecture: None,
        packages: &[],
    }],
    before: None,
    key: "EF925EA60F33D0CB85C44AD13056513887B78AEB",
    keyserver: "keyserver.ubuntu.com",
    bootstrap: Bootstrap::Urls(&[
        "https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-keyring.pkg.tar.zst",
        "https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-mirrorlist.pkg.tar.zst",
    ]),
    bootstrap_packages: &["chaotic-keyring", "chaotic-mirrorlist"],
    bootstrap_replaces: &[],
};

/// CachyOS packages built for newer processors.
pub static CACHYOS: Profile = Profile {
    id: "cachyos",
    levels: &[
        Level {
            level: 4,
            repos: &[
                ("cachyos-v4", "/etc/pacman.d/cachyos-v4-mirrorlist"),
                ("cachyos-core-v4", "/etc/pacman.d/cachyos-v4-mirrorlist"),
                ("cachyos-extra-v4", "/etc/pacman.d/cachyos-v4-mirrorlist"),
                ("cachyos", "/etc/pacman.d/cachyos-mirrorlist"),
            ],
            architecture: Some("x86_64 x86_64_v3 x86_64_v4"),
            packages: &["cachyos-v4-mirrorlist"],
        },
        Level {
            level: 3,
            repos: &[
                ("cachyos-v3", "/etc/pacman.d/cachyos-v3-mirrorlist"),
                ("cachyos-core-v3", "/etc/pacman.d/cachyos-v3-mirrorlist"),
                ("cachyos-extra-v3", "/etc/pacman.d/cachyos-v3-mirrorlist"),
                ("cachyos", "/etc/pacman.d/cachyos-mirrorlist"),
            ],
            architecture: Some("x86_64 x86_64_v3"),
            packages: &["cachyos-v3-mirrorlist"],
        },
    ],
    before: Some("core"),
    key: "882DCFE48E2051D48E2562ABF3B607488DB35A47",
    keyserver: "keyserver.ubuntu.com",
    bootstrap: Bootstrap::Server {
        repo: "cachyos",
        server: "https://mirror.cachyos.org/repo/$arch/$repo",
    },
    bootstrap_packages: &[
        "cachyos-keyring",
        "cachyos-mirrorlist",
        "cachyos-v3-mirrorlist",
        "cachyos-v4-mirrorlist",
    ],
    // Its mirror lists use $arch_v3, which only CachyOS's pacman expands.
    bootstrap_replaces: &["pacman"],
};

pub static ALL: &[&Profile] = &[&CACHYOS, &CHAOTIC_AUR];

pub fn find(id: &str) -> Option<&'static Profile> {
    ALL.iter().copied().find(|p| p.id == id)
}

/// The repository set for a CPU of `cpu_level`, if any fits.
pub fn level(profile: &Profile, cpu_level: u8) -> Option<&Level> {
    profile.levels.iter().find(|l| l.level <= cpu_level)
}

/// The repository set that is in `pacman.conf`, if any.
pub fn active_level<'a>(profile: &'a Profile, pacman_conf: &str) -> Option<&'a Level> {
    profile.levels.iter().find(|l| {
        l.repos
            .iter()
            .all(|(name, _)| crate::pacman_conf::has_repo(pacman_conf, name))
    })
}

pub fn is_active(profile: &Profile, pacman_conf: &str) -> bool {
    active_level(profile, pacman_conf).is_some()
}

/// Keyring and mirror list packages installed for this level.
fn packages_for(profile: &Profile, level: &Level) -> Vec<String> {
    profile
        .bootstrap_packages
        .iter()
        .filter(|p| !profile.levels.iter().any(|l| l.packages.contains(p)) || level.packages.contains(p))
        .map(|p| (*p).to_owned())
        .collect()
}

/// The server a repository of the profile has before its mirror list is
/// installed, for previews. CachyOS keeps optimised builds under the
/// architecture name (`x86_64_v3`) and the rest under `x86_64`.
pub fn server_for(profile: &Profile, repo: &str) -> Option<String> {
    let Bootstrap::Server { server, .. } = &profile.bootstrap else {
        return None;
    };
    let arch = ["v3", "v4"]
        .iter()
        .find(|v| repo.ends_with(&format!("-{v}")))
        .map_or_else(|| "x86_64".to_owned(), |v| format!("x86_64_{v}"));
    Some(server.replace("$arch", &arch).replace("$repo", repo))
}

/// What adding the profile at `level` does, in order.
pub fn add_actions(profile: &Profile, level: &Level) -> Vec<Action> {
    let mut out = vec![Action::TrustKey {
        key: profile.key.to_owned(),
        keyserver: profile.keyserver.to_owned(),
    }];
    match &profile.bootstrap {
        Bootstrap::Urls(urls) => out.push(Action::InstallFrom {
            urls: urls.iter().map(|u| (*u).to_owned()).collect(),
        }),
        Bootstrap::Server { repo, server } => {
            out.push(Action::AddRepoServer {
                name: (*repo).to_owned(),
                server: (*server).to_owned(),
                before: profile.before.map(str::to_owned),
            });
            let mut packages = packages_for(profile, level);
            packages.extend(profile.bootstrap_replaces.iter().map(|p| (*p).to_owned()));
            out.push(Action::InstallFromRepo {
                repo: (*repo).to_owned(),
                packages,
            });
            out.push(Action::RemoveRepo {
                name: (*repo).to_owned(),
            });
        }
    }
    out.extend(level.repos.iter().map(|(name, include)| Action::AddRepo {
        name: (*name).to_owned(),
        include: (*include).to_owned(),
        before: profile.before.map(str::to_owned),
    }));
    if let Some(arch) = level.architecture {
        out.push(Action::SetArchitecture {
            value: arch.to_owned(),
        });
    }
    out
}

/// What taking the profile away does. Its keyring and mirror list packages
/// are removed in the same plan; apps installed from it stay.
pub fn remove_actions(profile: &Profile, level: &Level) -> Vec<Action> {
    let mut out: Vec<Action> = level
        .repos
        .iter()
        .map(|(name, _)| Action::RemoveRepo {
            name: (*name).to_owned(),
        })
        .collect();
    if level.architecture.is_some() {
        out.push(Action::SetArchitecture { value: "auto".into() });
    }
    out.push(Action::ForgetKey {
        key: profile.key.to_owned(),
    });
    out
}

/// The profile and level whose add or remove actions are exactly `actions`,
/// and whether they add it. The helper runs nothing else.
pub fn known(actions: &[Action]) -> Option<(&'static Profile, &'static Level, bool)> {
    ALL.iter().find_map(|p| {
        p.levels.iter().find_map(|l| {
            if add_actions(p, l) == actions {
                Some((*p, l, true))
            } else if remove_actions(p, l) == actions {
                Some((*p, l, false))
            } else {
                None
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaotic_aur_comes_and_goes() {
        let p = find("chaotic-aur").unwrap();
        let l = level(p, 1).unwrap();
        let add = add_actions(p, l);
        assert!(matches!(&add[0], Action::TrustKey { key, .. } if key.len() == 40));
        assert!(matches!(&add[2], Action::AddRepo { name, before: None, .. } if name == "chaotic-aur"));
        assert_eq!(
            known(&add).map(|(p, _, add)| (p.id, add)),
            Some(("chaotic-aur", true))
        );
        let remove = remove_actions(p, l);
        assert_eq!(
            remove,
            [
                Action::RemoveRepo {
                    name: "chaotic-aur".into()
                },
                Action::ForgetKey { key: p.key.into() }
            ]
        );
        assert_eq!(known(&remove).map(|(_, _, add)| add), Some(false));
        assert!(known(&[Action::ForgetKey { key: p.key.into() }]).is_none());
        assert!(find("nope").is_none());

        assert!(!is_active(p, "[options]\n[core]\n"));
        assert!(is_active(p, "[core]\n[chaotic-aur]\nInclude = x\n"));
    }

    #[test]
    fn cachyos_follows_the_cpu_level() {
        let p = find("cachyos").unwrap();
        assert!(level(p, 2).is_none());
        let v3 = level(p, 3).unwrap();
        assert_eq!(v3.level, 3);
        assert_eq!(level(p, 4).unwrap().level, 4);

        let add = add_actions(p, v3);
        assert_eq!(
            add[2],
            Action::InstallFromRepo {
                repo: "cachyos".into(),
                packages: vec![
                    "cachyos-keyring".into(),
                    "cachyos-mirrorlist".into(),
                    "cachyos-v3-mirrorlist".into(),
                    "pacman".into()
                ],
            }
        );
        assert_eq!(
            add.last(),
            Some(&Action::SetArchitecture {
                value: "x86_64 x86_64_v3".into()
            })
        );
        let conf = "[options]\n[cachyos-v3]\n[cachyos-core-v3]\n[cachyos-extra-v3]\n[cachyos]\n[core]\n";
        assert_eq!(active_level(p, conf).map(|l| l.level), Some(3));
        assert_eq!(
            server_for(p, "cachyos-core-v3").as_deref(),
            Some("https://mirror.cachyos.org/repo/x86_64_v3/cachyos-core-v3")
        );
        assert_eq!(
            server_for(p, "cachyos").as_deref(),
            Some("https://mirror.cachyos.org/repo/x86_64/cachyos")
        );
        assert_eq!(server_for(&CHAOTIC_AUR, "chaotic-aur"), None);
        let remove = remove_actions(p, v3);
        assert!(remove.contains(&Action::SetArchitecture { value: "auto".into() }));
        assert_eq!(
            known(&remove).map(|(p, l, add)| (p.id, l.level, add)),
            Some(("cachyos", 3, false))
        );
    }
}
