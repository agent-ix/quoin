// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The governed graph portfolio report itself.
//!
//! Ports `GovernedGraphRepositoryReport` and `GovernedGraphPortfolioReport`
//! (`graph-portfolio.ts:166-207`).
//!
//! # Why the portfolio entry is a member and not a flattened copy
//!
//! `GovernedGraphRepositoryReport extends PortfolioRepositoryReport`, and
//! `graph-portfolio.ts:452-457` builds one by spreading the portfolio entry it
//! was handed. So the entry is kept whole here, as [`base`]
//! (`GovernedGraphRepositoryReport::base`), and the JSON view flattens it —
//! which means a member added to the portfolio entry appears here without this
//! crate being edited, exactly as the spread does.

use quoin_measurement::portfolio::{PORTFOLIO_STALE_AFTER_DAYS, PortfolioRepositoryReport};
use quoin_measurement::types::plan::MeasurementPlan;

use crate::input::{GraphPortfolioGap, NormalizedStructuralGraph};
use crate::reading::{GraphQualityComparison, GraphQualityHistoryRow};

/// One repository's governed graph readings. `graph-portfolio.ts:167-172`.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphQuality {
    /// The active `graph_quality` plan, when there is one.
    pub plan: Option<MeasurementPlan>,
    /// The newest reading that may be used as current evidence.
    pub current: Option<GraphQualityHistoryRow>,
    /// Every reading that holds a graph partition, oldest first.
    pub history: Vec<GraphQualityHistoryRow>,
    /// The two newest readings compared, when there are two.
    pub comparison: Option<GraphQualityComparison>,
}

/// One repository's entry in the governed view. `graph-portfolio.ts:166-175`.
#[derive(Clone, Debug, PartialEq)]
pub struct GovernedGraphRepositoryReport {
    /// The ungoverned portfolio entry this extends, with its `graph_quality`
    /// current row sanitised (`graph-portfolio.ts:530-556`).
    pub base: PortfolioRepositoryReport,
    /// The governed graph readings.
    pub graph_quality: GraphQuality,
    /// The structural graph, with its change-impact member settled.
    pub graph: NormalizedStructuralGraph,
    /// What the governed view needed and did not have.
    pub gaps: Vec<GraphPortfolioGap>,
}

/// The whole governed view. `graph-portfolio.ts:202-207`.
#[derive(Clone, Debug, PartialEq)]
pub struct GovernedGraphPortfolioReport {
    /// The newest collection timestamp across every repository.
    pub newest_collection_timestamp: Option<String>,
    /// The repositories, in resolved-root order.
    pub repositories: Vec<GovernedGraphRepositoryReport>,
}

impl GovernedGraphPortfolioReport {
    /// The report's schema version. `graph-portfolio.ts:203,290`: always 1.
    pub const SCHEMA_VERSION: i64 = 1;

    /// The staleness threshold, which is the portfolio's own.
    ///
    /// `graph-portfolio.ts:292` re-exports `PORTFOLIO_STALE_AFTER_DAYS`; there
    /// is no second threshold here either.
    pub const STALE_AFTER_DAYS: i64 = PORTFOLIO_STALE_AFTER_DAYS;
}
