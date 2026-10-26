// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The object QML talks to. Slow work (preparing, installing) runs in
//! background threads and reports back through the Qt event loop, so the
//! window never freezes. Changes go through kura-helper only.

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
        type Kura = super::KuraRust;
    }

    extern "RustQt" {
        /// Translated message for a Fluent ID.
        #[qinvokable]
        fn tr(self: &Kura, id: &QString) -> QString;

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

fn check_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("kura")
        .join("checkdb")
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
            "items": steps.iter().map(|s| json!({"name": s.package(), "detail": present::step_detail(s)})).collect::<Vec<_>>(),
        })
    };
    let all = [
        ("check-group-apps", groups.apps.as_slice()),
        ("check-group-system", system.as_slice()),
        ("check-group-replaced", replaced.as_slice()),
        ("check-group-cleanup", groups.cleanup.as_slice()),
    ];
    Value::Array(
        all.iter()
            .filter(|(_, s)| !s.is_empty())
            .map(|(id, s)| group(id, s))
            .collect(),
    )
}

struct Prepared {
    plan: Plan,
    preview: String,
    news: String,
    summary: String,
    headline: String,
}

/// Runs in a background thread.
fn prepare_preview(request: &Request) -> Result<Prepared, String> {
    let l = Localizer::from_env();
    let fail = |e: kura_alpm::Error| l.msg_args("error-check", &[("reason", e.to_string().into())]);
    let conf = kura_alpm::load_config().map_err(fail)?;
    let (outcome, repair) = kura_alpm::prepare::prepare_with_repair(
        &conf,
        Some(&check_dir()),
        request,
        !matches!(request, Request::Remove(_)),
        &user_locales(),
    )
    .map_err(fail)?;
    let mut plan = match outcome.plan {
        Ok(plan) => plan,
        Err(problems) => {
            let mut text = l.msg(if matches!(request, Request::Update) {
                "check-blocked"
            } else {
                "change-blocked"
            });
            for p in &problems {
                text.push_str("\n• ");
                text.push_str(&kura_helper::text::problem(&l, p));
            }
            return Err(text);
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
    let solved = repair.map(|r| {
        format!(
            "{}\n{}",
            l.msg_args("solver-found", &[("count", r.remove.len().into())]),
            l.msg("solver-tested")
        )
    });
    let headline = if let Some(text) = solved {
        text
    } else if !matches!(request, Request::Update) {
        String::new()
    } else if outcome.available.is_empty() {
        l.msg("check-none")
    } else {
        l.msg_args("check-found", &[("count", outcome.available.len().into())])
    };
    let summary = l.msg_args(
        "check-sizes",
        &[
            ("download", l.size(outcome.download_size).into()),
            ("delta", l.size_delta(outcome.installed_delta).into()),
        ],
    );
    Ok(Prepared {
        preview: preview_groups(&l, &plan, &apps).to_string(),
        news: news.to_string(),
        summary,
        headline,
        plan,
    })
}

impl qobject::Kura {
    fn tr(&self, id: &QString) -> QString {
        QString::from(&self.localizer.msg(&id.to_string()))
    }

    fn refresh(mut self: Pin<&mut Self>) {
        let l = &self.localizer;
        let distro = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|t| {
                t.lines()
                    .find_map(|l| l.strip_prefix("NAME=").map(|v| v.trim_matches('"').to_owned()))
            })
            .unwrap_or_else(|| "Linux".to_owned());

        let status = kura_alpm::background::read_status(Path::new(kura_alpm::background::STATUS));
        let now = kura_alpm::background::now();
        let (state, text) = match status.state() {
            State::Security => (4, present::security_headline(l, &status.security)),
            State::Unknown => (0, l.msg("tray-unknown")),
            State::UpToDate => (1, l.msg("tray-uptodate")),
            State::UpdatesReady => (2, l.msg_args("tray-updates", &[("count", status.updates.into())])),
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
                            .map(|r| {
                                json!({"id": r.id, "when": present::when(l, r.created, now), "text": present::record_text(l, r)})
                            })
                            .collect(),
                    )
                },
            )
            .to_string();
        let security = security_value(l, &status.security).to_string();
        let loaded = kura_alpm::load_config().and_then(|conf| kura_alpm::installed(&conf));
        let error_text = loaded
            .as_ref()
            .err()
            .map(|e| l.msg_args("error-database", &[("reason", e.to_string().into())]))
            .unwrap_or_default();

        self.as_mut().set_distro(QString::from(&distro));
        self.as_mut().set_state(state);
        self.as_mut().set_status_text(QString::from(&text));
        self.as_mut().set_last_check(QString::from(&last_check));
        self.as_mut().set_history_json(QString::from(&history));
        self.as_mut().set_security_json(QString::from(&security));
        self.as_mut().set_error(QString::from(&error_text));
        if let Ok(set) = loaded {
            let explicit = set.iter().filter(|p| p.reason == InstallReason::Explicit).count();
            let foreign = set.iter().filter(|p| p.repo.is_none()).count();
            self.as_mut().set_packages(count(set.len()));
            self.as_mut().set_explicit_count(count(explicit));
            self.as_mut().set_foreign_count(count(foreign));
            self.as_mut().set_orphan_count(count(set.orphans().len()));
        }
    }

    fn prepare_update(self: Pin<&mut Self>) {
        let title = self.localizer.msg("nav-updates");
        let label = self.localizer.msg("gui-install");
        self.start_preview(Request::Update, &title, &label);
    }

    fn prepare_install(self: Pin<&mut Self>, names: &QString) {
        let names: Vec<String> = names.to_string().split_whitespace().map(str::to_owned).collect();
        let title = self
            .localizer
            .msg_args("gui-install-title", &[("name", names.join(", ").into())]);
        let label = self.localizer.msg("gui-install");
        self.start_preview(Request::Install(names), &title, &label);
    }

    fn prepare_remove(self: Pin<&mut Self>, name: &QString) {
        let name = name.to_string();
        let title = self
            .localizer
            .msg_args("gui-remove-title", &[("name", name.clone().into())]);
        let label = self.localizer.msg("gui-remove");
        self.start_preview(Request::Remove(vec![name]), &title, &label);
    }

    /// Prepares any request in a background thread and shows it as the preview.
    fn start_preview(mut self: Pin<&mut Self>, request: Request, title: &str, label: &str) {
        if *self.busy() {
            return;
        }
        let running = self.localizer.msg("check-running");
        self.as_mut().set_preview_title(QString::from(title));
        self.as_mut().set_action_label(QString::from(label));
        self.as_mut().set_busy(true);
        self.as_mut().set_can_install(false);
        self.as_mut().set_message(QString::default());
        self.as_mut().set_preview_json(QString::from("[]"));
        self.as_mut().set_news_json(QString::from("[]"));
        self.as_mut().set_progress_text(QString::from(&running));
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let result = prepare_preview(&request);
            let _ = qt.queue(move |mut this| {
                match result {
                    Ok(p) => {
                        let has_changes = !p.plan.steps.is_empty();
                        this.as_mut().set_preview_json(QString::from(&p.preview));
                        this.as_mut().set_news_json(QString::from(&p.news));
                        this.as_mut().set_summary_text(QString::from(&p.summary));
                        this.as_mut().set_progress_text(QString::from(&p.headline));
                        this.as_mut().set_can_install(has_changes);
                        this.as_mut().rust_mut().pending = Some(p.plan);
                    }
                    Err(text) => {
                        this.as_mut().set_message(QString::from(&text));
                        this.as_mut().set_progress_text(QString::default());
                    }
                }
                this.as_mut().set_busy(false);
            });
        });
    }

    fn refresh_health(mut self: Pin<&mut Self>) {
        let value = kura_alpm::load_config()
            .and_then(|conf| kura_alpm::system::collect(&conf))
            .map_or_else(|_| json!({}), |info| health_value(&self.localizer, &info));
        self.as_mut().set_health_json(QString::from(&value.to_string()));
    }

    fn search(mut self: Pin<&mut Self>, query: &QString) {
        let hits = kura_alpm::load_config()
            .and_then(|conf| kura_alpm::system::search(&conf, &query.to_string(), 40))
            .unwrap_or_default();
        let value: Vec<Value> = hits
            .iter()
            .map(|h| json!({"name": h.name, "version": h.version, "description": h.description, "installed": h.installed}))
            .collect();
        self.as_mut()
            .set_results_json(QString::from(&Value::Array(value).to_string()));
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
                this.as_mut().set_message(QString::from(&text));
                this.as_mut().set_busy(false);
                this.as_mut().refresh_health();
            });
        });
    }

    fn prepare_restore(mut self: Pin<&mut Self>, generation: i32) {
        if *self.busy() {
            return;
        }
        let Ok(target) = u64::try_from(generation) else {
            return;
        };
        let l = &self.localizer;
        let journal = kura_alpm::history::load(Path::new(kura_alpm::history::JOURNAL));
        let conf = kura_alpm::load_config();
        let prepared = match (journal, conf) {
            (Ok(journal), Ok(conf)) => {
                let generation = journal.generation(target);
                match (kura_alpm::restore::plan(&journal, target), generation) {
                    (Some(plan), Some(generation)) if !plan.steps.is_empty() => {
                        match kura_alpm::restore::sources(&conf.cache_dir, &plan, &generation) {
                            Ok(_) => {
                                let apps = kura_alpm::apps(&conf).unwrap_or_default();
                                let when = present::when(l, generation.created, kura_alpm::background::now());
                                Ok((
                                    plan.clone(),
                                    preview_groups(l, &plan, &apps).to_string(),
                                    l.msg_args("undo-title", &[("id", target.into()), ("when", when.into())]),
                                ))
                            }
                            Err(problems) => Err(problems
                                .iter()
                                .map(|p| kura_helper::text::problem(l, p))
                                .collect::<Vec<_>>()
                                .join("\n")),
                        }
                    }
                    (Some(_), Some(_)) => Err(l.msg("undo-nothing")),
                    _ => Err(l.msg_args("undo-unknown", &[("id", target.into())])),
                }
            }
            (Err(e), _) | (_, Err(e)) => {
                Err(l.msg_args("error-database", &[("reason", e.to_string().into())]))
            }
        };
        let limits = l.msg("undo-limits");
        match prepared {
            Ok((plan, preview, title)) => {
                let label = self.localizer.msg("gui-go-back");
                self.as_mut().set_preview_title(QString::from(&title));
                self.as_mut().set_action_label(QString::from(&label));
                self.as_mut().set_preview_json(QString::from(&preview));
                self.as_mut().set_news_json(QString::from("[]"));
                self.as_mut().set_summary_text(QString::from(&limits));
                self.as_mut().set_progress_text(QString::default());
                self.as_mut().set_message(QString::default());
                self.as_mut().set_can_install(true);
                self.as_mut().rust_mut().pending = Some(plan);
            }
            Err(text) => {
                self.as_mut().set_preview_json(QString::from("[]"));
                self.as_mut().set_can_install(false);
                self.as_mut().set_message(QString::from(&text));
            }
        }
    }

    fn install_update(mut self: Pin<&mut Self>) {
        let Some(plan) = self.as_mut().rust_mut().pending.take() else {
            return;
        };
        self.as_mut().set_busy(true);
        self.as_mut().set_can_install(false);
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let l = Localizer::from_env();
            let events = qt.clone();
            let result = kura_helper::apply(&plan, |event| {
                let line = kura_helper::text::event(&l, &event);
                let _ = events.queue(move |mut this| this.as_mut().set_progress_text(QString::from(&line)));
            });
            let (text, fresh) = match result {
                Ok(reply) => {
                    let fresh = match &reply {
                        kura_helper::Reply::Changed { plan } => Some(plan.clone()),
                        _ => None,
                    };
                    (kura_helper::text::reply(&l, &reply), fresh)
                }
                Err(e) => (
                    l.msg_args("error-helper", &[("reason", e.to_string().into())]),
                    None,
                ),
            };
            let _ = qt.queue(move |mut this| {
                this.as_mut().set_message(QString::from(&text));
                this.as_mut().set_busy(false);
                this.as_mut().set_progress_text(QString::default());
                this.as_mut().set_preview_json(QString::from("[]"));
                // A changed system needs a fresh preview before anything else.
                if fresh.is_some() {
                    this.as_mut().prepare_update();
                } else {
                    this.as_mut().refresh();
                }
            });
        });
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
    let kernels: Vec<Value> = info
        .kernels
        .iter()
        .map(|(name, version)| {
            let running = info
                .running
                .starts_with(&version.replace(".arch", "-arch").replace('-', "."))
                || info
                    .running
                    .replace('-', ".")
                    .starts_with(&version.replace('-', "."));
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
                kura_core::health::Suggestion::Snapshots => (
                    "snapshots",
                    l.msg("suggest-snapshots"),
                    l.msg("suggest-snapshots-text"),
                ),
                kura_core::health::Suggestion::Mirrors => {
                    ("mirrors", l.msg("suggest-mirrors"), l.msg("suggest-mirrors-text"))
                }
                kura_core::health::Suggestion::Cleanup { count } => (
                    "cleanup",
                    l.msg_args("suggest-cleanup", &[("count", count.into())]),
                    l.msg("suggest-cleanup-text"),
                ),
            };
            json!({"id": id, "title": title, "text": text})
        })
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
