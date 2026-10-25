// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `kura` command line interface.

use std::fmt::Write as _;
use std::io::Write as _;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use kura_alpm::prepare::{Problem, Request};
use kura_core::package::{InstallReason, PackageSet};
use kura_core::plan::{RemovalReason, Step};
use kura_i18n::Localizer;

fn main() -> ExitCode {
    let l = Localizer::from_env();
    match run(&l) {
        Ok(text) => {
            // A closed pipe (`kura sbom | head`) is not an error worth reporting.
            let _ = std::io::stdout().lock().write_all(text.as_bytes());
            ExitCode::SUCCESS
        }
        Err(msg) => {
            eprintln!("{msg}");
            ExitCode::FAILURE
        }
    }
}

fn run(l: &Localizer) -> Result<String, String> {
    use lexopt::prelude::*;

    let mut parser = lexopt::Parser::from_env();
    let mut command: Option<String> = None;
    let mut output: Option<String> = None;
    while let Some(arg) = parser.next().map_err(|e| e.to_string())? {
        match arg {
            Short('h') | Long("help") => return Ok(l.msg("cli-usage") + "\n"),
            Short('o') | Long("output") => {
                output = Some(
                    parser
                        .value()
                        .map_err(|e| e.to_string())?
                        .string()
                        .map_err(|e| e.to_string())?,
                );
            }
            Value(v) if command.is_none() => command = Some(v.string().map_err(|e| e.to_string())?),
            other => return Err(other.unexpected().to_string()),
        }
    }

    match command.as_deref() {
        Some("version") => Ok(format!("kura {}\n", env!("CARGO_PKG_VERSION"))),
        Some("check") => check(l),
        Some("history") => history(l),
        Some("doctor") => doctor(l),
        Some("orphans") => orphans(l),
        Some("sbom") => sbom(l, output.as_deref()),
        Some(other) => Err(format!(
            "{}\n\n{}",
            l.msg_args("cli-unknown-command", &[("command", other.to_owned().into())]),
            l.msg("cli-usage")
        )),
        None => Err(format!(
            "{}\n\n{}",
            l.msg("cli-missing-command"),
            l.msg("cli-usage")
        )),
    }
}

fn load(l: &Localizer) -> Result<PackageSet, String> {
    kura_alpm::load_config()
        .and_then(|conf| kura_alpm::installed(&conf))
        .map_err(|e| l.msg_args("error-database", &[("reason", e.to_string().into())]))
}

/// `NAME` and `ID` from /etc/os-release.
fn os_release() -> (String, String) {
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

/// Private copy of the sync databases, so checks never touch pacman's own.
fn check_dir() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir);
    base.join("kura").join("checkdb")
}

fn problem_text(l: &Localizer, p: &Problem) -> String {
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
        Problem::InvalidArchitecture(pkgs) => {
            l.msg_args("problem-arch", &[("packages", pkgs.join(", ").into())])
        }
        Problem::NotFound(name) => l.msg_args("problem-not-found", &[("name", name.clone().into())]),
        Problem::Other(msg) => msg.clone(),
    }
}

fn check(l: &Localizer) -> Result<String, String> {
    eprintln!("{}", l.msg("check-running"));
    let fail = |e: kura_alpm::Error| l.msg_args("error-check", &[("reason", e.to_string().into())]);
    let config = kura_alpm::load_config().map_err(fail)?;
    let outcome =
        kura_alpm::prepare::prepare(&config, Some(&check_dir()), &Request::Update, true).map_err(fail)?;

    let plan = match outcome.plan {
        Err(problems) => {
            let mut out = l.msg("check-blocked") + "\n";
            for p in &problems {
                let _ = writeln!(out, "  {}", problem_text(l, p));
            }
            return Err(out);
        }
        Ok(plan) => plan,
    };
    let set = load(l)?;
    let cleanup = kura_alpm::cleanup(&set);
    if plan.steps.is_empty() && cleanup.is_empty() {
        return Ok(l.msg("check-none") + "\n");
    }
    let apps = kura_alpm::apps(&config).map_err(fail)?;
    let is_app = |p: &str| apps.contains(p);
    let groups = plan.groups(&is_app);

    let mut out = if outcome.available.is_empty() {
        l.msg("check-none")
    } else {
        l.msg_args("check-found", &[("count", outcome.available.len().into())])
    } + "\n";
    let width = plan
        .steps
        .iter()
        .map(|s| s.package().len())
        .chain(cleanup.iter().map(|(n, _)| n.len()))
        .max()
        .unwrap_or(0);
    let mut section = |title: &str, steps: &[&Step]| {
        if steps.is_empty() {
            return;
        }
        let _ = writeln!(out, "\n{}", l.msg_args(title, &[("count", steps.len().into())]));
        for step in steps {
            let detail = match step {
                Step::Upgrade { from, to, .. } | Step::Downgrade { from, to, .. } => format!("{from} → {to}"),
                Step::Install { version, .. } => format!("+ {version}"),
                Step::Remove { version, .. } => format!("− {version}"),
                Step::MarkReason { .. } => String::new(),
            };
            let _ = writeln!(out, "  {:width$}  {detail}", step.package());
        }
    };
    let (replaced, system): (Vec<&Step>, Vec<&Step>) = groups.system.iter().partition(|s| {
        matches!(
            s,
            Step::Remove {
                why: RemovalReason::Replaced,
                ..
            }
        )
    });
    section("check-group-apps", &groups.apps);
    section("check-group-system", &system);
    section("check-group-replaced", &replaced);
    if !cleanup.is_empty() {
        let _ = writeln!(
            out,
            "\n{}",
            l.msg_args("check-group-cleanup", &[("count", cleanup.len().into())])
        );
        for (name, version) in &cleanup {
            let _ = writeln!(out, "  {name:width$}  − {version}");
        }
    }
    if !plan.steps.is_empty() {
        let _ = writeln!(
            out,
            "\n{}",
            l.msg_args(
                "check-sizes",
                &[
                    ("download", l.size(outcome.download_size).into()),
                    ("delta", l.size_delta(outcome.installed_delta).into()),
                ]
            )
        );
    }
    let _ = writeln!(out, "\n{}", l.msg("check-preview-only"));
    Ok(out)
}

fn when(l: &Localizer, created: i64, now: i64) -> String {
    match (now - created).max(0) / 86_400 {
        0 => l.msg("history-today"),
        1 => l.msg("history-yesterday"),
        days => l.msg_args("history-days-ago", &[("days", days.into())]),
    }
}

fn history(l: &Localizer) -> Result<String, String> {
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
            let s = r.summary();
            let text = if r.id == 1 {
                l.msg_args("history-start", &[("count", s.installed.into())])
            } else {
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
                if r.cause.as_deref() == Some("pacman") {
                    text = format!("{text} {}", l.msg("history-outside"));
                }
                text
            };
            (format!("#{}", r.id), when(l, r.created, now), text)
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

fn doctor(l: &Localizer) -> Result<String, String> {
    let set = load(l)?;
    let (name, _) = os_release();
    let explicit = set.iter().filter(|p| p.reason == InstallReason::Explicit).count();
    let foreign = set.iter().filter(|p| p.repo.is_none()).count();
    let rows = [
        (l.msg("doctor-distro"), name),
        (l.msg("doctor-packages"), set.len().to_string()),
        (l.msg("doctor-explicit"), explicit.to_string()),
        (l.msg("doctor-foreign"), foreign.to_string()),
        (l.msg("doctor-orphans"), set.orphans().len().to_string()),
        (l.msg("doctor-language"), l.language()),
    ];
    let width = rows.iter().map(|(k, _)| k.chars().count()).max().unwrap_or(0);
    let mut out = format!("{}\n\n", l.msg("doctor-title"));
    for (k, v) in rows {
        let pad = width - k.chars().count();
        let _ = writeln!(out, "  {k}{}  {v}", " ".repeat(pad));
    }
    Ok(out)
}

fn orphans(l: &Localizer) -> Result<String, String> {
    let set = load(l)?;
    let orphans = set.orphans();
    if orphans.is_empty() {
        return Ok(l.msg("orphans-none") + "\n");
    }
    let mut out = l.msg_args("orphans-found", &[("count", orphans.len().into())]) + "\n";
    for name in &orphans {
        let version = set.get(name).map_or("", |p| p.version.as_str());
        let _ = writeln!(out, "  {name} {version}");
    }
    let _ = writeln!(out, "\n{}", l.msg("orphans-hint"));
    Ok(out)
}

fn sbom(l: &Localizer, output: Option<&str>) -> Result<String, String> {
    let set = load(l)?;
    let (name, id) = os_release();
    let uuid = std::fs::read_to_string("/proc/sys/kernel/random/uuid").unwrap_or_default();
    let serial = format!("urn:uuid:{}", uuid.trim());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let timestamp = kura_sbom::rfc3339(now);
    let bom = kura_sbom::build(
        &set,
        &kura_sbom::System {
            distro_id: &id,
            distro_name: &name,
        },
        &kura_sbom::RunInfo {
            serial: &serial,
            timestamp: &timestamp,
            tool_version: env!("CARGO_PKG_VERSION"),
        },
    );
    let text = serde_json::to_string_pretty(&bom).map_err(|e| e.to_string())?;
    match output {
        None | Some("-") => Ok(text + "\n"),
        Some(path) => {
            std::fs::write(path, text + "\n").map_err(|e| {
                l.msg_args(
                    "error-write",
                    &[("path", path.to_owned().into()), ("reason", e.to_string().into())],
                )
            })?;
            Ok(l.msg_args("sbom-written", &[("path", path.to_owned().into())]) + "\n")
        }
    }
}
