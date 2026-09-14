// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The multi-store reports: the portfolio, and the governed graph portfolio
//! read over it.
//!
//! Four routes over several stores. The graph pair differs from the plain pair
//! only in the mappings it accepts, so the mappings are bounded and resolved in
//! one place rather than once per graph route.

use quoin_measurement::{
    build_portfolio_report, render_portfolio_report, render_portfolio_report_json,
};
use quoin_measurement_graph::{
    GovernedGraphPortfolioReport, GraphPortfolioMappingOptions, build_governed_graph_portfolio,
    canonical_graph_portfolio_json, render_governed_graph_portfolio,
};

use crate::error::CoreError;
use crate::protocol::Response;

use super::taxonomy::{map_graph, map_measurement};
use super::wire::{
    GraphPortfolioRequest, MAX_GRAPH_MAPPING_BYTES, MAX_PORTFOLIO_ROOTS_BYTES, PortfolioRequest,
};
use super::{bound, bound_field, document, parse, rendered, roots};
use crate::ops::request_size;

/// The four mapping members, named once.
///
/// The bound is applied to each separately, so a refusal names the list that
/// was too big. A loop over this array rather than four copies of the same
/// three lines: four copies is where the fourth gets a different constant.
const MAPPINGS: [&str; 4] = ["graph_exports", "graph_premises", "graph_audits", "changed"];

/// Answer a `measurement.build_portfolio`.
///
/// # Errors
///
/// - [`crate::error::CoreErrorCode::BadRequest`] when stdin is not a
///   [`PortfolioRequest`].
/// - [`crate::error::CoreErrorCode::Refused`] when the ceiling is exceeded, or
///   when a store cannot be read.
pub fn build_portfolio(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "measurement.build_portfolio";
    let report = build_portfolio_report(&locate(request, OP)?);
    let canonical = render_portfolio_report_json(&report).map_err(|e| map_measurement(&e, OP))?;
    document(&canonical, OP)
}

/// Answer a `measurement.render_portfolio`.
///
/// # Errors
///
/// As [`build_portfolio`].
pub fn render_portfolio(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "measurement.render_portfolio";
    let report = build_portfolio_report(&locate(request, OP)?);
    let markdown = render_portfolio_report(&report).map_err(|e| map_measurement(&e, OP))?;
    rendered(markdown, OP)
}

/// Answer a `measurement.build_graph_portfolio`.
///
/// # Errors
///
/// - [`crate::error::CoreErrorCode::BadRequest`] when stdin is not a
///   [`GraphPortfolioRequest`], or when a `<repository>=<value>` mapping is
///   malformed or names a repository outside the portfolio.
/// - [`crate::error::CoreErrorCode::Refused`] when a ceiling is exceeded, or
///   when a store or a mapped document cannot be read.
pub fn build_graph_portfolio(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "measurement.build_graph_portfolio";
    let report = governed(request, OP)?;
    let canonical = canonical_graph_portfolio_json(&report).map_err(|e| map_graph(&e, OP))?;
    document(&canonical, OP)
}

/// Answer a `measurement.render_graph_portfolio`.
///
/// # Errors
///
/// As [`build_graph_portfolio`].
pub fn render_graph_portfolio(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "measurement.render_graph_portfolio";
    let report = governed(request, OP)?;
    let markdown = render_governed_graph_portfolio(&report).map_err(|e| map_graph(&e, OP))?;
    rendered(markdown, OP)
}

/// Bound and parse a request that names several stores.
fn locate(
    request: &serde_json::Value,
    op: &'static str,
) -> Result<Vec<std::path::PathBuf>, CoreError> {
    bound(request, op, MAX_PORTFOLIO_ROOTS_BYTES)?;
    let request: PortfolioRequest = parse(request, op)?;
    Ok(roots(&request.locations))
}

/// Bound the request and its four mapping lists, then build the governed
/// portfolio.
///
/// Every ceiling is applied before `quoin-measurement-graph` opens anything.
fn governed(
    request: &serde_json::Value,
    op: &'static str,
) -> Result<GovernedGraphPortfolioReport, CoreError> {
    bound(request, op, MAX_PORTFOLIO_ROOTS_BYTES)?;
    for field in MAPPINGS {
        if let Some(list) = request.get(field) {
            bound_field(op, field, request_size(list)?, MAX_GRAPH_MAPPING_BYTES)?;
        }
    }

    let request: GraphPortfolioRequest = parse(request, op)?;
    let options = GraphPortfolioMappingOptions {
        graph_exports: request.graph_exports,
        graph_premises: request.graph_premises,
        graph_audits: request.graph_audits,
        changed: request.changed,
        cwd: request.cwd.map(std::path::PathBuf::from),
    };
    build_governed_graph_portfolio(&roots(&request.locations), &options)
        .map_err(|e| map_graph(&e, op))
}
