// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Transcribe one producer observation into the governed observations it
//! states, and refuse two producer facts that claim one partition.
//!
//! Ports `normalizeGraphQuality`, `uniqueAndSort` and `compare`
//! (`src/measurement/graph-adapters.ts:578-712`).
//!
//! # Why the observations are built as JSON rather than as typed values
//!
//! The transcription's output is validated by
//! [`quoin_measurement::validate::measurement_collection`], which *returns* the
//! typed [`quoin_measurement::MeasurementCollection`]. Building the typed value
//! here and validating it too would be two constructions of one thing, and the
//! typed one would be the unchecked one. So this module states the evidence and
//! the measurement validator mints the type: parse, don't validate, with the
//! parser that already exists.

use quoin_store::json::order::cmp_utf16;
use serde_json::{Value, json};

use crate::canonical::pretty_text;
use crate::error::{GraphAdapterError, GraphAdapterErrorCode, Result};

use super::GraphQualityObservationV1;
use super::population::PopulationState;
use super::producer::MeasurementPlanReference;

/// The one metric every observation this crate mints reports.
const METRIC: &str = "graph_quality";

/// Collects the observations, so the emission order is the retained order and
/// the shared members are written once.
struct Transcription<'a> {
    plan_id: &'a str,
    definition_version: &'a str,
    population: Value,
    out: Vec<Value>,
}

impl Transcription<'_> {
    /// One measured count, sliced by `(measure, dimension, key)`.
    fn count(&mut self, measure: &str, dimension: &str, key: &str, value: u64) {
        let observation = json!({
            "metric": METRIC,
            "planId": self.plan_id,
            "definitionVersion": self.definition_version,
            "state": "measured",
            // Every count in the retained schema is a `z.number().int()`, so it
            // reaches the canonical writer as an integer and is widened to a
            // double there, exactly as zod already held it.
            "value": value,
            "unit": "count",
            "shape": "count",
            "population": self.population.clone(),
            "dimensions": { "measure": measure, "dimension": dimension, "key": key },
        });
        self.out.push(observation);
    }
}

/// Transcribe `record` into the observations it states.
///
/// `identity` is the record's own `population` member as it was received,
/// which every observation carries as its population identity.
///
/// # Errors
///
/// [`GraphAdapterErrorCode::DuplicatePartition`] when two producer facts map to
/// one `(measure, dimension, key)`, and
/// [`GraphAdapterErrorCode::Store`] for a value the canonical writer cannot
/// express.
pub fn normalize_graph_quality(
    record: &GraphQualityObservationV1,
    identity: &Value,
) -> Result<Vec<Value>> {
    let population = &record.population;
    let mut work = Transcription {
        plan_id: MeasurementPlanReference::PLAN_ID,
        definition_version: crate::wire::GraphQualityDefinitionVersion::SPELLING,
        population: json!({
            "examined": population.files_seen,
            "matched": population.supported_files,
            "complete": population.state == PopulationState::Measured,
            "identity": identity,
        }),
        out: Vec::new(),
    };

    for (name, value) in population.file_counts() {
        work.count("population", "overall", name, value);
    }
    for (dimension, rows) in population.census.axes() {
        for row in rows {
            work.count("census", dimension, row.key.as_str(), row.count);
        }
    }

    if population.state != PopulationState::Measured {
        let state = population.state.as_str();
        work.out.push(json!({
            "metric": METRIC,
            "planId": work.plan_id,
            "definitionVersion": work.definition_version,
            "state": "not_computed",
            "value": Value::Null,
            "unit": "state",
            "shape": "scalar",
            "population": work.population.clone(),
            "reason": state,
            "dimensions": { "measure": "quality_state", "dimension": "overall", "key": state },
        }));
        return unique_and_sort(work.out);
    }

    // `record.results` is `Some` here: the refinement in `super::refine`
    // requires it for a measured population, and a record that failed it never
    // became a `GraphQualityObservationV1`. The retained `?? []` is the same
    // fact spelled as a fallback.
    if let Some(results) = record.results.as_ref() {
        for row in &results.confusion_matrices {
            for (component, value) in row.components() {
                work.count(
                    "confusion_matrix",
                    row.dimension.as_str(),
                    &format!("{}.{component}", row.key),
                    value,
                );
            }
        }
        for row in &results.unresolved {
            work.count(
                "unresolved",
                row.dimension.as_str(),
                row.key.as_str(),
                row.count,
            );
        }
        for row in &results.ambiguous {
            work.count(
                "ambiguous",
                row.dimension.as_str(),
                row.key.as_str(),
                row.count,
            );
        }
        for row in &results.recall {
            let observation = json!({
                "metric": METRIC,
                "planId": work.plan_id,
                "definitionVersion": work.definition_version,
                "state": "measured",
                "value": row.ratio.get(),
                "unit": "ratio",
                "shape": "ratio",
                "population": {
                    "examined": row.expected.get(),
                    "matched": row.recovered,
                    "complete": true,
                    "identity": identity,
                },
                "dimensions": {
                    "measure": "recall",
                    "dimension": row.dimension.as_str(),
                    "key": row.key.as_str(),
                },
            });
            work.out.push(observation);
        }
    }
    unique_and_sort(work.out)
}

/// Refuse a repeated partition, then sort by the canonical text of the
/// dimensions.
///
/// The comparison is [`cmp_utf16`] rather than Rust's `str: Ord` because the
/// retained comparator is JavaScript `<`, which is UTF-16 lexicographic. The
/// two orders disagree above the BMP, and there is no reason to leave that
/// difference latent when the workspace already states the right one.
fn unique_and_sort(observations: Vec<Value>) -> Result<Vec<Value>> {
    let mut keyed: Vec<(String, Value)> = Vec::with_capacity(observations.len());
    for observation in observations {
        let dimensions = observation.get("dimensions").cloned().unwrap_or(json!({}));
        let key = pretty_text(&dimensions)?;
        if keyed.iter().any(|(seen, _)| *seen == key) {
            return Err(GraphAdapterError::new(
                GraphAdapterErrorCode::DuplicatePartition,
                format!("more than one producer fact maps to {}", key.trim_end()),
            ));
        }
        keyed.push((key, observation));
    }
    keyed.sort_by(|(left, _), (right, _)| cmp_utf16(left, right));
    Ok(keyed
        .into_iter()
        .map(|(_, observation)| observation)
        .collect())
}
