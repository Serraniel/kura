// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! kura's window. Prototype: shows a read-only overview of the system.

// The cxx-qt bridge generates FFI code, including a boxed constructor.
#![allow(unsafe_code, clippy::unnecessary_box_returns)]

mod bridge;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    if let Some(mut app) = app.as_mut() {
        app.as_mut().set_application_name(&QString::from("kura"));
        app.as_mut().set_organization_name(&QString::from("kura"));
        QGuiApplication::set_desktop_file_name(&QString::from("org.kura.app"));
    }
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/org/kura/app/qml/Main.qml"));
    }
    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
