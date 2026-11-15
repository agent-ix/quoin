// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! SRS document generator (FR-192, PLAT-1437).
//!
//! Lists every requirement in scope with id, title, statement, criteria and
//! recorded criticality.

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::order::dossier_cmp;
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};

/// The SRS generator.
pub struct SrsGenerator;

impl Generator for SrsGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "srs",
            id: "DOS-SRS",
            title: "Software requirements specification",
            needs_as_of: false,
        }
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        // Read requirements from the scope
        let modules = inputs
            .modules
            .iter()
            .map(|path| quoin_quire::ids::ModuleRoot::open(path))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|error| SourceFailure {
                source: "modules".to_owned(),
                detail: format!("failed to open module: {error}"),
            })?;

        let corpus = quoin_quire::records::read(
            &inputs.scope,
            &quoin_quire::ModuleSelection::Closed(modules),
            &["FR", "NFR", "StR"],
        )
        .map_err(|error| SourceFailure {
            source: "requirements".to_owned(),
            detail: format!("failed to read requirement documents: {error}"),
        })?;

        let mut requirements: Vec<_> = corpus
            .documents
            .iter()
            .filter(|doc| ["FR", "NFR", "StR"].contains(&doc.kind.as_str()))
            .collect();

        requirements.sort_by(|a, b| dossier_cmp(&a.id, &b.id));

        let mut gaps = Vec::new();
        let mut body = String::from("# Software requirements specification\n\n");

        if requirements.is_empty() {
            gaps.push(Gap::new(
                GapKind::NoRequirements,
                "this document",
                "the scope holds no requirement",
            ));
        }

        for req in &requirements {
            body.push_str(&format!("## {}: {}\n\n", req.id, title_of(req)));

            // Statement section
            let statement = statement_of(req);
            if !statement.is_empty() {
                body.push_str(&statement);
                body.push_str("\n\n");
            }

            // Criticality
            let criticality = criticality_of(req);
            if let Some(level) = criticality {
                body.push_str(&format!("Criticality: {level}\n\n"));
            } else {
                body.push_str("Criticality: unassigned\n\n");
                gaps.push(
                    Gap::new(
                        GapKind::CriticalityUnassigned,
                        &req.id,
                        "has no criticality",
                    )
                    .for_requirement(&req.id),
                );
            }

            // Criteria table
            let criteria = criteria_of(req);
            if criteria.is_empty() {
                gaps.push(
                    Gap::new(
                        GapKind::NoAcceptanceCriteria,
                        &req.id,
                        "has no acceptance criterion",
                    )
                    .for_requirement(&req.id),
                );
            } else {
                body.push_str("| Criterion | Text |\n");
                body.push_str("|-----------|------|\n");
                for (id, text) in &criteria {
                    let escaped_id = markdown_escape(id);
                    let escaped_text = markdown_escape(text);
                    body.push_str(&format!("| {escaped_id} | {escaped_text} |\n"));
                }
                body.push_str("\n");
            }
        }

        Ok(Generated { body, gaps })
    }
}

/// Extract the title from a requirement document.
fn title_of(doc: &quoin_quire::records::Document) -> String {
    doc.frontmatter
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_owned()
}

/// Extract the statement from a requirement document.
fn statement_of(doc: &quoin_quire::records::Document) -> String {
    let section_name = match doc.kind.as_str() {
        "FR" => "Description",
        "NFR" => "Statement",
        "StR" => "Stakeholder Need",
        _ => return String::new(),
    };

    doc.sections
        .get(section_name)
        .map_or_else(String::new, |text| text.trim().to_owned())
}

/// Extract criticality from frontmatter.
fn criticality_of(doc: &quoin_quire::records::Document) -> Option<String> {
    doc.frontmatter
        .get("criticality")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
}

/// Extract criteria from a requirement document.
fn criteria_of(doc: &quoin_quire::records::Document) -> Vec<(String, String)> {
    let table_name = match doc.kind.as_str() {
        "FR" | "NFR" => "Acceptance Criteria",
        "StR" => "Validation Criteria",
        _ => return Vec::new(),
    };

    let Some(table) = doc.tables.get(table_name) else {
        return Vec::new();
    };

    // Find the ID and Criteria/Text columns
    let id_col = table
        .headers
        .iter()
        .position(|h| h.to_lowercase() == "id" || h.to_lowercase() == "criteria");
    let text_col = table.headers.iter().position(|h| {
        let lower = h.to_lowercase();
        lower == "text" || lower == "criteria" || lower == "verification"
    });

    let (Some(id_idx), Some(text_idx)) = (id_col, text_col) else {
        return Vec::new();
    };

    table
        .rows
        .iter()
        .filter_map(|row| {
            let id = row.get(id_idx)?;
            let text = row.get(text_idx)?;
            if id.trim().is_empty() {
                return None;
            }
            Some((id.trim().to_owned(), text.trim().to_owned()))
        })
        .collect()
}

/// Escape Markdown table cell content so it cannot break table syntax.
fn markdown_escape(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
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

    #[trace("FR-192")]
    #[test]
    fn srs_generator_has_correct_spec() {
        let generator = SrsGenerator;
        let spec = generator.spec();
        assert_eq!(spec.kind, "srs");
        assert_eq!(spec.id, "DOS-SRS");
        assert_eq!(spec.title, "Software requirements specification");
        assert!(!spec.needs_as_of);
    }

    #[trace("FR-192")]
    #[test]
    fn markdown_escape_replaces_pipe_and_newline() {
        assert_eq!(markdown_escape("text|with|pipes"), "text\\|with\\|pipes");
        assert_eq!(markdown_escape("text\nwith\nnewlines"), "text with newlines");
        assert_eq!(markdown_escape("text|and\nnewline"), "text\\|and newline");
    }
}
