// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A paired-run plan's verdict (quoin FR-132-AC-8 to AC-12).
//!
//! Split from [`super`] so the checker stays under this crate's module-size
//! ceiling. The count is [`crate::compare_latest_pair`]'s, the same
//! comparison `quoin report` prints; this module only maps it to the
//! checker's reasons. [`judge`] is the one place the latest pair is chosen and
//! its order attested: `verify` maps its result to reasons and
//! [`report_pair`] maps it to the report's row, so the two cannot differ.

use std::collections::BTreeMap;

use crate::paired::{
    PairedRunObservation, PairedRunOutcome, compare_latest_pair, measures, order_matters,
};
use crate::types::collection::MeasurementCollection;
use crate::types::plan::MeasurementPlan;

use super::order::{self, Placed};
use super::{Estimate, EstimateBasis, Finding, OrderSource, Ranked, Reason, SliceDecision, id_of};

/// What the pair added to the verdict.
pub(super) struct PairedVerdict {
    /// The pair as `compare_latest_pair` observed it.
    pub observation: PairedRunObservation,
    /// The reasons the pair gives, none when the relation holds.
    pub findings: Vec<Finding>,
    /// The count as a decision, when the pair was observed.
    pub decision: Option<SliceDecision>,
}

/// Judge `plan`'s latest two `runs`, or `None` when it declares no
/// `paired_run`.
pub(super) fn judge<P: Placed>(
    plan: &MeasurementPlan,
    runs: &[P],
    order: OrderSource,
) -> Option<PairedVerdict> {
    let paired = plan.paired_run?;
    let collections: Vec<&MeasurementCollection> = runs.iter().map(Placed::collection).collect();
    let observation = compare_latest_pair(plan, &collections)?;
    let candidate = runs.last().map(|run| id_of(run.collection()));
    let finding = |reason| Finding {
        reason,
        collection_id: candidate.clone(),
        dimensions: None,
    };
    let mut findings = Vec::new();
    let mut decision = None;
    match observation.outcome.clone() {
        PairedRunOutcome::Observed { differing } => {
            if differing > 0 {
                findings.push(finding(Reason::PairedRunDiffers));
            }
            decision = Some(SliceDecision {
                dimensions: BTreeMap::new(),
                estimate: Estimate {
                    value: f64::from(u32::try_from(differing).unwrap_or(u32::MAX)),
                    basis: EstimateBasis::Recomputed,
                },
                baseline: Some(0.0),
                holds: Some(differing == 0),
                interval: None,
                paired: true,
            });
        }
        PairedRunOutcome::Incomparable => findings.push(finding(Reason::PairedRunIncomparable)),
        PairedRunOutcome::Inconclusive => findings.push(finding(Reason::PairedRunInconclusive)),
        PairedRunOutcome::Unpaired => findings.push(finding(Reason::PairedRunUnpaired)),
    }
    // Which two runs are the latest only matters when it can change the answer;
    // a shallow clone attests no order for any plan (FR-108-AC-8).
    let last = runs.len().saturating_sub(1);
    let shallow =
        order == OrderSource::GitShallow && observation.outcome != PairedRunOutcome::Unpaired;
    if shallow
        || (order_matters(paired, runs.len())
            && runs.len() >= 2
            && (order::unattested(runs, last) || order::unattested(runs, last.saturating_sub(1))))
    {
        findings.push(finding(Reason::OrderUnattested));
    }
    Some(PairedVerdict {
        observation,
        findings,
        decision,
    })
}

/// `plan`'s paired-run row for the report, over the collections `ranked` by an
/// intake order whose source is `order`.
///
/// The collections are judged by [`judge`], as `verify` judges them. Where
/// that finds the order of the latest two unattested, the pair is
/// `inconclusive` with `order_unattested` and states no count, because the
/// pair `verify` would name is not known.
pub(crate) fn report_pair(
    plan: &MeasurementPlan,
    ranked: &[Ranked<'_>],
    order: OrderSource,
) -> Option<PairedRunObservation> {
    let mut runs: Vec<Ranked<'_>> = ranked
        .iter()
        .filter(|ranked| measures(plan, ranked.collection))
        .copied()
        .collect();
    order::sort(&mut runs);
    let verdict = judge(plan, &runs, order)?;
    let mut pair = verdict.observation;
    if verdict
        .findings
        .iter()
        .any(|finding| finding.reason == Reason::OrderUnattested)
    {
        pair.outcome = PairedRunOutcome::Inconclusive;
        pair.reasons.push(
            "order_unattested: the intake order of the latest collections is not attested \
             (FR-108-AC-2)"
                .to_owned(),
        );
    }
    Some(pair)
}
