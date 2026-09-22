// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Slices a `ratchet` plan measured before that its newest collection drops
//! (PLAT-958).
//!
//! Split from [`crate::report::verdict`] so both stay under this crate's
//! module-size ceiling. "Usable" is that module's one definition.

use std::collections::BTreeMap;

use quoin_store::JsonValue;

use crate::report::verdict::{InconclusiveReason, RatchetOutcome, StageVerdict, usable};
use crate::types::collection::MeasurementCollection;
use crate::types::plan::{GroundTruthKind, MeasurementPlan, MeasurementStage};

/// A slice a `ratchet` plan measured usably in an earlier collection that the
/// newest collection does not measure at all.
///
/// A ratchet that only looked at the slices still present would let a slice
/// regress by omission: stop reporting it and it can never be `regressed`. So
/// each such slice is reported, `inconclusive` for `no_current_value`.
#[derive(Clone, Debug, PartialEq)]
pub struct VanishedSlice {
    /// The metric the governing plan names.
    pub metric: String,
    /// The governing plan's identity.
    pub plan_id: String,
    /// Where the governing plan is authored.
    pub plan_path: String,
    /// How the governing plan's ground truth was produced, when it says.
    pub plan_ground_truth_kind: Option<GroundTruthKind>,
    /// The slice's dimensions, as the earlier collection stated them.
    pub dimensions: BTreeMap<String, JsonValue>,
    /// Always a `ratchet` verdict, `inconclusive` for `no_current_value`.
    pub stage_verdict: StageVerdict,
}

/// Every slice of a `ratchet` plan with an `objective` that some earlier
/// collection measured usably and `newest` — the newest collection measuring
/// the plan's metric — does not measure, in first-seen order.
#[must_use]
pub fn vanished_slices(
    plan: &MeasurementPlan,
    newest: Option<&MeasurementCollection>,
    earlier: &[MeasurementCollection],
) -> Vec<VanishedSlice> {
    let (Some(objective), MeasurementStage::Ratchet, Some(newest)) =
        (plan.objective, plan.stage, newest)
    else {
        return Vec::new();
    };
    let present: Vec<&BTreeMap<String, JsonValue>> = newest
        .observations
        .iter()
        .filter(|observation| observation.metric.as_str() == plan.metric.as_str())
        .map(|observation| observation.dimensions.entries())
        .collect();
    let mut vanished: Vec<&BTreeMap<String, JsonValue>> = Vec::new();
    for observation in earlier
        .iter()
        .flat_map(|collection| &collection.observations)
    {
        let slice = observation.dimensions.entries();
        if observation.metric.as_str() == plan.metric.as_str()
            && usable(plan, Some(observation)).is_ok()
            && !present.contains(&slice)
            && !vanished.contains(&slice)
        {
            vanished.push(slice);
        }
    }
    vanished
        .into_iter()
        .map(|dimensions| VanishedSlice {
            metric: plan.metric.as_str().to_owned(),
            plan_id: plan.id.as_str().to_owned(),
            plan_path: plan.path.clone(),
            plan_ground_truth_kind: plan.ground_truth_kind,
            dimensions: dimensions.clone(),
            stage_verdict: StageVerdict::Ratchet {
                objective,
                outcome: RatchetOutcome::Inconclusive(InconclusiveReason::NoCurrentValue),
            },
        })
        .collect()
}
