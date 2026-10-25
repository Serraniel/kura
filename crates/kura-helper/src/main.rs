// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `kura-helper`: the only part of kura that changes the system.
//!
//! - `kura-helper serve` is started by D-Bus on demand, runs approved plans
//!   after a polkit check and exits after a minute without work.
//! - `kura-helper record` is run by a pacman hook after every transaction and
//!   appends the new generation to the history.
//! - `kura-helper check` is run by a timer: finds and downloads updates
//!   without installing them and writes the status for the tray.

use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kura_alpm::apply::{ApplyError, Event};
use kura_alpm::history::{CAUSE, JOURNAL};
use kura_core::plan::Plan;
use kura_helper::{BUS_NAME, OBJECT_PATH, Reply, action_for, cause_for};
use zbus::message::Header;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::Value;

const IDLE_EXIT: Duration = Duration::from_secs(60);

#[zbus::proxy(
    interface = "org.freedesktop.PolicyKit1.Authority",
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority"
)]
trait Authority {
    #[allow(clippy::type_complexity)]
    fn check_authorization(
        &self,
        subject: &(&str, HashMap<&str, Value<'_>>),
        action_id: &str,
        details: HashMap<&str, &str>,
        flags: u32,
        cancellation_id: &str,
    ) -> zbus::Result<(bool, bool, HashMap<String, String>)>;
}

/// Asks polkit whether the D-Bus caller may run this action. The user may be
/// asked for a password.
async fn authorized(conn: &zbus::Connection, sender: &str, action: &str) -> bool {
    const ALLOW_USER_INTERACTION: u32 = 1;
    let Ok(authority) = AuthorityProxy::new(conn).await else {
        return false;
    };
    let subject = ("system-bus-name", HashMap::from([("name", Value::from(sender))]));
    authority
        .check_authorization(&subject, action, HashMap::new(), ALLOW_USER_INTERACTION, "")
        .await
        .is_ok_and(|(ok, _, _)| ok)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

struct Helper {
    busy: Arc<AtomicBool>,
    last_activity: Arc<AtomicU64>,
}

/// Runs a plan with the cause file in place, translating errors into replies.
fn run(plan: &Plan, events: &mpsc::Sender<Event>) -> Reply {
    let conf = match kura_alpm::load_config() {
        Ok(c) => c,
        Err(e) => {
            return Reply::Failed {
                message: e.to_string(),
            };
        }
    };
    let cause = Path::new(CAUSE);
    if let Some(dir) = cause.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(cause, cause_for(plan.kind));
    let result = if matches!(plan.kind, kura_core::plan::PlanKind::Restore { .. }) {
        kura_alpm::apply::apply_restore(&conf, plan, Path::new(JOURNAL), events)
    } else {
        kura_alpm::apply::apply(&conf, plan, events)
    };
    let _ = std::fs::remove_file(cause);
    match result {
        Ok(()) => Reply::Done,
        Err(ApplyError::Changed(plan)) => Reply::Changed { plan: *plan },
        Err(ApplyError::Blocked(problems)) => Reply::Blocked { problems },
        Err(ApplyError::Locked) => Reply::Locked,
        Err(e) => Reply::Failed {
            message: e.to_string(),
        },
    }
}

#[zbus::interface(name = "org.kura.Helper1")]
impl Helper {
    async fn apply(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] conn: &zbus::Connection,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        plan: String,
    ) -> String {
        self.last_activity.store(now(), Ordering::Relaxed);
        let reply = self.handle(&header, conn, &emitter, &plan).await;
        self.last_activity.store(now(), Ordering::Relaxed);
        serde_json::to_string(&reply).unwrap_or_else(|_| r#"{"result":"failed","message":"reply"}"#.into())
    }

    async fn rank_mirrors(
        &self,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] conn: &zbus::Connection,
    ) -> String {
        self.last_activity.store(now(), Ordering::Relaxed);
        let reply = match header.sender() {
            Some(sender) if authorized(conn, sender.as_str(), "org.kura.mirrors").await => {
                if self.busy.swap(true, Ordering::AcqRel) {
                    Reply::Busy
                } else {
                    let reply = blocking::unblock(rank).await;
                    self.busy.store(false, Ordering::Release);
                    reply
                }
            }
            _ => Reply::NotAuthorized,
        };
        self.last_activity.store(now(), Ordering::Relaxed);
        serde_json::to_string(&reply).unwrap_or_else(|_| r#"{"result":"failed","message":"reply"}"#.into())
    }

    #[zbus(signal)]
    async fn progress(emitter: &SignalEmitter<'_>, event: &str) -> zbus::Result<()>;
}

impl Helper {
    async fn handle(
        &self,
        header: &Header<'_>,
        conn: &zbus::Connection,
        emitter: &SignalEmitter<'_>,
        plan: &str,
    ) -> Reply {
        let plan: Plan = match serde_json::from_str(plan) {
            Ok(p) => p,
            Err(e) => {
                return Reply::Failed {
                    message: format!("not a plan: {e}"),
                };
            }
        };
        let Some(sender) = header.sender() else {
            return Reply::NotAuthorized;
        };
        if !authorized(conn, sender.as_str(), action_for(plan.kind)).await {
            return Reply::NotAuthorized;
        }
        if self.busy.swap(true, Ordering::AcqRel) {
            return Reply::Busy;
        }
        let (tx, rx) = mpsc::channel::<Event>();
        let work = blocking::unblock(move || run(&plan, &tx));
        let forward = async {
            let rx = Arc::new(std::sync::Mutex::new(rx));
            loop {
                let rx = Arc::clone(&rx);
                let next = blocking::unblock(move || rx.lock().map_or(None, |r| r.recv().ok())).await;
                let Some(event) = next else { break };
                if let Ok(json) = serde_json::to_string(&event) {
                    let _ = Helper::progress(emitter, &json).await;
                }
            }
        };
        // `forward` ends when `run` drops its sender.
        let (reply, ()) = futures_lite::future::zip(work, forward).await;
        self.busy.store(false, Ordering::Release);
        reply
    }
}

fn serve() -> Result<(), String> {
    let busy = Arc::new(AtomicBool::new(false));
    let last_activity = Arc::new(AtomicU64::new(now()));
    let helper = Helper {
        busy: Arc::clone(&busy),
        last_activity: Arc::clone(&last_activity),
    };
    let _conn = zbus::blocking::connection::Builder::system()
        .and_then(|b| b.name(BUS_NAME))
        .and_then(|b| b.serve_at(OBJECT_PATH, helper))
        .and_then(zbus::blocking::connection::Builder::build)
        .map_err(|e| e.to_string())?;
    loop {
        std::thread::sleep(Duration::from_secs(5));
        let idle = now().saturating_sub(last_activity.load(Ordering::Relaxed));
        if !busy.load(Ordering::Acquire) && idle >= IDLE_EXIT.as_secs() {
            return Ok(());
        }
    }
}

/// Today's date for file names, `YYYY-MM-DD`.
fn today() -> String {
    kura_sbom::rfc3339(u64::try_from(kura_alpm::background::now()).unwrap_or(0))[..10].to_owned()
}

fn rank() -> Reply {
    match kura_alpm::mirrors::rank(Path::new(kura_alpm::mirrors::MIRRORLIST), &today()) {
        Ok(r) => {
            let _ = std::fs::write(MIRRORS_RANKED, today());
            Reply::Mirrors {
                count: r.count,
                backup: r.backup.display().to_string(),
            }
        }
        Err(e) => Reply::Failed {
            message: e.to_string(),
        },
    }
}

/// Written after each ranking; the background check ranks again after 30 days.
const MIRRORS_RANKED: &str = "/var/lib/kura/mirrors-ranked";

/// Run by kura-check.timer: looks for updates, downloads them and writes the status.
fn check() -> Result<(), String> {
    use kura_alpm::background::{CHECK_DB, STATUS, check, fetch_news, news_since, now, write_status};
    let stale = std::fs::metadata(MIRRORS_RANKED)
        .and_then(|m| m.modified())
        .map_or(true, |t| {
            t.elapsed().unwrap_or_default() > Duration::from_secs(30 * 86_400)
        });
    if stale && let Reply::Failed { message } = rank() {
        eprintln!("kura: mirrors not ranked: {message}");
    }
    let conf = kura_alpm::load_config().map_err(|e| e.to_string())?;
    let news = fetch_news();
    let since = news_since(Path::new(JOURNAL), now());
    let status =
        check(&conf, Path::new(CHECK_DB), true, news.as_deref(), since).map_err(|e| e.to_string())?;
    write_status(Path::new(STATUS), &status).map_err(|e| e.to_string())?;
    eprintln!(
        "kura: {} updates, downloaded: {}",
        status.updates, status.downloaded
    );
    Ok(())
}

fn record() -> Result<(), String> {
    let conf = kura_alpm::load_config().map_err(|e| e.to_string())?;
    match kura_alpm::history::record(&conf, Path::new(JOURNAL), Path::new(CAUSE))
        .map_err(|e| e.to_string())?
    {
        Some(r) => eprintln!("kura: saved generation {}", r.id),
        None => eprintln!("kura: nothing changed"),
    }
    Ok(())
}

fn main() -> ExitCode {
    let result = match std::env::args().nth(1).as_deref() {
        Some("serve") => serve(),
        Some("record") => record(),
        Some("check") => check(),
        _ => Err("usage: kura-helper serve | record | check".to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // A failed record must never fail the pacman transaction.
            eprintln!("kura: {e}");
            ExitCode::FAILURE
        }
    }
}
