// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Only one window: a second start asks the running window to come forward
//! and ends. Without a session bus, each start simply opens a window.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Mutex, OnceLock};

use zbus::fdo::RequestNameFlags;

const NAME: &str = "org.kura.App1";
const PATH: &str = "/org/kura/App1";

/// Requests from later starts: `true` asks for the updates page.
static REQUESTS: Mutex<Option<Receiver<bool>>> = Mutex::new(None);
/// The bus name stays ours as long as this connection lives.
static BUS: OnceLock<zbus::blocking::Connection> = OnceLock::new();

struct App {
    requests: Mutex<Sender<bool>>,
}

#[zbus::interface(name = "org.kura.App1")]
impl App {
    fn show(&self, updates: bool) {
        if let Ok(requests) = self.requests.lock() {
            let _ = requests.send(updates);
        }
    }
}

/// Whether the updates page was asked for on the command line.
pub fn wants_updates() -> bool {
    std::env::args().any(|a| a == "--updates")
}

/// Takes the window's bus name. Returns false when another window has it and
/// was told to come forward: this start is finished then.
pub fn claim() -> bool {
    let (tx, rx) = channel();
    let app = App {
        requests: Mutex::new(tx),
    };
    // Asked for separately: the builder would take the name from a running window.
    let Ok(conn) = zbus::blocking::connection::Builder::session()
        .and_then(|b| b.serve_at(PATH, app))
        .and_then(zbus::blocking::connection::Builder::build)
    else {
        return true;
    };
    if let Err(zbus::Error::NameTaken) =
        conn.request_name_with_flags(NAME, RequestNameFlags::DoNotQueue.into())
    {
        return !tell_running_window();
    }
    let _ = BUS.set(conn);
    if let Ok(mut requests) = REQUESTS.lock() {
        *requests = Some(rx);
    }
    true
}

fn tell_running_window() -> bool {
    zbus::blocking::Connection::session()
        .and_then(|conn| conn.call_method(Some(NAME), PATH, Some(NAME), "Show", &(wants_updates(),)))
        .is_ok()
}

/// The requests from later starts, once.
pub fn requests() -> Option<Receiver<bool>> {
    REQUESTS.lock().ok()?.take()
}
