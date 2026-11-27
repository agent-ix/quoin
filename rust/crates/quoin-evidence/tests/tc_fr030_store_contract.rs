// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Direct evidence-store checks for the FR-030 append and ordering contracts.

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use quoin_evidence::source::{EvidenceSource, MemoryEvidence};
use quoin_evidence::store::{bind, write_bindings};
use quoin_evidence::types::{Binding, BindingsFile};
use quoin_evidence::{ObligationId, SuiteId, SymbolId, paths};

fn binding(obligation: &str, suite: &str, symbols: &[&str]) -> Binding {
    Binding {
        obligation: ObligationId::new(obligation),
        suite: SuiteId::new(suite),
        symbols: symbols
            .iter()
            .map(|symbol| SymbolId::new(*symbol))
            .collect(),
        lineage: None,
    }
}

#[trace("FR-030-AC-11")]
#[test]
fn distinct_suites_append_and_recharging_a_suite_merges_its_binding() {
    let unit = binding("FR-030-AC-11", "unit-suite", &["tests::unit"]);
    let integration = binding("FR-030-AC-11", "integration-suite", &["tests::http"]);

    let first = bind(&[], &unit);
    let second = bind(&first.bindings, &integration);
    let repeated = bind(
        &second.bindings,
        &binding(
            "FR-030-AC-11",
            "unit-suite",
            &["tests::unit", "tests::unit_extra"],
        ),
    );

    assert!(first.created);
    assert!(second.created);
    assert!(!repeated.created);
    assert_eq!(repeated.bindings.len(), 2);
    let suites: Vec<&str> = repeated
        .bindings
        .iter()
        .map(|entry| entry.suite.as_str())
        .collect();
    assert_eq!(suites.len(), 2);
    assert!(suites.contains(&"integration-suite"));
    assert!(suites.contains(&"unit-suite"));
    let mut store = MemoryEvidence::new();
    write_bindings(&mut store, &repeated.bindings).expect("bindings are persisted");
    let stored = store
        .read(&paths::bindings_path())
        .expect("bindings file is readable")
        .expect("bindings file was written");
    let file: BindingsFile = serde_json::from_str(&stored).expect("bindings JSON is valid");
    assert_eq!(file.bindings.len(), 2);
    assert_eq!(
        file.bindings
            .iter()
            .map(|entry| entry.suite.as_str())
            .collect::<Vec<_>>(),
        ["integration-suite", "unit-suite"]
    );
    assert_eq!(
        file.bindings[1].symbols,
        [
            SymbolId::new("tests::unit"),
            SymbolId::new("tests::unit_extra")
        ]
    );
}

#[trace("FR-030-AC-14")]
#[test]
fn binding_file_bytes_use_fixed_lexical_order() {
    let mut store = MemoryEvidence::new();
    let bindings = [
        binding("FR-b-AC-1", "suite", &["b"]),
        binding("FR-10-AC-1", "suite", &["ten"]),
        binding("FR-2-AC-1", "suite", &["two"]),
        binding("FR-A-AC-1", "suite", &["upper"]),
        binding("FR-a-AC-1", "suite", &["lower"]),
    ];

    write_bindings(&mut store, &bindings).expect("bindings are persisted canonically");
    let bytes = store
        .read(&paths::bindings_path())
        .expect("bindings file is readable")
        .expect("bindings file was written");
    assert_eq!(
        bytes,
        concat!(
            "{\n",
            "  \"bindings\": [\n",
            "    {\n",
            "      \"obligation\": \"FR-10-AC-1\",\n",
            "      \"suite\": \"suite\",\n",
            "      \"symbols\": [\n",
            "        \"ten\"\n",
            "      ]\n",
            "    },\n",
            "    {\n",
            "      \"obligation\": \"FR-2-AC-1\",\n",
            "      \"suite\": \"suite\",\n",
            "      \"symbols\": [\n",
            "        \"two\"\n",
            "      ]\n",
            "    },\n",
            "    {\n",
            "      \"obligation\": \"FR-A-AC-1\",\n",
            "      \"suite\": \"suite\",\n",
            "      \"symbols\": [\n",
            "        \"upper\"\n",
            "      ]\n",
            "    },\n",
            "    {\n",
            "      \"obligation\": \"FR-a-AC-1\",\n",
            "      \"suite\": \"suite\",\n",
            "      \"symbols\": [\n",
            "        \"lower\"\n",
            "      ]\n",
            "    },\n",
            "    {\n",
            "      \"obligation\": \"FR-b-AC-1\",\n",
            "      \"suite\": \"suite\",\n",
            "      \"symbols\": [\n",
            "        \"b\"\n",
            "      ]\n",
            "    }\n",
            "  ],\n",
            "  \"schemaVersion\": 1\n",
            "}\n",
        )
    );

    let mut reversed = bindings;
    reversed.reverse();
    write_bindings(&mut store, &reversed).expect("reordered input is persisted canonically");
    let reordered_bytes = store
        .read(&paths::bindings_path())
        .expect("bindings file is readable")
        .expect("bindings file was written");
    assert_eq!(reordered_bytes, bytes);
}
