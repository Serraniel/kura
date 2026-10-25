// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Mapping the system language to the codes package families use for
//! language packs (`libreoffice-fresh-de`, `tesseract-data-deu`, ...).

/// A language with an optional region, parsed from a POSIX locale or BCP 47 tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locale {
    /// ISO 639-1 code, lowercase (`de`).
    pub language: String,
    /// ISO 3166 region, uppercase (`DE`).
    pub region: Option<String>,
}

impl Locale {
    /// Parses `de_DE.UTF-8`, `de_DE@euro`, `de-DE`, `de` and similar.
    /// Returns `None` for `C`, `POSIX` and empty values.
    pub fn parse(s: &str) -> Option<Self> {
        let base = s.split(['.', '@']).next()?.trim();
        if base.is_empty() || base.eq_ignore_ascii_case("c") || base.eq_ignore_ascii_case("posix") {
            return None;
        }
        let mut parts = base.split(['_', '-']);
        let language = parts.next()?.to_ascii_lowercase();
        if !(2..=3).contains(&language.len()) || !language.bytes().all(|b| b.is_ascii_alphabetic()) {
            return None;
        }
        let region = parts
            .next()
            .filter(|r| r.len() == 2 && r.bytes().all(|b| b.is_ascii_alphabetic()))
            .map(str::to_ascii_uppercase);
        Some(Self { language, region })
    }

    /// Reads the user's language preferences from the environment, most
    /// preferred first, in the order glibc uses.
    pub fn from_env_vars(get: impl Fn(&str) -> Option<String>) -> Vec<Self> {
        let mut out: Vec<Self> = Vec::new();
        let mut push = |l: Option<Self>| {
            if let Some(l) = l
                && !out.contains(&l)
            {
                out.push(l);
            }
        };
        let primary = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .find_map(|k| get(k).filter(|v| !v.is_empty()));
        // LANGUAGE is ignored by glibc when the locale is C.
        if primary.as_deref().and_then(Self::parse).is_some()
            && let Some(list) = get("LANGUAGE")
        {
            for item in list.split(':') {
                push(Self::parse(item));
            }
        }
        push(primary.as_deref().and_then(Self::parse));
        out
    }

    fn codes(&self) -> Option<&'static LanguageCodes> {
        LANGUAGES
            .iter()
            .find(|c| c.iso639_1 == self.language || c.iso639_2t == self.language)
    }

    /// Fills a package name pattern. Placeholders:
    /// `{iso639_1}` de, `{iso639_2t}` deu, `{iso639_2b}` ger, `{bcp47}` de-DE,
    /// `{bcp47_lower}` de-de, `{posix}` `de_DE`.
    ///
    /// Returns `None` when the pattern needs a code this locale does not have.
    pub fn fill(&self, pattern: &str) -> Option<String> {
        let mut out = pattern.to_owned();
        if out.contains("{iso639_2t}") || out.contains("{iso639_2b}") {
            let codes = self.codes()?;
            out = out
                .replace("{iso639_2t}", codes.iso639_2t)
                .replace("{iso639_2b}", codes.iso639_2b);
        }
        let lang = self.codes().map_or(self.language.as_str(), |c| c.iso639_1);
        let needs_region = ["{bcp47}", "{bcp47_lower}", "{posix}"]
            .iter()
            .any(|p| out.contains(p));
        if needs_region {
            let region = self.region.as_deref()?;
            out = out
                .replace(
                    "{bcp47_lower}",
                    &format!("{lang}-{}", region.to_ascii_lowercase()),
                )
                .replace("{bcp47}", &format!("{lang}-{region}"))
                .replace("{posix}", &format!("{lang}_{region}"));
        }
        Some(out.replace("{iso639_1}", lang))
    }

    /// Candidate package names for a pattern, most specific first.
    /// `firefox-i18n-{bcp47_lower}` for `de_AT` gives `firefox-i18n-de-at`; the
    /// caller then tries the plain-language variant from a second pattern.
    pub fn candidates(&self, patterns: &[&str]) -> Vec<String> {
        patterns.iter().filter_map(|p| self.fill(p)).collect()
    }
}

/// ISO 639 codes for one language.
#[derive(Debug, PartialEq, Eq)]
pub struct LanguageCodes {
    pub iso639_1: &'static str,
    /// Terminology code, used by tesseract and most tools.
    pub iso639_2t: &'static str,
    /// Bibliographic code, differs for some languages (`ger`, `fre`).
    pub iso639_2b: &'static str,
}

macro_rules! langs {
    ($(($a:literal, $t:literal, $b:literal)),* $(,)?) => {
        &[$(LanguageCodes { iso639_1: $a, iso639_2t: $t, iso639_2b: $b }),*]
    };
}

/// Languages commonly shipped as package language packs.
pub static LANGUAGES: &[LanguageCodes] = langs![
    ("ar", "ara", "ara"),
    ("bg", "bul", "bul"),
    ("ca", "cat", "cat"),
    ("cs", "ces", "cze"),
    ("da", "dan", "dan"),
    ("de", "deu", "ger"),
    ("el", "ell", "gre"),
    ("en", "eng", "eng"),
    ("es", "spa", "spa"),
    ("et", "est", "est"),
    ("eu", "eus", "baq"),
    ("fa", "fas", "per"),
    ("fi", "fin", "fin"),
    ("fr", "fra", "fre"),
    ("he", "heb", "heb"),
    ("hi", "hin", "hin"),
    ("hr", "hrv", "hrv"),
    ("hu", "hun", "hun"),
    ("id", "ind", "ind"),
    ("is", "isl", "ice"),
    ("it", "ita", "ita"),
    ("ja", "jpn", "jpn"),
    ("ko", "kor", "kor"),
    ("lt", "lit", "lit"),
    ("lv", "lav", "lav"),
    ("nb", "nob", "nob"),
    ("nl", "nld", "dut"),
    ("nn", "nno", "nno"),
    ("pl", "pol", "pol"),
    ("pt", "por", "por"),
    ("ro", "ron", "rum"),
    ("ru", "rus", "rus"),
    ("sk", "slk", "slo"),
    ("sl", "slv", "slv"),
    ("sr", "srp", "srp"),
    ("sv", "swe", "swe"),
    ("th", "tha", "tha"),
    ("tr", "tur", "tur"),
    ("uk", "ukr", "ukr"),
    ("vi", "vie", "vie"),
    ("zh", "zho", "chi"),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn l(s: &str) -> Locale {
        Locale::parse(s).unwrap()
    }

    #[test]
    fn parses_common_forms() {
        assert_eq!(
            l("de_DE.UTF-8"),
            Locale {
                language: "de".into(),
                region: Some("DE".into())
            }
        );
        assert_eq!(l("de_AT@euro").region.as_deref(), Some("AT"));
        assert_eq!(l("pt-br").region.as_deref(), Some("BR"));
        assert_eq!(
            l("ja"),
            Locale {
                language: "ja".into(),
                region: None
            }
        );
        assert_eq!(l("en_US.utf8").language, "en");
        for bad in ["", "C", "C.UTF-8", "POSIX", "x", "toolong", "1a_DE"] {
            assert_eq!(Locale::parse(bad), None, "{bad}");
        }
        assert_eq!(l("de_123").region, None);
    }

    #[test]
    fn fills_package_patterns() {
        let de = l("de_DE.UTF-8");
        assert_eq!(
            de.fill("tesseract-data-{iso639_2t}").as_deref(),
            Some("tesseract-data-deu")
        );
        assert_eq!(de.fill("something-{iso639_2b}").as_deref(), Some("something-ger"));
        assert_eq!(
            de.fill("libreoffice-fresh-{iso639_1}").as_deref(),
            Some("libreoffice-fresh-de")
        );
        assert_eq!(
            de.fill("firefox-i18n-{bcp47_lower}").as_deref(),
            Some("firefox-i18n-de-de")
        );
        assert_eq!(de.fill("hunspell-{posix}").as_deref(), Some("hunspell-de_DE"));
        assert_eq!(de.fill("x-{bcp47}").as_deref(), Some("x-de-DE"));
        assert_eq!(de.fill("plain").as_deref(), Some("plain"));
    }

    #[test]
    fn missing_codes_give_none() {
        assert_eq!(l("ja").fill("firefox-i18n-{bcp47_lower}"), None);
        assert_eq!(l("xx_XX").fill("tesseract-data-{iso639_2t}"), None);
        assert_eq!(l("xx_XX").fill("pack-{iso639_1}").as_deref(), Some("pack-xx"));
    }

    #[test]
    fn three_letter_input_maps_back() {
        assert_eq!(
            l("deu").fill("libreoffice-fresh-{iso639_1}").as_deref(),
            Some("libreoffice-fresh-de")
        );
    }

    #[test]
    fn candidates_skip_unfillable_patterns() {
        let fr = l("fr");
        assert_eq!(
            fr.candidates(&["firefox-i18n-{bcp47_lower}", "firefox-i18n-{iso639_1}"]),
            ["firefox-i18n-fr"]
        );
    }

    #[test]
    fn reads_environment_in_glibc_order() {
        let env = |vars: &'static [(&'static str, &'static str)]| {
            move |k: &str| vars.iter().find(|(n, _)| *n == k).map(|(_, v)| (*v).to_owned())
        };
        let got = Locale::from_env_vars(env(&[("LANG", "de_DE.UTF-8"), ("LANGUAGE", "de_DE:en_GB:de")]));
        let langs: Vec<_> = got
            .iter()
            .map(|l| (l.language.as_str(), l.region.as_deref()))
            .collect();
        assert_eq!(langs, [("de", Some("DE")), ("en", Some("GB")), ("de", None)]);

        let c_locale = Locale::from_env_vars(env(&[("LANG", "C"), ("LANGUAGE", "de")]));
        assert_eq!(c_locale, Vec::<Locale>::new());

        let lc_all = Locale::from_env_vars(env(&[("LC_ALL", "fr_FR.UTF-8"), ("LANG", "de_DE.UTF-8")]));
        assert_eq!(lc_all[0].language, "fr");
        assert_eq!(lc_all.len(), 1);
    }

    #[test]
    fn code_table_is_consistent() {
        for c in LANGUAGES {
            assert_eq!(c.iso639_1.len(), 2);
            assert_eq!(c.iso639_2t.len(), 3);
            assert_eq!(c.iso639_2b.len(), 3);
        }
    }
}
