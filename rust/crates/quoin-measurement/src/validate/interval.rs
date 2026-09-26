// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The per-observation `interval` checks at intake (EA-26, FR-044-AC-10 and
//! FR-044-AC-11).
//!
//! Quoin computes no interval: a producer states one, and this only decides
//! whether the stated one is well-formed ([`crate::interval::reading`]) and,
//! under a plan whose `decision_rule` states an `interval_level`, whether a
//! `measured` observation states one at that level or above. A malformed
//! interval is [`Code::IntervalMalformed`] only — the level checks are never
//! also raised for it.

use quoin_store::JsonValue;

use crate::common::scalar::js_f64_string;
use crate::error::MeasurementErrorCode as Code;
use crate::interval::{IntervalReading, reading};
use crate::json_bridge::to_serde;
use crate::report::render::metric_label;
use crate::types::observation::{MeasurementObservation, MeasurementState};
use crate::types::plan::MeasurementPlan;

/// One typed interval finding.
type Finding = (Code, String);

/// Every interval finding for one observation.
///
/// `plan` is the plan whose own metric this observation reports. It is `None`
/// for a constant-predictor item row (FR-108-AC-10), which is one graded
/// item and not a population, so it is held to well-formedness only.
pub(super) fn findings(
    observation: &MeasurementObservation,
    plan: Option<&MeasurementPlan>,
) -> Vec<Finding> {
    let label = metric_label(
        observation.metric.as_str(),
        observation.dimensions.entries(),
    )
    .unwrap_or_else(|_| observation.metric.as_str().to_owned());
    let interval = match reading(observation) {
        IntervalReading::Malformed(reason) => {
            let stored = observation.interval.as_ref().unwrap_or(&JsonValue::Null);
            // A stored value always crosses the bridge (every JSON number here
            // is finite); the type name is the fallback, not a refusal.
            let spelled =
                to_serde(stored).map_or_else(|_| stored.type_name().to_owned(), |v| v.to_string());
            return vec![(
                Code::IntervalMalformed,
                format!("metric `{label}` states interval {spelled}; {reason}"),
            )];
        }
        IntervalReading::Absent => None,
        IntervalReading::Stated(interval) => Some(interval),
    };
    let required = plan
        .filter(|_| observation.state == MeasurementState::Measured)
        .and_then(|plan| {
            let level = plan
                .statistical_design
                .and_then(|design| design.decision_rule)
                .and_then(|rule| rule.interval_level())?;
            Some((plan, level))
        });
    let Some((plan, required)) = required else {
        return Vec::new();
    };
    match interval {
        None => vec![(
            Code::IntervalUnstated,
            format!(
                "metric `{label}` states no interval; plan {} requires an interval at level {required} or above",
                plan.id
            ),
        )],
        Some(interval) if interval.level() < required => vec![(
            Code::IntervalLevelShort,
            format!(
                "metric `{label}` states an interval at level {} but plan {} requires {required}",
                js_f64_string(interval.level().get()),
                plan.id
            ),
        )],
        Some(_) => Vec::new(),
    }
}
