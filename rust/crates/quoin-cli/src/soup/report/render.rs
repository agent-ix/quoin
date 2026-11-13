// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The renderings of the SOUP report: Markdown and JSON from one [`Report`].
//!
//! Both print only what an input stated. A scanner's severity is its own word;
//! no key or line here scores, ranks or classifies.

use std::fmt::Write as _;

use serde_json::{Value, json};

use super::{Anomaly, Listed, Report, Row};
use crate::soup::advisory::{
    Attached, Resolution, ScanOutcome, ScanState, Tool, Unmatched, Unresolved,
};
use crate::soup::policy::{
    LicenceOutcome, LicenceState, PolicyFinding, PolicyStatus, PolicyUnmatched,
};
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
    finding_text(
        advisory.tool.name(),
        (&advisory.rule_id, &advisory.package),
        advisory.severity.as_deref(),
        &advisory.scan_timestamp,
        &advisory.resolution,
    )
}

fn policy_finding_text(finding: &PolicyFinding) -> String {
    finding_text(
        finding.tool.name(),
        (&finding.rule_id, &finding.package),
        finding.severity.as_deref(),
        &finding.scan_timestamp,
        &finding.resolution,
    )
}

/// One attached finding, advisory or policy: tool, rule, package, the tool's
/// own severity word, the scan, and how it stands against the disposition.
fn finding_text(
    tool: &str,
    (rule_id, package): (&str, &str),
    severity: Option<&str>,
    scan_timestamp: &str,
    resolution: &Resolution,
) -> String {
    let severity = severity.unwrap_or(NOT_STATED);
    let outcome = match resolution {
        Resolution::Resolved(disposition) => format!(
            "resolved: {} by {} ({}; {})",
            decision_word(disposition.decision),
            one_line(&disposition.attested_by),
            one_line(&disposition.approval_ref),
            one_line(&disposition.rationale)
        ),
        Resolution::Unresolved(reason) => format!("unresolved: {reason}"),
    };
    format!("{tool} {rule_id} {package} (severity: {severity}; scan {scan_timestamp}): {outcome}")
}

/// The spec wording of a row's policy status, or `None` when the row lists
/// findings instead.
fn policy_status_word(status: &PolicyStatus) -> Option<&'static str> {
    Some(match status {
        PolicyStatus::Findings => return None,
        PolicyStatus::NoFindings { .. } => "no policy findings in the scan read",
        PolicyStatus::NotRun => "policy findings not assessed: licence-policy not run",
        PolicyStatus::Vacuous => "policy findings not assessed: licence-policy scan vacuous",
        PolicyStatus::Unordered => "policy findings not assessed: licence-policy scan unordered",
        PolicyStatus::NotEvaluated => "not evaluated by the licence-policy scan read",
        PolicyStatus::Blocked => {
            "policy findings not assessed: an unmatched licence-policy finding could belong to this record"
        }
    })
}

/// The Markdown line of a row's policy status: the wording, and for a clean
/// scan the scan read.
fn policy_status_text(status: &PolicyStatus) -> Option<String> {
    let word = policy_status_word(status)?;
    Some(match status {
        PolicyStatus::NoFindings { timestamp, ruleset } => format!(
            "{word} (licence-policy, {timestamp}, ruleset: {})",
            ruleset.as_deref().unwrap_or(NOT_STATED)
        ),
        _ => word.to_owned(),
    })
}

/// Stated on a row whose disposition covers, or was too old to cover, a finding.
const DISPOSITION_SCOPE: &str = "a disposition covers the findings of this record reported by a scan at or before its attestation; a finding reported by a later scan is unresolved";

/// Whether a disposition is in play on the row: some finding it resolved, or
/// was refused for being older than the scan that reported the finding.
fn has_disposition(row: &Row) -> bool {
    let engaged = |resolution: &Resolution| {
        matches!(
            resolution,
            Resolution::Resolved(_) | Resolution::Unresolved(Unresolved::ScanNewerThanAttestation)
        )
    };
    row.join.advisories.iter().any(|a| engaged(&a.resolution))
        || row.policy.findings.iter().any(|f| engaged(&f.resolution))
}

fn decision_word(decision: quoin_assurance::DispositionDecision) -> String {
    serde_json::to_value(decision)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn flags(row: &Row) -> String {
    let mut flags = Vec::new();
    if row.unresolved_advisory() {
        flags.push("unresolved advisory");
    }
    if row.unresolved_policy() {
        flags.push("unresolved policy finding");
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
        let _ = writeln!(text, "- Policy findings:");
        for finding in &row.policy.findings {
            let _ = writeln!(text, "  - {}", policy_finding_text(finding));
        }
        if let Some(status) = policy_status_text(&row.policy.status) {
            let _ = writeln!(text, "  - {status}");
        }
        if has_disposition(row) {
            let _ = writeln!(text, "- Disposition scope: {DISPOSITION_SCOPE}");
        }
    }
    text.push_str("\n## Unmatched advisories\n\n");
    if report.unmatched.is_empty() {
        text.push_str("none\n");
    }
    for unmatched in &report.unmatched {
        let _ = writeln!(text, "- {}", unmatched_text(unmatched));
    }
    text.push_str("\n## Unmatched policy findings\n\n");
    if report.unmatched_policy.is_empty() {
        text.push_str("none\n");
    }
    for unmatched in &report.unmatched_policy {
        let _ = writeln!(text, "- {}", policy_unmatched_text(unmatched));
    }
    text.push_str("\n## Scans\n\n");
    for state in &report.scans {
        let _ = writeln!(text, "- {}: {}", state.tool.name(), scan_text(state));
    }
    let _ = writeln!(text, "- licence-policy: {}", licence_text(&report.licence));
    let _ = writeln!(
        text,
        "- cargo-deny policy findings: {}",
        report.deny_policy_read.map_or_else(
            || "not read (see the cargo-deny scan above)".to_owned(),
            |count| format!("{count} read")
        )
    );
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

fn policy_unmatched_text(unmatched: &PolicyUnmatched) -> String {
    format!(
        "{} {} {} (severity: {}): {}",
        unmatched.tool.name(),
        unmatched.rule_id,
        unmatched.package.as_deref().unwrap_or(NOT_STATED),
        unmatched.severity.as_deref().unwrap_or(NOT_STATED),
        unmatched.reason
    )
}

fn licence_text(state: &LicenceState) -> String {
    let mut text = match &state.outcome {
        LicenceOutcome::NotRun => {
            "not run (no scan is retained, or its recording was refused)".to_owned()
        }
        LicenceOutcome::Vacuous { suite, timestamp } => format!(
            "vacuous: the newest scan (suite {suite}, {timestamp}) evaluated no component, so nothing is known"
        ),
        LicenceOutcome::Unordered { suites } => format!(
            "not read: a scan has a timestamp that is not an instant (suites: {})",
            suites.join(", ")
        ),
        LicenceOutcome::Read {
            suite,
            timestamp,
            ruleset,
            evaluated,
            findings,
        } => format!(
            "read (suite {suite}, {timestamp}): {evaluated} components evaluated, {findings} findings; ruleset: {}",
            ruleset.as_deref().unwrap_or(NOT_STATED)
        ),
    };
    if !state.unread.is_empty() {
        let _ = write!(text, "; older scans not read: {}", state.unread.join(", "));
    }
    text
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
        "unmatched_policy_findings": report.unmatched_policy.iter().map(policy_unmatched_json).collect::<Vec<_>>(),
        "scans": report.scans.iter().map(scan_json).collect::<Vec<_>>(),
        "licence_policy_scan": licence_json(&report.licence),
        "cargo_deny_policy_findings_read": report.deny_policy_read,
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
    json!({
        "tool": advisory.tool.name(),
        "rule_id": advisory.rule_id,
        "severity": advisory.severity,
        "message": advisory.message,
        "package": advisory.package,
        "scan_timestamp": advisory.scan_timestamp,
        "resolution": resolution_json(&advisory.resolution),
    })
}

fn policy_finding_json(finding: &PolicyFinding) -> Value {
    json!({
        "tool": finding.tool.name(),
        "rule_id": finding.rule_id,
        "severity": finding.severity,
        "message": finding.message,
        "package": finding.package,
        "scan_timestamp": finding.scan_timestamp,
        "resolution": resolution_json(&finding.resolution),
    })
}

fn policy_status_json(status: &PolicyStatus) -> Value {
    match (policy_status_word(status), status) {
        (None, _) => json!({ "state": "findings" }),
        (Some(word), PolicyStatus::NoFindings { timestamp, ruleset }) => {
            json!({ "state": word, "scan_timestamp": timestamp, "ruleset": ruleset })
        }
        (Some(word), _) => json!({ "state": word }),
    }
}

fn resolution_json(resolution: &Resolution) -> Value {
    match resolution {
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
    }
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
        "unresolved_advisory": row.unresolved_advisory(),
        "unresolved_policy_finding": row.unresolved_policy(),
        "past_end_of_support": row.support.past_end_of_support,
        "support_disagreement": row.support.support_status.disagrees()
            || row.support.end_of_support.disagrees(),
        "disposition_clause": row.join.clause,
        "advisories": row.join.advisories.iter().map(advisory_json).collect::<Vec<_>>(),
        "no_advisory_reported_by": free_of,
        "policy_findings": row.policy.findings.iter().map(policy_finding_json).collect::<Vec<_>>(),
        "policy_status": policy_status_json(&row.policy.status),
        "disposition_scope": has_disposition(row).then_some(DISPOSITION_SCOPE),
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

fn policy_unmatched_json(unmatched: &PolicyUnmatched) -> Value {
    json!({
        "tool": unmatched.tool.name(),
        "rule_id": unmatched.rule_id,
        "package": unmatched.package,
        "severity": unmatched.severity,
        "reason": unmatched.reason.to_string(),
    })
}

fn licence_json(state: &LicenceState) -> Value {
    let mut scan = match &state.outcome {
        LicenceOutcome::NotRun => json!({ "state": "not run" }),
        LicenceOutcome::Vacuous { suite, timestamp } => {
            json!({ "state": "vacuous", "suite": suite, "timestamp": timestamp })
        }
        LicenceOutcome::Unordered { suites } => json!({ "state": "unordered", "suites": suites }),
        LicenceOutcome::Read {
            suite,
            timestamp,
            ruleset,
            evaluated,
            findings,
        } => json!({
            "state": "read", "suite": suite, "timestamp": timestamp, "ruleset": ruleset,
            "components_evaluated": evaluated, "findings": findings,
        }),
    };
    if let Some(scan) = scan.as_object_mut() {
        scan.insert("older_suites_not_read".to_owned(), json!(state.unread));
    }
    scan
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
