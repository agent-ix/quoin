// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The checker's protected-apparatus rule (quoin FR-110-AC-7, PLAT-975).
//!
//! Split from [`super`] so the checker stays under this crate's module-size
//! ceiling.
//!
//! Each run carries the resolved (path, digest) set its collection recorded
//! at intake, read whatever the plan declares today: deleting the plan's
//! `protected_apparatus` list must not switch protection off for runs that
//! were recorded under it. A series is protected when the plan declares a
//! list or any of its runs recorded a set. Every run the checker considers
//! shares the plan's `definition_version`, and a changed set needs a new one
//! (engineering-assurance FR-024), so within a protected series:
//!
//! - a run is a baseline only for runs that recorded the same set;
//! - an earlier run that recorded a different set than the candidate is
//!   `apparatus_edit`: the stored data contradicts the version bump FR-024
//!   makes unconditional, so it rejects whether or not the plan declares the
//!   `apparatus-edit` negative control;
//! - a run, earlier or the candidate, that recorded no set is
//!   `apparatus_unrecorded`: missing evidence, which never rejects.
//!
//! A plan that never protected anything has no recorded set on any run, so
//! nothing here changes its verdict.

use crate::types::collection::MeasurementCollection;
use crate::types::plan::MeasurementPlan;

use super::{Finding, Reason, Run};

/// Whether `plan`'s series is protected: the plan declares a list, or any
/// run recorded a set.
pub(super) fn protected(plan: &MeasurementPlan, runs: &[Run<'_>]) -> bool {
    plan.protected_apparatus.is_some() || runs.iter().any(|run| run.apparatus.is_some())
}

/// The apparatus findings for the candidate, the last of `runs`.
pub(super) fn findings(plan: &MeasurementPlan, runs: &[Run<'_>]) -> Vec<Finding> {
    let Some((candidate, history)) = runs.split_last() else {
        return Vec::new();
    };
    if !protected(plan, runs) {
        return Vec::new();
    }
    let at = |reason: Reason, collection: &MeasurementCollection| Finding {
        reason,
        collection_id: Some(collection.collection_id.as_str().to_owned()),
        dimensions: None,
    };
    let mut out = Vec::new();
    let Some(own) = candidate.apparatus else {
        out.push(at(Reason::ApparatusUnrecorded, candidate.collection));
        return out;
    };
    for run in history {
        match run.apparatus {
            Some(theirs) if theirs == own => {}
            Some(_) => out.push(at(Reason::ApparatusEdit, run.collection)),
            None => out.push(at(Reason::ApparatusUnrecorded, run.collection)),
        }
    }
    out
}
