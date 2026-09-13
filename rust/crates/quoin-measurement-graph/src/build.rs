// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Assembling the governed view from already-read inputs.
//!
//! Ports `buildGovernedGraphPortfolioFrom`, `buildRepository`,
//! `sanitizeInheritedGraphCurrent` and `normalizeGraph`
//! (`graph-portfolio.ts:269-296,408-518,557-565`).
//!
//! # Nothing here reads a file
//!
//! Every input arrives already read: the collections, the plans, the
//! ungoverned portfolio entry and the opaque structural graph. That is the
//! same boundary `graph-portfolio-load.ts` draws in the retained tree, and it
//! is what makes the whole projection testable against a golden corpus with no
//! filesystem and no oracle process.
//!
//! # Why the inherited current row is sanitised
//!
//! The ungoverned portfolio entry already carries a `graph_quality` current
//! row, chosen by recency alone. The governed view holds evidence to a higher
//! bar — it must come from the collection this view accepted, under the active
//! plan, at that plan's definition version — so a row that does not meet it is
//! **emptied rather than dropped**: the metric is still governed and still
//! reported, with no observation behind it. Dropping the row would report a
//! plan that does not exist.

use quoin_measurement::portfolio::PortfolioRepositoryReport;
use quoin_measurement::report::build::CurrentRow;
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::plan::{LifecycleStatus, MeasurementPlan};

use crate::availability::GapSubject;
use crate::compare::compare_history_pair;
use crate::error::GraphAdapterError;
use crate::history::{GRAPH_QUALITY_METRIC, has_graph_observations, history_row};
use crate::input::{
    ChangeImpact, ChangeImpactAbsence, GraphCollectionRead, GraphPortfolioGap,
    GraphPortfolioRepositoryInput, InjectedStructuralGraph, NormalizedStructuralGraph,
};
use crate::order::{compare_instants, compare_text};
use crate::reading::GraphQualityHistoryRow;
use crate::report::{GovernedGraphPortfolioReport, GovernedGraphRepositoryReport, GraphQuality};

/// The sentence a history row that is not current-compatible states when it
/// gave no reason of its own. `graph-portfolio.ts:497`.
const NOT_CURRENT_COMPATIBLE: &str = "collection is not current-compatible";

/// Build the governed graph portfolio from already-read inputs.
///
/// `buildGovernedGraphPortfolioFrom` (`graph-portfolio.ts:269-287`).
///
/// # Errors
///
/// [`crate::error::GraphAdapterErrorCode::Store`] for a retained population identity no
/// canonical JSON writer can spell.
pub fn build_governed_graph_portfolio_from(
    inputs: &[GraphPortfolioRepositoryInput],
) -> Result<GovernedGraphPortfolioReport, GraphAdapterError> {
    let mut ordered: Vec<&GraphPortfolioRepositoryInput> = inputs.iter().collect();
    ordered.sort_by(|left, right| compare_text(&left.portfolio.root, &right.portfolio.root));
    let repositories = ordered
        .into_iter()
        .map(build_repository)
        .collect::<Result<Vec<_>, GraphAdapterError>>()?;

    let mut timestamps: Vec<&str> = repositories
        .iter()
        .filter_map(|repository| {
            repository
                .base
                .latest_collection
                .as_ref()
                .map(|collection| collection.timestamp.as_str())
        })
        .collect();
    timestamps.sort_by(|left, right| compare_instants(left, right));

    Ok(GovernedGraphPortfolioReport {
        newest_collection_timestamp: timestamps.last().map(|last| (*last).to_owned()),
        repositories,
    })
}

/// One repository's governed entry. `buildRepository` (`graph-portfolio.ts:408-518`).
fn build_repository(
    input: &GraphPortfolioRepositoryInput,
) -> Result<GovernedGraphRepositoryReport, GraphAdapterError> {
    let active_plan = input
        .plans
        .iter()
        .find(|plan| {
            plan.status == LifecycleStatus::Active && plan.metric.as_str() == GRAPH_QUALITY_METRIC
        })
        .cloned();

    let graph_quality = graph_quality_of(input, active_plan)?;
    let graph = normalize_graph(&input.graph);
    let gaps = gaps_of(input, &graph_quality, &graph);

    Ok(GovernedGraphRepositoryReport {
        base: sanitize_inherited_graph_current(
            &input.portfolio,
            graph_quality.plan.as_ref(),
            graph_quality.current.as_ref(),
        ),
        graph_quality,
        graph,
        gaps,
    })
}

/// The governed `graphQuality` block: the readable history in retained order,
/// the newest available row, and the last-pair comparison.
/// `graph-portfolio.ts:414-462`.
fn graph_quality_of(
    input: &GraphPortfolioRepositoryInput,
    active_plan: Option<MeasurementPlan>,
) -> Result<GraphQuality, GraphAdapterError> {
    let mut pairs: Vec<(&MeasurementCollection, GraphQualityHistoryRow)> = Vec::new();
    for read in &input.collections {
        let Some(collection) = read.collection() else {
            continue;
        };
        if !has_graph_observations(collection) {
            continue;
        }
        pairs.push((
            collection,
            history_row(read.path(), collection, active_plan.as_ref())?,
        ));
    }
    pairs.sort_by(|(_, left), (_, right)| {
        compare_instants(&left.timestamp, &right.timestamp)
            .then_with(|| compare_text(&left.id, &right.id))
    });

    let history: Vec<GraphQualityHistoryRow> = pairs.iter().map(|(_, row)| row.clone()).collect();
    let current = pairs
        .iter()
        .rev()
        .find(|(_, row)| row.availability.is_available())
        .map(|(_, row)| row.clone());
    let comparison = match pairs.as_slice() {
        [.., before, after] => Some(compare_history_pair(
            (before.0, &before.1),
            (after.0, &after.1),
        )?),
        _ => None,
    };

    Ok(GraphQuality {
        plan: active_plan,
        current,
        history,
        comparison,
    })
}

/// Every gap the governed view states, in the retained order: refused
/// collections, then unavailable history rows, then the structural graph, each
/// carrying the active plan's governance fields when it states them.
/// `graph-portfolio.ts:464-515`.
fn gaps_of(
    input: &GraphPortfolioRepositoryInput,
    graph_quality: &GraphQuality,
    graph: &NormalizedStructuralGraph,
) -> Vec<GraphPortfolioGap> {
    let mut gaps: Vec<GraphPortfolioGap> = input
        .collections
        .iter()
        .filter_map(|read| match *read {
            GraphCollectionRead::Read { .. } => None,
            GraphCollectionRead::Refused {
                ref path,
                availability,
                ref reason,
            } => Some(GraphPortfolioGap {
                availability: availability.into(),
                subject: GapSubject::Collection,
                path: Some(path.clone()),
                reason: reason.clone(),
                owner: None,
                action: None,
            }),
        })
        .collect();
    for row in &graph_quality.history {
        if let Some(availability) = row.availability.as_gap() {
            gaps.push(GraphPortfolioGap {
                availability,
                subject: GapSubject::Collection,
                path: Some(row.path.clone()),
                reason: row
                    .reason
                    .clone()
                    .unwrap_or_else(|| NOT_CURRENT_COMPATIBLE.to_owned()),
                owner: None,
                action: None,
            });
        }
    }
    if let NormalizedStructuralGraph::Unavailable {
        availability,
        ref path,
        ref reason,
    } = *graph
    {
        gaps.push(GraphPortfolioGap {
            availability,
            subject: GapSubject::GraphExport,
            path: path.clone(),
            reason: reason.clone(),
            owner: None,
            action: None,
        });
    }
    if let Some(plan) = graph_quality.plan.as_ref() {
        // `graph-portfolio.ts:511-515` applies each governance field only when
        // the plan states it, so a plan with an owner and no action leaves
        // `action` absent rather than emptying it.
        if plan.owner.is_some() || plan.action.is_some() {
            for gap in &mut gaps {
                gap.owner.clone_from(&plan.owner);
                gap.action.clone_from(&plan.action);
            }
        }
    }
    gaps
}

/// `sanitizeInheritedGraphCurrent` (`graph-portfolio.ts:557-565`).
fn sanitize_inherited_graph_current(
    portfolio: &PortfolioRepositoryReport,
    active_plan: Option<&MeasurementPlan>,
    current: Option<&GraphQualityHistoryRow>,
) -> PortfolioRepositoryReport {
    let mut sanitized = portfolio.clone();
    let Some(measurements) = sanitized.measurements.as_mut() else {
        return sanitized;
    };
    for row in &mut measurements.current {
        if row.metric != GRAPH_QUALITY_METRIC {
            continue;
        }
        if !accepted(row, active_plan, current) {
            row.observation = None;
            row.collection = None;
        }
    }
    sanitized
}

/// Whether the inherited current row is the one this view accepted.
/// `graph-portfolio.ts:546-551`.
fn accepted(
    row: &CurrentRow,
    active_plan: Option<&MeasurementPlan>,
    current: Option<&GraphQualityHistoryRow>,
) -> bool {
    let (Some(active_plan), Some(current)) = (active_plan, current) else {
        return false;
    };
    let same_collection = row
        .collection
        .as_ref()
        .is_some_and(|collection| collection.collection_id == current.id);
    let same_plan = row.observation.as_ref().is_some_and(|observation| {
        observation.plan_id.as_str() == active_plan.id.as_str()
            && observation.definition_version.as_str() == active_plan.definition_version.as_str()
    });
    same_collection && same_plan
}

/// `normalizeGraph` (`graph-portfolio.ts:492-518` in the retained numbering:
/// the `changeImpact` default is the whole of it).
fn normalize_graph(graph: &InjectedStructuralGraph) -> NormalizedStructuralGraph {
    match *graph {
        InjectedStructuralGraph::Available {
            ref path,
            ref premises,
            ref fan_out,
            ref churn,
            ref change_impact,
        } => NormalizedStructuralGraph::Available {
            path: path.clone(),
            premises: premises.clone(),
            fan_out: fan_out.clone(),
            churn: churn.clone(),
            change_impact: change_impact.clone().map_or(
                ChangeImpact::NotApplicable(ChangeImpactAbsence::NoSeedRequested),
                ChangeImpact::Analyses,
            ),
        },
        InjectedStructuralGraph::Unavailable {
            availability,
            ref path,
            ref reason,
        } => NormalizedStructuralGraph::Unavailable {
            availability,
            path: path.clone(),
            reason: reason.clone(),
        },
    }
}
