// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading a stored measurement collection, and admitting a new one.
//!
//! Ports `src/measurement/validate.ts`. Two entry points, in the order the
//! retained module states them:
//!
//! - [`stored_measurement_collection`] reads a **historical** envelope without
//!   rewriting it to the current plan. It is what every read path uses, and it
//!   is what must keep accepting the 48 collections retained under
//!   `spec/evidence/measurements/`.
//! - [`measurement_collection`] is the intake check for a **new** collection:
//!   the stored contract, plus the current schema version, plus a release
//!   build, plus an active version-matching `MeasurementPlan` per observation.
//!
//! # `asserts value is` becomes a returned value
//!
//! Both retained functions are TypeScript assertion functions: they narrow the
//! caller's `unknown` and return nothing, so the caller keeps handling the same
//! loosely-typed object it passed in. Here they return the parsed
//! [`MeasurementCollection`], which is the `parse, don't validate` rule the
//! Stage 6 plan §13.3 names and `RawFileSha256Digest::parse_stored` models.
//!
//! The write path still writes the **caller's** bytes, not a re-serialization
//! of the parsed value (`store.ts:40` canonicalizes `candidate`). Nothing here
//! is a lossy round trip, because nothing here round-trips.

pub(crate) mod read;
mod stack;

use std::collections::BTreeMap;

use quoin_store::{JsonObject, JsonValue};

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::types::collection::{
    BuildProfile, HISTORICAL_MEASUREMENT_SCHEMA_VERSIONS, MEASUREMENT_SCHEMA_VERSION,
    MeasurementCollection,
};
use crate::types::observation::{
    Dimensions, MeasurementObservation, MeasurementPopulation, MeasurementShape, MeasurementState,
};
use crate::types::plan::{LifecycleStatus, MeasurementPlan};

/// The code every refusal in this module carries.
const CODE: MeasurementErrorCode = MeasurementErrorCode::CollectionInvalid;

fn refuse(message: impl Into<String>) -> MeasurementError {
    MeasurementError::new(CODE, message.into())
}

/// Validate a historical stored envelope without rewriting it to the current
/// plan.
///
/// # Errors
///
/// [`MeasurementErrorCode::CollectionInvalid`] for every shape
/// `validateStoredMeasurementCollection` rejects.
pub fn stored_measurement_collection(
    value: &JsonValue,
) -> Result<MeasurementCollection, MeasurementError> {
    let object = read::object(value, CODE, "collection must be an object")?;

    let schema_version = schema_version(object)?;
    // The seven members `validate.ts:68-84` requires as non-empty strings,
    // read in the order that loop reads them so the first refusal names the
    // same member the oracle's would.
    let collection_id = read::non_empty(object, "collectionId", CODE, "collection")?;
    let subject = read::non_empty(object, "subject", CODE, "collection")?;
    let tool_identity = read::non_empty(object, "toolIdentity", CODE, "collection")?;
    let tool_version = read::non_empty(object, "toolVersion", CODE, "collection")?;
    let config_digest = read::non_empty(object, "configDigest", CODE, "collection")?;
    let timestamp = read::non_empty(object, "timestamp", CODE, "collection")?;
    let source_revision = read::non_empty(object, "sourceRevision", CODE, "collection")?;

    let Some(JsonValue::Array(raw_observations)) = object.get("observations") else {
        return Err(refuse("collection requires at least one observation"));
    };
    if raw_observations.is_empty() {
        return Err(refuse("collection requires at least one observation"));
    }
    let environment = read::object(
        object
            .get("environment")
            .ok_or_else(|| refuse("environment must be an object"))?,
        CODE,
        "environment must be an object",
    )?
    .clone();
    if !read::present(object, "scope") {
        return Err(refuse("collection requires `scope`"));
    }
    if !read::present(object, "rawEvidence") {
        return Err(refuse("collection requires attached `rawEvidence`"));
    }
    let verification_stack = if schema_version == MEASUREMENT_SCHEMA_VERSION {
        Some(stack::verification_stack(object.get("verificationStack"))?)
    } else {
        None
    };

    let mut observations: Vec<MeasurementObservation> = Vec::with_capacity(raw_observations.len());
    for raw in raw_observations {
        let observation = self::observation(raw)?;
        // `validate.ts:185-191` keys on the metric and the sorted dimension
        // entries. `dimensions` is a `BTreeMap` and `JsonValue` is `PartialEq`,
        // so the key comparison is structural equality and needs no rendering
        // step. A collection carries a handful of observations, so the scan is
        // cheaper than the map a hashable key would need.
        if observations
            .iter()
            .any(|seen| seen.identity() == observation.identity())
        {
            return Err(refuse(format!(
                "duplicate observation for `{}` and its dimensions",
                observation.metric
            )));
        }
        observations.push(observation);
    }

    Ok(MeasurementCollection {
        schema_version,
        collection_id,
        subject,
        scope: object.get("scope").cloned().unwrap_or(JsonValue::Null),
        tool_identity,
        tool_version,
        config_digest,
        timestamp,
        source_revision,
        corpus_revision: read::string(object, "corpusRevision").map(str::to_owned),
        environment,
        verification_stack,
        observations,
        raw_evidence: object
            .get("rawEvidence")
            .cloned()
            .unwrap_or(JsonValue::Null),
    })
}

/// Validate the typed envelope and require an active, version-matching plan.
///
/// # Errors
///
/// Everything [`stored_measurement_collection`] refuses, plus a schema version
/// other than [`MEASUREMENT_SCHEMA_VERSION`], a build profile other than
/// release, absent toolchains, and any observation whose metric has no active
/// plan at the observation's own definition version.
pub fn measurement_collection(
    value: &JsonValue,
    plans: &[MeasurementPlan],
) -> Result<MeasurementCollection, MeasurementError> {
    let collection = stored_measurement_collection(value)?;
    if collection.schema_version != MEASUREMENT_SCHEMA_VERSION {
        return Err(refuse(format!(
            "new collections must use schemaVersion {MEASUREMENT_SCHEMA_VERSION}; v1 is read-only \
             historical evidence"
        )));
    }
    let stack = collection.verification_stack.as_ref().ok_or_else(|| {
        refuse("verificationStack.buildProfile must be release for new collections")
    })?;
    if stack.build_profile != Some(BuildProfile::Release) {
        return Err(refuse(
            "verificationStack.buildProfile must be release for new collections",
        ));
    }
    if stack.toolchains.is_none() {
        return Err(refuse(
            "verificationStack.toolchains must pin node, rust, and python",
        ));
    }

    let by_metric: BTreeMap<&str, &MeasurementPlan> = plans
        .iter()
        .map(|plan| (plan.metric.as_str(), plan))
        .collect();
    for observation in &collection.observations {
        let metric = observation.metric.as_str();
        let Some(plan) = by_metric.get(metric) else {
            return Err(refuse(format!(
                "metric `{metric}` has no MeasurementPlan under spec/assurance or assurance; \
                 record refused"
            )));
        };
        if plan.status != LifecycleStatus::Active {
            return Err(refuse(format!(
                "metric `{metric}` plan {} is {}, not active",
                plan.id,
                plan.status.as_str()
            )));
        }
        if observation.plan_id != plan.id {
            return Err(refuse(format!(
                "metric `{metric}` names plan {}; active plan is {}",
                observation.plan_id, plan.id
            )));
        }
        if observation.definition_version != plan.definition_version {
            return Err(refuse(format!(
                "metric `{metric}` definition {} does not match {}",
                observation.definition_version, plan.definition_version
            )));
        }
    }
    Ok(collection)
}

/// The schema version, refusing anything but the current one and the retained
/// historical ones.
fn schema_version(object: &JsonObject) -> Result<u32, MeasurementError> {
    let refusal = || {
        refuse(format!(
            "schemaVersion must be 1 or {MEASUREMENT_SCHEMA_VERSION}"
        ))
    };
    let found = read::number(object, "schemaVersion").ok_or_else(refusal)?;
    // The oracle compares with `!==` against the literals 1 and 2, so a
    // fractional or out-of-range number is refused rather than rounded.
    let accepted = HISTORICAL_MEASUREMENT_SCHEMA_VERSIONS
        .into_iter()
        .chain(std::iter::once(MEASUREMENT_SCHEMA_VERSION))
        .find(|known| f64::from(*known).partial_cmp(&found) == Some(std::cmp::Ordering::Equal));
    accepted.ok_or_else(refusal)
}

/// Read one observation, or refuse.
fn observation(value: &JsonValue) -> Result<MeasurementObservation, MeasurementError> {
    let object = read::object(value, CODE, "observation must be an object")?;
    let metric = read::non_empty(object, "metric", CODE, "observation")?;
    let plan_id = read::non_empty(object, "planId", CODE, "observation")?;
    let definition_version = read::non_empty(object, "definitionVersion", CODE, "observation")?;
    let unit = read::non_empty(object, "unit", CODE, "observation")?;
    let shape = read::non_empty(object, "shape", CODE, "observation")?;
    let state = read::non_empty(object, "state", CODE, "observation")?;

    let shape = MeasurementShape::from_wire(shape.as_str())
        .ok_or_else(|| refuse(format!("observation `{metric}` has invalid shape")))?;
    let state = MeasurementState::from_wire(state.as_str())
        .ok_or_else(|| refuse(format!("observation `{metric}` has invalid state")))?;

    let raw_value = object.get("value");
    let value = match state {
        MeasurementState::Measured => {
            Some(raw_value.and_then(JsonValue::as_f64).ok_or_else(|| {
                refuse(format!(
                    "measured observation `{metric}` requires a numeric value"
                ))
            })?)
        }
        MeasurementState::NotComputed => {
            if raw_value != Some(&JsonValue::Null) {
                return Err(refuse(format!(
                    "not_computed observation `{metric}` requires null value"
                )));
            }
            None
        }
    };

    Ok(MeasurementObservation {
        metric,
        plan_id,
        definition_version,
        state,
        value,
        unit,
        shape,
        population: population(object),
        dimensions: dimensions(object),
        reason: read::string(object, "reason").map(str::to_owned),
    })
}

/// The `population` member, when it is an object.
///
/// `validate.ts` does not check `population` at all, so neither does this: an
/// absent or non-object member reads as absent rather than as a refusal.
fn population(object: &JsonObject) -> Option<MeasurementPopulation> {
    let JsonValue::Object(population) = object.get("population")? else {
        return None;
    };
    Some(MeasurementPopulation {
        examined: read::number(population, "examined"),
        matched: read::number(population, "matched"),
        complete: read::boolean(population, "complete"),
        identity: population.get("identity").cloned(),
        // Kept rather than dropped: see `MeasurementPopulation`'s header for
        // the two retained call sites that see every stored member.
        unmodelled: population
            .iter()
            .filter(|(name, _)| !MeasurementPopulation::MODELLED.contains(&name.as_str()))
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
    })
}

/// The `dimensions` member, when it is an object.
fn dimensions(object: &JsonObject) -> Dimensions {
    match object.get("dimensions") {
        Some(JsonValue::Object(dimensions)) => Dimensions::stated(
            dimensions
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect(),
        ),
        // A `dimensions` member that is not an object is not one the retained
        // reader would see either: `report.ts:120` spreads it with
        // `Object.entries(… ?? {})`, which a non-object would make throw
        // before this crate is reached.
        _ => Dimensions::ABSENT,
    }
}
