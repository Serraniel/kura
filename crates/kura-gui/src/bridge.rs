// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! The object QML talks to. It only reads; changes will go through plans and
//! the privileged helper.

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
        type Kura = super::KuraRust;
    }

    extern "RustQt" {
        /// Translated message for a Fluent ID.
        #[qinvokable]
        fn tr(self: &Kura, id: &QString) -> QString;

        /// Reads the package database again.
        #[qinvokable]
        fn refresh(self: Pin<&mut Kura>);
    }
}

use std::pin::Pin;

use cxx_qt_lib::QString;
use kura_core::package::InstallReason;
use kura_i18n::Localizer;

pub struct KuraRust {
    distro: QString,
    packages: i32,
    explicit_count: i32,
    foreign_count: i32,
    orphan_count: i32,
    error: QString,
    localizer: Localizer,
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
            localizer: Localizer::from_env(),
        }
    }
}

fn count(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

impl qobject::Kura {
    fn tr(&self, id: &QString) -> QString {
        QString::from(&self.localizer.msg(&id.to_string()))
    }

    fn refresh(mut self: Pin<&mut Self>) {
        let distro = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|t| {
                t.lines()
                    .find_map(|l| l.strip_prefix("NAME=").map(|v| v.trim_matches('"').to_owned()))
            })
            .unwrap_or_else(|| "Linux".to_owned());
        self.as_mut().set_distro(QString::from(&distro));
        match kura_alpm::load_config().and_then(|conf| kura_alpm::installed(&conf)) {
            Ok(set) => {
                let explicit = set.iter().filter(|p| p.reason == InstallReason::Explicit).count();
                let foreign = set.iter().filter(|p| p.repo.is_none()).count();
                self.as_mut().set_packages(count(set.len()));
                self.as_mut().set_explicit_count(count(explicit));
                self.as_mut().set_foreign_count(count(foreign));
                self.as_mut().set_orphan_count(count(set.orphans().len()));
                self.as_mut().set_error(QString::default());
            }
            Err(e) => {
                let msg = self
                    .localizer
                    .msg_args("error-database", &[("reason", e.to_string().into())]);
                self.as_mut().set_error(QString::from(&msg));
            }
        }
    }
}
