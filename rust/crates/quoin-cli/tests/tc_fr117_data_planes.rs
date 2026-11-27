// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Keep the semantic data-plane architecture record aligned with FR-117.

use ix_trace_rs::trace;

const INDEX: &str = include_str!("../../../../docs/semantic-module-architecture/index.md");
const PLANES: &str =
    include_str!("../../../../docs/semantic-module-architecture/planes-and-authority.md");
const OWNERSHIP: &str =
    include_str!("../../../../docs/semantic-module-architecture/ownership-and-boundaries.md");
const DYNAMIC: &str =
    include_str!("../../../../docs/semantic-module-architecture/dynamic-and-generated.md");
const DECISIONS: &str = include_str!("../../../../docs/semantic-module-architecture/adr/index.md");
const LEDGER: &str =
    include_str!("../../../../docs/semantic-module-architecture/decision-ledger.md");

fn normalized(document: &str) -> String {
    document.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[trace("FR-117-AC-1")]
#[test]
fn each_plane_names_concepts_authorities_and_projections() {
    let planes = normalized(PLANES);
    for (heading, concept, authority, projection) in [
        (
            "### Meta plane",
            "packages, semantic type definitions, mappings, profiles",
            "TypeSpec schema/package source",
            "normalized or bundled JSON Schema, language types, wire descriptors",
        ),
        (
            "### Definition plane",
            "requirements, architecture, plans, policy, test definitions",
            "Reviewed typed Markdown",
            "Extracted JSON, graph records, search indexes",
        ),
        (
            "### Execution and observation plane",
            "runs, events, results, evidence, incidents, measurements",
            "Owning run, evidence, or event store",
            "reports, dashboards, alerts",
        ),
        (
            "### Presentation plane",
            "documents, reports, dashboards, UI views, tables, and exports",
            "normally derived",
            "Markdown, HTML, PDF, React views",
        ),
    ] {
        assert!(planes.contains(heading), "missing {heading}");
        assert!(
            planes.contains(concept),
            "{heading} omits representative concepts"
        );
        assert!(planes.contains(authority), "{heading} omits its authority");
        assert!(
            planes.contains(projection),
            "{heading} omits its projections"
        );
    }
}

#[trace("FR-117-AC-2")]
#[test]
fn the_record_distinguishes_definition_occurrence_and_presentation() {
    let planes = normalized(PLANES);
    assert!(planes.contains("A `TestCase` definition"));
    assert!(planes.contains("One `TestExecution`"));
    assert!(planes.contains("A run report summarizing that execution"));
    assert!(planes.contains("These are linked objects, not three interchangeable encodings"));
}

#[trace("FR-117-AC-3")]
#[test]
fn structural_kind_and_semantic_role_are_independent_without_a_universal_envelope() {
    let planes = normalized(PLANES);
    assert!(planes.contains("Structural kind and semantic role are independent"));
    assert!(planes.contains("must not infer domain meaning from `record`, `enum`, a heading"));
    assert!(planes.contains("Explicit non-goal: No universal `SemanticObject` runtime envelope"));
}

#[trace("FR-117-AC-4")]
#[test]
fn the_index_links_normative_records_and_labels_decision_gates() {
    let index = normalized(INDEX);
    for link in [
        "[Planes and authority](planes-and-authority.md)",
        "[Ownership and boundaries](ownership-and-boundaries.md)",
        "[Dynamic and generated modules](dynamic-and-generated.md)",
        "[External decision ledger](decision-ledger.md)",
    ] {
        assert!(index.contains(link), "index is missing {link}");
    }
    assert!(index.contains("proposed normative"));
    assert!(index.contains("normative evidence"));

    let decisions = normalized(DECISIONS);
    assert!(decisions.contains("Named Quoin/Quire maintainer review on issue #289 PR"));
    assert!(decisions.contains("After acceptance, a decision remains normative"));

    let ledger = normalized(LEDGER);
    assert!(ledger.contains("provisional; gate `filament-core-data#9`"));
    assert!(ledger.contains("ADR-0005 normative; ADR-0004 historical"));
    assert!(OWNERSHIP.contains("# Semantic-module ownership and subsystem boundaries"));
    assert!(DYNAMIC.contains("# Dynamic modules and finite generated packages"));
}
