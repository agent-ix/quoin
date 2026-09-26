// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The JSON wire shape of a [`MeasurementVerdict`] (quoin FR-108-AC-6).
//!
//! This is the document engineering-assurance's promotion invariant
//! (PLAT-962) reads. Every top-level member is always present, `null` where
//! there is no value, so a consumer never has to tell an absent member from a
//! null one. The one exception is inside `decisions`: the three interval
//! members (EA-26, FR-108-AC-11) are absent, not `null`, on a slice no
//! interval decided, so a verdict for a plan without an `interval_level` is
//! byte-identical to what it was. Nothing is added at the top level, which
//! engineering-assurance's reader closes.
//! `schema` names the shape; a change to any member's meaning is a new
//! `schema` value, never an edit in place.

use std::collections::BTreeMap;

use quoin_store::JsonValue;
use serde::Serialize;
use serde_json::Value;

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::json_bridge::to_serde;
use crate::verify::{Counts, Finding, MeasurementVerdict, SliceDecision};

/// The `schema` every verdict document carries.
pub const VERDICT_SCHEMA: &str = "quoin.measurement-verdict.v1";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerdictWire<'a> {
    schema: &'static str,
    plan_id: &'a str,
    definition_version: &'a str,
    verdict: &'static str,
    reasons: Vec<&'static str>,
    claimed: Option<&'static str>,
    candidate: Option<&'a str>,
    decisions: Vec<DecisionWire>,
    findings: Vec<FindingWire<'a>>,
    regressed_runs: &'a [String],
    order_source: &'static str,
    counts: CountsWire,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DecisionWire {
    dimensions: Value,
    estimate: f64,
    estimate_basis: &'static str,
    baseline: Option<f64>,
    holds: Option<bool>,
    /// `lower` or `upper`: the interval bound that decided the slice.
    #[serde(skip_serializing_if = "Option::is_none")]
    interval_bound: Option<&'static str>,
    /// That bound's value.
    #[serde(skip_serializing_if = "Option::is_none")]
    interval_bound_value: Option<f64>,
    /// The confidence level the deciding interval states.
    #[serde(skip_serializing_if = "Option::is_none")]
    interval_level: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FindingWire<'a> {
    reason: &'static str,
    collection_id: Option<&'a str>,
    dimensions: Option<Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CountsWire {
    collections_considered: usize,
    regressed_runs: usize,
    observations_recomputed: usize,
    observations_asserted: usize,
    order_attested: usize,
    order_unattested: usize,
}

/// The verdict as the JSON document FR-108 specifies.
///
/// # Errors
///
/// [`MeasurementErrorCode::Store`] when a stored dimension value cannot be
/// carried into JSON, which the store's own reader never produces.
pub fn verdict_json(verdict: &MeasurementVerdict) -> Result<Value, MeasurementError> {
    let wire = VerdictWire {
        schema: VERDICT_SCHEMA,
        plan_id: &verdict.plan_id,
        definition_version: &verdict.definition_version,
        verdict: verdict.verdict.as_str(),
        reasons: verdict
            .reasons
            .iter()
            .map(|reason| reason.as_str())
            .collect(),
        claimed: verdict.claimed.map(super::Verdict::as_str),
        candidate: verdict.candidate.as_deref(),
        decisions: verdict
            .decisions
            .iter()
            .map(decision)
            .collect::<Result<_, _>>()?,
        findings: verdict
            .findings
            .iter()
            .map(finding)
            .collect::<Result<_, _>>()?,
        regressed_runs: &verdict.regressed_runs,
        order_source: verdict.order_source.as_str(),
        counts: counts(verdict.counts),
    };
    serde_json::to_value(&wire)
        .map_err(|error| MeasurementError::new(MeasurementErrorCode::Store, error.to_string()))
}

fn decision(decision: &SliceDecision) -> Result<DecisionWire, MeasurementError> {
    Ok(DecisionWire {
        dimensions: dimensions(&decision.dimensions)?,
        estimate: decision.estimate.value,
        estimate_basis: decision.estimate.basis.as_str(),
        baseline: decision.baseline,
        holds: decision.holds,
        interval_bound: decision.interval.map(|decided| decided.bound.as_str()),
        interval_bound_value: decision.interval.map(|decided| decided.bound_value),
        interval_level: decision.interval.map(|decided| decided.level),
    })
}

fn finding(finding: &Finding) -> Result<FindingWire<'_>, MeasurementError> {
    Ok(FindingWire {
        reason: finding.reason.as_str(),
        collection_id: finding.collection_id.as_deref(),
        dimensions: finding.dimensions.as_ref().map(dimensions).transpose()?,
    })
}

const fn counts(counts: Counts) -> CountsWire {
    CountsWire {
        collections_considered: counts.collections_considered,
        regressed_runs: counts.regressed_runs,
        observations_recomputed: counts.observations_recomputed,
        observations_asserted: counts.observations_asserted,
        order_attested: counts.order_attested,
        order_unattested: counts.order_unattested,
    }
}

fn dimensions(entries: &BTreeMap<String, JsonValue>) -> Result<Value, MeasurementError> {
    entries
        .iter()
        .map(|(key, value)| Ok((key.clone(), to_serde(value)?)))
        .collect::<Result<serde_json::Map<_, _>, MeasurementError>>()
        .map(Value::Object)
}
