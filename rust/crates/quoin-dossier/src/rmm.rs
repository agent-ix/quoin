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
            .map(|column| value_cell(&row[*column]))
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
