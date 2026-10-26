// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

use cxx_qt_build::{CxxQtBuilder, QResource, QResources};
use qt_build_utils::QResourceFile;

fn main() {
    // The icons are built in, so the window has them even when the icon
    // theme does not (for example when run from the source tree).
    let icons = QResource::new()
        .prefix("/kura")
        .file(QResourceFile::new("../../packaging/icons/kura.svg").alias("kura.svg"))
        .file(QResourceFile::new("../../packaging/icons/kura-symbolic.svg").alias("kura-symbolic.svg"));
    CxxQtBuilder::new_qml_module(cxx_qt_build::QmlModule::new("org.kura.app").qml_files(["qml/Main.qml"]))
        .files(["src/bridge.rs"])
        .qrc_resources(QResources::new().resource(icons))
        .build();
}
