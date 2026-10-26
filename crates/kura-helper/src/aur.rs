// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Building AUR packages.
//!
//! Every build runs as a throwaway user (`systemd-run` with `DynamicUser`)
//! that cannot see anyone's home. It downloads the package base at the
//! commit the preview showed, imports the source signing keys into its own
//! keyring and runs `makepkg`. Only files built this way are installed.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Sender};

use kura_alpm::Config;
use kura_alpm::apply::{ApplyError, Event};
use kura_core::aur::{self, Build};
use kura_core::package::InstallReason;
use kura_core::plan::{AurBuild, Plan, PlanKind, Step};

/// Seen by root through systemd's symlink to `/var/lib/private/kura-build`.
const BUILD_ROOT: &str = "/var/lib/kura-build";
const HELPER: &str = "/usr/lib/kura/kura-helper";

fn checked(build: &AurBuild) -> Result<(), String> {
    if aur::valid_base(&build.base) && aur::valid_commit(&build.commit) {
        Ok(())
    } else {
        Err(format!("invalid AUR build {} {}", build.base, build.commit))
    }
}

/// The last lines of a build log, for the error message.
fn tail(log: &[u8]) -> String {
    let text = String::from_utf8_lossy(log);
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(15)..].join("\n")
}

/// Builds one package base in the sandbox and returns the package files.
pub fn build(build: &AurBuild, threads: usize) -> Result<Vec<PathBuf>, String> {
    checked(build)?;
    let out = Command::new("systemd-run")
        .args([
            "--wait",
            "--pipe",
            "--collect",
            "--quiet",
            "--property=DynamicUser=yes",
            "--property=StateDirectory=kura-build",
            "--property=ProtectHome=yes",
            "--property=PrivateTmp=yes",
            "--property=NoNewPrivileges=yes",
            &format!("--setenv=MAKEFLAGS=-j{threads}"),
            &format!("--setenv=HOME={BUILD_ROOT}"),
            "--",
            HELPER,
            "build",
            &build.base,
            &build.commit,
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("systemd-run: {e}"))?;
    if !out.status.success() {
        return Err(format!("{}:\n{}", build.base, tail(&out.stderr)));
    }
    // Only files below the build directory, whatever the sandbox printed.
    // With DynamicUser it really is /var/lib/private/kura-build.
    let root = std::fs::canonicalize(BUILD_ROOT).map_err(|e| format!("{BUILD_ROOT}: {e}"))?;
    let files: Vec<PathBuf> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| std::fs::canonicalize(l).ok())
        .filter(|p| p.starts_with(&root) && p.is_file())
        .collect();
    if files.is_empty() {
        return Err(format!(
            "{}: makepkg built no packages\n{}\n{}",
            build.base,
            String::from_utf8_lossy(&out.stdout).trim(),
            tail(&out.stderr)
        ));
    }
    Ok(files)
}

fn run(program: &str, args: &[&str], dir: &Path, env: &[(&str, &Path)]) -> Result<(), String> {
    // makepkg talks on stdout; stdout is reserved for the list of files.
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .envs(env.iter().map(|(k, v)| (*k, *v)))
        .stdin(Stdio::null())
        .stdout(std::io::stderr())
        .status()
        .map_err(|e| format!("{program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed", args.join(" ")))
    }
}

/// Runs inside the sandbox: downloads, builds and prints the package files.
pub fn build_in_sandbox(base: &str, commit: &str) -> Result<(), String> {
    if !aur::valid_base(base) || !aur::valid_commit(commit) {
        return Err(format!("invalid AUR build {base} {commit}"));
    }
    let root = PathBuf::from(std::env::var("STATE_DIRECTORY").unwrap_or_else(|_| BUILD_ROOT.into()));
    let dir = root.join(format!("{base}-{commit}"));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let url = format!(
        "{}/cgit/aur.git/snapshot/aur-{commit}.tar.gz",
        kura_alpm::aur::BASE_URL
    );
    run(
        "curl",
        &[
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "-o",
            "aur.tar.gz",
            &url,
        ],
        &dir,
        &[],
    )?;
    run("tar", &["-xzf", "aur.tar.gz"], &dir, &[])?;
    let src = dir.join(format!("aur-{commit}"));
    let info = std::fs::read_to_string(src.join(".SRCINFO")).map_err(|e| format!(".SRCINFO: {e}"))?;
    let info = aur::parse_srcinfo(&info);
    if info.pkgbase != base {
        return Err(format!("{base}: the download contains {}", info.pkgbase));
    }

    let gnupg = root.join("gnupg");
    std::fs::create_dir_all(&gnupg).map_err(|e| e.to_string())?;
    let _ = std::fs::set_permissions(&gnupg, std::os::unix::fs::PermissionsExt::from_mode(0o700));
    let out = dir.join("out");
    let env = [
        ("GNUPGHOME", gnupg.as_path()),
        ("PKGDEST", out.as_path()),
        ("SRCDEST", src.as_path()),
        ("BUILDDIR", src.as_path()),
    ];
    for key in &info.validpgpkeys {
        // A missing key makes makepkg stop with a clear message.
        let _ = run("gpg", &["--batch", "--recv-keys", key], &src, &env);
    }
    run(
        "makepkg",
        &["--noconfirm", "--noprogressbar", "--nocolor"],
        &src,
        &env,
    )?;

    let list = Command::new("makepkg")
        .arg("--packagelist")
        .current_dir(&src)
        .envs(env.iter().map(|(k, v)| (*k, *v)))
        .output()
        .map_err(|e| e.to_string())?;
    let mut stdout = std::io::stdout().lock();
    for line in String::from_utf8_lossy(&list.stdout).lines() {
        if Path::new(line).is_file() {
            let _ = writeln!(stdout, "{line}");
        }
    }
    Ok(())
}

/// The package name in a file name like `name-1.0-1-x86_64.pkg.tar.zst`.
/// The version is not compared: VCS packages set it during the build.
fn package_name(file: &str) -> Option<&str> {
    let mut parts = file.rsplitn(4, '-');
    let _arch = parts.next()?;
    let _rel = parts.next()?;
    let _ver = parts.next()?;
    parts.next()
}

fn graph(builds: &[AurBuild]) -> BTreeMap<String, Build> {
    builds
        .iter()
        .map(|b| {
            (
                b.base.clone(),
                Build {
                    version: b.version.clone(),
                    packages: b.packages.iter().cloned().collect(),
                    after: b.after.iter().cloned().collect(),
                },
            )
        })
        .collect()
}

/// Bytes of memory available, from `/proc/meminfo`.
fn memory_available() -> u64 {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|m| {
            m.lines().find_map(|l| {
                let kib: u64 = l
                    .strip_prefix("MemAvailable:")?
                    .split_whitespace()
                    .next()?
                    .parse()
                    .ok()?;
                Some(kib * 1024)
            })
        })
        .unwrap_or(0)
}

/// Installs an AUR plan: derives it again, runs the repository part, then
/// builds the bases in parallel and installs each one as soon as it is built.
pub fn apply_aur(conf: &Config, approved: &Plan, events: &Sender<Event>) -> Result<(), ApplyError> {
    for b in &approved.builds {
        checked(b).map_err(ApplyError::Failed)?;
    }
    let targets: Vec<String> = approved
        .steps
        .iter()
        .filter_map(|s| match s {
            Step::Install {
                name,
                reason: InstallReason::Explicit,
                ..
            } if approved.builds.iter().any(|b| b.packages.contains(name)) => Some(name.clone()),
            _ => None,
        })
        .collect();
    let fresh = kura_alpm::aur::prepare_install(conf, None, &targets, false)?
        .plan
        .map_err(ApplyError::Blocked)?;
    if fresh.builds != approved.builds {
        return Err(ApplyError::Changed(Box::new(fresh)));
    }

    let aur_names: BTreeSet<&String> = approved.builds.iter().flat_map(|b| &b.packages).collect();
    let repo_steps: Vec<Step> = approved
        .steps
        .iter()
        .filter(|s| !aur_names.contains(&s.package().to_owned()))
        .cloned()
        .collect();
    if !repo_steps.is_empty() {
        let repo = Plan {
            kind: PlanKind::AurInstall,
            steps: repo_steps,
            actions: Vec::new(),
            builds: Vec::new(),
        };
        kura_alpm::apply::apply(conf, &repo, events).map_err(|e| match e {
            ApplyError::Changed(_) => ApplyError::Changed(Box::new(fresh.clone())),
            other => other,
        })?;
    }
    build_all(conf, approved, &targets, events)
}

/// Builds the bases in parallel, each as soon as the bases it needs are
/// installed, and installs each one right after its build.
fn build_all(
    conf: &Config,
    approved: &Plan,
    targets: &[String],
    events: &Sender<Event>,
) -> Result<(), ApplyError> {
    let reason = |name: &str| {
        if targets.iter().any(|t| t == name) {
            InstallReason::Explicit
        } else {
            InstallReason::Dependency
        }
    };
    let graph = graph(&approved.builds);
    let cpus = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
    let jobs = aur::jobs(cpus, memory_available(), graph.len());
    let total = graph.len();
    let (tx, rx) = mpsc::channel::<(String, Result<Vec<PathBuf>, String>)>();
    let mut started = BTreeSet::new();
    let mut done = BTreeSet::new();
    let mut running = 0usize;
    let mut failure: Option<String> = None;

    loop {
        if failure.is_none() {
            for base in aur::ready(&graph, &done, &started) {
                if running >= jobs.parallel {
                    break;
                }
                let Some(build) = approved.builds.iter().find(|b| b.base == base).cloned() else {
                    continue;
                };
                started.insert(base.to_owned());
                running += 1;
                let _ = events.send(Event::Building {
                    base: base.to_owned(),
                    current: started.len(),
                    total,
                });
                let tx = tx.clone();
                let threads = jobs.threads_each;
                std::thread::spawn(move || {
                    let result = self::build(&build, threads);
                    let _ = tx.send((build.base, result));
                });
            }
        }
        if running == 0 {
            break;
        }
        let Ok((base, result)) = rx.recv() else { break };
        running -= 1;
        match result {
            Ok(files) => {
                let build = approved
                    .builds
                    .iter()
                    .find(|b| b.base == base)
                    .expect("known base");
                // Split packages: install only the ones the plan names.
                let wanted: Vec<PathBuf> = files
                    .into_iter()
                    .filter(|f| {
                        f.file_name()
                            .and_then(|n| package_name(&n.to_string_lossy()).map(str::to_owned))
                            .is_some_and(|n| build.packages.contains(&n))
                    })
                    .collect();
                let reasons: Vec<(String, InstallReason)> =
                    build.packages.iter().map(|p| (p.clone(), reason(p))).collect();
                match kura_alpm::apply::install_files(conf, &wanted, &reasons, events) {
                    Ok(()) => {
                        done.insert(base);
                    }
                    Err(e) => failure = Some(format!("{base}: {e}")),
                }
            }
            Err(e) => failure = Some(e),
        }
    }
    match failure {
        Some(message) => Err(ApplyError::Failed(message)),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_from_package_files() {
        assert_eq!(package_name("pfetch-1.11.0-1-any.pkg.tar.zst"), Some("pfetch"));
        assert_eq!(
            package_name("lib32-foo-bar-2:1.0.r5.gabc-2-x86_64.pkg.tar.zst"),
            Some("lib32-foo-bar")
        );
        assert_eq!(package_name("broken.pkg.tar.zst"), None);
    }

    #[test]
    fn keeps_the_last_log_lines() {
        let log = (1..=20)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let t = tail(log.as_bytes());
        assert!(t.starts_with("line 6\n"));
        assert!(t.ends_with("line 20"));
    }

    #[test]
    fn refuses_odd_names() {
        let mut b = AurBuild {
            base: "pfetch".into(),
            commit: "23a5d6a1d2e2b1b9a3f0f6d8b7e3f2c1a0b9c8d7".into(),
            version: "1-1".into(),
            packages: vec!["pfetch".into()],
            after: Vec::new(),
        };
        assert_eq!(checked(&b), Ok(()));
        b.base = "../x".into();
        assert!(checked(&b).is_err());
    }
}
