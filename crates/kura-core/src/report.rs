// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Problem reports. kura never sends them: it shows the report, and on
//! request opens a prefilled GitHub issue in the browser.

use std::fmt::Write as _;

pub const ISSUES: &str = "https://github.com/Serraniel/kura/issues/new";

/// Browsers and GitHub cut very long addresses; the report is shortened to fit.
const MAX_URL: usize = 7_500;

/// Who and where, to be removed from reports.
#[derive(Debug, Clone, Default)]
pub struct Identity {
    pub user: String,
    pub host: String,
    pub home: String,
}

fn replace_word(text: &str, word: &str, with: &str) -> String {
    if word.len() < 2 {
        return text.to_owned();
    }
    let boundary = |c: Option<char>| c.is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '-'));
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(word) {
        let before = rest[..i].chars().next_back();
        let after = rest[i + word.len()..].chars().next();
        out.push_str(&rest[..i]);
        if boundary(before) && boundary(after) {
            out.push_str(with);
        } else {
            out.push_str(word);
        }
        rest = &rest[i + word.len()..];
    }
    out.push_str(rest);
    out
}

/// Removes the home directory, the user name and the host name.
pub fn anonymize(text: &str, who: &Identity) -> String {
    let mut out = text.to_owned();
    if who.home.len() > 1 {
        out = out.replace(who.home.trim_end_matches('/'), "~");
    }
    out = replace_word(&out, &who.host, "<host>");
    replace_word(&out, &who.user, "<user>")
}

/// Percent-encodes everything except unreserved characters.
pub fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// A link that opens the bug form with the fields filled in. Long reports
/// are cut at a line end so the link stays usable.
pub fn issue_url(title: &str, what: &str, report: &str, version: &str) -> String {
    let head = format!(
        "{ISSUES}?template=bug.yml&title={}&what={}&version={}&report=",
        encode(title),
        encode(what),
        encode(version)
    );
    let budget = MAX_URL.saturating_sub(head.len());
    let mut body = String::new();
    for (i, line) in report.lines().enumerate() {
        let piece = if i == 0 {
            encode(line)
        } else {
            format!("%0A{}", encode(line))
        };
        if body.len() + piece.len() > budget {
            break;
        }
        body.push_str(&piece);
    }
    head + &body
}

#[cfg(test)]
mod tests {
    use super::*;

    fn me() -> Identity {
        Identity {
            user: "anna".into(),
            host: "annas-pc".into(),
            home: "/home/anna".into(),
        }
    }

    #[test]
    fn removes_who_and_where() {
        let text = "anna@annas-pc: cannot read /home/anna/.cache/kura (owner anna, group annalena)";
        assert_eq!(
            anonymize(text, &me()),
            "<user>@<host>: cannot read ~/.cache/kura (owner <user>, group annalena)"
        );
        let short = Identity {
            user: "a".into(),
            ..Identity::default()
        };
        assert_eq!(anonymize("a package", &short), "a package");
    }

    #[test]
    fn builds_prefilled_links() {
        assert_eq!(encode("a b/ä"), "a%20b%2F%C3%A4");
        let url = issue_url("Update fails", "kura update stopped", "line 1\nline 2", "0.1.0");
        assert!(url.starts_with(
            "https://github.com/Serraniel/kura/issues/new?template=bug.yml&title=Update%20fails"
        ));
        assert!(url.ends_with("&report=line%201%0Aline%202"));

        let long = (0..2000)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let url = issue_url("t", "w", &long, "0.1.0");
        assert!(url.len() <= MAX_URL);
        assert!(url.contains("&report=line%200%0Aline%201%0A"));
        assert!(!url.ends_with("%0A"));
        let huge = "x".repeat(10_000);
        assert!(issue_url("t", "w", &huge, "v").ends_with("&report="));
    }
}
