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
//! - a collection once added to the store and later removed — by a commit,
//!   or in the uncommitted work tree ([`deleted_and_edited`]'s `deleted`);
//! - a collection's stored file changed after intake added it — by a later
//!   commit, by a delete and re-add under the same id, or in the uncommitted
//!   work tree ([`deleted_and_edited`]'s `edited`);
//! - a collection's recorded protected-apparatus digest disagreeing with
//!   `git show <sourceRevision>:<path>` ([`forged_apparatus`]);
//! - the plan's own document changed its objective, estimator, decision rule
//!   or protected apparatus across two revisions — committed, or the work
//!   tree against the last commit — that share a `definition_version`
//!   ([`definition_changed`]).
//!
//! Every check degrades to "nothing found" rather than an error when git
//! cannot answer — outside a work tree, a shallow clone missing the revision
//! in question, or a path git does not track — the same posture
//! [`super::intake_order`] takes for the order itself: a caller with no git
//! access attests nothing, rather than manufacturing a false tamper finding.
//!
//! Every check attests the history this clone presents. A history rewritten
//! and force-pushed so the tampering commits never existed is out of reach of
//! anything reading that history; branch protection on the store's branch is
//! what closes that, not this module.
//!
//! Any value read from a collection file (`sourceRevision` above all) is
//! attacker-controlled: it reaches git only after `--end-of-options`, so a
//! revision spelled like `--output=<file>` is a revision git cannot resolve,
//! never an option.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use quoin_measurement::MeasurementPlan;
use quoin_measurement::plans::{PlanLoadOptions, definition_changed_without_version_bump};
use quoin_measurement::source::{DiskMeasurement, MeasurementSource as _};

use super::MEASUREMENTS_DIRECTORY;

/// One collection the store's git history shows was tampered with — removed,
/// or changed after intake added it — and the plan ids its observations
/// named in any content read for it.
pub(super) struct TamperedCollection {
    id: String,
    plan_ids: BTreeSet<String>,
}

/// `collections` in `measurement.verify`'s `TamperedCollectionRequest` shape.
pub(super) fn to_wire(collections: Vec<TamperedCollection>) -> serde_json::Value {
    collections
        .into_iter()
        .map(|collection| {
            serde_json::json!({ "id": collection.id, "plan_ids": collection.plan_ids })
        })
        .collect()
}

fn git(repo: &str, arguments: &[&str]) -> Result<std::process::Output, String> {
    std::process::Command::new("git")
        .args(["-C", repo])
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot run git: {error}"))
}

/// `git show <revision>:<path>`'s bytes, or `None` when git cannot answer.
/// `revision` may be attacker-controlled, so it is passed after
/// `--end-of-options` and can never be read as an option.
fn show(repo: &str, revision: &str, path: &str) -> Option<Vec<u8>> {
    let output = git(
        repo,
        &["show", "--end-of-options", &format!("{revision}:{path}")],
    )
    .ok()?;
    output.status.success().then_some(output.stdout)
}

/// What the store's history says happened to one collection id.
#[derive(Default)]
struct History {
    /// The commit that first added it, when the history reaches that far.
    first_added: Option<String>,
    /// The revision holding its content just before its most recent removal,
    /// while it is still removed.
    removed_from: Option<String>,
    /// Whether anything changed it after it was first added.
    edited: bool,
}

/// Every collection id the store's git history added and later removed
/// (`deleted`, PLAT-985), and every id changed after intake first added it
/// (`edited`, PLAT-985) — each with the plan ids its observations named.
///
/// One `git log` walks `--diff-filter=AMD` over the measurements directory,
/// oldest first. A `M` after the first `A`, or a second `A` (a delete and
/// re-add under the same id, which `--no-renames` would otherwise let pass as
/// a fresh intake), marks the id edited; a `D` marks it removed until a later
/// `A` puts it back. The uncommitted work tree is then compared with `HEAD`,
/// since `measurement.verify` reads the work tree: a modified file is
/// edited, a removed one is deleted. A path this crate cannot key to a
/// collection id (nested, or not `.json`) is ignored, matching
/// [`super::intake_order`].
///
/// Plan ids are read from the content intake first added *and* the content
/// last seen (before removal, or on disk now), so neither an edit that
/// re-targets the observations nor one that corrupts the file before
/// deleting it can leave the tampering unattributed to the plan it hid from.
pub(super) fn deleted_and_edited(
    repo: &str,
) -> Result<(Vec<TamperedCollection>, Vec<TamperedCollection>), String> {
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
        // `intake_order` already reported the git problem (a non-work-tree
        // or shallow clone); this check alone reports nothing rather than an
        // error.
        return Ok((Vec::new(), Vec::new()));
    }
    let mut histories: BTreeMap<String, History> = BTreeMap::new();
    let mut commit = String::new();
    for line in String::from_utf8_lossy(&log.stdout).lines() {
        if let Some(hash) = line.strip_prefix("commit ") {
            hash.clone_into(&mut commit);
            continue;
        }
        let Some((status, id)) = status_line(line) else {
            continue;
        };
        let history = histories.entry(id).or_default();
        match status {
            'A' if history.first_added.is_none() && history.removed_from.is_none() => {
                history.first_added = Some(commit.clone());
            }
            'A' => {
                history.edited = true;
                history.removed_from = None;
            }
            'M' => history.edited = true,
            'D' => history.removed_from = Some(format!("{commit}^")),
            _ => {}
        }
    }
    // The work tree against `HEAD`: `measurement.verify` reads the work
    // tree, so an uncommitted edit or removal is as much tampering as a
    // committed one. A new, uncommitted collection (`A`, or untracked) is an
    // intake not yet committed, not a change to one.
    if let Ok(diff) = git(
        repo,
        &[
            "diff",
            "--no-renames",
            "--relative",
            "--name-status",
            "HEAD",
            "--",
            MEASUREMENTS_DIRECTORY,
        ],
    ) && diff.status.success()
    {
        for line in String::from_utf8_lossy(&diff.stdout).lines() {
            let Some((status, id)) = status_line(line) else {
                continue;
            };
            let history = histories.entry(id).or_default();
            match status {
                'M' => history.edited = true,
                'D' => history.removed_from = Some("HEAD".to_owned()),
                _ => {}
            }
        }
    }
    let mut deleted = Vec::new();
    let mut edited = Vec::new();
    for (id, history) in histories {
        let first = history
            .first_added
            .as_deref()
            .map(|commit| plan_ids_at(repo, commit, &id))
            .unwrap_or_default();
        if let Some(before) = &history.removed_from {
            let mut plan_ids = first.clone();
            plan_ids.extend(plan_ids_at(repo, before, &id));
            deleted.push(TamperedCollection {
                id: id.clone(),
                plan_ids,
            });
        }
        if history.edited && history.removed_from.is_none() {
            let mut plan_ids = first;
            let path = Path::new(repo)
                .join(MEASUREMENTS_DIRECTORY)
                .join(format!("{id}.json"));
            if let Ok(bytes) = std::fs::read(path) {
                plan_ids.extend(plan_ids_of(&bytes));
            }
            edited.push(TamperedCollection { id, plan_ids });
        }
    }
    Ok((deleted, edited))
}

/// A `--name-status` line's status letter and the collection id its path
/// names, or `None` for anything else.
fn status_line(line: &str) -> Option<(char, String)> {
    let (status, path) = line.split_once('\t')?;
    Some((status.chars().next()?, id_of(path)?))
}

/// The `MeasurementPlan` ids collection `id`'s observations named at
/// `revision`. Empty when that content cannot be read or parsed — an
/// unattributed collection is not reported against any plan, rather than
/// every plan.
fn plan_ids_at(repo: &str, revision: &str, id: &str) -> BTreeSet<String> {
    show(
        repo,
        revision,
        &format!("{MEASUREMENTS_DIRECTORY}/{id}.json"),
    )
    .map(|bytes| plan_ids_of(&bytes))
    .unwrap_or_default()
}

/// The `planId` of every observation in a collection document's bytes.
fn plan_ids_of(bytes: &[u8]) -> BTreeSet<String> {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return BTreeSet::new();
    };
    value
        .get("observations")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|observation| observation.get("planId"))
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect()
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
/// collection in the store ever named. The `git show` calls this spends are
/// still bounded overall by intake's own resolver ceilings
/// (`MAX_PROTECTED_FILES`, `MAX_TOTAL_PROTECTED_FILES`), which is what stops
/// a plan's own recorded set — and so this walk — from growing unbounded.
///
/// A `sourceRevision` git cannot resolve leaves the collection unattested
/// rather than forged (FR-108 "Known limits"): a squash-merged branch's
/// commits are legitimately absent from a fresh clone.
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
            // The revision, or the path within it, not being reachable is
            // unattested rather than forged (a shallow clone, most often).
            let Some(committed) = show(repo, revision, file_path) else {
                continue;
            };
            let actual = quoin_store::digest_bytes_sha256(&committed).to_stored();
            if actual != recorded {
                forged.push(id.clone());
                break;
            }
        }
    }
    forged
}

/// Whether `plan_id`'s document changed its objective, estimator, decision
/// rule or protected apparatus between two revisions that share a
/// `definition_version` (engineering-assurance's
/// `definition_change_without_version_bump`, PLAT-985).
///
/// The plan's current document is located through `quoin-measurement`'s own
/// assurance-document walk (the same one `measurement.verify` loads plans
/// from). Every revision `git log --follow` finds for it is read at the path
/// the document had *in that commit*, so a rename does not hide the
/// revisions before it, and compared with the one before it, oldest first;
/// the work-tree document `measurement.verify` will actually use is compared
/// with the last committed one. Two adjacent revisions naming different plan
/// ids are not compared. `false` when the plan cannot be found, its history
/// cannot be read, or every adjacent pair agrees — including a plan never
/// committed, or committed once and never changed.
pub(super) fn definition_changed(repo: &str, plan_id: &str) -> bool {
    let Some((path, current)) = plan_document(repo, plan_id) else {
        return false;
    };
    let mut previous: Option<MeasurementPlan> = None;
    for (commit, path_then) in plan_revisions(repo, &path) {
        let Some(bytes) = show(repo, &commit, &path_then) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let Ok(Some(revision)) =
            quoin_measurement::plans::plan_from_text(&path_then, &text, PlanLoadOptions::default())
        else {
            continue;
        };
        if changed_without_bump(previous.as_ref(), &revision) {
            return true;
        }
        previous = Some(revision);
    }
    changed_without_bump(previous.as_ref(), &current)
}

fn changed_without_bump(before: Option<&MeasurementPlan>, after: &MeasurementPlan) -> bool {
    before.is_some_and(|before| {
        before.id == after.id && definition_changed_without_version_bump(before, after)
    })
}

/// Every commit touching the document now at `path`, oldest first, with
/// the path it had in that commit (`git log --follow --name-only`). Empty
/// when git cannot answer.
fn plan_revisions(repo: &str, path: &str) -> Vec<(String, String)> {
    let Ok(log) = git(
        repo,
        &[
            "log",
            "--follow",
            "--format=commit %H",
            "--name-only",
            "--",
            path,
        ],
    ) else {
        return Vec::new();
    };
    if !log.status.success() {
        return Vec::new();
    }
    // Newest first as git walks it; a commit with no name line (a merge
    // git shows no diff for) keeps the path of the commit after it.
    let mut newest_first: Vec<(String, Option<String>)> = Vec::new();
    for line in String::from_utf8_lossy(&log.stdout).lines() {
        if let Some(hash) = line.strip_prefix("commit ") {
            newest_first.push((hash.to_owned(), None));
        } else if !line.is_empty()
            && let Some((_, name @ None)) = newest_first.last_mut()
        {
            *name = Some(line.to_owned());
        }
    }
    let mut known = path.to_owned();
    let mut revisions: Vec<(String, String)> = newest_first
        .into_iter()
        .map(|(commit, name)| {
            if let Some(name) = name {
                known = name;
            }
            (commit, known.clone())
        })
        .collect();
    revisions.reverse();
    revisions
}

/// The repository-relative document path of the `MeasurementPlan` named
/// `plan_id`, and that plan as the work tree states it — read the same way
/// `measurement.verify` will load it — or `None` when no such plan exists.
fn plan_document(repo: &str, plan_id: &str) -> Option<(String, MeasurementPlan)> {
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
            return Some((path, plan));
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
