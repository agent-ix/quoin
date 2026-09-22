// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! ajv and the Rust `jsonschema` crate return the same verdict.
//!
//! # The corpus, and how it was chosen
//!
//! `tests/goldens/ajv-verdicts.json` was captured once by
//! `oracle/capture-ajv-verdicts.mjs` (deleted at cutover, recoverable from the
//! revision the golden records) under the two ajv configurations the
//! retained code uses verbatim (`intervention.ts:22-27`,
//! `operational.ts:30-35`). The corpus is **retained records × a fixed mutation
//! ladder**:
//!
//! * every record retained under `spec/evidence/interventions/` and
//!   `spec/evidence/operational/pairs/` is a base document;
//! * each base is mutated once per rung of a named ladder — every required key
//!   removed in turn, an unknown key added, the two `const` fields broken, the
//!   identity pattern broken, and the `observed_at` and `producer.tool_version`
//!   forms that the two recorded deltas move.
//!
//! Mutation rather than hand-written fixtures, because a hand-written invalid
//! document proves whatever its author believed. A required key removed from a
//! record that actually occurs proves the two validators agree about a real
//! document, minus one thing. [`CORPUS_FLOOR`] keeps the census from passing by
//! measuring nothing.
//!
//! # Verdicts are contractual; error text is not
//!
//! Only `ajv_valid` is asserted. The two libraries disagree about message
//! wording, about how many errors one refusal produces, and about the order
//! they arrive in — see `quoin_jsonschema::validator` and `DIVERGENCE.md`. What
//! they may never disagree about is whether a document satisfies the schema.
//!
//! # The exception, which is the point of the wave
//!
//! Two deltas were applied to the vendored intervention schema, so on the
//! corpus entries those deltas move, parity is a *declared* disagreement rather
//! than a defect: [`DECLARED_DIVERGENCES`] names each one and the direction it
//! must move in. An entry that disagrees without being listed fails, and a
//! listed entry that has stopped disagreeing fails too.
//!
//! Trace: FR-100-AC-4, FR-098

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeMap;
use std::path::Path;

use quoin_jsonschema::{MeasurementSchema, MeasurementValidator};
use quoin_measurement::date_time::Rfc3339DateTime;
use serde_json::Value;

/// The count below which this parity measurement is measuring nothing.
const CORPUS_FLOOR: usize = 100;

/// The corpus entries where the Rust verdict deliberately differs from ajv's.
///
/// `(mutation, ajv verdict, Rust verdict, the ruling)`. Every entry is one of
/// the two deltas recorded in `DIVERGENCE.md`, and both are intervention-only:
/// the operational schema is vendored verbatim, so its `blake3:` alternative
/// and its permissive `date-time` are unchanged.
const DECLARED_DIVERGENCES: &[(&str, bool, bool, &str)] = &[
    (
        "set/observed_at=lowercase-t-and-z",
        false,
        true,
        "quoin#440: RFC 3339 §5.6 permits lowercase t and z",
    ),
    (
        "set/observed_at=lowercase-t-only",
        false,
        true,
        "quoin#440: RFC 3339 §5.6 permits lowercase t",
    ),
    (
        "set/observed_at=lowercase-z-only",
        false,
        true,
        "quoin#440: RFC 3339 §5.6 permits lowercase z",
    ),
    (
        "set/producer.tool_version=blake3-prefixed",
        true,
        false,
        "quoin#409: blake3: has no producer and no instance",
    ),
];

/// The `date-time` check both retained ajv instances asked for, unified on the
/// permissive grammar per quoin#440. There is no second grammar here: this is
/// `quoin-measurement`'s, called.
fn is_rfc3339_date_time(value: &str) -> bool {
    Rfc3339DateTime::parse(value).is_ok()
}

fn goldens() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("goldens")
        .join("ajv-verdicts.json");
    serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display())),
    )
    .expect("the captured verdicts are JSON")
}

fn validators() -> BTreeMap<&'static str, MeasurementValidator> {
    let mut out = BTreeMap::new();
    out.insert(
        "intervention_experiment_v1",
        MeasurementSchema::InterventionExperimentV1
            .compile(is_rfc3339_date_time)
            .expect("the vendored intervention schema compiles"),
    );
    out.insert(
        "operational_evidence_v1",
        MeasurementSchema::OperationalEvidenceV1
            .compile(is_rfc3339_date_time)
            .expect("the vendored operational schema compiles"),
    );
    out
}

/// Every corpus document gets the same verdict from both validators, except
/// the declared divergences, which move in the declared direction.
///
/// Trace: FR-100-AC-4, FR-098
#[test]
fn tc_470_ajv_and_jsonschema_agree_on_every_verdict() {
    let goldens = goldens();
    let entries = goldens["entries"].as_array().expect("entries is an array");
    assert!(
        entries.len() >= CORPUS_FLOOR,
        "anti-vacuity floor: at least {CORPUS_FLOOR} corpus documents expected, saw {} — a \
         parity measurement over nothing proves nothing",
        entries.len()
    );

    let validators = validators();
    let mut agreed = 0_usize;
    let mut declared_seen: BTreeMap<&str, usize> = BTreeMap::new();

    for entry in entries {
        let id = entry["id"].as_str().expect("an id");
        let schema = entry["schema"].as_str().expect("a schema name");
        let mutation = entry["mutation"].as_str().expect("a mutation name");
        let ajv_valid = entry["ajv_valid"].as_bool().expect("an ajv verdict");
        let validator = validators.get(schema).expect("a known schema name");
        let rust_valid = validator.is_valid(&entry["document"]);

        let declared = DECLARED_DIVERGENCES
            .iter()
            .find(|(name, ..)| *name == mutation && schema == "intervention_experiment_v1");
        if let Some((name, expect_ajv, expect_rust, ruling)) = declared {
            assert_eq!(
                (ajv_valid, rust_valid),
                (*expect_ajv, *expect_rust),
                "{id}: the declared divergence {name} ({ruling}) no longer moves the way it was \
                 recorded — ajv said {ajv_valid}, this crate said {rust_valid}"
            );
            *declared_seen.entry(name).or_insert(0) += 1;
            continue;
        }

        assert_eq!(
            ajv_valid,
            rust_valid,
            "{id}: ajv said {ajv_valid} and this crate said {rust_valid}. The verdict is \
             contractual. If the difference is intended, it is a delta and belongs in \
             DIVERGENCE.md and in DECLARED_DIVERGENCES; otherwise it is a port defect.\n{:#?}",
            validator.errors(&entry["document"])
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
            "the declared divergence {name} was never exercised: the corpus no longer contains \
             the document it was recorded against, so it is an unmeasured claim"
        );
    }
}

/// The corpus contains both accepted and refused documents.
///
/// A parity run in which everything was refused would agree perfectly and prove
/// that the schema never accepts anything.
///
/// Trace: FR-100-AC-4
#[test]
fn tc_470_the_corpus_carries_both_verdicts_for_both_schemas() {
    let goldens = goldens();
    let entries = goldens["entries"].as_array().expect("entries is an array");
    for schema in ["intervention_experiment_v1", "operational_evidence_v1"] {
        let mine: Vec<&Value> = entries
            .iter()
            .filter(|e| e["schema"].as_str() == Some(schema))
            .collect();
        let accepted = mine
            .iter()
            .filter(|e| e["ajv_valid"].as_bool() == Some(true))
            .count();
        assert!(
            accepted > 0 && accepted < mine.len(),
            "{schema}: {accepted} of {} corpus documents were accepted; a corpus that is all \
             one verdict cannot separate the two validators",
            mine.len()
        );
    }
}

/// The capture names the ajv it ran and the revision it ran at.
///
/// Trace: FR-101-AC-11
#[test]
fn tc_470_the_verdict_capture_records_its_provenance() {
    let goldens = goldens();
    for key in [
        "producer",
        "ajv_version",
        "intervention_config",
        "operational_config",
        "quoin_revision",
    ] {
        let value = goldens["provenance"][key].as_str().unwrap_or("");
        assert!(
            !value.is_empty(),
            "the verdict capture must name {key}; an oracle without its version and revision \
             cannot be re-derived or audited"
        );
    }
}
