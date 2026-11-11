// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The graded-corpus run's record: the aggregate agreement observation and
//! the per-item rows behind it, as one `quoin measurement record` input
//! (PLAT-1021).
//!
//! [`record_input`] is the producer a graded run ends in. It takes the run's
//! identity and every graded [`Item`], and derives both halves from the same
//! slice: the governed aggregate (`examined` is the item count, `matched` the
//! items the tool agreed on) and the [`item_observations`] rows. The item
//! rows therefore cover exactly the aggregate's `population.examined`, which
//! is what `quoin measurement verify`'s constant-predictor baseline requires
//! (`constant_predictor_rows_mismatch` otherwise).
//!
//! It performs no I/O. The caller writes the returned JSON to a file and
//! hands it to `quoin measurement record --input`; intake attests the
//! protected apparatus and source state itself.

use quoin_measurement::MeasurementObservation;
use quoin_measurement::error::{MeasurementError, MeasurementErrorCode};
use quoin_measurement::json_bridge::to_serde;
use serde_json::{Value, json};

use crate::constant_predictor::{Item, item_observations};

/// Who and what a graded run is: the plan it answers and the collection it
/// becomes.
#[derive(Debug, Clone, Copy)]
pub struct GradedRun<'a> {
    /// The governing `MeasurementPlan` id.
    pub plan_id: &'a str,
    /// The plan's `definition_version`.
    pub definition_version: &'a str,
    /// The plan's governed metric; the aggregate is recorded under it.
    pub metric: &'a str,
    /// The stored collection's id.
    pub collection_id: &'a str,
    /// What was graded.
    pub subject: &'a str,
    /// The producing tool's version.
    pub tool_version: &'a str,
    /// RFC 3339 time of the run.
    pub timestamp: &'a str,
    /// The plan document, repo-relative: the apparatus the verdict is graded by.
    pub plan_path: &'a str,
}

/// Build the `quoin measurement record` input for a graded run over `items`.
///
/// # Errors
///
/// [`MeasurementErrorCode::CollectionInvalid`] when `items` is empty (a rate
/// over nothing is not a measurement), or for any reason
/// [`item_observations`] refuses `items`.
pub fn record_input(run: &GradedRun<'_>, items: &[Item]) -> Result<Value, MeasurementError> {
    if items.is_empty() {
        return Err(MeasurementError::new(
            MeasurementErrorCode::CollectionInvalid,
            "a graded run needs at least one graded item".to_owned(),
        ));
    }
    let rows = item_observations(run.plan_id, run.definition_version, run.metric, items)?;
    let matched = items
        .iter()
        .filter(|item| item.actual == item.expected || item.contested.contains(&item.actual))
        .count();
    let examined = items.len();

    let mut observations = vec![json!({
        "metric": run.metric,
        "planId": run.plan_id,
        "definitionVersion": run.definition_version,
        "state": "measured",
        "value": ratio(matched, examined),
        "unit": "fraction",
        "shape": "ratio",
        "population": { "examined": examined, "matched": matched, "complete": true },
    })];
    for row in &rows {
        observations.push(item_json(row)?);
    }

    Ok(json!({
        "schemaVersion": quoin_measurement::MEASUREMENT_SCHEMA_VERSION,
        "collectionId": run.collection_id,
        "subject": run.subject,
        "scope": { "items": examined },
        "toolIdentity": "quoin-jev",
        "toolVersion": run.tool_version,
        "timestamp": run.timestamp,
        "environment": { "runner": "quoin-jev graded run" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "release",
            "capabilities": ["jev.graded-run"],
            "artifacts": [run.plan_path],
        },
        "observations": observations,
        "rawEvidence": null,
    }))
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
        .collect::<Result<serde_json::Map<_, _>, _>>()?;
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
