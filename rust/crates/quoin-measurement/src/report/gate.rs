// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! A `gate` plan's pass/fail verdict on its newest value (PLAT-958 part 2,
//! EA-26).
//!
//! Split from [`crate::report::verdict`] on responsibility (quoin#464's
//! module-size ceiling) when the interval-aware decision (FR-107-AC-10) took
//! it to the ceiling. It shares that module's `InconclusiveReason`
//! vocabulary and its `usable`/`earlier_values`/`same_apparatus` evidence
//! pool, so nothing is restated here.
//!
//! # Interval-aware
//!
//! A rule that states an `interval_level` is decided by
//! [`crate::interval::judge`] — engineering-assurance's
//! `DecisionRule::holds_on_interval` at the unfavourable bound of the newest
//! observation's own `interval` — never by the point estimate. The report
//! layer only ever reports `inconclusive` for a missing, malformed or short
//! interval and leaves rejection to `quoin measurement verify`. The interval
//! is judged before the baseline is resolved, so a missing interval is
//! reported even when the baseline has no prior.

use std::collections::BTreeMap;

use engineering_assurance::measurement::{Baseline, Comparator, DecisionRule, RuleReference};
use quoin_store::JsonValue;

use crate::interval::{self, Decided, Refusal};
use crate::report::verdict::{InconclusiveReason, earlier_values, same_apparatus, usable};
use crate::types::collection::MeasurementCollection;
use crate::types::observation::MeasurementObservation;
use crate::types::plan::MeasurementPlan;

/// A `gate` plan's pass/fail verdict on its newest value (PLAT-958 part 2).
///
/// Unlike `RatchetOutcome` and `TargetOutcome`, this is the one outcome
/// the report layer decides that is a real pass/fail verdict, not information
/// — see [`crate::report::verdict`]'s module header.
#[derive(Clone, Debug, PartialEq)]
pub enum GateOutcome {
    /// The decision rule holds for the newest value.
    Pass {
        /// The newest value.
        current: f64,
        /// The baseline value the rule was evaluated against, when its
        /// reference is a `baseline`; `None` for a `threshold` rule.
        baseline: Option<f64>,
        /// The interval bound that decided it, when the rule states an
        /// `interval_level` (FR-107-AC-10); `None` otherwise.
        interval: Option<Decided>,
    },
    /// The decision rule does not hold for the newest value.
    Fail {
        /// The newest value.
        current: f64,
        /// As [`Self::Pass`]'s.
        baseline: Option<f64>,
        /// As [`Self::Pass`]'s.
        interval: Option<Decided>,
    },
    /// No verdict, for this reason.
    Inconclusive(InconclusiveReason),
}

impl GateOutcome {
    /// The stable wire spelling of the verdict.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match *self {
            Self::Pass { .. } => "pass",
            Self::Fail { .. } => "fail",
            Self::Inconclusive(_) => "inconclusive",
        }
    }
}

/// The inconclusive reason an interval refusal is reported as. The report
/// layer never rejects: a malformed interval is as good as none here, and
/// `quoin measurement verify` is what rejects it.
const fn refusal_reason(refusal: Refusal) -> InconclusiveReason {
    match refusal {
        Refusal::Unstated | Refusal::Malformed => InconclusiveReason::IntervalUnstated,
        Refusal::LevelShort => InconclusiveReason::IntervalLevelShort,
        Refusal::EstimateOutside | Refusal::Unevaluable => InconclusiveReason::RuleNotEvaluable,
    }
}

/// A `gate` plan's verdict on `observation`: entirely from
/// `statistical_design.decision_rule`, never from `objective.bound`. A
/// `baseline` rule refuses a baseline across a changed protected apparatus
/// exactly as a ratchet refuses a floor (`same_apparatus`).
pub(super) fn gate(
    plan: &MeasurementPlan,
    observation: Option<&MeasurementObservation>,
    collection: Option<&MeasurementCollection>,
    earlier: &[MeasurementCollection],
) -> GateOutcome {
    let (observation, current) = match usable(plan, observation) {
        Ok(found) => found,
        Err(reason) => return GateOutcome::Inconclusive(reason),
    };
    let Some(rule) = plan
        .statistical_design
        .and_then(|design| design.decision_rule)
    else {
        return GateOutcome::Inconclusive(InconclusiveReason::NoDecisionRule);
    };
    let baseline_value = match rule.reference() {
        RuleReference::Threshold(_) => Ok(None),
        RuleReference::Baseline { baseline, .. } => {
            let slice = observation.dimensions.entries();
            same_apparatus(plan, observation, collection, earlier)
                .and_then(|()| gate_baseline(plan, rule, baseline, slice, earlier))
                .map(Some)
        }
    };
    // The interval requirement does not wait on the baseline (FR-107-AC-10):
    // an unresolved baseline leaves `None`, and EA checks the interval before
    // it looks at the reference, so an interval refusal still surfaces first.
    let on_interval = interval::judge(
        &rule,
        current,
        observation,
        baseline_value.as_ref().ok().copied().flatten(),
    );
    if let Some(Err(refusal)) = on_interval
        && !(refusal == Refusal::Unevaluable && baseline_value.is_err())
    {
        return GateOutcome::Inconclusive(refusal_reason(refusal));
    }
    let baseline_value = match baseline_value {
        Ok(value) => value,
        Err(reason) => return GateOutcome::Inconclusive(reason),
    };
    let (holds, decided) = match on_interval {
        Some(Ok(decided)) => (decided.holds, Some(decided)),
        Some(Err(_)) => return GateOutcome::Inconclusive(InconclusiveReason::RuleNotEvaluable),
        None => match rule.holds(current, baseline_value) {
            Ok(holds) => (holds, None),
            Err(_) => return GateOutcome::Inconclusive(InconclusiveReason::RuleNotEvaluable),
        },
    };
    if holds {
        GateOutcome::Pass {
            current,
            baseline: baseline_value,
            interval: decided,
        }
    } else {
        GateOutcome::Fail {
            current,
            baseline: baseline_value,
            interval: decided,
        }
    }
}

/// The baseline value `rule`'s reference asks for, over `earlier`'s usable
/// values of `slice`: the nearest earlier usable value for
/// `prior-collection`, and the maximum (`gt`/`ge`) or minimum
/// (`lt`/`le`/`eq`) for `best-seen` — the same pool and the same rule
/// [`crate::verify`]'s own `baseline` applies, restated here because the
/// report layer sees one row's `earlier` collections, not the checker's
/// full, order-attested history. `constant-predictor` and `external-reference`
/// are not resolvable from `earlier` at all, so both return their own reason
/// instead of a value (PLAT-1032).
fn gate_baseline(
    plan: &MeasurementPlan,
    rule: DecisionRule,
    baseline: Baseline,
    slice: &BTreeMap<String, JsonValue>,
    earlier: &[MeasurementCollection],
) -> Result<f64, InconclusiveReason> {
    let found = match baseline {
        Baseline::ConstantPredictor => {
            return Err(InconclusiveReason::ConstantPredictorUnsupported);
        }
        Baseline::ExternalReference => return Err(InconclusiveReason::ExternalReferenceUnsupplied),
        Baseline::PriorCollection => earlier_values(plan, slice, earlier).last(),
        Baseline::BestSeen => {
            let values = earlier_values(plan, slice, earlier);
            match rule.comparator() {
                Comparator::Gt | Comparator::Ge => values.max_by(|l, r| l.0.total_cmp(&r.0)),
                Comparator::Lt | Comparator::Le | Comparator::Eq => {
                    values.min_by(|l, r| l.0.total_cmp(&r.0))
                }
            }
        }
    };
    found
        .map(|(value, _)| value)
        .ok_or(InconclusiveReason::NoPrior)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    /// The gate reads an observation's stored `value` as its estimate, and a
    /// stored value outside its own interval is a malformed interval, so
    /// `EstimateOutside` cannot arise from a stored observation here; the
    /// mapping still holds FR-107-AC-10's `rule_not_evaluable` for it.
    ///
    /// Trace: FR-107-AC-10
    /// Provenance: EA-26
    #[test]
    fn each_interval_refusal_maps_to_its_inconclusive_reason() {
        for (refusal, reason) in [
            (Refusal::Unstated, InconclusiveReason::IntervalUnstated),
            (Refusal::Malformed, InconclusiveReason::IntervalUnstated),
            (Refusal::LevelShort, InconclusiveReason::IntervalLevelShort),
            (
                Refusal::EstimateOutside,
                InconclusiveReason::RuleNotEvaluable,
            ),
            (Refusal::Unevaluable, InconclusiveReason::RuleNotEvaluable),
        ] {
            assert_eq!(refusal_reason(refusal), reason, "{refusal:?}");
        }
    }
}
