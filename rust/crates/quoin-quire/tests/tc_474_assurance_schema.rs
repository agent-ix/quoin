// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The vendored `assurance-v1` document is the retained one, and it is
//! compiled with the `format` policy the retained ajv actually had (quoin#474).
//!
//! # Two anchors, not one
//!
//! 1. The vendored bytes hash to [`schema::VENDORED_SHA256`], the digest taken when the
//!    file was copied out of the pinned `quire-rs` git object. Until quoin#502
//!    there was a second anchor — byte-equality with the retained
//!    `src/quire/schemas/assurance-v1.schema.json` — and it was the weaker of
//!    the two: it would pass if somebody edited **both** copies. The retained
//!    tree is gone; the digest, derived from upstream and editable from
//!    neither side, is the one that was carrying the weight.
//! 2. The document reaches the validator through `include_str!`. A schema read
//!    with `std::fs` at run time is a schema that can differ from the one these
//!    tests measured, so the vendored side is always the compiled-in constant.
//!
//! # The `format` policy is asserted from both sides
//!
//! The retained `src/quire/validate.ts` registered no format check (quoin#502
//! deleted it), so `format: "uuid"` on
//! `artifact.uuid` is an annotation and `"not-a-uuid"` is accepted. ajv says so
//! out loud when the capture runs: *unknown format "uuid" ignored in schema at
//! path "#/properties/uuid"*.
//!
//! So this crate compiles with `compile_vendored`. A test that only asserted
//! "the valid document is accepted" would pass either way; these assert the
//! **difference** — the same document, the same schema, one compiled with a
//! registered `uuid` check and one without, and only one of them refuses.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::Path;

use quoin_jsonschema::{FormatCheck, SchemaValidator};
use quoin_quire::schema;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn corpus() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("goldens")
        .join("assurance-bases.json");
    serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display())),
    )
    .expect("the captured bases are JSON")
}

/// One base export the engine actually produced, to compile against.
fn a_valid_document() -> Value {
    corpus()["bases"][0]["document"].clone()
}

/// The vendored bytes hash to what was recorded from upstream.
///
/// Trace: FR-099-AC-2
/// Provenance: quoin#474
#[test]
fn tc_474_011_the_vendored_schema_hashes_to_the_recorded_upstream_digest() {
    let observed = hex(&Sha256::digest(schema::SOURCE.as_bytes()));
    assert_eq!(
        observed,
        schema::VENDORED_SHA256,
        "the vendored assurance-v1 document does not hash to the digest recorded \
         when it was copied out of quire-rs. Re-derive it from \
         `schemas/output/assurance-v1.schema.json` at \
         schema::VENDORED_SOURCE_REVISION rather than re-recording what is on \
         disk here."
    );
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// `format` is an annotation here, exactly as it is under the retained ajv.
///
/// The negative half is the point: registering a `uuid` check that refuses
/// everything makes the *same* document fail. If this crate had reached for
/// `compile_with_formats`, the first assertion would fail — so the constructor
/// choice cannot quietly invert without a red test.
///
/// The registered check is `|_| false` rather than a real UUID grammar on
/// purpose: this measures whether format assertion is wired on at all, and a
/// second UUID grammar written to answer that question would be a worse defect
/// than the one it detects.
///
/// Trace: FR-099-AC-2
/// Provenance: quoin#474
#[test]
fn tc_474_012_format_is_annotated_here_because_the_retained_ajv_annotated_it() {
    let mut document = a_valid_document();
    document["artifacts"][0]["uuid"] = Value::String("not-a-uuid".into());

    schema::validate_assurance(document.clone())
        .expect("the retained ajv registers no format check, so `uuid` is an annotation");

    let schema_document: Value =
        serde_json::from_str(schema::SOURCE).expect("the vendored document is JSON");
    let asserting = SchemaValidator::compile_with_formats(
        Path::new(schema::VENDORED_PATH),
        &schema_document,
        &[],
        &[FormatCheck {
            name: "uuid",
            check: |_| false,
        }],
    )
    .expect("the vendored schema compiles with a format registered too");
    assert!(
        !asserting.is_valid(&document),
        "registering a `uuid` check must change the verdict; if it does not, the \
         `should_validate_formats` wiring in quoin-jsonschema has come loose and \
         `compile_vendored` vs `compile_with_formats` no longer means anything"
    );
}

/// A document the engine produced satisfies the published schema.
///
/// The producing half and the reading half are the same contract, and nothing
/// in this repository asserted that before: `assurance::build` wrote exports
/// that only the engine's own reader ever checked.
///
/// Trace: FR-099-AC-2
/// Provenance: quoin#474
#[test]
fn tc_474_013_every_captured_base_satisfies_the_published_schema() {
    let corpus = corpus();
    let bases = corpus["bases"].as_array().expect("bases is an array");
    assert!(
        bases.len() >= 2,
        "anti-vacuity floor: the capture must carry both fixture shapes, saw {}",
        bases.len()
    );
    for base in bases {
        let name = base["name"].as_str().expect("a name");
        schema::validate_assurance(base["document"].clone())
            .unwrap_or_else(|e| panic!("{name}: the engine's own export must validate: {e}"));
    }
}

/// The capture names the engine and the revision it ran at.
///
/// Trace: FR-099-AC-2
/// Provenance: quoin#474
#[test]
fn tc_474_014_the_base_capture_records_its_provenance() {
    let corpus = corpus();
    for key in ["producer", "engine", "quoin_revision"] {
        let value = corpus["provenance"][key].as_str().unwrap_or("");
        assert!(
            !value.is_empty(),
            "the base capture must name {key}; a fixture without the producer and \
             revision that made it cannot be re-derived or audited"
        );
    }
}
