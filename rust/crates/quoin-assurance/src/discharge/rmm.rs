// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The requirements mapping matrix: a read-only view of a discharge report
//! (FR-135).
//!
//! One row per clause of the binding report, in the report's order, so a gap
//! is a row and never an absent line. Every cell was decided in
//! [`super::build`]; this module only names it.

use std::collections::HashMap;

use quoin_quire_types::{ClauseBinding, ClauseBindingReport, ClauseForce, ClauseSetKey};
use serde::{Deserialize, Serialize};

use super::report::{
    ClauseDischarge, DischargeError, DischargeFact, DischargeReport, DischargeState,
    UnusedDischargeFact,
};

/// The one accepted `schemaVersion` of a requirements mapping matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum RequirementsMappingSchemaVersion {
    /// `requirements-mapping-v1`.
    #[serde(rename = "requirements-mapping-v1")]
    V1,
}

/// Whether the requirement applies to the evaluated context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum RmmApplicability {
    /// The clause binds.
    Applicable,
    /// The clause does not bind.
    NotApplicable,
    /// Applicability could not be decided.
    Unresolved,
}

impl RmmApplicability {
    fn label(self) -> &'static str {
        match self {
            Self::Applicable => "Applicable",
            Self::NotApplicable => "Not applicable",
            Self::Unresolved => "Unresolved",
        }
    }
}

/// How the requirement stands. `Compliant` needs a current direct fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum RmmCompliance {
    /// A current direct fact discharges the clause.
    Compliant,
    /// A current approved disposition decides the clause.
    Tailored,
    /// Binding, with no fact.
    Undischarged,
    /// Binding, with a fact whose attestation is expired or not yet valid.
    AttestationNotCurrent,
    /// Applicability is unresolved.
    Undetermined,
    /// The clause does not bind.
    NotApplicable,
}

impl RmmCompliance {
    fn label(self) -> &'static str {
        match self {
            Self::Compliant => "Compliant",
            Self::Tailored => "Tailored",
            Self::Undischarged => "Not compliant - undischarged",
            Self::AttestationNotCurrent => "Not compliant - attestation not current",
            Self::Undetermined => "Undetermined",
            Self::NotApplicable => "Not applicable",
        }
    }
}

/// One requirement's row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RmmRow {
    /// Req#: the clause id.
    pub clause_id: String,
    /// How strongly the clause binds.
    pub force: ClauseForce,
    /// Applicability.
    pub applicability: RmmApplicability,
    /// Compliance.
    pub compliance: RmmCompliance,
    /// Why the row is not compliant or undetermined, when there is a why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Evidence links of a current direct fact.
    pub evidence_refs: Vec<String>,
    /// Tailoring Description: the decision and its approval reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tailoring_description: Option<String>,
    /// Tailoring Rationale: the disposition's rationale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tailoring_rationale: Option<String>,
}

/// The matrix, never scored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RequirementsMapping {
    /// Always `requirements-mapping-v1`.
    pub schema_version: RequirementsMappingSchemaVersion,
    /// The clause set the matrix is about.
    pub clause_set: ClauseSetKey,
    /// The evaluation instant of the discharge report it was derived from.
    pub as_of: String,
    /// One row per clause of the binding report, in its order.
    pub rows: Vec<RmmRow>,
    /// Facts that were not spent.
    pub unused_facts: Vec<UnusedDischargeFact>,
}

/// Fill the matrix from a binding report and the discharge report built from
/// it.
///
/// # Errors
///
/// [`DischargeError`] when the discharge report holds no entry for a clause of
/// the binding report, which cannot happen for a report built from `binding`;
/// the row is refused rather than invented.
pub fn build_requirements_mapping(
    binding: &ClauseBindingReport,
    report: &DischargeReport,
) -> Result<RequirementsMapping, DischargeError> {
    let entries: HashMap<&str, &ClauseDischarge> = report
        .binding
        .direct
        .iter()
        .chain(&report.binding.dispositions)
        .chain(&report.binding.open)
        .chain(&report.unresolved)
        .chain(&report.not_binding)
        .map(|entry| (entry.clause_id.as_str(), entry))
        .collect();
    let rows = binding
        .clauses
        .iter()
        .map(|clause| {
            entries
                .get(clause.clause_id.as_str())
                .map(|entry| row(entry, clause))
                .ok_or_else(|| {
                    DischargeError(format!(
                        "the discharge report holds no entry for clause {}",
                        clause.clause_id
                    ))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RequirementsMapping {
        schema_version: RequirementsMappingSchemaVersion::V1,
        clause_set: report.clause_set.clone(),
        as_of: report.as_of.clone(),
        rows,
        unused_facts: report.unused_facts.clone(),
    })
}

fn row(entry: &ClauseDischarge, clause: &ClauseBinding) -> RmmRow {
    let (applicability, compliance) = match entry.state {
        DischargeState::Direct => (RmmApplicability::Applicable, RmmCompliance::Compliant),
        DischargeState::Disposition => (RmmApplicability::Applicable, RmmCompliance::Tailored),
        DischargeState::Open if entry.fact.is_some() => (
            RmmApplicability::Applicable,
            RmmCompliance::AttestationNotCurrent,
        ),
        DischargeState::Open => (RmmApplicability::Applicable, RmmCompliance::Undischarged),
        DischargeState::Unresolved => (RmmApplicability::Unresolved, RmmCompliance::Undetermined),
        DischargeState::NotBinding => (
            RmmApplicability::NotApplicable,
            RmmCompliance::NotApplicable,
        ),
    };
    let current = matches!(
        compliance,
        RmmCompliance::Compliant | RmmCompliance::Tailored
    );
    let mut evidence_refs = Vec::new();
    let mut tailoring_description = None;
    let mut tailoring_rationale = None;
    if current {
        match &entry.fact {
            Some(DischargeFact::Direct(fact)) => evidence_refs.clone_from(&fact.evidence_refs),
            Some(DischargeFact::Disposition(fact)) => {
                tailoring_description = Some(format!(
                    "{} (approval {})",
                    fact.decision.as_str(),
                    fact.approval_ref
                ));
                tailoring_rationale = Some(fact.rationale.clone());
            }
            None => {}
        }
    }
    if compliance == RmmCompliance::NotApplicable {
        tailoring_description = Some("Not applicable per binding derivation".to_owned());
        tailoring_rationale = Some(if clause.reasons.is_empty() {
            "no reason given by the binding report".to_owned()
        } else {
            clause
                .reasons
                .iter()
                .map(|reason| reason.message.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        });
    }
    RmmRow {
        clause_id: entry.clause_id.clone(),
        force: entry.force,
        applicability,
        compliance,
        reason: entry.reason.clone(),
        evidence_refs,
        tailoring_description,
        tailoring_rationale,
    }
}

/// Render the matrix as deterministic Markdown.
#[must_use]
pub fn render_requirements_mapping(matrix: &RequirementsMapping) -> String {
    let mut lines = vec![
        format!(
            "# Requirements mapping matrix: {}",
            escape(&format!(
                "{}/{}@{}",
                matrix.clause_set.authority, matrix.clause_set.id, matrix.clause_set.version
            ))
        ),
        String::new(),
        format!("- Evaluated as of: {}", escape(&matrix.as_of)),
        String::new(),
    ];
    if matrix.rows.is_empty() {
        lines.push("_The binding report names no clauses._".to_owned());
        lines.push(String::new());
    } else {
        lines.push(
            "| Req# | Applicability | Compliance | Evidence | Tailoring Description | Tailoring Rationale |"
                .to_owned(),
        );
        lines.push("| --- | --- | --- | --- | --- | --- |".to_owned());
        for row in &matrix.rows {
            lines.push(table_row(row));
        }
        lines.push(String::new());
    }
    if !matrix.unused_facts.is_empty() {
        lines.push("## Unused facts".to_owned());
        lines.push(String::new());
        for fact in &matrix.unused_facts {
            lines.push(format!(
                "- {} ({}): {}",
                escape(&fact.clause_id),
                fact.kind.as_str(),
                fact.reason.as_str()
            ));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

fn table_row(row: &RmmRow) -> String {
    let compliance = match &row.reason {
        Some(reason) if row.compliance != RmmCompliance::Compliant => {
            format!("{}: {reason}", row.compliance.label())
        }
        _ => row.compliance.label().to_owned(),
    };
    let evidence = if row.evidence_refs.is_empty() {
        "-".to_owned()
    } else {
        row.evidence_refs.join(", ")
    };
    let cells = [
        row.clause_id.clone(),
        row.applicability.label().to_owned(),
        compliance,
        evidence,
        row.tailoring_description
            .clone()
            .unwrap_or_else(|| "-".to_owned()),
        row.tailoring_rationale
            .clone()
            .unwrap_or_else(|| "-".to_owned()),
    ];
    let cells: Vec<String> = cells.iter().map(|cell| escape(cell)).collect();
    format!("| {} |", cells.join(" | "))
}

/// Every string from a binding report or a stored fact goes through here, so
/// it renders as the literal text it is: control characters become spaces and
/// each CommonMark/GFM punctuation character that can start markup, HTML, a
/// link, an image, a table cell or a code span is backslash-escaped.
pub(super) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_control() {
            out.push(' ');
        } else if "\\`*_{}[]<>()#+!|~&$".contains(ch) {
            out.push('\\');
            out.push(ch);
        } else {
            out.push(ch);
        }
    }
    out
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
    use super::{
        RmmApplicability, RmmCompliance, build_requirements_mapping, render_requirements_mapping,
    };

    fn disposition(clause: &str) -> serde_json::Value {
        serde_json::json!({
            "kind": "disposition",
            "clauseId": clause,
            "decision": "accepted_risk",
            "rationale": "carried | for now\nsecond line",
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
             "reasons": [{"code": "missing", "message": "context lacks deployment"}],
             "expectedOutputs": []},
            {"clauseId": "SYN-005", "force": "permitted", "outcome": "not_binding",
             "reasons": [{"code": "a", "message": "no safety-critical software"},
                         {"code": "b", "message": "class D"}],
             "expectedOutputs": []},
            {"clauseId": "SYN-006", "force": "mandatory", "outcome": "binding",
             "reasons": [], "expectedOutputs": []},
            {"clauseId": "SYN-007", "force": "permitted", "outcome": "not_binding",
             "reasons": [], "expectedOutputs": []}
        ]);
        serde_json::from_value(value).unwrap()
    }

    fn matrix_for(facts: Vec<serde_json::Value>) -> super::RequirementsMapping {
        let mut request = request(facts);
        request.binding = wide_binding();
        let report = build_discharge_report(&request).unwrap();
        build_requirements_mapping(&request.binding, &report).unwrap()
    }

    #[trace("FR-135-AC-1", "TC-3200")]
    #[test]
    fn one_row_per_binding_clause_in_the_reports_order() {
        let matrix = matrix_for(vec![]);
        let ids: Vec<_> = matrix.rows.iter().map(|r| r.clause_id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "SYN-001", "SYN-002", "SYN-003", "SYN-004", "SYN-005", "SYN-006", "SYN-007"
            ]
        );
        let text = render_requirements_mapping(&matrix);
        assert!(
            text.contains("| Req# | Applicability | Compliance | Evidence | Tailoring Description | Tailoring Rationale |"),
            "{text}"
        );
    }

    #[trace("FR-135-AC-1", "TC-3200")]
    #[test]
    fn an_empty_binding_report_states_that_it_names_no_clauses() {
        let mut request = request(vec![]);
        request.binding.clauses.clear();
        let report = build_discharge_report(&request).unwrap();
        let matrix = build_requirements_mapping(&request.binding, &report).unwrap();
        assert!(matrix.rows.is_empty());
        let text = render_requirements_mapping(&matrix);
        assert!(
            text.contains("_The binding report names no clauses._"),
            "{text}"
        );
        assert!(!text.contains("| Req#"), "{text}");
    }

    #[trace("FR-135-AC-2", "TC-3201")]
    #[test]
    fn a_current_direct_fact_is_compliant_and_a_disposition_is_tailored_only() {
        let matrix = matrix_for(vec![direct(), disposition("SYN-002")]);
        let first = &matrix.rows[0];
        assert_eq!(first.compliance, RmmCompliance::Compliant);
        assert_eq!(first.evidence_refs, ["evidence://run/one"]);
        let second = &matrix.rows[1];
        assert_eq!(second.compliance, RmmCompliance::Tailored);
        assert_ne!(second.compliance, RmmCompliance::Compliant);
        assert!(second.evidence_refs.is_empty());
        assert_eq!(
            second.tailoring_description.as_deref(),
            Some("accepted_risk (approval approval://1)")
        );
        assert!(
            second
                .tailoring_rationale
                .as_deref()
                .unwrap()
                .contains("carried")
        );
    }

    fn at(mut fact: serde_json::Value, attested: &str, expires: &str) -> serde_json::Value {
        fact["attestation"]["attestedAt"] = attested.into();
        fact["attestation"]["expiresAt"] = expires.into();
        fact
    }

    #[trace("FR-135-AC-3", "TC-3202")]
    #[test]
    fn gaps_are_rows_and_a_fact_that_is_not_current_is_neither_compliant_nor_tailored() {
        let expired = at(
            direct(),
            "2026-08-01T00:00:00.000Z",
            "2026-08-10T00:00:00.000Z",
        );
        let expired_disposition = at(
            disposition("SYN-002"),
            "2026-08-01T00:00:00.000Z",
            "2026-08-10T00:00:00.000Z",
        );
        let mut not_yet = disposition("SYN-003");
        not_yet = at(
            not_yet,
            "2026-08-20T00:00:00.000Z",
            "2026-09-01T00:00:00.000Z",
        );
        let mut inverted = disposition("SYN-006");
        inverted = at(
            inverted,
            "2026-08-01T00:00:00.000Z",
            "2026-08-01T00:00:00.000Z",
        );
        let matrix = matrix_for(vec![expired, expired_disposition, not_yet, inverted]);
        let by = |id: &str| matrix.rows.iter().find(|r| r.clause_id == id).unwrap();
        for (id, why) in [
            ("SYN-001", "discharge fact is expired"),
            ("SYN-002", "discharge fact is expired"),
            ("SYN-003", "attestation is in the future"),
            ("SYN-006", "attestation expiry is not after attestation"),
        ] {
            let row = by(id);
            assert_eq!(row.compliance, RmmCompliance::AttestationNotCurrent, "{id}");
            assert_eq!(row.reason.as_deref(), Some(why), "{id}");
            assert!(row.evidence_refs.is_empty());
            assert!(row.tailoring_description.is_none());
        }
        assert_eq!(by("SYN-004").compliance, RmmCompliance::Undetermined);
        assert_eq!(by("SYN-004").applicability, RmmApplicability::Unresolved);
        let na = by("SYN-005");
        assert_eq!(na.compliance, RmmCompliance::NotApplicable);
        assert_eq!(na.applicability, RmmApplicability::NotApplicable);
        assert_eq!(
            na.tailoring_description.as_deref(),
            Some("Not applicable per binding derivation")
        );
        assert_eq!(
            na.tailoring_rationale.as_deref(),
            Some("no safety-critical software; class D")
        );
        assert_eq!(
            by("SYN-007").tailoring_rationale.as_deref(),
            Some("no reason given by the binding report")
        );
        let text = render_requirements_mapping(&matrix);
        assert!(!text.contains("| Compliant |"), "{text}");
        assert!(!text.contains("| Tailored |"), "{text}");
        let undischarged = matrix_for(vec![]);
        let text = render_requirements_mapping(&undischarged);
        assert!(
            text.contains("Not compliant - undischarged: no discharge fact"),
            "{text}"
        );
        assert!(
            text.contains("Undetermined: context lacks deployment"),
            "{text}"
        );
    }

    #[trace("FR-135-AC-4", "TC-3203")]
    #[test]
    fn unused_facts_follow_the_table_and_nothing_is_scored() {
        let matrix = matrix_for(vec![disposition("SYN-005"), disposition("SYN-999")]);
        assert_eq!(matrix.unused_facts.len(), 2);
        let text = render_requirements_mapping(&matrix);
        let table_end = text.rfind("| SYN-007").unwrap();
        let unused = text.find("## Unused facts").unwrap();
        assert!(unused > table_end, "{text}");
        assert!(
            text.contains("- SYN-999 (disposition): unknown_clause"),
            "{text}"
        );
        // A disposition on a not-binding clause is not tailoring.
        let na = matrix
            .rows
            .iter()
            .find(|r| r.clause_id == "SYN-005")
            .unwrap();
        assert_eq!(na.compliance, RmmCompliance::NotApplicable);
        assert!(
            !na.tailoring_rationale
                .as_deref()
                .unwrap()
                .contains("carried")
        );
        let json = serde_json::to_string(&matrix).unwrap();
        for word in ["score", "percent", "ratio", "compliantCount"] {
            assert!(!json.contains(word), "{json}");
        }
    }

    #[trace("FR-135-AC-5", "TC-3204")]
    #[test]
    fn the_render_is_the_same_across_independent_builds_whatever_the_fact_order() {
        let first = render_requirements_mapping(&matrix_for(vec![
            direct(),
            disposition("SYN-002"),
            disposition("SYN-999"),
        ]));
        let second = render_requirements_mapping(&matrix_for(vec![
            disposition("SYN-999"),
            disposition("SYN-002"),
            direct(),
        ]));
        assert_eq!(first, second);
    }

    #[trace("FR-135-AC-5", "TC-3204")]
    #[test]
    fn untrusted_text_renders_literally_in_every_position() {
        let mut value = serde_json::to_value(binding()).unwrap();
        value["clauseSet"]["authority"] = "ex\n# h".into();
        value["clauseSet"]["id"] = "set_1".into();
        value["clauses"] = serde_json::json!([
            {"clauseId": "C-1", "force": "mandatory", "outcome": "binding",
             "reasons": [], "expectedOutputs": []}
        ]);
        let mut request = request(vec![]);
        request.binding = serde_json::from_value(value).unwrap();
        let mut fact = disposition("C-1");
        fact["rationale"] =
            "![t](http://e/p.png) <img src=x> [Compliant](http://e)\nback\\slash `tick` | pipe"
                .into();
        fact["approvalRef"] = "ap://1".into();
        let mut stray = disposition("x\n## h");
        stray["approvalRef"] = "ap://2".into();
        request.facts = vec![fact, stray];
        let report = build_discharge_report(&request).unwrap();
        let text = render_requirements_mapping(
            &build_requirements_mapping(&request.binding, &report).unwrap(),
        );
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines[0],
            "# Requirements mapping matrix: ex \\# h/set\\_1@1.0.0"
        );
        assert_eq!(
            lines[6],
            "| C-1 | Applicable | Tailored | - | accepted\\_risk \\(approval ap://1\\) | \\!\\[t\\]\\(http://e/p.png\\) \\<img src=x\\> \\[Compliant\\]\\(http://e\\) back\\\\slash \\`tick\\` \\| pipe |"
        );
        assert_eq!(lines[10], "- x \\#\\# h (disposition): unknown_clause");
        assert_eq!(lines.len(), 11, "{text}");
    }
}
