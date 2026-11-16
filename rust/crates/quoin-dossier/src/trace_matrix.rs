// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The evidence-backed dossier trace matrix (FR-194).

use std::fmt::Write as _;

use quoin_assurance::matrix::{EvidenceStatus, MatrixOutput, design_cell};
use quoin_quire::records::Document;

use crate::markdown::{cell, line, prose, unusable_input};
use crate::order::dossier_cmp;
use crate::srs::{criteria_of, criticality_of};
use crate::{DocumentSpec, Gap, GapKind, Generated, Generator, Inputs, SourceFailure};

pub(crate) struct TraceMatrixGenerator(
    pub(crate) fn(&Inputs) -> Result<MatrixOutput, SourceFailure>,
);

impl Generator for TraceMatrixGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "trace-matrix",
            id: "DOS-TRACE",
            title: "Trace matrix",
            needs_as_of: false,
        }
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        let matrix = (self.0)(inputs)?;
        let modules = inputs
            .modules
            .iter()
            .map(quoin_quire::ModuleRoot::open)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| SourceFailure {
                source: "requirements".to_owned(),
                detail: error.to_string(),
            })?;
        let selection = if modules.is_empty() {
            quoin_quire::ModuleSelection::ScopeOrAmbient
        } else {
            quoin_quire::ModuleSelection::Closed(modules)
        };
        let corpus = quoin_quire::records::read(&inputs.scope, &selection, &["FR", "NFR", "StR"])
            .map_err(|error| SourceFailure {
            source: "requirements".to_owned(),
            detail: error.to_string(),
        })?;
        if let Some(failure) = unusable_input("requirements", &corpus) {
            return Err(failure);
        }
        Ok(render(&matrix, &corpus.documents))
    }
}

fn render(matrix: &MatrixOutput, documents: &[Document]) -> Generated {
    let mut body = String::from("# Trace matrix\n\n");
    let mut gaps = Vec::new();
    if matrix
        .requirements
        .iter()
        .all(|requirement| requirement.criteria.is_empty())
    {
        let reason = matrix
            .reason
            .as_deref()
            .unwrap_or("the matrix has no criteria");
        let _ = write!(body, "{}\n\n", prose(reason));
        gaps.push(Gap::new(GapKind::NoRequirements, "this document", reason));
        return Generated { body, gaps };
    }
    let mut requirements: Vec<_> = matrix.requirements.iter().collect();
    requirements.sort_by(|a, b| dossier_cmp(&a.id, &b.id));
    for requirement in requirements {
        let document = documents.iter().find(|doc| doc.id == requirement.id);
        let criticality = document.and_then(criticality_of);
        let criteria = document.map(criteria_of).unwrap_or_default();
        let _ = write!(
            body,
            "## {}\n\nCriticality: {}\n\n",
            line(&requirement.id),
            line(criticality.as_deref().unwrap_or("unassigned"))
        );
        if criticality.is_none() {
            gaps.push(
                Gap::new(
                    GapKind::CriticalityUnassigned,
                    &requirement.id,
                    "has no criticality",
                )
                .for_requirement(&requirement.id),
            );
        }
        if requirement.design.as_ref().is_some_and(|blocks| {
            blocks.iter().any(|block| {
                block.get("state").and_then(serde_json::Value::as_str) == Some("uncovered")
            })
        }) {
            gaps.push(
                Gap::new(
                    GapKind::NoDesignElement,
                    &requirement.id,
                    "has no design element",
                )
                .for_requirement(&requirement.id),
            );
        }
        body.push_str("| Criterion | Statement | Design | Method | Static status | Evidence status |\n| --- | --- | --- | --- | --- | --- |\n");
        let mut rows: Vec<_> = requirement.criteria.iter().collect();
        rows.sort_by(|a, b| dossier_cmp(&a.id, &b.id));
        for criterion in rows {
            let statement = criteria
                .iter()
                .find(|(id, _)| id == &criterion.id)
                .map_or("", |(_, statement)| statement.as_str());
            let _ = writeln!(
                body,
                "| {} | {} | {} | {} | {} | {} |",
                cell(&criterion.id),
                cell(statement),
                cell(&design_cell(requirement, criterion)),
                cell(criterion.method.as_deref().unwrap_or("—")),
                cell(&criterion.static_status),
                cell(criterion.evidence_status.as_str())
            );
            if criterion.static_status != "tagged" {
                gaps.push(
                    Gap::new(
                        GapKind::UnbackedCriterion,
                        &criterion.id,
                        &criterion.static_status,
                    )
                    .for_requirement(&requirement.id)
                    .for_criterion(&criterion.id),
                );
            }
            if criterion.evidence_status != EvidenceStatus::Bound {
                gaps.push(
                    Gap::new(
                        GapKind::NoEvidence,
                        &criterion.id,
                        criterion.evidence_status.as_str(),
                    )
                    .for_requirement(&requirement.id)
                    .for_criterion(&criterion.id),
                );
            }
        }
        body.push('\n');
    }
    Generated { body, gaps }
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
    use quoin_assurance::matrix::{MatrixInput, build};
    use quoin_quire::records::{Contract, Table};
    use serde_json::json;

    fn matrix() -> MatrixOutput {
        let input: MatrixInput = serde_json::from_value(json!({
            "design_coverage": [
                {"document": "FR-10.md", "id": "FR-10", "criteria": [
                    {"id": "FR-10-AC-1", "status": "untagged", "binders": [], "method": "Test"}],
                 "design": [{"check": "allocated", "state": "uncovered", "elements": []}]},
                {"document": "FR-2.md", "id": "FR-2", "criteria": [
                    {"id": "FR-2-AC-1", "status": "tagged", "binders": [], "method": "Analysis", "design_elements": [{"id":"DE-1"}]}],
                 "design": [{"state":"covered", "elements":[{"id":"DE-1", "kind":"component"}]}]}
            ],
            "bindings": {"schemaVersion": 1, "bindings": [{"obligation":"FR-2-AC-1", "suite":"unit", "symbols":[]}]},
            "audit": {"findings":[], "healthy":["FR-2-AC-1"], "unevaluated":[]}
        })).unwrap();
        build(&input).unwrap()
    }

    fn requirement(id: &str, criticality: Option<&str>) -> Document {
        Document {
            path: format!("{id}.md").into(),
            id: id.to_owned(),
            kind: "FR".to_owned(),
            frontmatter: criticality
                .map(|value| ("criticality".to_owned(), json!(value)))
                .into_iter()
                .collect(),
            tables: [(
                "Acceptance Criteria".to_owned(),
                Table {
                    headers: vec!["ID".to_owned(), "Criteria".to_owned()],
                    rows: vec![vec![
                        format!("{id}-AC-1"),
                        "statement | line\n`tick`".to_owned(),
                    ]],
                },
            )]
            .into_iter()
            .collect(),
            sections: std::collections::BTreeMap::default(),
            edges: vec![],
            raw: String::new(),
            contract: Contract::Satisfied,
        }
    }

    #[trace(
        "FR-194-AC-1",
        "FR-194-AC-2",
        "FR-194-AC-4",
        "FR-194-AC-5",
        "FR-194-AC-6"
    )]
    #[test]
    fn rows_preserve_the_real_matrix_builder_values_and_join_authored_statements() {
        let source = matrix();
        let generated = render(
            &source,
            &[
                requirement("FR-10", None),
                requirement("FR-2", Some("high")),
            ],
        );
        let rows: Vec<_> = generated
            .body
            .lines()
            .filter(|line| line.starts_with("| FR-"))
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            "| FR-2-AC-1 | statement \\| line \\`tick\\` | DE-1 (component) | Analysis | tagged | bound |"
        );
        assert_eq!(
            rows[1],
            "| FR-10-AC-1 | statement \\| line \\`tick\\` | no design element (allocated) | Test | untagged | undischarged |"
        );
        for requirement in &source.requirements {
            for criterion in &requirement.criteria {
                let row = rows.iter().find(|row| row.contains(&criterion.id)).unwrap();
                assert!(row.ends_with(&format!(
                    "| {} | {} |",
                    criterion.static_status,
                    criterion.evidence_status.as_str()
                )));
            }
        }
        assert!(generated.body.contains("Criticality: high"));
        assert!(generated.body.contains("Criticality: unassigned"));
        for kind in [
            GapKind::CriticalityUnassigned,
            GapKind::NoDesignElement,
            GapKind::UnbackedCriterion,
            GapKind::NoEvidence,
        ] {
            assert_eq!(
                generated
                    .gaps
                    .iter()
                    .filter(|gap| gap.gap_kind == kind)
                    .count(),
                1,
                "{kind:?}"
            );
        }
        let gap = generated
            .gaps
            .iter()
            .find(|gap| gap.gap_kind == GapKind::NoEvidence)
            .unwrap();
        assert_eq!(
            (&*gap.subject, &*gap.detail),
            ("FR-10-AC-1", "undischarged")
        );
    }

    #[trace("FR-194-AC-3")]
    #[test]
    fn an_empty_matrix_preserves_its_reason_as_a_gap() {
        let generated = render(
            &MatrixOutput {
                requirements: vec![],
                reason: Some("no selected criteria".to_owned()),
                unclassified: None,
            },
            &[],
        );
        assert!(generated.body.contains("no selected criteria"));
        assert_eq!(
            generated.gaps,
            [Gap::new(
                GapKind::NoRequirements,
                "this document",
                "no selected criteria"
            )]
        );
    }
}
