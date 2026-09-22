// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The per-observation population checks behind a plan's
//! `statistical_design.minimum_population` and `statistical_design.repetitions`
//! (PLAT-960).

use std::num::NonZeroU32;

use crate::common::scalar::js_f64_string;
use crate::error::MeasurementErrorCode;
use crate::report::render::metric_label;
use crate::types::observation::{MeasurementObservation, MeasurementState};
use crate::types::plan::MeasurementPlan;

/// Every population refusal for one observation, in the order minimum then
/// repetitions (PLAT-960).
///
/// Only a `measured` observation is checked: a `not_computed` observation
/// carries no value for the plan's design to make trustworthy.
///
/// - When the plan declares `minimum_population`, an `examined` below it is
///   [`MeasurementErrorCode::PopulationBelowMinimum`], and no numeric
///   `examined` at all is [`MeasurementErrorCode::PopulationUnstated`].
/// - When the plan declares `repetitions`, a stated `population.repetitions`
///   below it is [`MeasurementErrorCode::RepetitionsShort`]. An unstated count
///   is [`MeasurementErrorCode::PopulationUnstated`] only when the plan
///   requires more than one repetition: a single run is what every collection
///   predating the member already is.
///
/// Reaching either declared value exactly is enough.
pub(super) fn findings(
    observation: &MeasurementObservation,
    plan: &MeasurementPlan,
) -> Vec<(MeasurementErrorCode, String)> {
    let mut out = Vec::new();
    let Some(design) = plan.statistical_design else {
        return out;
    };
    if observation.state != MeasurementState::Measured {
        return out;
    }
    // A label only fails for a non-finite dimension value, which no JSON
    // document can hold; the bare metric is the fallback, not a refusal.
    let label = metric_label(
        observation.metric.as_str(),
        observation.dimensions.entries(),
    )
    .unwrap_or_else(|_| observation.metric.as_str().to_owned());
    let population = observation.population.as_ref();
    let examined = population.and_then(|population| population.examined);
    let repetitions = population.and_then(|population| population.repetitions);

    if let Some(minimum) = design.minimum_population {
        out.extend(checked(
            examined,
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
                    MeasurementErrorCode::PopulationBelowMinimum,
                    format!(
                        "metric `{label}` examined {examined} but plan {} requires a population \
                         of at least {minimum}",
                        plan.id
                    ),
                )
            },
        ));
    }
    if let Some(required) = design.repetitions {
        out.extend(checked(
            repetitions,
            required,
            required.get() > 1,
            || {
                format!(
                    "metric `{label}` states no population.repetitions; plan {} requires \
                     {required} repetitions",
                    plan.id
                )
            },
            |performed| {
                (
                    MeasurementErrorCode::RepetitionsShort,
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

/// One stated-or-not count against its declared floor.
///
/// `None` stated is [`MeasurementErrorCode::PopulationUnstated`] when
/// `unstated_refused`, and admitted otherwise; a stated value below `floor`
/// is whatever `short` builds from its rendered spelling.
fn checked(
    stated: Option<f64>,
    floor: NonZeroU32,
    unstated_refused: bool,
    unstated: impl FnOnce() -> String,
    short: impl FnOnce(String) -> (MeasurementErrorCode, String),
) -> Option<(MeasurementErrorCode, String)> {
    match stated {
        None => unstated_refused.then(|| (MeasurementErrorCode::PopulationUnstated, unstated())),
        Some(value) if value < f64::from(floor.get()) => Some(short(js_f64_string(value))),
        Some(_) => None,
    }
}
