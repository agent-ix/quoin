// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The [`Obligation`] reader, against bytes a real quire produced (quoin#384).
//!
//! # Why the fixture is captured and not written
//!
//! The retire-after-parity rule names the self-fixture tautology: a test asserting against bytes the
//! test itself invented proves that the author's idea of the format agrees
//! with the author's idea of the format. quoin does not own this format —
//! quire does — so a hand-written fixture here would be exactly that, and the
//! first field quire renamed would read as an empty string in a passing suite.
//!
//! `tests/fixtures/coverage-obligations.json` is therefore output from
//! `quire coverage --scope . --json` run over this repository.
//!
//! The second check below is the one that carries the weight: it verifies a
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
