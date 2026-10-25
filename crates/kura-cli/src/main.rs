// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! `kura` command line interface.

use std::fmt::Write as _;
use std::io::Write as _;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use kura_core::package::{InstallReason, PackageSet};
use kura_i18n::Localizer;

fn main() -> ExitCode {
    let l = Localizer::from_env();
    match run(&l) {
        Ok(text) => {
            // A closed pipe (`kura sbom | head`) is not an error worth reporting.
            let _ = std::io::stdout().lock().write_all(text.as_bytes());
            ExitCode::SUCCESS
        }
        Err(msg) => {
            eprintln!("{msg}");
            ExitCode::FAILURE
        }
    }
}

fn run(l: &Localizer) -> Result<String, String> {
    use lexopt::prelude::*;

    let mut parser = lexopt::Parser::from_env();
    let mut command: Option<String> = None;
    let mut output: Option<String> = None;
    while let Some(arg) = parser.next().map_err(|e| e.to_string())? {
        match arg {
            Short('h') | Long("help") => return Ok(l.msg("cli-usage") + "\n"),
            Short('o') | Long("output") => {
                output = Some(
                    parser
                        .value()
                        .map_err(|e| e.to_string())?
                        .string()
                        .map_err(|e| e.to_string())?,
                );
            }
            Value(v) if command.is_none() => command = Some(v.string().map_err(|e| e.to_string())?),
            other => return Err(other.unexpected().to_string()),
        }
    }

    match command.as_deref() {
        Some("version") => Ok(format!("kura {}\n", env!("CARGO_PKG_VERSION"))),
        Some("doctor") => doctor(l),
        Some("orphans") => orphans(l),
        Some("sbom") => sbom(l, output.as_deref()),
        Some(other) => Err(format!(
            "{}\n\n{}",
            l.msg_args("cli-unknown-command", &[("command", other.to_owned().into())]),
            l.msg("cli-usage")
        )),
        None => Err(format!(
            "{}\n\n{}",
            l.msg("cli-missing-command"),
            l.msg("cli-usage")
        )),
    }
}

fn load(l: &Localizer) -> Result<PackageSet, String> {
    kura_alpm::installed(&kura_alpm::Paths::default())
        .map_err(|e| l.msg_args("error-database", &[("reason", e.to_string().into())]))
}

/// `NAME` and `ID` from /etc/os-release.
fn os_release() -> (String, String) {
    let text = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
            .map(|v| v.trim_matches('"').to_owned())
    };
    (
        field("NAME").unwrap_or_else(|| "Linux".into()),
        field("ID").unwrap_or_else(|| "linux".into()),
    )
}

fn doctor(l: &Localizer) -> Result<String, String> {
    let set = load(l)?;
    let (name, _) = os_release();
    let explicit = set.iter().filter(|p| p.reason == InstallReason::Explicit).count();
    let foreign = set.iter().filter(|p| p.repo.is_none()).count();
    let rows = [
        (l.msg("doctor-distro"), name),
        (l.msg("doctor-packages"), set.len().to_string()),
        (l.msg("doctor-explicit"), explicit.to_string()),
        (l.msg("doctor-foreign"), foreign.to_string()),
        (l.msg("doctor-orphans"), set.orphans().len().to_string()),
        (l.msg("doctor-language"), l.language()),
    ];
    let width = rows.iter().map(|(k, _)| k.chars().count()).max().unwrap_or(0);
    let mut out = format!("{}\n\n", l.msg("doctor-title"));
    for (k, v) in rows {
        let pad = width - k.chars().count();
        let _ = writeln!(out, "  {k}{}  {v}", " ".repeat(pad));
    }
    Ok(out)
}

fn orphans(l: &Localizer) -> Result<String, String> {
    let set = load(l)?;
    let orphans = set.orphans();
    if orphans.is_empty() {
        return Ok(l.msg("orphans-none") + "\n");
    }
    let mut out = l.msg_args("orphans-found", &[("count", orphans.len().into())]) + "\n";
    for name in &orphans {
        let version = set.get(name).map_or("", |p| p.version.as_str());
        let _ = writeln!(out, "  {name} {version}");
    }
    let _ = writeln!(out, "\n{}", l.msg("orphans-hint"));
    Ok(out)
}

fn sbom(l: &Localizer, output: Option<&str>) -> Result<String, String> {
    let set = load(l)?;
    let (name, id) = os_release();
    let uuid = std::fs::read_to_string("/proc/sys/kernel/random/uuid").unwrap_or_default();
    let serial = format!("urn:uuid:{}", uuid.trim());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let timestamp = kura_sbom::rfc3339(now);
    let bom = kura_sbom::build(
        &set,
        &kura_sbom::System {
            distro_id: &id,
            distro_name: &name,
        },
        &kura_sbom::RunInfo {
            serial: &serial,
            timestamp: &timestamp,
            tool_version: env!("CARGO_PKG_VERSION"),
        },
    );
    let text = serde_json::to_string_pretty(&bom).map_err(|e| e.to_string())?;
    match output {
        None | Some("-") => Ok(text + "\n"),
        Some(path) => {
            std::fs::write(path, text + "\n").map_err(|e| {
                l.msg_args(
                    "error-write",
                    &[("path", path.to_owned().into()), ("reason", e.to_string().into())],
                )
            })?;
            Ok(l.msg_args("sbom-written", &[("path", path.to_owned().into())]) + "\n")
        }
    }
}
