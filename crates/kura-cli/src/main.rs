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
use kura_core::package::PackageSet;
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
        Some("update") => update(l),
        Some("settings") => settings(l, &names),
        Some("report") => Ok(report(l, &names.join(" "))),
        Some("profiles") => profiles(l, &names),
        Some("install") if !names.is_empty() => install(l, names),
        Some("remove") if !names.is_empty() => change(l, &Request::Remove(names), "confirm-remove"),
        Some("history") => kura_helper::report::history(l),
        Some("doctor") => kura_helper::report::summary(l),
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

/// Private copy of the sync databases, so checks never touch pacman's own.
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
        Some(&kura_alpm::background::check_dir()),
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
    let mut out = if plan.steps.is_empty() {
        l.msg("check-none") + "\n"
    } else {
        format!(
            "{}\n{}\n{}\n",
            headline(l, &outcome),
            render(l, &plan, Some(&outcome), &apps),
            l.msg("check-preview-only")
        )
    };
    let settings = kura_alpm::aur::settings();
    if settings.aur
        && let Ok(conf) = kura_alpm::load_config()
    {
        match kura_alpm::aur::updates(&conf, &settings, kura_alpm::background::now(), false) {
            Ok(updates) if !updates.is_empty() => {
                let ready = updates.iter().filter(|u| u.waits_until.is_none()).count();
                let _ = writeln!(
                    out,
                    "\n{}",
                    l.msg_args(
                        "check-aur",
                        &[
                            ("ready", ready.into()),
                            ("waiting", (updates.len() - ready).into())
                        ]
                    )
                );
                for u in &updates {
                    let _ = writeln!(out, "  {}  {} → {}", u.name, u.from, u.to);
                }
            }
            Ok(_) => {}
            Err(reason) => {
                let _ = writeln!(
                    out,
                    "\n{}",
                    l.msg_args("aur-check-failed", &[("reason", reason.into())])
                );
            }
        }
        if let Ok(broken) = kura_alpm::aur::needs_rebuild(&conf)
            && !broken.is_empty()
        {
            let _ = writeln!(out, "\n{}", l.msg("check-rebuild"));
            for (name, libs) in &broken {
                let _ = writeln!(out, "  {name}  ({})", libs.join(", "));
            }
        }
    }
    Ok(out)
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
    change_with(l, request, question_id, "preview-title")
}

fn change_with(
    l: &Localizer,
    request: &Request,
    question_id: &str,
    title_id: &str,
) -> Result<String, String> {
    eprintln!("{}", l.msg("check-running"));
    let (plan, outcome, apps) = prepare(l, request)?;
    if plan.steps.is_empty() {
        return Ok(l.msg("check-none") + "\n");
    }
    let title = if matches!(request, Request::Update) {
        headline(l, &outcome)
    } else {
        l.msg(title_id)
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
    if !kura_alpm::aur::settings().aur {
        return Err(l.msg_args("aur-off", &[("names", aur.join(" ").into())]) + "\n");
    }
    aur_change(l, &conf, &aur, true, "preview-title", "confirm-aur")
}

/// Previews building `names` from the AUR, asks and runs it.
fn aur_change(
    l: &Localizer,
    conf: &kura_alpm::Config,
    names: &[String],
    refresh: bool,
    title_id: &str,
    question_id: &str,
) -> Result<String, String> {
    let fail = |e: kura_alpm::Error| l.msg_args("error-database", &[("reason", e.to_string().into())]);
    eprintln!("{}", l.msg("check-running"));
    let check = |e: kura_alpm::Error| l.msg_args("error-check", &[("reason", e.to_string().into())]);
    let outcome =
        kura_alpm::aur::prepare_install(conf, Some(&kura_alpm::background::check_dir()), names, refresh)
            .map_err(check)?;
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
    let apps = kura_alpm::apps(conf).map_err(fail)?;
    let now = kura_alpm::background::now();
    let mut text = format!("{}\n{}", l.msg(title_id), render(l, &plan, Some(&outcome), &apps));
    let _ = writeln!(text, "\n{}", l.msg("aur-build-list"));
    let infos = kura_alpm::aur::info(names).unwrap_or_default();
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
    confirm_and_apply(l, plan, &outcome, &apps, question_id)
}

/// AUR updates: the ready ones are offered, the others are named with the
/// reason they wait.
fn aur_updates(l: &Localizer) -> Result<String, String> {
    let settings = kura_alpm::aur::settings();
    if !settings.aur {
        return Ok(String::new());
    }
    let fail = |e: kura_alpm::Error| l.msg_args("error-database", &[("reason", e.to_string().into())]);
    let conf = kura_alpm::load_config().map_err(fail)?;
    let updates = kura_alpm::aur::updates(&conf, &settings, kura_alpm::background::now(), false)
        .map_err(|reason| l.msg_args("aur-check-failed", &[("reason", reason.into())]))?;
    let (ready, waiting): (Vec<_>, Vec<_>) = updates.iter().partition(|u| u.waits_until.is_none());
    // Packages broken by a library update are built again right away.
    let rebuild = kura_alpm::aur::rebuild_targets(&conf).unwrap_or_default();
    let mut note = String::new();
    if !waiting.is_empty() {
        let names: Vec<&str> = waiting.iter().map(|u| u.name.as_str()).collect();
        note = l.msg_args(
            "aur-waiting",
            &[
                ("count", waiting.len().into()),
                ("days", settings.aur_cooling_days.into()),
                ("names", names.join(", ").into()),
            ],
        ) + "\n";
    }
    if ready.is_empty() && rebuild.is_empty() {
        return Ok(note);
    }
    say(&note);
    let mut names: Vec<String> = ready.iter().map(|u| u.name.clone()).collect();
    for name in rebuild {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    aur_change(l, &conf, &names, false, "aur-updates-title", "confirm-aur-update")
}

/// The repository update, then AUR updates that are ready.
fn update(l: &Localizer) -> Result<String, String> {
    let repo = change(l, &Request::Update, "confirm-update")?;
    say(repo.trim_end());
    aur_updates(l)
}

/// Shows an anonymised problem report and, on request, opens a prefilled
/// GitHub issue in the browser. kura itself sends nothing.
fn report(l: &Localizer, what: &str) -> String {
    let (text, url) = kura_helper::report::build(what);
    say(&format!("{}\n\n{text}", l.msg("report-intro")));
    if !confirm(l, "confirm-report") {
        return String::new();
    }
    let _ = std::process::Command::new("xdg-open")
        .arg(&url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
    l.msg_args("report-opened", &[("url", url.into())]) + "\n"
}

/// The built-in repository profiles and whether they are added.
fn profiles_list(l: &Localizer) -> String {
    let mut out = l.msg("profiles-title") + "\n";
    for p in kura_core::profiles::ALL {
        let state = if kura_alpm::profiles::is_active(p) {
            l.msg("profile-on")
        } else {
            l.msg("profile-off")
        };
        let _ = writeln!(
            out,
            "\n  {} ({state})\n  {}",
            p.id,
            l.msg(&format!("profile-{}-text", p.id))
        );
    }
    out
}

/// Switches every package now after choosing "gradually" before.
fn profiles_switch(l: &Localizer, id: &str) -> Result<String, String> {
    let profile = kura_core::profiles::find(id).ok_or_else(|| l.msg("profiles-usage") + "\n")?;
    let level = kura_alpm::profiles::active_level(profile)
        .filter(|l| l.architecture.is_some())
        .ok_or_else(|| l.msg_args("profile-missing", &[("name", profile.id.into())]) + "\n")?;
    let repos = level.repos.iter().map(|(n, _)| (*n).to_owned()).collect();
    change_with(
        l,
        &Request::Switch(Some(repos)),
        "confirm-switch",
        "profile-switch-title",
    )
}

/// Lists repository profiles, or adds or takes one away.
fn profiles(l: &Localizer, args: &[String]) -> Result<String, String> {
    let fail = |e: kura_alpm::Error| l.msg_args("error-database", &[("reason", e.to_string().into())]);
    let (add, id) = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        [] => return Ok(profiles_list(l)),
        ["add", id] => (true, *id),
        ["remove", id] => (false, *id),
        ["switch", id] => return profiles_switch(l, id),
        _ => return Err(l.msg("profiles-usage") + "\n"),
    };
    let profile = kura_core::profiles::find(id).ok_or_else(|| l.msg("profiles-usage") + "\n")?;
    if add == kura_alpm::profiles::is_active(profile) {
        let id = if add { "profile-already" } else { "profile-missing" };
        return Ok(l.msg_args(id, &[("name", profile.id.into())]) + "\n");
    }
    let conf = kura_alpm::load_config().map_err(fail)?;
    let switches = kura_core::profiles::level(profile, kura_alpm::profiles::cpu_level())
        .or_else(|| kura_alpm::profiles::active_level(profile))
        .is_some_and(|l| l.architecture.is_some());
    if add && kura_core::profiles::level(profile, kura_alpm::profiles::cpu_level()).is_none() {
        return Err(l.msg_args("profile-cpu", &[("name", profile.id.into())]) + "\n");
    }
    eprintln!("{}", l.msg("check-running"));
    let check = |e: kura_alpm::Error| l.msg_args("error-check", &[("reason", e.to_string().into())]);
    let (change, estimate) = kura_alpm::profiles::prepare(
        &conf,
        Some(&kura_alpm::background::check_dir()),
        profile,
        switches,
    )
    .map_err(check)?;
    let apps = kura_alpm::apps(&conf).map_err(fail)?;
    let mut text = format!(
        "{}\n{}\n{}",
        l.msg("preview-title"),
        render(l, &change, None, &apps),
        l.msg(&format!("profile-{}-text", profile.id))
    );
    if let Some(Ok(plan)) = estimate.as_ref().map(|o| o.plan.as_ref()) {
        let outcome = estimate.as_ref().expect("checked above");
        let _ = writeln!(
            text,
            "\n{}",
            l.msg_args(
                if add {
                    "profile-switch-estimate"
                } else {
                    "profile-switch-back-estimate"
                },
                &[
                    ("count", plan.steps.len().into()),
                    ("download", l.size(outcome.download_size).into()),
                ]
            )
        );
    }
    say(&text);
    let question = if add {
        "confirm-profile-add"
    } else {
        "confirm-profile-remove"
    };
    if !confirm(l, question) {
        return Ok(l.msg("apply-cancelled") + "\n");
    }
    // Going back always switches back; adding asks.
    let switch_now = switches && (!add || confirm(l, "confirm-profile-switch"));
    let reply = kura_helper::apply(&change, progress_printer(l))
        .map_err(|e| l.msg_args("error-helper", &[("reason", e.to_string().into())]))?;
    if !matches!(reply, Reply::Done) {
        return Err(kura_helper::text::reply(l, &reply));
    }
    if !switch_now {
        return Ok(l.msg("apply-done") + "\n");
    }
    // The real switch, prepared with the repositories as they are now.
    let request = if add {
        let level = kura_alpm::profiles::active_level(profile).ok_or_else(|| l.msg("apply-changed"))?;
        Request::Switch(Some(level.repos.iter().map(|(n, _)| (*n).to_owned()).collect()))
    } else {
        Request::Switch(None)
    };
    change_with(l, &request, "confirm-switch", "profile-switch-title")
}

/// Shows or changes the settings for all users.
fn settings(l: &Localizer, args: &[String]) -> Result<String, String> {
    let mut s = kura_alpm::aur::settings();
    let on_off = |on: bool| l.msg(if on { "settings-on" } else { "settings-off" });
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        [] => {
            return Ok(format!(
                "{}: {}\n{}: {}\n{}: {}\n",
                l.msg("settings-aur"),
                on_off(s.aur),
                l.msg("settings-aur-wait"),
                l.msg_args("settings-days", &[("days", s.aur_cooling_days.into())]),
                l.msg("settings-auto-security"),
                on_off(s.auto_security)
            ));
        }
        ["aur", "on"] => s.aur = true,
        ["aur", "off"] => s.aur = false,
        ["auto-security", "on"] => s.auto_security = true,
        ["auto-security", "off"] => s.auto_security = false,
        ["aur-wait", days] => {
            s.aur_cooling_days = days
                .parse::<u32>()
                .ok()
                .filter(|d| *d <= kura_core::settings::Settings::MAX_COOLING_DAYS)
                .ok_or_else(|| l.msg("settings-usage") + "\n")?;
        }
        _ => return Err(l.msg("settings-usage") + "\n"),
    }
    let reply = kura_helper::set_settings(&s)
        .map_err(|e| l.msg_args("error-helper", &[("reason", e.to_string().into())]))?;
    match reply {
        Reply::Done => Ok(l.msg("settings-saved") + "\n"),
        other => Err(kura_helper::text::reply(l, &other)),
    }
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
    let Some(outcome) = kura_alpm::snapshots::prepare(&conf, Some(&kura_alpm::background::check_dir()), true)
        .map_err(fail)?
    else {
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
    let (name, id) = kura_helper::report::os_release();
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
