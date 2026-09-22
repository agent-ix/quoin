// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The JSON view of a row's stage verdict (PLAT-958).
//!
//! Split from [`crate::report::wire`] so that module stays under this crate's
//! module-size ceiling. The shape is [`crate::report::verdict`]'s, projected
//! member by member; the bytes still come from
//! [`crate::report::wire::canonical_json_of`].

use engineering_assurance::measurement::Objective;
use serde::Serialize;

use crate::report::verdict::{InconclusiveReason, RatchetOutcome, StageVerdict, TargetOutcome};

/// A row's stage verdict (PLAT-958). Each stage states every one of its own
/// members, `null` where the verdict has no such number.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum StageVerdictWire<'a> {
    /// A `ratchet` plan's verdict.
    Ratchet(RatchetWire<'a>),
    /// A `target` plan's progress.
    Target(TargetWire),
}

/// `stage: "ratchet"`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RatchetWire<'a> {
    stage: &'static str,
    objective: Objective,
    /// `held`, `regressed` or `inconclusive`.
    verdict: &'static str,
    /// The inconclusive reason's code; `null` on a verdict.
    reason: Option<&'static str>,
    current: Option<f64>,
    best_prior: Option<BestPriorWire<'a>>,
}

/// The best earlier value a ratchet held against.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BestPriorWire<'a> {
    value: f64,
    collection_id: &'a str,
}

/// `stage: "target"`. Informational: `reached` is not a pass/fail verdict.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TargetWire {
    stage: &'static str,
    objective: Objective,
    /// `reached`, `not_reached` or `inconclusive`.
    verdict: &'static str,
    /// As [`RatchetWire`]'s.
    reason: Option<&'static str>,
    current: Option<f64>,
    distance: Option<f64>,
    reached: Option<bool>,
}

impl<'a> StageVerdictWire<'a> {
    /// The wire view of one verdict.
    pub(crate) fn of(verdict: &'a StageVerdict) -> Self {
        let stage = verdict.stage().as_str();
        let reason = verdict
            .inconclusive_reason()
            .map(InconclusiveReason::as_str);
        match verdict {
            StageVerdict::Ratchet { objective, outcome } => {
                let (current, best_prior) = match outcome {
                    RatchetOutcome::Held {
                        current,
                        best_prior,
                    }
                    | RatchetOutcome::Regressed {
                        current,
                        best_prior,
                    } => (
                        Some(*current),
                        Some(BestPriorWire {
                            value: best_prior.value,
                            collection_id: &best_prior.collection_id,
                        }),
                    ),
                    RatchetOutcome::Inconclusive(_) => (None, None),
                };
                Self::Ratchet(RatchetWire {
                    stage,
                    objective: *objective,
                    verdict: outcome.as_str(),
                    reason,
                    current,
                    best_prior,
                })
            }
            StageVerdict::Target { objective, outcome } => {
                let (current, distance, reached) = match *outcome {
                    TargetOutcome::Measured {
                        current,
                        distance,
                        reached,
                        ..
                    } => (Some(current), Some(distance), Some(reached)),
                    TargetOutcome::Inconclusive(_) => (None, None, None),
                };
                Self::Target(TargetWire {
                    stage,
                    objective: *objective,
                    verdict: outcome.as_str(),
                    reason,
                    current,
                    distance,
                    reached,
                })
            }
        }
    }
}
