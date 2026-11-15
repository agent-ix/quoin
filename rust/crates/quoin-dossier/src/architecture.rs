// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Architecture document generator (FR-193, PLAT-1437).
//!
//! Lists parts, ports, connections, allocations, ADRs and the requirements each
//! part serves, from the design join.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::order::dossier_cmp;
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};

/// The Architecture generator.
pub struct ArchitectureGenerator;

impl Generator for ArchitectureGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "architecture",
            id: "DOS-ARCH",
            title: "Architecture",
            needs_as_of: false,
        }
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        let request = quoin_quire::coverage::Request {
            scope: quoin_quire::ids::ScopeRoot::open(&inputs.scope).map_err(|error| {
                SourceFailure {
                    source: "design-join".to_owned(),
                    detail: format!("failed to open scope: {error}"),
                }
            })?,
            modules: quoin_quire::ModuleSelection::from_paths(&inputs.modules),
        };

        // Load systems records
        let systems = quoin_quire::systems::load(&request).map_err(|error| SourceFailure {
            source: "systems-records".to_owned(),
            detail: format!("failed to load systems records: {error}"),
        })?;

        // Load design join
        let join = quoin_quire::design_join::compute(&request).map_err(|error| {
            // Check if design_relations is unavailable
            let detail = error.to_string();
            SourceFailure {
                source: "design-join".to_owned(),
                detail,
            }
        })?;

        // Load ADRs
        let corpus = quoin_quire::records::read(
            &inputs.scope,
            &quoin_quire::ModuleSelection::from_paths(&inputs.modules),
            &["ADR"],
        )
        .map_err(|error| SourceFailure {
            source: "adr".to_owned(),
            detail: format!("failed to read ADR documents: {error}"),
        })?;

        let mut adrs: Vec<_> = corpus
            .documents
            .iter()
            .filter(|doc| doc.kind == "ADR")
            .collect();
        adrs.sort_by(|a, b| dossier_cmp(&a.id, &b.id));

        let mut gaps = Vec::new();
        let mut body = String::from("# Architecture\n\n");

        // Check for design_relations availability
        if join.unavailable.is_some() {
            gaps.push(Gap::new(
                GapKind::DesignRelationsUnavailable,
                "this document",
                "design_relations is not declared by loaded modules",
            ));
        }

        // Build requirement-to-elements map from design join
        let mut req_to_elements: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut uncovered_requirements = BTreeSet::new();
        let mut dangling_refs = Vec::new();
        let mut kind_mismatches = Vec::new();

        for row in &join.requirements {
            let req_id = &row.requirement;
            match &row.state {
                quoin_quire::model::DesignBlock::Uncovered => {
                    uncovered_requirements.insert(req_id.clone());
                }
                quoin_quire::model::DesignBlock::Elements(elements) => {
                    for element in elements {
                        match element {
                            quoin_quire::model::DesignElement::Resolved(resolved) => {
                                req_to_elements
                                    .entry(req_id.clone())
                                    .or_default()
                                    .push(resolved.element.clone());
                            }
                            quoin_quire::model::DesignElement::Dangling(reference) => {
                                dangling_refs.push((req_id.clone(), reference.clone()));
                            }
                            quoin_quire::model::DesignElement::KindMismatch {
                                relation,
                                element,
                            } => {
                                kind_mismatches.push((req_id.clone(), relation.clone(), element.clone()));
                            }
                        }
                    }
                }
            }
        }

        // Parts section
        body.push_str("## Parts\n\n");
        if !systems.parts.is_empty() {
            body.push_str("| Part | Ports | Allocated or realized requirements |\n");
            body.push_str("|------|-------|------------------------------------|\n");

            let mut parts = systems.parts.clone();
            parts.sort_by(|a, b| dossier_cmp(&a.id, &b.id));

            for part in &parts {
                let part_id = markdown_escape(&part.id);

                // Find ports for this part
                let part_ports: Vec<_> = systems
                    .ports
                    .iter()
                    .filter(|p| p.part == part.id)
                    .collect();
                let ports_str = if part_ports.is_empty() {
                    String::from("—")
                } else {
                    part_ports
                        .iter()
                        .map(|p| markdown_escape(&p.id))
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                // Find requirements for this part (from allocations and join)
                let mut part_reqs = BTreeSet::new();

                // From allocations
                for alloc in &systems.allocations {
                    if alloc.part == part.id {
                        part_reqs.insert(&alloc.requirement);
                    }
                }

                // From design join (requirements realized by this part)
                for (req_id, elements) in &req_to_elements {
                    if elements.contains(&part.id) {
                        part_reqs.insert(req_id);
                    }
                }

                let reqs_str = if part_reqs.is_empty() {
                    gaps.push(
                        Gap::new(
                            GapKind::NoAllocation,
                            &part.id,
                            "part has no allocated requirement",
                        )
                        .for_element(&part.id),
                    );
                    String::from("—")
                } else {
                    part_reqs
                        .into_iter()
                        .map(|r| markdown_escape(r))
                        .collect::<Vec<_>>()
                        .join(", ")
                };

                body.push_str(&format!("| {part_id} | {ports_str} | {reqs_str} |\n"));
            }
            body.push_str("\n");
        }

        // Connections section
        body.push_str("## Connections\n\n");
        if !systems.connections.is_empty() {
            body.push_str("| Connection | From | To | Direction |\n");
            body.push_str("|------------|------|----|-----------|\n");

            let mut connections = systems.connections.clone();
            connections.sort_by(|a, b| dossier_cmp(&a.id, &b.id));

            for conn in &connections {
                let conn_id = markdown_escape(&conn.id);
                let from = markdown_escape(&conn.from);
                let to = markdown_escape(&conn.to);
                let direction = format!("{:?}", conn.direction);
                body.push_str(&format!("| {conn_id} | {from} | {to} | {direction} |\n"));
            }
            body.push_str("\n");
        }

        // Decisions (ADRs) section
        body.push_str("## Decisions\n\n");
        if !adrs.is_empty() {
            body.push_str("| ID | Title | Status |\n");
            body.push_str("|----|-------|--------|\n");

            for adr in &adrs {
                let id = markdown_escape(&adr.id);
                let title = markdown_escape(
                    &adr.frontmatter
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                );
                let status = markdown_escape(
                    &adr.frontmatter
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("—"),
                );
                body.push_str(&format!("| {id} | {title} | {status} |\n"));
            }
            body.push_str("\n");
        }

        // Requirements without a design element
        body.push_str("## Requirements without a design element\n\n");
        if !uncovered_requirements.is_empty() {
            for req_id in &uncovered_requirements {
                body.push_str(&format!("- {}\n", markdown_escape(req_id)));
                gaps.push(
                    Gap::new(
                        GapKind::NoDesignElement,
                        req_id,
                        "requirement has no design element",
                    )
                    .for_requirement(req_id),
                );
            }
            body.push_str("\n");
        } else {
            body.push_str("None.\n\n");
        }

        // Dangling design references
        for (req_id, reference) in &dangling_refs {
            gaps.push(
                Gap::new(
                    GapKind::DanglingDesignReference,
                    reference,
                    &format!("requirement {req_id} references {reference}, which does not exist"),
                )
                .for_requirement(req_id),
            );
        }

        // Kind mismatches
        for (req_id, _relation, element) in &kind_mismatches {
            gaps.push(
                Gap::new(
                    GapKind::DesignKindMismatch,
                    element,
                    &format!("requirement {req_id} references {element} with wrong kind"),
                )
                .for_requirement(req_id)
                .for_element(element),
            );
        }

        Ok(Generated { body, gaps })
    }
}

/// Escape Markdown table cell content so it cannot break table syntax.
fn markdown_escape(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}
