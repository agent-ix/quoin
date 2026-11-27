// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-030 checks for stable cross-suite binding and locale-independent store ordering.

#![allow(
    clippy::expect_used,
    reason = "test fixtures deliberately panic to report a failed store contract"
)]

use ix_trace_rs::trace;
use quoin_evidence::record::{RecordRequest, record_run};
use quoin_evidence::store::read_bindings;
use quoin_evidence::types::{Outcome, RunEntry};
use quoin_evidence::{EvidenceSource, MemoryEvidence, STORE_SCHEMA_VERSION, SuiteId, SymbolId};
use quoin_quire_types::Obligation;

fn obligation() -> Obligation {
    Obligation {
        id: "FR-030-AC-11".to_owned(),
        identity: None,
        document: None,
        statement: "The evidence store retains each binding.".to_owned(),
        target_ids: None,
        method: Some("Test".to_owned()),
        criticality: None,
        parameters: None,
    }
}

fn entry(symbol: &str) -> RunEntry {
    let mut entry = RunEntry::new(symbol, Outcome::Pass);
    entry.trace_ids = Some(vec!["FR-030-AC-11".to_owned()]);
    entry
}

fn request(suite: &str, timestamp: &str, entries: Vec<RunEntry>) -> RecordRequest {
    RecordRequest {
        suite: SuiteId::new(suite),
        tool: "vitest 3.2.4".to_owned(),
        evidence_kind: None,
        lineage: None,
        timestamp: timestamp.to_owned(),
        entries,
    }
}

#[trace("FR-030-AC-11")]
#[test]
fn tc_fr030_bindings_append_across_suites_and_merge_within_a_suite() {
    let mut source = MemoryEvidence::new();
    let obligation = obligation();

    record_run(
        &mut source,
        &request(
            "SUITE-UNIT",
            "2026-11-01T00:00:00Z",
            vec![entry("tests::unit")],
        ),
        std::slice::from_ref(&obligation),
    )
    .expect("first suite is recorded");
    record_run(
        &mut source,
        &request(
            "SUITE-UNIT",
            "2026-11-02T00:00:00Z",
            vec![entry("tests::unit_followup")],
        ),
        std::slice::from_ref(&obligation),
    )
    .expect("a later run extends its suite binding");
    record_run(
        &mut source,
        &request(
            "SUITE-INTEGRATION",
            "2026-11-01T00:00:00Z",
            vec![entry("tests::api")],
        ),
        std::slice::from_ref(&obligation),
    )
    .expect("another suite appends an independent binding");

    let graph = read_bindings(&source).expect("binding graph reads");
    assert_eq!(graph.bindings.len(), 2);
    assert_eq!(graph.bindings[0].suite.as_str(), "SUITE-INTEGRATION");
    assert_eq!(graph.bindings[0].symbols, [SymbolId::new("tests::api")]);
    assert_eq!(graph.bindings[1].suite.as_str(), "SUITE-UNIT");
    assert_eq!(
        graph.bindings[1].symbols,
        [
            SymbolId::new("tests::unit"),
            SymbolId::new("tests::unit_followup")
        ]
    );
}

#[trace("FR-030-AC-14")]
#[test]
fn tc_fr030_binding_store_bytes_use_rust_order_not_locale_collation() {
    let mut source = MemoryEvidence::new();
    let obligation = obligation();
    record_run(
        &mut source,
        &request(
            "SUITE-ORDER",
            "2026-11-01T00:00:00Z",
            vec![entry("tests::zeta"), entry("tests::alpha")],
        ),
        std::slice::from_ref(&obligation),
    )
    .expect("record is stored");

    let stored = source
        .read("bindings.json")
        .expect("binding graph reads")
        .expect("binding graph is written");
    assert_eq!(
        stored,
        format!(
            "{{\n  \"bindings\": [\n    {{\n      \"obligation\": \"FR-030-AC-11\",\n      \"suite\": \"SUITE-ORDER\",\n      \"symbols\": [\n        \"tests::alpha\",\n        \"tests::zeta\"\n      ]\n    }}\n  ],\n  \"schemaVersion\": {STORE_SCHEMA_VERSION}\n}}\n"
        )
    );
}
