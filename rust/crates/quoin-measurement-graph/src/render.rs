// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The governed graph portfolio as Markdown.
//!
//! Ports `renderGovernedGraphPortfolio` (`graph-portfolio.ts:331-408`).
//!
//! # It renders the ungoverned report first, and then says what it refuses
//!
//! The first section is [`quoin_measurement::portfolio::render_portfolio_report`]
//! verbatim — not a second rendering of the same data — and the governed
//! section that follows opens by stating that no partition or repository is
//! aggregated and no quality verdict is derived. That sentence is in the
//! output because a reader of a governed report will otherwise supply a
//! verdict of their own.

use quoin_measurement::common::scalar::js_f64_string;
use quoin_measurement::portfolio::{PortfolioReport, render_portfolio_report};
use quoin_measurement::types::observation::MeasurementState;

use crate::canonical::stored_pretty_text_trimmed;
use crate::error::GraphAdapterError;
use crate::input::NormalizedStructuralGraph;
use crate::reading::{GraphPartitionRow, GraphQualityHistoryRow};
use crate::render_json::change_impact_value;
use crate::report::{GovernedGraphPortfolioReport, GovernedGraphRepositoryReport};
use crate::text::{inline_text, markdown_cell};

/// What a digest member renders as when the collection stated none.
/// `graph-portfolio.ts:353-354`.
const UNKNOWN_DIGEST: &str = "unknown";

/// What a not-computed partition renders as when it gave no reason.
/// `graph-portfolio.ts:363`.
const UNKNOWN_REASON: &str = "unknown";

/// What a row with no comparable delta renders as. `graph-portfolio.ts:382`.
const NO_DELTA: &str = "not_computed";

/// What a comparison row with no blocking reason renders as.
/// `graph-portfolio.ts:383`.
const COMPATIBLE: &str = "compatible";

/// Render the governed graph portfolio.
///
/// `renderGovernedGraphPortfolio` (`graph-portfolio.ts:331-408`).
///
/// # Errors
///
/// [`crate::error::GraphAdapterErrorCode::Store`] for an opaque graph analysis no canonical
/// JSON writer can spell, and [`crate::error::GraphAdapterErrorCode::Measurement`] for
/// whatever the ungoverned renderer refuses.
pub fn render_governed_graph_portfolio(
    report: &GovernedGraphPortfolioReport,
) -> Result<String, GraphAdapterError> {
    let inherited = PortfolioReport {
        newest_collection_timestamp: report.newest_collection_timestamp.clone(),
        repositories: report
            .repositories
            .iter()
            .map(|repository| repository.base.clone())
            .collect(),
    };
    let mut lines: Vec<String> = vec![
        render_portfolio_report(&inherited)?.trim_end().to_owned(),
        String::new(),
        "# Governed graph evidence".to_owned(),
        String::new(),
        "No partition or repository is aggregated and no quality verdict is derived.".to_owned(),
        String::new(),
    ];
    for repository in &report.repositories {
        render_repository(&mut lines, repository)?;
    }
    Ok(lines.join("\n"))
}

/// One repository's section. `graph-portfolio.ts:341-406`.
fn render_repository(
    lines: &mut Vec<String>,
    repository: &GovernedGraphRepositoryReport,
) -> Result<(), GraphAdapterError> {
    lines.push(format!("## {}", repository.base.name));
    lines.push(String::new());
    lines.push(format!("Root: {}", repository.base.root));
    lines.push(format!("Graph export: {}", repository.graph.availability()));
    match repository.graph_quality.current {
        None => lines.push("Graph quality: not_computed".to_owned()),
        Some(ref current) => render_current(lines, current),
    }
    render_history(lines, &repository.graph_quality.history);
    render_comparison(lines, repository);
    render_graph(lines, &repository.graph)?;
    render_gaps(lines, repository);
    lines.push(String::new());
    Ok(())
}

/// The current reading and its partition table. `graph-portfolio.ts:346-366`.
fn render_current(lines: &mut Vec<String>, current: &GraphQualityHistoryRow) {
    lines.push(format!(
        "Graph quality: {}; {}; producer {}; scorer {}",
        current.id,
        current.availability.as_str(),
        current
            .producer_record_digest
            .as_deref()
            .unwrap_or(UNKNOWN_DIGEST),
        current.scorer_digest.as_deref().unwrap_or(UNKNOWN_DIGEST),
    ));
    lines.push(String::new());
    lines.push("| Measure | Dimension | Key | State/value |".to_owned());
    lines.push("| --- | --- | --- | --- |".to_owned());
    for row in &current.partitions {
        lines.push(format!(
            "| {} | {} | {} | {} |",
            markdown_cell(&row.identity.measure),
            markdown_cell(&row.identity.dimension),
            markdown_cell(&row.identity.key),
            state_and_value(row),
        ));
    }
}

/// The retained ternary on `row.state` (`measured` renders
/// `${row.value} ${row.unit}`)
/// (`graph-portfolio.ts:362-364`).
fn state_and_value(row: &GraphPartitionRow) -> String {
    match row.state {
        MeasurementState::Measured => format!("{} {}", number(row.value), row.unit),
        MeasurementState::NotComputed => format!(
            "not_computed: {}",
            row.reason.as_deref().unwrap_or(UNKNOWN_REASON)
        ),
    }
}

/// `${value}` for a `number | null`, which is JavaScript `String(value)`.
fn number(value: Option<f64>) -> String {
    value.map_or_else(|| "null".to_owned(), js_f64_string)
}

/// Every reading, oldest first. `graph-portfolio.ts:367-381`.
fn render_history(lines: &mut Vec<String>, history: &[GraphQualityHistoryRow]) {
    if history.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("History:".to_owned());
    for row in history {
        lines.push(format!(
            "- {} {}: {}; producer {}; scorer {}",
            row.timestamp,
            row.id,
            row.availability.as_str(),
            row.producer_record_digest
                .as_deref()
                .unwrap_or(UNKNOWN_DIGEST),
            row.scorer_digest.as_deref().unwrap_or(UNKNOWN_DIGEST),
        ));
        for partition in &row.partitions {
            lines.push(format!(
                "  - {}/{}/{}: {}",
                inline_text(&partition.identity.measure),
                inline_text(&partition.identity.dimension),
                inline_text(&partition.identity.key),
                state_and_value(partition),
            ));
        }
    }
}

/// The two newest readings compared. `graph-portfolio.ts:382-395`.
fn render_comparison(lines: &mut Vec<String>, repository: &GovernedGraphRepositoryReport) {
    let Some(ref comparison) = repository.graph_quality.comparison else {
        return;
    };
    lines.push(String::new());
    lines.push(format!(
        "Graph comparison: {} -> {}",
        comparison.before.id, comparison.after.id
    ));
    for row in &comparison.observations {
        let codes = row
            .reasons
            .iter()
            .map(|reason| reason.code.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "- {}/{}/{}: {}; delta {}; {}",
            inline_text(&row.identity.measure),
            inline_text(&row.identity.dimension),
            inline_text(&row.identity.key),
            row.status.as_str(),
            row.delta.map_or_else(|| NO_DELTA.to_owned(), js_f64_string),
            // `join(", ") || "compatible"`: the empty string is falsy.
            if codes.is_empty() { COMPATIBLE } else { &codes },
        ));
    }
}

/// The three opaque analyses, as canonical JSON. `graph-portfolio.ts:396-403`.
fn render_graph(
    lines: &mut Vec<String>,
    graph: &NormalizedStructuralGraph,
) -> Result<(), GraphAdapterError> {
    let NormalizedStructuralGraph::Available {
        ref fan_out,
        ref churn,
        ref change_impact,
        ..
    } = *graph
    else {
        return Ok(());
    };
    lines.push(String::new());
    lines.push(format!("Fan-out: {}", stored_pretty_text_trimmed(fan_out)?));
    lines.push(format!("Churn: {}", stored_pretty_text_trimmed(churn)?));
    lines.push(format!(
        "Change impact: {}",
        stored_pretty_text_trimmed(&change_impact_value(change_impact))?
    ));
    Ok(())
}

/// What the governed view needed and did not have.
/// `graph-portfolio.ts:404-410`.
fn render_gaps(lines: &mut Vec<String>, repository: &GovernedGraphRepositoryReport) {
    if repository.gaps.is_empty() {
        return;
    }
    lines.push(String::new());
    lines.push("Gaps:".to_owned());
    for gap in &repository.gaps {
        lines.push(format!(
            "- {}: {} \u{2014} {}",
            gap.availability.as_str(),
            gap.path.as_deref().unwrap_or_else(|| gap.subject.as_str()),
            gap.reason,
        ));
    }
}
