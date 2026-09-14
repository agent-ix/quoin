// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The three normative schema assets this crate now owns (quoin#503).
//!
//! Two things are asserted, and the second is the point. First, that the
//! vocabulary in [`quoin_change_assurance::schemas`] is closed and each asset
//! declares its own identity. Second — and this is what a schema carried
//! beside its reader never had — that a document **this crate actually seals**
//! satisfies the schema that describes it. A shape change in
//! `quoin_change_assurance::model` that nobody mirrored into the asset fails
//! here rather than shipping a schema describing the previous release.
//!
//! The sealed documents come from `tests/fixtures/oracle.json`, so the inputs
//! are the retained TypeScript's and not this crate's invention; what is
//! measured is the agreement between two things this crate owns, which is
//! exactly the drift the assets were previously free to have.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use std::path::Path;

use quoin_change_assurance::attestations::seal_attestation;
use quoin_change_assurance::records::seal_change_record;
use quoin_change_assurance::schemas::{
    ASSET_NAMES, CHANGE_ASSURANCE_RECORD_V1, PROOF_ATTESTATION_V1, VERIFICATION_RECEIPT_V1, asset,
};
use quoin_change_assurance::verify::verify_change_assurance;
use quoin_jsonschema::SchemaValidator;
use quoin_store::{JsonValue, canonical_bytes};

use crate::common::{member, oracle, section, text, verification_input};

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
/// Every record this crate seals satisfies `change-assurance-record-v1`.
#[test]
fn tc_503_012_sealed_records_satisfy_the_record_schema() {
    let captured = section(&oracle(), "records");
    assert!(
        captured.len() >= 4,
        "anti-vacuity floor: at least 4 captured records, found {}",
        captured.len(),
    );
    let validator = validator(ASSET_NAMES[0]);
    for entry in &captured {
        let name = text(entry, "name");
        let sealed = seal_change_record(member(entry, "input"))
            .unwrap_or_else(|error| panic!("sealing {name}: {error}"));
        let instance = as_instance(&sealed.to_json().unwrap());
        assert_valid(&validator, ASSET_NAMES[0], name, &instance);
    }
}

/// Trace: FR-064-AC-1
/// Provenance: quoin#503
///
/// Every attestation this crate seals satisfies `proof-attestation-v1`.
#[test]
fn tc_503_013_sealed_attestations_satisfy_the_attestation_schema() {
    let captured = section(&oracle(), "attestations");
    assert!(
        captured.len() >= 4,
        "anti-vacuity floor: at least 4 captured attestations, found {}",
        captured.len(),
    );
    let validator = validator(ASSET_NAMES[1]);
    for entry in &captured {
        let name = text(entry, "name");
        // The capture kept only the sealed attestation, so that is the input:
        // `seal_attestation` removes any `digest` before it computes one, so
        // re-sealing a sealed attestation runs the whole sealing path.
        let sealed = seal_attestation(member(entry, "sealed"))
            .unwrap_or_else(|error| panic!("sealing {name}: {error}"));
        let instance = as_instance(&sealed.to_json().unwrap());
        assert_valid(&validator, ASSET_NAMES[1], name, &instance);
    }
}

/// Trace: FR-065-AC-1
/// Provenance: quoin#503
///
/// Every receipt this crate emits satisfies `verification-receipt-v1` —
/// including the refusal receipts, which are the shapes a consumer is least
/// likely to have a sample of and most likely to meet in anger.
#[test]
fn tc_503_014_emitted_receipts_satisfy_the_receipt_schema() {
    let captured = section(&oracle(), "verifications");
    assert!(
        captured.len() >= 30,
        "anti-vacuity floor: at least 30 captured scenarios, found {}",
        captured.len(),
    );
    let validator = validator(ASSET_NAMES[2]);
    let mut validated = 0_usize;
    for scenario in &captured {
        let name = text(scenario, "name");
        let Ok(receipt) = verify_change_assurance(&verification_input(member(scenario, "input")))
        else {
            continue;
        };
        let instance = as_instance(&receipt.to_json().unwrap());
        assert_valid(&validator, ASSET_NAMES[2], name, &instance);
        validated += 1;
    }
    assert!(
        validated >= 30,
        "anti-vacuity floor: at least 30 receipts validated, validated {validated}",
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
