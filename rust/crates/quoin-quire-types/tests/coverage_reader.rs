// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The [`Obligation`] reader, against bytes a real quire produced (quoin#384).
//!
//! # Why the fixture is captured and not written
//!
//! FR-101 names the self-fixture tautology: a test asserting against bytes the
//! test itself invented proves that the author's idea of the format agrees
//! with the author's idea of the format. quoin does not own this format —
//! quire does — so a hand-written fixture here would be exactly that, and the
//! first field quire renamed would read as an empty string in a passing suite.
//!
//! `tests/fixtures/coverage-obligations.json` is therefore output from
//! `quire coverage --scope . --json` run over this repository, and
//! `tests/fixtures/provenance.json` records which quire produced it: version
//! string, resolved path and sha256 of the bytes at that path.
//!
//! # Why the digest is recorded but not compared against a live quire
//!
//! It would be better if this test resolved quire and compared. It cannot, and
//! the reason is worth writing down rather than rediscovering: CI's `rust` job
//! installs no quire at all, and the job that does have one **builds it from
//! source** (`.ci/quire-cli`), so its bytes are not the npm-installed shim's
//! bytes and never will be. A digest assertion here would fail in CI for a
//! reason unrelated to drift, which is the fastest way to get a gate disabled.
//!
//! So provenance is recorded for a human, and the two checks below are what
//! actually run. The second is the one that carries the weight: it verifies a
//! property of the captured bytes that a person writing a fixture by hand
//! could not produce, which is a machine-checkable answer to "did this really
//! come from the engine".

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::Path;

use quoin_quire_types::Obligation;
use sha2::{Digest as _, Sha256};

fn fixture() -> serde_json::Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/coverage-obligations.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "the captured fixture must be readable at {}: {e}",
            path.display()
        )
    });
    serde_json::from_str(&text).expect("the captured fixture must be JSON")
}

fn obligations(value: &serde_json::Value) -> &Vec<serde_json::Value> {
    value
        .get("obligations")
        .and_then(serde_json::Value::as_array)
        .expect("the capture must carry an `obligations` array")
}

/// Trace: FR-101
#[test]
fn reads_every_obligation_quire_emitted() {
    let value = fixture();
    let raw = obligations(&value);
    assert!(
        !raw.is_empty(),
        "a fixture with no obligations checks nothing"
    );

    for entry in raw {
        let obligation: Obligation = serde_json::from_value(entry.clone()).unwrap_or_else(|e| {
            panic!("quire emitted an obligation this reader cannot read: {e}\n{entry}")
        });
        // The two fields quoin reads, present and non-empty. A reader that
        // parses but yields "" is the silent-drift outcome this exists to
        // prevent, and `Option` would have produced exactly that.
        assert!(
            !obligation.id.is_empty(),
            "an obligation with no id: {entry}"
        );
        assert!(
            !obligation.statement.is_empty(),
            "an obligation with no statement: {entry}"
        );
    }
}

/// Trace: FR-101
#[test]
fn the_fixture_carries_engine_computed_digests() {
    // The anti-self-fixture check. `statement_hash` is sha256 of `statement`,
    // computed by the engine. Every captured row verifies, which a fixture
    // typed by a person would not — the author would have to have run the
    // digest themselves, and at that point they are capturing, not inventing.
    //
    // It also pins the pair: editing a `statement` in the fixture to make a
    // test convenient breaks the digest, so the bytes cannot be quietly
    // adjusted after capture.
    let value = fixture();
    for entry in obligations(&value) {
        let statement = entry
            .get("statement")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("no statement: {entry}"));
        let recorded = entry
            .get("statement_hash")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("no statement_hash: {entry}"));
        let computed = format!("{:x}", Sha256::digest(statement.as_bytes()));
        assert_eq!(
            computed, recorded,
            "statement_hash does not verify, so these bytes did not come from the engine: {entry}"
        );
    }
}

/// Trace: FR-101
#[test]
fn the_capture_still_covers_every_shape_quire_mints() {
    // The capture was selected by census: one obligation per distinct
    // (source, key-set) pair the run emitted, all six pairs present out of 991
    // obligations. Asserting the count here means a careless recapture that
    // narrows the fixture to one comfortable shape fails rather than passing
    // with less coverage than the name implies.
    let value = fixture();
    let mut shapes: Vec<(String, Vec<String>)> = obligations(&value)
        .iter()
        .map(|entry| {
            let source = entry
                .get("source")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| panic!("no source: {entry}"))
                .to_owned();
            let mut keys: Vec<String> = entry
                .as_object()
                .unwrap_or_else(|| panic!("not an object: {entry}"))
                .keys()
                .cloned()
                .collect();
            keys.sort();
            (source, keys)
        })
        .collect();
    shapes.sort();
    shapes.dedup();
    assert_eq!(
        shapes.len(),
        6,
        "the capture must exhibit all six (source, key-set) pairs; it exhibits {shapes:?}"
    );

    // All three obligation sources quire minted, named rather than counted —
    // a count of three could be one source appearing under three key sets.
    let sources: std::collections::BTreeSet<&str> =
        shapes.iter().map(|(source, _)| source.as_str()).collect();
    assert_eq!(
        sources.iter().copied().collect::<Vec<_>>(),
        vec![
            "acceptance-criterion",
            "nfr-acceptance-criterion",
            "nfr-metric"
        ]
    );
}
