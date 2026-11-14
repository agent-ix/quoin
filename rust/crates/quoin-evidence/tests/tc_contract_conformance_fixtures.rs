// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The contract-conformance adapter read against rows captured from the real
//! `filament-contract-conformance` runner (`tests/fixtures/evidence/`).

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use quoin_evidence::Outcome;
use quoin_evidence::adapters::{PROTOCOL, parse_contract_conformance};

const REAL: &str =
    include_str!("../../../../tests/fixtures/evidence/contract-conformance-real.jsonl");
const TRACES: &str =
    include_str!("../../../../tests/fixtures/evidence/contract-conformance-traces-real.jsonl");
const CRITERIA: &str =
    include_str!("../../../../tests/fixtures/evidence/contract-conformance-criteria-real.jsonl");

/// Every captured row declares the adapter's protocol, so the fixtures prove
/// `PROTOCOL` against the producer rather than against itself.
#[test]
fn the_real_rows_declare_the_adapters_protocol() {
    for raw in [REAL, TRACES, CRITERIA] {
        for line in raw.lines() {
            assert!(line.contains(&format!(r#""protocol":"{PROTOCOL}""#)));
        }
    }
}

#[test]
fn the_real_corpus_rows_parse_one_entry_per_fixture_and_operation() {
    let entries = parse_contract_conformance(REAL).unwrap().entries;
    let symbols: Vec<&str> = entries.iter().map(|e| e.symbol.as_str()).collect();
    assert_eq!(
        symbols,
        [
            "contract-v0.1::package::package-constructs",
            "contract-v0.1::package::package-invalid-namespace",
            "contract-v0.1::expression::expression-boolean-literal",
            "contract-v0.1::expression::expression-invalid-wire",
            "contract-v0.1::coverage::coverage-shallow",
        ]
    );
    assert!(entries.iter().all(|e| e.outcome == Outcome::Pass));
}

#[test]
fn the_real_trace_rows_keep_their_ids_in_order() {
    let traces = parse_contract_conformance(TRACES).unwrap().entries;
    assert_eq!(
        traces[0].trace_ids.as_deref().unwrap(),
        [
            "FR-011-AC-1",
            "FR-012-AC-2",
            "FR-012-AC-4",
            "FR-012-AC-6",
            "FR-014-AC-3",
            "FR-016-AC-1",
            "FR-018-AC-1"
        ]
    );
    let criteria = parse_contract_conformance(CRITERIA).unwrap().entries;
    assert_eq!(
        criteria[0].trace_ids.as_deref().unwrap(),
        ["FR-011-AC-3", "FR-018-AC-1"]
    );
}
