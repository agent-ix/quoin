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
    canonical_json_of(&PortfolioReportWire::of(report)?)
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
}

impl<'a> PortfolioReportWire<'a> {
    /// The wire view of one portfolio.
    fn of(report: &'a PortfolioReport) -> Result<Self, MeasurementError> {
        Ok(Self {
            schema_version: PortfolioReport::SCHEMA_VERSION,
            newest_collection_timestamp: report.newest_collection_timestamp.as_deref(),
            stale_after_days: PORTFOLIO_STALE_AFTER_DAYS,
            repositories: report
                .repositories
                .iter()
                .map(RepositoryWire::of)
                .collect::<Result<Vec<_>, MeasurementError>>()?,
        })
    }
}

/// `portfolio.ts:46-63`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryWire<'a> {
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
    fn of(repository: &'a PortfolioRepositoryReport) -> Result<Self, MeasurementError> {
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
