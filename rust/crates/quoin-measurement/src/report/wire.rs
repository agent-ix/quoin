// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The JSON view of a measurement report, and the one `canonicalJson` route.
//!
//! `renderMeasurementReportJson` and `renderPortfolioReportJson` are both
//! `canonicalJson(report)` (`report.ts:202-204`, `portfolio.ts:219-221`), which
//! serialises whatever the in-memory object happens to hold. The retained
//! objects are the parsed store documents themselves — `validate.ts:16` is an
//! `asserts` and rebuilds nothing — so the JSON must state exactly the members
//! the record stated, and no others.
//!
//! That is what these wire types are for. They are not a second model: they are
//! the projection that decides, member by member, what `JSON.stringify` would
//! have emitted, including the two places it emits nothing:
//!
//! * an absent optional member is dropped, because `JSON.stringify` drops an
//!   `undefined` value — so [`Option`] plus `skip_serializing_if` is the rule
//!   for every `?:` member and the bare `Option` (rendered `null`) is the rule
//!   for every `| null` member; and
//! * an absent `dimensions` object is not an empty one, which is why
//!   [`crate::types::observation::Dimensions`] models the difference.
//!
//! # Where the bytes come from
//!
//! [`canonical_json_of`] is the only route: `serde` states the shape,
//! [`crate::json_bridge::from_serde`] carries it to the store's value, and
//! [`quoin_store::canonical_json`] writes it. `canonicalJson` sorts member
//! names at every level and `JSON.stringify` then hoists array-index names, and
//! that is precisely `ecmascript_own_property_order`, so the sort the retained
//! function performs is already the writer's.

use std::collections::BTreeMap;

use quoin_store::{JsonValue, canonical_json};
use serde::Serialize;
use serde_json::Value;

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::json_bridge::{from_serde, to_serde};
use crate::report::build::{CollectionSummary, CurrentRow, MeasurementReport};
use crate::types::comparison::MeasurementComparison;
use crate::types::observation::{MeasurementObservation, MeasurementPopulation};
use crate::types::plan::MeasurementPlan;

/// `canonicalJson(value)` (`src/store/canonical.ts:19`), for a wire view.
///
/// # Errors
///
/// [`MeasurementErrorCode::Store`] for a value no canonical JSON writer can
/// spell — a non-finite number, which no wire view here can hold.
pub fn canonical_json_of<T: Serialize>(value: &T) -> Result<String, MeasurementError> {
    let analysis = serde_json::to_value(value).map_err(|error| {
        MeasurementError::new(
            MeasurementErrorCode::Store,
            format!("report does not serialise: {error}"),
        )
    })?;
    Ok(canonical_json(&from_serde(&analysis)?)?)
}

/// A `MeasurementPlan` as `plans.ts:69-80` returns it.
///
/// `pub` because the governed graph portfolio (`quoin-measurement-graph`,
/// quoin#476) states the same plan under its own `graphQuality.plan` member.
/// Two spellings of one stored object would be two chances to drop a member.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanWire<'a> {
    id: &'a str,
    title: &'a str,
    status: &'a str,
    stage: &'a str,
    metric: &'a str,
    definition_version: &'a str,
    path: &'a str,
    /// Stated only when governance fields were requested (`plans.ts:77-80`).
    #[serde(skip_serializing_if = "Option::is_none")]
    owner: Option<&'a str>,
    /// As `owner`.
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<&'a str>,
}

impl<'a> PlanWire<'a> {
    /// The wire view of one plan.
    #[must_use]
    pub fn of(plan: &'a MeasurementPlan) -> Self {
        Self {
            id: plan.id.as_str(),
            title: plan.title.as_str(),
            status: plan.status.as_str(),
            stage: plan.stage.as_str(),
            metric: plan.metric.as_str(),
            definition_version: plan.definition_version.as_str(),
            path: &plan.path,
            owner: plan.owner.as_deref(),
            action: plan.action.as_deref(),
        }
    }
}

/// A stored `population` object, with the members it stated.
#[derive(Debug, Serialize)]
pub(crate) struct PopulationWire {
    #[serde(skip_serializing_if = "Option::is_none")]
    examined: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    matched: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    complete: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    identity: Option<Value>,
    /// Everything else the record stated, kept as it was stored.
    #[serde(flatten)]
    unmodelled: BTreeMap<String, Value>,
}

impl PopulationWire {
    /// The wire view of one population.
    ///
    /// # Errors
    ///
    /// As [`canonical_json_of`].
    fn of(population: &MeasurementPopulation) -> Result<Self, MeasurementError> {
        Ok(Self {
            examined: population.examined,
            matched: population.matched,
            complete: population.complete,
            identity: population.identity.as_ref().map(to_serde).transpose()?,
            unmodelled: analysis_map(&population.unmodelled)?,
        })
    }
}

/// A stored observation. `types.ts:15-26`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ObservationWire {
    metric: String,
    plan_id: String,
    definition_version: String,
    state: &'static str,
    /// `number | null`: stated even when there is no value.
    value: Option<f64>,
    unit: String,
    shape: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    population: Option<PopulationWire>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

impl ObservationWire {
    /// The wire view of one observation.
    ///
    /// # Errors
    ///
    /// As [`canonical_json_of`].
    pub(crate) fn of(observation: &MeasurementObservation) -> Result<Self, MeasurementError> {
        Ok(Self {
            metric: observation.metric.as_str().to_owned(),
            plan_id: observation.plan_id.as_str().to_owned(),
            definition_version: observation.definition_version.as_str().to_owned(),
            state: observation.state.as_str(),
            value: observation.value,
            unit: observation.unit.as_str().to_owned(),
            shape: observation.shape.as_str(),
            population: observation
                .population
                .as_ref()
                .map(PopulationWire::of)
                .transpose()?,
            dimensions: observation
                .dimensions
                .stated_entries()
                .map(analysis_map)
                .transpose()?,
            reason: observation.reason.clone(),
        })
    }
}

/// The collection members a report row quotes. `report.ts:85-96`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CollectionSummaryWire<'a> {
    collection_id: &'a str,
    timestamp: &'a str,
    tool_identity: &'a str,
    tool_version: &'a str,
    config_digest: &'a str,
    source_revision: &'a str,
    /// Assigned from an optional member, so absent rather than `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    corpus_revision: Option<&'a str>,
    path: &'a str,
}

impl<'a> CollectionSummaryWire<'a> {
    /// The wire view of one collection quote.
    pub(crate) fn of(summary: &'a CollectionSummary) -> Self {
        Self {
            collection_id: &summary.collection_id,
            timestamp: &summary.timestamp,
            tool_identity: &summary.tool_identity,
            tool_version: &summary.tool_version,
            config_digest: &summary.config_digest,
            source_revision: &summary.source_revision,
            corpus_revision: summary.corpus_revision.as_deref(),
            path: &summary.path,
        }
    }
}

/// One row of the current table. `report.ts:26-44`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CurrentRowWire<'a> {
    metric: &'a str,
    plan_id: &'a str,
    plan_path: &'a str,
    plan_definition_version: &'a str,
    stage: &'static str,
    /// `| null`: stated as `null` when no collection computed the metric.
    observation: Option<ObservationWire>,
    /// As `observation`.
    collection: Option<CollectionSummaryWire<'a>>,
}

impl<'a> CurrentRowWire<'a> {
    /// The wire view of one row.
    ///
    /// # Errors
    ///
    /// As [`canonical_json_of`].
    fn of(row: &'a CurrentRow) -> Result<Self, MeasurementError> {
        Ok(Self {
            metric: &row.metric,
            plan_id: &row.plan_id,
            plan_path: &row.plan_path,
            plan_definition_version: &row.plan_definition_version,
            stage: row.stage.as_str(),
            observation: row
                .observation
                .as_ref()
                .map(ObservationWire::of)
                .transpose()?,
            collection: row.collection.as_ref().map(CollectionSummaryWire::of),
        })
    }
}

/// The whole report. `report.ts:24-46`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MeasurementReportWire<'a> {
    plans: Vec<PlanWire<'a>>,
    current: Vec<CurrentRowWire<'a>>,
    corpus_gaps: Option<f64>,
    /// Already `snake_case` on the wire (`intervention-report.ts:3-22`), so it
    /// crosses as its own `serde` view rather than being restated here.
    interventions: Value,
    /// As `interventions` (`operational-report.ts:3-15`).
    operational: Value,
}

impl<'a> MeasurementReportWire<'a> {
    /// The wire view of one report.
    ///
    /// # Errors
    ///
    /// As [`canonical_json_of`].
    pub(crate) fn of(report: &'a MeasurementReport) -> Result<Self, MeasurementError> {
        Ok(Self {
            plans: report.plans.iter().map(PlanWire::of).collect(),
            current: report
                .current
                .iter()
                .map(CurrentRowWire::of)
                .collect::<Result<Vec<_>, MeasurementError>>()?,
            corpus_gaps: report.corpus_gaps,
            interventions: analysis_value(&report.interventions)?,
            operational: analysis_value(&report.operational)?,
        })
    }
}

/// A `serde` view of a value that already has a `Serialize` impl.
fn analysis_value<T: Serialize>(value: &T) -> Result<Value, MeasurementError> {
    serde_json::to_value(value).map_err(|error| {
        MeasurementError::new(
            MeasurementErrorCode::Store,
            format!("report entry does not serialise: {error}"),
        )
    })
}

/// A map of stored values, crossed to the analysis value once.
fn analysis_map(
    stored: &BTreeMap<String, JsonValue>,
) -> Result<BTreeMap<String, Value>, MeasurementError> {
    stored
        .iter()
        .map(|(name, value)| Ok((name.clone(), to_serde(value)?)))
        .collect()
}

/// One comparison reason. `types.ts:79-87`.
#[derive(Debug, Serialize)]
pub(crate) struct ComparisonReasonWire<'a> {
    code: &'static str,
    message: &'a str,
    blocking: bool,
}

/// One compared slice. `types.ts:89-99`.
///
/// Shared with [`crate::portfolio`], whose JSON view embeds the same
/// comparisons (`portfolio.ts:40-44`).
#[derive(Debug, Serialize)]
pub(crate) struct ComparisonWire<'a> {
    metric: &'a str,
    /// Always stated: `compare.ts:136` writes `a.dimensions ?? {}`.
    dimensions: BTreeMap<String, Value>,
    before: Option<f64>,
    after: Option<f64>,
    delta: Option<f64>,
    status: &'static str,
    reasons: Vec<ComparisonReasonWire<'a>>,
}

impl<'a> ComparisonWire<'a> {
    /// The wire view of one comparison.
    ///
    /// # Errors
    ///
    /// As [`canonical_json_of`].
    pub(crate) fn of(comparison: &'a MeasurementComparison) -> Result<Self, MeasurementError> {
        Ok(Self {
            metric: &comparison.metric,
            dimensions: analysis_map(&comparison.dimensions)?,
            before: comparison.before,
            after: comparison.after,
            delta: comparison.delta,
            status: comparison.status.as_str(),
            reasons: comparison
                .reasons
                .iter()
                .map(|reason| ComparisonReasonWire {
                    code: reason.code.as_str(),
                    message: &reason.message,
                    blocking: reason.blocking,
                })
                .collect(),
        })
    }

    /// The wire view of a list of comparisons.
    ///
    /// # Errors
    ///
    /// As [`canonical_json_of`].
    pub(crate) fn all(
        comparisons: &'a [MeasurementComparison],
    ) -> Result<Vec<Self>, MeasurementError> {
        comparisons.iter().map(Self::of).collect()
    }
}
