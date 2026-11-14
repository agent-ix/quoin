// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Inward completeness: is every clause that binds me discharged by something
//! I have (FR-133).
//!
//! A pure function of a [`DischargeReport`]. It states the binding population
//! as counts that must add up, `discharged + Σ dispositions + open == binding`,
//! and lists the undischarged remainder by clause, so the answer is never a
//! bare ratio. Unresolved applicability and unused facts are separate
//! populations outside the binding denominator and are counted beside it.

use quoin_quire_types::{ClauseForce, ClauseSetKey};
use serde::{Deserialize, Serialize};

use super::report::{DischargeFact, DischargeReport, DispositionDecision};

/// The one accepted `schemaVersion` of an inward-completeness report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum CompletenessSchemaVersion {
    /// `inward-completeness-v1`.
    #[serde(rename = "inward-completeness-v1")]
    V1,
}

/// Dispositions by what was decided.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DispositionCounts {
    /// Risk knowingly carried.
    pub accepted_risk: usize,
    /// An exception with an expiry.
    pub temporary_exception: usize,
    /// Responsibility passed to another party.
    pub delegated: usize,
}

impl DispositionCounts {
    /// The sum over every decision.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.accepted_risk + self.temporary_exception + self.delegated
    }
}

/// A binding clause nothing discharges, with why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct OpenClause {
    /// The clause's id.
    pub clause_id: String,
    /// How strongly it binds.
    pub force: ClauseForce,
    /// Why it is open, as the discharge report states it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The binding population accounted for, never scored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct InwardCompleteness {
    /// Always `inward-completeness-v1`.
    pub schema_version: CompletenessSchemaVersion,
    /// The clause set the report is about.
    pub clause_set: ClauseSetKey,
    /// The evaluation instant of the discharge report it was derived from.
    pub as_of: String,
    /// Clauses that bind: `discharged + dispositions.total() + open.len()`.
    pub binding: usize,
    /// Binding clauses discharged by direct evidence.
    pub discharged: usize,
    /// Binding clauses discharged by an approved disposition.
    pub dispositions: DispositionCounts,
    /// The undischarged remainder.
    pub open: Vec<OpenClause>,
    /// Clauses whose applicability is undecided; outside `binding`.
    pub unresolved: usize,
    /// Facts supplied and not spent; outside `binding`.
    pub unused_facts: usize,
}

/// Account for the binding population of a discharge report.
///
/// Private to the module's store path on purpose: the report is always built
/// from the live binding report, so the populations add up by construction.
#[must_use]
pub(super) fn build_inward_completeness(report: &DischargeReport) -> InwardCompleteness {
    let mut dispositions = DispositionCounts::default();
    for entry in &report.binding.dispositions {
        if let Some(DischargeFact::Disposition(fact)) = &entry.fact {
            match fact.decision {
                DispositionDecision::AcceptedRisk => dispositions.accepted_risk += 1,
                DispositionDecision::TemporaryException => dispositions.temporary_exception += 1,
                DispositionDecision::Delegated => dispositions.delegated += 1,
            }
        }
    }
    let open: Vec<OpenClause> = report
        .binding
        .open
        .iter()
        .map(|entry| OpenClause {
            clause_id: entry.clause_id.clone(),
            force: entry.force,
            reason: entry.reason.clone(),
        })
        .collect();
    let discharged = report.binding.direct.len();
    InwardCompleteness {
        schema_version: CompletenessSchemaVersion::V1,
        clause_set: report.clause_set.clone(),
        as_of: report.as_of.clone(),
        binding: discharged + report.binding.dispositions.len() + open.len(),
        discharged,
        dispositions,
        open,
        unresolved: report.unresolved.len(),
        unused_facts: report.unused_facts.len(),
    }
}

/// Render the accounting as deterministic Markdown.
#[must_use]
pub fn render_inward_completeness(report: &InwardCompleteness) -> String {
    let mut lines = vec![
        format!(
            "# Inward completeness: {}/{}@{}",
            report.clause_set.authority, report.clause_set.id, report.clause_set.version
        ),
        String::new(),
        format!("- Evaluated as of: `{}`", report.as_of),
        format!("- Binding clauses: {}", report.binding),
        format!("- Discharged by evidence: {}", report.discharged),
        format!(
            "- Approved dispositions: {} (accepted risk {}, temporary exception {}, delegated {})",
            report.dispositions.total(),
            report.dispositions.accepted_risk,
            report.dispositions.temporary_exception,
            report.dispositions.delegated
        ),
        format!("- Open: {}", report.open.len()),
        format!(
            "- Unresolved applicability (outside binding): {}",
            report.unresolved
        ),
        format!("- Unused facts (outside binding): {}", report.unused_facts),
        String::new(),
        "## Open binding clauses".to_owned(),
        String::new(),
    ];
    if report.open.is_empty() {
        lines.push("_None._".to_owned());
    }
    for clause in &report.open {
        lines.push(open_line(clause));
    }
    lines.push(String::new());
    lines.join("\n")
}

fn open_line(clause: &OpenClause) -> String {
    let force = serde_json::to_value(clause.force)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default();
    match &clause.reason {
        Some(reason) => format!("- `{}` — {force}; {reason}", clause.clause_id),
        None => format!("- `{}` — {force}", clause.clause_id),
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use ix_trace_rs::trace;

    use super::super::build::build_discharge_report;
    use super::super::fixtures::{attestation, binding, direct, request};
    use super::{build_inward_completeness, render_inward_completeness};

    fn disposition() -> serde_json::Value {
        serde_json::json!({
            "kind": "disposition",
            "clauseId": "SYN-002",
            "decision": "accepted_risk",
            "owner": "risk-owner-1",
            "rationale": "carried",
            "approvalRef": "approval://1",
            "attestation": attestation()
        })
    }

    fn wide_binding() -> quoin_quire_types::ClauseBindingReport {
        let mut value = serde_json::to_value(binding()).unwrap();
        value["clauses"] = serde_json::json!([
            {"clauseId": "SYN-001", "force": "mandatory", "outcome": "binding",
             "reasons": [], "expectedOutputs": []},
            {"clauseId": "SYN-002", "force": "mandatory", "outcome": "binding",
             "reasons": [], "expectedOutputs": []},
            {"clauseId": "SYN-003", "force": "mandatory", "outcome": "binding",
             "reasons": [], "expectedOutputs": []},
            {"clauseId": "SYN-004", "force": "mandatory", "outcome": "unresolved",
             "reasons": [], "expectedOutputs": []},
            {"clauseId": "SYN-005", "force": "permitted", "outcome": "not_binding",
             "reasons": [], "expectedOutputs": []}
        ]);
        serde_json::from_value(value).unwrap()
    }

    #[trace("FR-133-AC-5")]
    #[test]
    fn the_populations_add_up_to_the_binding_clauses_and_the_remainder_is_named() {
        let mut request = request(vec![direct(), disposition()]);
        request.binding = wide_binding();
        let report = build_discharge_report(&request).unwrap();
        let completeness = build_inward_completeness(&report);
        assert_eq!(completeness.binding, 3);
        assert_eq!(completeness.discharged, 1);
        assert_eq!(completeness.dispositions.accepted_risk, 1);
        assert_eq!(completeness.open.len(), 1);
        assert_eq!(completeness.open[0].clause_id, "SYN-003");
        assert_eq!(
            completeness.discharged + completeness.dispositions.total() + completeness.open.len(),
            completeness.binding
        );
        // Unresolved applicability is counted beside the denominator, not in it.
        assert_eq!(completeness.unresolved, 1);
        let json = serde_json::to_string(&completeness).unwrap();
        assert!(!json.contains("score") && !json.contains("percent"));
    }

    #[trace("FR-133-AC-5")]
    #[test]
    fn nothing_discharged_is_all_open_and_renders_the_remainder() {
        let report = build_discharge_report(&request(vec![])).unwrap();
        let completeness = build_inward_completeness(&report);
        assert_eq!(completeness.binding, 1);
        assert_eq!(completeness.discharged, 0);
        let text = render_inward_completeness(&completeness);
        assert!(text.contains("- Binding clauses: 1"), "{text}");
        assert!(
            text.contains("- `SYN-001` — mandatory; no discharge fact"),
            "{text}"
        );
    }

    #[trace("FR-133-AC-5")]
    #[test]
    fn unused_facts_and_expired_facts_are_counted_and_named() {
        let mut unused = disposition();
        unused["clauseId"] = "SYN-005".into();
        let mut expired = direct();
        expired["attestation"]["expiresAt"] = "2026-08-10T00:00:00.000Z".into();
        let mut request = request(vec![expired, unused]);
        request.binding = wide_binding();
        let completeness = build_inward_completeness(&build_discharge_report(&request).unwrap());
        // The fact for the not-binding clause is outside binding, as an unused fact.
        assert_eq!(completeness.unused_facts, 1);
        // The expired attestation leaves its clause open with the reason visible.
        assert_eq!(completeness.discharged, 0);
        let expired_clause = completeness
            .open
            .iter()
            .find(|c| c.clause_id == "SYN-001")
            .unwrap();
        assert!(expired_clause.reason.is_some());
        assert_eq!(
            completeness.discharged + completeness.dispositions.total() + completeness.open.len(),
            completeness.binding
        );
    }
}
