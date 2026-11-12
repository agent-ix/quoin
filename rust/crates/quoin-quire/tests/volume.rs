// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A synthetic payload the size of the one that killed six commands, read
//! through the Cargo edge.
//!
//! Nothing here runs `node`, `quire`, or any other process, and nothing is
//! checked in: the payload is generated deterministically at test time.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::sync::LazyLock;

use ix_trace_rs::trace;
use quoin_quire::{CoveragePayload, ErrorCode, PayloadLimit, payload};

const OBLIGATIONS: usize = 2_800;
const CRITERIA: usize = 1_100;
const UNBACKED_ROWS: usize = 1_500;

/// The generated payload: realistic structure, synthetic ids and text, no
/// randomness, so every run reads the same bytes.
static VOLUME: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let obligations: Vec<_> = (0..OBLIGATIONS)
        .map(|n| {
            let fr = n / 4 + 1;
            serde_json::json!({
                "source": "acceptance-criterion",
                "id": format!("FR-{fr:03}-AC-{}", n % 4 + 1),
                "document": format!("spec/area-{}/functional/FR-{fr:03}-synthetic-feature.md", fr % 9),
                "statement": format!(
                    "Every operation {n} round-trips: a serialized envelope deserializes to an equal value"
                ),
                "method": "Test",
                "target_ids": [format!("TC-{}", n * 3 + 1), format!("TC-{}", n * 3 + 2), format!("TC-{}", n * 3 + 3)],
            })
        })
        .collect();
    let criteria: Vec<_> = (0..CRITERIA)
        .map(|n| {
            serde_json::json!({
                "document": format!("spec/area-{}/functional/FR-{:03}-synthetic-feature.md", n % 9, n + 1),
                "archetype": "FR",
                "criteria": 8,
                "property_shaped": 7,
                "specific_shaped": 1,
                "by_property": {"example": 1, "round-trip": 1, "universal": 6},
                "grounding": {
                    "example": {"records": 1, "domain": 0, "precondition": 0, "oracle": 0, "all_three": 0},
                    "round-trip": {"records": 1, "domain": 0, "precondition": 0, "oracle": 0, "all_three": 0},
                    "universal": {"records": 6, "domain": 5, "precondition": 0, "oracle": 5, "all_three": 0},
                },
            })
        })
        .collect();
    let unbacked_rows: Vec<_> = (0..UNBACKED_ROWS)
        .map(|n| {
            serde_json::json!({
                "reference": "constraint-validation",
                "document": format!("spec/area-{}/functional/FR-{:03}-synthetic-feature.md", n % 9, n + 1),
                "row_id": format!("FR-{:03}-CON-{}", n + 1, n % 6 + 1),
                "target_ids": [format!("FR-{:03}-CON-{}", n + 1, n % 6 + 1), format!("TC-{}", n + 1)],
                "line": n + 40,
            })
        })
        .collect();
    let value = serde_json::json!({
        "unbacked_rows": unbacked_rows,
        "status_lies": [],
        "no_symbol_rows": [],
        "untracked_symbols": [],
        "shared_trace_ids": [],
        "groups": [],
        "minted_targets": [],
        "diagnostics": [],
        "diagnostic_reason_registry": [],
        "vocabulary_coverage": [],
        "binding_census": [],
        "unmatched_tags": [],
        "metrics": [],
        "suspicions": [],
        "criteria": criteria,
        "obligations": obligations,
        "totals": {
            "backed": 1_555,
            "total": OBLIGATIONS,
            "criteria": CRITERIA,
            "property_shaped": 553,
            "specific_shaped": 79,
        },
        "engine": {
            "cli": "0.0.0",
            "engine": "synthetic",
            "capabilities": ["binding_census", "metrics_envelope", "minted_targets"],
        },
    });
    let bytes = serde_json::to_vec(&value).expect("the synthetic payload serializes");
    assert!(
        bytes.len() > 1_400_000,
        "the payload must exceed 1 MiB to mean anything: {} bytes",
        bytes.len(),
    );
    bytes
});

///
/// The headline: a payload the size of the one that killed six commands is read whole, by value, with no ceiling anywhere near it.
#[test]
fn tc_379_100_the_volume_payload_is_read_whole() {
    let parsed: CoveragePayload = payload::from_slice(
        "the synthetic coverage payload",
        "CoverageReport",
        &VOLUME,
        PayloadLimit::DEFAULT,
    )
    .expect("the payload reads");

    // The read loses nothing: each collection has exactly the length it has in
    // the raw bytes, which is read here independently of the engine's structs.
    let raw: serde_json::Value = serde_json::from_slice(&VOLUME).expect("the payload is JSON");
    let length = |key: &str| raw[key].as_array().map_or(0, Vec::len);
    assert!(length("obligations") > 0, "the payload carries obligations");
    assert_eq!(parsed.report.unbacked_rows.len(), length("unbacked_rows"));
    assert_eq!(parsed.report.obligations.len(), length("obligations"));
    assert_eq!(parsed.report.criteria.len(), length("criteria"));

    // The provenance block survived the read rather than being dropped by the
    // flattened envelope — the field `quire_rs::CoverageReport` does not have
    // and every stored artifact does.
    let provenance = parsed
        .engine
        .as_ref()
        .expect("the payload carries provenance");
    assert_ne!(provenance.cli.len(), 0);
    assert_ne!(provenance.engine.len(), 0);
}

/// Round-trip, in the sense that matters for an evidence store: re-encoding
/// the value and reading it again must yield the same value. NFR-025 makes
/// stored bytes immutable across the port, so a reader that quietly drops a
/// field would be unrecoverable — and a field dropped on the way in is
/// invisible in a single read.
#[trace("NFR-025")]
#[test]
fn tc_379_101_the_volume_payload_round_trips() {
    let first: CoveragePayload = payload::from_slice(
        "the synthetic coverage payload",
        "CoverageReport",
        &VOLUME,
        PayloadLimit::DEFAULT,
    )
    .expect("the payload reads");

    let encoded = payload::encode("re-encoded coverage", &first, PayloadLimit::DEFAULT)
        .expect("the value encodes under the ceiling");
    let second: CoveragePayload = payload::from_slice(
        "re-encoded coverage",
        "CoverageReport",
        encoded.as_bytes(),
        PayloadLimit::DEFAULT,
    )
    .expect("the re-encoded payload reads");

    assert_eq!(first, second, "the value did not survive a round trip");

    // Re-encoding twice is byte-stable. Not asserted against the *original*
    // bytes: the engine omits empty collections, so a payload whose producer
    // emitted `"no_symbol_rows": []` would re-encode shorter while meaning the
    // same thing. Value equality above is the claim; this is determinism.
    let again =
        payload::encode("re-encoded coverage", &second, PayloadLimit::DEFAULT).expect("encodes");
    assert_eq!(encoded, again);
}

///
/// The bound is real, and it names itself. `src/quire/exec.ts` could only
/// report this as a process killed by `ENOBUFS`; here it is a refusal that
/// says which payload, how big, and against what ceiling.
#[test]
fn tc_379_102_a_ceiling_below_the_payload_refuses_it_by_name() {
    let limit = PayloadLimit::bytes(1_000_000).expect("non-zero");
    let error = payload::from_slice::<CoveragePayload>(
        "the synthetic coverage payload",
        "CoverageReport",
        &VOLUME,
        limit,
    )
    .expect_err("a payload over the ceiling must be refused");

    assert_eq!(error.code(), ErrorCode::PayloadTooLarge);
    let message = error.to_string();
    assert!(message.contains(&VOLUME.len().to_string()), "{message}");
    assert!(message.contains("1000000"), "{message}");
    assert!(
        message.contains("the synthetic coverage payload"),
        "the refusal must name what was being read: {message}",
    );
}
