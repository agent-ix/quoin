// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Architecture document generator (FR-193, PLAT-1437).
//!
//! The requirement-to-element mapping comes only from the quire-rs design
//! matrix (the design join per requirement) and the authored allocation rows;
//! nothing is matched by name.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use quoin_quire::coverage::Request;
use quoin_quire::records::Document;
use quoin_quire::systems::Systems;
use serde_json::Value;

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::markdown::{cell, unusable_input};
use crate::order::dossier_cmp;
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};
use crate::srs::{recorded_text, safety_class_gap_for_part};

/// The Architecture generator.
pub(crate) struct ArchitectureGenerator;

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
        let modules = inputs
            .modules
            .iter()
            .map(quoin_quire::ids::ModuleRoot::open)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| SourceFailure {
                source: "modules".to_owned(),
                detail: format!("failed to open module: {error}"),
            })?;
        let selection = quoin_quire::ModuleSelection::Closed(modules);

        let scope =
            quoin_quire::ids::ScopeRoot::open(&inputs.scope).map_err(|error| SourceFailure {
                source: "scope".to_owned(),
                detail: format!("failed to open scope: {error}"),
            })?;
        let request = Request {
            scope,
            modules: selection.clone(),
        };

        let outcome =
            quoin_quire::design_matrix::compute(&request).map_err(|error| SourceFailure {
                source: "design_matrix".to_owned(),
                detail: format!("failed to compute design matrix: {error}"),
            })?;
        let Ok(Value::Array(entries)) = serde_json::to_value(&outcome.design_matrix) else {
            return Err(SourceFailure {
                source: "design_matrix".to_owned(),
                detail: "the design matrix did not serialize to a list".to_owned(),
            });
        };

        let systems = quoin_quire::systems::load(&request).map_err(|error| SourceFailure {
            source: "systems".to_owned(),
            detail: format!("failed to read systems records: {error}"),
        })?;

        let corpus =
            quoin_quire::records::read(&inputs.scope, &selection, &["ADR"]).map_err(|error| {
                SourceFailure {
                    source: "adrs".to_owned(),
                    detail: format!("failed to read ADR documents: {error}"),
                }
            })?;
        if let Some(failure) = unusable_input("adrs", &corpus) {
            return Err(failure);
        }
        let adrs: Vec<&Document> = corpus
            .documents
            .iter()
            .filter(|doc| doc.kind == "ADR")
            .collect();

        Ok(render(&entries, &systems, &adrs))
    }
}

/// What the document knows of one part.
#[derive(Default)]
struct Part {
    requirements: BTreeSet<String>,
    ports: BTreeSet<String>,
    /// The software safety class the part's own document records, as written.
    class: Option<String>,
}

/// The architecture document over the design-matrix `entries` (as JSON), the
/// systems records and the ADRs.
fn render(entries: &[Value], systems: &Systems, adrs: &[&Document]) -> Generated {
    let mut gaps = Vec::new();
    let relations_declared = entries
        .iter()
        .any(|entry| entry.get("design").is_some_and(Value::is_array));
    if !relations_declared {
        gaps.push(Gap::new(
            GapKind::DesignRelationsUnavailable,
            "design_relations",
            "the loaded modules declare no design_relations, so no requirement can be joined to a design element",
        ));
    }

    let mut join = collect_join(entries);
    add_systems(&mut join.parts, systems);

    let mut body = String::from("# Architecture\n\n");
    parts_section(&mut body, &mut gaps, &join.parts);
    connections_section(&mut body, systems);
    allocations_section(&mut body, systems);
    decisions_section(&mut body, adrs);
    findings_sections(&mut body, &mut gaps, &join);
    Generated {
        body,
        document_version: None,
        gaps,
    }
}

/// What the design join says, before the systems records are added.
#[derive(Default)]
struct Join {
    parts: BTreeMap<String, Part>,
    without_design: BTreeSet<String>,
    /// `(requirement, reference)`.
    dangling: BTreeSet<(String, String)>,
    /// `(requirement, element id, archetype)`.
    mismatched: BTreeSet<(String, String, String)>,
}

fn collect_join(entries: &[Value]) -> Join {
    let mut join = Join::default();
    for entry in entries {
        let Some(blocks) = entry.get("design").and_then(Value::as_array) else {
            continue;
        };
        let requirements = requirement_ids(entry);
        for block in blocks {
            let state = block.get("state").and_then(Value::as_str);
            if state == Some("uncovered") {
                join.without_design.extend(requirements.iter().cloned());
            }
            for element in array(block, "elements") {
                join.parts
                    .entry(qualified(element))
                    .or_default()
                    .requirements
                    .extend(requirements.iter().cloned());
            }
            for reference in array(block, "dangling") {
                let name = text(reference, "reference");
                for requirement in &requirements {
                    join.dangling.insert((requirement.clone(), name.clone()));
                }
            }
            for mismatch in array(block, "kind_mismatch") {
                let id = qualified(mismatch);
                let archetype = text(mismatch, "archetype");
                for requirement in &requirements {
                    join.mismatched
                        .insert((requirement.clone(), id.clone(), archetype.clone()));
                }
            }
        }
    }
    join
}

/// Add the parts the systems records declare and the requirements they
/// allocate.
fn add_systems(parts: &mut BTreeMap<String, Part>, systems: &Systems) {
    // A part is identified by the document that declares it. The `owner` cell
    // names the composite that holds the part and is not the part's identity.
    for part in &systems.parts {
        parts.entry(part.document.clone()).or_default().class =
            part.software_safety_class.as_ref().and_then(recorded_text);
    }
    for port in &systems.ports {
        parts
            .entry(port.owner.clone())
            .or_default()
            .ports
            .insert(format!("{} ({})", port.interface, port.direction));
    }
    // An allocation names a requirement at one end and a part at the other.
    for allocation in &systems.allocations {
        if let Some(part) = parts.get_mut(&allocation.target) {
            part.requirements.insert(allocation.source.clone());
        } else if let Some(part) = parts.get_mut(&allocation.source) {
            part.requirements.insert(allocation.target.clone());
        }
    }
}

fn parts_section(body: &mut String, gaps: &mut Vec<Gap>, parts: &BTreeMap<String, Part>) {
    body.push_str("## Parts\n\n");
    if parts.is_empty() {
        body.push_str("None.\n\n");
    } else {
        body.push_str(
            "| Part | Software safety class | Ports | Allocated or realized requirements |\n",
        );
        body.push_str(
            "|------|-----------------------|-------|-------------------------------------|\n",
        );
        let mut ids: Vec<&String> = parts.keys().collect();
        ids.sort_by(|a, b| dossier_cmp(a, b));
        for id in ids {
            let Some(part) = parts.get(id) else {
                continue;
            };
            let _ = writeln!(
                body,
                "| {} | {} | {} | {} |",
                cell(id),
                cell(part.class.as_deref().unwrap_or("unassigned")),
                cell(&part.ports.iter().cloned().collect::<Vec<_>>().join(", ")),
                cell(&sorted(&part.requirements).join(", ")),
            );
            if part.class.is_none() {
                gaps.push(safety_class_gap_for_part(id));
            }
            if part.requirements.is_empty() {
                gaps.push(
                    Gap::new(GapKind::NoAllocation, id, "has no allocated requirement")
                        .for_element(id),
                );
            }
        }
        body.push('\n');
    }
}

fn connections_section(body: &mut String, systems: &Systems) {
    body.push_str("## Connections\n\n");
    if systems.connections.is_empty() {
        body.push_str("None.\n\n");
    } else {
        body.push_str("| Connection | From | To | Direction |\n");
        body.push_str("|------------|------|----|-----------|\n");
        let mut connections: Vec<_> = systems.connections.iter().collect();
        connections.sort_by(|a, b| {
            dossier_cmp(&a.source, &b.source).then_with(|| dossier_cmp(&a.target, &b.target))
        });
        for connection in connections {
            let _ = writeln!(
                body,
                "| {} | {} | {} | {} |",
                cell(&format!("{} -> {}", connection.source, connection.target)),
                cell(&end(&connection.source, &connection.source_multiplicity)),
                cell(&end(&connection.target, &connection.target_multiplicity)),
                cell(&connection.direction),
            );
        }
        body.push('\n');
    }
}

fn allocations_section(body: &mut String, systems: &Systems) {
    body.push_str("## Allocations\n\n");
    if systems.allocations.is_empty() {
        body.push_str("None.\n\n");
    } else {
        body.push_str("| Source | Target |\n|--------|--------|\n");
        let mut allocations: Vec<_> = systems.allocations.iter().collect();
        allocations.sort_by(|a, b| {
            dossier_cmp(&a.source, &b.source).then_with(|| dossier_cmp(&a.target, &b.target))
        });
        for allocation in allocations {
            let _ = writeln!(
                body,
                "| {} | {} |",
                cell(&allocation.source),
                cell(&allocation.target)
            );
        }
        body.push('\n');
    }
}

fn decisions_section(body: &mut String, adrs: &[&Document]) {
    body.push_str("## Decisions\n\n");
    if adrs.is_empty() {
        body.push_str("None.\n\n");
    } else {
        body.push_str("| ID | Title | Status |\n|----|-------|--------|\n");
        let mut sorted_adrs = adrs.to_vec();
        sorted_adrs.sort_by(|a, b| dossier_cmp(&a.id, &b.id));
        for adr in sorted_adrs {
            let field = |key: &str| {
                adr.frontmatter
                    .get(key)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
            };
            let _ = writeln!(
                body,
                "| {} | {} | {} |",
                cell(&adr.id),
                cell(field("title")),
                cell(field("status")),
            );
        }
        body.push('\n');
    }
}

/// The sections for what the join could not place, each with its gaps.
fn findings_sections(body: &mut String, gaps: &mut Vec<Gap>, join: &Join) {
    body.push_str("## Requirements without a design element\n\n");
    if join.without_design.is_empty() {
        body.push_str("None.\n\n");
    } else {
        for requirement in sorted(&join.without_design) {
            let _ = writeln!(body, "- {}", cell(&requirement));
            gaps.push(
                Gap::new(
                    GapKind::NoDesignElement,
                    &requirement,
                    "has no realizing design element",
                )
                .for_requirement(&requirement),
            );
        }
        body.push('\n');
    }

    body.push_str("## Unresolved design references\n\n");
    if join.dangling.is_empty() {
        body.push_str("None.\n\n");
    } else {
        for (requirement, reference) in sorted_pairs(&join.dangling) {
            let _ = writeln!(body, "- {} -> {}", cell(&requirement), cell(&reference));
            gaps.push(
                Gap::new(
                    GapKind::DanglingDesignReference,
                    &reference,
                    format!("{requirement} names a design reference that resolves to no element"),
                )
                .for_requirement(&requirement),
            );
        }
        body.push('\n');
    }

    body.push_str("## Design elements of the wrong kind\n\n");
    if join.mismatched.is_empty() {
        body.push_str("None.\n\n");
    } else {
        let mut rows: Vec<_> = join.mismatched.iter().collect();
        rows.sort_by(|a, b| dossier_cmp(&a.0, &b.0).then_with(|| dossier_cmp(&a.1, &b.1)));
        for (requirement, element, archetype) in rows {
            let _ = writeln!(
                body,
                "- {} -> {} ({})",
                cell(requirement),
                cell(element),
                cell(archetype)
            );
            gaps.push(
                Gap::new(
                    GapKind::DesignKindMismatch,
                    element,
                    format!("{requirement} lists {element}, an element of a kind the relation does not accept"),
                )
                .for_requirement(requirement)
                .for_element(element),
            );
        }
        body.push('\n');
    }
}

/// The requirements a design-matrix entry stands for: its `id` when it has one,
/// otherwise the requirement of each of its criteria. An entry carries no `id`
/// when its document is not design-obliged, so the criteria are the only name.
fn requirement_ids(entry: &Value) -> BTreeSet<String> {
    if let Some(id) = entry.get("id").and_then(Value::as_str) {
        return BTreeSet::from([id.to_owned()]);
    }
    array(entry, "criteria")
        .filter_map(|criterion| criterion.get("id").and_then(Value::as_str))
        .map(|id| quoin_quire_types::requirement_of(id).to_owned())
        .collect()
}

fn array<'a>(value: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// An element id, prefixed by its repo when the join names one.
fn qualified(element: &Value) -> String {
    let id = text(element, "id");
    match element.get("repo").and_then(Value::as_str) {
        Some(repo) => format!("{repo}/{id}"),
        None => id,
    }
}

/// A connection end with its multiplicity, when it states one.
fn end(name: &str, multiplicity: &str) -> String {
    if multiplicity.is_empty() {
        name.to_owned()
    } else {
        format!("{name} [{multiplicity}]")
    }
}

fn sorted(ids: &BTreeSet<String>) -> Vec<String> {
    let mut ids: Vec<String> = ids.iter().cloned().collect();
    ids.sort_by(|a, b| dossier_cmp(a, b));
    ids
}

fn sorted_pairs(pairs: &BTreeSet<(String, String)>) -> Vec<(String, String)> {
    let mut pairs: Vec<_> = pairs.iter().cloned().collect();
    pairs.sort_by(|a, b| dossier_cmp(&a.0, &b.0).then_with(|| dossier_cmp(&a.1, &b.1)));
    pairs
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

    use quoin_quire::records::Contract;
    use quoin_quire::systems::{Allocation, Connection, Part as PartRow, Port};
    use serde_json::{Map, json};

    use super::*;
    use ix_trace_rs::trace;

    fn covered(id: &str, elements: &[&str]) -> Value {
        json!({
            "document": format!("{id}.md"),
            "id": id,
            "criteria": [],
            "design": [{
                "relation": "realized-by", "check": "c", "state": "covered",
                "elements": elements.iter().map(|e| json!({"id": e, "kind": "Component", "verb": "implements"})).collect::<Vec<_>>(),
                "dangling": [], "kind_mismatch": []
            }]
        })
    }

    fn uncovered(id: &str) -> Value {
        json!({
            "document": format!("{id}.md"), "id": id, "criteria": [],
            "design": [{"relation": "realized-by", "check": "fr-needs-design", "state": "uncovered",
                        "elements": [], "dangling": [], "kind_mismatch": []}]
        })
    }

    fn adr(id: &str, title: &str, status: &str) -> Document {
        let mut frontmatter = Map::new();
        frontmatter.insert("title".to_owned(), json!(title));
        frontmatter.insert("status".to_owned(), json!(status));
        Document {
            path: PathBuf::from(format!("{id}.md")),
            id: id.to_owned(),
            kind: "ADR".to_owned(),
            frontmatter,
            edges: Vec::new(),
            tables: BTreeMap::new(),
            sections: BTreeMap::new(),
            raw: String::new(),
            contract: Contract::Satisfied,
        }
    }

    /// The gaps in FR-191 order.
    fn kinds(generated: &Generated) -> Vec<(&'static str, String)> {
        let mut gaps = generated.gaps.clone();
        crate::gap::sort_gaps(&mut gaps);
        gaps.iter()
            .map(|gap| (gap.gap_kind.as_str(), gap.subject.clone()))
            .collect()
    }

    /// A part row declared by the document `document`, owned by `OWNER`.
    fn part_row(document: &str, class: Option<&str>) -> PartRow {
        PartRow {
            document: document.to_owned(),
            owner: "OWNER".to_owned(),
            declared_type: "T".to_owned(),
            multiplicity: "1".to_owned(),
            software_safety_class: class.map(|class| json!(class)),
        }
    }

    #[trace("FR-193-AC-1")]
    #[test]
    fn parts_connections_adr_are_listed_and_an_uncovered_requirement_is_a_gap() {
        let entries = vec![covered("FR-1", &["PART-A", "PART-B"]), uncovered("FR-2")];
        let systems = Systems {
            connections: vec![Connection {
                document: "COMP.md".to_owned(),
                source: "PART-A".to_owned(),
                source_multiplicity: String::new(),
                target: "PART-B".to_owned(),
                target_multiplicity: String::new(),
                direction: "forward".to_owned(),
            }],
            ..Systems::default()
        };
        let record = adr("ADR-1", "Use a queue", "accepted");
        let generated = render(&entries, &systems, &[&record]);

        assert!(generated.body.contains("| PART-A | unassigned |  | FR-1 |"));
        assert!(generated.body.contains("| PART-B | unassigned |  | FR-1 |"));
        assert!(
            generated
                .body
                .contains("| PART-A -> PART-B | PART-A | PART-B | forward |")
        );
        assert!(
            generated
                .body
                .contains("| ADR-1 | Use a queue | accepted |")
        );
        assert!(
            generated
                .body
                .contains("## Requirements without a design element\n\n- FR-2\n")
        );
        assert_eq!(
            kinds(&generated),
            vec![
                ("safety_class_unassigned", "PART-A".to_owned()),
                ("safety_class_unassigned", "PART-B".to_owned()),
                ("no_design_element", "FR-2".to_owned()),
            ]
        );
    }

    #[trace("FR-193-AC-2")]
    #[test]
    fn a_dangling_design_reference_is_listed_and_a_gap() {
        let entries = vec![json!({
            "document": "FR-1.md", "id": "FR-1", "criteria": [],
            "design": [{"relation": "r", "check": "c", "state": "covered", "elements": [],
                        "dangling": [{"reference": "IMPL-X", "verb": "implements"}],
                        "kind_mismatch": []}]
        })];
        let generated = render(&entries, &Systems::default(), &[]);
        assert!(generated.body.contains("- FR-1 -> IMPL-X"));
        assert_eq!(
            kinds(&generated),
            vec![("dangling_design_reference", "IMPL-X".to_owned())]
        );
    }

    #[trace("FR-193-AC-3")]
    #[test]
    fn no_design_relations_is_one_cause_not_a_gap_per_requirement() {
        let entries = vec![
            json!({"document": "FR-1.md", "criteria": [{"id": "FR-1-AC-1"}]}),
            json!({"document": "FR-2.md", "criteria": [{"id": "FR-2-AC-1"}]}),
        ];
        let generated = render(&entries, &Systems::default(), &[]);
        assert_eq!(
            kinds(&generated),
            vec![(
                "design_relations_unavailable",
                "design_relations".to_owned()
            )]
        );
        assert!(generated.gaps[0].detail.contains("no design_relations"));
    }

    #[trace("FR-193-AC-4")]
    #[test]
    fn a_part_with_no_allocated_requirement_is_a_gap_and_an_allocation_counts() {
        let systems = Systems {
            parts: vec![
                part_row("PART-FREE", Some("class-1")),
                part_row("PART-ALLOCATED", Some("class-1")),
            ],
            allocations: vec![Allocation {
                document: "C.md".to_owned(),
                source: "FR-7".to_owned(),
                target: "PART-ALLOCATED".to_owned(),
            }],
            ..Systems::default()
        };
        let generated = render(&[covered("FR-1", &["PART-A"])], &systems, &[]);
        assert_eq!(
            kinds(&generated),
            vec![
                ("safety_class_unassigned", "PART-A".to_owned()),
                ("no_allocation", "PART-FREE".to_owned()),
            ]
        );
        assert!(
            generated
                .body
                .contains("| PART-ALLOCATED | class-1 |  | FR-7 |")
        );
        assert!(generated.body.contains("| FR-7 | PART-ALLOCATED |"));
    }

    #[trace("FR-193-AC-5")]
    #[test]
    fn a_connection_shows_both_ends_with_multiplicity_and_its_direction() {
        let systems = Systems {
            ports: vec![Port {
                document: "C.md".to_owned(),
                owner: "PART-A".to_owned(),
                direction: "out".to_owned(),
                interface: "Telemetry".to_owned(),
                multiplicity: "1".to_owned(),
            }],
            connections: vec![Connection {
                document: "C.md".to_owned(),
                source: "PART-A.out".to_owned(),
                source_multiplicity: "1".to_owned(),
                target: "PART-B.in".to_owned(),
                target_multiplicity: "0..*".to_owned(),
                direction: "bidirectional".to_owned(),
            }],
            ..Systems::default()
        };
        let generated = render(&[], &systems, &[]);
        assert!(generated.body.contains(
            "| PART-A.out -> PART-B.in | PART-A.out [1] | PART-B.in [0..*] | bidirectional |"
        ));
        assert!(
            generated
                .body
                .contains("| PART-A | unassigned | Telemetry (out) |")
        );
    }

    #[trace("FR-193-AC-6")]
    #[test]
    fn a_wrong_kind_element_is_listed_and_names_requirement_and_element() {
        let entries = vec![json!({
            "document": "FR-1.md", "id": "FR-1", "criteria": [],
            "design": [{"relation": "r", "check": "c", "state": "covered", "elements": [],
                        "dangling": [],
                        "kind_mismatch": [{"id": "PLAN-X", "archetype": "Plan", "verb": "implements", "accepted": ["Component"]}]}]
        })];
        let generated = render(&entries, &Systems::default(), &[]);
        assert!(generated.body.contains("- FR-1 -> PLAN-X (Plan)"));
        assert_eq!(
            kinds(&generated),
            vec![("design_kind_mismatch", "PLAN-X".to_owned())]
        );
        let detail = &generated.gaps[0].detail;
        assert!(detail.contains("FR-1") && detail.contains("PLAN-X"));
    }

    #[test]
    fn an_entry_without_an_id_is_named_by_its_criteria() {
        let entries = vec![json!({
            "document": "FR-3.md", "criteria": [{"id": "FR-3-AC-1"}, {"id": "FR-3-AC-2"}],
            "design": [{"relation": "r", "check": "c", "state": "uncovered", "elements": [],
                        "dangling": [], "kind_mismatch": []}]
        })];
        let generated = render(&entries, &Systems::default(), &[]);
        assert_eq!(
            kinds(&generated),
            vec![("no_design_element", "FR-3".to_owned())]
        );
    }

    #[test]
    fn a_part_title_with_a_pipe_and_a_newline_is_one_table_row() {
        let generated = render(&[covered("FR-1", &["A|B\nC"])], &Systems::default(), &[]);
        assert!(
            generated
                .body
                .contains("| A\\|B C | unassigned |  | FR-1 |\n")
        );
        assert_eq!(generated.body.matches("\n| A").count(), 1);
    }

    #[trace("FR-193-AC-7")]
    #[test]
    fn a_part_shows_the_class_its_document_records_as_written() {
        let systems = Systems {
            parts: vec![part_row("PART-A", Some("class-3"))],
            ..Systems::default()
        };
        let generated = render(&[covered("FR-1", &["PART-A"])], &systems, &[]);
        assert!(generated.body.contains("| PART-A | class-3 |  | FR-1 |"));
        assert!(kinds(&generated).is_empty());
    }

    #[trace("FR-193-AC-8")]
    #[test]
    fn an_unclassed_part_is_unassigned_whatever_its_parent_or_requirement_records() {
        // The owner cell names a classed parent, and the allocated requirement
        // is classed through its own document; neither lends a class.
        let systems = Systems {
            parts: vec![
                part_row("PART-HIGH", Some("class-2")),
                part_row("PART-NONE", None),
            ],
            allocations: vec![Allocation {
                document: "C.md".to_owned(),
                source: "FR-1".to_owned(),
                target: "PART-NONE".to_owned(),
            }],
            ..Systems::default()
        };
        let generated = render(&[covered("FR-1", &["PART-HIGH"])], &systems, &[]);
        assert!(
            generated
                .body
                .contains("| PART-NONE | unassigned |  | FR-1 |")
        );
        assert!(generated.body.contains("| PART-HIGH | class-2 |"));
        assert_eq!(
            kinds(&generated),
            vec![("safety_class_unassigned", "PART-NONE".to_owned())]
        );
        assert_eq!(generated.gaps[0].subject, "PART-NONE");
    }
}
