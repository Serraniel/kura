// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Domain logic of kura. Nothing in here touches the real system, so all of it
//! can be tested without root or a pacman database.

pub mod action;
pub mod aur;
pub mod bundles;
pub mod drivers;
pub mod elf;
pub mod generation;
pub mod hardware;
pub mod health;
pub mod history;
pub mod kernels;
pub mod langpacks;
pub mod locale;
pub mod mirrors;
pub mod news;
pub mod package;
pub mod pacman_conf;
pub mod plan;
pub mod profiles;
pub mod report;
pub mod security;
pub mod settings;
pub mod snapshots;
pub mod solver;
pub mod status;
pub mod version;

/// kura's version: the release tag when built by the release pipeline or a
/// package (`KURA_VERSION`), otherwise the workspace version.
pub const VERSION: &str = match option_env!("KURA_VERSION") {
    Some(v) => v,
    None => env!("CARGO_PKG_VERSION"),
};
