// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The renderings of the SOUP report: Markdown and JSON from one [`Report`].
//!
//! Both print only what an input stated. A scanner's severity is its own word;
//! no key or line here scores, ranks or classifies.

use std::fmt::Write as _;

use serde_json::{Value, json};

use super::{Anomaly, Listed, Report, Row};
use crate::soup::advisory::{Attached, Resolution, ScanOutcome, ScanState, Tool, Unmatched};
use crate::soup::support::Resolved;

const NOT_STATED: &str = "not stated";

/// Text on one line, so a multi-line message cannot break the layout.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn anomaly_text(anomaly: &Anomaly) -> String {
    match &anomaly.source_ref {
        Some(source) => format!("{} ({source})", one_line(&anomaly.statement)),
        None => one_line(&anomaly.statement),
    }
}

/// `not stated`, `none` or the items.
fn listed_text<T>(listed: &Listed<T>, item: impl Fn(&T) -> String) -> String {
    match listed {
        Listed::Absent => NOT_STATED.to_owned(),
        Listed::Empty => "none".to_owned(),
        Listed::Items(items) => items.iter().map(item).collect::<Vec<_>>().join("; "),
    }
}

fn resolved_text<T: Copy>(resolved: &Resolved<T>, show: impl Fn(T) -> String) -> String {
    match resolved {
        Resolved::Unknown => "unknown".to_owned(),
        Resolved::Record(value) => show(*value),
        Resolved::Supplied { value, source } => {
            format!(
                "{} (support file, source: {})",
                show(*value),
                one_line(source)
            )
        }
        Resolved::Disagree {
            record,
            supplied,
            source,
        } => format!(
            "{} (record; DISAGREES with the support file: {}, source: {})",
            show(*record),
            show(*supplied),
            one_line(source)
        ),
    }
}

fn advisory_text(advisory: &Attached) -> String {
    let severity = advisory.severity.as_deref().unwrap_or(NOT_STATED);
    let outcome = match &advisory.resolution {
        Resolution::Resolved(disposition) => format!(
            "resolved: {} by {} ({}; {})",
            decision_word(disposition.decision),
            one_line(&disposition.attested_by),
            one_line(&disposition.approval_ref),
            one_line(&disposition.rationale)
        ),
        Resolution::Unresolved(reason) => format!("unresolved: {reason}"),
    };
    format!(
        "{} {} {} (severity: {severity}; scan {}): {outcome}",
        advisory.tool.name(),
        advisory.rule_id,
        advisory.package,
        advisory.scan_timestamp
    )
}

fn decision_word(decision: quoin_assurance::DispositionDecision) -> String {
    serde_json::to_value(decision)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn flags(row: &Row) -> String {
    let mut flags = Vec::new();
    if row.unresolved() {
        flags.push("unresolved advisory");
    }
    if row.support.past_end_of_support {
        flags.push("past end of support");
    }
    if row.support.support_status.disagrees() || row.support.end_of_support.disagrees() {
        flags.push("support disagreement");
    }
    if flags.is_empty() {
        "none".to_owned()
    } else {
        flags.join(", ")
    }
}

/// The row's standing for one tool: its advisories, or why it has none to show.
fn tool_lines(row: &Row, tool: Tool) -> Vec<String> {
    let found: Vec<String> = row
        .join
        .advisories
        .iter()
        .filter(|a| a.tool == tool)
        .map(advisory_text)
        .collect();
    if !found.is_empty() {
        found
    } else if row.join.free_of.contains(&tool) {
        vec![format!("{}: no advisory reported (scan read)", tool.name())]
    } else {
        vec![format!("{}: not assessed (see Scans)", tool.name())]
    }
}

pub(super) fn markdown(report: &Report) -> String {
    let mut text = String::from("# SOUP report\n\n");
    let _ = writeln!(text, "As of: {}", report.as_of);
    let _ = writeln!(text, "Records: {}", report.rows.len());
    for row in &report.rows {
        let _ = writeln!(text, "\n## {}\n", row.join.purl);
        let _ = writeln!(
            text,
            "- Record: {}{}",
            row.join.path.display(),
            row.join
                .id
                .as_ref()
                .map_or_else(String::new, |id| format!(" ({id})"))
        );
        let _ = writeln!(
            text,
            "- Version: {}",
            row.version.as_deref().unwrap_or(NOT_STATED)
        );
        let _ = writeln!(text, "- Serves: {}", listed_text(&row.serves, Clone::clone));
        let _ = writeln!(
            text,
            "- Known anomalies: {}",
            listed_text(&row.anomalies, anomaly_text)
        );
        let _ = writeln!(
            text,
            "- Support status: {}",
            resolved_text(&row.support.support_status, |s| s.as_str().to_owned())
        );
        let _ = writeln!(
            text,
            "- End of support: {}",
            resolved_text(&row.support.end_of_support, |d| d.to_string())
        );
        let _ = writeln!(text, "- Flags: {}", flags(row));
        let _ = writeln!(text, "- Advisories:");
        for tool in Tool::ALL {
            for line in tool_lines(row, tool) {
                let _ = writeln!(text, "  - {line}");
            }
        }
    }
    text.push_str("\n## Unmatched advisories\n\n");
    if report.unmatched.is_empty() {
        text.push_str("none\n");
    }
    for unmatched in &report.unmatched {
        let _ = writeln!(text, "- {}", unmatched_text(unmatched));
    }
    text.push_str("\n## Scans\n\n");
    for state in &report.scans {
        let _ = writeln!(text, "- {}: {}", state.tool.name(), scan_text(state));
    }
    for path in &report.unreadable_scans {
        let _ = writeln!(text, "- unreadable scan file, not read: {path}");
    }
    text
}

fn unmatched_text(unmatched: &Unmatched) -> String {
    format!(
        "{} {} {} (severity: {}): {}",
        unmatched.tool.name(),
        unmatched.rule_id,
        unmatched.package.as_deref().unwrap_or(NOT_STATED),
        unmatched.severity.as_deref().unwrap_or(NOT_STATED),
        unmatched.reason
    )
}

fn scan_text(state: &ScanState) -> String {
    let mut text = match &state.outcome {
        ScanOutcome::NotRun => {
            "not run (no scan is retained, or its log was refused at recording)".to_owned()
        }
        ScanOutcome::Vacuous { suite, timestamp } => format!(
            "vacuous: the newest scan (suite {suite}, {timestamp}) evaluated no rules, so nothing is known"
        ),
        ScanOutcome::Unordered { suites } => format!(
            "not read: a scan has a timestamp that is not an instant (suites: {})",
            suites.join(", ")
        ),
        ScanOutcome::Read {
            suite,
            timestamp,
            advisories,
            set_aside,
        } => format!(
            "read (suite {suite}, {timestamp}): {advisories} advisories, {set_aside} other diagnostics not read{}",
            if state.outcome.is_clean() {
                "; clean"
            } else {
                ""
            }
        ),
    };
    if !state.unread.is_empty() {
        let _ = write!(text, "; older scans not read: {}", state.unread.join(", "));
    }
    text
}

pub(super) fn json(report: &Report) -> Result<String, String> {
    let rows: Vec<Value> = report.rows.iter().map(row_json).collect();
    let document = json!({
        "as_of": report.as_of,
        "rows": rows,
        "unmatched": report.unmatched.iter().map(unmatched_json).collect::<Vec<_>>(),
        "scans": report.scans.iter().map(scan_json).collect::<Vec<_>>(),
        "unreadable_scans": report.unreadable_scans,
    });
    serde_json::to_string_pretty(&document).map_err(|error| error.to_string())
}

fn listed_json<T>(listed: &Listed<T>, item: impl Fn(&T) -> Value) -> Value {
    match listed {
        Listed::Absent => json!({ "state": NOT_STATED, "items": [] }),
        Listed::Empty => json!({ "state": "none", "items": [] }),
        Listed::Items(items) => json!({
            "state": "stated",
            "items": items.iter().map(item).collect::<Vec<_>>(),
        }),
    }
}

fn resolved_json<T: Copy>(resolved: &Resolved<T>, show: impl Fn(T) -> String) -> Value {
    match resolved {
        Resolved::Unknown => json!({ "value": "unknown", "origin": "none" }),
        Resolved::Record(value) => json!({ "value": show(*value), "origin": "record" }),
        Resolved::Supplied { value, source } => {
            json!({ "value": show(*value), "origin": "support file", "source": source })
        }
        Resolved::Disagree {
            record,
            supplied,
            source,
        } => json!({
            "value": show(*record),
            "origin": "record",
            "disagrees_with": { "value": show(*supplied), "origin": "support file", "source": source },
        }),
    }
}

fn advisory_json(advisory: &Attached) -> Value {
    let resolution = match &advisory.resolution {
        Resolution::Resolved(disposition) => json!({
            "state": "resolved",
            "decision": decision_word(disposition.decision),
            "rationale": disposition.rationale,
            "approval_ref": disposition.approval_ref,
            "attested_by": disposition.attested_by,
        }),
        Resolution::Unresolved(reason) => {
            json!({ "state": "unresolved", "reason": reason.to_string() })
        }
    };
    json!({
        "tool": advisory.tool.name(),
        "rule_id": advisory.rule_id,
        "severity": advisory.severity,
        "message": advisory.message,
        "package": advisory.package,
        "scan_timestamp": advisory.scan_timestamp,
        "resolution": resolution,
    })
}

fn row_json(row: &Row) -> Value {
    let free_of: Vec<&str> = row.join.free_of.iter().map(|tool| tool.name()).collect();
    json!({
        "purl": row.join.purl,
        "id": row.join.id,
        "path": row.join.path.display().to_string(),
        "version": row.version.as_deref().unwrap_or(NOT_STATED),
        "serves": listed_json(&row.serves, |id| json!(id)),
        "known_anomalies": listed_json(&row.anomalies, |a| json!({
            "statement": a.statement,
            "source_ref": a.source_ref,
        })),
        "support_status": resolved_json(&row.support.support_status, |s| s.as_str().to_owned()),
        "end_of_support": resolved_json(&row.support.end_of_support, |d| d.to_string()),
        "unresolved_advisory": row.unresolved(),
        "past_end_of_support": row.support.past_end_of_support,
        "support_disagreement": row.support.support_status.disagrees()
            || row.support.end_of_support.disagrees(),
        "disposition_clause": row.join.clause,
        "advisories": row.join.advisories.iter().map(advisory_json).collect::<Vec<_>>(),
        "no_advisory_reported_by": free_of,
    })
}

fn unmatched_json(unmatched: &Unmatched) -> Value {
    json!({
        "tool": unmatched.tool.name(),
        "rule_id": unmatched.rule_id,
        "package": unmatched.package,
        "severity": unmatched.severity,
        "reason": unmatched.reason.to_string(),
    })
}

fn scan_json(state: &ScanState) -> Value {
    let detail = match &state.outcome {
        ScanOutcome::NotRun => json!({ "state": "not run" }),
        ScanOutcome::Vacuous { suite, timestamp } => {
            json!({ "state": "vacuous", "suite": suite, "timestamp": timestamp })
        }
        ScanOutcome::Unordered { suites } => json!({ "state": "unordered", "suites": suites }),
        ScanOutcome::Read {
            suite,
            timestamp,
            advisories,
            set_aside,
        } => json!({
            "state": "read", "suite": suite, "timestamp": timestamp,
            "advisories": advisories, "diagnostics_not_read": set_aside,
        }),
    };
    let mut scan = json!({
        "tool": state.tool.name(),
        "older_suites_not_read": state.unread,
    });
    if let (Some(scan), Value::Object(detail)) = (scan.as_object_mut(), detail) {
        scan.extend(detail);
    }
    scan
}
