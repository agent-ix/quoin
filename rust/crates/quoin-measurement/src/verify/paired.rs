// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A paired-run plan's verdict (quoin FR-132-AC-8 to AC-12).
//!
//! Split from [`super`] so the checker stays under this crate's module-size
//! ceiling. The count is [`crate::compare_latest_pair`]'s, the same
//! comparison `quoin report` prints; this module only maps it to the
//! checker's reasons.

use std::collections::BTreeMap;

use crate::paired::{PairedRunOutcome, compare_latest_pair, order_matters};
use crate::types::collection::MeasurementCollection;
use crate::types::plan::MeasurementPlan;

use super::{Estimate, EstimateBasis, Finding, Reason, Run, SliceDecision, id_of, order};

/// What the pair added to the verdict.
pub(super) struct PairedVerdict {
    /// The reasons the pair gives, none when the relation holds.
    pub findings: Vec<Finding>,
    /// The count as a decision, when the pair was observed.
    pub decision: Option<SliceDecision>,
}

/// Judge `plan`'s latest two `runs`, or `None` when it declares no
/// `paired_run`.
pub(super) fn judge(plan: &MeasurementPlan, runs: &[Run<'_>]) -> Option<PairedVerdict> {
    let paired = plan.paired_run?;
    let collections: Vec<&MeasurementCollection> = runs.iter().map(|run| run.collection).collect();
    let observation = compare_latest_pair(plan, &collections)?;
    let candidate = runs.last().map(|run| id_of(run.collection));
    let finding = |reason| Finding {
        reason,
        collection_id: candidate.clone(),
        dimensions: None,
    };
    let mut findings = Vec::new();
    let mut decision = None;
    match observation.outcome {
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
    // Which two runs are the latest only matters when it can change the answer.
    let last = runs.len().saturating_sub(1);
    if order_matters(paired, runs.len())
        && runs.len() >= 2
        && (order::unattested(runs, last) || order::unattested(runs, last.saturating_sub(1)))
    {
        findings.push(finding(Reason::OrderUnattested));
    }
    Some(PairedVerdict { findings, decision })
}
