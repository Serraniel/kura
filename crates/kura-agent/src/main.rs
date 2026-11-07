// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `kura-agent`: tray icon and notifications in the user's session.
//!
//! Reads the status kura-helper's background check writes and shows it as a
//! `StatusNotifierItem` (the tray protocol of Plasma and most desktops). When no
//! tray exists, notifications are the only output. Clicking opens the window.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use kura_core::status::{State, Status};
use kura_i18n::Localizer;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Structure, Value};

const POLL: Duration = Duration::from_secs(30);

/// What the tray shows for a status.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Look {
    /// `StatusNotifierItem` status: `Passive` or `Active`. Never
    /// `NeedsAttention`: desktops let such an icon pulse. The badge says it.
    status: &'static str,
    icon: String,
    tooltip: String,
    /// Second tooltip line: updates next to something more important.
    detail: String,
}

/// The icon name: the logo with badges for what is going on.
fn icon_name(status: &Status, monochrome: bool) -> String {
    let warning = if status.security.iter().any(kura_core::security::Finding::urgent) {
        "-security"
    } else if status.blocked || !status.news.is_empty() || status.aur_rebuild > 0 {
        "-attention"
    } else {
        ""
    };
    let updates = if status.updates + status.aur_updates > 0 {
        "-updates"
    } else {
        ""
    };
    let symbolic = if monochrome { "-symbolic" } else { "" };
    format!("kura{warning}{updates}{symbolic}")
}

fn look(status: &Status, l: &Localizer, monochrome: bool) -> Look {
    let updates = || {
        l.msg_args(
            "tray-updates",
            &[("count", (status.updates + status.aur_updates).into())],
        )
    };
    let (sni, title) = match status.state() {
        State::Security => (
            "Active",
            kura_i18n::present::security_headline(l, &kura_core::security::urgent(&status.security)),
        ),
        State::NeedsAttention => ("Active", l.msg("tray-attention")),
        State::UpdatesReady => ("Active", updates()),
        State::UpToDate => ("Passive", l.msg("tray-uptodate")),
        State::Unknown => ("Passive", l.msg("tray-unknown")),
    };
    let more = matches!(status.state(), State::Security | State::NeedsAttention)
        && status.updates + status.aur_updates > 0;
    Look {
        status: sni,
        icon: icon_name(status, monochrome),
        tooltip: title,
        detail: if more { updates() } else { String::new() },
    }
}

/// Tells the user that kura installed security updates on its own, as
/// they allowed in the settings.
fn notify_auto_installed(
    conn: &zbus::blocking::Connection,
    sent: &Sent,
    l: &Localizer,
    look: &Look,
    fixed: usize,
) {
    let title = l.msg("notify-auto-security-title");
    let body = l.msg_args("notify-auto-security-body", &[("count", fixed.into())]);
    send(conn, sent, l, look, &title, &body);
}

/// The PCI devices present now, by their sysfs names.
fn pci_devices() -> BTreeSet<String> {
    std::fs::read_dir("/sys/bus/pci/devices")
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Driver suggestions that are new compared to `known`.
fn new_drivers(known: &[kura_core::drivers::Suggestion]) -> Vec<kura_core::drivers::Suggestion> {
    let Ok(conf) = kura_alpm::load_config() else {
        return Vec::new();
    };
    let Ok(set) = kura_alpm::installed(&conf) else {
        return Vec::new();
    };
    kura_alpm::system::drivers(&conf, &set)
        .unwrap_or_default()
        .into_iter()
        .filter(|d| !known.contains(d))
        .collect()
}

/// Whether a change from `before` to `after` is worth a notification.
fn should_notify(before: &Status, after: &Status) -> bool {
    let advisories = |s: &Status| {
        kura_core::security::urgent(&s.security)
            .into_iter()
            .map(|f| f.advisory)
            .collect::<Vec<_>>()
    };
    match after.state() {
        State::Security => advisories(before) != advisories(after),
        State::NeedsAttention => before.state() != State::NeedsAttention || before.news != after.news,
        State::UpdatesReady => {
            after.updates + after.aur_updates > before.updates + before.aur_updates
                || before.state() != State::UpdatesReady
        }
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

/// Opens the window, or brings it forward when it is open already.
fn open_window(updates: bool) {
    let mut gui = std::process::Command::new("kura-gui");
    if updates {
        gui.arg("--updates");
    }
    if let Ok(mut child) = gui.spawn() {
        std::thread::spawn(move || child.wait());
    }
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
        OwnedObjectPath::try_from(MENU_PATH).expect("valid path")
    }
    #[zbus(property)]
    fn tool_tip(&self) -> ToolTip {
        (
            self.look.icon.clone(),
            Vec::new(),
            self.look.tooltip.clone(),
            self.look.detail.clone(),
        )
    }

    fn activate(&self, x: i32, y: i32) {
        let _ = (x, y);
        open_window(false);
    }
    fn secondary_activate(&self, x: i32, y: i32) {
        let _ = (x, y);
        open_window(false);
    }
    fn context_menu(&self, x: i32, y: i32) {
        let _ = (x, y);
        open_window(false);
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
const MENU_PATH: &str = "/MenuBar";

const MENU_STATUS: i32 = 1;
const MENU_OPEN: i32 = 2;
const MENU_CHECK: i32 = 3;
const MENU_SEPARATOR: i32 = 4;
const MENU_QUIT: i32 = 5;

type Properties = HashMap<String, OwnedValue>;
/// Id, properties and children of a menu entry.
type MenuNode = (i32, Properties, Vec<OwnedValue>);

/// A menu entry's properties.
fn entry(pairs: &[(&str, Value<'_>)]) -> Properties {
    pairs
        .iter()
        .filter_map(|(k, v)| Some(((*k).to_owned(), v.try_to_owned().ok()?)))
        .collect()
}

/// The right-click menu of the tray icon, for `com.canonical.dbusmenu`.
struct Menu {
    /// The first line: what is going on right now.
    status: String,
    open: String,
    check: String,
    quit: String,
    revision: u32,
}

impl Menu {
    fn new(l: &Localizer, status: String) -> Self {
        Self {
            status,
            open: l.msg("notify-open"),
            check: l.msg("gui-check-now"),
            quit: l.msg("tray-quit"),
            revision: 1,
        }
    }

    fn entries(&self) -> Vec<(i32, Properties)> {
        let label = |text: &str| ("label", Value::from(text.replace('_', "__")));
        vec![
            (
                MENU_STATUS,
                entry(&[label(&self.status), ("enabled", Value::from(false))]),
            ),
            (MENU_OPEN, entry(&[label(&self.open)])),
            (MENU_CHECK, entry(&[label(&self.check)])),
            (MENU_SEPARATOR, entry(&[("type", Value::from("separator"))])),
            (MENU_QUIT, entry(&[label(&self.quit)])),
        ]
    }

    /// What a click on `id` does; false when the id does nothing.
    fn clicked(id: i32) -> bool {
        match id {
            MENU_OPEN => open_window(false),
            MENU_CHECK => open_window(true),
            MENU_QUIT => std::process::exit(0),
            _ => return false,
        }
        true
    }
}

// D-Bus properties and methods always take `&self`, even for constant values,
// and get their arguments by value, needed or not.
#[allow(
    clippy::unused_self,
    clippy::unnecessary_literal_bound,
    clippy::needless_pass_by_value
)]
#[zbus::interface(name = "com.canonical.dbusmenu")]
impl Menu {
    #[zbus(property)]
    fn version(&self) -> u32 {
        3
    }
    #[zbus(property)]
    fn text_direction(&self) -> &str {
        "ltr"
    }
    #[zbus(property)]
    fn status(&self) -> &str {
        "normal"
    }
    #[zbus(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        Vec::new()
    }

    /// The menu is flat, so every request gets the whole of it.
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<String>,
    ) -> (u32, MenuNode) {
        let _ = (parent_id, recursion_depth, property_names);
        let children = self
            .entries()
            .into_iter()
            .filter_map(|(id, properties)| {
                let child: Value<'_> = Structure::from((id, properties, Vec::<OwnedValue>::new())).into();
                child.try_to_owned().ok()
            })
            .collect();
        let root = entry(&[("children-display", Value::from("submenu"))]);
        (self.revision, (0, root, children))
    }

    fn get_group_properties(&self, ids: Vec<i32>, property_names: Vec<String>) -> Vec<(i32, Properties)> {
        let _ = property_names;
        self.entries()
            .into_iter()
            .filter(|(id, _)| ids.is_empty() || ids.contains(id))
            .collect()
    }

    fn get_property(&self, id: i32, name: &str) -> zbus::fdo::Result<OwnedValue> {
        self.entries()
            .into_iter()
            .find(|(entry, _)| *entry == id)
            .and_then(|(_, mut properties)| properties.remove(name))
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs(format!("{id} {name}")))
    }

    fn event(&self, id: i32, event_id: &str, data: OwnedValue, timestamp: u32) {
        let _ = (data, timestamp);
        if event_id == "clicked" {
            Self::clicked(id);
        }
    }

    fn event_group(&self, events: Vec<(i32, String, OwnedValue, u32)>) -> Vec<i32> {
        for (id, event_id, ..) in &events {
            if event_id == "clicked" {
                Self::clicked(*id);
            }
        }
        Vec::new()
    }

    fn about_to_show(&self, id: i32) -> bool {
        let _ = id;
        false
    }

    fn about_to_show_group(&self, ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        let _ = ids;
        (Vec::new(), Vec::new())
    }

    #[zbus(signal)]
    async fn layout_updated(emitter: &SignalEmitter<'_>, revision: u32, parent: i32) -> zbus::Result<()>;
}

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

/// The ids of the notifications kura sent.
type Sent = Arc<Mutex<BTreeSet<u32>>>;

fn notify(conn: &zbus::blocking::Connection, sent: &Sent, l: &Localizer, look: &Look, state: State) {
    let body = if state == State::NeedsAttention {
        l.msg("notify-attention-body")
    } else {
        l.msg("notify-updates-body")
    };
    send(conn, sent, l, look, &look.tooltip, &body);
}

fn send(conn: &zbus::blocking::Connection, sent: &Sent, l: &Localizer, look: &Look, title: &str, body: &str) {
    let actions = vec!["default".to_owned(), l.msg("notify-open")];
    let hints: HashMap<&str, Value<'_>> = HashMap::from([("desktop-entry", Value::from("org.kura.app"))]);
    let reply = conn.call_method(
        Some("org.freedesktop.Notifications"),
        "/org/freedesktop/Notifications",
        Some("org.freedesktop.Notifications"),
        "Notify",
        &(
            "kura",
            0u32,
            look.icon.as_str(),
            title,
            body,
            actions,
            hints,
            -1i32,
        ),
    );
    if let Ok(id) = reply.and_then(|m| m.body().deserialize::<u32>())
        && let Ok(mut sent) = sent.lock()
    {
        sent.insert(id);
    }
}

/// Opens the window when the user clicks a kura notification. The desktop
/// announces the clicks on every program's notifications to everyone.
fn watch_notification_clicks(conn: &zbus::blocking::Connection, sent: Sent) {
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
        for signal in signals {
            let clicked = signal.body().deserialize::<(u32, String)>();
            if clicked.is_ok_and(|(id, _)| sent.lock().is_ok_and(|s| s.contains(&id))) {
                open_window(false);
            }
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
    let menu = Menu::new(&l, look(&status, &l, settings.tray_monochrome).tooltip);
    let conn = zbus::blocking::connection::Builder::session()
        .and_then(|b| b.name(name.as_str()))
        .and_then(|b| b.serve_at(ITEM_PATH, item))
        .and_then(|b| b.serve_at(MENU_PATH, menu))
        .and_then(zbus::blocking::connection::Builder::build)
        .map_err(|e| e.to_string())?;
    let has_tray = register(&conn, &name);
    let sent = Sent::default();
    watch_notification_clicks(&conn, Arc::clone(&sent));
    if matches!(
        status.state(),
        State::Security | State::NeedsAttention | State::UpdatesReady
    ) {
        notify(
            &conn,
            &sent,
            &l,
            &look(&status, &l, settings.tray_monochrome),
            status.state(),
        );
    }

    let mut seen = modified(status_path);
    // New hardware: a driver suggestion that was not there before is worth a message.
    let mut devices = pci_devices();
    let mut drivers = new_drivers(&[]);
    loop {
        std::thread::sleep(POLL);
        let now_devices = pci_devices();
        if now_devices != devices {
            devices = now_devices;
            let fresh = new_drivers(&drivers);
            for d in &fresh {
                let (title, text) = kura_i18n::present::driver(&l, d);
                send(
                    &conn,
                    &sent,
                    &l,
                    &look(&status, &l, settings.tray_monochrome),
                    &title,
                    &text,
                );
            }
            drivers.extend(fresh);
        }
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
        if fresh.auto_installed > status.auto_installed {
            notify_auto_installed(&conn, &sent, &l, &new_look, fresh.auto_fixed);
        }
        if should_notify(&status, &fresh) {
            notify(&conn, &sent, &l, &new_look, fresh.state());
        }
        status = fresh;
        if has_tray && let Ok(iface) = conn.object_server().interface::<_, Item>(ITEM_PATH) {
            iface.get_mut().look = new_look.clone();
            let emitter = iface.signal_emitter();
            let _ = zbus::block_on(Item::new_icon(emitter));
            let _ = zbus::block_on(Item::new_tool_tip(emitter));
            let _ = zbus::block_on(Item::new_status(emitter, new_look.status));
        }
        if has_tray && let Ok(iface) = conn.object_server().interface::<_, Menu>(MENU_PATH) {
            let revision = {
                let mut menu = iface.get_mut();
                menu.status.clone_from(&new_look.tooltip);
                menu.revision += 1;
                menu.revision
            };
            let _ = zbus::block_on(Menu::layout_updated(iface.signal_emitter(), revision, 0));
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
            ("Active", "kdenlive has a known security problem")
        );
        assert_eq!(shown.icon, "kura-security-updates");
        assert_eq!(shown.detail, "3 updates are ready");
        assert!(should_notify(&status(3, &["news"], false), &s));
        assert!(!should_notify(&s, &s.clone()));

        // Low risk with no fix: no badge and no notification, until a fix exists.
        let before = status(0, &[], false);
        let mut calm = before.clone();
        calm.security = s.security.clone();
        calm.security[0].fixed = None;
        calm.security[0].severity = kura_core::security::Severity::Low;
        assert_eq!(look(&calm, &l, false).icon, "kura");
        assert!(!should_notify(&before, &calm));
        let mut fixable = calm.clone();
        fixable.security[0].fixed = Some("2".into());
        assert_eq!(look(&fixable, &l, false).icon, "kura-security");
        assert!(should_notify(&calm, &fixable));
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
            ("Active", "kura-updates-symbolic", "3 updates are ready")
        );
        assert_eq!(ready.detail, "");
        let attention = look(&status(3, &["news"], false), &l, false);
        assert_eq!(
            (attention.status, attention.icon.as_str()),
            ("Active", "kura-attention-updates")
        );
        let calm = look(&status(0, &[], false), &l, true);
        assert_eq!(calm.icon, "kura-symbolic");
        assert_eq!(look(&status(0, &[], true), &l, false).icon, "kura-attention");
    }

    #[test]
    fn menu_shows_the_status_and_its_actions() {
        let menu = Menu::new(&en(), "a_b".to_owned());
        let entries = menu.entries();
        let ids: Vec<i32> = entries.iter().map(|(id, _)| *id).collect();
        assert_eq!(
            ids,
            [MENU_STATUS, MENU_OPEN, MENU_CHECK, MENU_SEPARATOR, MENU_QUIT]
        );
        let label = |i: usize| String::try_from(entries[i].1["label"].try_clone().unwrap()).unwrap();
        // A single underscore would underline the next letter.
        assert_eq!(label(0), "a__b");
        assert_eq!(label(1), "Open kura");
        assert!(!Menu::clicked(MENU_STATUS));
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
        let mut aur = three.clone();
        aur.aur_updates = 2;
        assert!(should_notify(&three, &aur));
        assert_eq!(look(&aur, &en(), false).tooltip, "5 updates are ready");
    }
}
