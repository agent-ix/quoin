// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The governed graph portfolio as canonical JSON.
//!
//! Ports `canonicalGraphPortfolioJson` (`graph-portfolio.ts:326-329`), which
//! is `canonicalJson(report)` over the whole object.
//!
//! # There is one canonicaliser and it is not here
//!
//! [`quoin_store::canonical_json_bytes`] writes every byte. This module
//! decides only *which members exist* — which is the part `JSON.stringify`
//! decides by looking at the in-memory object, and therefore the part a port
//! has to state. Two rules do all of it:
//!
//! * a member the retained object assigns unconditionally is stated, `null`
//!   included ([`Option`] with no `skip_serializing_if`); and
//! * a member the retained object spreads in conditionally is absent when it
//!   is not spread (`skip_serializing_if = "Option::is_none"`), because
//!   `JSON.stringify` drops `undefined`.
//!
//! `tests/tc_476_boundary.rs` asserts over this crate's sources that no second
//! canonicaliser, JSON writer or sha256 appeared here (FR-100-AC-8,
//! FR-100-CON-4).

use quoin_measurement::json_bridge::{from_serde, to_serde};
use quoin_measurement::portfolio::render_json::RepositoryWire;
use quoin_measurement::report::wire::PlanWire;
use quoin_store::{JsonObject, JsonValue};
use serde::Serialize;
use serde_json::Value;

use crate::canonical::stored_pretty_bytes;
use crate::error::GraphAdapterError;
use crate::input::{
    ChangeImpact, ChangeImpactAbsence, GraphPortfolioGap, NormalizedStructuralGraph,
};
use crate::reading::{
    GraphCompatibilityReason, GraphPartitionRow, GraphQualityComparison, GraphQualityComparisonRow,
    GraphQualityHistoryRow, HistoryPlanRef,
};
use crate::report::{GovernedGraphPortfolioReport, GovernedGraphRepositoryReport, GraphQuality};

/// The wire spelling of a settled-absent change-impact member.
/// `graph-portfolio.ts:180,187-189,196-199`.
const NOT_APPLICABLE: &str = "not_applicable";

/// Render the governed graph portfolio as the store's canonical JSON.
///
/// `canonicalGraphPortfolioJson` (`graph-portfolio.ts:326-329`), trailing
/// newline included.
///
/// # Errors
///
/// [`crate::error::GraphAdapterErrorCode::Store`] for a value no canonical JSON writer can
/// spell — a non-finite number, which no wire view here can hold.
pub fn canonical_graph_portfolio_json(
    report: &GovernedGraphPortfolioReport,
) -> Result<String, GraphAdapterError> {
    let bytes = canonical_graph_portfolio_json_bytes(report)?;
    // `canonical_json_bytes` is `canonical_json(..).into_bytes()`, so these
    // bytes are UTF-8 by construction; the fallback exists so the renderer is
    // total rather than because the branch can be reached.
    Ok(String::from_utf8(bytes).unwrap_or_default())
}

/// [`canonical_graph_portfolio_json`], as the bytes that are written.
///
/// # Errors
///
/// As [`canonical_graph_portfolio_json`].
pub fn canonical_graph_portfolio_json_bytes(
    report: &GovernedGraphPortfolioReport,
) -> Result<Vec<u8>, GraphAdapterError> {
    let wire = ReportWire::of(report)?;
    let analysis = serde_json::to_value(&wire).map_err(|error| {
        GraphAdapterError::new(
            crate::error::GraphAdapterErrorCode::Store,
            format!("governed graph portfolio does not serialise: {error}"),
        )
    })?;
    stored_pretty_bytes(&from_serde(&analysis)?)
}

/// The `changeImpact` member as a stored value.
///
/// Shared with [`crate::render`], which renders the same member as canonical
/// JSON inside the Markdown report (`graph-portfolio.ts:401`) — one spelling,
/// not two.
#[must_use]
pub fn change_impact_value(change_impact: &ChangeImpact) -> JsonValue {
    match *change_impact {
        ChangeImpact::Analyses(ref analyses) => JsonValue::Array(analyses.clone()),
        ChangeImpact::NotApplicable(absence) => not_applicable(absence),
    }
}

/// `{ availability: "not_applicable", reason }`.
fn not_applicable(absence: ChangeImpactAbsence) -> JsonValue {
    let mut object = JsonObject::new();
    object.set("availability", JsonValue::string(NOT_APPLICABLE));
    object.set("reason", JsonValue::string(absence.reason()));
    JsonValue::Object(object)
}

/// `graph-portfolio.ts:202-207`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportWire<'a> {
    schema_version: i64,
    /// `| null`.
    newest_collection_timestamp: Option<&'a str>,
    stale_after_days: i64,
    repositories: Vec<RepositoryEntryWire<'a>>,
}

impl<'a> ReportWire<'a> {
    /// The wire view of the whole report.
    fn of(report: &'a GovernedGraphPortfolioReport) -> Result<Self, GraphAdapterError> {
        Ok(Self {
            schema_version: GovernedGraphPortfolioReport::SCHEMA_VERSION,
            newest_collection_timestamp: report.newest_collection_timestamp.as_deref(),
            stale_after_days: GovernedGraphPortfolioReport::STALE_AFTER_DAYS,
            repositories: report
                .repositories
                .iter()
                .map(RepositoryEntryWire::of)
                .collect::<Result<Vec<_>, GraphAdapterError>>()?,
        })
    }
}

/// `{ ...portfolio, graphQuality, graph, gaps }` (`graph-portfolio.ts:452-457`).
///
/// The spread is a `serde` flatten of the portfolio crate's own wire view, so
/// a member added there appears here without this file being edited — which is
/// exactly what the retained spread does.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryEntryWire<'a> {
    #[serde(flatten)]
    portfolio: RepositoryWire<'a>,
    graph_quality: GraphQualityWire<'a>,
    graph: GraphWire<'a>,
    gaps: Vec<GapWire<'a>>,
}

impl<'a> RepositoryEntryWire<'a> {
    /// The wire view of one repository entry.
    fn of(repository: &'a GovernedGraphRepositoryReport) -> Result<Self, GraphAdapterError> {
        Ok(Self {
            portfolio: RepositoryWire::of(&repository.base)?,
            graph_quality: GraphQualityWire::of(&repository.graph_quality)?,
            graph: GraphWire::of(&repository.graph)?,
            gaps: repository.gaps.iter().map(GapWire::of).collect(),
        })
    }
}

/// `graph-portfolio.ts:167-172`.
#[derive(Debug, Serialize)]
struct GraphQualityWire<'a> {
    /// `| null`.
    plan: Option<PlanWire<'a>>,
    /// `| null`.
    current: Option<HistoryRowWire<'a>>,
    history: Vec<HistoryRowWire<'a>>,
    /// `| null`.
    comparison: Option<ComparisonWire<'a>>,
}

impl<'a> GraphQualityWire<'a> {
    /// The wire view of one repository's governed readings.
    fn of(quality: &'a GraphQuality) -> Result<Self, GraphAdapterError> {
        Ok(Self {
            plan: quality.plan.as_ref().map(PlanWire::of),
            current: quality
                .current
                .as_ref()
                .map(HistoryRowWire::of)
                .transpose()?,
            history: quality
                .history
                .iter()
                .map(HistoryRowWire::of)
                .collect::<Result<Vec<_>, GraphAdapterError>>()?,
            comparison: quality
                .comparison
                .as_ref()
                .map(ComparisonWire::of)
                .transpose()?,
        })
    }
}

/// `graph-portfolio.ts:113-131`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HistoryRowWire<'a> {
    id: &'a str,
    path: &'a str,
    timestamp: &'a str,
    availability: &'static str,
    /// `| null`.
    reason: Option<&'a str>,
    /// `| null`.
    plan: Option<PlanRefWire<'a>>,
    tool_identity: &'a str,
    tool_version: &'a str,
    config_digest: &'a str,
    source_revision: &'a str,
    /// `| null`.
    corpus_revision: Option<&'a str>,
    /// `unknown`, assigned `?? null`, so stated as `null` when absent.
    population_identity: Value,
    /// As `populationIdentity`.
    producer: Value,
    /// `| null`.
    producer_record_digest: Option<&'a str>,
    /// `| null`.
    scorer_digest: Option<&'a str>,
    partitions: Vec<PartitionWire<'a>>,
}

impl<'a> HistoryRowWire<'a> {
    /// The wire view of one reading.
    fn of(row: &'a GraphQualityHistoryRow) -> Result<Self, GraphAdapterError> {
        Ok(Self {
            id: &row.id,
            path: &row.path,
            timestamp: &row.timestamp,
            availability: row.availability.as_str(),
            reason: row.reason.as_deref(),
            plan: row.plan.as_ref().map(PlanRefWire::of),
            tool_identity: &row.tool_identity,
            tool_version: &row.tool_version,
            config_digest: &row.config_digest,
            source_revision: &row.source_revision,
            corpus_revision: row.corpus_revision.as_deref(),
            population_identity: opaque(row.population_identity.as_ref())?,
            producer: opaque(row.producer.as_ref())?,
            producer_record_digest: row.producer_record_digest.as_deref(),
            scorer_digest: row.scorer_digest.as_deref(),
            partitions: row.partitions.iter().map(PartitionWire::of).collect(),
        })
    }
}

/// An opaque stored value assigned through `?? null`.
fn opaque(value: Option<&JsonValue>) -> Result<Value, GraphAdapterError> {
    Ok(to_serde(value.unwrap_or(&JsonValue::Null))?)
}

/// `graph-portfolio.ts:119`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanRefWire<'a> {
    id: &'a str,
    definition_version: &'a str,
}

impl<'a> PlanRefWire<'a> {
    /// The wire view of one plan reference.
    fn of(plan: &'a HistoryPlanRef) -> Self {
        Self {
            id: &plan.id,
            definition_version: &plan.definition_version,
        }
    }
}

/// `graph-portfolio.ts:102-111`.
#[derive(Debug, Serialize)]
struct PartitionWire<'a> {
    measure: &'a str,
    dimension: &'a str,
    key: &'a str,
    state: &'static str,
    /// `number | null`.
    value: Option<f64>,
    unit: &'a str,
    shape: &'static str,
    /// Spread in only when truthy, so absent rather than `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'a str>,
}

impl<'a> PartitionWire<'a> {
    /// The wire view of one partition.
    fn of(row: &'a GraphPartitionRow) -> Self {
        Self {
            measure: &row.identity.measure,
            dimension: &row.identity.dimension,
            key: &row.identity.key,
            state: row.state.as_str(),
            value: row.value,
            unit: &row.unit,
            shape: row.shape.as_str(),
            reason: row.reason.as_deref(),
        }
    }
}

/// `graph-portfolio.ts:160-164`.
#[derive(Debug, Serialize)]
struct ComparisonWire<'a> {
    before: HistoryRowWire<'a>,
    after: HistoryRowWire<'a>,
    observations: Vec<ComparisonRowWire<'a>>,
}

impl<'a> ComparisonWire<'a> {
    /// The wire view of one comparison.
    fn of(comparison: &'a GraphQualityComparison) -> Result<Self, GraphAdapterError> {
        Ok(Self {
            before: HistoryRowWire::of(&comparison.before)?,
            after: HistoryRowWire::of(&comparison.after)?,
            observations: comparison
                .observations
                .iter()
                .map(ComparisonRowWire::of)
                .collect(),
        })
    }
}

/// `graph-portfolio.ts:150-158`.
#[derive(Debug, Serialize)]
struct ComparisonRowWire<'a> {
    measure: &'a str,
    dimension: &'a str,
    key: &'a str,
    /// `| null`.
    before: Option<f64>,
    /// `| null`.
    after: Option<f64>,
    /// `| null`.
    delta: Option<f64>,
    status: &'static str,
    reasons: Vec<ReasonWire<'a>>,
}

impl<'a> ComparisonRowWire<'a> {
    /// The wire view of one compared partition.
    fn of(row: &'a GraphQualityComparisonRow) -> Self {
        Self {
            measure: &row.identity.measure,
            dimension: &row.identity.dimension,
            key: &row.identity.key,
            before: row.before,
            after: row.after,
            delta: row.delta,
            status: row.status.as_str(),
            reasons: row.reasons.iter().map(ReasonWire::of).collect(),
        }
    }
}

/// `graph-portfolio.ts:144-148`.
#[derive(Debug, Serialize)]
struct ReasonWire<'a> {
    code: &'static str,
    blocking: bool,
    message: &'a str,
}

impl<'a> ReasonWire<'a> {
    /// The wire view of one stated incompatibility.
    fn of(reason: &'a GraphCompatibilityReason) -> Self {
        Self {
            code: reason.code.as_str(),
            blocking: GraphCompatibilityReason::BLOCKING,
            message: &reason.message,
        }
    }
}

/// `graph-portfolio.ts:176-200`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GraphWire<'a> {
    availability: &'a str,
    /// Optional in the retained type, so absent rather than `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<&'a str>,
    /// Stated only by the available variant.
    #[serde(skip_serializing_if = "Option::is_none")]
    premises: Option<Value>,
    /// As `premises`.
    #[serde(skip_serializing_if = "Option::is_none")]
    fan_out: Option<Value>,
    /// As `premises`.
    #[serde(skip_serializing_if = "Option::is_none")]
    churn: Option<Value>,
    /// Stated only by the unavailable variant.
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'a str>,
    /// Always stated: normalising it is what [`crate::build`] does.
    change_impact: Value,
}

impl<'a> GraphWire<'a> {
    /// The wire view of one normalised graph.
    fn of(graph: &'a NormalizedStructuralGraph) -> Result<Self, GraphAdapterError> {
        Ok(match *graph {
            NormalizedStructuralGraph::Available {
                ref path,
                ref premises,
                ref fan_out,
                ref churn,
                ref change_impact,
            } => Self {
                availability: graph.availability(),
                path: path.as_deref(),
                premises: Some(to_serde(premises)?),
                fan_out: Some(to_serde(fan_out)?),
                churn: Some(to_serde(churn)?),
                reason: None,
                change_impact: to_serde(&change_impact_value(change_impact))?,
            },
            NormalizedStructuralGraph::Unavailable {
                ref path,
                ref reason,
                ..
            } => Self {
                availability: graph.availability(),
                path: path.as_deref(),
                premises: None,
                fan_out: None,
                churn: None,
                reason: Some(reason),
                change_impact: to_serde(&not_applicable(ChangeImpactAbsence::NoAcceptedExport))?,
            },
        })
    }
}

/// `graph-portfolio.ts:26-33`.
#[derive(Debug, Serialize)]
struct GapWire<'a> {
    availability: &'static str,
    subject: &'static str,
    /// `| null`.
    path: Option<&'a str>,
    reason: &'a str,
    /// Assigned only when the active plan states it.
    #[serde(skip_serializing_if = "Option::is_none")]
    owner: Option<&'a str>,
    /// As `owner`.
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<&'a str>,
}

impl<'a> GapWire<'a> {
    /// The wire view of one gap.
    fn of(gap: &'a GraphPortfolioGap) -> Self {
        Self {
            availability: gap.availability.as_str(),
            subject: gap.subject.as_str(),
            path: gap.path.as_deref(),
            reason: &gap.reason,
            owner: gap.owner.as_deref(),
            action: gap.action.as_deref(),
        }
    }
}
