// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The per-observation population check behind a plan's
//! `statistical_design.minimum_population` (PLAT-960).

use crate::common::scalar::js_f64_string;
use crate::error::MeasurementErrorCode;
use crate::report::render::metric_label;
use crate::types::observation::{MeasurementObservation, MeasurementState};
use crate::types::plan::MeasurementPlan;

/// The population refusal for one observation, if it has one (PLAT-960).
///
/// Only a plan that declares `statistical_design.minimum_population` has one,
/// and only a `measured` observation is checked: a `not_computed` observation
/// carries no value for the minimum to make trustworthy. A measured
/// observation with no numeric `population.examined` cannot show it met the
/// minimum, so it is refused as [`MeasurementErrorCode::PopulationUnstated`]
/// rather than admitted on silence; one whose `examined` is below the minimum
/// is [`MeasurementErrorCode::PopulationBelowMinimum`]. Reaching the minimum
/// exactly is enough.
pub(super) fn finding(
    observation: &MeasurementObservation,
    plan: &MeasurementPlan,
) -> Option<(MeasurementErrorCode, String)> {
    let minimum = plan.statistical_design?.minimum_population?;
    if observation.state != MeasurementState::Measured {
        return None;
    }
    // A label only fails for a non-finite dimension value, which no JSON
    // document can hold; the bare metric is the fallback, not a refusal.
    let label = metric_label(
        observation.metric.as_str(),
        observation.dimensions.entries(),
    )
    .unwrap_or_else(|_| observation.metric.as_str().to_owned());
    let examined = observation
        .population
        .as_ref()
        .and_then(|population| population.examined);
    match examined {
        None => Some((
            MeasurementErrorCode::PopulationUnstated,
            format!(
                "metric `{label}` states no population.examined; plan {} requires a population \
                 of at least {minimum}",
                plan.id
            ),
        )),
        Some(examined) if examined < f64::from(minimum.get()) => Some((
            MeasurementErrorCode::PopulationBelowMinimum,
            format!(
                "metric `{label}` examined {} but plan {} requires a population of at least \
                 {minimum}",
                js_f64_string(examined),
                plan.id
            ),
        )),
        Some(_) => None,
    }
}
