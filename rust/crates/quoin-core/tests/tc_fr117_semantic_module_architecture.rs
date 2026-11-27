// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Structural checks keep the semantic-module architecture records aligned with FR-117..119.

use ix_trace_rs::trace;

const PLANES: &str =
    include_str!("../../../../docs/semantic-module-architecture/planes-and-authority.md");
const OWNERSHIP: &str =
    include_str!("../../../../docs/semantic-module-architecture/ownership-and-boundaries.md");
const INDEX: &str = include_str!("../../../../docs/semantic-module-architecture/index.md");
const ADR_INDEX: &str = include_str!("../../../../docs/semantic-module-architecture/adr/index.md");
const LEDGER: &str =
    include_str!("../../../../docs/semantic-module-architecture/decision-ledger.md");
const AUTHORITY_ADR: &str =
    include_str!("../../../../docs/semantic-module-architecture/adr/0001-authority-by-concern.md");

fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn plane_section<'a>(text: &'a str, heading: &str) -> Option<&'a str> {
    let marker = format!("### {heading}");
    let start = text.find(&marker)? + marker.len();
    let body = text.get(start..)?;
    let end = body
        .find("\n### ")
        .or_else(|| body.find("\n## "))
        .unwrap_or(body.len());
    body.get(..end)
}

fn heading_section<'a>(text: &'a str, heading: &str) -> Option<&'a str> {
    let marker = format!("## {heading}");
    let start = text.find(&marker)? + marker.len();
    let body = text.get(start..)?;
    let end = body.find("\n## ").unwrap_or(body.len());
    body.get(..end)
}

#[trace("FR-117-AC-1")]
#[test]
fn tc_fr117_ac1_defines_four_planes_with_concepts_authorities_and_projections() {
    let expected = [
        (
            "Meta plane",
            "semantic type definitions",
            "TypeSpec schema/package source",
            "JSON Schema, language types, wire descriptors, and registry metadata are outputs",
        ),
        (
            "Definition plane",
            "requirements, architecture, plans, policy, test definitions",
            "Reviewed typed Markdown is the normal authority",
            "Extracted JSON, graph records, search indexes, embeddings, and rendered views are projections",
        ),
        (
            "Execution and observation plane",
            "runs, events, results, evidence, incidents, measurements",
            "Owning run, evidence, or event store is authoritative",
            "a text view",
        ),
        (
            "Presentation plane",
            "documents, reports, dashboards, UI views, tables, and exports",
            "It is normally derived",
            "Markdown report is a presentation projection",
        ),
    ];
    for (plane, concept, authority, projection) in expected {
        assert!(
            plane_section(PLANES, plane).is_some_and(|section| {
                let section = normalized(section);
                section.contains(concept)
                    && section.contains(authority)
                    && section.contains(projection)
            }),
            "{plane} section must state its own representative concept, authority, and projection"
        );
    }
}

#[trace("FR-117-AC-2")]
#[test]
fn tc_fr117_ac2_separates_test_definition_execution_and_report() {
    let planes = normalized(PLANES);
    for phrase in [
        "A `TestCase` definition",
        "One `TestExecution`",
        "A run report summarizing that execution",
        "These are linked objects, not three interchangeable encodings.",
        "Editing the run report does not mutate the `TestExecution`",
    ] {
        assert!(
            planes.contains(phrase),
            "missing definition/occurrence/presentation distinction: {phrase}"
        );
    }
}

#[trace("FR-117-AC-3")]
#[test]
fn tc_fr117_ac3_rejects_inference_from_structural_kind_and_universal_envelope() {
    let planes = normalized(PLANES);
    assert!(planes.contains("Structural kind and semantic role are independent."));
    assert!(planes.contains(
        "must not infer domain meaning from `record`, `enum`, a heading, or a table layout alone."
    ));
    assert!(planes.contains("No universal `SemanticObject` runtime envelope."));
}

#[trace("FR-117-AC-4")]
#[test]
fn tc_fr117_ac4_indexes_normative_records_and_marks_provisional_external_gates() {
    let index = normalized(INDEX);
    for link in [
        "[Planes and authority](planes-and-authority.md)",
        "[Ownership and boundaries](ownership-and-boundaries.md)",
        "[Dynamic and generated modules](dynamic-and-generated.md)",
        "[External decision ledger](decision-ledger.md)",
        "[ADR index](adr/index.md)",
    ] {
        assert!(index.contains(link), "architecture index is missing {link}");
    }
    assert!(index.contains("proposed normative"));
    let adr_index = normalized(ADR_INDEX);
    assert!(adr_index.contains("proposed"));
    assert!(adr_index.contains("Named Quoin/Quire maintainer review on issue #289 PR"));
    let ledger = normalized(LEDGER);
    assert!(ledger.contains("provisional; gate `filament-core-data#9`"));
    assert!(ledger.contains("ADR-0005 normative; ADR-0004 historical"));
}

#[trace("FR-118-AC-1")]
#[test]
fn tc_fr118_ac1_assigns_quire_typed_markdown_scope_and_exclusions() {
    let ownership = normalized(OWNERSHIP);
    assert!(
        ownership
            .contains("Quire | parse, validate, extract, address, and byte-splice typed Markdown with module-provided contracts | template rendering, cross-language generation, schema-package publication, catalog sourcing, application policy")
    );
    for exclusion in [
        "template rendering",
        "cross-language generation",
        "schema-package publication",
        "catalog sourcing",
        "application policy",
    ] {
        assert!(
            ownership.contains(exclusion),
            "Quire exclusion missing: {exclusion}"
        );
    }
}

#[trace("FR-118-AC-2")]
#[test]
fn tc_fr118_ac2_assigns_quoin_orchestration_and_excludes_other_owners_work() {
    let ownership = normalized(OWNERSHIP);
    assert!(ownership.contains("Quoin | catalog discovery, locks, installation, update; authoring-contract discovery, skills, workflows, and evidence-oriented orchestration"));
    for exclusion in [
        "parser semantics",
        "compiler implementation",
        "runtime persistence",
        "consumer adapters",
    ] {
        assert!(
            ownership.contains(exclusion),
            "Quoin exclusion missing: {exclusion}"
        );
    }
}

#[trace("FR-118-AC-3")]
#[test]
fn tc_fr118_ac3_assigns_shared_kernel_to_filament_and_vocabulary_to_modules() {
    let ownership = normalized(OWNERSHIP);
    assert!(ownership.contains(
        "`filament-core-data` | semantic kernel, IR, compatibility rules, compiler, and emitters"
    ));
    assert!(ownership.contains("Module repositories | vocabulary, constraints, archetypes, skeletons, mappings, examples, and semantic versions"));
}

#[trace("FR-118-AC-4")]
#[test]
fn tc_fr118_ac4_assigns_adapters_state_and_presentation_to_consumers() {
    let ownership = normalized(OWNERSHIP);
    assert!(ownership.contains("Consumers | application adapters, API and IPC projections, persistence mappings and migrations, runtime state, and UI presentation"));
    assert!(
        ownership.contains(
            "must not fork a shared semantic identity under the same package/type/version"
        )
    );
}

#[trace("FR-118-AC-5")]
#[test]
fn tc_fr118_ac5_preserves_validation_levels_roles_and_consumer_l2_execution() {
    let ownership = normalized(OWNERSHIP);
    for phrase in [
        "| L0 | Quire and Quoin themselves | Their own repository CI.",
        "| L1 | The specification corpus | Deterministic Quire validators and Quoin analyses/evidence policy.",
        "| L2 | A consumer implementation | Consumer CI always executes L2 work.",
        "A **Validator** determines a fact about the specification alone.",
        "An **Advisor** emits a typed verification obligation.",
        "A **Generator** proposes a consumer-owned test skeleton or seed.",
        "An **Auditor** binds consumer evidence back to obligations and applies policy.",
    ] {
        assert!(
            ownership.contains(phrase),
            "missing accepted ADR-0011 allocation: {phrase}"
        );
    }
}

#[trace("FR-119-AC-1")]
#[test]
fn tc_fr119_ac1_makes_reviewed_markdown_authoritative_for_durable_knowledge() {
    let planes = normalized(PLANES);
    assert!(planes.contains("Human/agent-authored durable knowledge | Reviewed typed Markdown plus the owning module contract"));
    assert!(
        planes.contains(
            "extracted JSON, graph records, search indexes, embeddings, and rendered views"
        )
    );
    assert!(planes.contains("Edit and review Markdown; re-extract"));
}

#[trace("FR-119-AC-2")]
#[test]
fn tc_fr119_ac2_makes_schema_package_source_authoritative_and_generated_packages_projections() {
    let planes = normalized(PLANES);
    assert!(planes.contains(
        "Shared schema/package definitions | Accepted schema/package source plus package metadata"
    ));
    assert!(planes.contains("generated Rust, TypeScript, and Python"));
    assert!(planes.contains("The generated Rust, TypeScript, and Python packages are reproducible outputs and never independent authorities."));
}

#[trace("FR-119-AC-3")]
#[test]
fn tc_fr119_ac3_cites_typespec_as_structural_source_without_reopening_choice() {
    let planes = normalized(PLANES);
    assert!(planes.contains("TypeSpec schema/package source (filament-core-data ADR-0005)"));
    assert!(planes.contains("normalized or bundled JSON Schema, language types, wire descriptors, and registry metadata are outputs or distribution forms."));
    assert!(plane_section(PLANES, "Meta plane").is_some_and(|section| {
        let section = normalized(section);
        let decision = section.to_lowercase();
        section.contains("TypeSpec schema/package source (filament-core-data ADR-0005)")
            && !decision.contains("preferred")
            && !decision.contains("fallback")
    }));
    assert!(
        heading_section(LEDGER, "TypeSpec and structural schema status").is_some_and(|section| {
            let section = normalized(section);
            let decision = section.to_lowercase();
            section.contains("TypeSpec is the structural source.")
                && section
                    .contains("JSON Schema 2020-12 and Protobuf are official-emitter projections.")
                && !decision.contains("preferred")
                && !decision.contains("fallback")
        })
    );
    assert!(
        normalized(AUTHORITY_ADR)
            .contains("TypeSpec is the structural schema source per filament-core-data ADR-0005")
    );
}

#[trace("FR-119-AC-4")]
#[test]
fn tc_fr119_ac4_assigns_transactional_and_operational_authority_to_owning_stores() {
    let planes = normalized(PLANES);
    assert!(
        planes.contains(
            "Transactional application state | Owning PostgreSQL database or event store"
        )
    );
    assert!(planes.contains("Operational observations | Owning run, evidence, or event store"));
    assert!(planes.contains(
        "Markdown report is a presentation projection of the referenced observations unless it is separately declared and reviewed as a new authored artifact"
    ));
}

#[trace("FR-119-AC-5")]
#[test]
fn tc_fr119_ac5_separates_interface_and_analytical_projections_with_declared_loss() {
    let planes = normalized(PLANES);
    assert!(planes.contains("Interface payload conformance | Versioned interface schema package"));
    assert!(planes.contains("Protobuf, JSON, and Avro payloads and clients"));
    assert!(planes.contains("Arrow and Parquet batches; CSV and TSV exports"));
    assert!(planes.contains("require declared loss, null, ordering, and type conventions"));
}

#[trace("FR-119-AC-6")]
#[test]
fn tc_fr119_ac6_stops_competing_authority_promotion_and_rejects_last_writer_wins() {
    let planes = normalized(PLANES);
    assert!(
        planes.contains("If two sources appear authoritative for one concern, promotion stops.")
    );
    assert!(planes.contains("whether one is a projection, whether the concerns are distinct, or whether a reviewed ADR and migration are required."));
    assert!(planes.contains("Last-writer-wins is rejected as an authority policy"));
}
