// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `cargo deny --format json check` → findings (FR-034).
//!
//! The output is JSON lines on stderr: one `diagnostic` object per problem the
//! run found, closed by a single `summary` object counting what each check
//! (`advisories`, `bans`, `licenses`, `sources`) reported. **The summary is the
//! proof the scan executed**, the same job the SARIF `run` envelope does and
//! `vulnerabilities.found` does for cargo-audit. A clean run is a summary with
//! zero diagnostics before it; a run that never completed has no summary and is
//! refused rather than read as clean.
//!
//! cargo-deny reports no count of the rules it evaluated, so `rules_evaluated`
//! stays unset: the vacuity question cannot be asked, and the check stays
//! silent rather than guess (FR-034-AC-11).

use serde_json::Value;

use super::FindingResult;
use crate::error::EvidenceError;
use crate::types::Finding;

/// Parse `cargo deny --format json check` output.
///
/// # Errors
///
/// Refuses a line that is not JSON and output with no `summary` object.
pub fn parse_cargo_deny(raw: &str) -> Result<FindingResult, EvidenceError> {
    let mut findings: Vec<Finding> = Vec::new();
    let mut summarised = false;
    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let object: Value = serde_json::from_str(line).map_err(|error| {
            EvidenceError::adapter(
                "cargo-deny",
                format!("line {} is not JSON: {error}", index.saturating_add(1)),
            )
        })?;
        match object.get("type").and_then(Value::as_str) {
            Some("summary") => summarised = true,
            Some("diagnostic") => {
                if let Some(finding) = object.get("fields").and_then(diagnostic_finding) {
                    findings.push(finding);
                }
            }
            _ => {}
        }
    }
    if !summarised {
        return Err(EvidenceError::adapter(
            "cargo-deny",
            "no `summary` object — expected complete `cargo deny --format json check` output; a run that never finished proves no scan executed",
        ));
    }
    Ok(FindingResult {
        findings,
        tool: Some("cargo-deny".to_owned()),
        ruleset: None,
        rules_evaluated: None,
    })
}

/// One diagnostic as a finding. An advisory is identified by its RUSTSEC id,
/// anything else by the check's own `code`; a diagnostic with neither is
/// skipped. The severity is the tool's word, never normalized (FR-034-CON-2).
fn diagnostic_finding(fields: &Value) -> Option<Finding> {
    let rule_id = fields
        .get("advisory")
        .and_then(|advisory| advisory.get("id"))
        .and_then(Value::as_str)
        .or_else(|| fields.get("code").and_then(Value::as_str))
        .filter(|id| !id.is_empty())?;
    let mut finding = Finding::new(rule_id);
    finding.severity = fields
        .get("severity")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    finding.message = fields
        .get("message")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    finding.path = fields
        .get("graphs")
        .and_then(Value::as_array)
        .and_then(|graphs| graphs.first())
        .and_then(|graph| graph.get("Krate"))
        .and_then(|krate| {
            let name = krate.get("name").and_then(Value::as_str)?;
            Some(match krate.get("version").and_then(Value::as_str) {
                Some(version) => format!("{name}@{version}"),
                None => name.to_owned(),
            })
        });
    Some(finding)
}
