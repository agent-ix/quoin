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

use ix_trace_rs::trace;
use std::path::Path;

use quoin_change_assurance::attestations::read_attestation_json;
use quoin_change_assurance::records::prepare_change_record;
use quoin_change_assurance::schemas::{
    ASSET_NAMES, CHANGE_ASSURANCE_RECORD_V2, PROOF_ATTESTATION_V1, VERIFICATION_RECEIPT_V2, asset,
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

/// Provenance: quoin#503
///
/// The asset vocabulary is closed: three names, each resolving, and nothing
/// else resolving to anything.
#[trace("FR-068-AC-8")]
#[test]
fn tc_503_010_the_asset_vocabulary_is_closed() {
    assert_eq!(ASSET_NAMES.len(), 3);
    for name in ASSET_NAMES {
        assert!(asset(name).is_some(), "{name} does not resolve");
    }
    for name in [
        "not-a-schema.json",
        "change-assurance-record-v1",
        "change-assurance-record-v1.schema.json",
        "verification-receipt-v1.schema.json",
        "",
    ] {
        assert!(asset(name).is_none(), "{name} resolved to an asset");
    }
}

/// Provenance: quoin#503
///
/// Each asset is a draft 2020-12 document whose `$id` names the file it is
/// served as — the pairing a consumer relies on when it asks for one by name.
#[trace("FR-068-AC-8")]
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

/// Provenance: quoin#503
///
/// A record and an attestation this crate writes, and the receipt it emits,
/// satisfy the schemas that describe them.
#[trace("FR-063-AC-1")]
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

/// Provenance: quoin#503
///
/// The named constants and the `asset` lookup are the same bytes. They are two
/// surfaces onto one file and a consumer may read either.
#[trace("FR-068-AC-8")]
#[test]
fn tc_503_015_the_named_constants_and_the_lookup_agree() {
    assert_eq!(asset(ASSET_NAMES[0]), Some(CHANGE_ASSURANCE_RECORD_V2));
    assert_eq!(asset(ASSET_NAMES[1]), Some(PROOF_ATTESTATION_V1));
    assert_eq!(asset(ASSET_NAMES[2]), Some(VERIFICATION_RECEIPT_V2));
}

/// The published record schema refuses the same malformed source paths the
/// reader refuses, and accepts the paths the reader accepts.
#[trace("FR-063-AC-12")]
#[test]
fn tc_503_the_record_schema_agrees_with_the_reader_on_source_paths() {
    let validator = validator(ASSET_NAMES[0]);
    let record = base_input().record;
    let instance = |path: &str| {
        let mut value = as_instance(&record);
        value["source_connections"][0]["path"] = serde_json::Value::String(path.to_owned());
        value
    };
    for bad in [
        ".", "/abs", "../x", "a/../b", "a/./b", "./a", "a//b", "a\\b", "a/b/", "a\0b", "",
    ] {
        assert!(
            !validator.is_valid(&instance(bad)),
            "the schema must refuse the path {bad:?}"
        );
        assert!(
            prepare_change_record(&{
                let mut value = record.clone();
                set_first_path(&mut value, bad);
                value
            })
            .is_err(),
            "the reader must refuse the path {bad:?}"
        );
    }
    for good in ["a.md", "spec/a.md", "dir/.hidden", "a/..b/c"] {
        assert!(
            validator.is_valid(&instance(good)),
            "the schema must accept the path {good:?}"
        );
    }
}

fn set_first_path(record: &mut JsonValue, path: &str) {
    let mut root = record.as_object().unwrap().clone();
    let JsonValue::Array(mut connections) = root.get("source_connections").unwrap().clone() else {
        panic!("source_connections is not an array");
    };
    let mut first = connections[0].as_object().unwrap().clone();
    first.set("path", JsonValue::string(path));
    connections[0] = JsonValue::Object(first);
    root.set("source_connections", JsonValue::Array(connections));
    *record = JsonValue::Object(root);
}

fn record_with_policy(policy: &str) -> JsonValue {
    let text = common::record_text().replace(
        r#"{"required": false, "signature_kinds": [], "approvers": []}"#,
        policy,
    );
    common::json(&text)
}

/// The published record schema refuses the same `signed_approval` policies the
/// reader refuses, accepts the same ones, and names no trust input anywhere.
#[trace("FR-063-AC-16", "FR-063-AC-1")]
#[test]
fn tc_1511_the_record_schema_agrees_with_the_reader_on_the_policy() {
    let validator = validator(ASSET_NAMES[0]);
    for (policy, accepted) in [
        (
            r#"{"required": false, "signature_kinds": [], "approvers": []}"#,
            true,
        ),
        (
            r#"{"required": true, "signature_kinds": ["ssh"], "approvers": []}"#,
            true,
        ),
        (
            r#"{"required": true, "signature_kinds": ["gitsign", "ssh"], "approvers": ["a", "b"]}"#,
            true,
        ),
        (
            r#"{"required": true, "signature_kinds": [], "approvers": []}"#,
            false,
        ),
        (
            r#"{"required": false, "signature_kinds": ["ssh"], "approvers": []}"#,
            false,
        ),
        (
            r#"{"required": false, "signature_kinds": [], "approvers": ["a"]}"#,
            false,
        ),
        (
            r#"{"required": true, "signature_kinds": ["ssh", "ssh"], "approvers": []}"#,
            false,
        ),
        (
            r#"{"required": true, "signature_kinds": ["pgp"], "approvers": []}"#,
            false,
        ),
        (
            r#"{"required": true, "signature_kinds": ["ssh"], "approvers": [""]}"#,
            false,
        ),
        (
            r#"{"required": true, "signature_kinds": ["ssh"], "approvers": ["a", "a"]}"#,
            false,
        ),
        (
            r#"{"required": true, "signature_kinds": ["ssh"], "approvers": [], "allowed_signers": "f"}"#,
            false,
        ),
    ] {
        let record = record_with_policy(policy);
        assert_eq!(
            validator.is_valid(&as_instance(&record)),
            accepted,
            "the schema on {policy}"
        );
        assert_eq!(
            prepare_change_record(&record).is_ok(),
            accepted,
            "the reader on {policy}"
        );
    }
    let mut without = as_instance(&common::json(&common::record_text()));
    without["review_workflow"]
        .as_object_mut()
        .unwrap()
        .remove("signed_approval");
    assert!(!validator.is_valid(&without));
    let mut digestful = as_instance(&common::json(&common::record_text()));
    digestful["digest"] = serde_json::Value::String("sha256:00".to_owned());
    assert!(!validator.is_valid(&digestful));

    let schema: serde_json::Value = serde_json::from_str(asset(ASSET_NAMES[0]).unwrap()).unwrap();
    let mut names = Vec::new();
    collect_property_names(&schema, &mut names);
    for trust_input in [
        "allowed_signers",
        "certificate_identity",
        "certificate_oidc_issuer",
        "principal",
        "key",
        "issuer",
        "ref",
        "digest",
    ] {
        assert!(
            !names.iter().any(|name| name == trust_input),
            "the record schema names {trust_input}"
        );
    }
}

fn collect_property_names(value: &serde_json::Value, into: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(members) => {
            if let Some(serde_json::Value::Object(properties)) = members.get("properties") {
                into.extend(properties.keys().cloned());
            }
            for member in members.values() {
                collect_property_names(member, into);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_property_names(item, into);
            }
        }
        _ => {}
    }
}

/// The receipt schema, a receipt carrying no claim, and a claim of the
/// approved shape, for the tests below.
struct ClaimFixture {
    validator: SchemaValidator,
    base: serde_json::Value,
    claim: serde_json::Value,
}

impl ClaimFixture {
    fn new() -> Self {
        let receipt = verify_change_assurance(&base_input()).unwrap();
        Self {
            validator: validator(ASSET_NAMES[2]),
            base: as_instance(&receipt.to_json().unwrap()),
            claim: serde_json::json!({
                "protocol": "engineering-assurance.approval-verify-result/v1",
                "tag": "approval/change-1/1",
                "record_digest": "sha256:00",
                "trust": {"allowed_signers": "/keys/allowed"},
                "approver": "a@example.test",
                "meaning": ["reviewed"],
                "signature_kind": "ssh",
                "signed_at": "2026-08-31T10:20:30Z",
                "signed_at_source": "tagger_date",
            }),
        }
    }

    /// The receipt with `claim` as its `claimed_approval`.
    fn with(&self, claim: serde_json::Value) -> serde_json::Value {
        let mut receipt = self.base.clone();
        receipt["decision_event"]["claimed_approval"] = claim;
        receipt
    }

    /// The claim with `member` set to `value`.
    fn changed(&self, member: &str, value: serde_json::Value) -> serde_json::Value {
        let mut changed = self.claim.clone();
        changed[member] = value;
        self.with(changed)
    }

    /// The claim with `member` removed.
    fn without(&self, member: &str) -> serde_json::Value {
        let mut changed = self.claim.clone();
        changed.as_object_mut().unwrap().remove(member);
        self.with(changed)
    }

    /// Whether the receipt reader accepts `instance`.
    fn reads(instance: &serde_json::Value) -> bool {
        let bytes = serde_json::to_vec(instance).unwrap();
        quoin_change_assurance::VerificationReceipt::from_json(
            &quoin_store::parse_strict_json(&bytes).unwrap(),
        )
        .is_ok()
    }

    /// Assert the schema and the reader both refuse every instance.
    fn assert_refused(&self, forged: &[serde_json::Value]) {
        for instance in forged {
            assert!(
                !self.validator.is_valid(instance),
                "schema accepted {instance}"
            );
            assert!(!Self::reads(instance), "reader accepted {instance}");
        }
    }
}

/// The receipt carries `claimed_approval` as null or as exactly the declared
/// shape: the schema and the reader accept a real claim and refuse a forged
/// shape, an absent member and a version 1 receipt.
#[trace("FR-065-AC-1", "FR-065-AC-13", "FR-065-AC-18")]
#[test]
fn tc_1511_the_receipt_schema_and_reader_agree_on_the_claimed_approval() {
    let fixture = ClaimFixture::new();
    for accepted in [
        fixture.with(serde_json::Value::Null),
        fixture.with(fixture.claim.clone()),
    ] {
        assert!(fixture.validator.is_valid(&accepted));
        assert!(ClaimFixture::reads(&accepted));
    }
    let mut forged = vec![
        fixture.with(serde_json::json!({"approver": "a"})),
        fixture.with(serde_json::json!("approved")),
        fixture.without("approver"),
    ];
    for (member, value) in [
        ("extra", serde_json::json!(1)),
        ("protocol", serde_json::json!("other/v1")),
        ("signature_kind", serde_json::json!("pgp")),
        ("meaning", serde_json::json!([])),
        ("trust", serde_json::json!({})),
        ("tag", serde_json::json!("")),
    ] {
        forged.push(fixture.changed(member, value));
    }
    let mut absent = fixture.base.clone();
    absent["decision_event"]
        .as_object_mut()
        .unwrap()
        .remove("claimed_approval");
    forged.push(absent);
    let mut version_one = fixture.base.clone();
    version_one["schema_version"] = serde_json::json!(1);
    forged.push(version_one);
    fixture.assert_refused(&forged);
}

/// A claim carries exactly one time form: the instant with its source, or the
/// missing reason. Both, neither, half of the first form, the missing form
/// beside half of it, a malformed instant, an unknown source (the reserved
/// `transparency_log` included), an unknown reason and an undeclared time
/// member are refused by the schema and the reader.
#[trace("FR-065-AC-21")]
#[test]
fn tc_1592_the_receipt_schema_and_reader_agree_on_the_time_forms() {
    let fixture = ClaimFixture::new();
    let missing = |extra: Option<(&str, serde_json::Value)>| {
        let mut claim = fixture.claim.clone();
        let members = claim.as_object_mut().unwrap();
        members.remove("signed_at");
        members.remove("signed_at_source");
        members.insert(
            "signed_at_missing".to_owned(),
            serde_json::json!("tagger_date_unreadable"),
        );
        if let Some((name, value)) = extra {
            members.insert(name.to_owned(), value);
        }
        claim
    };
    let missing_form = fixture.with(missing(None));
    assert!(fixture.validator.is_valid(&missing_form));
    assert!(ClaimFixture::reads(&missing_form));

    let mut neither = fixture.claim.clone();
    neither.as_object_mut().unwrap().remove("signed_at");
    neither.as_object_mut().unwrap().remove("signed_at_source");
    let mut unknown_reason = missing(None);
    unknown_reason["signed_at_missing"] = serde_json::json!("unknown");
    let mut forged = vec![
        fixture.changed(
            "signed_at_missing",
            serde_json::json!("tagger_date_unreadable"),
        ),
        fixture.without("signed_at"),
        fixture.without("signed_at_source"),
        fixture.with(neither),
        fixture.with(missing(Some((
            "signed_at",
            serde_json::json!("2026-08-31T10:20:30Z"),
        )))),
        fixture.with(missing(Some((
            "signed_at_source",
            serde_json::json!("tagger_date"),
        )))),
        fixture.with(missing(Some(("signed_at_zone", serde_json::json!("UTC"))))),
        fixture.with(unknown_reason),
    ];
    for (member, value) in [
        ("signed_at", serde_json::json!("2026-08-31T10:20:30+00:00")),
        ("signed_at", serde_json::json!("2026-13-31T10:20:30Z")),
        ("signed_at", serde_json::json!("2026-08-31T23:59:60Z")),
        ("signed_at_source", serde_json::json!("clock")),
        ("signed_at_source", serde_json::json!("transparency_log")),
        ("signed_at_zone", serde_json::json!("UTC")),
    ] {
        forged.push(fixture.changed(member, value));
    }
    fixture.assert_refused(&forged);

    // A day the month lacks matches the schema pattern; the reader alone
    // checks the calendar.
    let impossible_day = fixture.changed("signed_at", serde_json::json!("2026-02-31T10:20:30Z"));
    assert!(fixture.validator.is_valid(&impossible_day));
    assert!(
        !ClaimFixture::reads(&impossible_day),
        "reader accepted {impossible_day}"
    );
}
