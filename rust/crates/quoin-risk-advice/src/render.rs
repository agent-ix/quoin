// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The Markdown rendering (FR-146 to FR-148, NFR-031).
//!
//! Every sentence has the policy or a named record as its subject. The tool
//! says nothing is acceptable, ineffective, unsafe, sound or low priority.

use std::fmt::Write as _;

use quoin_anomaly::{Derived, Signature};

use crate::Advice;
use crate::attribution::Attribution;
use crate::evidence::EvidenceEntry;
use crate::finding::{Finding, Token};
use crate::overall::OverallCheck;
use crate::policy::PolicyCell;
use crate::queue::QueueEntry;

const WAITING: &str = "waiting for a decision by a named person";

fn describe(finding: &Finding) -> String {
    let token = serde_json::to_value(finding.finding)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default();
    let missing = match finding.finding {
        Token::ControlWithoutRequirement => Some("no requirement is linked"),
        Token::ControlWithoutVerification => Some("no verification is linked"),
        _ => None,
    };
    match (missing, finding.names.is_empty()) {
        (Some(words), _) => format!("{words} ({token})"),
        (None, true) => token,
        (None, false) => format!("{token} naming {}", finding.names.join(", ")),
    }
}

fn findings_line(findings: &[Finding]) -> String {
    findings.iter().map(describe).collect::<Vec<_>>().join("; ")
}

fn cell_line(cell: &PolicyCell) -> String {
    let head = format!(
        "- {} ({}): severity {}, probability {}",
        cell.estimate, cell.phase, cell.severity, cell.probability
    );
    let policy = cell
        .policy
        .as_ref()
        .map_or(String::new(), |id| format!(", policy {id}"));
    match (&cell.region, cell.outside_acceptable) {
        (Some(region), flag) => {
            let mut line = format!("{head}{policy}: the policy marks this cell as {region}.");
            if flag == Some(true) {
                line.push_str(" The policy flags this region as outside its acceptable region.");
            }
            line
        }
        (None, _) => format!(
            "{head}{policy}: no region; {}.",
            findings_line(&cell.findings)
        ),
    }
}

fn evidence_block(out: &mut String, entry: &EvidenceEntry) {
    let situation = entry.situation.as_deref().unwrap_or("none");
    let hazards = if entry.hazards.is_empty() {
        "none".to_owned()
    } else {
        entry.hazards.join(", ")
    };
    let _ = writeln!(
        out,
        "### {} (situation {situation}; hazards {hazards})\n",
        entry.estimate
    );
    let mut any = false;
    for finding in &entry.findings {
        any = true;
        let _ = writeln!(out, "- {}", describe(finding));
    }
    for control in &entry.controls {
        let line = if control.findings.is_empty() {
            "requirements and verifications are linked".to_owned()
        } else {
            findings_line(&control.findings)
        };
        let _ = writeln!(
            out,
            "- control {}: requirements [{}], verifications [{}]; {line}",
            control.control,
            control.requirements.join(", "),
            control.verifications.join(", "),
        );
        any |= !control.findings.is_empty();
    }
    for anomaly in &entry.anomalies {
        any = true;
        let _ = writeln!(
            out,
            "- open anomaly {} (kind {}, impact {}) names {}",
            anomaly.anomaly, anomaly.kind, anomaly.impact, anomaly.names
        );
    }
    if !any {
        out.push_str("- no findings listed\n");
    }
    out.push('\n');
}

fn queue_line(entry: &QueueEntry) -> String {
    let mut line = format!("- {} ({}): {WAITING}", entry.record, entry.kind);
    if let Some(estimate) = &entry.estimate {
        let _ = write!(line, "; estimate {estimate}");
    }
    if let Some(policy) = &entry.policy {
        let _ = write!(line, "; policy {policy}");
    }
    if let (Some(severity), Some(probability)) = (&entry.severity, &entry.probability) {
        let _ = write!(line, "; severity {severity}, probability {probability}");
    }
    if let Some(region) = &entry.region {
        let _ = write!(line, "; the policy marks this cell as {region}");
        if entry.outside_acceptable == Some(true) {
            line.push_str("; the policy flags this region as outside its acceptable region");
        }
    }
    if !entry.references.is_empty() {
        let _ = write!(line, "; references {}", entry.references.join(", "));
    }
    if !entry.findings.is_empty() {
        let _ = write!(line, "; {}", findings_line(&entry.findings));
    }
    line
}

fn overall_line(check: &OverallCheck) -> String {
    let product = check.product.as_deref().unwrap_or("null");
    let head = format!(
        "- {} (product {product}): decision {}",
        check.evaluation, check.decision
    );
    if check.findings.is_empty() {
        format!("{head}; no findings")
    } else {
        format!("{head}; {}", findings_line(&check.findings))
    }
}

fn attribution_line(entry: &Attribution) -> String {
    let decider = entry.decided_by.as_deref().unwrap_or("null");
    let mut line = format!(
        "- {} ({}): decision {}, decided by {decider}",
        entry.record, entry.kind, entry.decision
    );
    if let Some(principal) = entry.signer_principal.as_deref() {
        let _ = write!(line, "; verified signer {principal}");
    }
    match &entry.decision_change {
        Derived::Present(change) => {
            let _ = write!(
                line,
                "; set by commit {} authored by {} on {}, signature {} ({})",
                change.commit,
                change.author,
                change.authored_at,
                signature_word(change.signature),
                change.signature_code
            );
        }
        other => {
            let _ = write!(line, "; decision commit {}", other.state());
        }
    }
    if !entry.findings.is_empty() {
        let _ = write!(line, "; {}", findings_line(&entry.findings));
    }
    line
}

fn signature_word(signature: Signature) -> &'static str {
    match signature {
        Signature::Signed => "signed",
        Signature::Unsigned => "unsigned",
        Signature::Unknown => "unknown",
    }
}

pub(crate) fn markdown(advice: &Advice) -> String {
    let mut out = format!(
        "# Risk advice\n\n{}\n\n## policy_cells\n\n",
        advice.attribution
    );
    if advice.policy_cells.is_empty() {
        out.push_str("none\n");
    }
    for cell in &advice.policy_cells {
        let _ = writeln!(out, "{}", cell_line(cell));
    }
    out.push_str("\n## evidence_gaps\n\n");
    if advice.evidence_gaps.is_empty() {
        out.push_str("none\n\n");
    }
    for entry in &advice.evidence_gaps {
        evidence_block(&mut out, entry);
    }
    out.push_str("## decision_queue\n\n");
    if advice.decision_queue.queue.is_empty() && advice.decision_queue.no_record.is_empty() {
        out.push_str("none\n");
    }
    for entry in &advice.decision_queue.queue {
        let _ = writeln!(out, "{}", queue_line(entry));
    }
    for entry in &advice.decision_queue.no_record {
        let _ = writeln!(
            out,
            "- {}: no residual-risk record, {WAITING}; {}",
            entry.estimate,
            findings_line(&entry.findings)
        );
    }
    out.push_str("\n## overall_evaluation_checks\n\n");
    if advice.overall_evaluation_checks.is_empty() {
        out.push_str("none\n");
    }
    for check in &advice.overall_evaluation_checks {
        let _ = writeln!(out, "{}", overall_line(check));
    }
    out.push_str("\n## decision_attribution\n\n");
    if advice.decision_attribution.is_empty() {
        out.push_str("none\n");
    }
    for entry in &advice.decision_attribution {
        let _ = writeln!(out, "{}", attribution_line(entry));
    }
    out
}
