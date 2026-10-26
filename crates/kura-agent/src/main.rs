// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `kura-agent`: tray icon and notifications in the user's session.
//!
//! Reads the status kura-helper's background check writes and shows it as a
//! `StatusNotifierItem` (the tray protocol of Plasma and most desktops). When no
//! tray exists, notifications are the only output. Clicking opens the window.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use kura_core::status::{State, Status};
use kura_i18n::Localizer;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{OwnedObjectPath, Value};

const POLL: Duration = Duration::from_secs(30);

/// What the tray shows for a status.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Look {
    /// `StatusNotifierItem` status: `Passive`, `Active` or `NeedsAttention`.
    status: &'static str,
    icon: String,
    tooltip: String,
}

fn look(status: &Status, l: &Localizer, monochrome: bool) -> Look {
    let icon = if monochrome { "kura-symbolic" } else { "kura" };
    let (sni, title) = match status.state() {
        State::Security => (
            "NeedsAttention",
            kura_i18n::present::security_headline(l, &status.security),
        ),
        State::NeedsAttention => ("NeedsAttention", l.msg("tray-attention")),
        State::UpdatesReady => (
            "Active",
            l.msg_args("tray-updates", &[("count", status.updates.into())]),
        ),
        State::UpToDate => ("Passive", l.msg("tray-uptodate")),
        State::Unknown => ("Passive", l.msg("tray-unknown")),
    };
    Look {
        status: sni,
        icon: icon.to_owned(),
        tooltip: title,
    }
}

/// Whether a change from `before` to `after` is worth a notification.
fn should_notify(before: &Status, after: &Status) -> bool {
    let advisories = |s: &Status| s.security.iter().map(|f| f.advisory.clone()).collect::<Vec<_>>();
    match after.state() {
        State::Security => advisories(before) != advisories(after),
        State::NeedsAttention => before.state() != State::NeedsAttention || before.news != after.news,
        State::UpdatesReady => after.updates > before.updates || before.state() != State::UpdatesReady,
        State::UpToDate | State::Unknown => false,
    }
}

#[derive(Debug, Default, serde::Deserialize)]
struct Settings {
    #[serde(default)]
    tray_monochrome: bool,
}

fn settings_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".config")))
        .unwrap_or_default()
        .join("kura/settings.json")
}

fn read_settings() -> Settings {
    std::fs::read_to_string(settings_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn open_window() {
    let _ = std::process::Command::new("kura-gui").spawn();
}

struct Item {
    look: Look,
}

/// Icon name, icon pixmaps, title, text.
type ToolTip = (String, Vec<(i32, i32, Vec<u8>)>, String, String);

// D-Bus properties and methods always take `&self`, even for constant values.
#[allow(clippy::unused_self, clippy::unnecessary_literal_bound)]
#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl Item {
    #[zbus(property)]
    fn category(&self) -> &str {
        "SystemServices"
    }
    #[zbus(property)]
    fn id(&self) -> &str {
        "kura"
    }
    #[zbus(property)]
    fn title(&self) -> &str {
        "kura"
    }
    #[zbus(property)]
    fn status(&self) -> &str {
        self.look.status
    }
    #[zbus(property)]
    fn icon_name(&self) -> &str {
        &self.look.icon
    }
    #[zbus(property)]
    fn attention_icon_name(&self) -> &str {
        &self.look.icon
    }
    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn menu(&self) -> OwnedObjectPath {
        OwnedObjectPath::try_from("/NO_DBUSMENU").expect("valid path")
    }
    #[zbus(property)]
    fn tool_tip(&self) -> ToolTip {
        (
            self.look.icon.clone(),
            Vec::new(),
            self.look.tooltip.clone(),
            String::new(),
        )
    }

    fn activate(&self, x: i32, y: i32) {
        let _ = (x, y);
        open_window();
    }
    fn secondary_activate(&self, x: i32, y: i32) {
        let _ = (x, y);
        open_window();
    }
    fn context_menu(&self, x: i32, y: i32) {
        let _ = (x, y);
        open_window();
    }
    fn scroll(&self, delta: i32, orientation: &str) {
        let _ = (delta, orientation);
    }

    #[zbus(signal)]
    async fn new_icon(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn new_attention_icon(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn new_tool_tip(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn new_status(emitter: &SignalEmitter<'_>, status: &str) -> zbus::Result<()>;
}

const ITEM_PATH: &str = "/StatusNotifierItem";

/// Registers the item with the desktop's tray. Returns false without a tray.
fn register(conn: &zbus::blocking::Connection, name: &str) -> bool {
    conn.call_method(
        Some("org.kde.StatusNotifierWatcher"),
        "/StatusNotifierWatcher",
        Some("org.kde.StatusNotifierWatcher"),
        "RegisterStatusNotifierItem",
        &(name,),
    )
    .is_ok()
}

fn notify(conn: &zbus::blocking::Connection, l: &Localizer, look: &Look, state: State) {
    let body = if state == State::NeedsAttention {
        l.msg("notify-attention-body")
    } else {
        l.msg("notify-updates-body")
    };
    let actions = vec!["default".to_owned(), l.msg("notify-open")];
    let hints: HashMap<&str, Value<'_>> = HashMap::from([("desktop-entry", Value::from("org.kura.app"))]);
    let _ = conn.call_method(
        Some("org.freedesktop.Notifications"),
        "/org/freedesktop/Notifications",
        Some("org.freedesktop.Notifications"),
        "Notify",
        &(
            "kura",
            0u32,
            look.icon.as_str(),
            look.tooltip.as_str(),
            body.as_str(),
            actions,
            hints,
            -1i32,
        ),
    );
}

/// Opens the window when the user clicks a kura notification.
fn watch_notification_clicks(conn: &zbus::blocking::Connection) {
    let Ok(proxy) = zbus::blocking::Proxy::new(
        conn,
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    ) else {
        return;
    };
    let Ok(signals) = proxy.receive_signal("ActionInvoked") else {
        return;
    };
    std::thread::spawn(move || {
        for _ in signals {
            open_window();
        }
    });
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn main() -> Result<(), String> {
    let l = Localizer::from_env();
    let status_path = Path::new(kura_alpm::background::STATUS);
    let mut status = kura_alpm::background::read_status(status_path);
    let mut settings = read_settings();
    let name = format!("org.kde.StatusNotifierItem-{}-1", std::process::id());

    let item = Item {
        look: look(&status, &l, settings.tray_monochrome),
    };
    let conn = zbus::blocking::connection::Builder::session()
        .and_then(|b| b.name(name.as_str()))
        .and_then(|b| b.serve_at(ITEM_PATH, item))
        .and_then(zbus::blocking::connection::Builder::build)
        .map_err(|e| e.to_string())?;
    let has_tray = register(&conn, &name);
    watch_notification_clicks(&conn);
    if matches!(
        status.state(),
        State::Security | State::NeedsAttention | State::UpdatesReady
    ) {
        notify(
            &conn,
            &l,
            &look(&status, &l, settings.tray_monochrome),
            status.state(),
        );
    }

    let mut seen = modified(status_path);
    loop {
        std::thread::sleep(POLL);
        let now = modified(status_path);
        let new_settings = read_settings();
        let settings_changed = new_settings.tray_monochrome != settings.tray_monochrome;
        if now == seen && !settings_changed {
            continue;
        }
        seen = now;
        settings = new_settings;
        let fresh = kura_alpm::background::read_status(status_path);
        let new_look = look(&fresh, &l, settings.tray_monochrome);
        if should_notify(&status, &fresh) {
            notify(&conn, &l, &new_look, fresh.state());
        }
        status = fresh;
        if has_tray && let Ok(iface) = conn.object_server().interface::<_, Item>(ITEM_PATH) {
            iface.get_mut().look = new_look.clone();
            let emitter = iface.signal_emitter();
            let _ = zbus::block_on(Item::new_icon(emitter));
            let _ = zbus::block_on(Item::new_tool_tip(emitter));
            let _ = zbus::block_on(Item::new_status(emitter, new_look.status));
        }
    }
}

#[cfg(test)]
mod tests {
    use kura_core::locale::Locale;

    use super::*;

    fn en() -> Localizer {
        Localizer::new(&[Locale::parse("en").unwrap()])
    }

    fn status(updates: usize, news: &[&str], blocked: bool) -> Status {
        Status {
            checked: 1,
            updates,
            downloaded: true,
            blocked,
            news: news.iter().map(|s| (*s).to_owned()).collect(),
            security: vec![],
            ..Status::default()
        }
    }

    #[test]
    fn security_comes_first() {
        let l = en();
        let mut s = status(3, &["news"], false);
        s.security.push(kura_core::security::Finding {
            advisory: "AVG-1".into(),
            package: "ffmpeg".into(),
            installed: "1".into(),
            fixed: Some("2".into()),
            severity: kura_core::security::Severity::High,
            kind: "x".into(),
            cves: vec![],
            apps: vec!["kdenlive".into()],
        });
        let shown = look(&s, &l, false);
        assert_eq!(
            (shown.status, shown.tooltip.as_str()),
            ("NeedsAttention", "kdenlive has a known security problem")
        );
        assert!(should_notify(&status(3, &["news"], false), &s));
        assert!(!should_notify(&s, &s.clone()));
    }

    #[test]
    fn looks_follow_the_state() {
        let l = en();
        assert_eq!(look(&Status::default(), &l, false).status, "Passive");
        assert_eq!(
            look(&status(0, &[], false), &l, false).tooltip,
            "Everything is up to date"
        );
        let ready = look(&status(3, &[], false), &l, true);
        assert_eq!(
            (ready.status, ready.icon.as_str(), ready.tooltip.as_str()),
            ("Active", "kura-symbolic", "3 updates are ready")
        );
        assert_eq!(
            look(&status(3, &["news"], false), &l, false).status,
            "NeedsAttention"
        );
    }

    #[test]
    fn notifications_only_for_news_worth_telling() {
        let none = status(0, &[], false);
        let three = status(3, &[], false);
        let five = status(5, &[], false);
        let news = status(5, &["linux"], false);
        assert!(should_notify(&none, &three));
        assert!(should_notify(&three, &five));
        assert!(!should_notify(&five, &three));
        assert!(!should_notify(&three, &none));
        assert!(should_notify(&five, &news));
        assert!(!should_notify(&news, &news.clone()));
        assert!(should_notify(&news, &status(5, &["linux", "nvidia"], false)));
    }
}
