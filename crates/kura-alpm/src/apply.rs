// SPDX-FileCopyrightText: 2026 The kura contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Carries out a plan the user approved. Only `kura-helper` calls this.
//!
//! The plan is prepared again against the real databases right before the
//! commit. If libalpm would now do anything different from what the user saw,
//! nothing happens and the new plan is returned for another review.

use std::collections::BTreeSet;
use std::sync::mpsc::Sender;

use alpm::{Alpm, AnyEvent, CommitData, Event as AlpmEvent, Progress, Question};
use kura_core::plan::{Plan, RemovalReason, Step};

use crate::prepare::{Problem, Request, available_upgrades, build, flags};
use crate::{Config, Error};

/// What the user sees while a plan runs.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "event", rename_all = "kebab-case")]
pub enum Event {
    /// Database refresh started.
    Refreshing,
    /// Packages are being downloaded.
    Downloading { file: String, done: u64, total: u64 },
    /// A phase of the transaction with its progress.
    Progress {
        phase: Phase,
        package: String,
        percent: u8,
        current: usize,
        total: usize,
    },
    /// Hooks after the transaction (initramfs, boot menu, DKMS).
    Hook {
        name: String,
        current: usize,
        total: usize,
    },
    /// Old orphaned packages are being removed.
    CleaningUp,
    /// Signing keys are updated before the rest of the update.
    UpdatingKeyring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    CheckingFiles,
    CheckingSpace,
    Verifying,
    Installing,
    Removing,
}

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error("kura must run as root to change the system")]
    NotRoot,
    #[error("another program is installing software right now")]
    Locked,
    #[error("the change is different from the preview")]
    Changed(Box<Plan>),
    #[error("the change cannot be prepared")]
    Blocked(Vec<Problem>),
    #[error("files would conflict: {0}")]
    FileConflicts(String),
    #[error("{0}")]
    Failed(String),
    #[error(transparent)]
    Setup(#[from] Error),
}

impl From<alpm::Error> for ApplyError {
    fn from(e: alpm::Error) -> Self {
        if e == alpm::Error::HandleLock {
            ApplyError::Locked
        } else {
            ApplyError::Setup(Error::Alpm(e))
        }
    }
}

/// The part of a plan libalpm decides itself; cleanup and reason changes are applied separately.
fn core_steps(plan: &Plan) -> BTreeSet<String> {
    plan.steps
        .iter()
        .filter_map(|s| match s {
            Step::Install { name, version, .. } => Some(format!("install {name} {version}")),
            Step::Upgrade { name, to, .. } => Some(format!("upgrade {name} {to}")),
            Step::Downgrade { name, to, .. } => Some(format!("downgrade {name} {to}")),
            Step::Remove {
                why: RemovalReason::Orphan,
                ..
            } if plan.kind != kura_core::plan::PlanKind::Remove => None,
            Step::Remove { name, .. } => Some(format!("remove {name}")),
            Step::MarkReason { .. } => None,
        })
        .collect()
}

/// Whether two plans would make libalpm do the same thing.
pub fn same_change(approved: &Plan, fresh: &Plan) -> bool {
    approved.kind == fresh.kind && core_steps(approved) == core_steps(fresh)
}

/// The request a plan was made from.
pub fn request_for(plan: &Plan) -> Option<Request> {
    use kura_core::plan::PlanKind;
    let named = |want_install: bool| -> Vec<String> {
        plan.steps
            .iter()
            .filter_map(|s| match s {
                Step::Install {
                    name,
                    reason: kura_core::package::InstallReason::Explicit,
                    ..
                } if want_install => Some(name.clone()),
                Step::Remove {
                    name,
                    why: RemovalReason::Requested,
                    ..
                } if !want_install => Some(name.clone()),
                _ => None,
            })
            .collect()
    };
    match plan.kind {
        PlanKind::SystemUpdate => Some(Request::Update),
        PlanKind::Install => Some(Request::Install(named(true))),
        PlanKind::Remove => Some(Request::Remove(named(false))),
        PlanKind::Repair => Some(Request::Repair(
            plan.steps
                .iter()
                .filter_map(|s| match s {
                    Step::Remove {
                        name,
                        why: RemovalReason::Repair,
                        ..
                    } => Some(name.clone()),
                    _ => None,
                })
                .collect(),
        )),
        PlanKind::Restore { .. } => None,
    }
}

fn install_callbacks(handle: &Alpm, plan: &Plan, events: &Sender<Event>) {
    let replaced: BTreeSet<String> = plan
        .steps
        .iter()
        .filter_map(|s| match s {
            Step::Remove { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    let planned: BTreeSet<String> = plan
        .steps
        .iter()
        .filter_map(|s| match s {
            Step::Install { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    // Answers follow the plan; anything the user did not see is declined.
    handle.set_question_cb(
        (replaced, planned),
        |q, data: &mut (BTreeSet<String>, BTreeSet<String>)| {
            let (replaced, planned) = (&data.0, &data.1);
            match q.question() {
                Question::Replace(r) => r.set_replace(replaced.contains(r.oldpkg().name())),
                Question::Conflict(mut c) => {
                    let other = c.conflict().package2().name().to_owned();
                    c.set_remove(replaced.contains(&other));
                }
                Question::Corrupted(mut c) => c.set_remove(true),
                Question::RemovePkgs(mut r) => r.set_skip(false),
                Question::SelectProvider(mut s) => {
                    // Take the provider the approved plan installs.
                    let index = s
                        .providers()
                        .iter()
                        .position(|p| planned.contains(p.name()))
                        .unwrap_or(0);
                    s.set_index(i32::try_from(index).unwrap_or(0));
                }
                Question::ImportKey(mut k) => k.set_import(false),
                Question::InstallIgnorepkg(mut i) => i.set_install(false),
            }
        },
    );

    let tx = events.clone();
    handle.set_progress_cb(
        tx,
        |progress, pkg, percent, total, current, tx: &mut Sender<Event>| {
            let phase = match progress {
                Progress::AddStart
                | Progress::UpgradeStart
                | Progress::DowngradeStart
                | Progress::ReinstallStart => Phase::Installing,
                Progress::RemoveStart => Phase::Removing,
                Progress::ConflictsStart => Phase::CheckingFiles,
                Progress::DiskspaceStart => Phase::CheckingSpace,
                Progress::IntegrityStart | Progress::LoadStart | Progress::KeyringStart => Phase::Verifying,
            };
            let _ = tx.send(Event::Progress {
                phase,
                package: pkg.to_owned(),
                percent: u8::try_from(percent.clamp(0, 100)).unwrap_or(100),
                current,
                total,
            });
        },
    );

    let tx = events.clone();
    handle.set_dl_cb(tx, |file, event, tx: &mut Sender<Event>| {
        if let alpm::DownloadEvent::Progress(p) = event.event() {
            let _ = tx.send(Event::Downloading {
                file: file.to_owned(),
                done: u64::try_from(p.downloaded).unwrap_or(0),
                total: u64::try_from(p.total).unwrap_or(0),
            });
        }
    });

    let tx = events.clone();
    handle.set_event_cb(tx, |event: AnyEvent, tx: &mut Sender<Event>| {
        if let AlpmEvent::HookRunStart(h) = event.event() {
            let _ = tx.send(Event::Hook {
                name: h.name().to_owned(),
                current: h.position(),
                total: h.total(),
            });
        }
    });
}

/// Opens a transaction, runs `f` and always releases the transaction again.
fn with_trans<T>(
    handle: &mut Alpm,
    flags: alpm::TransFlag,
    f: impl FnOnce(&mut Alpm) -> Result<T, ApplyError>,
) -> Result<T, ApplyError> {
    handle.trans_init(flags)?;
    let result = f(handle);
    handle.trans_release()?;
    result
}

fn commit(handle: &mut Alpm) -> Result<(), ApplyError> {
    handle.trans_commit().map_err(|e| match e.data() {
        // alpm 5 types this list as package conflicts although libalpm fills it
        // with file conflicts, so the entries are not read.
        Some(CommitData::FileConflict(_)) => ApplyError::FileConflicts(e.to_string()),
        Some(CommitData::PkgInvalid(list)) => ApplyError::Failed(format!(
            "invalid packages: {}",
            list.iter().collect::<Vec<_>>().join(", ")
        )),
        None => ApplyError::Failed(e.to_string()),
    })
}

/// Keyring packages the plan upgrades. Arch recommends updating these first so
/// that packages signed by new packager keys can be verified.
pub fn keyring_upgrades(plan: &Plan) -> Vec<String> {
    plan.steps
        .iter()
        .filter_map(|s| match s {
            Step::Upgrade { name, .. } if name.ends_with("-keyring") => Some(name.clone()),
            _ => None,
        })
        .collect()
}

/// The plan without its keyring upgrades: what is left after they ran first.
fn without(plan: &Plan, names: &[String]) -> Plan {
    Plan {
        kind: plan.kind,
        steps: plan
            .steps
            .iter()
            .filter(|s| !names.iter().any(|n| n == s.package()))
            .cloned()
            .collect(),
    }
}

enum First {
    Done,
    Keyrings(Vec<String>),
}

/// Prepares the request again, checks it against the approved plan and returns it.
fn verified(handle: &mut Alpm, request: &Request, approved: &Plan) -> Result<Plan, ApplyError> {
    let available = available_upgrades(handle);
    let built = build(handle, request, &available)?;
    let fresh = built.plan.map_err(ApplyError::Blocked)?;
    if same_change(approved, &fresh) {
        Ok(fresh)
    } else {
        Err(ApplyError::Changed(Box::new(fresh)))
    }
}

/// Runs an approved plan. Returns `Changed` with the new plan when the
/// system moved on since the preview.
pub fn apply(conf: &Config, approved: &Plan, events: &Sender<Event>) -> Result<(), ApplyError> {
    if !crate::is_root() {
        return Err(ApplyError::NotRoot);
    }
    let request = request_for(approved)
        .ok_or_else(|| ApplyError::Failed("this kind of plan is not supported yet".into()))?;
    let mut handle = crate::handle(conf)?;
    install_callbacks(&handle, approved, events);

    if matches!(
        request,
        Request::Update | Request::Install(_) | Request::Repair(_)
    ) {
        let _ = events.send(Event::Refreshing);
        handle.syncdbs_mut().update(false)?;
    }
    let first = with_trans(&mut handle, flags(&request), |h| {
        let fresh = verified(h, &request, approved)?;
        let keyrings = keyring_upgrades(&fresh);
        let upgrades = fresh
            .steps
            .iter()
            .filter(|s| matches!(s, Step::Upgrade { .. }))
            .count();
        if fresh.steps.is_empty() {
            return Ok(First::Done);
        }
        if keyrings.is_empty() || keyrings.len() == upgrades && fresh.steps.len() == upgrades {
            commit(h)?;
            return Ok(First::Done);
        }
        Ok(First::Keyrings(keyrings))
    })?;

    if let First::Keyrings(names) = first {
        let _ = events.send(Event::UpdatingKeyring);
        with_trans(&mut handle, alpm::TransFlag::NONE, |h| {
            for name in &names {
                let pkg = h.syncdbs().iter().find_map(|db| db.pkg(name.as_str()).ok());
                let pkg = pkg.ok_or_else(|| ApplyError::Failed(format!("{name} disappeared")))?;
                h.trans_add_pkg(pkg)
                    .map_err(|e| ApplyError::Failed(e.to_string()))?;
            }
            h.trans_prepare().map_err(|e| ApplyError::Failed(e.to_string()))?;
            commit(h)
        })?;
        let rest = without(approved, &names);
        with_trans(&mut handle, flags(&request), |h| {
            verified(h, &request, &rest)?;
            commit(h)
        })?;
    }

    cleanup(&mut handle, approved, events)
}

/// Goes back to an earlier generation. The plan is computed from the journal
/// again and must match the approved one.
///
/// # Panics
///
/// Never: the generation exists because a plan to it was computed.
pub fn apply_restore(
    conf: &Config,
    approved: &Plan,
    journal: &std::path::Path,
    events: &Sender<Event>,
) -> Result<(), ApplyError> {
    use crate::restore::{Source, plan, sources};
    use kura_core::plan::PlanKind;

    if !crate::is_root() {
        return Err(ApplyError::NotRoot);
    }
    let PlanKind::Restore { generation } = approved.kind else {
        return Err(ApplyError::Failed("not a restore plan".into()));
    };
    let history = crate::history::load(journal)?;
    let fresh = plan(&history, generation)
        .ok_or_else(|| ApplyError::Failed(format!("no generation {generation}")))?;
    if core_steps(&fresh) != core_steps(approved) {
        return Err(ApplyError::Changed(Box::new(fresh)));
    }
    let target = history
        .generation(generation)
        .expect("plan exists, so the generation does");
    let found = sources(&conf.cache_dir, &fresh, &target).map_err(ApplyError::Blocked)?;

    let mut handle = crate::handle(conf)?;
    install_callbacks(&handle, approved, events);
    let mut files = Vec::new();
    for (name, source) in found {
        match source {
            Source::Cache(path) => {
                files.push((path.to_string_lossy().into_owned(), handle.local_file_siglevel()));
            }
            Source::Archive(urls) => {
                let _ = events.send(Event::Downloading {
                    file: name.clone(),
                    done: 0,
                    total: 0,
                });
                let fetched = urls
                    .iter()
                    .find_map(|u| handle.fetch_pkgurl([u.as_str()].iter().copied()).ok());
                let path = fetched
                    .and_then(|list| list.iter().next().map(ToOwned::to_owned))
                    .ok_or_else(|| {
                        ApplyError::Blocked(vec![Problem::NotRestorable {
                            name: name.clone(),
                            version: String::new(),
                        }])
                    })?;
                files.push((path, handle.remote_file_siglevel()));
            }
        }
    }

    with_trans(&mut handle, alpm::TransFlag::NONE, |h| {
        for (path, level) in &files {
            let pkg = h
                .pkg_load(path.as_str(), true, *level)
                .map_err(|e| ApplyError::Failed(format!("{path}: {e}")))?;
            h.trans_add_pkg(pkg)
                .map_err(|e| ApplyError::Failed(e.to_string()))?;
        }
        for step in &fresh.steps {
            if let Step::Remove { name, .. } = step
                && let Ok(pkg) = h.localdb().pkg(name.as_str())
            {
                h.trans_remove_pkg(pkg)?;
            }
        }
        if h.trans_add().is_empty() && h.trans_remove().is_empty() {
            return Ok(());
        }
        h.trans_prepare()
            .map_err(|e| ApplyError::Blocked(crate::prepare::problems(&e)))?;
        commit(h)
    })?;

    // Install reasons last, once every package is in place.
    for step in &fresh.steps {
        let reason = match step {
            Step::MarkReason { reason, .. } | Step::Install { reason, .. } => *reason,
            _ => continue,
        };
        if let Ok(pkg) = handle.localdb().pkg(step.package()) {
            let reason = match reason {
                kura_core::package::InstallReason::Explicit => alpm::PackageReason::Explicit,
                kura_core::package::InstallReason::Dependency => alpm::PackageReason::Depend,
            };
            pkg.set_reason(reason)?;
        }
    }
    Ok(())
}

/// Removes the orphans listed in the plan in a second transaction. Packages
/// that became needed in the meantime are left alone.
fn cleanup(handle: &mut Alpm, plan: &Plan, events: &Sender<Event>) -> Result<(), ApplyError> {
    if plan.kind == kura_core::plan::PlanKind::Remove {
        return Ok(());
    }
    let orphans: Vec<&str> = plan
        .steps
        .iter()
        .filter_map(|s| match s {
            Step::Remove {
                name,
                why: RemovalReason::Orphan,
                ..
            } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    if orphans.is_empty() {
        return Ok(());
    }
    let _ = events.send(Event::CleaningUp);
    handle.trans_init(alpm::TransFlag::NONE)?;
    let result = (|| {
        for name in &orphans {
            if let Ok(pkg) = handle.localdb().pkg(*name)
                && pkg.required_by().is_empty()
                && pkg.reason() == alpm::PackageReason::Depend
            {
                handle.trans_remove_pkg(pkg)?;
            }
        }
        if handle.trans_remove().is_empty() {
            return Ok(());
        }
        handle
            .trans_prepare()
            .map_err(|e| ApplyError::Failed(e.to_string()))?;
        handle
            .trans_commit()
            .map_err(|e| ApplyError::Failed(e.to_string()))
    })();
    handle.trans_release()?;
    result
}

#[cfg(test)]
mod tests {
    use kura_core::package::InstallReason;
    use kura_core::plan::PlanKind;

    use super::*;

    fn up(name: &str, to: &str) -> Step {
        Step::Upgrade {
            name: name.into(),
            from: "1".into(),
            to: to.into(),
        }
    }

    #[test]
    fn same_change_ignores_cleanup_and_old_versions() {
        let mut a = Plan {
            kind: PlanKind::SystemUpdate,
            steps: vec![up("firefox", "2")],
        };
        let b = Plan {
            kind: PlanKind::SystemUpdate,
            steps: vec![up("firefox", "2")],
        };
        a.add_cleanup([("electron36", "36")]);
        assert!(same_change(&a, &b));
        let c = Plan {
            kind: PlanKind::SystemUpdate,
            steps: vec![up("firefox", "3")],
        };
        assert!(!same_change(&a, &c));
        let d = Plan {
            kind: PlanKind::SystemUpdate,
            steps: vec![up("firefox", "2"), up("mesa", "2")],
        };
        assert!(!same_change(&a, &d));
    }

    #[test]
    fn keyrings_are_found_and_split_off() {
        let plan = Plan {
            kind: PlanKind::SystemUpdate,
            steps: vec![
                up("archlinux-keyring", "2"),
                up("firefox", "2"),
                up("endeavouros-keyring", "3"),
            ],
        };
        let keys = keyring_upgrades(&plan);
        assert_eq!(keys, ["archlinux-keyring", "endeavouros-keyring"]);
        let rest = without(&plan, &keys);
        assert_eq!(rest.steps, vec![up("firefox", "2")]);
        assert_eq!(keyring_upgrades(&rest), Vec::<String>::new());
    }

    #[test]
    fn remove_plans_compare_their_orphans() {
        let rm = |names: &[(&str, RemovalReason)]| Plan {
            kind: PlanKind::Remove,
            steps: names
                .iter()
                .map(|(n, why)| Step::Remove {
                    name: (*n).into(),
                    version: "1".into(),
                    why: *why,
                })
                .collect(),
        };
        let a = rm(&[
            ("gimp", RemovalReason::Requested),
            ("babl", RemovalReason::Orphan),
        ]);
        let b = rm(&[("gimp", RemovalReason::Requested)]);
        assert!(!same_change(&a, &b));
        assert!(same_change(&a, &a.clone()));
    }

    #[test]
    fn requests_are_recovered_from_plans() {
        let install = Plan {
            kind: PlanKind::Install,
            steps: vec![
                Step::Install {
                    name: "gimp".into(),
                    version: "3".into(),
                    reason: InstallReason::Explicit,
                },
                Step::Install {
                    name: "babl".into(),
                    version: "1".into(),
                    reason: InstallReason::Dependency,
                },
                up("mesa", "2"),
            ],
        };
        assert_eq!(request_for(&install), Some(Request::Install(vec!["gimp".into()])));
        let remove = Plan {
            kind: PlanKind::Remove,
            steps: vec![
                Step::Remove {
                    name: "gimp".into(),
                    version: "3".into(),
                    why: RemovalReason::Requested,
                },
                Step::Remove {
                    name: "babl".into(),
                    version: "1".into(),
                    why: RemovalReason::Orphan,
                },
            ],
        };
        assert_eq!(request_for(&remove), Some(Request::Remove(vec!["gimp".into()])));
        assert_eq!(
            request_for(&Plan::new(PlanKind::SystemUpdate)),
            Some(Request::Update)
        );
        assert_eq!(
            request_for(&Plan::new(PlanKind::Repair)),
            Some(Request::Repair(vec![]))
        );
        assert_eq!(request_for(&Plan::new(PlanKind::Restore { generation: 1 })), None);
    }

    #[test]
    fn not_root_is_refused() {
        if crate::is_root() {
            return;
        }
        let (tx, _rx) = std::sync::mpsc::channel();
        let conf = Config::default();
        assert!(matches!(
            apply(&conf, &Plan::new(PlanKind::SystemUpdate), &tx),
            Err(ApplyError::NotRoot)
        ));
    }

    /// Installs and removes a small package for real. Runs only in CI's
    /// throwaway Arch container, as root.
    #[test]
    #[ignore = "changes the system; CI container only"]
    fn installs_and_removes_in_a_container() {
        assert!(crate::is_root(), "run as root in a container");
        let conf = crate::load_config().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();

        let target = Request::Install(vec!["figlet".into()]);
        let preview = crate::prepare::prepare(&conf, None, &target, true, &[]).unwrap();
        let plan = preview.plan.unwrap();
        apply(&conf, &plan, &tx).unwrap();
        assert!(crate::installed(&conf).unwrap().get("figlet").is_some());
        assert!(rx.try_iter().any(|e| matches!(
            e,
            Event::Progress {
                phase: Phase::Installing,
                ..
            }
        )));

        // A plan that no longer matches the system is refused.
        let mut stale = plan.clone();
        stale.steps.push(Step::Install {
            name: "sl".into(),
            version: "1".into(),
            reason: InstallReason::Explicit,
        });
        assert!(matches!(apply(&conf, &stale, &tx), Err(ApplyError::Changed(_))));

        let removal =
            crate::prepare::prepare(&conf, None, &Request::Remove(vec!["figlet".into()]), false, &[])
                .unwrap();
        apply(&conf, &removal.plan.unwrap(), &tx).unwrap();
        assert!(crate::installed(&conf).unwrap().get("figlet").is_none());

        // A repair: the full update plus removing figlet, as the solver would propose.
        let again = crate::prepare::prepare(&conf, None, &target, true, &[]).unwrap();
        apply(&conf, &again.plan.unwrap(), &tx).unwrap();
        let repair =
            crate::prepare::prepare(&conf, None, &Request::Repair(vec!["figlet".into()]), false, &[])
                .unwrap();
        let plan = repair.plan.unwrap();
        assert!(
            plan.steps.iter().any(
                |s| matches!(s, Step::Remove { name, why: RemovalReason::Repair, .. } if name == "figlet")
            )
        );
        apply(&conf, &plan, &tx).unwrap();
        assert!(crate::installed(&conf).unwrap().get("figlet").is_none());
    }
}
