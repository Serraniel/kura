// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Domain logic of kura. Nothing in here touches the real system, so all of it
//! can be tested without root or a pacman database.

pub mod generation;
pub mod history;
pub mod locale;
pub mod mirrors;
pub mod news;
pub mod package;
pub mod plan;
pub mod status;
pub mod version;
