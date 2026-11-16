// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The dossier requirements mapping matrix (FR-195).

use std::fmt::Write as _;

use quoin_assurance::{RequirementsMapping, RmmCompliance};
use serde::Serialize;
use serde_json::Value;

use crate::markdown::{cell, line};
use crate::{DocumentSpec, Gap, GapKind, Generated, Generator, Inputs, RmmSource, SourceFailure};

pub(crate) struct RmmGenerator(pub(crate) fn(&Inputs) -> Result<RmmSource, SourceFailure>);

impl Generator for RmmGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "rmm",
            id: "DOS-RMM",
            title: "Requirements mapping matrix",
            needs_as_of: true,
        }
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        if inputs.clause_bindings.is_empty() && inputs.pack.is_none() {
            return Ok(Generated {
                body: "# Requirements mapping matrix\n\n".to_owned(),
                gaps: vec![Gap::new(
                    GapKind::NoClauseBinding,
                    "this document",
                    "no --clause-binding or --pack was supplied",
                )],
            });
        }
        render(&(self.0)(inputs)?)
    }
}

fn failure(error: serde_json::Error) -> SourceFailure {
    SourceFailure {
        source: "discharge".to_owned(),
        detail: error.to_string(),
    }
}

fn value_cell(value: &Value) -> String {
    cell(&match value {
        Value::Null => "(none)".to_owned(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    })
}

fn table<T: Serialize>(
    body: &mut String,
    columns: &[&str],
    rows: &[T],
) -> Result<(), SourceFailure> {
    let _ = writeln!(body, "| {} |", columns.join(" | "));
    let _ = writeln!(body, "| {} |", vec!["---"; columns.len()].join(" | "));
    for row in rows {
        let row = serde_json::to_value(row).map_err(failure)?;
        let cells: Vec<_> = columns
            .iter()
            .map(|column| value_cell(row.get(*column).unwrap_or(&Value::Null)))
            .collect();
        let _ = writeln!(body, "| {} |", cells.join(" | "));
    }
    body.push('\n');
    Ok(())
}

fn key_text(set: &quoin_quire_types::ClauseSetKey) -> String {
    format!("{}/{}@{}", set.authority, set.id, set.version)
}

fn matrix(
    body: &mut String,
    gaps: &mut Vec<Gap>,
    matrix: &RequirementsMapping,
) -> Result<(), SourceFailure> {
    let _ = write!(body, "## {}\n\n", line(&key_text(&matrix.clause_set)));
    table(
        body,
        &[
            "clauseId",
            "force",
            "applicability",
            "compliance",
            "reason",
            "evidenceRefs",
            "tailoringDescription",
            "tailoringRationale",
        ],
        &matrix.rows,
    )?;
    for row in &matrix.rows {
        match row.compliance {
            RmmCompliance::Undischarged
            | RmmCompliance::AttestationNotCurrent
            | RmmCompliance::TemporaryExceptionExpired
            | RmmCompliance::Undetermined => {
                let compliance = serde_json::to_value(row.compliance).map_err(failure)?;
                let detail = format!(
                    "{}: {}",
                    compliance.as_str().unwrap_or_default(),
                    row.reason.as_deref().unwrap_or("(none)")
                );
                gaps.push(
                    Gap::new(GapKind::ClauseOpen, &row.clause_id, detail)
                        .for_clause(&row.clause_id),
                );
            }
            RmmCompliance::Compliant | RmmCompliance::Tailored | RmmCompliance::NotApplicable => {}
        }
    }
    if !matrix.unused_facts.is_empty() {
        body.push_str("### unusedFacts\n\n");
        table(body, &["clauseId", "kind", "reason"], &matrix.unused_facts)?;
    }
    Ok(())
}

fn render(source: &RmmSource) -> Result<Generated, SourceFailure> {
    let mut body = String::from("# Requirements mapping matrix\n\n");
    let mut gaps = Vec::new();
    match source {
        RmmSource::Bindings(matrices) => {
            if let Some(first) = matrices.first() {
                let _ = write!(body, "As of: {}\n\n", line(&first.as_of));
            }
            for entry in matrices {
                matrix(&mut body, &mut gaps, entry)?;
            }
        }
        RmmSource::Pack(pack) => {
            let _ = write!(
                body,
                "As of: {}\n\n## Context\n\n| Key | Value |\n| --- | --- |\n",
                line(&pack.as_of)
            );
            for (key, value) in &pack.context {
                let _ = writeln!(body, "| {} | {} |", cell(key), cell(value));
            }
            body.push_str("\n## Index\n\n| Clause set | Rows |\n| --- | --- |\n");
            for entry in &pack.index {
                let _ = writeln!(
                    body,
                    "| {} | {} |",
                    cell(&key_text(&entry.clause_set)),
                    entry.row_count
                );
            }
            body.push('\n');
            for entry in &pack.index {
                let matrix = pack
                    .matrices
                    .iter()
                    .find(|matrix| matrix.clause_set == entry.clause_set)
                    .ok_or_else(|| SourceFailure {
                        source: "discharge".to_owned(),
                        detail: format!(
                            "pack index entry {} has no matrix",
                            key_text(&entry.clause_set)
                        ),
                    })?;
                self::matrix(&mut body, &mut gaps, matrix)?;
            }
        }
    }
    Ok(Generated { body, gaps })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use quoin_assurance::{
        PackIndexEntry, PackRequirementsMapping, PackRequirementsMappingSchemaVersion,
        build_requirements_mapping, discharge_report_from_store,
    };
    use quoin_evidence::source::MemoryEvidence;
    use quoin_quire_types::ClauseBindingReport;
    use serde_json::json;

    fn mapping(id: &str) -> RequirementsMapping {
        let binding: ClauseBindingReport = serde_json::from_value(json!({
            "schemaVersion":"clause-binding-v1", "clauseSet":{"authority":"example.invalid","id":id,"version":"1"},
            "context":{}, "clauses":[
                {"clauseId":"C-2", "force":"mandatory", "outcome":"binding", "reasons":[], "expectedOutputs":[]},
                {"clauseId":"C-1", "force":"permitted", "outcome":"not_binding", "reasons":[], "expectedOutputs":[]}
            ]
        })).unwrap();
        let report =
            discharge_report_from_store(&MemoryEvidence::new(), &binding, "2026-11-15T00:00:00Z")
                .unwrap();
        build_requirements_mapping(&binding, &report).unwrap()
    }

    #[trace("FR-195-AC-1", "FR-195-AC-6", "FR-195-AC-9")]
    #[test]
    fn binding_order_and_every_source_member_are_preserved() {
        let mut first = mapping("z");
        first.rows[0].reason = Some("reason | line\n`tick`".to_owned());
        first.rows[0].evidence_refs = vec!["evidence://a|b\n`c`".to_owned()];
        first.rows[0].tailoring_description = Some("decision".to_owned());
        first.rows[0].tailoring_rationale = Some("rationale".to_owned());
        first.unused_facts = serde_json::from_value(
            json!([{"clauseId":"C-9", "kind":"direct", "reason":"unknown_clause"}]),
        )
        .unwrap();
        let second = mapping("a");
        let rendered = render(&RmmSource::Bindings(vec![first.clone(), second.clone()])).unwrap();
        assert!(rendered.body.contains("As of: 2026-11-15T00:00:00Z"));
        assert!(rendered.body.contains("| C-9 | direct | unknown_clause |"));
        let rows: Vec<_> = rendered
            .body
            .lines()
            .filter(|line| line.starts_with("| C-"))
            .collect();
        assert_eq!(
            rows[0],
            "| C-2 | mandatory | applicable | undischarged | reason \\| line \\`tick\\` | [\"evidence://a\\|b\\\\n\\`c\\`\"] | decision | rationale |"
        );
        assert!(rows[1].starts_with("| C-1 | permitted | not_applicable | not_applicable |"));
        assert_eq!(rendered.body.matches("| C-2 |").count(), 2);
        let reverse = render(&RmmSource::Bindings(vec![second, first])).unwrap();
        let (prefix, rest) = rendered.body.split_once("## example.invalid/z@1").unwrap();
        let (z, a) = rest.split_once("## example.invalid/a@1").unwrap();
        assert_eq!(
            reverse.body,
            format!("{prefix}## example.invalid/a@1{a}## example.invalid/z@1{z}")
        );
    }

    #[trace("FR-195-AC-2")]
    #[test]
    fn pack_context_and_index_precede_tables_in_index_order() {
        let z = mapping("z");
        let a = mapping("a");
        let pack = PackRequirementsMapping {
            schema_version: PackRequirementsMappingSchemaVersion::V1,
            pack: "fixture".to_owned(),
            as_of: z.as_of.clone(),
            context: [("deployment".to_owned(), "test".to_owned())]
                .into_iter()
                .collect(),
            index: [&z, &a]
                .into_iter()
                .map(|matrix| PackIndexEntry {
                    clause_set: matrix.clause_set.clone(),
                    row_count: matrix.rows.len(),
                })
                .collect(),
            matrices: vec![a, z],
        };
        let generated = render(&RmmSource::Pack(pack)).unwrap();
        let positions: Vec<_> = [
            "## Context",
            "## Index",
            "## example.invalid/z@1",
            "## example.invalid/a@1",
        ]
        .map(|text| generated.body.find(text).unwrap())
        .to_vec();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(generated.body.contains("| deployment | test |"));
    }

    #[trace("FR-195-AC-5", "FR-195-AC-8")]
    #[test]
    fn compliance_alone_decides_open_gaps_even_with_empty_evidence() {
        let mut mapping = mapping("set");
        let template = mapping.rows[0].clone();
        let states = [
            RmmCompliance::Undischarged,
            RmmCompliance::AttestationNotCurrent,
            RmmCompliance::TemporaryExceptionExpired,
            RmmCompliance::Undetermined,
            RmmCompliance::Compliant,
            RmmCompliance::Tailored,
            RmmCompliance::NotApplicable,
        ];
        mapping.rows = states
            .into_iter()
            .enumerate()
            .map(|(index, state)| {
                let mut row = template.clone();
                row.clause_id = format!("C-{index}");
                row.compliance = state;
                row.reason = Some("recorded reason".to_owned());
                row
            })
            .collect();
        let rendered = render(&RmmSource::Bindings(vec![mapping])).unwrap();
        assert_eq!(rendered.gaps.len(), 4);
        for (gap, status) in rendered.gaps.iter().zip([
            "undischarged",
            "attestation_not_current",
            "temporary_exception_expired",
            "undetermined",
        ]) {
            assert_eq!(gap.gap_kind, GapKind::ClauseOpen);
            assert_eq!(gap.detail, format!("{status}: recorded reason"));
        }
        assert_eq!(
            rendered
                .gaps
                .iter()
                .map(|gap| gap.subject.as_str())
                .collect::<Vec<_>>(),
            ["C-0", "C-1", "C-2", "C-3"]
        );
    }
}
