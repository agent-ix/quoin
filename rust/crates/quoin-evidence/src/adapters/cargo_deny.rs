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
//! refused rather than read as clean. A summary counting more errors and
//! warnings than the diagnostics read is refused too: `--log-level off` prints
//! the summary alone, so the summary's presence is not enough.
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
/// The `summary` must be the last non-empty line and appear once; it must name
/// at least one check, which is recorded as the `ruleset` so the evidence says
/// which checks ran. Its error and warning counts must be accounted for by the
/// diagnostics read: a run whose diagnostics were suppressed (`--log-level off`)
/// reports `"errors":1` beside no diagnostic, and reading that as clean would
/// turn a failed run into a discharge.
///
/// # Errors
///
/// Refuses a line that is not JSON, output without exactly one trailing
/// `summary`, a summary naming no check, and a summary counting more errors and
/// warnings than the diagnostics read.
pub fn parse_cargo_deny(raw: &str) -> Result<FindingResult, EvidenceError> {
    let mut findings: Vec<Finding> = Vec::new();
    let mut counted_diagnostics: u64 = 0;
    let mut summary: Option<Value> = None;
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
        if summary.is_some() {
            return Err(refuse(
                "the `summary` object must be the last line and appear once",
            ));
        }
        match object.get("type").and_then(Value::as_str) {
            Some("summary") => summary = Some(object),
            Some("diagnostic") => {
                if let Some(fields) = object.get("fields") {
                    if matches!(
                        fields.get("severity").and_then(Value::as_str),
                        Some("error" | "warning")
                    ) {
                        counted_diagnostics = counted_diagnostics.saturating_add(1);
                    }
                    findings.push(diagnostic_finding(fields));
                }
            }
            _ => {}
        }
    }
    let Some(summary) = summary else {
        return Err(refuse(
            "no `summary` object — expected complete `cargo deny --format json check` output; a run that never finished proves no scan executed",
        ));
    };
    let checks: Vec<(&String, &Value)> = summary
        .get("fields")
        .and_then(Value::as_object)
        .map(|fields| fields.iter().filter(|(_, v)| v.is_object()).collect())
        .unwrap_or_default();
    if checks.is_empty() {
        return Err(refuse(
            "the `summary` names no check — it proves nothing was evaluated",
        ));
    }
    let reported: u64 = checks
        .iter()
        .flat_map(|(_, counts)| ["errors", "warnings"].map(|key| counts.get(key)))
        .filter_map(|count| count?.as_u64())
        .fold(0, u64::saturating_add);
    if reported > counted_diagnostics {
        return Err(refuse(format!(
            "the `summary` counts {reported} error and warning diagnostics but only {counted_diagnostics} were read — the run failed or its diagnostics were suppressed (`--log-level off`), so it is not a clean scan"
        )));
    }
    let mut names: Vec<&str> = checks.iter().map(|(name, _)| name.as_str()).collect();
    names.sort_unstable();
    Ok(FindingResult {
        findings,
        tool: Some("cargo-deny".to_owned()),
        evaluated: None,
        ruleset: Some(names.join(",")),
        rules_evaluated: None,
    })
}

fn refuse(message: impl Into<String>) -> EvidenceError {
    EvidenceError::adapter("cargo-deny", message)
}

/// One diagnostic as a finding. An advisory is identified by its RUSTSEC id,
/// anything else by the check's own `code`, and a diagnostic with neither is
/// kept as `unclassified` rather than dropped. The severity is the tool's word,
/// never normalized (FR-034-CON-2).
fn diagnostic_finding(fields: &Value) -> Finding {
    let rule_id = fields
        .get("advisory")
        .and_then(|advisory| advisory.get("id"))
        .and_then(Value::as_str)
        .or_else(|| fields.get("code").and_then(Value::as_str))
        .filter(|id| !id.is_empty())
        .unwrap_or("unclassified");
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
    finding
}
