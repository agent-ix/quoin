// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The two projections of one report: canonical JSON, and markdown for a
//! human (`render.ts`).
//!
//! # The JSON half is one call
//!
//! `renderGraphAnalysisJson` is `canonicalJson` and nothing else
//! (`render.ts:11`). Here it is `quoin_store::canonical_json`, reached
//! through `canonical_text`. There is no ordering, no
//! indentation and no number formatting in this file, and
//! `tests/tc_385_one_canonical_json.rs` asserts there is not.
//!
//! # The markdown half escapes twice, differently
//!
//! A table cell escapes `|` and flattens newlines ([`cell`]); a code span
//! escapes the backtick ([`escape_code`]). They are not interchangeable and
//! the retained source applies each in specific places — a suite name goes
//! through `cell`, a repository name through `escape_code` — so both are here
//! and neither is generalised into one "escape" that would silently change
//! which characters survive where.

use crate::error::Result;
use crate::model::report::{
    ChangeImpactAnalysis, ChurnAnalysis, FanOutAnalysis, GraphAnalysis, GraphReportBase,
};

/// The report as canonical JSON: 2-space indent, ordered members, trailing
/// newline (`render.ts:11`).
///
/// # Errors
///
/// [`crate::GraphError::Canonicalization`] when the report has no canonical
/// spelling.
pub fn render_graph_analysis_json(analysis: &GraphAnalysis) -> Result<String> {
    crate::json::canonical_text("a graph analysis report", analysis)
}

/// The report as markdown (`render.ts:15`).
#[must_use]
pub fn render_graph_analysis(analysis: &GraphAnalysis) -> String {
    let base = analysis.base();
    let mut lines: Vec<String> = vec![
        format!("# Evidence graph: {}", base.view.as_str()),
        String::new(),
        format!("State: **{}**", base.state.as_str()),
        format!(
            "Source: `{}@{}`",
            escape_code(&base.source.repository),
            base.source.revision
        ),
        format!(
            "Export: `{} v{}`",
            base.export.format, base.export.format_version
        ),
        String::new(),
        "## Accepted module premises".to_owned(),
        String::new(),
    ];
    render_premises(base, &mut lines);
    match analysis {
        GraphAnalysis::FanOut(report) => render_fan_out(report, &mut lines),
        GraphAnalysis::ChangeImpact(report) => render_impact(report, &mut lines),
        GraphAnalysis::Churn(report) => render_churn(report, &mut lines),
    }
    render_gaps(base, &mut lines);
    format!("{}\n", lines.join("\n").trim_end())
}

fn render_premises(base: &GraphReportBase, lines: &mut Vec<String>) {
    if base.premises.modules.is_empty() {
        lines.push("*(none)*".to_owned());
    } else {
        for module in &base.premises.modules {
            lines.push(format!(
                "- `{}@{}`: {} schema(s)",
                escape_code(module.name.as_str()),
                escape_code(module.version.as_str()),
                module.schemas.len()
            ));
        }
    }
    lines.push(String::new());
}

fn render_fan_out(report: &FanOutAnalysis, lines: &mut Vec<String>) {
    lines.push("## Fan-out".to_owned());
    lines.push(String::new());
    lines.push("| Suite | Live obligations | Count | Unresolved |".to_owned());
    lines.push("| --- | --- | ---: | --- |".to_owned());
    if report.rows.is_empty() {
        lines.push("| *(none)* | — | 0 | — |".to_owned());
    } else {
        for row in &report.rows {
            let obligations = join(
                row.obligations.iter().map(|owned| {
                    let requirements = join(
                        owned.requirements.iter().map(|id| id.as_str().to_owned()),
                        ", ",
                    );
                    let requirements = if requirements.is_empty() {
                        "owner unknown".to_owned()
                    } else {
                        requirements
                    };
                    format!("{} ({requirements})", owned.obligation)
                }),
                ", ",
            );
            let unresolved = join(
                row.unresolved_bindings
                    .iter()
                    .map(|id| id.as_str().to_owned()),
                ", ",
            );
            lines.push(format!(
                "| {} | {} | {} | {} |",
                cell(row.suite.as_str()),
                or_dash(&cell(&obligations)),
                row.obligation_count,
                or_dash(&cell(&unresolved))
            ));
        }
    }
    lines.push(String::new());
}

fn render_impact(report: &ChangeImpactAnalysis, lines: &mut Vec<String>) {
    lines.push("## Change impact".to_owned());
    lines.push(String::new());
    lines.push(format!(
        "Requested: {}",
        or_none(&join(
            report.requested.iter().map(|id| code(id.as_str())),
            ", "
        ))
    ));
    lines.push(format!(
        "Relations: {}",
        or_none(&join(
            report.relation_kinds.iter().map(|kind| code(kind.as_str())),
            ", "
        ))
    ));
    lines.push(String::new());
    lines.push("| Requirement | Depth | Seed | Obligations |".to_owned());
    lines.push("| --- | ---: | --- | --- |".to_owned());
    if report.rows.is_empty() {
        lines.push("| *(none)* | — | — | — |".to_owned());
    } else {
        for row in &report.rows {
            let obligations = join(
                row.obligations
                    .iter()
                    .map(|owned| owned.obligation.as_str().to_owned()),
                ", ",
            );
            lines.push(format!(
                "| {} | {} | {} | {} |",
                cell(row.requirement.as_str()),
                row.depth,
                cell(row.path.seed.as_str()),
                or_dash(&cell(&obligations))
            ));
        }
    }
    lines.push(String::new());
}

fn render_churn(report: &ChurnAnalysis, lines: &mut Vec<String>) {
    lines.push("## Retained reaffirmation history".to_owned());
    lines.push(String::new());
    lines.push("| Obligation | Requirements | Suites | Events |".to_owned());
    lines.push("| --- | --- | --- | ---: |".to_owned());
    if report.rows.is_empty() {
        lines.push("| *(none)* | — | — | 0 |".to_owned());
    } else {
        for row in &report.rows {
            let requirements = join(
                row.requirements.iter().map(|id| id.as_str().to_owned()),
                ", ",
            );
            let suites = join(row.suites.iter().map(|id| id.as_str().to_owned()), ", ");
            lines.push(format!(
                "| {} | {} | {} | {} |",
                cell(row.obligation.as_str()),
                or_dash(&cell(&requirements)),
                or_dash(&cell(&suites)),
                row.event_count
            ));
        }
    }
    lines.push(String::new());
}

fn render_gaps(base: &GraphReportBase, lines: &mut Vec<String>) {
    if base.gaps.is_empty() {
        return;
    }
    lines.push("## Gaps".to_owned());
    lines.push(String::new());
    for gap in &base.gaps {
        lines.push(format!(
            "- **{}** {}: {}",
            gap.kind.as_str(),
            code(&gap.subject),
            gap.reason
        ));
    }
    lines.push(String::new());
}

/// `[...].join(separator)`, for an iterator of already-rendered parts.
fn join(parts: impl Iterator<Item = String>, separator: &str) -> String {
    parts.collect::<Vec<_>>().join(separator)
}

/// The retained `|| "—"` on an empty cell (`render.ts:60`).
fn or_dash(value: &str) -> String {
    if value.is_empty() {
        "—".to_owned()
    } else {
        value.to_owned()
    }
}

/// The retained `|| "*(none)*"` on an empty list (`render.ts:71`).
fn or_none(value: &str) -> String {
    if value.is_empty() {
        "*(none)*".to_owned()
    } else {
        value.to_owned()
    }
}

/// A table cell: `|` escaped, newlines flattened (`render.ts:118`).
fn cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

/// A code span (`render.ts:122`).
fn code(value: &str) -> String {
    format!("`{}`", escape_code(value))
}

/// The backtick escape a code span needs (`render.ts:126`).
fn escape_code(value: &str) -> String {
    value.replace('`', "\\`")
}
