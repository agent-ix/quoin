// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The three normative schema assets this crate now owns (quoin#503).
//!
//! Two things are asserted, and the second is the point. First, that the
//! vocabulary in [`quoin_change_assurance::schemas`] is closed and each asset
//! declares its own identity. Second - and this is what a schema carried
//! beside its reader never had - that a document **this crate actually writes**
//! satisfies the schema that describes it. A shape change in
//! `quoin_change_assurance::model` that nobody mirrored into the asset fails
//! here rather than shipping a schema describing the previous release.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use std::path::Path;

use quoin_change_assurance::attestations::read_attestation_json;
use quoin_change_assurance::records::prepare_change_record;
use quoin_change_assurance::schemas::{
    ASSET_NAMES, CHANGE_ASSURANCE_RECORD_V1, PROOF_ATTESTATION_V1, VERIFICATION_RECEIPT_V1, asset,
};
use quoin_change_assurance::verify::verify_change_assurance;
use quoin_jsonschema::SchemaValidator;
use quoin_store::{JsonValue, canonical_bytes};

use crate::common::base_input;

/// One asset, compiled.
///
/// Every `$ref` in all three assets is a local `#/$defs` pointer, so no
/// resource registry is needed and nothing is fetched.
fn validator(name: &str) -> SchemaValidator {
    let text = asset(name).unwrap_or_else(|| panic!("{name} is not an asset"));
    let schema: serde_json::Value = serde_json::from_str(text).expect("the asset is JSON");
    SchemaValidator::compile_vendored(Path::new(name), &schema, &[])
        .unwrap_or_else(|error| panic!("{name} does not compile: {error}"))
}

/// A document this crate produced, in the shape the validator reads.
///
/// Through canonical bytes rather than a field-by-field conversion: those are
/// the bytes the document is retained as, so what is validated is what is
/// stored.
fn as_instance(value: &JsonValue) -> serde_json::Value {
    let bytes = canonical_bytes(value).expect("the document canonicalizes");
    serde_json::from_slice(&bytes).expect("canonical bytes are JSON")
}

/// Assert `instance` satisfies `name`, reporting every refusal if it does not.
fn assert_valid(
    validator: &SchemaValidator,
    name: &str,
    label: &str,
    instance: &serde_json::Value,
) {
    if !validator.is_valid(instance) {
        let refusals: Vec<String> = validator
            .errors(instance)
            .iter()
            .map(|error| {
                let (path, keyword) = error.identity();
                format!("{path} {keyword:?}")
            })
            .collect();
        panic!("{label} does not satisfy {name}: {}", refusals.join(", "));
    }
}

/// Trace: FR-068-AC-8
/// Provenance: quoin#503
///
/// The asset vocabulary is closed: three names, each resolving, and nothing
/// else resolving to anything.
#[test]
fn tc_503_010_the_asset_vocabulary_is_closed() {
    assert_eq!(ASSET_NAMES.len(), 3);
    for name in ASSET_NAMES {
        assert!(asset(name).is_some(), "{name} does not resolve");
    }
    for name in [
        "not-a-schema.json",
        "change-assurance-record-v1",
        "change-assurance-record-v2.schema.json",
        "",
    ] {
        assert!(asset(name).is_none(), "{name} resolved to an asset");
    }
}

/// Trace: FR-068-AC-8
/// Provenance: quoin#503
///
/// Each asset is a draft 2020-12 document whose `$id` names the file it is
/// served as — the pairing a consumer relies on when it asks for one by name.
#[test]
fn tc_503_011_every_asset_declares_its_own_identity() {
    for name in ASSET_NAMES {
        let text = asset(name).unwrap();
        let document: serde_json::Value = serde_json::from_str(text).expect("the asset is JSON");
        assert_eq!(
            document.get("$schema").and_then(serde_json::Value::as_str),
            Some("https://json-schema.org/draft/2020-12/schema"),
            "{name} does not declare draft 2020-12",
        );
        let id = document
            .get("$id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("{name} has no $id"));
        assert!(id.ends_with(name), "{name} has $id {id}");
    }
}

/// Trace: FR-063-AC-1
/// Provenance: quoin#503
///
/// A record and an attestation this crate writes, and the receipt it emits,
/// satisfy the schemas that describe them.
#[test]
fn tc_503_012_written_documents_satisfy_their_schemas() {
    let input = base_input();
    let record = prepare_change_record(&input.record).unwrap();
    assert_valid(
        &validator(ASSET_NAMES[0]),
        ASSET_NAMES[0],
        "record",
        &as_instance(&record.to_json().unwrap()),
    );
    let attestation = read_attestation_json(&input.attestations[0].attestation).unwrap();
    assert_valid(
        &validator(ASSET_NAMES[1]),
        ASSET_NAMES[1],
        "attestation",
        &as_instance(&attestation.to_json().unwrap()),
    );
    let receipt = verify_change_assurance(&input).unwrap();
    assert_valid(
        &validator(ASSET_NAMES[2]),
        ASSET_NAMES[2],
        "receipt",
        &as_instance(&receipt.to_json().unwrap()),
    );
    let mut refusal = input;
    refusal.record = JsonValue::Null;
    let receipt = verify_change_assurance(&refusal).unwrap();
    assert_valid(
        &validator(ASSET_NAMES[2]),
        ASSET_NAMES[2],
        "refusal receipt",
        &as_instance(&receipt.to_json().unwrap()),
    );
}

/// Trace: FR-068-AC-8
/// Provenance: quoin#503
///
/// The named constants and the `asset` lookup are the same bytes. They are two
/// surfaces onto one file and a consumer may read either.
#[test]
fn tc_503_015_the_named_constants_and_the_lookup_agree() {
    assert_eq!(asset(ASSET_NAMES[0]), Some(CHANGE_ASSURANCE_RECORD_V1));
    assert_eq!(asset(ASSET_NAMES[1]), Some(PROOF_ATTESTATION_V1));
    assert_eq!(asset(ASSET_NAMES[2]), Some(VERIFICATION_RECEIPT_V1));
}
