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
//! Nothing here sums, averages or ranks a metric across repositories, and the
//! rendered report says so in its second line. A portfolio is a list of what
//! each repository measured and how old that reading is; a number that mixed
//! two repositories' metrics would be a quality verdict this layer has no
//! evidence for.

pub mod build;
pub mod location;
pub mod render;
pub mod render_json;
pub mod types;

pub use build::{build_portfolio_report, build_portfolio_report_from_collections};
pub use render::render_portfolio_report;
pub use render_json::render_portfolio_report_json;
pub use types::{
    PORTFOLIO_STALE_AFTER_DAYS, PortfolioCollectionRef, PortfolioCollectionSnapshot,
    PortfolioComparison, PortfolioReport, PortfolioRepositoryReport, RepositoryStatus, Staleness,
    StoreState,
};
