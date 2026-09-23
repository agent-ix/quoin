// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The checker's protected-apparatus rule (quoin FR-110-AC-7, PLAT-975).
//!
//! Split from [`super`] so the checker stays under this crate's module-size
//! ceiling.
//!
//! Under a plan that protects apparatus, each run carries the resolved
//! (path, digest) set its collection recorded at intake. A changed set needs
//! a new `definition_version` (engineering-assurance FR-024), and every run
//! the checker considers shares the plan's, so within one series:
//!
//! - a run is a baseline only for runs that recorded the same set;
//! - an earlier run whose set differs from the candidate's, or that recorded
//!   none, is `apparatus_changed` — the evidence cannot carry a comparison,
//!   so the question stays open;
//! - when both recorded a set and they differ, and the plan declares the
//!   `apparatus-edit` negative control, it is `apparatus_edit` instead: the
//!   plan named exactly this as gaming it guards against, and the stored data
//!   shows it, so it is a contradiction and rejects;
//! - a candidate that recorded no set is `apparatus_unrecorded`.
//!
//! A run that recorded no set is missing evidence, never a contradiction, so
//! it only ever leaves the verdict open.

use engineering_assurance::measurement::NegativeControlKind;

use crate::types::collection::{MeasurementCollection, ResolvedApparatus};
use crate::types::plan::MeasurementPlan;

use super::{Finding, Reason, Run};

/// The set `collection` recorded for `plan`, when the plan protects
/// apparatus. `None` for a plan that protects nothing, so every run of such a
/// plan carries the same `None` and nothing here changes its verdict.
pub(super) fn recorded<'a>(
    plan: &MeasurementPlan,
    collection: &'a MeasurementCollection,
) -> Option<&'a ResolvedApparatus> {
    plan.protected_apparatus
        .as_ref()
        .and(collection.protected_apparatus_of(plan.id.as_str()))
}

/// The apparatus findings for the candidate, the last of `runs`.
pub(super) fn findings(plan: &MeasurementPlan, runs: &[Run<'_>]) -> Vec<Finding> {
    let (Some(_), Some((candidate, history))) = (&plan.protected_apparatus, runs.split_last())
    else {
        return Vec::new();
    };
    let at = |reason: Reason, collection: &MeasurementCollection| Finding {
        reason,
        collection_id: Some(collection.collection_id.as_str().to_owned()),
        dimensions: None,
    };
    let guarded = plan
        .negative_controls
        .as_ref()
        .is_some_and(|controls| controls.covers(NegativeControlKind::ApparatusEdit));
    let mut out = Vec::new();
    let Some(own) = candidate.apparatus else {
        out.push(at(Reason::ApparatusUnrecorded, candidate.collection));
        return out;
    };
    for run in history {
        match run.apparatus {
            Some(theirs) if theirs == own => {}
            Some(_) if guarded => out.push(at(Reason::ApparatusEdit, run.collection)),
            Some(_) | None => out.push(at(Reason::ApparatusChanged, run.collection)),
        }
    }
    out
}
