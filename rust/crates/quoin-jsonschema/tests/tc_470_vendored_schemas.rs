// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The vendored schema documents are the retained ones, minus two named deltas.
//!
//! # Two documents, two different obligations
//!
//! `operational-evidence-v1.schema.json` was already a JSON document in the
//! retained tree, so the obligation is **byte equality**. Nothing was decided
//! about it and nothing may drift in either copy.
//!
//! `intervention-schema.ts` was not a document: it is a program that assembles
//! an object out of shared fragments (`identity`, `digest`, `immutableVersion`,
//! `rfc3339DateTime`, `closed()`). It was run once and serialized with the
//! repository's own `canonicalJson` into
//! `tests/goldens/intervention-experiment-v1.captured.json`. Two deltas were
//! then applied to the vendored document, each an owner ruling:
//!
//! 1. **RFC 3339 case widening** (quoin#440). The retained `pattern` accepts
//!    only uppercase `T`/`Z`, while `src/measurement/date-time.ts:2` — the
//!    grammar the *operational* schema validates against — accepts both cases.
//!    RFC 3339 §5.6 permits lowercase, so the strict spelling was narrower than
//!    the standard it names, and widening is the only direction that cannot
//!    invalidate an already-retained record.
//! 2. **`blake3:` removal** (quoin#409). `immutableVersion` admitted
//!    `(sha256|blake3):[a-f0-9]{64}`. `blake3:` has no producer and no instance
//!    anywhere in the repository, and `quoin_store::DigestDomain` mints no
//!    prefixed blake3 value at all.
//!
//! # Why the difference set, and not "it differs"
//!
//! A test asserting the two documents are not equal passes for a typo. This one
//! **enumerates** every JSON pointer at which they disagree and asserts the set
//! equals the two deltas, with the exact before and after text. A third,
//! unrecorded difference fails.
//!
//! Trace: FR-100-AC-4, FR-098

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_jsonschema::VendoredSchema;
use serde_json::Value;

/// One place two JSON documents disagree.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Difference {
    /// JSON pointer into both documents.
    pointer: String,
    /// The captured TypeScript value, rendered as compact JSON.
    captured: String,
    /// The vendored value, rendered as compact JSON.
    vendored: String,
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()))
}

/// Every pointer at which `left` and `right` disagree.
///
/// A key present on one side only is reported at its own pointer, so a dropped
/// or added property cannot hide inside a parent's difference.
fn differences(pointer: &str, left: &Value, right: &Value, into: &mut Vec<Difference>) {
    match (left, right) {
        (Value::Object(l), Value::Object(r)) => {
            let mut keys: Vec<&String> = l.keys().chain(r.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                let escaped = key.replace('~', "~0").replace('/', "~1");
                let child = format!("{pointer}/{escaped}");
                match (l.get(key), r.get(key)) {
                    (Some(a), Some(b)) => differences(&child, a, b, into),
                    (a, b) => into.push(Difference {
                        pointer: child,
                        captured: a.map_or_else(|| "<absent>".to_owned(), ToString::to_string),
                        vendored: b.map_or_else(|| "<absent>".to_owned(), ToString::to_string),
                    }),
                }
            }
        }
        (Value::Array(l), Value::Array(r)) if l.len() == r.len() => {
            for (index, (a, b)) in l.iter().zip(r.iter()).enumerate() {
                differences(&format!("{pointer}/{index}"), a, b, into);
            }
        }
        (a, b) if a == b => {}
        (a, b) => into.push(Difference {
            pointer: pointer.to_owned(),
            captured: a.to_string(),
            vendored: b.to_string(),
        }),
    }
}

/// The vendored operational schema is byte-identical to the retained one.
///
/// Trace: FR-100-AC-4
#[test]
fn tc_470_the_operational_schema_is_vendored_byte_for_byte() {
    let retained = read(
        &repo()
            .join("src")
            .join("measurement")
            .join("schemas")
            .join("operational-evidence-v1.schema.json"),
    );
    assert_eq!(
        VendoredSchema::OperationalEvidenceV1.source(),
        retained,
        "the vendored operational schema and \
         src/measurement/schemas/operational-evidence-v1.schema.json have diverged. Neither copy \
         may move: this schema is vendored verbatim and no ruling applies to it."
    );
}

/// The vendored intervention schema differs from the capture by exactly two
/// recorded deltas.
///
/// Trace: FR-100-AC-4, FR-098
#[test]
fn tc_470_the_intervention_schema_carries_exactly_the_two_recorded_deltas() {
    let captured: Value = serde_json::from_str(&read(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("goldens")
            .join("intervention-experiment-v1.captured.json"),
    ))
    .expect("the capture is JSON");
    let vendored = VendoredSchema::InterventionExperimentV1
        .document()
        .expect("the vendored schema is JSON");

    let mut found = Vec::new();
    differences("", &captured, &vendored, &mut found);
    found.sort();

    let expected = vec![
        // Delta 2 — quoin#409: `blake3:` has no producer and no instance.
        Difference {
            pointer: "/properties/producer/properties/tool_version/pattern".to_owned(),
            captured: "\"^(v?[0-9]+[.][0-9]+[.][0-9]+([-+][0-9A-Za-z.-]+)?|[a-f0-9]{40}|(sha256|blake3):[a-f0-9]{64})$\"".to_owned(),
            vendored: "\"^(v?[0-9]+[.][0-9]+[.][0-9]+([-+][0-9A-Za-z.-]+)?|[a-f0-9]{40}|sha256:[a-f0-9]{64})$\"".to_owned(),
        },
        // Delta 1 — quoin#440: RFC 3339 §5.6 permits lowercase `t` and `z`.
        Difference {
            pointer: "/properties/observed_at/pattern".to_owned(),
            captured: "\"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(?:[.][0-9]+)?(?:Z|[+-][0-9]{2}:[0-9]{2})$\"".to_owned(),
            vendored: "\"^[0-9]{4}-[0-9]{2}-[0-9]{2}[Tt][0-9]{2}:[0-9]{2}:[0-9]{2}(?:[.][0-9]+)?(?:[Zz]|[+-][0-9]{2}:[0-9]{2})$\"".to_owned(),
        },
    ];
    let mut expected = expected;
    expected.sort();

    assert_eq!(
        found, expected,
        "the vendored intervention schema no longer differs from the TypeScript capture by \
         exactly the two recorded deltas (quoin#440 case widening, quoin#409 blake3 removal). \
         An unrecorded difference is a port defect, not a nuance: record it in DIVERGENCE.md \
         and here, or remove it."
    );
}

/// The capture that the delta set is measured against names where it came from.
///
/// Trace: FR-101-AC-11
#[test]
fn tc_470_the_capture_records_its_producer_and_revision() {
    let provenance: Value = serde_json::from_str(&read(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("goldens")
            .join("intervention-experiment-v1.captured.provenance.json"),
    ))
    .expect("the provenance is JSON");
    for key in [
        "captured_from",
        "exported_binding",
        "serializer",
        "producer",
        "quoin_revision",
    ] {
        let value = provenance.get(key).and_then(Value::as_str).unwrap_or("");
        assert!(
            !value.is_empty(),
            "the capture provenance must name {key}; a golden without its producing revision \
             cannot be re-derived or audited"
        );
    }
}

/// Both vendored documents parse, and the census is not measuring nothing.
///
/// Trace: FR-100-AC-4
#[test]
fn tc_470_every_vendored_schema_parses() {
    assert_eq!(
        VendoredSchema::ALL.len(),
        2,
        "anti-vacuity floor: two schemas are vendored here"
    );
    for schema in VendoredSchema::ALL {
        let document = schema
            .document()
            .unwrap_or_else(|e| panic!("{schema} must parse: {e}"));
        assert_eq!(
            document.get("$id").and_then(Value::as_str),
            Some(schema.id()),
            "{schema} must carry the $id this crate reports for it"
        );
    }
}
