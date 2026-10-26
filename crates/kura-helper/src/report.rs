// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The system summary, the history list and problem reports, shared by the
//! command line and the window.

use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use kura_core::package::InstallReason;
use kura_i18n::Localizer;

/// `NAME` and `ID` from /etc/os-release.
pub fn os_release() -> (String, String) {
    let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
            .map(|v| v.trim_matches('"').to_owned())
    };
    (
        field("NAME").unwrap_or_else(|| "Linux".into()),
        field("ID").unwrap_or_else(|| "linux".into()),
    )
}

/// A short summary of the system, as `kura doctor` shows it.
pub fn summary(l: &Localizer) -> Result<String, String> {
    let set = kura_alpm::load_config()
        .and_then(|conf| kura_alpm::installed(&conf))
        .map_err(|e| l.msg_args("error-database", &[("reason", e.to_string().into())]))?;
    let (name, _) = os_release();
    let explicit = set.iter().filter(|p| p.reason == InstallReason::Explicit).count();
    let foreign = set.iter().filter(|p| p.repo.is_none()).count();
    let conf = kura_alpm::load_config()
        .map_err(|e| l.msg_args("error-database", &[("reason", e.to_string().into())]))?;
    let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
    let report = kura_alpm::system::snapshot_report(&conf, &set, &mounts);
    let (snapshot_state, missing) = kura_i18n::present::snapshots(l, &report);
    let rows = [
        (l.msg("doctor-distro"), name),
        (l.msg("doctor-packages"), set.len().to_string()),
        (l.msg("doctor-explicit"), explicit.to_string()),
        (l.msg("doctor-foreign"), foreign.to_string()),
        (l.msg("doctor-orphans"), set.orphans().len().to_string()),
        (l.msg("doctor-language"), l.language()),
        (l.msg("health-snapshots"), snapshot_state.clone()),
    ];
    let width = rows.iter().map(|(k, _)| k.chars().count()).max().unwrap_or(0);
    let mut out = format!("{}\n\n", l.msg("doctor-title"));
    for (k, v) in rows {
        let pad = width - k.chars().count();
        let _ = writeln!(out, "  {k}{}  {v}", " ".repeat(pad));
    }
    if !missing.is_empty() {
        let _ = writeln!(out, "\n{}", l.msg("snapshots-missing"));
        for step in missing {
            let _ = writeln!(out, "  {step}");
        }
    }
    Ok(out)
}

/// Saved states of the system, newest first.
pub fn history(l: &Localizer) -> Result<String, String> {
    let journal = kura_alpm::history::load(std::path::Path::new(kura_alpm::history::JOURNAL))
        .map_err(|e| l.msg_args("error-database", &[("reason", e.to_string().into())]))?;
    if journal.is_empty() {
        return Ok(l.msg("history-empty") + "\n");
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
    let rows: Vec<(String, String, String)> = journal
        .records()
        .iter()
        .rev()
        .map(|r| {
            let text = kura_i18n::present::record_text(l, r);
            (
                format!("#{}", r.id),
                kura_i18n::present::when(l, r.created, now),
                text,
            )
        })
        .collect();
    let w0 = rows.iter().map(|r| r.0.chars().count()).max().unwrap_or(0);
    let w1 = rows.iter().map(|r| r.1.chars().count()).max().unwrap_or(0);
    let mut out = l.msg("history-title") + "\n\n";
    for (id, day, text) in rows {
        let _ = writeln!(out, "  {id:>w0$}  {day:w1$}  {text}");
    }
    Ok(out)
}

/// Who uses this computer, to be removed from reports.
fn identity() -> kura_core::report::Identity {
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    kura_core::report::Identity {
        user: std::env::var("USER").unwrap_or_else(|_| env("LOGNAME")),
        host: std::fs::read_to_string("/proc/sys/kernel/hostname")
            .unwrap_or_default()
            .trim()
            .to_owned(),
        home: env("HOME"),
    }
}

/// The anonymised report and the link that opens it as a GitHub issue.
/// Reports are read by the developers, so they are in English.
pub fn build(what: &str) -> (String, String) {
    let en = Localizer::new(
        &kura_core::locale::Locale::parse("en")
            .into_iter()
            .collect::<Vec<_>>(),
    );
    let mut body = format!("kura {}\n\n", kura_core::VERSION);
    body.push_str(&summary(&en).unwrap_or_else(|e| e));
    let status = kura_alpm::background::read_status(std::path::Path::new(kura_alpm::background::STATUS));
    let _ = writeln!(
        body,
        "\nLast check: {} · updates {} · blocked {} · AUR {} ready, {} waiting",
        status.checked, status.updates, status.blocked, status.aur_updates, status.aur_waiting
    );
    if let Ok(history) = history(&en) {
        let lines: Vec<&str> = history.lines().collect();
        let _ = writeln!(body, "\n{}", lines[lines.len().saturating_sub(8)..].join("\n"));
    }
    let who = identity();
    let text = kura_core::report::anonymize(&body, &who);
    let what = kura_core::report::anonymize(what, &who);
    let url = kura_core::report::issue_url("Problem report", &what, &text, kura_core::VERSION);
    (text, url)
}
