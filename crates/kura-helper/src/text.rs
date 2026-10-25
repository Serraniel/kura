// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Texts for progress events, problems and replies, shared by the command line
//! and the window.

use kura_alpm::apply::{Event, Phase};
use kura_alpm::prepare::Problem;
use kura_i18n::Localizer;

use crate::Reply;

pub fn problem(l: &Localizer, p: &Problem) -> String {
    match p {
        Problem::UnsatisfiedDependency {
            target, dependency, ..
        } => l.msg_args(
            "problem-unsatisfied",
            &[
                ("target", target.clone().into()),
                ("dependency", dependency.clone().into()),
            ],
        ),
        Problem::Conflict { first, second, .. } => l.msg_args(
            "problem-conflict",
            &[("first", first.clone().into()), ("second", second.clone().into())],
        ),
        Problem::InvalidArchitecture { packages } => {
            l.msg_args("problem-arch", &[("packages", packages.join(", ").into())])
        }
        Problem::NotFound { name } => l.msg_args("problem-not-found", &[("name", name.clone().into())]),
        Problem::NotRestorable { name, version } => l.msg_args(
            "problem-not-restorable",
            &[("name", name.clone().into()), ("version", version.clone().into())],
        ),
        Problem::Other { message } => message.clone(),
    }
}

/// One line describing a progress event.
pub fn event(l: &Localizer, event: &Event) -> String {
    let counted = |id: &str, package: &str, current: usize, total: usize| {
        l.msg_args(
            id,
            &[
                ("package", package.to_owned().into()),
                ("current", current.into()),
                ("total", total.into()),
            ],
        )
    };
    match event {
        Event::Refreshing => l.msg("apply-refreshing"),
        Event::Downloading { file, .. } => l.msg_args("apply-downloading", &[("file", file.clone().into())]),
        Event::Progress {
            phase: Phase::Installing,
            package,
            current,
            total,
            ..
        } => counted("apply-installing", package, *current, *total),
        Event::Progress {
            phase: Phase::Removing,
            package,
            current,
            total,
            ..
        } => counted("apply-removing", package, *current, *total),
        Event::Progress { .. } => l.msg("apply-checking"),
        Event::Hook { name, .. } => l.msg_args("apply-hook", &[("name", name.clone().into())]),
        Event::CleaningUp => l.msg("apply-cleanup"),
        Event::UpdatingKeyring => l.msg("apply-keyring"),
    }
}

/// What to tell the user about a reply. `Changed` asks for a new review.
pub fn reply(l: &Localizer, reply: &Reply) -> String {
    match reply {
        Reply::Done => l.msg("apply-done"),
        Reply::Changed { .. } => l.msg("apply-changed"),
        Reply::Blocked { problems } => {
            let mut out = l.msg("check-blocked");
            for p in problems {
                out.push_str("\n  ");
                out.push_str(&problem(l, p));
            }
            out
        }
        Reply::NotAuthorized => l.msg("apply-not-authorized"),
        Reply::Locked => l.msg("apply-locked"),
        Reply::Busy => l.msg("apply-busy"),
        Reply::Failed { message } => l.msg_args("apply-failed", &[("reason", message.clone().into())]),
        Reply::Mirrors { count, backup } => l.msg_args(
            "mirrors-done",
            &[("count", (*count).into()), ("backup", backup.clone().into())],
        ),
    }
}

#[cfg(test)]
mod tests {
    use kura_core::locale::Locale;
    use kura_core::plan::{Plan, PlanKind};

    use super::*;

    fn en() -> Localizer {
        Localizer::new(&[Locale::parse("en").unwrap()])
    }

    #[test]
    fn events() {
        let l = en();
        let installing = Event::Progress {
            phase: Phase::Installing,
            package: "gimp".into(),
            percent: 5,
            current: 1,
            total: 3,
        };
        assert_eq!(event(&l, &installing), "Installing gimp (1/3)");
        let removing = Event::Progress {
            phase: Phase::Removing,
            package: "x".into(),
            percent: 5,
            current: 2,
            total: 2,
        };
        assert_eq!(event(&l, &removing), "Removing x (2/2)");
        let checking = Event::Progress {
            phase: Phase::Verifying,
            package: String::new(),
            percent: 0,
            current: 0,
            total: 0,
        };
        assert_eq!(event(&l, &checking), "Checking the packages…");
        assert_eq!(event(&l, &Event::Refreshing), "Refreshing the package lists…");
        assert_eq!(
            event(
                &l,
                &Event::Downloading {
                    file: "f".into(),
                    done: 1,
                    total: 2
                }
            ),
            "Downloading f"
        );
        assert_eq!(
            event(
                &l,
                &Event::Hook {
                    name: "dracut".into(),
                    current: 1,
                    total: 2
                }
            ),
            "Finishing: dracut"
        );
        assert_eq!(
            event(&l, &Event::CleaningUp),
            "Removing parts that are no longer needed…"
        );
        assert_eq!(
            event(&l, &Event::UpdatingKeyring),
            "Updating the signing keys first…"
        );
    }

    #[test]
    fn replies_and_problems() {
        let l = en();
        assert_eq!(
            reply(&l, &Reply::Done),
            "Done. The change is saved in the history."
        );
        assert!(
            reply(
                &l,
                &Reply::Changed {
                    plan: Plan::new(PlanKind::SystemUpdate)
                }
            )
            .starts_with("Something changed")
        );
        let blocked = Reply::Blocked {
            problems: vec![
                Problem::UnsatisfiedDependency {
                    target: "a".into(),
                    dependency: "b".into(),
                    causing: None,
                },
                Problem::Conflict {
                    first: "c".into(),
                    second: "d".into(),
                    reason: "c".into(),
                },
                Problem::InvalidArchitecture {
                    packages: vec!["e".into(), "f".into()],
                },
                Problem::NotFound { name: "g".into() },
                Problem::NotRestorable {
                    name: "yay-bin".into(),
                    version: "12".into(),
                },
                Problem::Other { message: "h".into() },
            ],
        };
        let text = reply(&l, &blocked);
        assert!(text.contains("a needs b"));
        assert!(text.contains("c and d cannot be installed together"));
        assert!(text.contains("e, f"));
        assert!(text.contains("No package named g"));
        assert!(text.contains("yay-bin 12"));
        assert!(text.ends_with("\n  h"));
        for r in [
            Reply::NotAuthorized,
            Reply::Locked,
            Reply::Busy,
            Reply::Failed { message: "x".into() },
        ] {
            assert_ne!(reply(&l, &r), "");
        }
    }
}
