// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Language packs that match the system language.
//!
//! Package families name their language packs differently (`-de`, `-deu`,
//! `-de-de`, `_de_DE`). The patterns below are the built-in defaults; the
//! `kura-bundles` repository will extend them.

use crate::locale::Locale;

/// How one package family names its language packs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Family {
    /// The package whose installation calls for a language pack.
    pub base: &'static str,
    /// Patterns for [`Locale::fill`], most specific first.
    pub patterns: &'static [&'static str],
    /// The base package already contains English.
    pub has_english: bool,
}

pub static FAMILIES: &[Family] = &[
    Family {
        base: "libreoffice-fresh",
        patterns: &["libreoffice-fresh-{bcp47_lower}", "libreoffice-fresh-{iso639_1}"],
        has_english: true,
    },
    Family {
        base: "libreoffice-still",
        patterns: &["libreoffice-still-{bcp47_lower}", "libreoffice-still-{iso639_1}"],
        has_english: true,
    },
    Family {
        base: "firefox",
        patterns: &["firefox-i18n-{bcp47_lower}", "firefox-i18n-{iso639_1}"],
        has_english: true,
    },
    Family {
        base: "thunderbird",
        patterns: &["thunderbird-i18n-{bcp47_lower}", "thunderbird-i18n-{iso639_1}"],
        has_english: true,
    },
    Family {
        base: "hunspell",
        patterns: &["hunspell-{iso639_1}"],
        has_english: false,
    },
    Family {
        base: "aspell",
        patterns: &["aspell-{iso639_1}"],
        has_english: false,
    },
    Family {
        base: "hyphen",
        patterns: &["hyphen-{iso639_1}"],
        has_english: false,
    },
    Family {
        base: "mythes",
        patterns: &["mythes-{iso639_1}"],
        has_english: false,
    },
    Family {
        base: "tesseract",
        patterns: &["tesseract-data-{iso639_2t}"],
        has_english: false,
    },
    Family {
        base: "gimp",
        patterns: &["gimp-help-{iso639_1}"],
        has_english: false,
    },
];

/// Candidate package names for every family, most preferred language first.
fn names_for<'a>(family: &'a Family, locales: &'a [Locale]) -> impl Iterator<Item = String> + 'a {
    locales
        .iter()
        .filter(|l| !(family.has_english && l.language == "en"))
        .flat_map(|l| l.candidates(family.patterns))
}

/// Language packs to add when `base` gets installed: the first existing
/// candidate per family. `exists` checks the repositories.
pub fn for_package(base: &str, locales: &[Locale], exists: impl Fn(&str) -> bool) -> Option<String> {
    let family = FAMILIES.iter().find(|f| f.base == base)?;
    names_for(family, locales).find(|name| exists(name))
}

/// Whether `name` is a language pack of `base`, judged by the family's
/// name patterns. Used to remove packs together with their base package.
pub fn is_pack_of(base: &str, name: &str) -> bool {
    FAMILIES.iter().filter(|f| f.base == base).any(|f| {
        f.patterns.iter().any(|p| {
            let prefix = p.split('{').next().unwrap_or(p);
            name != base && name.len() > prefix.len() && name.starts_with(prefix)
        })
    })
}

/// Picks the provider that matches the user's language, when libalpm asks
/// which of several packages should satisfy a dependency (for example one of
/// the many `tesseract-data-*` packages).
pub fn pick_provider<'a>(providers: &[&'a str], locales: &[Locale]) -> Option<&'a str> {
    for locale in locales {
        for family in FAMILIES {
            for name in locale.candidates(family.patterns) {
                if let Some(p) = providers.iter().find(|p| **p == name) {
                    return Some(p);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locales(list: &[&str]) -> Vec<Locale> {
        list.iter().map(|l| Locale::parse(l).unwrap()).collect()
    }

    #[test]
    fn adds_the_matching_pack() {
        let de = locales(&["de_DE"]);
        let repo = [
            "libreoffice-fresh-de",
            "firefox-i18n-de",
            "hunspell-de",
            "tesseract-data-deu",
        ];
        let exists = |n: &str| repo.contains(&n);
        assert_eq!(
            for_package("libreoffice-fresh", &de, exists).as_deref(),
            Some("libreoffice-fresh-de")
        );
        assert_eq!(
            for_package("firefox", &de, exists).as_deref(),
            Some("firefox-i18n-de")
        );
        assert_eq!(
            for_package("hunspell", &de, exists).as_deref(),
            Some("hunspell-de")
        );
        assert_eq!(
            for_package("tesseract", &de, exists).as_deref(),
            Some("tesseract-data-deu")
        );
        assert_eq!(for_package("vlc", &de, exists), None);
    }

    #[test]
    fn regional_packs_win_and_english_is_built_in() {
        let repo = [
            "firefox-i18n-en-gb",
            "firefox-i18n-pt-br",
            "firefox-i18n-pt-pt",
            "hunspell-en",
        ];
        let exists = |n: &str| repo.contains(&n);
        assert_eq!(
            for_package("firefox", &locales(&["pt_BR"]), exists).as_deref(),
            Some("firefox-i18n-pt-br")
        );
        // Firefox already speaks English; hunspell needs an English dictionary.
        assert_eq!(for_package("firefox", &locales(&["en_GB"]), exists), None);
        assert_eq!(
            for_package("hunspell", &locales(&["en_US"]), exists).as_deref(),
            Some("hunspell-en")
        );
        // Fallback languages are tried in order.
        assert_eq!(
            for_package("firefox", &locales(&["xx", "pt_PT"]), exists).as_deref(),
            Some("firefox-i18n-pt-pt")
        );
    }

    #[test]
    fn recognises_packs() {
        assert!(is_pack_of("tesseract", "tesseract-data-deu"));
        assert!(is_pack_of("firefox", "firefox-i18n-de"));
        assert!(!is_pack_of("firefox", "firefox-developer-edition"));
        assert!(!is_pack_of("tesseract", "tesseract"));
        assert!(!is_pack_of("tesseract", "tesseract-data-"));
        assert!(!is_pack_of("vlc", "vlc-plugins-all"));
    }

    #[test]
    fn picks_providers_by_language() {
        let providers = ["tesseract-data-afr", "tesseract-data-deu", "tesseract-data-eng"];
        assert_eq!(
            pick_provider(&providers, &locales(&["de_DE.UTF-8"])),
            Some("tesseract-data-deu")
        );
        assert_eq!(pick_provider(&providers, &locales(&["ja_JP"])), None);
        assert_eq!(
            pick_provider(&providers, &locales(&["ja_JP", "en_US"])),
            Some("tesseract-data-eng")
        );
    }
}
