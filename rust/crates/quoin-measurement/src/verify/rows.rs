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
//! `examined` are stated on 8,183 observations, and no observation carries
//! per-item values or the two totals of a ratio.
//!
//! So `matched` and `examined` are the row data: they are the sufficient
//! statistic of a `proportion` (matched / examined) and a `count` (matched).
//! Those two estimators are **recomputed** here, and a `proportion` or
//! `count` observation with no `matched` has no rows to recompute from, so it
//! is `population_unstated`, not asserted. The stored `value` must agree with
//! the recomputation to within its own stated precision ([`consistent`]). `mean`, `median` and `ratio` need per-item values or two
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
pub fn recompute(estimator: Estimator, matched: f64, examined: f64) -> Option<f64> {
    match estimator {
        Estimator::Proportion => Some(matched / examined),
        Estimator::Count => Some(matched),
        Estimator::Mean | Estimator::Median | Estimator::Ratio => None,
    }
}

/// Whether a stored `value` is consistent with the rows it summarises,
/// `numerator / denominator`: within half a unit of the value's last stated
/// decimal.
///
/// The stated decimals are read from the value's shortest round-trip
/// spelling, which is what a JSON writer emits: `0.947` states three, so it
/// is consistent with any `numerator / denominator` within 0.0005 of it.
/// The comparison is exact, on integers: with the spelling `N` over `10^d`,
/// `|N / 10^d - numerator / denominator| <= 1 / (2 * 10^d)` is
/// `2 * |N * denominator - numerator * 10^d| <= denominator`. Comparing the
/// value's binary `f64` instead would misjudge a spelling one ULP from the
/// quotient. A value that is the `f64` quotient itself is always consistent
/// — its shortest spelling identifies that `f64`, not the exact rational — and
/// a spelling too long for the integer arithmetic must be that quotient.
///
/// The rule is only about the aggregate's honesty: the decision rule is
/// applied to the recomputed estimate, never to the stored value, so a
/// rounded aggregate cannot move a verdict.
#[must_use]
pub fn consistent(stored: f64, numerator: f64, denominator: f64) -> bool {
    let spelled = stored.to_string();
    let (whole, fraction) = spelled.split_once('.').unwrap_or((spelled.as_str(), ""));
    let places = u32::try_from(fraction.len()).ok();
    let exact =
        || (numerator / denominator).partial_cmp(&stored) == Some(std::cmp::Ordering::Equal);
    let digits = format!("{whole}{fraction}");
    let (Some(places), Ok(spelled), Some(numerator), Some(denominator)) = (
        places,
        digits.parse::<i128>(),
        integer(numerator),
        integer(denominator),
    ) else {
        return exact();
    };
    if exact() {
        return true;
    }
    let Some(scale) = 10_i128.checked_pow(places) else {
        return false;
    };
    spelled
        .checked_mul(denominator)
        .zip(numerator.checked_mul(scale))
        .and_then(|(left, right)| left.checked_sub(right))
        .and_then(i128::checked_abs)
        .and_then(|gap| gap.checked_mul(2))
        .is_some_and(|gap| gap <= denominator)
}

/// A whole `f64` as an integer, or `None` when it is not one.
fn integer(value: f64) -> Option<i128> {
    is_whole(value)
        .then(|| format!("{value:.0}").parse().ok())
        .flatten()
}

/// Whether `unit` states a proportion's unit: `fraction`, or `fraction of
/// …`. Units are free text in the stored corpus (`percent of …`, `seeded
/// failure`), and a proportion recomputed as `matched / examined` is only
/// comparable with a value stated as a fraction.
#[must_use]
pub fn is_fraction_unit(unit: &str) -> bool {
    unit == "fraction" || unit.starts_with("fraction ")
}

/// The estimate one observation contributes, or the first reason it cannot.
///
/// The checks run in a fixed order, and the first failure is the answer:
/// no value; no `population`, no `examined`, or — for `proportion` and
/// `count`, whose rows it is — no `matched`; a malformed `examined`,
/// `matched` or `repetitions`; `complete` unstated or `false`; a population
/// below the plan's `minimum_population` (an empty one included, when the
/// plan states a minimum); an empty population; fewer repetitions than the
/// plan requires, or none stated when it requires more than one; a
/// `proportion` whose unit is not a fraction; and last, a stored `value`
/// inconsistent with the recomputed estimate.
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
    let rows_required = matches!(estimator, Estimator::Proportion | Estimator::Count);
    if rows_required && population.matched.is_none() {
        return Err(Reason::PopulationUnstated);
    }
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
    if design
        .minimum_population
        .is_some_and(|minimum| examined < f64::from(minimum.get()))
    {
        return Err(Reason::PopulationBelowMinimum);
    }
    if examined.partial_cmp(&0.0) != Some(Ordering::Greater) {
        return Err(Reason::PopulationEmpty);
    }
    if let Some(required) = design.repetitions.filter(|required| required.get() > 1) {
        let performed = repetitions.ok_or(Reason::PopulationUnstated)?;
        if performed < f64::from(required.get()) {
            return Err(Reason::RepetitionsShort);
        }
    }
    if estimator == Estimator::Proportion && !is_fraction_unit(observation.unit.as_str()) {
        return Err(Reason::UnitUnsupported);
    }
    let Some((matched, recomputed)) = population
        .matched
        .and_then(|matched| Some((matched, recompute(estimator, matched, examined)?)))
    else {
        return Ok(Estimate {
            value,
            basis: EstimateBasis::Asserted,
        });
    };
    let denominator = if estimator == Estimator::Proportion {
        examined
    } else {
        1.0
    };
    if consistent(value, matched, denominator) {
        Ok(Estimate {
            value: recomputed,
            basis: EstimateBasis::Recomputed,
        })
    } else {
        Err(Reason::ValueDisagreesWithRows)
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
