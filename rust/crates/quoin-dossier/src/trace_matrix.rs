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
        let corpus = quoin_quire::records::read(
            &inputs.scope,
            &quoin_quire::ModuleSelection::Closed(modules),
            &["FR", "NFR", "StR"],
        )
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
