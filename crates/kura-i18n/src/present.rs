// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Texts built from kura's data, shared by the command line and the window so
//! both say the same thing.

use kura_core::action::Action;
use kura_core::aur;
use kura_core::history::Record;
use kura_core::plan::Step;
use kura_core::security::{Finding, Severity};
use kura_core::snapshots::{self, Report, Tool};

use crate::Localizer;

/// "today", "yesterday" or "3 days ago".
pub fn when(l: &Localizer, created: i64, now: i64) -> String {
    match (now - created).max(0) / 86_400 {
        0 => l.msg("history-today"),
        1 => l.msg("history-yesterday"),
        days => l.msg_args("history-days-ago", &[("days", days.into())]),
    }
}

/// What a history record changed, for example "46 updated, 1 removed".
pub fn record_text(l: &Localizer, record: &Record) -> String {
    let s = record.summary();
    if record.id == 1 {
        return l.msg_args("history-start", &[("count", s.installed.into())]);
    }
    let parts: Vec<String> = [
        ("history-upgraded", s.upgraded),
        ("history-installed", s.installed),
        ("history-downgraded", s.downgraded),
        ("history-removed", s.removed),
    ]
    .into_iter()
    .filter(|(_, n)| *n > 0)
    .map(|(id, n)| l.msg_args(id, &[("count", n.into())]))
    .collect();
    let mut text = parts.join(&l.msg("list-separator"));
    if record.cause.as_deref() == Some("pacman") {
        text = format!("{text} {}", l.msg("history-outside"));
    }
    text
}

/// Version change of a step: "151.0 → 152.0", "+ 3.0.6", "− 36.9.0".
pub fn step_detail(step: &Step) -> String {
    match step {
        Step::Upgrade { from, to, .. } | Step::Downgrade { from, to, .. } => format!("{from} → {to}"),
        Step::Install { version, .. } => format!("+ {version}"),
        Step::Remove { version, .. } => format!("− {version}"),
        Step::MarkReason { .. } => String::new(),
    }
}

/// One sentence naming what is affected, by the apps the user knows.
pub fn security_headline(l: &Localizer, findings: &[Finding]) -> String {
    let apps = kura_core::security::affected_apps(findings);
    let mut apps = apps.into_iter();
    match (apps.next(), apps.count()) {
        (None, _) if findings.is_empty() => l.msg("security-none"),
        (None, _) => l.msg("security-system"),
        (Some(app), 0) => l.msg_args("security-one-app", &[("app", app.to_owned().into())]),
        (Some(app), more) => l.msg_args(
            "security-more-apps",
            &[("app", app.to_owned().into()), ("count", more.into())],
        ),
    }
}

pub fn severity(l: &Localizer, s: Severity) -> String {
    l.msg(match s {
        Severity::Low => "severity-low",
        Severity::Medium => "severity-medium",
        Severity::High => "severity-high",
        Severity::Critical => "severity-critical",
    })
}

/// Trust signals for an AUR package, joined for one line.
pub fn aur_signals(l: &Localizer, info: &aur::Info, now: i64) -> String {
    let mut parts = vec![l.msg_args("aur-votes", &[("count", info.num_votes.into())])];
    for w in aur::warnings(info, now) {
        parts.push(match w {
            aur::Warning::New => l.msg("aur-new"),
            aur::Warning::Orphaned => l.msg("aur-orphaned"),
            aur::Warning::OutOfDate { .. } => l.msg("aur-out-of-date"),
        });
    }
    parts.join(" · ")
}

/// One line for a change that is not a package change.
pub fn action(l: &Localizer, action: &Action) -> String {
    match action {
        Action::Enable { unit } => l.msg_args("snapshots-step-enable", &[("unit", unit.clone().into())]),
        Action::SnapperRootConfig { .. } => l.msg("snapshots-step-config"),
    }
}

fn tool_name(tool: Tool) -> &'static str {
    match tool {
        Tool::Snapper => "Snapper",
        Tool::Timeshift => "Timeshift",
    }
}

/// The state of snapshots in one line, and what is missing.
pub fn snapshots(l: &Localizer, report: &Report) -> (String, Vec<String>) {
    match report {
        Report::Unsupported => (l.msg("snapshots-unsupported"), Vec::new()),
        Report::OtherTool(tool) => (
            l.msg_args("snapshots-on", &[("tool", tool_name(*tool).into())]),
            Vec::new(),
        ),
        Report::Complete => (
            l.msg_args("snapshots-on", &[("tool", tool_name(Tool::Snapper).into())]),
            Vec::new(),
        ),
        Report::Missing(steps) => {
            let fresh = steps.contains(&snapshots::Step::Install("snapper".into()));
            let lines = steps
                .iter()
                .map(|s| match s {
                    snapshots::Step::Install(p) => {
                        l.msg_args("snapshots-step-install", &[("package", p.clone().into())])
                    }
                    snapshots::Step::CreateRootConfig { .. } => l.msg("snapshots-step-config"),
                    snapshots::Step::Enable(u) => {
                        l.msg_args("snapshots-step-enable", &[("unit", u.clone().into())])
                    }
                })
                .collect();
            (
                l.msg(if fresh {
                    "snapshots-off"
                } else {
                    "snapshots-partial"
                }),
                lines,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use kura_core::history::{Record, RecordChange};
    use kura_core::locale::Locale;
    use kura_core::package::InstallReason;
    use kura_core::plan::RemovalReason;

    use super::*;

    fn l(lang: &str) -> Localizer {
        Localizer::new(&[Locale::parse(lang).unwrap()])
    }

    fn set(name: &str, from: Option<&str>) -> RecordChange {
        RecordChange::Set {
            name: name.into(),
            version: "2".into(),
            repo: None,
            reason: InstallReason::Explicit,
            from: from.map(str::to_owned),
        }
    }

    #[test]
    fn relative_days() {
        let en = l("en");
        assert_eq!(when(&en, 1000, 1000 + 3600), "today");
        assert_eq!(when(&en, 0, 86_400 + 5), "yesterday");
        assert_eq!(when(&en, 0, 3 * 86_400), "3 days ago");
        assert_eq!(when(&en, 5000, 0), "today");
        assert_eq!(when(&l("de"), 0, 3 * 86_400), "vor 3 Tagen");
    }

    #[test]
    fn record_texts() {
        let en = l("en");
        let start = Record {
            id: 1,
            created: 0,
            cause: None,
            changes: vec![set("a", None), set("b", None)],
        };
        assert_eq!(record_text(&en, &start), "Starting point: 2 packages");
        let update = Record {
            id: 2,
            created: 0,
            cause: Some("kura:system-update".into()),
            changes: vec![
                set("a", Some("1")),
                RecordChange::Unset {
                    name: "b".into(),
                    version: "2".into(),
                },
            ],
        };
        assert_eq!(record_text(&en, &update), "1 updated, 1 removed");
        let outside = Record {
            cause: Some("pacman".into()),
            ..update.clone()
        };
        assert_eq!(
            record_text(&en, &outside),
            "1 updated, 1 removed (changed outside kura)"
        );
        assert_eq!(record_text(&l("ja"), &update), "更新 1 件、削除 1 件");
    }

    fn finding(apps: &[&str], fixed: bool) -> Finding {
        Finding {
            advisory: "AVG-1".into(),
            package: "ffmpeg".into(),
            installed: "1".into(),
            fixed: fixed.then(|| "2".into()),
            severity: Severity::High,
            kind: "arbitrary code execution".into(),
            cves: vec![],
            apps: apps.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    #[test]
    fn snapshot_states() {
        use kura_core::snapshots::Step;
        let en = l("en");
        assert_eq!(
            action(&en, &Action::SnapperRootConfig { keep_mount: true }),
            "Create a snapper configuration for /"
        );
        assert_eq!(
            snapshots(&en, &Report::Unsupported).0,
            "Not possible on this file system"
        );
        assert_eq!(
            snapshots(&en, &Report::Complete).0,
            "Taken before every update by Snapper"
        );
        assert_eq!(
            snapshots(&en, &Report::OtherTool(Tool::Timeshift)).0,
            "Taken before every update by Timeshift"
        );
        let fresh = Report::Missing(vec![
            Step::Install("snapper".into()),
            Step::CreateRootConfig { keep_mount: false },
            Step::Enable("snapper-cleanup.timer".into()),
        ]);
        let (state, missing) = snapshots(&en, &fresh);
        assert_eq!(state, "Not set up");
        assert_eq!(
            missing,
            [
                "Install snapper",
                "Create a snapper configuration for /",
                "Turn on snapper-cleanup.timer"
            ]
        );
        let partial = Report::Missing(vec![Step::Install("snap-pac".into())]);
        assert_eq!(snapshots(&en, &partial).0, "Partly set up");
    }

    #[test]
    fn aur_signal_lines() {
        let mut info = aur::Info {
            name: "x".into(),
            num_votes: 12,
            first_submitted: 0,
            maintainer: Some("m".into()),
            ..aur::Info::default()
        };
        let now = 100 * 86_400;
        assert_eq!(aur_signals(&l("en"), &info, now), "12 votes");
        info.maintainer = None;
        info.out_of_date = Some(1);
        info.num_votes = 1;
        info.first_submitted = now;
        assert_eq!(
            aur_signals(&l("en"), &info, now),
            "1 vote · new · no maintainer · marked out of date"
        );
    }

    #[test]
    fn security_headlines() {
        let en = l("en");
        assert_eq!(security_headline(&en, &[]), "No known security problems.");
        assert_eq!(
            security_headline(&en, &[finding(&[], true)]),
            "The system has a known security problem"
        );
        assert_eq!(
            security_headline(&en, &[finding(&["kdenlive"], true)]),
            "kdenlive has a known security problem"
        );
        assert_eq!(
            security_headline(&en, &[finding(&["kdenlive", "vlc"], true)]),
            "kdenlive and one other app have a known security problem"
        );
        assert_eq!(
            security_headline(&en, &[finding(&["a", "b", "c"], true)]),
            "a and 2 other apps have a known security problem"
        );
        assert_eq!(severity(&en, Severity::Critical), "critical risk");
        assert_eq!(severity(&en, Severity::Low), "low risk");
        assert_eq!(severity(&l("de"), Severity::Medium), "mittleres Risiko");
        assert_eq!(severity(&en, Severity::High), "high risk");
    }

    #[test]
    fn step_details() {
        let up = Step::Upgrade {
            name: "f".into(),
            from: "1".into(),
            to: "2".into(),
        };
        assert_eq!(step_detail(&up), "1 → 2");
        let down = Step::Downgrade {
            name: "f".into(),
            from: "2".into(),
            to: "1".into(),
        };
        assert_eq!(step_detail(&down), "2 → 1");
        let add = Step::Install {
            name: "g".into(),
            version: "3".into(),
            reason: InstallReason::Explicit,
        };
        assert_eq!(step_detail(&add), "+ 3");
        let rm = Step::Remove {
            name: "e".into(),
            version: "36".into(),
            why: RemovalReason::Orphan,
        };
        assert_eq!(step_detail(&rm), "− 36");
        let mark = Step::MarkReason {
            name: "m".into(),
            reason: InstallReason::Explicit,
        };
        assert_eq!(step_detail(&mark), "");
    }
}
