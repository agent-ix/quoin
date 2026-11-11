// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The graded-corpus collection producer (FR-108-AC-15..AC-23, PLAT-1021).
//!
//! [`graded_corpus_collection`] turns one graded pass over a labelled corpus
//! into the candidate `MeasurementCollection` document that
//! `write_measurement_collection` and `quoin measurement record` accept: the
//! governed aggregate agreement observation, then the constant-predictor item
//! rows ([`item_observations`]) behind it. Both come from one pass over the
//! same items with one agreement rule, so the item count equals the
//! aggregate's `population.examined` by construction.
//!
//! The function performs no I/O and reads no clock, environment or random
//! source: the caller states identity, time and `verificationStack`, and
//! nothing is defaulted or invented here, the build profile included.

use quoin_measurement::MeasurementObservation;
use quoin_measurement::error::{MeasurementError, MeasurementErrorCode};
use quoin_measurement::json_bridge::{from_serde, to_serde};
use quoin_store::JsonValue;
use serde_json::{Map, Value, json};

use crate::constant_predictor::{Item, agrees, item_observations};

/// One graded pass over a labelled corpus, with the identity of the
/// collection it becomes.
#[derive(Debug, Clone)]
pub struct GradedCorpus {
    /// The governing `MeasurementPlan` id.
    pub plan_id: String,
    /// The plan's `definition_version`.
    pub definition_version: String,
    /// The plan's governed metric; the aggregate is recorded under it.
    pub metric: String,
    /// The stored collection's id.
    pub collection_id: String,
    /// What was graded.
    pub subject: String,
    /// The collection's `scope` member, as stated.
    pub scope: Value,
    /// The producing tool's identity.
    pub tool_identity: String,
    /// The producing tool's version.
    pub tool_version: String,
    /// RFC 3339 time of the run.
    pub timestamp: String,
    /// The collection's `environment` member, as stated.
    pub environment: Value,
    /// The `verificationStack` member exactly as the caller states it. Its
    /// `capabilities` and `artifacts` must be non-empty arrays.
    pub verification_stack: Value,
    /// How many passes the aggregate stands for.
    pub repetitions: u32,
    /// Every graded item of the pass, and no other.
    pub items: Vec<Item>,
}

/// Build the candidate collection document for `corpus`.
///
/// The first observation is the governed aggregate, followed by one item row
/// per graded item in `item_id` order, so the result does not depend on the
/// order the items arrive in.
///
/// # Errors
///
/// [`MeasurementErrorCode::CollectionInvalid`], naming the field, for an
/// empty identity or tool field, a `verificationStack` without non-empty
/// `capabilities` and `artifacts`, zero `repetitions`, no items, and every
/// refusal [`item_observations`] states. No partial collection is returned.
pub fn graded_corpus_collection(corpus: &GradedCorpus) -> Result<JsonValue, MeasurementError> {
    for (field, value) in [
        ("plan_id", &corpus.plan_id),
        ("definition_version", &corpus.definition_version),
        ("metric", &corpus.metric),
        ("collection_id", &corpus.collection_id),
        ("subject", &corpus.subject),
        ("tool_identity", &corpus.tool_identity),
        ("tool_version", &corpus.tool_version),
        ("timestamp", &corpus.timestamp),
    ] {
        if value.is_empty() {
            return Err(invalid(format!("requires non-empty `{field}`")));
        }
    }
    for member in ["capabilities", "artifacts"] {
        let present = corpus
            .verification_stack
            .get(member)
            .and_then(Value::as_array)
            .is_some_and(|entries| !entries.is_empty());
        if !present {
            return Err(invalid(format!(
                "requires non-empty `verification_stack.{member}`"
            )));
        }
    }
    if corpus.repetitions == 0 {
        return Err(invalid("requires `repetitions` of at least 1".to_owned()));
    }
    if corpus.items.is_empty() {
        return Err(invalid("requires at least one graded item".to_owned()));
    }

    let mut items = corpus.items.clone();
    items.sort_by(|left, right| left.item_id.cmp(&right.item_id));
    let rows = item_observations(
        &corpus.plan_id,
        &corpus.definition_version,
        &corpus.metric,
        &items,
    )?;
    let examined = items.len();
    let matched = items.iter().filter(|item| agrees(item)).count();

    let mut observations = vec![json!({
        "metric": corpus.metric,
        "planId": corpus.plan_id,
        "definitionVersion": corpus.definition_version,
        "state": "measured",
        "value": ratio(matched, examined),
        "unit": "fraction",
        "shape": "ratio",
        "population": {
            "examined": examined,
            "matched": matched,
            "complete": true,
            "repetitions": corpus.repetitions,
        },
    })];
    for row in &rows {
        observations.push(item_json(row)?);
    }

    from_serde(&json!({
        "schemaVersion": quoin_measurement::MEASUREMENT_SCHEMA_VERSION,
        "collectionId": corpus.collection_id,
        "subject": corpus.subject,
        "scope": corpus.scope,
        "toolIdentity": corpus.tool_identity,
        "toolVersion": corpus.tool_version,
        "timestamp": corpus.timestamp,
        "environment": corpus.environment,
        "verificationStack": corpus.verification_stack,
        "observations": observations,
        "rawEvidence": {},
    }))
}

fn invalid(message: String) -> MeasurementError {
    MeasurementError::new(MeasurementErrorCode::CollectionInvalid, message)
}

#[allow(
    clippy::cast_precision_loss,
    reason = "a corpus is far below 2^52 items"
)]
fn ratio(matched: usize, examined: usize) -> f64 {
    matched as f64 / examined as f64
}

fn item_json(row: &MeasurementObservation) -> Result<Value, MeasurementError> {
    let dimensions = row
        .dimensions
        .entries()
        .iter()
        .map(|(key, value)| to_serde(value).map(|value| (key.clone(), value)))
        .collect::<Result<Map<_, _>, _>>()?;
    Ok(json!({
        "metric": row.metric.as_str(),
        "planId": row.plan_id.as_str(),
        "definitionVersion": row.definition_version.as_str(),
        "state": "measured",
        "value": row.value,
        "unit": row.unit.as_str(),
        "shape": "scalar",
        "dimensions": dimensions,
    }))
}
