// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Markdown rendering for the evidence-backed test matrix.

use std::fmt::Write as _;

use super::{EvidenceDetail, MatrixCriterion, MatrixOutput, MatrixRequirement, UNASSIGNED};

/// Render the matrix as markdown.
///
/// One `##` section per requirement, in the payload's order, each holding one
/// table. An empty matrix renders its reason, never an empty table.
#[must_use]
pub fn render_markdown(matrix: &MatrixOutput) -> String {
    let mut out = String::from("# Test Matrix\n");
    if let Some(reason) = &matrix.reason {
        let _ = write!(out, "\n{reason}\n");
        if matrix.requirements.is_empty() {
            return out;
        }
    }
    let classed = matrix.unclassified.is_some();
    let designed = matrix
        .requirements
        .iter()
        .any(|requirement| requirement.design.is_some());
    for requirement in &matrix.requirements {
        let _ = write!(out, "\n## {}\n", cell(&requirement.id));
        for block in blocks_with_state(requirement.design.as_ref(), "uncovered") {
            if let Some(check) = block.get("check").and_then(serde_json::Value::as_str) {
                let _ = writeln!(out, "\nDesign gap: no design element ({check})");
            }
        }
        if requirement.criteria.is_empty() {
            continue;
        }
        let _ = write!(
            out,
            "\n| Requirement | Criterion | {}Method | Static Status | {}Evidence Status | Detail |\n|---|---|{}---|---|{}---|---|\n",
            if designed { "Design | " } else { "" },
            if classed { "Class | " } else { "" },
            if designed { "---|" } else { "" },
            if classed { "---|" } else { "" },
        );
        for criterion in &requirement.criteria {
            let class = if classed {
                format!(
                    "{} | ",
                    cell(criterion.class.as_deref().unwrap_or(UNASSIGNED))
                )
            } else {
                String::new()
            };
            let detail = detail_text(&criterion.evidence_detail);
            let analysis = criterion
                .analysis_evidence
                .as_ref()
                .into_iter()
                .flatten()
                .map(|item| format!("analysis {item}"))
                .collect::<Vec<_>>()
                .join("; ");
            let detail = if analysis.is_empty() {
                detail
            } else {
                format!("{detail}; {analysis}")
            };
            let design = if designed {
                format!("{} | ", design_cell(requirement, criterion))
            } else {
                String::new()
            };
            let _ = writeln!(
                out,
                "| {} | {} | {}{} | {} | {}{} | {} |",
                cell(&requirement.id),
                cell(&criterion.id),
                design,
                cell(criterion.method.as_deref().unwrap_or("—")),
                cell(&criterion.static_status),
                class,
                criterion.evidence_status.as_str(),
                cell(&detail),
            );
        }
    }
    if let Some(count) = matrix.unclassified {
        let _ = write!(
            out,
            "\n{count} criterion(s) state no software safety class.\n"
        );
    }
    out
}

fn blocks_with_state<'a>(
    design: Option<&'a Vec<serde_json::Value>>,
    state: &str,
) -> impl Iterator<Item = &'a serde_json::Value> {
    design
        .into_iter()
        .flatten()
        .filter(move |block| block.get("state").and_then(serde_json::Value::as_str) == Some(state))
}

fn design_cell(requirement: &MatrixRequirement, criterion: &MatrixCriterion) -> String {
    let Some(blocks) = &requirement.design else {
        return "n/a".to_owned();
    };
    let mut parts = Vec::new();
    for reference in criterion.design_elements.iter().flatten() {
        let id = reference
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let repo = reference.get("repo").and_then(serde_json::Value::as_str);
        let element = blocks
            .iter()
            .flat_map(|block| {
                block
                    .get("elements")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
            })
            .find(|element| {
                element.get("id").and_then(serde_json::Value::as_str) == Some(id)
                    && element.get("repo").and_then(serde_json::Value::as_str) == repo
            });
        let kind = element
            .and_then(|element| element.get("kind"))
            .and_then(serde_json::Value::as_str);
        let qualified_id = repo.map_or_else(|| id.to_owned(), |repo| format!("{repo}/{id}"));
        let label = kind.map_or_else(
            || qualified_id.clone(),
            |kind| format!("{qualified_id} ({kind})"),
        );
        parts.push(label);
    }
    for block in blocks
        .iter()
        .filter(|block| block.get("state").and_then(serde_json::Value::as_str) == Some("uncovered"))
    {
        if let Some(check) = block.get("check").and_then(serde_json::Value::as_str) {
            parts.push(format!("no design element ({check})"));
        }
    }
    for block in blocks.iter().filter(|block| {
        block.get("state").and_then(serde_json::Value::as_str) == Some("covered")
            && block
                .get("elements")
                .and_then(serde_json::Value::as_array)
                .is_some_and(Vec::is_empty)
    }) {
        for dangling in block
            .get("dangling")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(reference) = dangling
                .get("reference")
                .and_then(serde_json::Value::as_str)
            {
                parts.push(format!("-> {reference} (dangling)"));
            }
        }
    }
    for block in blocks {
        for mismatch in block
            .get("kind_mismatch")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let id = mismatch
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let repo = mismatch.get("repo").and_then(serde_json::Value::as_str);
            let id = repo.map_or_else(|| id.to_owned(), |repo| format!("{repo}/{id}"));
            let archetype = mismatch
                .get("archetype")
                .and_then(serde_json::Value::as_str);
            parts.push(archetype.map_or_else(
                || format!("kind mismatch: {id}"),
                |kind| format!("kind mismatch: {id} ({kind})"),
            ));
        }
    }
    if parts.is_empty() {
        "(none)".to_owned()
    } else {
        parts.join(", ")
    }
}

/// The detail column: findings, then bindings, then unevaluated checks.
fn detail_text(detail: &EvidenceDetail) -> String {
    let mut parts: Vec<String> = detail
        .findings
        .iter()
        .map(|finding| format!("{}: {}", finding.kind, finding.summary))
        .collect();
    parts.extend(
        detail
            .bindings
            .iter()
            .map(|binding| format!("bound by {}", binding.suite)),
    );
    parts.extend(
        detail
            .unevaluated
            .iter()
            .map(|check| format!("not evaluated: {check}")),
    );
    if parts.is_empty() {
        "—".to_owned()
    } else {
        parts.join("; ")
    }
}

/// One table cell: a `|` would end the cell and a newline the row.
fn cell(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace(['\r', '\n'], " ")
}
