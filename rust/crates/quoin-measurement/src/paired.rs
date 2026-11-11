// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Paired-run metrics (PLAT-114): a metric that states two runs agree is an
//! observation computed from a comparison of two collections, never a single
//! run's assertion.
//!
//! There is no second comparison engine. Every slice goes through
//! [`compare_measurement_collections`], so a moved definition, a changed
//! build profile or a moved protected apparatus blocks the pair exactly as it
//! blocks any other comparison. This module only reduces the compared slices
//! to the plan's stated [`PairedRelation`].
//!
//! # The count is the observation
//!
//! The observed value is the number of compared slices of the plan's metric
//! that violate the relation; the metric's target is 0. A pair holding fewer
//! comparable slices than the plan's `minimum_pairs` is `Inconclusive` and
//! carries no count: zero comparisons must never read as zero variance.
//! A slice stated by one run only counts as a violation of every relation; a
//! slice either run states without a value makes the pair `Inconclusive`.

use quoin_store::StoreError;

use crate::compare::compare_measurement_collections;
use crate::report::verdict::badness;
use crate::types::collection::MeasurementCollection;
use crate::types::comparison::{ComparisonStatus, MeasurementComparison};
use crate::types::observation::{MeasurementObservation, MeasurementState};
use crate::types::plan::{MeasurementPlan, PairedRelation};

/// What a paired comparison established.
#[derive(Clone, Debug, PartialEq)]
pub enum PairedRunOutcome {
    /// Enough slices were compared; this many violated the relation.
    Observed {
        /// Slices that violate the relation. The metric's value.
        differing: u64,
    },
    /// Something blocked the pair: no count is offered.
    Incomparable,
    /// Fewer comparable slices than `minimum_pairs`: no count is offered.
    Inconclusive,
}

/// One plan's paired-run result.
#[derive(Clone, Debug, PartialEq)]
pub struct PairedRunObservation {
    /// The governing plan.
    pub plan_id: String,
    /// The plan's metric.
    pub metric: String,
    /// What was established.
    pub outcome: PairedRunOutcome,
    /// How many slices were compared and comparable.
    pub comparisons: u64,
    /// The plan's required minimum.
    pub minimum_pairs: u32,
    /// Why the outcome is not `Observed`; empty when it is.
    pub reasons: Vec<String>,
}

/// Compute every paired-run observation `plans` declare, from two collections.
///
/// A plan without `paired_run` yields nothing.
///
/// # Errors
///
/// [`StoreError`] as [`compare_measurement_collections`] returns it.
pub fn compare_paired_runs(
    first: &MeasurementCollection,
    second: &MeasurementCollection,
    plans: &[MeasurementPlan],
) -> Result<Vec<PairedRunObservation>, StoreError> {
    let rows = compare_measurement_collections(first, second, plans)?;
    Ok(plans
        .iter()
        .filter_map(|plan| {
            let paired = plan.paired_run?;
            let slices: Vec<&MeasurementComparison> = rows
                .iter()
                .filter(|row| row.metric == plan.metric.as_str())
                .collect();
            Some(reduce(
                plan,
                paired.relation,
                paired.minimum_pairs.get(),
                &slices,
                (first, second),
            ))
        })
        .collect())
}

fn reduce(
    plan: &MeasurementPlan,
    relation: PairedRelation,
    minimum_pairs: u32,
    slices: &[&MeasurementComparison],
    runs: (&MeasurementCollection, &MeasurementCollection),
) -> PairedRunObservation {
    let comparable: Vec<&&MeasurementComparison> = slices
        .iter()
        .filter(|row| row.status == ComparisonStatus::Comparable)
        .collect();
    let comparisons = u64::try_from(comparable.len()).unwrap_or(u64::MAX);
    let mut reasons: Vec<String> = slices
        .iter()
        .filter(|row| row.status == ComparisonStatus::Incomparable)
        .flat_map(|row| row.reasons.iter().filter(|reason| reason.blocking))
        .map(|reason| format!("{}: {}", reason.code.as_str(), reason.message))
        .collect();
    // A slice stated by one run only is the disagreement this metric exists to
    // catch; a slice a run states but did not compute is neither agreement nor
    // disagreement, so the pair cannot be called clean.
    let mut one_sided = 0_u64;
    let mut uncomputed = 0_u64;
    for row in slices
        .iter()
        .filter(|row| row.status == ComparisonStatus::NotComputed)
    {
        let stated = (stated_in(runs.0, row), stated_in(runs.1, row));
        match stated {
            (Some(a), Some(b))
                if a.state == MeasurementState::Measured
                    && b.state == MeasurementState::Measured => {}
            (Some(_), Some(_)) => uncomputed += 1,
            _ => one_sided += 1,
        }
    }
    let outcome = if !reasons.is_empty() {
        PairedRunOutcome::Incomparable
    } else if comparisons < u64::from(minimum_pairs) || uncomputed > 0 {
        reasons.push(format!(
            "inconclusive: {comparisons} comparable slice(s) of {}, {minimum_pairs} required, \
             {uncomputed} slice(s) not computed",
            plan.metric
        ));
        PairedRunOutcome::Inconclusive
    } else {
        let differing = comparable
            .iter()
            .filter(|row| violates(plan, relation, row))
            .count();
        PairedRunOutcome::Observed {
            differing: u64::try_from(differing)
                .unwrap_or(u64::MAX)
                .saturating_add(one_sided),
        }
    };
    PairedRunObservation {
        plan_id: plan.id.as_str().to_owned(),
        metric: plan.metric.as_str().to_owned(),
        outcome,
        comparisons,
        minimum_pairs,
        reasons,
    }
}

/// Whether one comparable slice breaks the relation. A slice without both
/// values cannot be comparable, so absence counts as violation rather than as
/// agreement.
fn violates(plan: &MeasurementPlan, relation: PairedRelation, row: &MeasurementComparison) -> bool {
    let (Some(before), Some(after)) = (row.before, row.after) else {
        return true;
    };
    match relation {
        PairedRelation::Equal => after.partial_cmp(&before) != Some(std::cmp::Ordering::Equal),
        PairedRelation::WithinDelta(delta) => (after - before).abs() > delta,
        PairedRelation::NotWorse => plan
            .objective
            .and_then(|objective| Some((badness(objective, before)?, badness(objective, after)?)))
            .is_none_or(|(was, now)| now > was),
    }
}

/// The observation `collection` states for `row`'s slice, if any.
fn stated_in<'a>(
    collection: &'a MeasurementCollection,
    row: &MeasurementComparison,
) -> Option<&'a MeasurementObservation> {
    collection.observations.iter().find(|observation| {
        observation.metric.as_str() == row.metric
            && observation.dimensions.entries() == &row.dimensions
    })
}
