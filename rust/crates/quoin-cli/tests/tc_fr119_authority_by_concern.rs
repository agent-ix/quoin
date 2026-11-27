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

#[trace("FR-119-AC-1")]
#[test]
fn reviewed_markdown_owns_authored_knowledge_and_extracted_views_are_derived() {
    assert!(AUTHORITY.contains(
        "| Human/agent-authored durable knowledge | Reviewed typed Markdown plus the owning module contract"
    ));
    assert!(
        AUTHORITY.contains(
            "extracted JSON, graph records, search indexes, embeddings, and rendered views"
        )
    );
    assert!(ADR.contains("Reviewed typed Markdown owns authored durable knowledge"));
}

#[trace("FR-119-AC-2")]
#[test]
fn accepted_schema_package_source_owns_definitions_and_generated_packages_are_outputs() {
    assert!(AUTHORITY.contains("| Shared schema/package definitions"));
    assert!(AUTHORITY.contains("Accepted schema/package source plus package metadata"));
    assert!(AUTHORITY.contains("generated Rust, TypeScript, and Python"));
    assert!(
        ADR.contains(
            "accepted schema/package source plus metadata owns shared semantic definitions"
        )
    );
    assert!(ADR.contains("never become independent authorities"));
}

#[trace("FR-119-AC-3")]
#[test]
fn typespec_source_is_cited_as_the_owner_decision_without_a_fallback() {
    assert!(
        ADR.contains("TypeSpec is the structural schema source per filament-core-data ADR-0005")
    );
    assert!(ADR.contains("Modular JSON Schema as the authoring source"));
    assert!(ADR.contains("rejected. TypeSpec is the structural schema source"));
    assert!(LEDGER.contains("`ADR-0005 / issue #4 decision`"));
    assert!(LEDGER.contains("ADR-0005 normative"));
}

#[trace("FR-119-AC-4")]
#[test]
fn transaction_and_operational_stores_own_state_while_reports_are_projections() {
    assert!(
        AUTHORITY.contains(
            "| Transactional application state | Owning PostgreSQL database or event store"
        )
    );
    assert!(
        AUTHORITY.contains("| Operational observations | Owning run, evidence, or event store")
    );
    assert!(AUTHORITY.contains("Markdown report is a presentation projection"));
    assert!(ADR.contains("application and event stores own transactional state"));
    assert!(ADR.contains("run/evidence/event stores own operational observations"));
}

#[trace("FR-119-AC-5")]
#[test]
fn interface_and_analytical_formats_are_projections_with_declared_loss() {
    assert!(
        AUTHORITY.contains("| Interface payload conformance | Versioned interface schema package")
    );
    assert!(AUTHORITY.contains("Protobuf, JSON, and Avro payloads and clients"));
    assert!(AUTHORITY.contains("Arrow and Parquet batches; CSV and TSV exports"));
    assert!(AUTHORITY.contains("require declared loss, null, ordering, and type conventions"));
    assert!(ADR.contains("Versioned interface packages own payload conformance"));
}

#[trace("FR-119-AC-6")]
#[test]
fn competing_authorities_stop_promotion_and_reject_last_writer_wins() {
    assert!(ADR.contains("Stop promotion when two sources claim the same concern"));
    assert!(ADR.contains("projection, concern split, or successor ADR/migration resolves it"));
    assert!(ADR.contains("Last writer wins between representations"));
    assert!(ADR.contains("rejected because it hides the ownership error"));
}
