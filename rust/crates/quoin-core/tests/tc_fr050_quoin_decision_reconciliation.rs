// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Local reconciliation records preserve the referenced Quire decisions.

use ix_trace_rs::trace;

const ADR: &str = include_str!(
    "../../../../docs/semantic-module-architecture/adr/0002-preserve-quire-quoin-boundaries.md"
);
const LEDGER: &str =
    include_str!("../../../../docs/semantic-module-architecture/decision-ledger.md");

fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[trace("FR-050-AC-1")]
#[test]
fn tc_fr050_adr0003_stays_a_structural_archetype_model() {
    assert!(
        normalized(ADR).contains("ADR-0003's unified archetype shape is retained as a structural parser model, not promoted into a universal semantic base class."),
        "{ADR}"
    );
    assert!(
        normalized(LEDGER).contains(
            "preserved as a structural parsing model, not a universal semantic runtime base class"
        ),
        "{LEDGER}"
    );
}

#[trace("FR-050-AC-2")]
#[test]
fn tc_fr050_adr0004_keeps_direct_typed_and_canonical_markdown() {
    assert!(
        normalized(ADR).contains("ADR-0004's direct Markdown direction is retained and clarified by the current document-boundary canonical Markdown contract."),
        "{ADR}"
    );
    assert!(
        normalized(LEDGER).contains("direct typed Markdown preserved; canonical Markdown within the document boundary clarified by current spec"),
        "{LEDGER}"
    );
}

#[trace("FR-050-AC-3")]
#[test]
fn tc_fr050_adr0002_marks_rendering_historical_and_preserves_byte_splicing() {
    assert!(
        normalized(ADR).contains("byte-splices typed Markdown"),
        "{ADR}"
    );
    assert!(
        normalized(ADR).contains("its draft rendering allocation is historical"),
        "{ADR}"
    );
    assert!(
        normalized(LEDGER).contains("rendering responsibility is historical; parse/extract/address and byte-splicing remains preserved"),
        "{LEDGER}"
    );
}

#[trace("FR-050-AC-4")]
#[test]
fn tc_fr050_accepted_adr0011_remains_governing() {
    assert!(
        normalized(ADR).contains(
            "Accepted ADR-0011 remains governing for validation levels and capability roles."
        ),
        "{ADR}"
    );
    assert!(normalized(LEDGER).contains("ADR-0011"), "{LEDGER}");
    assert!(
        normalized(LEDGER).contains("L0/L1/L2 validation"),
        "{LEDGER}"
    );
}

#[trace("FR-050-AC-5")]
#[test]
fn tc_fr050_quire_and_quoin_keep_their_distinct_roles() {
    assert!(
        normalized(ADR).contains("Quire does not become a renderer or cross-language generator. Quoin does not become the parser or semantic compiler."),
        "{ADR}"
    );
    assert!(
        normalized(ADR).contains(
            "Quire parses, validates, extracts, addresses, and byte-splices typed Markdown"
        ),
        "{ADR}"
    );
    assert!(
        normalized(ADR).contains("Quoin discovers and distributes modules"),
        "{ADR}"
    );
}
