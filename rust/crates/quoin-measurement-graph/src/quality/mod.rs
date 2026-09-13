// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! `graph_quality_observation` schema version 1, and its transcription.
//!
//! Ports `graphQualityObservationSchema`, `graphQualityObservationId`,
//! `adaptGraphQualityObservation` and `normalizeGraphQuality`
//! (`src/measurement/graph-adapters.ts:269-680`).

pub mod adapt;
pub mod identity;
pub mod normalize;
pub mod population;
pub mod producer;
mod refine;
pub mod results;

use serde::Deserialize as _;
use serde_json::Value;

use crate::error::{GraphAdapterError, GraphAdapterErrorCode, Result};
use crate::scalars::Sha256Reference;
use crate::wire::{GraphQualityRecordType, SchemaVersion1};
use population::Population;
use producer::{MeasurementPlanReference, Producer, RawScorerOutput};
use results::Results;

/// One retained producer observation, as `quire-code-rs` emits it.
///
/// Minted only by [`parse`]: holding one is the proof that every member
/// satisfied its declaration *and* that the cross-member rules hold.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphQualityObservationV1 {
    /// Always `1`.
    pub schema_version: SchemaVersion1,
    /// Always `graph_quality_observation`.
    pub record_type: GraphQualityRecordType,
    /// The record's own identity, which [`identity`] re-derives.
    pub observation_id: Sha256Reference,
    /// What produced it.
    pub producer: Producer,
    /// Which plan it is against.
    pub measurement_plan: MeasurementPlanReference,
    /// What was walked.
    pub population: Population,
    /// The scores, present exactly when the population was measured.
    #[serde(default, deserialize_with = "crate::wire::absent_or")]
    pub results: Option<Results>,
    /// Where the scorer's complete output was retained.
    pub raw_scorer_output: RawScorerOutput,
}

/// Read a producer observation.
///
/// # Errors
///
/// [`GraphAdapterErrorCode::InvalidObservation`] when any member fails its
/// declaration, or any cross-member rule in [`refine`] fails. Every refusal is
/// reported, joined with `; `, as the retained `parseOrThrow` reports them.
pub fn parse(value: &Value) -> Result<GraphQualityObservationV1> {
    let refuse =
        |detail: String| GraphAdapterError::new(GraphAdapterErrorCode::InvalidObservation, detail);
    let record =
        GraphQualityObservationV1::deserialize(value).map_err(|error| refuse(error.to_string()))?;
    let violations = refine::violations(&record);
    if violations.is_empty() {
        Ok(record)
    } else {
        Err(refuse(violations.join("; ")))
    }
}
