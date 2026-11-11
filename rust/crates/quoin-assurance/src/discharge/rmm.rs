// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The requirements mapping matrix: a read-only view of a discharge report
//! (FR-135).
//!
//! One row per clause of the binding report, in the report's order, so a gap
//! is a row and never an absent line. Every cell was decided in
//! [`super::build`]; this module only names it.

use std::collections::HashMap;

use quoin_quire_types::{ClauseBindingReport, ClauseForce, ClauseSetKey};
use serde::{Deserialize, Serialize};

use super::report::{
    ClauseDischarge, DischargeFact, DischargeReport, DischargeState, UnusedDischargeFact,
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
/// A clause the discharge report does not hold cannot occur when the report
/// was built from `binding`; it is shown as undischarged rather than dropped.
#[must_use]
pub fn build_requirements_mapping(
    binding: &ClauseBindingReport,
    report: &DischargeReport,
) -> RequirementsMapping {
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
        .map(|clause| match entries.get(clause.clause_id.as_str()) {
            Some(entry) => row(entry),
            None => RmmRow {
                clause_id: clause.clause_id.clone(),
                force: clause.force,
                applicability: RmmApplicability::Applicable,
                compliance: RmmCompliance::Undischarged,
                reason: Some("no discharge fact".to_owned()),
                evidence_refs: Vec::new(),
                tailoring_description: None,
                tailoring_rationale: None,
            },
        })
        .collect();
    RequirementsMapping {
        schema_version: RequirementsMappingSchemaVersion::V1,
        clause_set: report.clause_set.clone(),
        as_of: report.as_of.clone(),
        rows,
        unused_facts: report.unused_facts.clone(),
    }
}

fn row(entry: &ClauseDischarge) -> RmmRow {
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
            "# Requirements mapping matrix: {}/{}@{}",
            matrix.clause_set.authority, matrix.clause_set.id, matrix.clause_set.version
        ),
        String::new(),
        format!("- Evaluated as of: `{}`", matrix.as_of),
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
                "- `{}` ({}): {}",
                fact.clause_id,
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

/// A cell cannot split its row: `|` is escaped and line breaks become spaces.
fn escape(cell: &str) -> String {
    let mut out = String::with_capacity(cell.len());
    for ch in cell.chars() {
        match ch {
            '|' => out.push_str("\\|"),
            '\n' | '\r' => out.push(' '),
            other => out.push(other),
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
             "reasons": [], "expectedOutputs": []}
        ]);
        serde_json::from_value(value).unwrap()
    }

    fn matrix_for(facts: Vec<serde_json::Value>) -> super::RequirementsMapping {
        let mut request = request(facts);
        request.binding = wide_binding();
        let report = build_discharge_report(&request).unwrap();
        build_requirements_mapping(&request.binding, &report)
    }

    #[trace("FR-135-AC-1", "TC-3200")]
    #[test]
    fn one_row_per_binding_clause_in_the_reports_order() {
        let matrix = matrix_for(vec![]);
        let ids: Vec<_> = matrix.rows.iter().map(|r| r.clause_id.as_str()).collect();
        assert_eq!(ids, ["SYN-001", "SYN-002", "SYN-003", "SYN-004", "SYN-005"]);
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
        let matrix = build_requirements_mapping(&request.binding, &report);
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

    #[trace("FR-135-AC-3", "TC-3202")]
    #[test]
    fn gaps_are_rows_and_an_expired_fact_is_neither_compliant_nor_tailored() {
        let mut expired = direct();
        expired["attestation"]["expiresAt"] = "2026-08-10T00:00:00.000Z".into();
        let mut expired_disposition = disposition("SYN-002");
        expired_disposition["attestation"]["expiresAt"] = "2026-08-10T00:00:00.000Z".into();
        let matrix = matrix_for(vec![expired, expired_disposition]);
        let by = |id: &str| matrix.rows.iter().find(|r| r.clause_id == id).unwrap();
        for id in ["SYN-001", "SYN-002"] {
            let row = by(id);
            assert_eq!(row.compliance, RmmCompliance::AttestationNotCurrent);
            assert!(row.reason.is_some());
            assert!(row.evidence_refs.is_empty());
            assert!(row.tailoring_description.is_none());
        }
        assert_eq!(by("SYN-003").compliance, RmmCompliance::Undischarged);
        assert_eq!(by("SYN-004").compliance, RmmCompliance::Undetermined);
        assert_eq!(by("SYN-004").applicability, RmmApplicability::Unresolved);
        assert_eq!(by("SYN-005").compliance, RmmCompliance::NotApplicable);
        assert_eq!(by("SYN-005").applicability, RmmApplicability::NotApplicable);
        let text = render_requirements_mapping(&matrix);
        assert!(
            text.contains("Not compliant - undischarged: no discharge fact"),
            "{text}"
        );
        assert!(
            text.contains("Undetermined: context lacks deployment"),
            "{text}"
        );
        assert!(!text.contains("| Compliant |"), "{text}");
    }

    #[trace("FR-135-AC-4", "TC-3203")]
    #[test]
    fn unused_facts_follow_the_table_and_nothing_is_scored() {
        let matrix = matrix_for(vec![disposition("SYN-005"), disposition("SYN-999")]);
        assert_eq!(matrix.unused_facts.len(), 2);
        let text = render_requirements_mapping(&matrix);
        let table_end = text.rfind("| SYN-005").unwrap();
        let unused = text.find("## Unused facts").unwrap();
        assert!(unused > table_end, "{text}");
        assert!(
            text.contains("- `SYN-999` (disposition): unknown_clause"),
            "{text}"
        );
        let json = serde_json::to_string(&matrix).unwrap();
        for word in ["score", "percent", "ratio", "compliantCount"] {
            assert!(!json.contains(word), "{json}");
        }
    }

    #[trace("FR-135-AC-5", "TC-3204")]
    #[test]
    fn the_render_is_stable_and_a_cell_cannot_split_its_row() {
        let matrix = matrix_for(vec![direct(), disposition("SYN-002")]);
        let text = render_requirements_mapping(&matrix);
        assert_eq!(text, render_requirements_mapping(&matrix));
        let row = text.lines().find(|l| l.starts_with("| SYN-002")).unwrap();
        assert!(row.contains("carried \\| for now second line"), "{row}");
        // Six cells: seven unescaped separators.
        assert_eq!(row.replace("\\|", "").matches('|').count(), 7, "{row}");
    }
}
