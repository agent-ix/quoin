// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! `validateAssurance` and [`quoin_quire::schema::validate_assurance`] return
//! the same verdict (quoin#474).
//!
//! # The corpus, and how it was chosen
//!
//! `tests/goldens/assurance-verdicts.json` was captured once by
//! `oracle/capture-assurance-verdicts.mjs`, which **imports the oracle** —
//! `validateAssurance` from `src/quire/validate.ts:99` — rather than rebuilding
//! its ajv instance. The corpus is `base documents × a fixed named mutation
//! ladder`:
//!
//! * the bases are two real exports, produced by the linked engine over the two
//!   committed fixture corpora by `oracle/capture-assurance-bases.rs`. This
//!   repository retains no `assurance-v1` document to read instead, and the
//!   engine is the only thing that can write one;
//! * each base is mutated once per rung of a named ladder — every top-level key
//!   removed in turn, every key of the first element of every collection
//!   removed and nulled in turn, an unknown key added at each level, the two
//!   `const` tags broken, the `revision` and `digest` patterns broken, every arm
//!   of the `locator.path` traversal lookahead, and the `uuid` format.
//!
//! Mutation rather than hand-written fixtures, because a hand-written invalid
//! document proves whatever its author believed. [`CORPUS_FLOOR`] keeps the
//! measurement from passing by measuring nothing.
//!
//! # Verdicts are contractual; diagnostics are not
//!
//! Only `oracle_valid` is asserted. quoin#403 asked whether ajv's error
//! ordering could be promised across this port: it is not, and neither is the
//! error count or the message text. Nothing here asserts any of the three, and
//! `DIVERGENCE.md` says so in as many words.
//!
//! # No live oracle runs from here
//!
//! FR-101-AC-5 forbids a Rust test spawning node, pnpm or tsx. The capture is a
//! committed golden carrying the ajv version, the node version, the engine pin
//! and the quoin revision it was taken at; this test reads that file and
//! nothing else.
//!
//! Trace: FR-099-AC-2
//! Provenance: quoin#474

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeMap;
use std::path::Path;

use quoin_quire::schema::validate_assurance;
use serde_json::Value;

/// The count below which this parity measurement is measuring nothing.
///
/// 217 entries were captured. The floor sits below that so a rung that stops
/// applying is a red test rather than a silent shrink, and far enough below to
/// survive a deliberate ladder edit without being re-tuned every time.
const CORPUS_FLOOR: usize = 180;

/// The corpus entries where this crate's verdict deliberately differs from the
/// oracle's — `(mutation, oracle verdict, Rust verdict, the ruling)`.
///
/// **Empty, and that is the finding.** The port takes no ruling here: the
/// schema is vendored byte-for-byte and compiled with the `format` policy the
/// retained ajv had, so there was nothing to diverge about. The constant stays
/// because the assertion below is what makes "empty" a measured claim rather
/// than an absence of measurement — an undeclared disagreement fails, and a
/// declared one that stopped disagreeing fails too.
const DECLARED_DIVERGENCES: &[(&str, bool, bool, &str)] = &[];

fn goldens() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("goldens")
        .join("assurance-verdicts.json");
    serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display())),
    )
    .expect("the captured verdicts are JSON")
}

fn entries(goldens: &Value) -> &Vec<Value> {
    goldens["entries"].as_array().expect("entries is an array")
}

/// Every corpus document gets the same verdict from both implementations.
///
/// Trace: FR-099-AC-2
/// Provenance: quoin#474
#[test]
fn tc_474_020_the_oracle_and_this_crate_agree_on_every_verdict() {
    let goldens = goldens();
    let entries = entries(&goldens);
    assert!(
        entries.len() >= CORPUS_FLOOR,
        "anti-vacuity floor: at least {CORPUS_FLOOR} corpus documents expected, saw {} — a \
         parity measurement over nothing proves nothing",
        entries.len()
    );

    let mut agreed = 0_usize;
    let mut declared_seen: BTreeMap<&str, usize> = BTreeMap::new();

    for entry in entries {
        let id = entry["id"].as_str().expect("an id");
        let mutation = entry["mutation"].as_str().expect("a mutation name");
        let oracle_valid = entry["oracle_valid"].as_bool().expect("an oracle verdict");
        let outcome = validate_assurance(entry["document"].clone());
        let rust_valid = outcome.is_ok();

        if let Some((name, expect_oracle, expect_rust, ruling)) = DECLARED_DIVERGENCES
            .iter()
            .find(|(name, ..)| *name == mutation)
        {
            assert_eq!(
                (oracle_valid, rust_valid),
                (*expect_oracle, *expect_rust),
                "{id}: the declared divergence {name} ({ruling}) no longer moves the way it \
                 was recorded — the oracle said {oracle_valid}, this crate said {rust_valid}"
            );
            *declared_seen.entry(name).or_insert(0) += 1;
            continue;
        }

        assert_eq!(
            oracle_valid,
            rust_valid,
            "{id}: validateAssurance said {oracle_valid} and this crate said {rust_valid}. \
             The verdict is contractual. If the difference is intended, it is a ruling and \
             belongs in DIVERGENCE.md and in DECLARED_DIVERGENCES; otherwise it is a port \
             defect.\n{:?}",
            outcome.err()
        );
        agreed += 1;
    }

    assert_eq!(
        agreed + declared_seen.values().sum::<usize>(),
        entries.len(),
        "every corpus entry must be either an agreement or a declared divergence"
    );
    for (name, ..) in DECLARED_DIVERGENCES {
        assert!(
            declared_seen.get(name).is_some_and(|count| *count > 0),
            "the declared divergence {name} was never exercised: the corpus no longer \
             contains the document it was recorded against, so it is an unmeasured claim"
        );
    }
}

/// The corpus contains both accepted and refused documents, for both bases.
///
/// A parity run in which everything was refused would agree perfectly and prove
/// only that the schema never accepts anything.
///
/// Trace: FR-099-AC-2
/// Provenance: quoin#474
#[test]
fn tc_474_021_the_corpus_carries_both_verdicts_for_both_bases() {
    let goldens = goldens();
    let entries = entries(&goldens);
    let mut bases: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for entry in entries {
        let base = entry["base"].as_str().expect("a base name");
        let slot = bases.entry(base).or_insert((0, 0));
        slot.0 += 1;
        if entry["oracle_valid"].as_bool() == Some(true) {
            slot.1 += 1;
        }
    }
    assert_eq!(bases.len(), 2, "both fixture shapes must be in the corpus");
    for (base, (total, accepted)) in bases {
        assert!(
            accepted > 0 && accepted < total,
            "{base}: {accepted} of {total} corpus documents were accepted; a corpus that is \
             all one verdict cannot separate the two implementations"
        );
    }
}

/// The ladder's named rungs are all still applied.
///
/// A rung whose pointer no longer resolves is skipped silently by the capture,
/// so the corpus can shrink without the floor noticing which rung went. These
/// are the rungs that carry the argument — the traversal lookahead, the format
/// annotation, the two `const` tags — and each must still be present.
///
/// Trace: FR-099-AC-2
/// Provenance: quoin#474
#[test]
fn tc_474_022_the_load_bearing_rungs_are_still_in_the_corpus() {
    let goldens = goldens();
    let mutations: Vec<&str> = entries(&goldens)
        .iter()
        .filter_map(|entry| entry["mutation"].as_str())
        .collect();
    for rung in [
        "none",
        "set/format=wrong",
        "set/format_version=2",
        "set/source.revision=uppercase",
        "set/artifacts.0.locator.path=absolute",
        "set/artifacts.0.locator.path=dot-dot-leading",
        "set/artifacts.0.locator.path=dot-dot-interior",
        "set/artifacts.0.locator.path=dot-dot-trailing",
        "set/artifacts.0.locator.path=dot-dot-as-name-prefix",
        "set/artifacts.0.uuid=not-a-uuid",
        "set/artifacts.0.uuid=valid",
        "add/unknown-top-level-key",
    ] {
        assert!(
            mutations.contains(&rung),
            "the rung {rung} is no longer in the corpus; it carries part of the argument \
             and its absence is not a smaller corpus, it is an unmeasured claim"
        );
    }
}

/// The capture names its oracle, its ajv, its node and its revision.
///
/// Trace: FR-099-AC-2
/// Provenance: quoin#474
#[test]
fn tc_474_023_the_verdict_capture_records_its_provenance() {
    let goldens = goldens();
    for key in [
        "producer",
        "oracle",
        "ajv_version",
        "node_version",
        "bases",
        "bases_producer",
        "engine",
        "quoin_revision",
    ] {
        let value = goldens["provenance"][key].as_str().unwrap_or("");
        assert!(
            !value.is_empty(),
            "the verdict capture must name {key}; an oracle without its version and \
             revision cannot be re-derived or audited"
        );
    }
}
