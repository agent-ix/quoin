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
//!
//! # Protected apparatus (PLAT-975)
//!
//! A slice whose plan's recorded protected apparatus differs between the two
//! collections is `apparatus_changed` and blocks the delta, as a moved
//! definition does. Movement in an artifact the plan does not protect is
//! `artifact_changed`, reported beside the delta and not blocking. Both read
//! only what each collection recorded when it was written.

use std::collections::BTreeSet;

use quoin_store::{JsonValue, StoreError, canonical_bytes};

use crate::types::collection::{MeasurementCollection, ResolvedApparatus};
use crate::types::comparison::{
    ComparisonReason, ComparisonReasonCode, ComparisonStatus, MeasurementComparison,
};
use crate::types::observation::{
    MeasurementObservation, MeasurementPopulation, MeasurementShape,
    constant_predictor_governed_metric,
};

/// Compare two collections, slice by slice.
///
/// Constant-predictor item observations (`{metric}.constant-predictor-item`,
/// PLAT-1016) are left out: each is one graded item's labels, retained for
/// `quoin measurement verify`'s baseline, not a slice of any measurement, and
/// comparing them item by item would bury the real slices under one row per
/// corpus item.
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
        if constant_predictor_governed_metric(observation.metric.as_str()).is_some() {
            continue;
        }
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
        dimensions: sample.dimensions.entries().clone(),
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
    let protected = (
        before.protected_apparatus_of(a.plan_id.as_str()),
        after.protected_apparatus_of(b.plan_id.as_str()),
    );
    if let Some(change) = apparatus_change(before, after, protected) {
        reasons.push(ComparisonReason::new(
            ComparisonReasonCode::ApparatusChanged,
            format!("{metric}: protected apparatus changed: {change}"),
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

    if let Some(moved) = unprotected_movement(before, after, protected) {
        reasons.push(ComparisonReason::new(
            ComparisonReasonCode::ArtifactChanged,
            format!("{metric}: unprotected artifacts moved: {moved}"),
        ));
    }

    let blocked = reasons.iter().any(|reason| reason.blocking);
    MeasurementComparison {
        metric: metric.to_owned(),
        dimensions: a.dimensions.entries().clone(),
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

/// How the plan's recorded protected apparatus differs between the two
/// collections, rendered, or `None` when it does not (PLAT-975).
///
/// The comparison is over what each collection **recorded** when it was
/// written, never the disk as it reads now: a file edited, a file added
/// under or removed from a directory entry, a set recorded on only one side,
/// and a protected path either side lists in `unverifiedArtifacts` are each
/// a change. Two collections that both recorded nothing for the plan are not
/// compared on apparatus at all — this layer reads no plan, so it cannot
/// tell that from a plan that protects nothing; the report's ratchet and the
/// checker, which hold the plan, refuse that case themselves.
fn apparatus_change(
    before: &MeasurementCollection,
    after: &MeasurementCollection,
    protected: (Option<&ResolvedApparatus>, Option<&ResolvedApparatus>),
) -> Option<String> {
    let (left, right) = match protected {
        (None, None) => return None,
        (Some(_), None) => return Some("recorded only by the earlier collection".to_owned()),
        (None, Some(_)) => return Some("recorded only by the later collection".to_owned()),
        (Some(left), Some(right)) => (left, right),
    };
    let mut changes = Vec::new();
    let edited: Vec<&str> = left
        .iter()
        .filter(|(path, digest)| right.get(*path).is_some_and(|other| other != *digest))
        .map(|(path, _)| path.as_str())
        .collect();
    let added: Vec<&str> = right
        .keys()
        .filter(|path| !left.contains_key(*path))
        .map(String::as_str)
        .collect();
    let removed: Vec<&str> = left
        .keys()
        .filter(|path| !right.contains_key(*path))
        .map(String::as_str)
        .collect();
    let unverified: Vec<&str> = [before, after]
        .iter()
        .filter_map(|collection| collection.verification_stack.as_ref())
        .flat_map(|stack| &stack.unverified_artifacts)
        .filter(|name| left.contains_key(*name) || right.contains_key(*name))
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    for (label, paths) in [
        ("edited", edited),
        ("added", added),
        ("removed", removed),
        ("listed as unverified", unverified),
    ] {
        if !paths.is_empty() {
            changes.push(format!("{label} {}", paths.join(", ")));
        }
    }
    (!changes.is_empty()).then(|| changes.join("; "))
}

/// The `verificationStack.artifacts` names the plan does not protect whose
/// digest moved, or that only one side states, rendered, or `None` (PLAT-975).
///
/// Only asked when at least one side recorded protected apparatus for the
/// plan: "unprotected" means something only against a declared protected
/// set, and a comparison under a plan that protects nothing reads exactly as
/// it did before PLAT-975.
fn unprotected_movement(
    before: &MeasurementCollection,
    after: &MeasurementCollection,
    protected: (Option<&ResolvedApparatus>, Option<&ResolvedApparatus>),
) -> Option<String> {
    let (left, right) = protected;
    if left.is_none() && right.is_none() {
        return None;
    }
    let is_protected = |name: &str| {
        left.is_some_and(|set| set.contains_key(name))
            || right.is_some_and(|set| set.contains_key(name))
    };
    let earlier = before
        .verification_stack
        .as_ref()
        .map(|stack| &stack.artifacts);
    let later = after
        .verification_stack
        .as_ref()
        .map(|stack| &stack.artifacts);
    let names: BTreeSet<&String> = earlier
        .into_iter()
        .chain(later)
        .flat_map(|map| map.keys())
        .collect();
    let moved: Vec<&str> = names
        .into_iter()
        .filter(|name| !is_protected(name))
        .filter(|name| {
            earlier.and_then(|map| map.get(*name)) != later.and_then(|map| map.get(*name))
        })
        .map(String::as_str)
        .collect();
    (!moved.is_empty()).then(|| moved.join(", "))
}

/// Whether an observation's population disqualifies it from comparison.
///
/// `compare.ts:78-88`: a declared `complete` decides on its own; otherwise a
/// ratio that matched none of a non-empty examined population is incomplete.
pub(crate) fn incomplete(observation: &MeasurementObservation) -> bool {
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
            .entries()
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
