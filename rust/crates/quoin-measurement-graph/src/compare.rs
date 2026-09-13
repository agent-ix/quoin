// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Two graph-quality collections, compared partition by partition.
//!
//! Ports `compareGraphQualityCollections`, `compareHistoryPair`,
//! `compatibilityReasons`, `mismatch`, `graphObservations` and `partitionKey`
//! (`graph-portfolio.ts:298-324,519-566,643-712,772-773`).
//!
//! # A delta is earned, not computed
//!
//! Every row states a `before` and an `after` whenever it has them, but
//! `delta` is stated **only** when nothing blocks the comparison. Two numbers
//! measured under different configurations, tool versions, corpora, plans or
//! populations are not a trend, and subtracting them would manufacture one.
//! That is why [`GraphCompatibilityReason`] carries `blocking: true` and no
//! other value: there is no advisory incompatibility here.

use std::collections::BTreeMap;

use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::observation::{MeasurementObservation, MeasurementState};
use quoin_store::JsonValue;

use crate::canonical::stored_pretty_text_trimmed;
use crate::error::GraphAdapterError;
use crate::history::{graph_observations, partition_identity};
use crate::order::compare_text;
use crate::reading::{
    ComparisonStatus, GraphCompatibilityCode, GraphCompatibilityReason, GraphQualityComparison,
    GraphQualityComparisonRow, GraphQualityHistoryRow, PartitionIdentity,
};

/// Compare the graph partitions of two retained collections.
///
/// `compareGraphQualityCollections` (`graph-portfolio.ts:298-324`).
///
/// # Errors
///
/// [`crate::error::GraphAdapterErrorCode::Store`] for a population identity or corpus
/// revision no canonical JSON writer can spell.
pub fn compare_graph_quality_collections(
    before: &MeasurementCollection,
    after: &MeasurementCollection,
) -> Result<Vec<GraphQualityComparisonRow>, GraphAdapterError> {
    let mut merged: BTreeMap<String, Pair<'_>> = BTreeMap::new();
    index_into(&mut merged, before, Side::Before);
    index_into(&mut merged, after, Side::After);
    let mut ordered: Vec<(&String, &Pair<'_>)> = merged.iter().collect();
    ordered.sort_by(|(left, _), (right, _)| compare_text(left, right));

    ordered
        .into_iter()
        .map(|(_, pair)| {
            let identity = pair.identity.clone();
            match (pair.before, pair.after) {
                (Some(a), Some(b))
                    if a.state == MeasurementState::Measured
                        && b.state == MeasurementState::Measured =>
                {
                    let reasons = compatibility_reasons(before, after, a, b)?;
                    let comparable = reasons.is_empty();
                    Ok(GraphQualityComparisonRow {
                        identity,
                        before: a.value,
                        after: b.value,
                        delta: match (comparable, a.value, b.value) {
                            (true, Some(earlier), Some(later)) => Some(later - earlier),
                            _ => None,
                        },
                        status: if comparable {
                            ComparisonStatus::Comparable
                        } else {
                            ComparisonStatus::Incomparable
                        },
                        reasons,
                    })
                }
                (a, b) => Ok(GraphQualityComparisonRow {
                    identity,
                    before: a.and_then(|row| row.value),
                    after: b.and_then(|row| row.value),
                    delta: None,
                    status: ComparisonStatus::NotComputed,
                    reasons: Vec::new(),
                }),
            }
        })
        .collect()
}

/// One partition seen from both sides.
///
/// The retained code builds two `Map`s and unions their keys, then throws
/// `"graph partition index lost its sample"` if a key is in neither — which it
/// cannot be. One merged index removes the branch rather than reproducing an
/// unreachable throw.
#[derive(Clone, Debug)]
struct Pair<'a> {
    /// The three names this partition is sliced by.
    identity: PartitionIdentity,
    /// The earlier collection's observation of it.
    before: Option<&'a MeasurementObservation>,
    /// The later collection's.
    after: Option<&'a MeasurementObservation>,
}

/// Which collection an observation came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Side {
    /// The earlier one.
    Before,
    /// The later one.
    After,
}

/// `graphObservations` (`graph-portfolio.ts:707-712`) folded into the merged
/// index, keyed by `partitionKey` — the three identity names joined by `\0`
/// (`graph-portfolio.ts:772-773`), which is the retained key exactly rather
/// than a field-wise comparison that agrees with it only while no name
/// contains a NUL.
fn index_into<'a>(
    merged: &mut BTreeMap<String, Pair<'a>>,
    collection: &'a MeasurementCollection,
    side: Side,
) {
    for row in graph_observations(collection) {
        let identity = partition_identity(row);
        let key = partition_key(&identity);
        // `new Map(entries)` keeps the LAST entry for a repeated key.
        let pair = merged.entry(key).or_insert_with(|| Pair {
            identity,
            before: None,
            after: None,
        });
        match side {
            Side::Before => pair.before = Some(row),
            Side::After => pair.after = Some(row),
        }
    }
}

/// `partitionKey` (`graph-portfolio.ts:772-773`).
fn partition_key(identity: &PartitionIdentity) -> String {
    format!(
        "{}\0{}\0{}",
        identity.measure, identity.dimension, identity.key
    )
}

/// Compare the two newest readings, and mark the whole comparison when either
/// reading is itself unusable.
///
/// `compareHistoryPair` (`graph-portfolio.ts:519-556`).
///
/// # Errors
///
/// As [`compare_graph_quality_collections`].
pub(crate) fn compare_history_pair(
    before: (&MeasurementCollection, &GraphQualityHistoryRow),
    after: (&MeasurementCollection, &GraphQualityHistoryRow),
) -> Result<GraphQualityComparison, GraphAdapterError> {
    let (before_collection, before_row) = before;
    let (after_collection, after_row) = after;
    let mut observations = compare_graph_quality_collections(before_collection, after_collection)?;
    let unusable: Vec<&GraphQualityHistoryRow> = [before_row, after_row]
        .into_iter()
        .filter(|row| !row.availability.is_available())
        .collect();
    if !unusable.is_empty() {
        let code = GraphCompatibilityCode::CollectionIncompatible;
        let listed = unusable
            .iter()
            .map(|row| format!("{} is {}", row.id, row.availability.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        let reason = GraphCompatibilityReason {
            code,
            message: format!("{}: {listed}", code.as_str()),
        };
        for row in &mut observations {
            if row.status != ComparisonStatus::NotComputed {
                row.delta = None;
                row.status = ComparisonStatus::Incomparable;
            }
            row.reasons.push(reason.clone());
        }
    }
    Ok(GraphQualityComparison {
        before: before_row.clone(),
        after: after_row.clone(),
        observations,
    })
}

/// `compatibilityReasons` (`graph-portfolio.ts:643-693`).
///
/// # Errors
///
/// As [`compare_graph_quality_collections`].
fn compatibility_reasons(
    before: &MeasurementCollection,
    after: &MeasurementCollection,
    a: &MeasurementObservation,
    b: &MeasurementObservation,
) -> Result<Vec<GraphCompatibilityReason>, GraphAdapterError> {
    let mut reasons = Vec::new();
    mismatch(
        &mut reasons,
        GraphCompatibilityCode::PlanChanged,
        &JsonValue::string(a.plan_id.as_str()),
        &JsonValue::string(b.plan_id.as_str()),
    )?;
    mismatch(
        &mut reasons,
        GraphCompatibilityCode::DefinitionChanged,
        &JsonValue::string(a.definition_version.as_str()),
        &JsonValue::string(b.definition_version.as_str()),
    )?;
    mismatch(
        &mut reasons,
        GraphCompatibilityCode::ConfigurationChanged,
        &JsonValue::string(before.config_digest.as_str()),
        &JsonValue::string(after.config_digest.as_str()),
    )?;
    mismatch(
        &mut reasons,
        GraphCompatibilityCode::ToolChanged,
        &JsonValue::string(format!(
            "{}@{}",
            before.tool_identity.as_str(),
            before.tool_version.as_str()
        )),
        &JsonValue::string(format!(
            "{}@{}",
            after.tool_identity.as_str(),
            after.tool_version.as_str()
        )),
    )?;
    mismatch(
        &mut reasons,
        GraphCompatibilityCode::CorpusChanged,
        &corpus_revision(before),
        &corpus_revision(after),
    )?;
    if !complete(a) || !complete(b) {
        let code = GraphCompatibilityCode::PopulationIncomplete;
        reasons.push(GraphCompatibilityReason {
            code,
            message: format!(
                "{}: both observations require complete populations",
                code.as_str()
            ),
        });
    }
    mismatch(
        &mut reasons,
        GraphCompatibilityCode::PopulationChanged,
        &population_identity(a),
        &population_identity(b),
    )?;
    Ok(reasons)
}

/// `before.corpusRevision ?? null` as a JSON value.
fn corpus_revision(collection: &MeasurementCollection) -> JsonValue {
    collection
        .corpus_revision
        .as_ref()
        .map_or(JsonValue::Null, JsonValue::string)
}

/// `row.population?.identity ?? null` as a JSON value.
fn population_identity(row: &MeasurementObservation) -> JsonValue {
    row.population
        .as_ref()
        .and_then(|population| population.identity.clone())
        .unwrap_or(JsonValue::Null)
}

/// `a.population?.complete === true` (`graph-portfolio.ts:675`).
fn complete(row: &MeasurementObservation) -> bool {
    row.population
        .as_ref()
        .and_then(|population| population.complete)
        == Some(true)
}

/// `mismatch` (`graph-portfolio.ts:695-705`).
///
/// The retained comparison is on `canonicalJson` text, not on the values, so
/// the port compares the same text — which is also what makes two structurally
/// equal identities with different member order compare equal.
fn mismatch(
    reasons: &mut Vec<GraphCompatibilityReason>,
    code: GraphCompatibilityCode,
    before: &JsonValue,
    after: &JsonValue,
) -> Result<(), GraphAdapterError> {
    let before = stored_pretty_text_trimmed(before)?;
    let after = stored_pretty_text_trimmed(after)?;
    if before == after {
        return Ok(());
    }
    reasons.push(GraphCompatibilityReason {
        code,
        message: format!("{}: {before} -> {after}", code.as_str()),
    });
    Ok(())
}
