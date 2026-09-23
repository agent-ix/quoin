// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One observation, re-estimated from the rows it carries (PLAT-961).
//!
//! # What row data a collection holds
//!
//! A stored collection carries, per observation, an aggregate `value` and a
//! `population` of `examined`, `matched`, `complete`, `repetitions` and a
//! producer-defined `identity`. `rawEvidence` is the producer's own output,
//! opaque to this crate: no schema says what its members mean, so a checker
//! that read it would be reading the producer's format through the
//! producer's eyes. Measured over the 49 retained tier-1 collections
//! (15,134 observations) when this module was written: `matched` and
//! `examined` are stated on 8,183 observations, per-item values on none.
//!
//! So `matched` and `examined` are the row data: they are the sufficient
//! statistic of a `proportion` (matched / examined) and a `count` (matched).
//! Those two estimators are **recomputed** here and the stored `value` must
//! agree exactly. `mean`, `median` and `ratio` need per-item values or two
//! totals, which no collection carries; for them the stored `value` is used
//! and the observation is counted as **asserted**, not recomputed.
//!
//! # Why this does not reuse the report's usability check
//!
//! `report::verdict::usable` reads an unstated `complete` as complete, and
//! `compare::incomplete` infers incompleteness from a zero `matched`. Both
//! are the retained report's reading. A checker that grants credit reads
//! nothing into silence: `complete` must be stated `true` (PLAT-110, PLAT-112,
//! PLAT-114).

use std::cmp::Ordering;

use engineering_assurance::measurement::Estimator;
use quoin_store::JsonValue;

use crate::types::observation::{MeasurementObservation, MeasurementState};
use crate::types::plan::StatisticalDesign;
use crate::verify::Reason;

/// Where an estimate came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum EstimateBasis {
    /// Recomputed by the checker from `matched` and `examined`.
    Recomputed,
    /// The producer's stored `value`, which no row data can check.
    Asserted,
}

impl EstimateBasis {
    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Recomputed => "recomputed",
            Self::Asserted => "asserted",
        }
    }
}

/// One observation's estimate, and where it came from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Estimate {
    /// The estimate the decision rule is applied to.
    pub value: f64,
    /// Whether the checker recomputed it or took the stored value.
    pub basis: EstimateBasis,
}

/// Recompute `estimator` from `matched` and `examined`, or `None` when the
/// estimator needs data a collection does not carry.
///
/// `matched` and `examined` are whole numbers with `matched <= examined` and
/// `examined > 0` by the time [`assess`] calls this.
#[must_use]
pub fn recompute(estimator: Estimator, matched: Option<f64>, examined: f64) -> Option<f64> {
    match estimator {
        Estimator::Proportion => matched.map(|matched| matched / examined),
        Estimator::Count => matched,
        Estimator::Mean | Estimator::Median | Estimator::Ratio => None,
    }
}

/// The estimate one observation contributes, or the first reason it cannot.
///
/// The checks run in a fixed order, and the first failure is the answer:
/// no value; no `population` or no `examined`; a malformed `examined`,
/// `matched` or `repetitions`; `complete` unstated or `false`; an empty
/// population; a population below the plan's `minimum_population`; fewer
/// repetitions than the plan requires, or none stated when it requires more
/// than one; and last, a stored `value` that disagrees with the recomputed
/// estimate.
///
/// # Errors
///
/// The [`Reason`] the observation cannot contribute an estimate.
pub fn assess(
    design: &StatisticalDesign,
    estimator: Estimator,
    observation: &MeasurementObservation,
) -> Result<Estimate, Reason> {
    let value = match (observation.state, observation.value) {
        (MeasurementState::Measured, Some(value)) => value,
        (MeasurementState::Measured | MeasurementState::NotComputed, _) => {
            return Err(Reason::NoValue);
        }
    };
    let population = observation
        .population
        .as_ref()
        .ok_or(Reason::PopulationUnstated)?;
    let examined = population.examined.ok_or(Reason::PopulationUnstated)?;
    if !is_whole(examined)
        || population
            .matched
            .is_some_and(|m| !is_whole(m) || m > examined)
    {
        return Err(Reason::PopulationMalformed);
    }
    let repetitions = stated_repetitions(population.repetitions.as_ref())?;
    match population.complete {
        Some(true) => {}
        Some(false) => return Err(Reason::PopulationIncomplete),
        None => return Err(Reason::PopulationUnstated),
    }
    if examined.partial_cmp(&0.0) != Some(Ordering::Greater) {
        return Err(Reason::PopulationEmpty);
    }
    if design
        .minimum_population
        .is_some_and(|minimum| examined < f64::from(minimum.get()))
    {
        return Err(Reason::PopulationBelowMinimum);
    }
    if let Some(required) = design.repetitions.filter(|required| required.get() > 1) {
        let performed = repetitions.ok_or(Reason::PopulationUnstated)?;
        if performed < f64::from(required.get()) {
            return Err(Reason::RepetitionsShort);
        }
    }
    match recompute(estimator, population.matched, examined) {
        Some(recomputed) if recomputed.partial_cmp(&value) == Some(Ordering::Equal) => {
            Ok(Estimate {
                value: recomputed,
                basis: EstimateBasis::Recomputed,
            })
        }
        Some(_) => Err(Reason::ValueDisagreesWithRows),
        None => Ok(Estimate {
            value,
            basis: EstimateBasis::Asserted,
        }),
    }
}

/// The stated `population.repetitions`, or `None` when unstated.
fn stated_repetitions(stated: Option<&JsonValue>) -> Result<Option<f64>, Reason> {
    stated
        .map(|stated| {
            stated
                .as_f64()
                .filter(|count| is_whole(*count) && *count >= 1.0)
                .ok_or(Reason::PopulationMalformed)
        })
        .transpose()
}

/// A finite, non-negative whole number.
fn is_whole(value: f64) -> bool {
    value.is_finite() && value >= 0.0 && value.fract() == 0.0
}
