// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Comparability only. This layer deliberately emits no quality verdict.
//!
//! Ports `src/measurement/compare.ts`. Whether a delta *matters* is a report
//! decision; whether it *means anything* is this one, and the retained module's
//! first line says so.
//!
//! # The sort key is the oracle's own string
//!
//! `compare.ts:95-101` keys an observation on
//! `` `${metric}\0${JSON.stringify(sortedDimensionEntries)}` `` and sorts the
//! union of both sides' keys as strings. The emitted order is therefore the
//! order of those rendered keys, not of the metric names, and reproducing it
//! needs the same rendering. That rendering comes from
//! [`quoin_store::canonical_bytes`] — the store's JCS writer, which agrees with
//! `JSON.stringify` on this shape — rather than from a second serializer
//! written here (FR-100-CON-4).

use quoin_store::{JsonValue, StoreError, canonical_bytes};

use crate::types::collection::MeasurementCollection;
use crate::types::comparison::{
    ComparisonReason, ComparisonReasonCode, ComparisonStatus, MeasurementComparison,
};
use crate::types::observation::{MeasurementObservation, MeasurementPopulation, MeasurementShape};

/// Compare two collections, slice by slice.
///
/// # Errors
///
/// [`StoreError`] when a dimension value cannot be rendered into the sort key,
/// which for a value read out of a store is only the nesting-depth budget.
pub fn compare_measurement_collections(
    before: &MeasurementCollection,
    after: &MeasurementCollection,
) -> Result<Vec<MeasurementComparison>, StoreError> {
    let mut keys: Vec<String> = Vec::new();
    for observation in before.observations.iter().chain(&after.observations) {
        let key = key_of(observation)?;
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys.sort_unstable();

    let mut out = Vec::with_capacity(keys.len());
    for key in keys {
        let left = find(&before.observations, &key)?;
        let right = find(&after.observations, &key)?;
        // The key came from one of the two lists, so one side is always
        // present. `else` cannot run; it is written as a skip rather than a
        // fabricated row so no unreachable value has to be invented.
        let Some(sample) = left.or(right) else {
            continue;
        };
        out.push(compare_one(before, after, sample, left, right));
    }
    Ok(out)
}

/// One slice, compared.
fn compare_one(
    before: &MeasurementCollection,
    after: &MeasurementCollection,
    sample: &MeasurementObservation,
    left: Option<&MeasurementObservation>,
    right: Option<&MeasurementObservation>,
) -> MeasurementComparison {
    // `compare.ts:19` takes the left sample when there is one, the right
    // otherwise; the caller has already resolved that.
    let not_computed = |sample: &MeasurementObservation| MeasurementComparison {
        metric: sample.metric.as_str().to_owned(),
        dimensions: sample.dimensions.clone(),
        before: left.and_then(|observation| observation.value),
        after: right.and_then(|observation| observation.value),
        delta: None,
        status: ComparisonStatus::NotComputed,
        reasons: Vec::new(),
    };
    let (Some(a), Some(b)) = (left, right) else {
        return not_computed(sample);
    };
    if a.value.is_none() || b.value.is_none() {
        return not_computed(a);
    }

    let metric = a.metric.as_str();
    let mut reasons = Vec::new();
    if a.definition_version != b.definition_version {
        reasons.push(ComparisonReason::new(
            ComparisonReasonCode::DefinitionChanged,
            format!(
                "{metric}: definition moved {} -> {}",
                a.definition_version, b.definition_version
            ),
        ));
    }
    if before.config_digest != after.config_digest {
        reasons.push(ComparisonReason::new(
            ComparisonReasonCode::ConfigurationChanged,
            format!(
                "{metric}: producer configuration moved {} -> {}",
                before.config_digest, after.config_digest
            ),
        ));
    }
    if incomplete(a) || incomplete(b) {
        reasons.push(ComparisonReason::new(
            ComparisonReasonCode::IncompletePopulation,
            format!(
                "{metric}: ratio matched none of a non-empty examined population, or collection \
                 marked it incomplete"
            ),
        ));
    }
    if a.population != b.population {
        reasons.push(ComparisonReason::new(
            ComparisonReasonCode::PopulationChanged,
            format!("{metric}: population changed; delta is shown with this warning"),
        ));
    }
    if before.tool_version != after.tool_version {
        reasons.push(ComparisonReason::new(
            ComparisonReasonCode::ToolChanged,
            format!(
                "{metric}: tool moved {} -> {}",
                before.tool_version, after.tool_version
            ),
        ));
    }

    let blocked = reasons.iter().any(|reason| reason.blocking);
    MeasurementComparison {
        metric: metric.to_owned(),
        dimensions: a.dimensions.clone(),
        before: a.value,
        after: b.value,
        delta: if blocked {
            None
        } else {
            b.value.zip(a.value).map(|(after, before)| after - before)
        },
        status: if blocked {
            ComparisonStatus::Incomparable
        } else {
            ComparisonStatus::Comparable
        },
        reasons,
    }
}

/// Whether an observation's population disqualifies it from comparison.
///
/// `compare.ts:78-88`: a declared `complete` decides on its own; otherwise a
/// ratio that matched none of a non-empty examined population is incomplete.
fn incomplete(observation: &MeasurementObservation) -> bool {
    let population = observation
        .population
        .as_ref()
        .unwrap_or(const { &MeasurementPopulation::EMPTY });
    if let Some(complete) = population.complete {
        return !complete;
    }
    observation.shape == MeasurementShape::Ratio
        && is_above_zero(population.examined.unwrap_or(0.0))
        && population.matched.is_some_and(is_zero)
}

/// `x > 0` on a JSON number, without an `as` cast or an epsilon.
fn is_above_zero(value: f64) -> bool {
    value.partial_cmp(&0.0) == Some(std::cmp::Ordering::Greater)
}

/// `x === 0` on a JSON number. IEEE equality, as the oracle's `===` is: `-0`
/// and `0` are both zero here and both there.
fn is_zero(value: f64) -> bool {
    value.partial_cmp(&0.0) == Some(std::cmp::Ordering::Equal)
}

/// The observation whose key is `key`, if the list holds one.
fn find<'a>(
    observations: &'a [MeasurementObservation],
    key: &str,
) -> Result<Option<&'a MeasurementObservation>, StoreError> {
    for observation in observations {
        if key_of(observation)? == key {
            return Ok(Some(observation));
        }
    }
    Ok(None)
}

/// `compare.ts:95-101`'s key: the metric, a NUL, and the rendered sorted
/// dimension entries.
fn key_of(observation: &MeasurementObservation) -> Result<String, StoreError> {
    let entries = JsonValue::Array(
        observation
            .dimensions
            .iter()
            .map(|(name, value)| {
                JsonValue::Array(vec![JsonValue::string(name.clone()), value.clone()])
            })
            .collect(),
    );
    let rendered = canonical_bytes(&entries)?;
    Ok(format!(
        "{}\u{0}{}",
        observation.metric,
        String::from_utf8_lossy(&rendered)
    ))
}
