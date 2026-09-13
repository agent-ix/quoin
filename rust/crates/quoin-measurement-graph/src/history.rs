// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One retained collection, read as a governed graph reading.
//!
//! Ports `historyRow`, `partitionRow`, `partitionIdentity` and
//! `partitionCompare` (`graph-portfolio.ts:567-641,757-779`).
//!
//! # The availability ladder
//!
//! A reading is `available` only if it survives four questions, asked in this
//! order because a later one presumes the earlier answered:
//!
//! 1. Does the retained scorer attachment hash to its declared digest?
//!    A contradiction here is `unreadable` (`graph-portfolio.ts:600-602`).
//! 2. Is the producer record and scorer identity there at all? `unknown`.
//! 3. Do this collection's own graph partitions agree on plan, definition
//!    version and population identity? `incompatible`.
//! 4. Does that plan match the repository's *active* `graph_quality` plan?
//!    `incompatible`.
//!
//! The order is load-bearing and is the retained `if`/`else if` chain, not a
//! re-derivation: a collection whose attachment is corrupt is reported as
//! corrupt, not as "disagrees with the active plan".

use std::collections::BTreeSet;

use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::observation::MeasurementObservation;
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_store::JsonValue;

use crate::availability::HistoryAvailability;
use crate::canonical::stored_pretty_text_trimmed;
use crate::error::GraphAdapterError;
use crate::fields::{member, record, string_field};
use crate::order::compare_text;
use crate::reading::{
    GraphPartitionRow, GraphQualityHistoryRow, HistoryPlanRef, PartitionIdentity,
};
use crate::scorer::{ScorerIntegrityFailure, integrity_failure};

/// The metric this whole crate is about. `graph-portfolio.ts:462`.
pub const GRAPH_QUALITY_METRIC: &str = "graph_quality";

/// The sentence stated when the producer record or scorer identity is absent.
/// `graph-portfolio.ts:612-613`.
const NO_PRODUCER_IDENTITY: &str = "raw producer record or scorer identity is unavailable";

/// The sentence stated when a collection's own partitions disagree.
/// `graph-portfolio.ts:616-617`.
const PARTITIONS_DISAGREE: &str =
    "graph partitions disagree on plan/definition or population identity";

/// The sentence stated when there is no active plan to be compatible with.
/// `graph-portfolio.ts:625`.
const NO_ACTIVE_PLAN: &str = "no active graph_quality MeasurementPlan";

/// The value a plan reference reports when the collection stated none.
/// `graph-portfolio.ts:624`.
const UNKNOWN_PLAN: &str = "unknown";

/// Whether this collection holds any graph-quality observation at all.
/// `graph-portfolio.ts:466-468`.
#[must_use]
pub fn has_graph_observations(collection: &MeasurementCollection) -> bool {
    graph_observations(collection).next().is_some()
}

/// This collection's graph-quality observations, in stored order.
pub(crate) fn graph_observations(
    collection: &MeasurementCollection,
) -> impl Iterator<Item = &MeasurementObservation> {
    collection
        .observations
        .iter()
        .filter(|row| row.metric.as_str() == GRAPH_QUALITY_METRIC)
}

/// Read one retained collection as a governed graph reading.
///
/// `historyRow` (`graph-portfolio.ts:567-641`).
///
/// # Errors
///
/// [`crate::error::GraphAdapterErrorCode::Store`] for a population identity no canonical
/// JSON writer can spell.
pub fn history_row(
    path: &str,
    collection: &MeasurementCollection,
    active_plan: Option<&MeasurementPlan>,
) -> Result<GraphQualityHistoryRow, GraphAdapterError> {
    let observations: Vec<&MeasurementObservation> = graph_observations(collection).collect();
    let raw = record(Some(&collection.raw_evidence));
    let producer_record = record(member(raw, "producer"));
    let producer = member(producer_record, "producer");
    let producer_record_digest = string_field(producer_record, "observation_id");
    let scorer = record(member(raw, "scorer"));
    let scorer_digest = string_field(scorer, "digest").or_else(|| {
        string_field(
            record(member(producer_record, "raw_scorer_output")),
            "digest",
        )
    });
    let plan = observations.first().map(|first| HistoryPlanRef {
        id: first.plan_id.as_str().to_owned(),
        definition_version: first.definition_version.as_str().to_owned(),
    });
    let attachment_failure = integrity_failure(producer_record, scorer);
    let (availability, reason) = availability_of(
        attachment_failure.as_ref(),
        producer_record_digest.as_deref(),
        scorer_digest.as_deref(),
        producer,
        &observations,
        plan.as_ref(),
        active_plan,
    )?;

    let mut partitions: Vec<GraphPartitionRow> =
        observations.iter().copied().map(partition_row).collect();
    partitions.sort_by(|left, right| partition_compare(&left.identity, &right.identity));

    Ok(GraphQualityHistoryRow {
        id: collection.collection_id.as_str().to_owned(),
        path: path.to_owned(),
        timestamp: collection.timestamp.as_str().to_owned(),
        availability,
        reason,
        plan,
        tool_identity: collection.tool_identity.as_str().to_owned(),
        tool_version: collection.tool_version.as_str().to_owned(),
        config_digest: collection.config_digest.as_str().to_owned(),
        source_revision: collection.source_revision.as_str().to_owned(),
        corpus_revision: collection.corpus_revision.clone(),
        population_identity: observations
            .first()
            .and_then(|first| first.population.as_ref())
            .and_then(|population| population.identity.clone()),
        producer: producer.cloned(),
        producer_record_digest,
        scorer_digest,
        partitions,
    })
}

/// The retained `if`/`else if` ladder (`graph-portfolio.ts:600-628`).
///
/// # Errors
///
/// As [`history_row`].
#[allow(
    clippy::too_many_arguments,
    reason = "the retained ladder asks seven independent questions in a fixed \
              order; grouping them into a struct would name a thing that does \
              not exist and hide the order, which is the load-bearing part"
)]
fn availability_of(
    attachment_failure: Option<&ScorerIntegrityFailure>,
    producer_record_digest: Option<&str>,
    scorer_digest: Option<&str>,
    producer: Option<&JsonValue>,
    observations: &[&MeasurementObservation],
    plan: Option<&HistoryPlanRef>,
    active_plan: Option<&MeasurementPlan>,
) -> Result<(HistoryAvailability, Option<String>), GraphAdapterError> {
    if let Some(failure @ ScorerIntegrityFailure::Unreadable { .. }) = attachment_failure {
        return Ok((
            HistoryAvailability::Unreadable,
            Some(failure.reason().to_owned()),
        ));
    }
    if producer_record_digest.is_none()
        || scorer_digest.is_none()
        || producer.is_none()
        || attachment_failure.is_some()
    {
        return Ok((
            HistoryAvailability::Unknown,
            Some(
                attachment_failure
                    .map_or(NO_PRODUCER_IDENTITY, ScorerIntegrityFailure::reason)
                    .to_owned(),
            ),
        ));
    }
    if !self_consistent(observations)? {
        return Ok((
            HistoryAvailability::Incompatible,
            Some(PARTITIONS_DISAGREE.to_owned()),
        ));
    }
    let Some(active_plan) = active_plan else {
        return Ok((
            HistoryAvailability::Incompatible,
            Some(NO_ACTIVE_PLAN.to_owned()),
        ));
    };
    let matches = plan.is_some_and(|plan| {
        plan.id == active_plan.id.as_str()
            && plan.definition_version == active_plan.definition_version.as_str()
    });
    if matches {
        return Ok((HistoryAvailability::Available, None));
    }
    let (id, definition_version) = plan.map_or((UNKNOWN_PLAN, UNKNOWN_PLAN), |plan| {
        (plan.id.as_str(), plan.definition_version.as_str())
    });
    Ok((
        HistoryAvailability::Incompatible,
        Some(format!(
            "collection plan {id}/{definition_version} does not match active {}/{}",
            active_plan.id.as_str(),
            active_plan.definition_version.as_str()
        )),
    ))
}

/// Whether every graph partition in one collection names the same plan,
/// definition version and population identity.
///
/// `graph-portfolio.ts:604-607` builds two `Set`s and asks whether each holds
/// exactly one member. A collection with **no** graph partitions therefore has
/// sets of size 0 and is not self-consistent — but this function is only
/// reached for a collection that has at least one
/// (`graph-portfolio.ts:466-468` filters), so the size-1 test and this
/// "all equal" test agree everywhere they are both defined.
fn self_consistent(observations: &[&MeasurementObservation]) -> Result<bool, GraphAdapterError> {
    let mut plan_keys: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut population_keys: BTreeSet<String> = BTreeSet::new();
    for row in observations {
        plan_keys.insert((row.plan_id.as_str(), row.definition_version.as_str()));
        population_keys.insert(stored_pretty_text_trimmed(
            row.population
                .as_ref()
                .and_then(|population| population.identity.as_ref())
                .unwrap_or(&JsonValue::Null),
        )?);
    }
    Ok(plan_keys.len() == 1 && population_keys.len() == 1)
}

/// `partitionRow` (`graph-portfolio.ts:757-768`).
fn partition_row(row: &MeasurementObservation) -> GraphPartitionRow {
    GraphPartitionRow {
        identity: partition_identity(row),
        state: row.state,
        value: row.value,
        unit: row.unit.as_str().to_owned(),
        shape: row.shape,
        // Spread in only when truthy, so the empty string is absent too
        // (`graph-portfolio.ts:766`).
        reason: row.reason.clone().filter(|reason| !reason.is_empty()),
    }
}

/// `partitionIdentity` (`graph-portfolio.ts:770-778`).
///
/// The retained `row.dimensions?.measure ?? "unknown"` reads a member declared
/// `Record<string, string>` and nothing validates that declaration, so a
/// non-string member reaches the renderer in TypeScript and is refused here.
/// That divergence is declared in `DIVERGENCE.md` §2 and is why this reads
/// through [`quoin_store::JsonValue::as_str`] rather than through
/// `js_string`: a number here would be a *different measure name* in the
/// partition key, not merely a different rendering.
pub(crate) fn partition_identity(row: &MeasurementObservation) -> PartitionIdentity {
    let dimensions = row.dimensions.entries();
    let named = |name: &str| {
        dimensions
            .get(name)
            .and_then(JsonValue::as_str)
            .unwrap_or(PartitionIdentity::UNKNOWN)
            .to_owned()
    };
    PartitionIdentity {
        measure: named("measure"),
        dimension: named("dimension"),
        key: named("key"),
    }
}

/// `partitionCompare` (`graph-portfolio.ts:774-779`), which is `compare` on
/// each of the three names in turn.
pub(crate) fn partition_compare(
    left: &PartitionIdentity,
    right: &PartitionIdentity,
) -> std::cmp::Ordering {
    compare_text(&left.measure, &right.measure)
        .then_with(|| compare_text(&left.dimension, &right.dimension))
        .then_with(|| compare_text(&left.key, &right.key))
}
