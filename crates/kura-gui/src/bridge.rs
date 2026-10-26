// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The object QML talks to. Slow work (preparing, installing) runs in
//! background threads and reports back through the Qt event loop, so the
//! window never freezes. Changes go through kura-helper only.

// cxx-qt's generated setter for the progress value compares floats exactly.
#![allow(clippy::float_cmp)]

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, distro)]
        #[qproperty(i32, packages)]
        #[qproperty(i32, explicit_count, cxx_name = "explicitCount")]
        #[qproperty(i32, foreign_count, cxx_name = "foreignCount")]
        #[qproperty(i32, orphan_count, cxx_name = "orphanCount")]
        #[qproperty(QString, error)]
        /// 0 unknown, 1 up to date, 2 updates ready, 3 needs attention, 4 security problem.
        #[qproperty(i32, state)]
        #[qproperty(QString, status_text, cxx_name = "statusText")]
        #[qproperty(QString, last_check, cxx_name = "lastCheck")]
        #[qproperty(bool, busy)]
        #[qproperty(QString, progress_text, cxx_name = "progressText")]
        // Between 0 and 1, or below 0 while nothing can be measured.
        #[qproperty(f64, progress_value, cxx_name = "progressValue")]
        #[qproperty(QString, progress_detail, cxx_name = "progressDetail")]
        // The details panel: what kura did, line by line, and whether it shows.
        #[qproperty(QString, log_text, cxx_name = "logText")]
        #[qproperty(bool, show_log, cxx_name = "showLog")]
        // Status and history, and the health page, are being read.
        #[qproperty(bool, refreshing)]
        #[qproperty(bool, health_loading, cxx_name = "healthLoading")]
        /// `[{"title": .., "items": [{"name": .., "detail": ..}]}]`
        #[qproperty(QString, preview_json, cxx_name = "previewJson")]
        /// `[{"title": .., "link": ..}]`
        #[qproperty(QString, news_json, cxx_name = "newsJson")]
        #[qproperty(QString, summary_text, cxx_name = "summaryText")]
        /// Heading of the preview page: an update or going back.
        #[qproperty(QString, preview_title, cxx_name = "previewTitle")]
        #[qproperty(bool, can_install, cxx_name = "canInstall")]
        #[qproperty(QString, message)]
        /// `[{"id": .., "when": .., "text": ..}]`, newest first.
        #[qproperty(QString, history_json, cxx_name = "historyJson")]
        #[qproperty(bool, monochrome)]
        // Text of the button that runs the previewed plan; JSON for the health
        // page (see health_value), the security cards and the search results.
        #[qproperty(QString, action_label, cxx_name = "actionLabel")]
        #[qproperty(QString, health_json, cxx_name = "healthJson")]
        #[qproperty(QString, security_json, cxx_name = "securityJson")]
        #[qproperty(QString, results_json, cxx_name = "resultsJson")]
        #[qproperty(bool, aur_enabled, cxx_name = "aurEnabled")]
        // The message is about a failure; the window offers a problem report.
        #[qproperty(bool, failed)]
        #[qproperty(i32, aur_wait, cxx_name = "aurWait")]
        #[qproperty(bool, auto_security, cxx_name = "autoSecurity")]
        #[qproperty(bool, trial_runs, cxx_name = "trialRuns")]
        // Bundles as JSON: [{"id", "name", "description"}].
        #[qproperty(QString, bundles_json, cxx_name = "bundlesJson")]
        // Repository profiles as JSON: [{"id", "text", "active"}].
        #[qproperty(QString, profiles_json, cxx_name = "profilesJson")]
        type Kura = super::KuraRust;
    }

    extern "RustQt" {
        /// Translated message for a Fluent ID.
        #[qinvokable]
        fn tr(self: &Kura, id: &QString) -> QString;

        // "3 days", for the AUR wait setting.
        #[qinvokable]
        #[cxx_name = "trDays"]
        fn tr_days(self: &Kura, days: i32) -> QString;

        /// Reads the system, the background check status and the history.
        #[qinvokable]
        fn refresh(self: Pin<&mut Kura>);

        /// Prepares the update preview on a private database copy.
        #[qinvokable]
        #[cxx_name = "prepareUpdate"]
        fn prepare_update(self: Pin<&mut Kura>);

        /// Prepares going back to an earlier generation.
        #[qinvokable]
        #[cxx_name = "prepareRestore"]
        fn prepare_restore(self: Pin<&mut Kura>, generation: i32);

        /// Sends the previewed plan to kura-helper.
        #[qinvokable]
        #[cxx_name = "installUpdate"]
        fn install_update(self: Pin<&mut Kura>);

        #[qinvokable]
        #[cxx_name = "setTrayMonochrome"]
        fn set_tray_monochrome(self: Pin<&mut Kura>, on: bool);

        /// Collects the facts for the system health page.
        #[qinvokable]
        #[cxx_name = "refreshHealth"]
        fn refresh_health(self: Pin<&mut Kura>);

        #[qinvokable]
        fn search(self: Pin<&mut Kura>, query: &QString);

        #[qinvokable]
        #[cxx_name = "prepareInstall"]
        fn prepare_install(self: Pin<&mut Kura>, names: &QString);

        #[qinvokable]
        #[cxx_name = "prepareRemove"]
        fn prepare_remove(self: Pin<&mut Kura>, name: &QString);

        #[qinvokable]
        #[cxx_name = "rankMirrors"]
        fn rank_mirrors(self: Pin<&mut Kura>);

        // Prepares setting up snapshots before updates.
        #[qinvokable]
        #[cxx_name = "prepareSnapshots"]
        fn prepare_snapshots(self: Pin<&mut Kura>);

        // System-wide settings, saved through kura-helper.
        #[qinvokable]
        #[cxx_name = "changeAur"]
        fn change_aur(self: Pin<&mut Kura>, on: bool);

        #[qinvokable]
        #[cxx_name = "changeAurWait"]
        fn change_aur_wait(self: Pin<&mut Kura>, days: i32);

        #[qinvokable]
        #[cxx_name = "changeAutoSecurity"]
        fn change_auto_security(self: Pin<&mut Kura>, on: bool);

        #[qinvokable]
        #[cxx_name = "changeTrialRuns"]
        fn change_trial_runs(self: Pin<&mut Kura>, on: bool);

        // Shows or hides the details panel and remembers the choice.
        #[qinvokable]
        #[cxx_name = "toggleLog"]
        fn toggle_log(self: Pin<&mut Kura>);

        // Previews installing a bundle.
        #[qinvokable]
        #[cxx_name = "prepareBundle"]
        fn prepare_bundle(self: Pin<&mut Kura>, id: &QString);

        // Previews adding (`add`) or removing a kernel.
        #[qinvokable]
        #[cxx_name = "prepareKernel"]
        fn prepare_kernel(self: Pin<&mut Kura>, name: &QString, add: bool);

        // Previews adding or taking away a repository profile.
        #[qinvokable]
        #[cxx_name = "prepareProfile"]
        fn prepare_profile(self: Pin<&mut Kura>, id: &QString);

        // The anonymised problem report as JSON: {"text", "url"}.
        #[qinvokable]
        #[cxx_name = "problemReport"]
        fn problem_report(self: &Kura) -> QString;
    }

    impl cxx_qt::Threading for Kura {}
}

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use kura_alpm::prepare::Request;
use kura_core::package::InstallReason;
use kura_core::plan::{Plan, RemovalReason, Step};
use kura_core::status::State;
use kura_i18n::Localizer;
use kura_i18n::present;
use serde_json::{Value, json};

#[allow(clippy::struct_excessive_bools)] // one field per Qt property
pub struct KuraRust {
    distro: QString,
    packages: i32,
    explicit_count: i32,
    foreign_count: i32,
    orphan_count: i32,
    error: QString,
    state: i32,
    status_text: QString,
    last_check: QString,
    busy: bool,
    progress_text: QString,
    progress_value: f64,
    progress_detail: QString,
    log_text: QString,
    show_log: bool,
    refreshing: bool,
    health_loading: bool,
    log: Vec<String>,
    preview_json: QString,
    news_json: QString,
    summary_text: QString,
    preview_title: QString,
    can_install: bool,
    message: QString,
    history_json: QString,
    monochrome: bool,
    action_label: QString,
    health_json: QString,
    security_json: QString,
    results_json: QString,
    aur_enabled: bool,
    failed: bool,
    aur_wait: i32,
    auto_security: bool,
    trial_runs: bool,
    profiles_json: QString,
    bundles_json: QString,
    localizer: Localizer,
    pending: Option<Plan>,
}

impl Default for KuraRust {
    fn default() -> Self {
        Self {
            distro: QString::default(),
            packages: 0,
            explicit_count: 0,
            foreign_count: 0,
            orphan_count: 0,
            error: QString::default(),
            state: 0,
            status_text: QString::default(),
            last_check: QString::default(),
            busy: false,
            progress_text: QString::default(),
            progress_value: -1.0,
            progress_detail: QString::default(),
            log_text: QString::default(),
            show_log: read_settings()
                .get("log_panel")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            refreshing: false,
            health_loading: false,
            log: Vec::new(),
            preview_json: QString::from("[]"),
            news_json: QString::from("[]"),
            summary_text: QString::default(),
            preview_title: QString::default(),
            can_install: false,
            message: QString::default(),
            history_json: QString::from("[]"),
            monochrome: read_settings()
                .get("tray_monochrome")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            action_label: QString::default(),
            health_json: QString::from("{}"),
            security_json: QString::from("[]"),
            results_json: QString::from("[]"),
            aur_enabled: kura_alpm::aur::settings().aur,
            failed: false,
            aur_wait: i32::try_from(kura_alpm::aur::settings().aur_cooling_days).unwrap_or(3),
            auto_security: kura_alpm::aur::settings().auto_security,
            trial_runs: kura_alpm::aur::settings().trial_runs,
            profiles_json: QString::from(&profiles_value(&Localizer::from_env()).to_string()),
            bundles_json: QString::from(&bundles_value(&Localizer::from_env()).to_string()),
            localizer: Localizer::from_env(),
            pending: None,
        }
    }
}

fn count(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".config")))
        .unwrap_or_default()
        .join("kura")
}

fn read_settings() -> Value {
    std::fs::read_to_string(config_dir().join("settings.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| json!({}))
}

/// The preview groups as JSON, in the order the window shows them.
fn preview_groups(l: &Localizer, plan: &Plan, apps: &BTreeSet<String>) -> Value {
    let is_app = |p: &str| apps.contains(p);
    let groups = plan.groups(&is_app);
    let (replaced, system): (Vec<&Step>, Vec<&Step>) = groups.system.iter().partition(|s| {
        matches!(
            s,
            Step::Remove {
                why: RemovalReason::Replaced,
                ..
            }
        )
    });
    let group = |id: &str, steps: &[&Step]| {
        json!({
            "title": l.msg_args(id, &[("count", steps.len().into())]),
            "items": steps.iter().map(|s| json!({"name": s.package(), "detail": present::step_text(l, s)})).collect::<Vec<_>>(),
        })
    };
    let reinstall = |s: &&&Step| {
        matches!(
            s,
            Step::Remove {
                why: RemovalReason::Reinstall,
                ..
            }
        )
    };
    let again: Vec<&Step> = plan.steps.iter().filter(|s| reinstall(&s)).collect();
    let apps: Vec<&Step> = groups.apps.iter().copied().filter(|s| !reinstall(&s)).collect();
    let system: Vec<&Step> = system.into_iter().filter(|s| !reinstall(&s)).collect();
    let (tools, system): (Vec<&Step>, Vec<&Step>) = system
        .into_iter()
        .partition(|s| kura_i18n::present::is_build_tool(plan, s));
    let all = [
        ("check-group-apps", apps.as_slice()),
        ("check-group-system", system.as_slice()),
        ("check-group-reinstall", again.as_slice()),
        ("check-group-build-tools", tools.as_slice()),
        ("check-group-replaced", replaced.as_slice()),
        ("check-group-cleanup", groups.cleanup.as_slice()),
    ];
    let mut out: Vec<Value> = all
        .iter()
        .filter(|(_, s)| !s.is_empty())
        .map(|(id, s)| group(id, s))
        .collect();
    if !plan.actions.is_empty() {
        out.push(json!({
            "title": l.msg("preview-actions"),
            "items": plan.actions.iter().map(|a| json!({"name": present::action(l, a), "detail": ""})).collect::<Vec<_>>(),
        }));
    }
    Value::Array(out)
}

struct Prepared {
    plan: Plan,
    preview: String,
    news: String,
    summary: String,
    headline: String,
}

/// Runs in a background thread: going back to generation `target`.
fn prepare_restore_preview(target: u64) -> Result<Prepared, String> {
    let l = Localizer::from_env();
    let db = |e: kura_alpm::Error| l.msg_args("error-database", &[("reason", e.to_string().into())]);
    let journal = kura_alpm::history::load(Path::new(kura_alpm::history::JOURNAL)).map_err(db)?;
    let conf = kura_alpm::load_config().map_err(db)?;
    let unknown = || l.msg_args("undo-unknown", &[("id", target.into())]);
    let generation = journal.generation(target).ok_or_else(unknown)?;
    let plan = kura_alpm::restore::plan(&journal, target).ok_or_else(unknown)?;
    if plan.steps.is_empty() {
        return Err(l.msg("undo-nothing"));
    }
    if let Err(problems) = kura_alpm::restore::sources(&conf.cache_dir, &plan, &generation) {
        return Err(problems
            .iter()
            .map(|p| kura_helper::text::problem(&l, p))
            .collect::<Vec<_>>()
            .join("\n"));
    }
    let apps = kura_alpm::apps(&conf).unwrap_or_default();
    let when = present::when(&l, generation.created, kura_alpm::background::now());
    Ok(Prepared {
        preview: preview_groups(&l, &plan, &apps).to_string(),
        news: "[]".into(),
        summary: l.msg("undo-limits"),
        headline: l.msg_args("undo-title", &[("id", target.into()), ("when", when.into())]),
        plan,
    })
}

/// What the Overview and History pages show, read in a background thread.
struct Overview {
    distro: String,
    state: i32,
    status_text: String,
    last_check: String,
    history: String,
    security: String,
    error: String,
    counts: Option<(i32, i32, i32, i32)>,
}

fn overview(l: &Localizer) -> Overview {
    let distro = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|t| {
            t.lines()
                .find_map(|l| l.strip_prefix("NAME=").map(|v| v.trim_matches('"').to_owned()))
        })
        .unwrap_or_else(|| "Linux".to_owned());
    let status = kura_alpm::background::read_status(Path::new(kura_alpm::background::STATUS));
    let now = kura_alpm::background::now();
    let (state, status_text) = match status.state() {
        State::Security => (4, present::security_headline(l, &status.security)),
        State::Unknown => (0, l.msg("tray-unknown")),
        State::UpToDate => (1, l.msg("tray-uptodate")),
        State::UpdatesReady => (
            2,
            l.msg_args(
                "tray-updates",
                &[("count", (status.updates + status.aur_updates).into())],
            ),
        ),
        State::NeedsAttention => (3, l.msg("tray-attention")),
    };
    let last_check = if status.checked > 0 {
        l.msg_args(
            "gui-last-check",
            &[("when", present::when(l, status.checked, now).into())],
        )
    } else {
        String::new()
    };
    let history = kura_alpm::history::load(Path::new(kura_alpm::history::JOURNAL))
        .map_or_else(
            |_| json!([]),
            |j| {
                Value::Array(
                    j.records()
                        .iter()
                        .rev()
                        .map(|r| json!({"id": r.id, "when": present::when(l, r.created, now), "text": present::record_text(l, r)}))
                        .collect(),
                )
            },
        )
        .to_string();
    let security = security_value(l, &status.security).to_string();
    let loaded = kura_alpm::load_config().and_then(|conf| kura_alpm::installed(&conf));
    let error = loaded
        .as_ref()
        .err()
        .map(|e| l.msg_args("error-database", &[("reason", e.to_string().into())]))
        .unwrap_or_default();
    let counts = loaded.ok().map(|set| {
        let explicit = set.iter().filter(|p| p.reason == InstallReason::Explicit).count();
        let foreign = set.iter().filter(|p| p.repo.is_none()).count();
        (
            count(set.len()),
            count(explicit),
            count(foreign),
            count(set.orphans().len()),
        )
    });
    Overview {
        distro,
        state,
        status_text,
        last_check,
        history,
        security,
        error,
        counts,
    }
}

/// The local time zone's offset from UTC in seconds, from `date`.
fn local_offset() -> i64 {
    std::process::Command::new("date")
        .arg("+%z")
        .output()
        .ok()
        .and_then(|o| {
            let z = String::from_utf8_lossy(&o.stdout).trim().to_owned();
            let sign = if z.starts_with('-') { -1 } else { 1 };
            let digits = z.trim_start_matches(['+', '-']);
            let h: i64 = digits.get(..2)?.parse().ok()?;
            let m: i64 = digits.get(2..4)?.parse().ok()?;
            Some(sign * (h * 3600 + m * 60))
        })
        .unwrap_or(0)
}

/// Seconds since 1970, for progress estimates.
fn seconds() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

/// Shows download progress of a preview while it is prepared.
fn forward_progress(
    rx: std::sync::mpsc::Receiver<kura_alpm::apply::Event>,
    qt: cxx_qt::CxxQtThread<qobject::Kura>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let l = Localizer::from_env();
        let mut tracker = kura_core::progress::Tracker::default();
        let mut seen = BTreeSet::new();
        for event in rx {
            let now = seconds();
            kura_helper::text::track(&mut tracker, &event, now);
            let value = tracker.fraction().unwrap_or(-1.0);
            let detail = kura_helper::text::progress(&l, &tracker, now);
            let line = kura_helper::text::event(&l, &event);
            let fresh = seen.insert(line.clone());
            let _ = qt.queue(move |mut this| {
                this.as_mut().set_progress_value(value);
                this.as_mut().set_progress_detail(QString::from(&detail));
                if fresh {
                    this.as_mut().append_log(&line);
                }
            });
        }
    })
}

/// Runs in a background thread.
fn prepare_preview(request: &Request) -> Result<Prepared, String> {
    let l = Localizer::from_env();
    let fail = |e: kura_alpm::Error| l.msg_args("error-check", &[("reason", e.to_string().into())]);
    let conf = kura_alpm::load_config().map_err(fail)?;
    let (outcome, repair) = kura_alpm::prepare::prepare_with_repair(
        &conf,
        Some(&kura_alpm::background::check_dir()),
        request,
        !matches!(request, Request::Remove(_)),
        &user_locales(),
    )
    .map_err(fail)?;
    let mut plan = match outcome.plan {
        Ok(ref plan) => plan.clone(),
        Err(ref problems) => {
            let id = if matches!(request, Request::Update) {
                "check-blocked"
            } else {
                "change-blocked"
            };
            return Err(blocked_text(&l, id, problems));
        }
    };
    if matches!(request, Request::Update) {
        let set = kura_alpm::installed(&conf).map_err(fail)?;
        let cleanup = kura_alpm::cleanup(&set);
        plan.add_cleanup(cleanup.iter().map(|(n, v)| (n.as_str(), v.as_str())));
    }
    let apps = kura_alpm::apps(&conf).map_err(fail)?;

    let packages: BTreeSet<String> = plan.steps.iter().map(|s| s.package().to_owned()).collect();
    let news = if matches!(request, Request::Update) {
        kura_alpm::background::fetch_news().map_or_else(
            || json!([{"title": l.msg("news-unavailable"), "link": "https://archlinux.org/news/"}]),
            |items| {
                let since = kura_alpm::background::news_since(
                    Path::new(kura_alpm::history::JOURNAL),
                    kura_alpm::background::now(),
                );
                let hits = kura_core::news::affecting(&items, since, &packages);
                Value::Array(
                    hits.iter()
                        .map(|i| json!({"title": i.title, "link": i.link}))
                        .collect(),
                )
            },
        )
    } else {
        json!([])
    };
    let solved = repair.map(|r| kura_helper::text::repair(&l, &r));
    // AUR updates come after the repository update, in their own preview.
    let aur_ready = if matches!(request, Request::Update) {
        aur_ready(&conf)
    } else {
        Vec::new()
    };
    if matches!(request, Request::Update) && outcome.available.is_empty() && !aur_ready.is_empty() {
        let mut prepared = prepare_aur_preview(&aur_ready)?;
        prepared.headline = format!(
            "{}\n{}",
            kura_helper::text::aur_context(&l, &conf),
            prepared.headline
        );
        return Ok(prepared);
    }
    let mut headline = if let Some(text) = solved {
        text
    } else if !matches!(request, Request::Update) {
        String::new()
    } else if outcome.available.is_empty() {
        l.msg("check-none")
    } else {
        l.msg_args("check-found", &[("count", outcome.available.len().into())])
    };
    if !aur_ready.is_empty() {
        headline = format!(
            "{headline}\n{}",
            l.msg_args("gui-aur-after", &[("count", aur_ready.len().into())])
        );
    }
    Ok(Prepared {
        preview: preview_groups(&l, &plan, &apps).to_string(),
        news: news.to_string(),
        summary: sizes(&l, &outcome),
        headline,
        plan,
    })
}

fn blocked_text(l: &Localizer, id: &str, problems: &[kura_alpm::prepare::Problem]) -> String {
    let mut text = l.msg(id);
    for p in problems {
        text.push_str("\n• ");
        text.push_str(&kura_helper::text::problem(l, p));
    }
    text
}

fn sizes(l: &Localizer, outcome: &kura_alpm::prepare::Outcome) -> String {
    l.msg_args(
        "check-sizes",
        &[
            ("download", l.size(outcome.download_size).into()),
            ("delta", l.size_delta(outcome.installed_delta).into()),
        ],
    )
}

/// Search results as JSON: repository hits first, then AUR hits with their
/// trust signals.
fn search_value(l: &Localizer, query: &str, aur: bool) -> Value {
    let Ok(conf) = kura_alpm::load_config() else {
        return json!([]);
    };
    let mut out: Vec<Value> = kura_alpm::system::search(&conf, query, 40)
        .unwrap_or_default()
        .iter()
        .map(|h| json!({"name": h.name, "version": h.version, "description": h.description, "installed": h.installed, "aur": false}))
        .collect();
    if aur && !query.trim().is_empty() {
        let installed = kura_alpm::installed(&conf).unwrap_or_default();
        let now = kura_alpm::background::now();
        let mut hits = kura_alpm::aur::search(query.trim(), "name-desc").unwrap_or_default();
        hits.sort_by(|a, b| b.popularity.total_cmp(&a.popularity));
        hits.truncate(30);
        out.extend(hits.iter().map(|h| {
            json!({
                "name": h.name,
                "version": h.version,
                "description": h.description.clone().unwrap_or_default(),
                "installed": installed.get(&h.name).is_some(),
                "aur": true,
                "signals": present::aur_signals(l, h, now),
            })
        }));
    }
    Value::Array(out)
}

/// AUR packages whose update is past the cooling period, and those that a
/// library update broke, which are built again right away.
fn aur_ready(conf: &kura_alpm::Config) -> Vec<String> {
    let settings = kura_alpm::aur::settings();
    if !settings.aur {
        return Vec::new();
    }
    let mut names: Vec<String> =
        kura_alpm::aur::updates(conf, &settings, kura_alpm::background::now(), false)
            .unwrap_or_default()
            .into_iter()
            .filter(|u| u.waits_until.is_none())
            .map(|u| u.name)
            .collect();
    for name in kura_alpm::aur::rebuild_targets(conf).unwrap_or_default() {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// Runs in a background thread: what building `names` from the AUR changes.
fn prepare_aur_preview(names: &[String]) -> Result<Prepared, String> {
    let l = Localizer::from_env();
    let fail = |e: kura_alpm::Error| l.msg_args("error-check", &[("reason", e.to_string().into())]);
    let conf = kura_alpm::load_config().map_err(fail)?;
    if !kura_alpm::aur::settings().aur {
        return Err(l.msg_args("aur-off", &[("names", names.join(" ").into())]));
    }
    let outcome =
        kura_alpm::aur::prepare_install(&conf, Some(&kura_alpm::background::check_dir()), names, true)
            .map_err(fail)?;
    let plan = match &outcome.plan {
        Ok(plan) => plan.clone(),
        Err(problems) => return Err(blocked_text(&l, "change-blocked", problems)),
    };
    let apps = kura_alpm::apps(&conf).map_err(fail)?;
    let infos = kura_alpm::aur::info(names).unwrap_or_default();
    let now = kura_alpm::background::now();
    let mut preview = preview_groups(&l, &plan, &apps);
    if let Value::Array(groups) = &mut preview {
        groups.push(json!({
            "title": l.msg("aur-build-list"),
            "items": plan.builds.iter().map(|b| json!({
                "name": b.base,
                "detail": infos
                    .iter()
                    .find(|i| i.package_base == b.base)
                    .map(|i| present::aur_signals(&l, i, now))
                    .unwrap_or_default(),
            })).collect::<Vec<_>>(),
        }));
    }
    Ok(Prepared {
        preview: preview.to_string(),
        news: "[]".into(),
        summary: sizes(&l, &outcome),
        headline: l.msg("aur-notice"),
        plan,
    })
}

/// The bundles for the Software page.
fn bundles_value(l: &Localizer) -> Value {
    let lang = l.language();
    let langs = [lang.as_str()];
    Value::Array(
        kura_alpm::bundles::load(Path::new(kura_alpm::bundles::DIR))
            .iter()
            .map(
                |b| json!({"id": b.id, "name": b.name.get(&langs), "description": b.description.get(&langs)}),
            )
            .collect(),
    )
}

/// Runs in a background thread: installing a bundle with its recommended apps.
fn prepare_bundle_preview(bundle: &kura_core::bundles::Bundle) -> Result<Prepared, String> {
    let l = Localizer::from_env();
    let fail = |e: kura_alpm::Error| l.msg_args("error-check", &[("reason", e.to_string().into())]);
    let conf = kura_alpm::load_config().map_err(fail)?;
    let user = std::env::var("USER").unwrap_or_default();
    let outcome = kura_alpm::bundles::prepare(
        &conf,
        Some(&kura_alpm::background::check_dir()),
        bundle,
        &user,
        &[],
    )
    .map_err(fail)?;
    let plan = match &outcome.plan {
        Ok(plan) => plan.clone(),
        Err(problems) => return Err(blocked_text(&l, "change-blocked", problems)),
    };
    let apps = kura_alpm::apps(&conf).map_err(fail)?;
    let lang = l.language();
    Ok(Prepared {
        preview: preview_groups(&l, &plan, &apps).to_string(),
        news: "[]".into(),
        summary: sizes(&l, &outcome),
        headline: bundle.description.get(&[lang.as_str()]).to_owned(),
        plan,
    })
}

/// Runs in a background thread: adding or removing a kernel.
fn prepare_kernel_preview(name: &str, add: bool) -> Result<Prepared, String> {
    let l = Localizer::from_env();
    let fail = |e: kura_alpm::Error| l.msg_args("error-check", &[("reason", e.to_string().into())]);
    let conf = kura_alpm::load_config().map_err(fail)?;
    if !add {
        let installed: Vec<String> = kura_alpm::installed(&conf)
            .map_err(fail)?
            .iter()
            .map(|p| p.name.clone())
            .collect();
        let running = kura_alpm::kernels::running_package();
        match kura_core::kernels::can_remove(name, &installed, running.as_deref()) {
            Err(kura_core::kernels::Keep::Running) => {
                return Err(l.msg_args("kernel-keep-running", &[("name", name.into())]));
            }
            Err(kura_core::kernels::Keep::Last) => {
                return Err(l.msg_args("kernel-keep-last", &[("name", name.into())]));
            }
            Ok(()) => {}
        }
    }
    let outcome = kura_alpm::kernels::prepare(&conf, Some(&kura_alpm::background::check_dir()), name, add)
        .map_err(fail)?;
    let plan = match &outcome.plan {
        Ok(plan) => plan.clone(),
        Err(problems) => return Err(blocked_text(&l, "change-blocked", problems)),
    };
    let apps = kura_alpm::apps(&conf).map_err(fail)?;
    Ok(Prepared {
        preview: preview_groups(&l, &plan, &apps).to_string(),
        news: "[]".into(),
        summary: sizes(&l, &outcome),
        headline: String::new(),
        plan,
    })
}

/// The repository profiles for the settings page.
fn profiles_value(l: &Localizer) -> Value {
    Value::Array(
        kura_core::profiles::ALL
            .iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "text": l.msg(&format!("profile-{}-text", p.id)),
                    "active": kura_alpm::profiles::is_active(p),
                })
            })
            .collect(),
    )
}

/// Runs in a background thread: what adding or taking away a profile changes.
fn prepare_profile_preview(profile: &'static kura_core::profiles::Profile) -> Result<Prepared, String> {
    let l = Localizer::from_env();
    let fail = |e: kura_alpm::Error| l.msg_args("error-check", &[("reason", e.to_string().into())]);
    let conf = kura_alpm::load_config().map_err(fail)?;
    let add = !kura_alpm::profiles::is_active(profile);
    if add && kura_core::profiles::level(profile, kura_alpm::profiles::cpu_level()).is_none() {
        return Err(l.msg_args("profile-cpu", &[("name", profile.id.into())]));
    }
    let (plan, estimate) =
        kura_alpm::profiles::prepare(&conf, Some(&kura_alpm::background::check_dir()), profile, true)
            .map_err(fail)?;
    let apps = kura_alpm::apps(&conf).map_err(fail)?;
    let mut headline = l.msg(&format!("profile-{}-text", profile.id));
    if let Some(outcome) = &estimate
        && let Ok(switch) = &outcome.plan
    {
        let id = if add {
            "profile-switch-estimate"
        } else {
            "profile-switch-back-estimate"
        };
        let line = l.msg_args(
            id,
            &[
                ("count", switch.steps.len().into()),
                ("download", l.size(outcome.download_size).into()),
            ],
        );
        headline = format!("{headline}\n{line}");
    }
    Ok(Prepared {
        preview: preview_groups(&l, &plan, &apps).to_string(),
        news: "[]".into(),
        summary: String::new(),
        headline,
        plan,
    })
}

/// Runs in a background thread: switching installed packages to the active
/// profile's builds, or back to the generic ones.
fn prepare_switch_preview(back: bool) -> Result<Prepared, String> {
    let request = if back {
        Request::Switch(None)
    } else {
        let level = kura_core::profiles::ALL
            .iter()
            .find_map(|p| kura_alpm::profiles::active_level(p).filter(|l| l.architecture.is_some()))
            .ok_or_else(|| Localizer::from_env().msg("check-none"))?;
        Request::Switch(Some(level.repos.iter().map(|(n, _)| (*n).to_owned()).collect()))
    };
    prepare_preview(&request)
}

/// Runs in a background thread: what setting up snapshots would change.
fn prepare_snapshots_preview() -> Result<Prepared, String> {
    let l = Localizer::from_env();
    let fail = |e: kura_alpm::Error| l.msg_args("error-check", &[("reason", e.to_string().into())]);
    let conf = kura_alpm::load_config().map_err(fail)?;
    let outcome = kura_alpm::snapshots::prepare(&conf, Some(&kura_alpm::background::check_dir()), true)
        .map_err(fail)?
        .ok_or_else(|| l.msg("snapshots-nothing"))?;
    let plan = match &outcome.plan {
        Ok(plan) => plan.clone(),
        Err(problems) => return Err(blocked_text(&l, "change-blocked", problems)),
    };
    let apps = kura_alpm::apps(&conf).map_err(fail)?;
    Ok(Prepared {
        preview: preview_groups(&l, &plan, &apps).to_string(),
        news: "[]".into(),
        summary: sizes(&l, &outcome),
        headline: String::new(),
        plan,
    })
}

impl qobject::Kura {
    fn tr(&self, id: &QString) -> QString {
        QString::from(&self.localizer.msg(&id.to_string()))
    }

    fn problem_report(&self) -> QString {
        let (text, url) = kura_helper::report::build(&self.message().to_string());
        QString::from(&json!({"text": text, "url": url}).to_string())
    }

    fn tr_days(&self, days: i32) -> QString {
        QString::from(&self.localizer.msg_args("settings-days", &[("days", days.into())]))
    }

    fn refresh(mut self: Pin<&mut Self>) {
        if *self.refreshing() {
            return;
        }
        self.as_mut().set_refreshing(true);
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let o = overview(&Localizer::from_env());
            let _ = qt.queue(move |mut this| {
                this.as_mut().set_distro(QString::from(&o.distro));
                this.as_mut().set_state(o.state);
                this.as_mut().set_status_text(QString::from(&o.status_text));
                this.as_mut().set_last_check(QString::from(&o.last_check));
                this.as_mut().set_history_json(QString::from(&o.history));
                this.as_mut().set_security_json(QString::from(&o.security));
                this.as_mut().set_error(QString::from(&o.error));
                if let Some((packages, explicit, foreign, orphans)) = o.counts {
                    this.as_mut().set_packages(packages);
                    this.as_mut().set_explicit_count(explicit);
                    this.as_mut().set_foreign_count(foreign);
                    this.as_mut().set_orphan_count(orphans);
                }
                this.as_mut().set_refreshing(false);
            });
        });
    }

    fn prepare_update(self: Pin<&mut Self>) {
        let title = self.localizer.msg("nav-updates");
        let label = self.localizer.msg("gui-install");
        self.start_preview(|| prepare_preview(&Request::Update), &title, &label);
    }

    fn prepare_install(self: Pin<&mut Self>, names: &QString) {
        let names: Vec<String> = names.to_string().split_whitespace().map(str::to_owned).collect();
        let title = self
            .localizer
            .msg_args("gui-install-title", &[("name", names.join(", ").into())]);
        let label = self.localizer.msg("gui-install");
        self.start_preview(
            move || {
                // Names no repository has come from the AUR.
                let in_repos = kura_alpm::load_config().is_ok_and(|conf| {
                    names
                        .iter()
                        .all(|n| kura_alpm::in_repos(&conf, n).unwrap_or(false))
                });
                if in_repos {
                    prepare_preview(&Request::Install(names))
                } else {
                    prepare_aur_preview(&names)
                }
            },
            &title,
            &label,
        );
    }

    fn prepare_remove(self: Pin<&mut Self>, name: &QString) {
        let name = name.to_string();
        let title = self
            .localizer
            .msg_args("gui-remove-title", &[("name", name.clone().into())]);
        let label = self.localizer.msg("gui-remove");
        self.start_preview(
            move || prepare_preview(&Request::Remove(vec![name])),
            &title,
            &label,
        );
    }

    fn prepare_bundle(self: Pin<&mut Self>, id: &QString) {
        let Some(bundle) = kura_alpm::bundles::find(&id.to_string()) else {
            return;
        };
        let lang = self.localizer.language();
        let title = bundle.name.get(&[lang.as_str()]).to_owned();
        let label = self.localizer.msg("gui-install");
        self.start_preview(move || prepare_bundle_preview(&bundle), &title, &label);
    }

    fn prepare_kernel(self: Pin<&mut Self>, name: &QString, add: bool) {
        let name = name.to_string();
        let (title, label) = if add {
            ("gui-install-title", "gui-install")
        } else {
            ("gui-remove-title", "gui-remove")
        };
        let title = self.localizer.msg_args(title, &[("name", name.clone().into())]);
        let label = self.localizer.msg(label);
        self.start_preview(move || prepare_kernel_preview(&name, add), &title, &label);
    }

    fn prepare_profile(self: Pin<&mut Self>, id: &QString) {
        let Some(profile) = kura_core::profiles::find(&id.to_string()) else {
            return;
        };
        let (title, label) = if kura_alpm::profiles::is_active(profile) {
            ("gui-profile-remove", "gui-remove")
        } else {
            ("gui-profile-add", "gui-add")
        };
        let title = self.localizer.msg_args(title, &[("name", profile.id.into())]);
        let label = self.localizer.msg(label);
        self.start_preview(move || prepare_profile_preview(profile), &title, &label);
    }

    fn prepare_snapshots(self: Pin<&mut Self>) {
        let title = self.localizer.msg("suggest-snapshots");
        let label = self.localizer.msg("gui-setup");
        self.start_preview(prepare_snapshots_preview, &title, &label);
    }

    /// Prepares any request in a background thread and shows it as the preview.
    fn start_preview(
        mut self: Pin<&mut Self>,
        prepare: impl FnOnce() -> Result<Prepared, String> + Send + 'static,
        title: &str,
        label: &str,
    ) {
        if *self.busy() {
            return;
        }
        let running = self.localizer.msg("check-running");
        self.as_mut().set_preview_title(QString::from(title));
        self.as_mut().set_action_label(QString::from(label));
        self.as_mut().set_busy(true);
        self.as_mut().set_can_install(false);
        self.as_mut().set_message(QString::default());
        self.as_mut().set_failed(false);
        self.as_mut().set_preview_json(QString::from("[]"));
        self.as_mut().set_news_json(QString::from("[]"));
        self.as_mut().set_progress_text(QString::from(&running));
        self.as_mut().append_log(&format!("{title}: {running}"));
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let forward = forward_progress(rx, qt.clone());
            kura_alpm::prepare::report_progress_to(Some(tx));
            let result = prepare();
            kura_alpm::prepare::report_progress_to(None);
            let _ = forward.join();
            let _ = qt.queue(move |mut this| {
                this.as_mut().set_progress_value(-1.0);
                this.as_mut().set_progress_detail(QString::default());
                match result {
                    Ok(p) => {
                        let has_changes = !p.plan.steps.is_empty()
                            || !p.plan.actions.is_empty()
                            || !p.plan.builds.is_empty();
                        this.as_mut().set_preview_json(QString::from(&p.preview));
                        this.as_mut().set_news_json(QString::from(&p.news));
                        this.as_mut().set_summary_text(QString::from(&p.summary));
                        this.as_mut().append_log(&p.headline);
                        this.as_mut().append_log(&p.summary);
                        this.as_mut().set_progress_text(QString::from(&p.headline));
                        this.as_mut().set_can_install(has_changes);
                        this.as_mut().rust_mut().pending = Some(p.plan);
                    }
                    Err(text) => {
                        this.as_mut().append_log(&text);
                        this.as_mut().set_message(QString::from(&text));
                        this.as_mut().set_failed(true);
                        this.as_mut().set_progress_text(QString::default());
                    }
                }
                this.as_mut().set_busy(false);
            });
        });
    }

    fn refresh_health(mut self: Pin<&mut Self>) {
        if *self.health_loading() {
            return;
        }
        self.as_mut().set_health_loading(true);
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let value = kura_alpm::load_config()
                .and_then(|conf| kura_alpm::system::collect(&conf))
                .map_or_else(|_| json!({}), |info| health_value(&Localizer::from_env(), &info));
            let _ = qt.queue(move |mut this| {
                this.as_mut().set_health_json(QString::from(&value.to_string()));
                this.as_mut().set_health_loading(false);
            });
        });
    }

    fn search(self: Pin<&mut Self>, query: &QString) {
        let query = query.to_string();
        let aur = *self.aur_enabled();
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let value = search_value(&Localizer::from_env(), &query, aur);
            let _ =
                qt.queue(move |mut this| this.as_mut().set_results_json(QString::from(&value.to_string())));
        });
    }

    fn change_aur(self: Pin<&mut Self>, on: bool) {
        let mut s = kura_alpm::aur::settings();
        s.aur = on;
        self.save_settings(s);
    }

    fn change_trial_runs(self: Pin<&mut Self>, on: bool) {
        let mut s = kura_alpm::aur::settings();
        s.trial_runs = on;
        self.save_settings(s);
    }

    fn change_auto_security(self: Pin<&mut Self>, on: bool) {
        let mut s = kura_alpm::aur::settings();
        s.auto_security = on;
        self.save_settings(s);
    }

    fn change_aur_wait(self: Pin<&mut Self>, days: i32) {
        let mut s = kura_alpm::aur::settings();
        s.aur_cooling_days = u32::try_from(days).unwrap_or(0);
        self.save_settings(s.clamped());
    }

    /// Saves through kura-helper and shows what is saved afterwards.
    fn save_settings(self: Pin<&mut Self>, settings: kura_core::settings::Settings) {
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let l = Localizer::from_env();
            let text = match kura_helper::set_settings(&settings) {
                Ok(kura_helper::Reply::Done) => String::new(),
                Ok(reply) => kura_helper::text::reply(&l, &reply),
                Err(e) => l.msg_args("error-helper", &[("reason", e.to_string().into())]),
            };
            let saved = kura_alpm::aur::settings();
            let _ = qt.queue(move |mut this| {
                this.as_mut().set_aur_enabled(saved.aur);
                this.as_mut().set_auto_security(saved.auto_security);
                this.as_mut().set_trial_runs(saved.trial_runs);
                this.as_mut()
                    .set_aur_wait(i32::try_from(saved.aur_cooling_days).unwrap_or(0));
                this.as_mut().set_message(QString::from(&text));
            });
        });
    }

    fn rank_mirrors(mut self: Pin<&mut Self>) {
        if *self.busy() {
            return;
        }
        let running = self.localizer.msg("mirrors-running");
        self.as_mut().set_busy(true);
        self.as_mut().set_message(QString::from(&running));
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let l = Localizer::from_env();
            let text = match kura_helper::rank_mirrors() {
                Ok(reply) => kura_helper::text::reply(&l, &reply),
                Err(e) => l.msg_args("error-helper", &[("reason", e.to_string().into())]),
            };
            let _ = qt.queue(move |mut this| {
                this.as_mut().append_log(&text);
                this.as_mut().set_message(QString::from(&text));
                this.as_mut().set_busy(false);
                this.as_mut().refresh_health();
            });
        });
    }

    fn prepare_restore(self: Pin<&mut Self>, generation: i32) {
        let Ok(target) = u64::try_from(generation) else {
            return;
        };
        let title = self.localizer.msg("gui-go-back");
        let label = self.localizer.msg("gui-go-back");
        self.start_preview(move || prepare_restore_preview(target), &title, &label);
    }

    fn install_update(mut self: Pin<&mut Self>) {
        let Some(plan) = self.as_mut().rust_mut().pending.take() else {
            return;
        };
        self.as_mut().set_busy(true);
        self.as_mut().set_can_install(false);
        let qt = self.qt_thread();
        // Apps a repair removed come back right after the update.
        let again: Vec<String> = plan
            .steps
            .iter()
            .filter_map(|s| match s {
                Step::Remove {
                    name,
                    why: RemovalReason::Reinstall,
                    ..
                } => Some(name.clone()),
                _ => None,
            })
            .collect();
        // After a profile with optimised builds changes, the switch follows.
        let switch_after = (plan.kind == kura_core::plan::PlanKind::Profile)
            .then(|| kura_core::profiles::known(&plan.actions))
            .flatten()
            .filter(|(_, level, _)| level.architecture.is_some())
            .map(|(_, _, add)| !add);
        std::thread::spawn(move || {
            let l = Localizer::from_env();
            let events = qt.clone();
            let mut tracker = kura_core::progress::Tracker::default();
            let mut last_line = String::new();
            let result = kura_helper::apply(&plan, |event| {
                let line = kura_helper::text::event(&l, &event);
                let now = seconds();
                kura_helper::text::track(&mut tracker, &event, now);
                let value = tracker.fraction().unwrap_or(-1.0);
                let detail = kura_helper::text::progress(&l, &tracker, now);
                let fresh = last_line != line;
                last_line.clone_from(&line);
                let _ = events.queue(move |mut this| {
                    this.as_mut().set_progress_text(QString::from(&line));
                    this.as_mut().set_progress_value(value);
                    this.as_mut().set_progress_detail(QString::from(&detail));
                    if fresh {
                        this.as_mut().append_log(&line);
                    }
                });
            });
            let (text, fresh, failed) = match result {
                Ok(reply) => {
                    let fresh = match &reply {
                        kura_helper::Reply::Changed { plan } => Some(plan.clone()),
                        _ => None,
                    };
                    let failed = !matches!(
                        reply,
                        kura_helper::Reply::Done | kura_helper::Reply::Changed { .. }
                    );
                    (kura_helper::text::reply(&l, &reply), fresh, failed)
                }
                Err(e) => (
                    l.msg_args("error-helper", &[("reason", e.to_string().into())]),
                    None,
                    true,
                ),
            };
            let _ = qt.queue(move |mut this| {
                this.as_mut().append_log(&text);
                this.as_mut().set_message(QString::from(&text));
                this.as_mut().set_failed(failed);
                this.as_mut().set_progress_value(-1.0);
                this.as_mut().set_progress_detail(QString::default());
                this.as_mut().set_busy(false);
                this.as_mut().set_progress_text(QString::default());
                this.as_mut().set_preview_json(QString::from("[]"));
                // A changed system needs a fresh preview before anything else.
                if !failed && !again.is_empty() {
                    let title = this.localizer.msg("reinstall-now");
                    let label = this.localizer.msg("gui-install");
                    let names = again.clone();
                    this.as_mut().start_preview(
                        move || prepare_preview(&Request::Install(names)),
                        &title,
                        &label,
                    );
                    return;
                }
                let profiles = profiles_value(&this.localizer).to_string();
                this.as_mut().set_profiles_json(QString::from(&profiles));
                if let Some(back) = switch_after.filter(|_| !failed) {
                    let title = this.localizer.msg("profile-switch-title");
                    let label = this.localizer.msg("gui-switch");
                    this.as_mut()
                        .start_preview(move || prepare_switch_preview(back), &title, &label);
                } else if fresh.is_some() {
                    this.as_mut().prepare_update();
                } else {
                    this.as_mut().refresh();
                }
            });
        });
    }

    fn toggle_log(mut self: Pin<&mut Self>) {
        let on = !*self.show_log();
        let mut settings = read_settings();
        settings["log_panel"] = Value::Bool(on);
        let dir = config_dir();
        if std::fs::create_dir_all(&dir).is_ok() {
            let _ = std::fs::write(dir.join("settings.json"), settings.to_string());
        }
        self.as_mut().set_show_log(on);
    }

    /// Adds a line with the time to the details panel.
    fn append_log(mut self: Pin<&mut Self>, line: &str) {
        const KEEP: usize = 1000;
        if line.trim().is_empty() {
            return;
        }
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let offset = local_offset();
        let t = (secs.cast_signed() + offset).rem_euclid(86_400);
        let stamp = format!("{:02}:{:02}:{:02}", t / 3600, t / 60 % 60, t % 60);
        let text = {
            let mut rust = self.as_mut().rust_mut();
            for part in line.lines() {
                rust.log.push(format!("{stamp}  {part}"));
            }
            let extra = rust.log.len().saturating_sub(KEEP);
            rust.log.drain(..extra);
            rust.log.join("\n")
        };
        self.as_mut().set_log_text(QString::from(&text));
    }

    fn set_tray_monochrome(mut self: Pin<&mut Self>, on: bool) {
        let mut settings = read_settings();
        settings["tray_monochrome"] = Value::Bool(on);
        let dir = config_dir();
        if std::fs::create_dir_all(&dir).is_ok() {
            let _ = std::fs::write(dir.join("settings.json"), settings.to_string());
        }
        self.as_mut().set_monochrome(on);
    }
}

/// The health page as JSON: `{"facts": [[label, value]], "kernels": [..], "suggestions": [{"id", "title", "text"}]}`.
fn health_value(l: &Localizer, info: &kura_alpm::system::Info) -> Value {
    let mut facts: Vec<Value> = Vec::new();
    if let Some(cpu) = &info.cpu {
        facts.push(json!([l.msg("health-cpu"), cpu]));
    }
    for gpu in &info.graphics {
        facts.push(json!([l.msg("health-gpu"), gpu]));
    }
    if let Some(mem) = info.memory {
        facts.push(json!([l.msg("health-memory"), l.size(mem)]));
    }
    if let Some((used, total)) = info.disk {
        let text = l.msg_args(
            "health-disk-used",
            &[("used", l.size(used).into()), ("total", l.size(total).into())],
        );
        facts.push(json!([l.msg("health-disk"), text]));
    }
    facts.push(json!([
        "",
        l.msg_args("health-cache", &[("size", l.size(info.cache).into())])
    ]));
    let (snapshots, missing) = kura_i18n::present::snapshots(l, &info.snapshots);
    facts.push(json!([l.msg("health-snapshots"), snapshots]));
    let kernels: Vec<Value> = info
        .kernels
        .iter()
        .map(|(name, version)| {
            let running = kura_core::health::is_running(version, &info.running);
            json!({"name": name, "version": version, "running": running})
        })
        .collect();
    let suggestions: Vec<Value> = kura_core::health::suggestions(&info.facts)
        .into_iter()
        .map(|s| {
            let (id, title, text) = match s {
                kura_core::health::Suggestion::BackupKernel => (
                    "backup-kernel",
                    l.msg("suggest-backup-kernel"),
                    l.msg("suggest-backup-kernel-text"),
                ),
                kura_core::health::Suggestion::Snapshots => {
                    let mut text = l.msg("suggest-snapshots-text");
                    if !missing.is_empty() {
                        text = format!("{text}\n{} {}", l.msg("snapshots-missing"), missing.join(", "));
                    }
                    ("snapshots", l.msg("suggest-snapshots"), text)
                }
                kura_core::health::Suggestion::Mirrors => {
                    ("mirrors", l.msg("suggest-mirrors"), l.msg("suggest-mirrors-text"))
                }
                kura_core::health::Suggestion::Cleanup { count } => (
                    "cleanup",
                    l.msg_args("suggest-cleanup", &[("count", count.into())]),
                    l.msg("suggest-cleanup-text"),
                ),
            };
            let install = kura_core::health::kernel_packages("linux-lts", info.facts.dkms).join(" ");
            json!({"id": id, "title": title, "text": text, "install": install})
        })
        .collect();
    // A new kernel that started fine: the old one may go.
    let suggestions: Vec<Value> = kura_alpm::kernels::replaced_kernel()
        .map(|(added, old)| {
            json!({
                "id": "old-kernel",
                "title": l.msg_args("suggest-old-kernel", &[("old", old.clone().into())]),
                "text": l.msg_args("kernel-old", &[("new", added.into()), ("old", old.clone().into())]),
                "install": old,
            })
        })
        .into_iter()
        .chain(suggestions)
        .collect();
    // Drivers first: they matter most for a computer that just got Linux.
    let suggestions: Vec<Value> = info
        .drivers
        .iter()
        .map(|d| {
            let (title, text) = present::driver(l, d);
            json!({"id": "driver", "title": title, "text": text, "install": d.packages.join(" ")})
        })
        .chain(suggestions)
        .collect();
    json!({"facts": facts, "kernels": kernels, "suggestions": suggestions})
}

/// Security findings as cards, named by the apps that use the vulnerable part.
fn security_value(l: &Localizer, findings: &[kura_core::security::Finding]) -> Value {
    Value::Array(
        findings
            .iter()
            .map(|f| {
                let who = if f.apps.is_empty() {
                    l.msg("security-system")
                } else {
                    f.apps.join(", ")
                };
                let mut lines = vec![l.msg_args(
                    "security-detail",
                    &[
                        ("package", f.package.clone().into()),
                        ("installed", f.installed.clone().into()),
                        ("kind", f.kind.clone().into()),
                    ],
                )];
                lines.push(match &f.fixed {
                    Some(v) => format!(
                        "{} · {}",
                        l.msg_args("security-fixed-in", &[("version", v.clone().into())]),
                        l.msg("security-fix")
                    ),
                    None => l.msg("security-nofix"),
                });
                lines.push(format!("{} {}", f.advisory, f.cves.join(" ")));
                json!({
                    "title": format!("{who} ({})", present::severity(l, f.severity)),
                    "lines": lines,
                    "fixed": f.fixed.is_some(),
                })
            })
            .collect(),
    )
}

/// The user's languages, most preferred first.
fn user_locales() -> Vec<kura_core::locale::Locale> {
    kura_core::locale::Locale::from_env_vars(|k| std::env::var(k).ok())
}

#[cfg(test)]
mod tests {
    use kura_core::locale::Locale;
    use kura_core::plan::PlanKind;

    use super::*;

    #[test]
    fn preview_groups_in_window_order() {
        let l = Localizer::new(&[Locale::parse("en").unwrap()]);
        let mut plan = Plan {
            kind: PlanKind::SystemUpdate,
            steps: vec![
                Step::Upgrade {
                    name: "mesa".into(),
                    from: "1".into(),
                    to: "2".into(),
                },
                Step::Upgrade {
                    name: "firefox".into(),
                    from: "151".into(),
                    to: "152".into(),
                },
                Step::Remove {
                    name: "old".into(),
                    version: "1".into(),
                    why: RemovalReason::Replaced,
                },
            ],
            actions: Vec::new(),
            builds: Vec::new(),
        };
        plan.add_cleanup([("electron36", "36")]);
        let apps = BTreeSet::from(["firefox".to_owned()]);
        let groups = preview_groups(&l, &plan, &apps);
        let titles: Vec<&str> = groups
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g["title"].as_str().unwrap())
            .collect();
        assert_eq!(
            titles,
            [
                "Apps (1)",
                "System parts (1)",
                "Replaced during the update (1)",
                "No longer needed (1)"
            ]
        );
        assert_eq!(groups[0]["items"][0]["detail"], "151 → 152");
    }
}
