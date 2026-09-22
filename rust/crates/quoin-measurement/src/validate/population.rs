// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The per-observation population checks behind a plan's
//! `statistical_design.minimum_population` and `statistical_design.repetitions`
//! (PLAT-960).
//!
//! These read the observation's **stored** `population` member rather than
//! the parsed [`crate::types::observation::MeasurementPopulation`]: the parse
//! is deliberately lenient so historical evidence keeps reading, and a
//! lenient parse cannot tell a malformed `"50"` from an absent member. Intake
//! must, because treating the one as the other is how a bad record would
//! slip past the plan (review finding on quoin#585).

use std::num::NonZeroU32;

use quoin_store::JsonValue;

use crate::common::scalar::js_f64_string;
use crate::error::MeasurementErrorCode as Code;
use crate::json_bridge::to_serde;
use crate::report::render::metric_label;
use crate::types::observation::{MeasurementObservation, MeasurementState};
use crate::types::plan::MeasurementPlan;

/// One typed population finding.
type Finding = (Code, String);

/// A population member as the stored document states it.
enum Stated {
    /// The member is not there.
    Absent,
    /// The member is there with a value the rule refuses; already reported.
    Malformed,
    /// The member is there and is an acceptable whole number.
    Count(f64),
}

/// Every population finding for one observation.
///
/// - A stated `population.repetitions` must be a whole number of at least 1,
///   whatever the plan says, and is [`Code::PopulationMalformed`] otherwise.
/// - Under a plan declaring `minimum_population`, a stated
///   `population.examined` must be a whole number of at least 0, and a stated
///   `population` must be an object; either is [`Code::PopulationMalformed`]
///   otherwise. A negative value is malformed, never short.
/// - For a `measured` observation only: an `examined` below the minimum is
///   [`Code::PopulationBelowMinimum`] and an absent one
///   [`Code::PopulationUnstated`]; a `repetitions` below the plan's is
///   [`Code::RepetitionsShort`], and an absent one is
///   [`Code::PopulationUnstated`] when the plan requires more than one.
///   Reaching either declared value exactly is enough.
pub(super) fn findings(
    observation: &MeasurementObservation,
    raw_population: Option<&JsonValue>,
    plan: Option<&MeasurementPlan>,
) -> Vec<Finding> {
    let design = plan.and_then(|plan| plan.statistical_design);
    let minimum = design.and_then(|design| design.minimum_population);
    let required = design.and_then(|design| design.repetitions);
    let requires_repetitions = required.is_some_and(|required| required.get() > 1);
    let label = metric_label(
        observation.metric.as_str(),
        observation.dimensions.entries(),
    )
    .unwrap_or_else(|_| observation.metric.as_str().to_owned());
    let mut out = Vec::new();

    let population = match raw_population {
        None => None,
        Some(JsonValue::Object(population)) => Some(population),
        Some(other) => {
            if minimum.is_some() || requires_repetitions {
                out.push(malformed(&label, "population", other, "an object"));
            }
            return out;
        }
    };
    let member = |name: &str| population.and_then(|population| population.get(name));

    let repetitions = stated_count(member("repetitions"), 1.0, || {
        out.push(malformed(
            &label,
            "population.repetitions",
            member("repetitions").unwrap_or(&JsonValue::Null),
            "a whole number of at least 1",
        ));
    });
    let examined = if minimum.is_some() {
        stated_count(member("examined"), 0.0, || {
            out.push(malformed(
                &label,
                "population.examined",
                member("examined").unwrap_or(&JsonValue::Null),
                "a whole number of at least 0",
            ));
        })
    } else {
        Stated::Absent
    };

    if observation.state != MeasurementState::Measured {
        return out;
    }
    let Some(plan) = plan else {
        return out;
    };
    if let Some(minimum) = minimum {
        out.extend(threshold(
            &examined,
            minimum,
            true,
            || {
                format!(
                    "metric `{label}` states no population.examined; plan {} requires a \
                     population of at least {minimum}",
                    plan.id
                )
            },
            |examined| {
                (
                    Code::PopulationBelowMinimum,
                    format!(
                        "metric `{label}` examined {examined} but plan {} requires a population \
                         of at least {minimum}",
                        plan.id
                    ),
                )
            },
        ));
    }
    if let Some(required) = required {
        out.extend(threshold(
            &repetitions,
            required,
            requires_repetitions,
            || {
                format!(
                    "metric `{label}` states no population.repetitions; plan {} requires \
                     {required} repetitions",
                    plan.id
                )
            },
            |performed| {
                (
                    Code::RepetitionsShort,
                    format!(
                        "metric `{label}` ran {performed} repetitions but plan {} requires \
                         {required}",
                        plan.id
                    ),
                )
            },
        ));
    }
    out
}

/// Read one stated count, calling `report` when it is stated but is not a
/// whole number of at least `floor`.
fn stated_count(value: Option<&JsonValue>, floor: f64, report: impl FnOnce()) -> Stated {
    let Some(value) = value else {
        return Stated::Absent;
    };
    match value.as_f64() {
        Some(count) if count.fract() == 0.0 && count >= floor => Stated::Count(count),
        _ => {
            report();
            Stated::Malformed
        }
    }
}

/// The finding refusing `member` of `label` for holding `value`.
fn malformed(label: &str, member: &str, value: &JsonValue, expected: &str) -> Finding {
    // A stored value always crosses the bridge (every JSON number here is
    // finite); the type name is the fallback, not a refusal.
    let spelled = to_serde(value).map_or_else(|_| value.type_name().to_owned(), |v| v.to_string());
    (
        Code::PopulationMalformed,
        format!("metric `{label}` states {member} {spelled}; it must be {expected}"),
    )
}

/// One count against its declared floor. An absent count is
/// [`Code::PopulationUnstated`] when `absent_refused`; a malformed one was
/// already reported; a count below `floor` is whatever `short` builds.
fn threshold(
    stated: &Stated,
    floor: NonZeroU32,
    absent_refused: bool,
    unstated: impl FnOnce() -> String,
    short: impl FnOnce(String) -> Finding,
) -> Option<Finding> {
    match *stated {
        Stated::Absent => absent_refused.then(|| (Code::PopulationUnstated, unstated())),
        Stated::Count(value) if value < f64::from(floor.get()) => Some(short(js_f64_string(value))),
        // Malformed was already reported; a count at or above the floor passes.
        Stated::Malformed | Stated::Count(_) => None,
    }
}
