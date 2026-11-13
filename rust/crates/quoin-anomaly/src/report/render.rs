// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The Markdown form of the anomaly report.
//!
//! Every section is present and states `none` when empty. Nothing here words an
//! anomaly, a risk or a release as acceptable (NFR-035).

use std::fmt::Write as _;

use super::{Gate, Report, ReportRow};

const NOT_STATED: &str = "not stated";

/// A table cell: pipes escaped, line breaks flattened.
fn cell(value: &str) -> String {
    value.replace('|', "\\|").replace(['\r', '\n'], " ")
}

fn or_not_stated(value: Option<&str>) -> String {
    cell(value.unwrap_or(NOT_STATED))
}

fn joined(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join("<br>")
    }
}

fn assessment_cell(row: &ReportRow) -> String {
    joined(
        &row.assessment
            .iter()
            .map(|entry| {
                cell(&format!(
                    "{} {} {}: {}",
                    entry.kind,
                    entry.target.as_deref().unwrap_or(NOT_STATED),
                    entry.impact,
                    entry.rationale,
                ))
            })
            .collect::<Vec<_>>(),
    )
}

fn affected_cell(row: &ReportRow) -> String {
    joined(
        &row.affected
            .iter()
            .map(|affected| {
                cell(&format!(
                    "{} {} ({})",
                    affected.kind, affected.target, affected.impact
                ))
            })
            .collect::<Vec<_>>(),
    )
}

fn rationale_cell(row: &ReportRow) -> String {
    joined(
        &row.release_rationale
            .iter()
            .map(|entry| cell(&format!("{}: {}", entry.author, entry.rationale)))
            .collect::<Vec<_>>(),
    )
}

fn rows_table(report: &Report, out: &mut String) {
    out.push_str("## Anomalies\n\n");
    if report.anomalies.is_empty() {
        out.push_str("none\n");
        return;
    }
    out.push_str(
        "| ID | Title | Path | Status | Resolution | Severity | Safety impact | Flag | Assessment | Affected | Release rationale | Fixing change | Rationale change |\n",
    );
    out.push_str("|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    for row in &report.anomalies {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            cell(&row.id),
            or_not_stated(row.title.as_deref()),
            cell(&row.path),
            cell(&row.status),
            or_not_stated(row.resolution.as_deref()),
            or_not_stated(row.severity.as_deref()),
            cell(&row.safety_impact),
            if row.unassessed { "unassessed" } else { "none" },
            assessment_cell(row),
            affected_cell(row),
            rationale_cell(row),
            cell(&row.fixing_change.markdown()),
            cell(&row.rationale_change.markdown()),
        );
    }
}

fn groups(title: &str, groups: &[super::Group], out: &mut String) {
    let _ = write!(out, "\n## {title}\n\n");
    if groups.is_empty() {
        out.push_str("none\n");
        return;
    }
    for group in groups {
        let _ = writeln!(out, "### {}\n", group.target);
        for member in &group.anomalies {
            let _ = writeln!(out, "- {} ({})", member.anomaly, member.impact);
        }
        out.push('\n');
    }
    // One blank line closes the section, as the empty form does.
    out.truncate(out.trim_end().len());
    out.push('\n');
}

fn gate_section(gate: &Gate, out: &mut String) {
    let modes = gate
        .modes
        .iter()
        .map(|mode| mode.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let _ = write!(out, "\n## Gate\n\nModes: {modes}\n\n");
    if gate.failures.is_empty() {
        let _ = writeln!(out, "No anomaly fails the evaluated modes: {modes}.");
        return;
    }
    for failure in &gate.failures {
        let _ = writeln!(
            out,
            "- {}: {} ({})",
            failure.mode.as_str(),
            failure.anomaly,
            failure.path,
        );
    }
}

pub(super) fn markdown(report: &Report) -> String {
    let mut out = String::from("# Anomaly report\n\n");
    rows_table(report, &mut out);
    groups("By hazard", &report.by_hazard, &mut out);
    groups("By requirement", &report.by_requirement, &mut out);
    out.push_str("\n## Ungrouped\n\n");
    if report.ungrouped.is_empty() {
        out.push_str("none\n");
    }
    for entry in &report.ungrouped {
        let _ = writeln!(out, "- {}: {}", entry.anomaly, entry.reason);
    }
    out.push_str("\n## Unreadable\n\n");
    if report.unreadable.is_empty() {
        out.push_str("none\n");
    }
    for entry in &report.unreadable {
        let _ = writeln!(out, "- {}: {}", entry.path, entry.code);
    }
    if let Some(gate) = &report.gate {
        gate_section(gate, &mut out);
    }
    out
}
