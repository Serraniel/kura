// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `kura-helper`: the only part of kura that changes the system.
//!
//! `kura-helper record` is run by a pacman hook after every transaction and
//! appends the new generation to the history.

use std::path::Path;
use std::process::ExitCode;

use kura_alpm::history::{CAUSE, JOURNAL};

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
        Some("record") => record(),
        _ => Err("usage: kura-helper record".to_owned()),
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
