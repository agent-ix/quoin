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

mod population;
pub(crate) mod read;
mod stack;

use std::collections::{BTreeMap, BTreeSet};

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

/// [`parse_stored_measurement_collection`]'s result: the parsed envelope, plus
/// the verification stack's own outcome kept separate from the rest of the
/// parse.
///
/// Keeping the stack's `Result` apart from `collection.verification_stack`
/// (which is `None` either way when the stack fails) is what lets
/// [`measurement_collection`] see every other collection-level problem even
/// when the stack itself is the broken part (PLAT-929) — `?`-ing on the stack
/// the way [`stored_measurement_collection`] still does would stop the parse
/// before the observations below it are ever read.
struct ParsedCollection {
    collection: MeasurementCollection,
    stack_error: Option<MeasurementError>,
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
    let parsed = parse_stored_measurement_collection(value)?;
    match parsed.stack_error {
        Some(error) => Err(error),
        None => Ok(parsed.collection),
    }
}

/// The shared parse behind [`stored_measurement_collection`] and
/// [`measurement_collection`]. See [`ParsedCollection`] for why the stack's
/// outcome is not simply folded into the returned `Err`.
fn parse_stored_measurement_collection(
    value: &JsonValue,
) -> Result<ParsedCollection, MeasurementError> {
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
    // The stack's own failure is kept alongside the parse instead of `?`-ing
    // immediately, so a caller checking further collection-level rules can
    // still see the observations read below (PLAT-929, review finding #5(a)
    // on quoin#580). `configDigest`'s shape rides along in the same call
    // (PLAT-939): it is a collection-envelope member, not a `verificationStack`
    // one, but it is checked at the same schemaVersion-2 gate as `lockDigest`
    // and `executableDigest`.
    let (verification_stack, stack_error) = if schema_version == MEASUREMENT_SCHEMA_VERSION {
        match stack::verification_stack(object.get("verificationStack"), config_digest.as_str()) {
            Ok(stack) => (Some(stack), None),
            Err(error) => (None, Some(error)),
        }
    } else {
        (None, None)
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

    Ok(ParsedCollection {
        collection: MeasurementCollection {
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
        },
        stack_error,
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
///
/// Every finding is accumulated into one refusal. Its code is
/// [`MeasurementErrorCode::CollectionInvalid`], except when every finding is
/// one population refusal kind (PLAT-960), in which case the refusal carries
/// that kind's own code — [`MeasurementErrorCode::PopulationBelowMinimum`],
/// [`MeasurementErrorCode::PopulationUnstated`],
/// [`MeasurementErrorCode::RepetitionsShort`] or
/// [`MeasurementErrorCode::PopulationMalformed`]. A population finding inside a
/// mixed refusal still names its code as the finding's first word, so the
/// typed reason survives the accumulation.
pub fn measurement_collection(
    value: &JsonValue,
    plans: &[MeasurementPlan],
) -> Result<MeasurementCollection, MeasurementError> {
    let parsed = parse_stored_measurement_collection(value)?;
    let collection = parsed.collection;

    // Every check below runs regardless of an earlier one's outcome, and every
    // failure is accumulated rather than returned immediately, so a caller
    // sees every problem with the candidate in one refusal instead of fixing
    // them one round trip at a time (PLAT-929). That includes the
    // verification stack itself: its failure is folded in here rather than
    // `?`-ed away, so e.g. a bad digest and an unplanned metric are both
    // named in the same refusal (review finding #5(a) on quoin#580).
    let mut findings = Vec::new();
    // The code of each finding that has one of its own, pushed alongside it.
    // Only the population checks (PLAT-960) are typed today; see this
    // function's `# Errors` for how the refusal's own code is chosen.
    let mut typed: Vec<MeasurementErrorCode> = Vec::new();

    if collection.schema_version != MEASUREMENT_SCHEMA_VERSION {
        findings.push(format!(
            "schemaVersion: new collections must use schemaVersion {MEASUREMENT_SCHEMA_VERSION}; \
             v1 is read-only historical evidence"
        ));
    }

    match &parsed.stack_error {
        Some(error) if error.findings().is_empty() => findings.push(error.subject().to_owned()),
        Some(error) => findings.extend(error.findings().iter().cloned()),
        None => match collection.verification_stack.as_ref() {
            None => findings.push(
                "verificationStack.buildProfile must be release for new collections".to_owned(),
            ),
            Some(stack) => {
                if stack.build_profile != Some(BuildProfile::Release) {
                    findings.push(
                        "verificationStack.buildProfile must be release for new collections"
                            .to_owned(),
                    );
                }
                if stack.toolchains.is_none() {
                    findings.push(
                        "verificationStack.toolchains must be present; mark a language not \
                         used as absent or null rather than omitting the whole member"
                            .to_owned(),
                    );
                }
                // `stack::verification_stack` already refuses a `toolchains`
                // object present with every language absent, so this check
                // (an omitted `toolchains` member entirely) is no longer
                // vacuous the way it was before that refusal existed (review
                // finding #3 on quoin#580).
            }
        },
    }

    let by_metric: BTreeMap<&str, &MeasurementPlan> = plans
        .iter()
        .map(|plan| (plan.metric.as_str(), plan))
        .collect();
    // The parse admitted every observation in order, so the stored array and
    // `collection.observations` pair up by position; the population checks
    // read the stored member, because a malformed value is exactly what the
    // parsed model cannot show (PLAT-960).
    let raw_observations: &[JsonValue] = match value
        .as_object()
        .ok()
        .and_then(|object| object.get("observations"))
    {
        Some(JsonValue::Array(raw)) => raw,
        _ => &[],
    };
    for (index, observation) in collection.observations.iter().enumerate() {
        let plan = by_metric.get(observation.metric.as_str()).copied();
        findings.extend(plan_findings(observation, plan));
        let raw_population = raw_observations
            .get(index)
            .and_then(|raw| raw.as_object().ok())
            .and_then(|raw| raw.get("population"));
        for (code, finding) in population::findings(observation, raw_population, plan) {
            typed.push(code);
            findings.push(format!("{code}: {finding}"));
        }
    }

    // PLAT-936: tamper evidence, not an ordering proof. A plan that declares a
    // `preregistration` block asserts that its own "Comparison and
    // Enforcement" section reads today the way it did when the digest was
    // recorded; this is the one place that assertion is checked, and only for
    // a *new* collection — `stored_measurement_collection` never reaches this
    // function, so historical evidence is unaffected. Residual gaps this does
    // not close (reporting only a favourable pre-registered variant, a
    // repeated re-run, editing an unprotected input such as the baseline
    // instead of the bar text) are exactly what PLAT-935's design research
    // found this class of check cannot buy, and nothing here claims
    // otherwise.
    //
    // Run once per distinct metric actually referenced by an observation, not
    // once per observation: two observations against the same gate plan (e.g.
    // different dimensions) share one preregistration, and duplicating an
    // identical finding for each would misreport how many distinct problems a
    // candidate has (review finding #3 on quoin#583).
    let mut checked_metrics = BTreeSet::new();
    for observation in &collection.observations {
        let metric = observation.metric.as_str();
        if !checked_metrics.insert(metric) {
            continue;
        }
        let Some(plan) = by_metric.get(metric) else {
            continue;
        };
        if let Some(preregistration) = &plan.preregistration
            && !preregistration.matches()
        {
            findings.push(format!(
                "metric `{metric}` plan {} bar text no longer matches its pre-registered digest \
                 {}; the \"Comparison and Enforcement\" section was edited since it was recorded",
                plan.id, preregistration.declared_digest
            ));
        }
    }

    if findings.is_empty() {
        Ok(collection)
    } else {
        Err(intake_refusal(findings, &typed))
    }
}

/// The untyped findings tying one observation to its plan: none governs it,
/// or the one that does is inactive, differently named, or at another
/// definition version.
fn plan_findings(
    observation: &MeasurementObservation,
    plan: Option<&MeasurementPlan>,
) -> Vec<String> {
    let metric = observation.metric.as_str();
    let Some(plan) = plan else {
        return vec![format!(
            "metric `{metric}` has no MeasurementPlan under spec/assurance or assurance; \
             record refused"
        )];
    };
    let mut out = Vec::new();
    if plan.status != LifecycleStatus::Active {
        out.push(format!(
            "metric `{metric}` plan {} is {}, not active",
            plan.id,
            plan.status.as_str()
        ));
    }
    if observation.plan_id != plan.id {
        out.push(format!(
            "metric `{metric}` names plan {}; active plan is {}",
            observation.plan_id, plan.id
        ));
    }
    if observation.definition_version != plan.definition_version {
        out.push(format!(
            "metric `{metric}` definition {} does not match {}",
            observation.definition_version, plan.definition_version
        ));
    }
    out
}

/// One refusal carrying every accumulated finding, under the code chosen as
/// [`measurement_collection`]'s `# Errors` states: the shared code of the
/// typed findings when every finding is typed and they agree, and
/// [`MeasurementErrorCode::CollectionInvalid`] otherwise.
fn intake_refusal(findings: Vec<String>, typed: &[MeasurementErrorCode]) -> MeasurementError {
    let code = match typed.first() {
        Some(first) if typed.len() == findings.len() && typed.iter().all(|code| code == first) => {
            *first
        }
        _ => CODE,
    };
    MeasurementError::with_findings(
        code,
        "measurement collection failed intake validation",
        findings,
    )
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
        repetitions: population.get("repetitions").cloned(),
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
