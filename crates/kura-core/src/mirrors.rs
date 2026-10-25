// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Choosing Arch Linux mirrors from the official mirror status.
//!
//! Only up-to-date HTTPS mirrors are considered. Mirrors in the user's country
//! come first when there are enough of them; the final order comes from a
//! speed test the caller runs on the candidates.

use std::fmt::Write as _;

use serde::Deserialize;

/// One entry of <https://archlinux.org/mirrors/status/json/>.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Mirror {
    pub url: String,
    pub protocol: String,
    #[serde(default)]
    pub completion_pct: Option<f64>,
    /// Seconds behind the main server.
    #[serde(default)]
    pub delay: Option<i64>,
    /// Arch's own ranking; lower is better.
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub country_code: String,
}

#[derive(Debug, Deserialize)]
struct StatusFile {
    urls: Vec<Mirror>,
}

/// Reads the mirror status JSON.
pub fn parse_status(json: &str) -> Result<Vec<Mirror>, serde_json::Error> {
    serde_json::from_str::<StatusFile>(json).map(|s| s.urls)
}

/// At most this far behind the main server.
const MAX_DELAY: i64 = 6 * 3600;
/// Prefer the own country when it has at least this many good mirrors.
const ENOUGH_LOCAL: usize = 5;

/// Good mirrors, best first, at most `limit`. `country` is an ISO 3166 code.
pub fn candidates<'a>(mirrors: &'a [Mirror], country: Option<&str>, limit: usize) -> Vec<&'a Mirror> {
    let mut good: Vec<&Mirror> = mirrors
        .iter()
        .filter(|m| m.active && m.protocol == "https")
        .filter(|m| m.completion_pct.is_some_and(|c| c >= 1.0))
        .filter(|m| m.delay.is_some_and(|d| d <= MAX_DELAY))
        .filter(|m| m.score.is_some())
        .collect();
    good.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
    if let Some(cc) = country {
        let local: Vec<&Mirror> = good
            .iter()
            .copied()
            .filter(|m| m.country_code.eq_ignore_ascii_case(cc))
            .collect();
        if local.len() >= ENOUGH_LOCAL {
            good = local;
        }
    }
    good.truncate(limit);
    good
}

/// The country of a timezone, from the contents of `zone.tab`.
pub fn country_of_zone(zone_tab: &str, zone: &str) -> Option<String> {
    zone_tab.lines().filter(|l| !l.starts_with('#')).find_map(|l| {
        let mut cols = l.split('\t');
        let cc = cols.next()?;
        let _coords = cols.next()?;
        (cols.next()? == zone).then(|| cc.to_owned())
    })
}

/// The mirrorlist file pacman reads.
pub fn mirrorlist(urls: &[&str], generated: &str) -> String {
    let mut out = format!(
        "# Arch Linux mirrors, chosen by kura on {generated}.\n# The previous list was kept next to this file.\n\n"
    );
    for url in urls {
        let base = url.trim_end_matches('/');
        let _ = writeln!(out, "Server = {base}/$repo/os/$arch");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUS: &str = r#"{"cutoff": 3600, "urls": [
      {"url": "https://de1.example/archlinux/", "protocol": "https", "completion_pct": 1.0, "delay": 600, "score": 1.5, "active": true, "country_code": "DE"},
      {"url": "https://de2.example/archlinux/", "protocol": "https", "completion_pct": 1.0, "delay": 900, "score": 0.9, "active": true, "country_code": "DE"},
      {"url": "http://de3.example/archlinux/", "protocol": "http", "completion_pct": 1.0, "delay": 60, "score": 0.1, "active": true, "country_code": "DE"},
      {"url": "https://old.example/archlinux/", "protocol": "https", "completion_pct": 0.8, "delay": 600, "score": 0.2, "active": true, "country_code": "DE"},
      {"url": "https://late.example/archlinux/", "protocol": "https", "completion_pct": 1.0, "delay": 90000, "score": 0.3, "active": true, "country_code": "FR"},
      {"url": "https://off.example/archlinux/", "protocol": "https", "completion_pct": 1.0, "delay": 60, "score": 0.3, "active": false, "country_code": "FR"},
      {"url": "https://fr.example/archlinux/", "protocol": "https", "completion_pct": 1.0, "delay": 60, "score": 0.5, "active": true, "country_code": "FR"},
      {"url": "https://unscored.example/", "protocol": "https", "completion_pct": null, "delay": null, "score": null, "active": true, "country_code": "FR"}
    ]}"#;

    #[test]
    fn filters_and_sorts() {
        let mirrors = parse_status(STATUS).unwrap();
        assert_eq!(mirrors.len(), 8);
        let urls: Vec<&str> = candidates(&mirrors, None, 10)
            .iter()
            .map(|m| m.url.as_str())
            .collect();
        assert_eq!(
            urls,
            [
                "https://fr.example/archlinux/",
                "https://de2.example/archlinux/",
                "https://de1.example/archlinux/"
            ]
        );
        assert_eq!(candidates(&mirrors, None, 1).len(), 1);
    }

    #[test]
    fn own_country_only_with_enough_mirrors() {
        let mut mirrors = parse_status(STATUS).unwrap();
        // Two German mirrors are not enough: the best of all countries stay.
        assert_eq!(candidates(&mirrors, Some("de"), 10).len(), 3);
        for i in 0..4 {
            let mut extra = mirrors[0].clone();
            extra.url = format!("https://de-extra{i}.example/");
            mirrors.push(extra);
        }
        let local = candidates(&mirrors, Some("DE"), 10);
        assert_eq!(local.len(), 6);
        assert!(local.iter().all(|m| m.country_code == "DE"));
    }

    #[test]
    fn zone_to_country() {
        let tab =
            "# comment\nDE\t+5230+01322\tEurope/Berlin\nJP\t+353916+1394441\tAsia/Tokyo\tsome comment\n";
        assert_eq!(country_of_zone(tab, "Europe/Berlin").as_deref(), Some("DE"));
        assert_eq!(country_of_zone(tab, "Asia/Tokyo").as_deref(), Some("JP"));
        assert_eq!(country_of_zone(tab, "Mars/Base"), None);
    }

    #[test]
    fn writes_pacman_format() {
        let text = mirrorlist(
            &["https://a.example/archlinux/", "https://b.example/archlinux"],
            "2026-10-25",
        );
        assert!(text.starts_with("# Arch Linux mirrors, chosen by kura on 2026-10-25."));
        assert!(text.contains("Server = https://a.example/archlinux/$repo/os/$arch\n"));
        assert!(text.contains("Server = https://b.example/archlinux/$repo/os/$arch\n"));
    }

    #[test]
    fn broken_json_is_an_error() {
        assert!(parse_status("{").is_err());
    }
}
