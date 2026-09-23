// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Resolving and digesting a plan's protected apparatus when a collection is
//! written (PLAT-975, engineering-assurance FR-024).
//!
//! # Why intake resolves the set itself
//!
//! `verificationStack.artifacts` is the producer's own map. A producer can
//! leave a file out of it, and a comparison over that map alone would then
//! never see the file change. So intake walks the repository itself, resolves
//! every `protected_apparatus` entry of every plan governing an observation,
//! digests each file, and refuses the write when the producer's map omits a
//! resolved file ([`MeasurementErrorCode::ApparatusUndeclared`]) or disagrees
//! with its bytes. The resolved (path, digest) set is what intake records, in
//! `verificationStack.protectedApparatus`, so a later comparison reads the
//! apparatus as it was when the collection was written and never today's
//! disk.
//!
//! # The resolver (engineering-assurance FR-024)
//!
//! - A file entry names one regular file. Nothing there, or a directory
//!   there, names no file ([`MeasurementErrorCode::ApparatusUnresolved`]).
//! - A `<directory>/**` entry names every regular file under the directory,
//!   recursively, dotfiles included, and must name at least one.
//! - Every name is matched **exactly** against the directory listing, so the
//!   match is case-sensitive even on a case-insensitive filesystem, where a
//!   bare `stat` of `Answers.json` would find `answers.json`.
//! - A symlink — named by an entry, passed through on the way to it, or found
//!   under a directory entry — is refused and never followed
//!   ([`MeasurementErrorCode::ApparatusSymlink`]). Following one would digest
//!   bytes the repository does not hold, which is PLAT-969's F3 in another
//!   place.
//! - A file that cannot be digested, or anything under a directory entry that
//!   is neither a file, a directory nor a symlink, is
//!   [`MeasurementErrorCode::ApparatusUnreadable`].
//! - One plan's set holds at most [`MAX_PROTECTED_FILES`] files; past that
//!   the write is [`MeasurementErrorCode::ApparatusTooLarge`], because every
//!   resolved file becomes a member of the stored collection. One write's
//!   walks visit at most [`MAX_WALKED_DIRECTORIES`] directories, and every
//!   plan's sets together hold at most [`MAX_TOTAL_PROTECTED_FILES`] files,
//!   under the same code (PLAT-985).
//! - A resolved file whose repository-relative path a stored record could not
//!   hold — a name `ApparatusPath` refuses — is
//!   [`MeasurementErrorCode::ApparatusUnreadable`], so intake never writes a
//!   record its own reader refuses (PLAT-985).

mod walk;

use std::collections::BTreeMap;
use std::path::Path;

use quoin_store::{JsonObject, JsonValue};

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::types::collection::{MeasurementCollection, ResolvedApparatus};
use crate::types::plan::MeasurementPlan;
use walk::{Walk, disk_listing};

/// The most files one plan's protected apparatus may resolve to.
///
/// Each resolved file is one `path: digest` member of the stored collection,
/// about a hundred bytes; this bounds that member near 5 MB.
pub const MAX_PROTECTED_FILES: usize = 50_000;

/// The most directories one write's protected-apparatus walks may visit,
/// counting empty ones, summed across every plan the write governs (PLAT-985,
/// quoin#600 review).
///
/// [`MAX_PROTECTED_FILES`] bounds the *files* a `<directory>/**` entry
/// resolves to, but a directory that holds no file contributes nothing to
/// that count. A repository-relative tree of a million empty subdirectories
/// costs the walk a `readdir` each and resolves to zero files, so the file
/// cap alone never stops it. This bounds the walk itself, before counting
/// what it found. It is per write, not per plan, for the reason
/// [`MAX_TOTAL_PROTECTED_FILES`] is: a per-plan bound leaves the sum over
/// many plans unbounded.
pub(crate) const MAX_WALKED_DIRECTORIES: usize = 100_000;

/// The most files every plan's protected apparatus may resolve to, summed
/// across every plan one collection's write governs (PLAT-985, quoin#600
/// review).
///
/// [`MAX_PROTECTED_FILES`] bounds one plan alone; a write governed by many
/// plans, each just under that ceiling, has no bound on their sum without
/// this one.
pub(crate) const MAX_TOTAL_PROTECTED_FILES: usize = 200_000;

/// Resolve every governing plan's protected apparatus under `repo`, keyed by
/// plan id, and require the candidate's `verificationStack.artifacts` to
/// declare every resolved file at its digest.
///
/// A plan governs the collection when an observation's metric names it, the
/// same lookup [`crate::validate::measurement_collection`] admitted the
/// observation under. A plan with no `protected_apparatus` contributes
/// nothing, so a collection no protecting plan governs resolves to an empty
/// map and is written exactly as before PLAT-975.
///
/// # Errors
///
/// [`MeasurementErrorCode::ApparatusUnresolved`],
/// [`MeasurementErrorCode::ApparatusSymlink`],
/// [`MeasurementErrorCode::ApparatusUnreadable`] and
/// [`MeasurementErrorCode::ApparatusTooLarge`] from resolution, each naming
/// the plan and the entry;
/// [`MeasurementErrorCode::ApparatusUndeclared`] naming every resolved file
/// the artifacts map omits; and
/// [`MeasurementErrorCode::CollectionInvalid`] when it states a resolved
/// file at another digest.
pub(super) fn resolve_protected(
    repo: &Path,
    collection: &MeasurementCollection,
    plans: &[MeasurementPlan],
) -> Result<BTreeMap<String, ResolvedApparatus>, MeasurementError> {
    let by_metric: BTreeMap<&str, &MeasurementPlan> = plans
        .iter()
        .map(|plan| (plan.metric.as_str(), plan))
        .collect();
    let mut resolved = BTreeMap::new();
    // One budget for the whole write, so both write-wide limits bound the sum
    // over every plan rather than each plan alone.
    let mut spent = Spent::default();
    for observation in &collection.observations {
        let Some(plan) = by_metric.get(observation.metric.as_str()) else {
            continue;
        };
        let Some(protected) = plan.protected_apparatus.as_ref() else {
            continue;
        };
        if resolved.contains_key(plan.id.as_str()) {
            continue;
        }
        let mut files = ResolvedApparatus::new();
        for entry in protected.iter() {
            Walk {
                repo,
                plan: plan.id.as_str(),
                entry,
                files: &mut files,
                list: disk_listing,
                limits: LIMITS,
                spent: &mut spent,
            }
            .resolve()?;
        }
        declared(collection, plan.id.as_str(), &files)?;
        resolved.insert(plan.id.as_str().to_owned(), files);
    }
    Ok(resolved)
}

/// The ceilings one write's resolution runs under. A value rather than the
/// constants read directly, so each limit is tested at a small boundary
/// against a real directory tree instead of against 200,001 files on disk.
#[derive(Clone, Copy, Debug)]
struct Limits {
    /// The most files one plan's set may hold.
    plan_files: usize,
    /// The most directories every walk of one write may visit, together.
    directories: usize,
    /// The most files every plan's set may hold, together.
    total_files: usize,
}

/// The production [`Limits`].
const LIMITS: Limits = Limits {
    plan_files: MAX_PROTECTED_FILES,
    directories: MAX_WALKED_DIRECTORIES,
    total_files: MAX_TOTAL_PROTECTED_FILES,
};

/// What one write's resolution has spent so far, across every plan.
#[derive(Debug, Default)]
struct Spent {
    /// Directories visited, counting empty ones.
    directories: usize,
    /// Files resolved, each counted once per plan whose set holds it.
    files: usize,
}

/// Require `artifacts` to state every resolved file at its digest.
fn declared(
    collection: &MeasurementCollection,
    plan: &str,
    files: &ResolvedApparatus,
) -> Result<(), MeasurementError> {
    let artifacts = collection
        .verification_stack
        .as_ref()
        .map(|stack| &stack.artifacts);
    let mut undeclared = Vec::new();
    for (path, digest) in files {
        match artifacts.and_then(|artifacts| artifacts.get(path)) {
            None => undeclared.push(format!(
                "verificationStack.artifacts does not declare `{path}`, which plan {plan} \
                 protects; it digests to {}",
                digest.to_stored()
            )),
            // This runs before `verify_local_artifacts`, so for a protected
            // file this is the check that refuses a disagreeing digest, under
            // the same code PLAT-931's check uses for any other file.
            Some(stated) if stated != digest => {
                return Err(MeasurementError::new(
                    MeasurementErrorCode::CollectionInvalid,
                    format!(
                        "verificationStack.artifacts.{path} does not match the protected file \
                         plan {plan} names: the record says {}, the file digests to {}",
                        stated.to_stored(),
                        digest.to_stored()
                    ),
                ));
            }
            Some(_) => {}
        }
    }
    if undeclared.is_empty() {
        Ok(())
    } else {
        Err(MeasurementError::with_findings(
            MeasurementErrorCode::ApparatusUndeclared,
            format!("plan {plan}'s protected apparatus is not declared in the collection"),
            undeclared,
        ))
    }
}

/// The `verificationStack.protectedApparatus` member intake writes, or `None`
/// when no plan protects anything, so the member is absent rather than empty.
pub(super) fn protected_member(
    resolved: &BTreeMap<String, ResolvedApparatus>,
) -> Option<JsonValue> {
    if resolved.is_empty() {
        return None;
    }
    let mut plans = JsonObject::new();
    for (plan, files) in resolved {
        let mut digests = JsonObject::new();
        for (path, digest) in files {
            digests.set(path.clone(), JsonValue::string(digest.to_stored()));
        }
        plans.set(plan.clone(), JsonValue::Object(digests));
    }
    Some(JsonValue::Object(plans))
}
