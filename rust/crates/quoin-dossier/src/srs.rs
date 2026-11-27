// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! SRS document generator (FR-192, PLAT-1437).
//!
//! Lists every requirement in scope with id, title, statement, criteria and
//! recorded criticality.

use std::fmt::Write as _;

use quoin_quire::records::Document;
use serde_json::Value;

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::markdown::{cell, line, prose, unusable_input};
use crate::order::dossier_cmp;
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};

/// The requirement types the SRS lists.
pub(crate) const REQUIREMENT_TYPES: [&str; 3] = ["FR", "NFR", "StR"];

/// The SRS generator.
pub(crate) struct SrsGenerator;

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
        let modules = inputs
            .modules
            .iter()
            .map(quoin_quire::ids::ModuleRoot::open)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| SourceFailure {
                source: "modules".to_owned(),
                detail: format!("failed to open module: {error}"),
            })?;

        let corpus = quoin_quire::records::read(
            &inputs.scope,
            &quoin_quire::ModuleSelection::Closed(modules),
            &REQUIREMENT_TYPES,
        )
        .map_err(|error| SourceFailure {
            source: "requirements".to_owned(),
            detail: format!("failed to read requirement documents: {error}"),
        })?;

        if let Some(failure) = unusable_input("requirements", &corpus) {
            return Err(failure);
        }

        Ok(render(&corpus.documents))
    }
}

/// The SRS over `documents`; documents of other types are ignored.
fn render(documents: &[Document]) -> Generated {
    let mut requirements: Vec<&Document> = documents
        .iter()
        .filter(|doc| REQUIREMENT_TYPES.contains(&doc.kind.as_str()))
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

    for req in requirements {
        let _ = write!(body, "## {}: {}\n\n", line(&req.id), line(&title_of(req)));

        let statement = statement_of(req);
        if !statement.is_empty() {
            body.push_str(&prose(&statement));
            body.push_str("\n\n");
        }

        if let Some(level) = criticality_of(req) {
            let _ = write!(body, "Criticality: {}\n\n", line(&level));
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

        if let Some(class) = software_safety_class_of(req) {
            let _ = write!(body, "Software safety class: {}\n\n", line(&class));
        } else {
            body.push_str("Software safety class: unassigned\n\n");
            gaps.push(safety_class_gap(&req.id));
        }

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
            body.push_str("| Criterion | Text |\n|-----------|------|\n");
            for (id, text) in &criteria {
                let _ = writeln!(body, "| {} | {} |", cell(id), cell(text));
            }
            body.push('\n');
        }
    }

    Generated {
        body,
        document_version: None,
        gaps,
    }
}

/// The title from frontmatter; empty when absent.
pub(crate) fn title_of(doc: &Document) -> String {
    doc.frontmatter
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// The statement section a requirement type carries.
fn statement_of(doc: &Document) -> String {
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

/// The criticality exactly as recorded; `None` when the key is absent, null or
/// blank. A level is never inferred or defaulted.
pub(crate) fn criticality_of(doc: &Document) -> Option<String> {
    let recorded = match doc.frontmatter.get("criticality")? {
        Value::Null => return None,
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    (!recorded.trim().is_empty()).then_some(recorded)
}

/// The software safety class exactly as recorded in the requirement's own
/// frontmatter; `None` when the key is absent, null or blank. A class is never
/// inferred, defaulted or inherited from criticality, an allocated part or
/// another requirement (spec-artifacts-iso FR-009).
pub(crate) fn software_safety_class_of(doc: &Document) -> Option<String> {
    recorded_text(doc.software_safety_class()?)
}

/// A recorded scalar as written: a string is its own text, any other value its
/// JSON. `None` when blank.
pub(crate) fn recorded_text(value: &Value) -> Option<String> {
    let text = match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    (!text.trim().is_empty()).then_some(text)
}

/// The gap for a requirement that records no software safety class.
pub(crate) fn safety_class_gap(requirement: &str) -> Gap {
    Gap::new(
        GapKind::SafetyClassUnassigned,
        requirement,
        "records no software safety class",
    )
    .for_requirement(requirement)
}

/// The criteria table of a requirement as `(id, text)` pairs.
pub(crate) fn criteria_of(doc: &Document) -> Vec<(String, String)> {
    let table_name = match doc.kind.as_str() {
        "FR" | "NFR" => "Acceptance Criteria",
        "StR" => "Validation Criteria",
        _ => return Vec::new(),
    };
    let Some(table) = doc.tables.get(table_name) else {
        return Vec::new();
    };

    let column = |names: &[&str]| {
        names.iter().find_map(|name| {
            table
                .headers
                .iter()
                .position(|header| header.trim().eq_ignore_ascii_case(name))
        })
    };
    let (Some(id_idx), Some(text_idx)) = (
        column(&["id"]),
        column(&["criteria", "text", "statement", "verification"]),
    ) else {
        return Vec::new();
    };

    table
        .rows
        .iter()
        .filter_map(|row| {
            let id = row.get(id_idx)?.trim();
            let text = row.get(text_idx)?.trim();
            (!id.is_empty()).then(|| (id.to_owned(), text.to_owned()))
        })
        .collect()
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
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use quoin_quire::records::{Contract, Corpus, Table, Unreadable};
    use serde_json::Map;

    use super::*;
    use ix_trace_rs::trace;

    fn requirement(
        kind: &str,
        id: &str,
        title: &str,
        criticality: Option<&str>,
        criteria: &[(&str, &str)],
    ) -> Document {
        let mut frontmatter = Map::new();
        frontmatter.insert("title".to_owned(), Value::String(title.to_owned()));
        frontmatter.insert(
            "software_safety_class".to_owned(),
            Value::String("class-1".to_owned()),
        );
        if let Some(level) = criticality {
            frontmatter.insert("criticality".to_owned(), Value::String(level.to_owned()));
        }
        let (section, table) = if kind == "StR" {
            ("Stakeholder Need", "Validation Criteria")
        } else if kind == "NFR" {
            ("Statement", "Acceptance Criteria")
        } else {
            ("Description", "Acceptance Criteria")
        };
        let mut sections = BTreeMap::new();
        sections.insert(section.to_owned(), format!("Statement of {id}.\n"));
        let mut tables = BTreeMap::new();
        if !criteria.is_empty() {
            tables.insert(
                table.to_owned(),
                Table {
                    headers: vec![
                        "ID".to_owned(),
                        "Criteria".to_owned(),
                        "Verification".to_owned(),
                    ],
                    rows: criteria
                        .iter()
                        .map(|(cid, text)| {
                            vec![(*cid).to_owned(), (*text).to_owned(), "Test".to_owned()]
                        })
                        .collect(),
                },
            );
        }
        Document {
            path: PathBuf::from(format!("{id}.md")),
            id: id.to_owned(),
            kind: kind.to_owned(),
            frontmatter,
            edges: Vec::new(),
            tables,
            sections,
            raw: String::new(),
            contract: Contract::Satisfied,
        }
    }

    fn gap_kinds(generated: &Generated) -> Vec<(&'static str, String)> {
        generated
            .gaps
            .iter()
            .map(|gap| (gap.gap_kind.as_str(), gap.subject.clone()))
            .collect()
    }

    #[trace("FR-192-AC-1")]
    #[test]
    fn three_requirements_render_in_dossier_order_and_the_third_is_unassigned() {
        let documents = vec![
            requirement("FR", "FR-10", "Third", None, &[("FR-10-AC-1", "c")]),
            requirement("FR", "FR-2", "Second", Some("high"), &[("FR-2-AC-1", "c")]),
            requirement("FR", "FR-1", "First", Some("low"), &[("FR-1-AC-1", "c")]),
        ];
        let generated = render(&documents);
        let first = generated.body.find("## FR-1: First").unwrap();
        let second = generated.body.find("## FR-2: Second").unwrap();
        let third = generated.body.find("## FR-10: Third").unwrap();
        assert!(first < second && second < third);
        let tail = &generated.body[third..];
        assert!(tail.contains("Criticality: unassigned"));
        assert_eq!(
            gap_kinds(&generated),
            vec![("criticality_unassigned", "FR-10".to_owned())]
        );
    }

    #[trace("FR-192-AC-2")]
    #[test]
    fn zero_criteria_is_a_no_acceptance_criteria_gap_and_never_unbacked() {
        let documents = vec![requirement("FR", "FR-1", "One", Some("low"), &[])];
        let generated = render(&documents);
        assert_eq!(
            gap_kinds(&generated),
            vec![("no_acceptance_criteria", "FR-1".to_owned())]
        );
        assert!(!generated.body.contains("| Criterion |"));
    }

    #[trace("FR-192-AC-3")]
    #[test]
    fn absent_criticality_renders_no_level() {
        let documents = vec![requirement(
            "NFR",
            "NFR-1",
            "Speed",
            None,
            &[("NFR-1-AC-1", "c")],
        )];
        let generated = render(&documents);
        let criticality: Vec<_> = generated
            .body
            .lines()
            .filter(|row| row.starts_with("Criticality:"))
            .collect();
        assert_eq!(criticality, ["Criticality: unassigned"]);
        for level in ["high", "medium", "low", "critical"] {
            assert!(!generated.body.contains(level), "{level} appeared");
        }
    }

    #[trace("FR-192-AC-4")]
    #[test]
    fn an_empty_scope_renders_no_section_and_a_no_requirements_gap() {
        let generated = render(&[]);
        assert!(!generated.body.contains("\n## "));
        assert_eq!(
            gap_kinds(&generated),
            vec![("no_requirements", "this document".to_owned())]
        );
    }

    #[trace("FR-192-AC-5")]
    #[test]
    fn every_criterion_id_and_text_appears_under_its_requirement() {
        let documents = vec![
            requirement(
                "FR",
                "FR-1",
                "One",
                Some("high"),
                &[("FR-1-AC-1", "first | text"), ("FR-1-AC-2", "second")],
            ),
            requirement(
                "NFR",
                "NFR-1",
                "Two",
                Some("low"),
                &[("NFR-1-AC-1", "third")],
            ),
        ];
        let generated = render(&documents);
        let fr = generated.body.find("## FR-1: One").unwrap();
        let nfr = generated.body.find("## NFR-1: Two").unwrap();
        let fr_section = &generated.body[fr..nfr];
        assert!(fr_section.contains("| FR-1-AC-1 | first \\| text |"));
        assert!(fr_section.contains("| FR-1-AC-2 | second |"));
        assert!(generated.body[nfr..].contains("| NFR-1-AC-1 | third |"));
    }

    #[trace("FR-192-AC-6")]
    #[test]
    fn validation_criteria_of_an_str_are_its_criteria() {
        let documents = vec![requirement(
            "StR",
            "StR-1",
            "Need",
            Some("high"),
            &[("StR-1-VC-1", "validated")],
        )];
        let generated = render(&documents);
        assert!(generated.body.contains("| StR-1-VC-1 | validated |"));
        assert_eq!(generated.gaps, [] as [crate::gap::Gap; 0]);
    }

    #[test]
    fn a_title_with_a_newline_cannot_inject_a_section() {
        let documents = vec![requirement(
            "FR",
            "FR-1",
            "One\n## FR-9: forged",
            Some("low"),
            &[("FR-1-AC-1", "c")],
        )];
        let generated = render(&documents);
        assert_eq!(generated.body.matches("\n## ").count(), 1);
    }

    #[test]
    fn a_malformed_or_unreadable_or_violating_input_is_a_source_failure() {
        let mut violating = requirement("FR", "FR-1", "One", None, &[]);
        violating.contract = Contract::Violated("header is not the declared one".to_owned());
        let corpus = Corpus {
            documents: vec![violating],
            unreadable: vec![Unreadable {
                path: PathBuf::from("a.md"),
                reason: "not utf-8".to_owned(),
            }],
            malformed: vec![PathBuf::from("b.md")],
        };
        let failure = unusable_input("requirements", &corpus).unwrap();
        assert_eq!(failure.source, "requirements");
        assert!(failure.detail.contains("a.md: unreadable: not utf-8"));
        assert!(failure.detail.contains("b.md: malformed frontmatter"));
        assert!(
            failure
                .detail
                .contains("FR-1.md: header is not the declared one")
        );
    }

    #[trace("FR-192-AC-7")]
    #[test]
    fn a_recorded_class_is_shown_as_written_apart_from_criticality() {
        let mut document = requirement("FR", "FR-1", "One", Some("high"), &[("FR-1-AC-1", "c")]);
        document.frontmatter.insert(
            "software_safety_class".to_owned(),
            Value::String("class-2".to_owned()),
        );
        let generated = render(&[document]);
        let lines: Vec<_> = generated.body.lines().collect();
        assert!(lines.contains(&"Criticality: high"));
        assert!(lines.contains(&"Software safety class: class-2"));
        assert_eq!(generated.gaps, [] as [crate::gap::Gap; 0]);
    }

    #[trace("FR-192-AC-8", "FR-192-AC-9")]
    #[test]
    fn an_unrecorded_class_is_unassigned_and_never_copied_from_criticality_or_a_part() {
        let mut document = requirement("FR", "FR-1", "One", Some("high"), &[("FR-1-AC-1", "c")]);
        document.frontmatter.remove("software_safety_class");
        // A part that records a class is allocated this requirement elsewhere;
        // the SRS reads only the requirement's own document.
        let mut part = requirement("FR", "PART-1", "Part", None, &[]);
        part.kind = "part".to_owned();
        let generated = render(&[document, part]);
        let section = generated.body.find("## FR-1: One").unwrap();
        let section = &generated.body[section
            ..generated
                .body
                .find("## PART-1")
                .unwrap_or(generated.body.len())];
        assert!(section.contains("Software safety class: unassigned"));
        assert!(!section.contains("class-"));
        assert_eq!(
            gap_kinds(&generated),
            vec![("safety_class_unassigned", "FR-1".to_owned())]
        );
    }
}
