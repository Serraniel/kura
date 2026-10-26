// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Settings that apply to the whole system, stored in
//! `/etc/kura/settings.json`. Missing or broken values fall back to the
//! defaults, so an old or hand-edited file never stops kura.

use serde::{Deserialize, Serialize};

pub const PATH: &str = "/etc/kura/settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Install and update packages from the AUR.
    pub aur: bool,
    /// Days an AUR update waits before it joins the normal update (0–14).
    pub aur_cooling_days: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            aur: true,
            aur_cooling_days: 3,
        }
    }
}

impl Settings {
    pub const MAX_COOLING_DAYS: u32 = 14;

    /// Reads settings; anything unreadable gives the defaults.
    pub fn parse(json: &str) -> Self {
        serde_json::from_str::<Self>(json).unwrap_or_default().clamped()
    }

    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            aur_cooling_days: self.aur_cooling_days.min(Self::MAX_COOLING_DAYS),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_limits() {
        assert_eq!(Settings::parse(""), Settings::default());
        assert_eq!(Settings::parse("{}"), Settings::default());
        assert_eq!(
            Settings::parse(r#"{"aur": false}"#),
            Settings {
                aur: false,
                aur_cooling_days: 3
            }
        );
        assert_eq!(
            Settings::parse(r#"{"aur_cooling_days": 40}"#).aur_cooling_days,
            14
        );
        assert_eq!(Settings::parse(r#"{"aur": "yes"}"#), Settings::default());
        let json = serde_json::to_string(&Settings::default()).unwrap();
        assert_eq!(json, r#"{"aur":true,"aur_cooling_days":3}"#);
    }
}
