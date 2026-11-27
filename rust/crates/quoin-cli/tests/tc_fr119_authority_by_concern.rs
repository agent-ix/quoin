// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Keep the authority-by-concern ADR aligned with FR-119.

use ix_trace_rs::trace;

const ADR: &str =
    include_str!("../../../../docs/semantic-module-architecture/adr/0001-authority-by-concern.md");
const AUTHORITY: &str =
    include_str!("../../../../docs/semantic-module-architecture/planes-and-authority.md");
const LEDGER: &str =
    include_str!("../../../../docs/semantic-module-architecture/decision-ledger.md");

fn contains_normalized(document: &str, phrase: &str) -> bool {
    document
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .contains(&phrase.split_whitespace().collect::<Vec<_>>().join(" "))
}

#[trace("FR-119-AC-1")]
#[test]
fn reviewed_markdown_owns_authored_knowledge_and_extracted_views_are_derived() {
    assert!(contains_normalized(
        AUTHORITY,
        "| Human/agent-authored durable knowledge | Reviewed typed Markdown plus the owning module contract"
    ));
    assert!(contains_normalized(
        AUTHORITY,
        "extracted JSON, graph records, search indexes, embeddings, and rendered views"
    ));
    assert!(contains_normalized(
        ADR,
        "Reviewed typed Markdown owns authored durable knowledge"
    ));
}

#[trace("FR-119-AC-2")]
#[test]
fn accepted_schema_package_source_owns_definitions_and_generated_packages_are_outputs() {
    assert!(contains_normalized(
        AUTHORITY,
        "| Shared schema/package definitions"
    ));
    assert!(contains_normalized(
        AUTHORITY,
        "Accepted schema/package source plus package metadata"
    ));
    assert!(contains_normalized(
        AUTHORITY,
        "generated Rust, TypeScript, and Python"
    ));
    assert!(contains_normalized(
        ADR,
        "accepted schema/package source plus metadata owns shared semantic definitions"
    ));
    assert!(contains_normalized(
        AUTHORITY,
        "never independent authorities"
    ));
}

#[trace("FR-119-AC-3")]
#[test]
fn typespec_source_is_cited_as_the_owner_decision_without_a_fallback() {
    assert!(contains_normalized(
        AUTHORITY,
        "TypeSpec schema/package source (filament-core-data ADR-0005)"
    ));
    assert!(contains_normalized(LEDGER, "ADR-0005 / issue #4 decision"));
    assert!(LEDGER.contains("agent-ix/filament-core-data"));
    assert!(contains_normalized(
        LEDGER,
        "ADR-0005 normative; ADR-0004 historical"
    ));
    assert!(contains_normalized(
        LEDGER,
        "TypeSpec is the structural source. The owner resolved filament-core-data ADR-0004"
    ));
    assert!(!ADR.contains("Modular JSON Schema as the authoring source"));
}

#[trace("FR-119-AC-4")]
#[test]
fn transaction_and_operational_stores_own_state_while_reports_are_projections() {
    assert!(contains_normalized(
        AUTHORITY,
        "| Transactional application state | Owning PostgreSQL database or event store"
    ));
    assert!(contains_normalized(
        AUTHORITY,
        "| Operational observations | Owning run, evidence, or event store"
    ));
    assert!(contains_normalized(
        AUTHORITY,
        "A Markdown report is a presentation projection of the referenced observations"
    ));
    assert!(contains_normalized(
        ADR,
        "application and event stores own transactional state"
    ));
    assert!(contains_normalized(
        ADR,
        "run/evidence/event stores own operational observations"
    ));
}

#[trace("FR-119-AC-5")]
#[test]
fn interface_and_analytical_formats_are_projections_with_declared_loss() {
    assert!(contains_normalized(
        AUTHORITY,
        "| Interface payload conformance | Versioned interface schema package"
    ));
    assert!(contains_normalized(
        AUTHORITY,
        "Protobuf, JSON, and Avro payloads and clients"
    ));
    assert!(contains_normalized(
        AUTHORITY,
        "Arrow and Parquet batches; CSV and TSV exports"
    ));
    assert!(contains_normalized(
        AUTHORITY,
        "require declared loss, null, ordering, and type conventions"
    ));
    assert!(contains_normalized(
        ADR,
        "versioned interface packages own payload conformance"
    ));
}

#[trace("FR-119-AC-6")]
#[test]
fn competing_authorities_stop_promotion_and_reject_last_writer_wins() {
    assert!(contains_normalized(
        AUTHORITY,
        "If two sources appear authoritative for one concern, promotion stops"
    ));
    assert!(contains_normalized(
        AUTHORITY,
        "whether one is a projection, whether the concerns are distinct, or whether a reviewed ADR and migration are required"
    ));
    assert!(contains_normalized(
        AUTHORITY,
        "Last-writer-wins is rejected as an authority policy"
    ));
    assert!(contains_normalized(
        AUTHORITY,
        "hides the ownership error instead of resolving it"
    ));
}
