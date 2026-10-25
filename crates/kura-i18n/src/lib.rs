// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Translations for kura.
//!
//! Messages live in `i18n/<locale>/kura.ftl` at the repository root and are
//! compiled into the binary. English is the source and the last fallback.

pub mod present;

use std::borrow::Cow;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use kura_core::locale::Locale;
use unic_langid::LanguageIdentifier;

/// Every shipped language with its message file.
pub const LOCALES: &[(&str, &str)] = &[
    ("en", include_str!("../../../i18n/en/kura.ftl")),
    ("de", include_str!("../../../i18n/de/kura.ftl")),
    ("fr", include_str!("../../../i18n/fr/kura.ftl")),
    ("ja", include_str!("../../../i18n/ja/kura.ftl")),
];

/// Looks up messages in the user's languages, falling back to English.
pub struct Localizer {
    bundles: Vec<FluentBundle<FluentResource>>,
}

impl std::fmt::Debug for Localizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Localizer")
            .field("language", &self.language())
            .finish()
    }
}

fn bundle(code: &str, source: &str) -> FluentBundle<FluentResource> {
    let lang: LanguageIdentifier = code.parse().expect("valid built-in language code");
    let resource = FluentResource::try_new(source.to_owned()).unwrap_or_else(|(res, _)| res); // syntax errors are caught by tests
    let mut b = FluentBundle::new(vec![lang]);
    // Bidi isolation marks show up as garbage in terminals; Qt handles direction itself.
    b.set_use_isolating(false);
    b.add_resource(resource).expect("message IDs are unique per file");
    b
}

impl Localizer {
    /// Builds a localizer for the given preferences, most preferred first.
    ///
    /// # Panics
    ///
    /// Only if a built-in message file is broken, which the tests rule out.
    pub fn new(preferred: &[Locale]) -> Self {
        let mut codes: Vec<&str> = Vec::new();
        for l in preferred {
            if let Some((code, _)) = LOCALES.iter().find(|(c, _)| *c == l.language)
                && !codes.contains(code)
            {
                codes.push(code);
            }
        }
        if !codes.contains(&"en") {
            codes.push("en");
        }
        let bundles = codes
            .iter()
            .map(|code| {
                let (_, src) = LOCALES.iter().find(|(c, _)| c == code).expect("listed above");
                bundle(code, src)
            })
            .collect();
        Self { bundles }
    }

    /// Uses the language settings of the current process.
    pub fn from_env() -> Self {
        Self::new(&Locale::from_env_vars(|k| std::env::var(k).ok()))
    }

    /// The language messages are shown in first.
    pub fn language(&self) -> String {
        self.bundles[0].locales[0].to_string()
    }

    /// A message without variables.
    pub fn msg(&self, id: &str) -> String {
        self.format(id, None)
    }

    /// A message with variables, for example `[("count", 3.into())]`.
    pub fn msg_args(&self, id: &str, args: &[(&str, FluentValue<'_>)]) -> String {
        let mut fa = FluentArgs::new();
        for (k, v) in args {
            fa.set(*k, v.clone());
        }
        self.format(id, Some(&fa))
    }

    /// A size in bytes for people: `184 MB`, `1,2 GB` in German.
    ///
    /// Decimal units (1 MB = 1,000,000 bytes), as file managers show them.
    /// A no-break space keeps number and unit on one line.
    pub fn size(&self, bytes: u64) -> String {
        const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
        let mut unit = 0;
        let mut whole = bytes;
        let mut tenths = 0;
        while whole >= 1000 && unit < UNITS.len() - 1 {
            tenths = (whole % 1000) / 100;
            whole /= 1000;
            unit += 1;
        }
        let sep = if matches!(self.language().as_str(), "de" | "fr") {
            ','
        } else {
            '.'
        };
        if unit > 0 && whole < 10 && tenths > 0 {
            format!("{whole}{sep}{tenths}\u{a0}{}", UNITS[unit])
        } else {
            format!("{whole}\u{a0}{}", UNITS[unit])
        }
    }

    /// A size change with sign: `+184 MB`, `−12 MB`.
    pub fn size_delta(&self, bytes: i64) -> String {
        let sign = if bytes < 0 { '\u{2212}' } else { '+' };
        format!("{sign}{}", self.size(bytes.unsigned_abs()))
    }

    fn format(&self, id: &str, args: Option<&FluentArgs<'_>>) -> String {
        for b in &self.bundles {
            if let Some(pattern) = b.get_message(id).and_then(|m| m.value()) {
                let mut errors = Vec::new();
                let text: Cow<'_, str> = b.format_pattern(pattern, args, &mut errors);
                return text.into_owned();
            }
        }
        // A missing ID is a bug caught by the tests; show the ID rather than nothing.
        id.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use fluent_syntax::ast;
    use fluent_syntax::parser::parse;

    use super::*;

    fn loc(s: &str) -> Localizer {
        Localizer::new(&[Locale::parse(s).unwrap()])
    }

    fn messages(src: &str) -> Vec<(String, BTreeSet<String>)> {
        let res = parse(src).expect("file parses without errors");
        res.body
            .into_iter()
            .filter_map(|e| match e {
                ast::Entry::Message(m) => {
                    let mut vars = BTreeSet::new();
                    if let Some(p) = &m.value {
                        collect_pattern(p, &mut vars);
                    }
                    Some((m.id.name.to_owned(), vars))
                }
                _ => None,
            })
            .collect()
    }

    fn collect_pattern(p: &ast::Pattern<&str>, vars: &mut BTreeSet<String>) {
        for el in &p.elements {
            if let ast::PatternElement::Placeable { expression } = el {
                collect_expr(expression, vars);
            }
        }
    }

    fn collect_expr(e: &ast::Expression<&str>, vars: &mut BTreeSet<String>) {
        match e {
            ast::Expression::Inline(i) => collect_inline(i, vars),
            ast::Expression::Select { selector, variants } => {
                collect_inline(selector, vars);
                for v in variants {
                    collect_pattern(&v.value, vars);
                }
            }
        }
    }

    fn collect_inline(i: &ast::InlineExpression<&str>, vars: &mut BTreeSet<String>) {
        match i {
            ast::InlineExpression::VariableReference { id } => {
                vars.insert(id.name.to_owned());
            }
            ast::InlineExpression::Placeable { expression } => collect_expr(expression, vars),
            _ => {}
        }
    }

    #[test]
    fn every_locale_has_exactly_the_english_messages_and_variables() {
        let en = messages(LOCALES[0].1);
        assert_ne!(en.len(), 0);
        for (code, src) in &LOCALES[1..] {
            let other = messages(src);
            let en_ids: BTreeSet<_> = en.iter().map(|(id, _)| id).collect();
            let other_ids: BTreeSet<_> = other.iter().map(|(id, _)| id).collect();
            assert_eq!(en_ids, other_ids, "message IDs differ in {code}");
            for (id, vars) in &en {
                let (_, ov) = other.iter().find(|(i, _)| i == id).unwrap();
                assert_eq!(vars, ov, "variables differ for {id} in {code}");
            }
        }
    }

    #[test]
    fn picks_the_preferred_language_and_falls_back() {
        assert_eq!(loc("de_DE.UTF-8").msg("doctor-language"), "Sprache");
        assert_eq!(loc("ja_JP").msg("doctor-language"), "言語");
        assert_eq!(loc("fr").language(), "fr");
        assert_eq!(loc("es_ES").language(), "en");
        assert_eq!(loc("es_ES").msg("doctor-language"), "Language");
        assert_eq!(Localizer::new(&[]).language(), "en");
    }

    #[test]
    fn plurals_and_variables() {
        let en = loc("en");
        assert_eq!(
            en.msg_args("orphans-found", &[("count", 1.into())]),
            "One package is no longer needed:"
        );
        assert_eq!(
            en.msg_args("orphans-found", &[("count", 3.into())]),
            "3 packages are no longer needed:"
        );
        assert_eq!(
            loc("de").msg_args("cli-unknown-command", &[("command", "foo".into())]),
            "Unbekannter Befehl: foo"
        );
    }

    #[test]
    fn sizes_follow_the_language() {
        let en = loc("en");
        assert_eq!(en.size(0), "0\u{a0}B");
        assert_eq!(en.size(999), "999\u{a0}B");
        assert_eq!(en.size(1_000), "1\u{a0}kB");
        assert_eq!(en.size(184_200_000), "184\u{a0}MB");
        assert_eq!(en.size(1_250_000_000), "1.2\u{a0}GB");
        assert_eq!(en.size(5_000_000_000_000_000), "5000\u{a0}TB");
        assert_eq!(loc("de").size(1_250_000_000), "1,2\u{a0}GB");
        assert_eq!(loc("fr").size(2_100_000_000), "2,1\u{a0}GB");
        assert_eq!(loc("ja").size(2_100_000_000), "2.1\u{a0}GB");
        assert_eq!(en.size_delta(184_000_000), "+184\u{a0}MB");
        assert_eq!(en.size_delta(-12_000_000), "\u{2212}12\u{a0}MB");
    }

    #[test]
    fn unknown_ids_show_the_id() {
        assert_eq!(loc("en").msg("does-not-exist"), "does-not-exist");
    }

    #[test]
    fn duplicate_preferences_are_merged() {
        let l = Localizer::new(&[Locale::parse("de_DE").unwrap(), Locale::parse("de_AT").unwrap()]);
        assert_eq!(l.bundles.len(), 2);
        assert!(format!("{l:?}").contains("de"));
    }

    #[test]
    fn from_env_does_not_panic() {
        let _ = Localizer::from_env().language();
    }
}
