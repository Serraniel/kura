// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `kura` command line interface.

use std::fmt::Write as _;
use std::io::Write as _;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use std::collections::BTreeSet;

use kura_alpm::apply::Event;
use kura_alpm::prepare::{Outcome, Problem, Request};
use kura_core::package::{InstallReason, PackageSet};
use kura_core::plan::Plan;
use kura_core::plan::{RemovalReason, Step};
use kura_helper::Reply;
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
    let mut names: Vec<String> = Vec::new();
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
            Value(v) => names.push(v.string().map_err(|e| e.to_string())?),
            other => return Err(other.unexpected().to_string()),
        }
    }

    match command.as_deref() {
        Some("version") => Ok(format!("kura {}\n", kura_core::VERSION)),
        Some("check") => check(l),
        Some("security") => security(l),
        Some("mirrors") => {
            eprintln!("{}", l.msg("mirrors-running"));
            let reply = kura_helper::rank_mirrors()
                .map_err(|e| l.msg_args("error-helper", &[("reason", e.to_string().into())]))?;
            let text = kura_helper::text::reply(l, &reply);
            if matches!(reply, Reply::Mirrors { .. }) {
                Ok(text + "\n")
            } else {
                Err(text)
            }
        }
        Some("undo") => undo(l, names.first().map(String::as_str)),
        Some("update") => change(l, &Request::Update, "confirm-update"),
        Some("install") if !names.is_empty() => install(l, names),
        Some("remove") if !names.is_empty() => change(l, &Request::Remove(names), "confirm-remove"),
        Some("history") => history(l),
        Some("doctor") => doctor(l),
        Some("search") if !names.is_empty() => search(l, &names.join(" ")),
        Some("snapshots") => match names.first().map(String::as_str) {
            None => snapshots(l, false),
            Some("setup") => snapshots(l, true),
            Some(other) => Err(l.msg_args("cli-unknown-command", &[("command", other.to_owned().into())])),
        },
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
    kura_helper::text::problem(l, p)
}

/// Prepares a request on the private database copy. For updates, packages
/// nothing needs any more are added as cleanup.
fn prepare(l: &Localizer, request: &Request) -> Result<(Plan, Outcome, BTreeSet<String>), String> {
    let fail = |e: kura_alpm::Error| l.msg_args("error-check", &[("reason", e.to_string().into())]);
    let config = kura_alpm::load_config().map_err(fail)?;
    let refresh = !matches!(request, Request::Remove(_));
    let (outcome, repair) = kura_alpm::prepare::prepare_with_repair(
        &config,
        Some(&check_dir()),
        request,
        refresh,
        &user_locales(),
    )
    .map_err(fail)?;
    if let Some(repair) = &repair {
        say(&format!(
            "{}\n{}\n",
            l.msg_args("solver-found", &[("count", repair.remove.len().into())]),
            l.msg("solver-tested")
        ));
    }
    let mut plan = match &outcome.plan {
        Err(problems) => {
            let id = if matches!(request, Request::Update) {
                "check-blocked"
            } else {
                "change-blocked"
            };
            let mut out = l.msg(id) + "\n";
            for p in problems {
                let _ = writeln!(out, "  {}", problem_text(l, p));
            }
            return Err(out);
        }
        Ok(plan) => plan.clone(),
    };
    if matches!(request, Request::Update) {
        let set = load(l)?;
        let cleanup = kura_alpm::cleanup(&set);
        plan.add_cleanup(cleanup.iter().map(|(n, v)| (n.as_str(), v.as_str())));
    }
    let apps = kura_alpm::apps(&config).map_err(fail)?;
    Ok((plan, outcome, apps))
}

/// The preview of a plan, grouped the way the window groups it.
fn render(l: &Localizer, plan: &Plan, outcome: Option<&Outcome>, apps: &BTreeSet<String>) -> String {
    let is_app = |p: &str| apps.contains(p);
    let groups = plan.groups(&is_app);
    let mut out = String::new();
    let width = plan.steps.iter().map(|s| s.package().len()).max().unwrap_or(0);
    let mut section = |title: &str, steps: &[&Step]| {
        if steps.is_empty() {
            return;
        }
        let _ = writeln!(out, "\n{}", l.msg_args(title, &[("count", steps.len().into())]));
        for step in steps {
            let detail = kura_i18n::present::step_detail(step);
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
    section("check-group-cleanup", &groups.cleanup);
    if !plan.actions.is_empty() {
        let _ = writeln!(out, "\n{}", l.msg("preview-actions"));
        for action in &plan.actions {
            let _ = writeln!(out, "  {}", kura_i18n::present::action(l, action));
        }
    }
    if let Some(outcome) = outcome
        && plan.steps.iter().any(|s| {
            !matches!(
                s,
                Step::Remove {
                    why: RemovalReason::Orphan,
                    ..
                }
            )
        })
    {
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
    out
}

fn headline(l: &Localizer, outcome: &Outcome) -> String {
    if outcome.available.is_empty() {
        l.msg("check-none")
    } else {
        l.msg_args("check-found", &[("count", outcome.available.len().into())])
    }
}

fn check(l: &Localizer) -> Result<String, String> {
    eprintln!("{}", l.msg("check-running"));
    let (plan, outcome, apps) = prepare(l, &Request::Update)?;
    if plan.steps.is_empty() {
        return Ok(l.msg("check-none") + "\n");
    }
    Ok(format!(
        "{}\n{}\n{}\n",
        headline(l, &outcome),
        render(l, &plan, Some(&outcome), &apps),
        l.msg("check-preview-only")
    ))
}

fn say(text: &str) {
    let _ = writeln!(std::io::stdout().lock(), "{text}");
}

/// Asks a yes/no question on the terminal. Only an explicit yes counts.
fn confirm(l: &Localizer, question_id: &str) -> bool {
    let _ = write!(std::io::stdout().lock(), "\n{} ", l.msg(question_id));
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    let answer = answer.trim().to_lowercase();
    !answer.is_empty() && (answer.starts_with(&l.msg("confirm-yes")) || answer.starts_with('y'))
}

/// Prints progress events, each line once. Parallel downloads report the
/// same file many times.
fn progress_printer(l: &Localizer) -> impl FnMut(Event) + '_ {
    let mut seen = BTreeSet::new();
    move |event| {
        let line = kura_helper::text::event(l, &event);
        if seen.insert(line.clone()) {
            say(&line);
        }
    }
}

/// Arch news items that concern this update, or `None` if the feed could not be read.
fn news_for(plan: &Plan) -> Option<Vec<kura_core::news::Item>> {
    let items = kura_alpm::background::fetch_news()?;
    let packages: BTreeSet<String> = plan.steps.iter().map(|s| s.package().to_owned()).collect();
    let since = kura_alpm::background::news_since(
        std::path::Path::new(kura_alpm::history::JOURNAL),
        kura_alpm::background::now(),
    );
    Some(
        kura_core::news::affecting(&items, since, &packages)
            .into_iter()
            .cloned()
            .collect(),
    )
}

fn show_news(l: &Localizer, items: &[kura_core::news::Item]) {
    say(&format!("\n{}", l.msg("news-title")));
    for item in items {
        say(&format!("\n  {}\n  {}", item.title, item.link));
        for line in item.text.lines().take(8) {
            say(&format!("    {line}"));
        }
    }
}

/// Preview, confirm, apply. If the system changed since the preview, the new
/// preview is shown and confirmed again.
fn change(l: &Localizer, request: &Request, question_id: &str) -> Result<String, String> {
    eprintln!("{}", l.msg("check-running"));
    let (plan, outcome, apps) = prepare(l, request)?;
    if plan.steps.is_empty() {
        return Ok(l.msg("check-none") + "\n");
    }
    let title = if matches!(request, Request::Update) {
        headline(l, &outcome)
    } else {
        l.msg("preview-title")
    };
    say(&format!("{title}\n{}", render(l, &plan, Some(&outcome), &apps)));
    let mut question_id = question_id;
    if matches!(request, Request::Update) {
        match news_for(&plan) {
            Some(items) if !items.is_empty() => {
                show_news(l, &items);
                question_id = "confirm-update-news";
            }
            Some(_) => {}
            None => say(&format!("\n{}", l.msg("news-unavailable"))),
        }
    }
    confirm_and_apply(l, plan, &outcome, &apps, question_id)
}

/// Asks, runs the plan through the helper and shows a new preview when the
/// system changed in the meantime.
fn confirm_and_apply(
    l: &Localizer,
    mut plan: Plan,
    outcome: &Outcome,
    apps: &BTreeSet<String>,
    question_id: &str,
) -> Result<String, String> {
    loop {
        if !confirm(l, question_id) {
            return Ok(l.msg("apply-cancelled") + "\n");
        }
        let reply = kura_helper::apply(&plan, progress_printer(l))
            .map_err(|e| l.msg_args("error-helper", &[("reason", e.to_string().into())]))?;
        match reply {
            Reply::Done => return Ok(l.msg("apply-done") + "\n"),
            Reply::Changed { plan: fresh } => {
                plan = fresh;
                say(&format!(
                    "\n{}\n{}",
                    l.msg("apply-changed"),
                    render(l, &plan, Some(outcome), apps)
                ));
            }
            other => return Err(kura_helper::text::reply(l, &other)),
        }
    }
}

/// Installs from the repositories; names they do not know come from the AUR.
fn install(l: &Localizer, names: Vec<String>) -> Result<String, String> {
    let fail = |e: kura_alpm::Error| l.msg_args("error-database", &[("reason", e.to_string().into())]);
    let conf = kura_alpm::load_config().map_err(fail)?;
    let mut aur = Vec::new();
    for name in &names {
        if !kura_alpm::in_repos(&conf, name).map_err(fail)? {
            aur.push(name.clone());
        }
    }
    if aur.is_empty() {
        return change(l, &Request::Install(names), "confirm-install");
    }
    if aur.len() != names.len() {
        return Err(l.msg_args("aur-mixed", &[("names", aur.join(" ").into())]) + "\n");
    }
    eprintln!("{}", l.msg("check-running"));
    let check = |e: kura_alpm::Error| l.msg_args("error-check", &[("reason", e.to_string().into())]);
    let outcome = kura_alpm::aur::prepare_install(&conf, Some(&check_dir()), &aur, true).map_err(check)?;
    let plan = match &outcome.plan {
        Ok(plan) => plan.clone(),
        Err(problems) => {
            let mut out = l.msg("change-blocked") + "\n";
            for p in problems {
                let _ = writeln!(out, "  {}", problem_text(l, p));
            }
            return Err(out);
        }
    };
    let apps = kura_alpm::apps(&conf).map_err(fail)?;
    let now = kura_alpm::background::now();
    let mut text = format!(
        "{}\n{}",
        l.msg("preview-title"),
        render(l, &plan, Some(&outcome), &apps)
    );
    let _ = writeln!(text, "\n{}", l.msg("aur-build-list"));
    let infos = kura_alpm::aur::info(&aur).unwrap_or_default();
    for b in &plan.builds {
        let signals = infos
            .iter()
            .find(|i| i.package_base == b.base)
            .map(|i| format!("  ({})", kura_i18n::present::aur_signals(l, i, now)))
            .unwrap_or_default();
        let _ = writeln!(text, "  {} {}{signals}", b.base, b.version);
    }
    let _ = writeln!(text, "\n{}", l.msg("aur-notice"));
    say(&text);
    confirm_and_apply(l, plan, &outcome, &apps, "confirm-aur")
}

/// Searches the repositories, then the AUR.
fn search(l: &Localizer, term: &str) -> Result<String, String> {
    let fail = |e: kura_alpm::Error| l.msg_args("error-database", &[("reason", e.to_string().into())]);
    let conf = kura_alpm::load_config().map_err(fail)?;
    let repo = kura_alpm::system::search(&conf, term, 20).map_err(fail)?;
    let aur = kura_alpm::aur::search(term, "name-desc");
    let mut out = String::new();
    if !repo.is_empty() {
        let _ = writeln!(out, "{}", l.msg("search-repos"));
        for h in &repo {
            let mark = if h.installed {
                format!("  [{}]", l.msg("gui-installed"))
            } else {
                String::new()
            };
            let _ = writeln!(out, "  {} {}{mark}\n      {}", h.name, h.version, h.description);
        }
    }
    match aur {
        Ok(mut hits) if !hits.is_empty() => {
            hits.sort_by(|a, b| b.popularity.total_cmp(&a.popularity));
            hits.truncate(20);
            let now = kura_alpm::background::now();
            let gap = if out.is_empty() { "" } else { "\n" };
            let _ = writeln!(out, "{gap}{}", l.msg("search-aur"));
            for h in &hits {
                let _ = writeln!(
                    out,
                    "  {} {}  ({})\n      {}",
                    h.name,
                    h.version,
                    kura_i18n::present::aur_signals(l, h, now),
                    h.description.as_deref().unwrap_or_default()
                );
            }
        }
        Ok(_) => {}
        Err(reason) => {
            let _ = writeln!(
                out,
                "\n{}",
                l.msg_args("search-aur-failed", &[("reason", reason.into())])
            );
        }
    }
    if out.is_empty() {
        out = l.msg("search-none") + "\n";
    }
    Ok(out)
}

/// The snapshot state; with `setup`, a preview of what kura adds.
fn snapshots(l: &Localizer, setup: bool) -> Result<String, String> {
    let fail = |e: kura_alpm::Error| l.msg_args("error-database", &[("reason", e.to_string().into())]);
    let conf = kura_alpm::load_config().map_err(fail)?;
    let set = load(l)?;
    let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
    let report = kura_alpm::system::snapshot_report(&conf, &set, &mounts);
    let (state, missing) = kura_i18n::present::snapshots(l, &report);
    let mut out = format!("{}: {state}\n", l.msg("health-snapshots"));
    if !missing.is_empty() {
        let _ = writeln!(out, "\n{}", l.msg("snapshots-missing"));
        for step in &missing {
            let _ = writeln!(out, "  {step}");
        }
    }
    if !setup {
        if !missing.is_empty() {
            let _ = writeln!(out, "\n{}", l.msg("snapshots-setup-hint"));
        }
        return Ok(out);
    }
    say(&out);
    eprintln!("{}", l.msg("check-running"));
    let Some(outcome) = kura_alpm::snapshots::prepare(&conf, Some(&check_dir()), true).map_err(fail)? else {
        return Ok(l.msg("snapshots-nothing") + "\n");
    };
    let plan = match &outcome.plan {
        Ok(plan) => plan.clone(),
        Err(problems) => {
            let mut out = l.msg("change-blocked") + "\n";
            for p in problems {
                let _ = writeln!(out, "  {}", problem_text(l, p));
            }
            return Err(out);
        }
    };
    let apps = kura_alpm::apps(&conf).map_err(fail)?;
    say(&format!(
        "{}\n{}",
        l.msg("preview-title"),
        render(l, &plan, Some(&outcome), &apps)
    ));
    confirm_and_apply(l, plan, &outcome, &apps, "confirm-snapshots")
}

/// Known vulnerabilities, named by the apps that use the affected package.
fn security(l: &Localizer) -> Result<String, String> {
    eprintln!("{}", l.msg("security-running"));
    let advisories =
        kura_alpm::background::fetch_advisories().ok_or_else(|| l.msg("security-unavailable"))?;
    let set = load(l)?;
    let findings = kura_core::security::scan(&advisories, &set);
    let mut out = kura_i18n::present::security_headline(l, &findings) + "\n";
    for f in &findings {
        let _ = writeln!(
            out,
            "\n  {} ({})\n    {}",
            if f.apps.is_empty() {
                l.msg("security-system")
            } else {
                f.apps.join(", ")
            },
            kura_i18n::present::severity(l, f.severity),
            l.msg_args(
                "security-detail",
                &[
                    ("package", f.package.clone().into()),
                    ("installed", f.installed.clone().into()),
                    ("kind", f.kind.clone().into()),
                ]
            ),
        );
        match &f.fixed {
            Some(v) => {
                let _ = writeln!(
                    out,
                    "    {} · {}",
                    l.msg_args("security-fixed-in", &[("version", v.clone().into())]),
                    l.msg("security-fix")
                );
            }
            None => {
                let _ = writeln!(out, "    {}", l.msg("security-nofix"));
            }
        }
        let _ = writeln!(out, "    {} {}", f.advisory, f.cves.join(" "));
    }
    Ok(out)
}

/// Goes back to an earlier generation, by default the one before the newest.
fn undo(l: &Localizer, arg: Option<&str>) -> Result<String, String> {
    let fail = |e: kura_alpm::Error| l.msg_args("error-database", &[("reason", e.to_string().into())]);
    let journal =
        kura_alpm::history::load(std::path::Path::new(kura_alpm::history::JOURNAL)).map_err(fail)?;
    let latest = journal.latest().id;
    let target = match arg {
        Some(a) => a
            .trim_start_matches('#')
            .parse::<u64>()
            .map_err(|_| l.msg("cli-usage"))?,
        None => latest.saturating_sub(1),
    };
    let unknown = || l.msg_args("undo-unknown", &[("id", target.into())]);
    if target == latest && latest > 0 {
        return Ok(l.msg("undo-nothing") + "\n");
    }
    let plan = kura_alpm::restore::plan(&journal, target).ok_or_else(unknown)?;
    if plan.steps.is_empty() {
        return Ok(l.msg("undo-nothing") + "\n");
    }
    let config = kura_alpm::load_config().map_err(fail)?;
    let generation = journal.generation(target).ok_or_else(unknown)?;
    if let Err(problems) = kura_alpm::restore::sources(&config.cache_dir, &plan, &generation) {
        let mut out = l.msg("change-blocked") + "\n";
        for p in &problems {
            let _ = writeln!(out, "  {}", problem_text(l, p));
        }
        return Err(out);
    }
    let apps = kura_alpm::apps(&config).map_err(fail)?;
    let when = kura_i18n::present::when(l, generation.created, kura_alpm::background::now());
    say(&format!(
        "{}\n{}\n{}",
        l.msg_args("undo-title", &[("id", target.into()), ("when", when.into())]),
        render(l, &plan, None, &apps),
        l.msg("undo-limits")
    ));
    if !confirm(l, "confirm-undo") {
        return Ok(l.msg("apply-cancelled") + "\n");
    }
    let reply = kura_helper::apply(&plan, progress_printer(l))
        .map_err(|e| l.msg_args("error-helper", &[("reason", e.to_string().into())]))?;
    let text = kura_helper::text::reply(l, &reply);
    if reply == Reply::Done {
        Ok(text + "\n")
    } else {
        Err(text)
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

fn doctor(l: &Localizer) -> Result<String, String> {
    let set = load(l)?;
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
            tool_version: kura_core::VERSION,
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

/// The user's languages, most preferred first.
fn user_locales() -> Vec<kura_core::locale::Locale> {
    kura_core::locale::Locale::from_env_vars(|k| std::env::var(k).ok())
}
