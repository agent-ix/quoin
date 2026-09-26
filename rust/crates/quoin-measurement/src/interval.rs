// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! An observation's stated uncertainty interval, read and applied (EA-26).
//!
//! Quoin computes no interval and states no second copy of one's rules: the
//! value type, its validity and its unfavourable-bound decision are
//! engineering-assurance's ([`Interval`], [`DecisionRule::holds_on_interval`],
//! its FR-021-AC-12..14). This module adds only what is quoin's own — an
//! interval must contain the observation's stated `value` and is forbidden on
//! a `not_computed` observation (FR-044-AC-10) — and the one place that
//! translates EA's refusals into the three outcomes intake, `verify` and the
//! report gate each name in their own vocabulary.
//!
//! The retained `interval` stays a [`quoin_store::JsonValue`] on
//! [`MeasurementObservation`], exactly as stored, so a malformed one is still
//! shown by every view (FR-044-AC-12); [`reading`] is the only place it is
//! interpreted.

use engineering_assurance::measurement::{Comparator, DecisionRule, Interval, RuleEvaluationError};

use crate::common::scalar::js_f64_string;
use crate::json_bridge::to_serde;
use crate::types::observation::{MeasurementObservation, MeasurementState};

/// What an observation's stored `interval` member is.
#[derive(Clone, Debug, PartialEq)]
pub enum IntervalReading {
    /// The observation states no `interval`.
    Absent,
    /// A well-formed interval that satisfies FR-044-AC-10.
    Stated(Interval),
    /// A stated `interval` that does not; the reason, for a finding.
    Malformed(String),
}

/// Read `observation`'s stored `interval` against FR-044-AC-10.
///
/// A stored JSON `null` is a stated member and is malformed, as it is for
/// engineering-assurance's own readers.
#[must_use]
pub fn reading(observation: &MeasurementObservation) -> IntervalReading {
    let Some(stored) = &observation.interval else {
        return IntervalReading::Absent;
    };
    let parsed = to_serde(stored)
        .map_err(|_| "the value cannot be carried into JSON".to_owned())
        .and_then(|value| serde_json::from_value::<Interval>(value).map_err(|e| e.to_string()));
    let interval = match parsed {
        Ok(interval) => interval,
        Err(reason) => return IntervalReading::Malformed(reason),
    };
    match (observation.state, observation.value) {
        (MeasurementState::NotComputed, _) => IntervalReading::Malformed(
            "an interval is not allowed on a not_computed observation".to_owned(),
        ),
        (MeasurementState::Measured, Some(value)) if !interval.contains(value) => {
            IntervalReading::Malformed(format!(
                "value {} lies outside [{}, {}]",
                js_f64_string(value),
                js_f64_string(interval.lower()),
                js_f64_string(interval.upper())
            ))
        }
        (MeasurementState::Measured, _) => IntervalReading::Stated(interval),
    }
}

/// Which end of the interval decided a rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Bound {
    /// The lower bound, for `gt` and `ge`.
    Lower,
    /// The upper bound, for `lt` and `le`.
    Upper,
}

impl Bound {
    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lower => "lower",
            Self::Upper => "upper",
        }
    }
}

/// A rule decided on an interval's unfavourable bound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Decided {
    /// Whether the rule holds at that bound.
    pub holds: bool,
    /// The bound that decided it.
    pub bound: Bound,
    /// That bound's value.
    pub bound_value: f64,
    /// The confidence level the deciding interval states.
    pub level: f64,
}

/// Why a rule that states an `interval_level` could not be decided on the
/// observation's interval.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// The observation states no interval.
    Unstated,
    /// It states one that does not satisfy FR-044-AC-10.
    Malformed,
    /// Its level is below the rule's `interval_level`.
    LevelShort,
    /// The estimate lies outside the interval (engineering-assurance's
    /// `EstimateOutsideInterval`).
    EstimateOutside,
    /// Engineering-assurance could not evaluate the rule for another reason:
    /// a non-finite number, or a baseline rule given no baseline value.
    Unevaluable,
}

/// Decide `rule` for `estimate` on `observation`'s own interval, or `None`
/// when the rule states no `interval_level` and is judged on the point
/// estimate as before.
///
/// `DecisionRule::holds` is never called for a rule that states an
/// `interval_level`; EA refuses it (`IntervalRequired`), so the point
/// estimate cannot stand in for a missing interval. `baseline_value` is
/// `None` for a threshold rule, and for a baseline rule whose baseline could
/// not be resolved: EA checks the interval before it resolves the reference,
/// so an interval refusal still surfaces ([`Refusal::Unevaluable`] then only
/// means the missing baseline).
#[must_use]
pub fn judge(
    rule: &DecisionRule,
    estimate: f64,
    observation: &MeasurementObservation,
    baseline_value: Option<f64>,
) -> Option<Result<Decided, Refusal>> {
    rule.interval_level()?;
    let stated = match reading(observation) {
        IntervalReading::Absent => None,
        IntervalReading::Stated(interval) => Some(interval),
        IntervalReading::Malformed(_) => return Some(Err(Refusal::Malformed)),
    };
    Some(decide(rule, estimate, stated.as_ref(), baseline_value))
}

fn decide(
    rule: &DecisionRule,
    estimate: f64,
    stated: Option<&Interval>,
    baseline_value: Option<f64>,
) -> Result<Decided, Refusal> {
    let holds = match rule.holds_on_interval(estimate, stated, baseline_value) {
        Ok(holds) => holds,
        Err(RuleEvaluationError::MissingInterval { .. }) => return Err(Refusal::Unstated),
        Err(RuleEvaluationError::IntervalLevelTooLow { .. }) => return Err(Refusal::LevelShort),
        Err(RuleEvaluationError::EstimateOutsideInterval { .. }) => {
            return Err(Refusal::EstimateOutside);
        }
        Err(_) => return Err(Refusal::Unevaluable),
    };
    // `holds_on_interval` answered, so an interval was stated.
    let interval = stated.ok_or(Refusal::Unevaluable)?;
    let (bound, bound_value) = match rule.comparator() {
        Comparator::Gt | Comparator::Ge => (Bound::Lower, interval.lower()),
        Comparator::Lt | Comparator::Le => (Bound::Upper, interval.upper()),
        // EA refuses an `eq` rule that states an `interval_level`.
        Comparator::Eq => return Err(Refusal::Unevaluable),
    };
    Ok(Decided {
        holds,
        bound,
        bound_value,
        level: interval.level().get(),
    })
}
