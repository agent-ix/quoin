// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The portfolio report as canonical JSON.
//!
//! `renderPortfolioReportJson` (`portfolio.ts:219-221`) is `canonicalJson`
//! over the whole portfolio object, so every member of `portfolio.ts:29-70` is
//! stated here — including the three that are absent rather than `null`:
//! a collection reference's `corpusRevision` (`portfolio.ts:327-329`), and a
//! plan's `owner` and `action` (`plans.ts:77-80`).

use serde::Serialize;

use crate::error::MeasurementError;
use crate::portfolio::ranking::{
    PortfolioRanking, PortfolioRankingEntry, RANKING_ADVISORY_NOTE, UnrankedPlan, rank_portfolio,
};
use crate::portfolio::types::{
    PORTFOLIO_STALE_AFTER_DAYS, PortfolioCollectionRef, PortfolioComparison, PortfolioReport,
    PortfolioRepositoryReport,
};
use crate::report::wire::{ComparisonWire, MeasurementReportWire, PlanWire, canonical_json_of};

/// Render the portfolio as the store's canonical JSON, trailing newline
/// included.
///
/// # Errors
///
/// As [`crate::report::render_measurement_report_json`].
pub fn render_portfolio_report_json(report: &PortfolioReport) -> Result<String, MeasurementError> {
    let ranking = rank_portfolio(report)?;
    canonical_json_of(&PortfolioReportWire::of(report, &ranking)?)
}

/// `portfolio.ts:65-70`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PortfolioReportWire<'a> {
    schema_version: i64,
    /// `| null`: stated as `null` when no repository has a collection.
    newest_collection_timestamp: Option<&'a str>,
    stale_after_days: i64,
    repositories: Vec<RepositoryWire<'a>>,
    /// PLAT-968 (FR-113): the advisory priority ranking. Never absent — a
    /// reader who parses the JSON without reading the prose still gets
    /// `advisory` stated next to `ranked` and `unranked`.
    ranking: RankingWire<'a>,
}

impl<'a> PortfolioReportWire<'a> {
    /// The wire view of one portfolio. `ranking` is computed by the caller so
    /// it outlives this borrow — see [`render_portfolio_report_json`].
    fn of(
        report: &'a PortfolioReport,
        ranking: &'a PortfolioRanking,
    ) -> Result<Self, MeasurementError> {
        Ok(Self {
            schema_version: PortfolioReport::SCHEMA_VERSION,
            newest_collection_timestamp: report.newest_collection_timestamp.as_deref(),
            stale_after_days: PORTFOLIO_STALE_AFTER_DAYS,
            repositories: report
                .repositories
                .iter()
                .map(RepositoryWire::of)
                .collect::<Result<Vec<_>, MeasurementError>>()?,
            ranking: RankingWire::of(ranking),
        })
    }
}

/// The wire view of [`PortfolioRanking`] (PLAT-968, FR-113).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RankingWire<'a> {
    advisory: &'static str,
    ranked: Vec<RankingEntryWire<'a>>,
    unranked: Vec<UnrankedPlanWire<'a>>,
}

impl<'a> RankingWire<'a> {
    fn of(ranking: &'a PortfolioRanking) -> Self {
        Self {
            advisory: RANKING_ADVISORY_NOTE,
            ranked: ranking.ranked.iter().map(RankingEntryWire::of).collect(),
            unranked: ranking.unranked.iter().map(UnrankedPlanWire::of).collect(),
        }
    }
}

/// One scored row of `ranking.ranked`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RankingEntryWire<'a> {
    repository: &'a str,
    plan_id: &'a str,
    plan_path: &'a str,
    metric: &'a str,
    /// `metric` with the row's dimension slice, when it has one — the
    /// distinguishing label a multi-slice plan needs (PLAT-1019). See
    /// [`crate::portfolio::ranking::PortfolioRankingEntry::label`].
    label: &'a str,
    current: f64,
    bound: f64,
    direction: &'static str,
    gap: f64,
    weight: f64,
    /// `| null`.
    value_half_life: Option<f64>,
    /// `| null`.
    age_days: Option<i64>,
    decay_factor: f64,
    /// `| null`.
    budget: Option<f64>,
    score: f64,
}

impl<'a> RankingEntryWire<'a> {
    fn of(entry: &'a PortfolioRankingEntry) -> Self {
        Self {
            repository: &entry.repository,
            plan_id: &entry.plan_id,
            plan_path: &entry.plan_path,
            metric: &entry.metric,
            label: &entry.label,
            current: entry.current,
            bound: entry.bound,
            direction: entry.direction.wire_name(),
            gap: entry.gap,
            weight: entry.weight,
            value_half_life: entry.value_half_life,
            age_days: entry.age_days,
            decay_factor: entry.decay_factor,
            budget: entry.budget,
            score: entry.score,
        }
    }
}

/// One row of `ranking.unranked`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UnrankedPlanWire<'a> {
    repository: &'a str,
    plan_id: &'a str,
    plan_path: &'a str,
    metric: &'a str,
    /// As [`RankingEntryWire::label`] (PLAT-1019).
    label: &'a str,
    reason: &'static str,
}

impl<'a> UnrankedPlanWire<'a> {
    fn of(plan: &'a UnrankedPlan) -> Self {
        Self {
            repository: &plan.repository,
            plan_id: &plan.plan_id,
            plan_path: &plan.plan_path,
            metric: &plan.metric,
            label: &plan.label,
            reason: plan.reason.as_str(),
        }
    }
}

/// `portfolio.ts:46-63`.
///
/// `pub` because the governed graph portfolio (`quoin-measurement-graph`,
/// quoin#476) **spreads this object** — `graph-portfolio.ts:452-457` returns
/// `{ ...portfolio, graphQuality, graph, gaps }` — so its wire view flattens
/// this one rather than transcribing eleven members a second time.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryWire<'a> {
    name: &'a str,
    root: &'a str,
    status: &'static str,
    /// `| null`: stated as `null` on a readable repository.
    error: Option<&'a str>,
    store: &'static str,
    profiles: Vec<ProfileWire<'a>>,
    plans: Vec<PlanWire<'a>>,
    /// `| null`.
    measurements: Option<MeasurementReportWire<'a>>,
    /// `| null`.
    latest_collection: Option<CollectionRefWire<'a>>,
    /// `| null`.
    comparison: Option<ComparisonBlockWire<'a>>,
    staleness: StalenessWire<'a>,
}

impl<'a> RepositoryWire<'a> {
    /// The wire view of one repository entry.
    ///
    /// # Errors
    ///
    /// As [`crate::report::render_measurement_report_json`].
    pub fn of(repository: &'a PortfolioRepositoryReport) -> Result<Self, MeasurementError> {
        Ok(Self {
            name: &repository.name,
            root: &repository.root,
            status: repository.status.as_str(),
            error: repository.status.error(),
            store: repository.store.as_str(),
            profiles: repository.profiles.iter().map(ProfileWire::of).collect(),
            plans: repository.plans.iter().map(PlanWire::of).collect(),
            measurements: repository
                .measurements
                .as_ref()
                .map(MeasurementReportWire::of)
                .transpose()?,
            latest_collection: repository
                .latest_collection
                .as_ref()
                .map(CollectionRefWire::of),
            comparison: repository
                .comparison
                .as_ref()
                .map(ComparisonBlockWire::of)
                .transpose()?,
            staleness: StalenessWire {
                status: repository.staleness.as_str(),
                age_days: repository.staleness.age_days(),
                relative_to: repository.staleness.relative_to(),
                threshold_days: PORTFOLIO_STALE_AFTER_DAYS,
            },
        })
    }
}

/// `profiles.ts:6-11`.
#[derive(Debug, Serialize)]
struct ProfileWire<'a> {
    id: &'a str,
    title: &'a str,
    status: &'static str,
    path: &'a str,
}

impl<'a> ProfileWire<'a> {
    /// The wire view of one profile summary.
    fn of(profile: &'a crate::types::profile::AssuranceProfileSummary) -> Self {
        Self {
            id: profile.id.as_str(),
            title: profile.title.as_str(),
            status: profile.status.as_str(),
            path: &profile.path,
        }
    }
}

/// `portfolio.ts:29-38`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionRefWire<'a> {
    id: &'a str,
    path: &'a str,
    timestamp: &'a str,
    source_revision: &'a str,
    tool_identity: &'a str,
    tool_version: &'a str,
    config_digest: &'a str,
    /// Spread in only when truthy, so absent rather than `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    corpus_revision: Option<&'a str>,
    /// Absent when there is nothing unverified (PLAT-969), matching
    /// `corpus_revision` above rather than an empty array.
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    unverified_artifacts: &'a [String],
}

impl<'a> CollectionRefWire<'a> {
    /// The wire view of one collection reference.
    fn of(reference: &'a PortfolioCollectionRef) -> Self {
        Self {
            id: &reference.id,
            path: &reference.path,
            timestamp: &reference.timestamp,
            source_revision: &reference.source_revision,
            tool_identity: &reference.tool_identity,
            tool_version: &reference.tool_version,
            config_digest: &reference.config_digest,
            corpus_revision: reference.corpus_revision.as_deref(),
            unverified_artifacts: &reference.unverified_artifacts,
        }
    }
}

/// `portfolio.ts:40-44`.
#[derive(Debug, Serialize)]
struct ComparisonBlockWire<'a> {
    before: CollectionRefWire<'a>,
    after: CollectionRefWire<'a>,
    observations: Vec<ComparisonWire<'a>>,
}

impl<'a> ComparisonBlockWire<'a> {
    /// The wire view of one repository's comparison.
    fn of(comparison: &'a PortfolioComparison) -> Result<Self, MeasurementError> {
        Ok(Self {
            before: CollectionRefWire::of(&comparison.before),
            after: CollectionRefWire::of(&comparison.after),
            observations: ComparisonWire::all(&comparison.observations)?,
        })
    }
}

/// `portfolio.ts:57-62`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StalenessWire<'a> {
    status: &'static str,
    /// `| null`.
    age_days: Option<i64>,
    /// `| null`.
    relative_to: Option<&'a str>,
    threshold_days: i64,
}
