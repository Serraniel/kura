// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Arch Linux news: reading the feed and finding items that concern an update.
//!
//! Arch announces manual steps in its news. An update that contains a package
//! named in a recent news item is stopped until the user has seen the item.

use std::collections::BTreeSet;

/// One news item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub title: String,
    pub link: String,
    /// Unix time in seconds.
    pub published: i64,
    /// Text without HTML.
    pub text: String,
}

/// Reads an RSS 2.0 feed. Items that cannot be read are skipped.
pub fn parse_rss(xml: &str) -> Vec<Item> {
    let mut items = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<item>") {
        let after = &rest[start + 6..];
        let Some(end) = after.find("</item>") else { break };
        let body = &after[..end];
        rest = &after[end + 7..];
        let field = |tag: &str| element(body, tag).map(|v| decode(&v));
        let (Some(title), Some(date)) = (field("title"), field("pubDate")) else {
            continue;
        };
        let Some(published) = parse_rfc2822(&date) else {
            continue;
        };
        items.push(Item {
            title: title.trim().to_owned(),
            link: field("link").unwrap_or_default().trim().to_owned(),
            published,
            text: strip_html(&field("description").unwrap_or_default()),
        });
    }
    items
}

fn element(body: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = body.find(&open)? + open.len();
    let end = start + body[start..].find(&close)?;
    let raw = body[start..end].trim();
    let raw = raw
        .strip_prefix("<![CDATA[")
        .and_then(|r| r.strip_suffix("]]>"))
        .unwrap_or(raw);
    Some(raw.to_owned())
}

/// Replaces the XML and HTML entities that appear in feeds.
fn decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let Some(semi) = tail.find(';').filter(|&n| n <= 10) else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };
        let entity = &tail[1..semi];
        let ch = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ => entity
                .strip_prefix("#x")
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        if let Some(c) = ch {
            out.push(c);
            rest = &tail[semi + 1..];
        } else {
            out.push('&');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

/// Turns the HTML of a news item into plain text with paragraphs.
fn strip_html(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    let mut tag = String::new();
    for c in html.chars() {
        match (in_tag, c) {
            (false, '<') => {
                in_tag = true;
                tag.clear();
            }
            (true, '>') => {
                in_tag = false;
                let name = tag
                    .trim_start_matches('/')
                    .split_whitespace()
                    .next()
                    .unwrap_or_default();
                if matches!(name, "p" | "br" | "li" | "pre") && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            (true, c) => tag.push(c),
            (false, c) => out.push(c),
        }
    }
    let text = decode(&out);
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parses dates like `Sat, 01 Nov 2025 12:30:00 +0000` into Unix seconds.
pub fn parse_rfc2822(s: &str) -> Option<i64> {
    let s = s.split_once(',').map_or(s, |(_, rest)| rest).trim();
    let mut parts = s.split_whitespace();
    let day: i64 = parts.next()?.parse().ok()?;
    let month = match parts.next()?.to_ascii_lowercase().as_str() {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        _ => return None,
    };
    let year: i64 = parts.next()?.parse().ok()?;
    let mut time = parts.next()?.split(':').map(str::parse::<i64>);
    let (h, m) = (time.next()?.ok()?, time.next()?.ok()?);
    let sec = time.next().and_then(Result::ok).unwrap_or(0);
    let offset = match parts.next() {
        Some(z) if z.len() == 5 && (z.starts_with('+') || z.starts_with('-')) => {
            let sign = if z.starts_with('-') { -1 } else { 1 };
            let hh: i64 = z[1..3].parse().ok()?;
            let mm: i64 = z[3..5].parse().ok()?;
            sign * (hh * 3600 + mm * 60)
        }
        _ => 0, // GMT, UTC or missing
    };
    if !(1..=31).contains(&day) || h > 23 || m > 59 || sec > 60 {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + h * 3600 + m * 60 + sec - offset)
}

/// Days since 1970-01-01 (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Words of a text, lowercase, keeping the characters package names use.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+' | '.')))
        .map(|w| w.trim_matches('.').to_ascii_lowercase())
        .filter(|w| !w.is_empty())
        .collect()
}

/// Package names an item talks about, out of `packages`. "Arch Linux" does not
/// count as the `linux` package.
pub fn mentioned<'a>(item: &Item, packages: &'a BTreeSet<String>) -> BTreeSet<&'a str> {
    let mut found = BTreeSet::new();
    for source in [&item.title, &item.text] {
        let ws = words(source);
        for (i, w) in ws.iter().enumerate() {
            if w == "linux" && i > 0 && ws[i - 1] == "arch" {
                continue;
            }
            if let Some(p) = packages.get(w.as_str()) {
                found.insert(p.as_str());
            }
        }
    }
    found
}

/// Items published after `since` that name a package in `packages`, newest first.
pub fn affecting<'a>(items: &'a [Item], since: i64, packages: &BTreeSet<String>) -> Vec<&'a Item> {
    let mut hits: Vec<&Item> = items
        .iter()
        .filter(|i| i.published > since && !mentioned(i, packages).is_empty())
        .collect();
    hits.sort_by_key(|i| std::cmp::Reverse(i.published));
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEED: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0"><channel><title>Arch Linux: Recent news updates</title>
<item><title>NVIDIA 590 driver drops Pascal support; main packages switch to Open Kernel Modules</title>
<link>https://archlinux.org/news/nvidia-590/</link>
<description>&lt;p&gt;The &lt;code&gt;nvidia&lt;/code&gt; 590 driver no longer supports Pascal.&lt;/p&gt;&lt;ul&gt;&lt;li&gt;Install nvidia-580xx-dkms&lt;/li&gt;&lt;/ul&gt;</description>
<pubDate>Sat, 20 Dec 2025 10:00:00 +0000</pubDate></item>
<item><title><![CDATA[Arch Linux 2026 survey & results]]></title>
<link>https://archlinux.org/news/survey/</link>
<description>Thanks &#8212; everyone!</description>
<pubDate>Mon, 05 Jan 2026 08:30:00 GMT</pubDate></item>
<item><title>broken item without date</title></item>
<item><title>linux 7.0 needs a manual step</title><link>x</link><pubDate>Tue, 03 Feb 2026 23:00:00 -0100</pubDate></item>
</channel></rss>"#;

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn parses_items_entities_and_html() {
        let items = parse_rss(FEED);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].link, "https://archlinux.org/news/nvidia-590/");
        assert_eq!(
            items[0].text,
            "The nvidia 590 driver no longer supports Pascal.\nInstall nvidia-580xx-dkms"
        );
        assert_eq!(items[1].title, "Arch Linux 2026 survey & results");
        assert_eq!(items[1].text, "Thanks \u{2014} everyone!");
    }

    #[test]
    fn dates() {
        assert_eq!(parse_rfc2822("Thu, 01 Jan 1970 00:00:00 +0000"), Some(0));
        assert_eq!(
            parse_rfc2822("Sat, 20 Dec 2025 10:00:00 +0000"),
            Some(1_766_224_800)
        );
        assert_eq!(parse_rfc2822("20 Dec 2025 11:00:00 +0100"), Some(1_766_224_800));
        assert_eq!(
            parse_rfc2822("Tue, 03 Feb 2026 23:00:00 -0100"),
            Some(1_770_163_200)
        );
        assert_eq!(parse_rfc2822("Mon, 29 Feb 2016 00:00 GMT"), Some(1_456_704_000));
        for bad in [
            "",
            "Mon, 32 Jan 2026 00:00:00 +0000",
            "Mon, 01 Foo 2026 00:00:00",
            "Mon, 01 Jan 2026 25:00:00",
            "Mon, 01 Jan 2026",
        ] {
            assert_eq!(parse_rfc2822(bad), None, "{bad}");
        }
    }

    #[test]
    fn finds_items_that_concern_the_update() {
        let items = parse_rss(FEED);
        let update = set(&["nvidia-580xx-dkms", "linux", "firefox"]);
        let hits = affecting(&items, 0, &update);
        let titles: Vec<_> = hits.iter().map(|i| i.title.as_str()).collect();
        // "Arch Linux" in the survey title is not the linux package.
        assert_eq!(titles, ["linux 7.0 needs a manual step", items[0].title.as_str()]);
        assert_eq!(
            mentioned(&items[0], &update).into_iter().collect::<Vec<_>>(),
            ["nvidia-580xx-dkms"]
        );
        // Only items newer than the last update count.
        assert_eq!(affecting(&items, 1_770_000_000, &update).len(), 1);
        assert_eq!(affecting(&items, 0, &set(&["firefox"])).len(), 0);
    }

    #[test]
    fn decoding_edge_cases() {
        assert_eq!(
            decode("a &amp; b &lt;c&gt; &quot;d&quot; &apos;e&apos; &#x41;&#66;"),
            "a & b <c> \"d\" 'e' AB"
        );
        assert_eq!(decode("fish & chips"), "fish & chips");
        assert_eq!(decode("&unknown; &#xZZ;"), "&unknown; &#xZZ;");
        assert_eq!(decode("&nbsp;"), " ");
        assert_eq!(strip_html("<pre>one</pre><br/>two"), "one\ntwo");
    }
}
