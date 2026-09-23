// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The cross-repository portfolio view (FR-045).
//!
//! Ports `src/measurement/portfolio.ts`.
//!
//! | retained TypeScript | here |
//! | --- | --- |
//! | `portfolio.ts:26-75` | [`types`] |
//! | `portfolio.ts:77-140`, `223-331` | [`build`] |
//! | `portfolio.ts:142-217`, `333-367` | [`render`] |
//! | `portfolio.ts:219-221` | [`render_json`] |
//! | `resolve`, `existsSync`, `statSync`, `basename` | [`location`] |
//!
//! # What this view refuses to do
//!
//! Nothing here sums or averages a metric across repositories into one
//! number, and the rendered report says so in its second line: no cross-
//! repository quality score, no policy verdict. [`ranking`] is the one
//! deliberate exception, and it is not that number — PLAT-968 ranks plans by
//! declared priority, it does not average their metrics, and it states in
//! both rendered forms that it decides nothing (see
//! [`ranking::RANKING_ADVISORY_NOTE`]).

pub mod build;
pub mod location;
pub mod ranking;
pub mod render;
pub mod render_json;
pub mod types;

pub use build::{build_portfolio_report, build_portfolio_report_from_collections};
pub use ranking::{
    PortfolioRanking, PortfolioRankingEntry, RANKING_ADVISORY_NOTE, UnrankedPlan, UnrankedReason,
    rank_portfolio,
};
pub use render::render_portfolio_report;
pub use render_json::render_portfolio_report_json;
pub use types::{
    PORTFOLIO_STALE_AFTER_DAYS, PortfolioCollectionRef, PortfolioCollectionSnapshot,
    PortfolioComparison, PortfolioReport, PortfolioRepositoryReport, RepositoryStatus, Staleness,
    StoreState,
};
