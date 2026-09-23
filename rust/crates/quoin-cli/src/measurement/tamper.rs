// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Git-derived tamper facts for `quoin measurement verify` (PLAT-985).
//!
//! `quoin-measurement`'s checker is pure and reads no git; [`super::intake_order`]
//! already crosses that line once, for the intake order itself, because only
//! this binary has git. These four checks are the same kind of fact,
//! gathered the same way, and handed to `measurement.verify` alongside the
//! order:
//!
//! - a collection once added to the store and later removed
//!   ([`deleted_and_edited`]'s `deleted`);
//! - a collection's stored file edited by a commit after the one that added
//!   it ([`deleted_and_edited`]'s `edited`);
//! - a collection's recorded protected-apparatus digest disagreeing with
//!   `git show <sourceRevision>:<path>` ([`forged_apparatus`]);
//! - the plan's own document changed its objective, estimator, decision rule
//!   or protected apparatus across two committed revisions that share a
//!   `definition_version` ([`definition_changed`]).
//!
//! Every check degrades to "nothing found" rather than an error when git
//! cannot answer — outside a work tree, a shallow clone missing the revision
//! in question, or a path git does not track — the same posture
//! [`super::intake_order`] takes for the order itself: a caller with no git
//! access attests nothing, rather than manufacturing a false tamper finding.

use std::collections::BTreeMap;
use std::path::Path;

use quoin_measurement::plans::{PlanLoadOptions, definition_changed_without_version_bump};
use quoin_measurement::source::{DiskMeasurement, MeasurementSource as _};

use super::MEASUREMENTS_DIRECTORY;

/// One collection the store's git history shows was added and later removed,
/// and the plan ids its observations named before it was gone.
pub(super) struct DeletedCollection {
    pub(super) id: String,
    pub(super) plan_ids: Vec<String>,
}

fn git(repo: &str, arguments: &[&str]) -> Result<std::process::Output, String> {
    std::process::Command::new("git")
        .args(["-C", repo])
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))
}

/// Every collection id the store's git history added and later removed
/// (`deleted`, PLAT-985), and every id whose file a commit modified after the
/// one that first added it (`edited`, PLAT-985).
///
/// One combined `git log` walks `--diff-filter=AMD` over the measurements
/// directory; an `A` line opens a collection's own commit list, and any `M`
/// or `D` seen for the same path after that is recorded against it. A path
/// this crate cannot key to a collection id (nested, or not `.json`) is
/// ignored, matching [`super::intake_order`].
pub(super) fn deleted_and_edited(
    repo: &str,
) -> Result<(Vec<DeletedCollection>, Vec<String>), String> {
    let log = git(
        repo,
        &[
            "log",
            "--first-parent",
            "--reverse",
            "--topo-order",
            "--no-renames",
            "--diff-filter=AMD",
            "--relative",
            "--format=commit %H",
            "--name-status",
            "--",
            MEASUREMENTS_DIRECTORY,
        ],
    )?;
    if !log.status.success() {
        // A shallow clone or a non-work-tree already stopped `intake_order`
        // from reaching here with a positioned order; treat any other git
        // failure the same way this check alone: nothing found rather than
        // an error, since `intake_order` already reported the git problem.
        return Ok((Vec::new(), Vec::new()));
    }
    let mut commit = String::new();
    // status ('A', 'M' or 'D') per collection id, in the order first seen.
    let mut events: Vec<(String, char, String)> = Vec::new();
    for line in String::from_utf8_lossy(&log.stdout).lines() {
        if let Some(hash) = line.strip_prefix("commit ") {
            hash.clone_into(&mut commit);
            continue;
        }
        let Some((status, path)) = line.split_once('\t') else {
            continue;
        };
        let Some(id) = id_of(path) else { continue };
        let Some(status) = status.chars().next() else {
            continue;
        };
        events.push((id, status, commit.clone()));
    }
    let mut added_commit: BTreeMap<&str, &str> = BTreeMap::new();
    let mut edited: Vec<String> = Vec::new();
    // `Some(commit)` once the id's most recent event is the deletion at
    // `commit`; an `A` after that clears it, so a re-added id is not
    // reported as deleted — the events are in git order (oldest first),
    // so replaying them in order is what makes "most recent" correct.
    let mut deleted_at: BTreeMap<String, String> = BTreeMap::new();
    for (id, status, commit) in &events {
        match status {
            'A' => {
                added_commit.insert(id.as_str(), commit.as_str());
                deleted_at.remove(id);
            }
            'M' if added_commit.contains_key(id.as_str()) && !edited.contains(id) => {
                edited.push(id.clone());
            }
            'D' => {
                deleted_at.insert(id.clone(), commit.clone());
            }
            _ => {}
        }
    }
    let mut deleted = Vec::new();
    for (id, delete_commit) in deleted_at {
        let plan_ids = plan_ids_before_deletion(repo, &delete_commit, &id);
        deleted.push(DeletedCollection { id, plan_ids });
    }
    Ok((deleted, edited))
}

/// The `MeasurementPlan` ids a collection's observations named, read from the
/// store's content at the commit just before `delete_commit` removed it.
/// Empty when that content cannot be read or parsed — an unattributed
/// deletion is not reported against any plan, rather than every plan.
fn plan_ids_before_deletion(repo: &str, delete_commit: &str, id: &str) -> Vec<String> {
    let path = format!("{MEASUREMENTS_DIRECTORY}/{id}.json");
    let Ok(show) = git(repo, &["show", &format!("{delete_commit}^:{path}")]) else {
        return Vec::new();
    };
    if !show.status.success() {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&show.stdout) else {
        return Vec::new();
    };
    let mut ids: Vec<String> = value
        .get("observations")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|observation| observation.get("planId"))
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// Every currently-stored collection id whose recorded protected-apparatus
/// digest for `plan_id` disagrees with `git show <sourceRevision>:<path>` —
/// the file's actual bytes at the commit the collection claims to be from
/// (PLAT-985, from Peter's PLAT-985 review comment: a collection committed by
/// hand can carry a forged `verificationStack.protectedApparatus`).
///
/// Scoped to `plan_id` alone: intake resolves apparatus per plan a
/// collection governs, and checking only the plan being verified keeps this
/// bounded by that plan's own protected set rather than every plan every
/// collection in the store ever named.
pub(super) fn forged_apparatus(repo: &str, plan_id: &str) -> Vec<String> {
    let mut forged = Vec::new();
    let Ok(entries) = std::fs::read_dir(Path::new(repo).join(MEASUREMENTS_DIRECTORY)) else {
        return forged;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(std::ffi::OsStr::to_str) != Some("json") {
            continue;
        }
        let Some(id) = path
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .map(str::to_owned)
        else {
            continue;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        let Some(revision) = value
            .get("sourceRevision")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let Some(protected) = value
            .pointer(&format!(
                "/verificationStack/protectedApparatus/{}",
                plan_id.replace('~', "~0").replace('/', "~1")
            ))
            .and_then(serde_json::Value::as_object)
        else {
            continue;
        };
        for (file_path, recorded) in protected {
            let Some(recorded) = recorded.as_str() else {
                forged.push(id.clone());
                break;
            };
            let Ok(show) = git(repo, &["show", &format!("{revision}:{file_path}")]) else {
                continue;
            };
            if !show.status.success() {
                // The revision, or the path within it, is not reachable —
                // unattested rather than forged (a shallow clone, most
                // often).
                continue;
            }
            let actual = quoin_store::digest_bytes_sha256(&show.stdout).to_stored();
            if actual != recorded {
                forged.push(id.clone());
                break;
            }
        }
    }
    forged
}

/// Whether `plan_id`'s document changed its objective, estimator, decision
/// rule or protected apparatus between two committed revisions that share a
/// `definition_version` (engineering-assurance's
/// `definition_change_without_version_bump`, PLAT-985).
///
/// The plan's current document is located through `quoin-measurement`'s own
/// assurance-document walk (the same one `measurement.verify` will load
/// plans from), and every revision `git log --follow` finds for that path is
/// parsed and compared to the one before it, oldest first. `false` when the
/// plan cannot be found, its history cannot be read, or every adjacent pair
/// agrees.
pub(super) fn definition_changed(repo: &str, plan_id: &str) -> bool {
    let Some(path) = plan_document_path(repo, plan_id) else {
        return false;
    };
    let Ok(log) = git(
        repo,
        &[
            "log",
            "--follow",
            "--reverse",
            "--format=commit %H",
            "--",
            &path,
        ],
    ) else {
        return false;
    };
    if !log.status.success() {
        return false;
    }
    let commits: Vec<String> = String::from_utf8_lossy(&log.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("commit "))
        .map(str::to_owned)
        .collect();
    let mut previous: Option<quoin_measurement::MeasurementPlan> = None;
    for commit in &commits {
        let Ok(show) = git(repo, &["show", &format!("{commit}:{path}")]) else {
            continue;
        };
        if !show.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&show.stdout).into_owned();
        let Ok(Some(revision)) =
            quoin_measurement::plans::plan_from_text(&path, &text, PlanLoadOptions::default())
        else {
            continue;
        };
        if let Some(before) = &previous
            && definition_changed_without_version_bump(before, &revision)
        {
            return true;
        }
        previous = Some(revision);
    }
    false
}

/// The repository-relative document path of the `MeasurementPlan` named
/// `plan_id`, read off the current tree the same way `measurement.verify`
/// will load it — or `None` when no such plan exists.
fn plan_document_path(repo: &str, plan_id: &str) -> Option<String> {
    let source = DiskMeasurement::new(Path::new(repo));
    let paths = source.assurance_documents().ok()?;
    for path in paths {
        let Ok(text) = source.document_text(&path) else {
            continue;
        };
        let Ok(Some(plan)) =
            quoin_measurement::plans::plan_from_text(&path, &text, PlanLoadOptions::default())
        else {
            continue;
        };
        if plan.id.as_str() == plan_id {
            return Some(path);
        }
    }
    None
}

/// The collection id a `spec/evidence/measurements/<id>.json` path names, or
/// `None` for anything else — the same key `super::intake_order` reads.
fn id_of(path: &str) -> Option<String> {
    path.strip_prefix(MEASUREMENTS_DIRECTORY)
        .and_then(|rest| rest.strip_prefix('/'))
        .and_then(|name| name.strip_suffix(".json"))
        .filter(|id| !id.contains('/'))
        .map(str::to_owned)
}
