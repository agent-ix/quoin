// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A decision's claimed approval, judged against the record and its narrowing
//! policy (FR-063, FR-065).
//!
//! Quoin never verifies a signature here: a claim is the event writer's data,
//! compared with the reviewed record and with the digest of the stored bytes.
//! Every fixture is hand-built and fictional.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use ix_trace_rs::trace;

use quoin_change_assurance::model::outcome::Outcome;
use quoin_change_assurance::model::receipt::VerificationReceipt;
use quoin_change_assurance::verify::input::VerificationInput;
use quoin_change_assurance::verify::verify_change_assurance;
use quoin_store::JsonValue;

use crate::common::{base_input, digest_of, history, json, member, record_text};

const KIND: &str = "change_assurance.review_decided";
const POLICY_OFF: &str = r#"{"required": false, "signature_kinds": [], "approvers": []}"#;
const POLICY_SSH: &str = r#"{"required": true, "signature_kinds": ["ssh"], "approvers": []}"#;
const POLICY_NAMED: &str = r#"{"required": true, "signature_kinds": ["gitsign", "ssh"], "approvers": ["reviewer@example.test"]}"#;

/// The scenario record with `policy` as its `signed_approval`.
fn record_with(policy: &str) -> JsonValue {
    json(&record_text().replace(POLICY_OFF, policy))
}

/// The digest of the scenario record under `policy`, as a store would hold it.
fn digest_under(policy: &str) -> String {
    digest_of(&record_with(policy))
}

/// A claim of `status`, bound to `digest`; `approved` carries its extra facts.
fn claim(status: &str, digest: &str) -> String {
    let facts = if status == "approved" {
        r#", "approver": "reviewer@example.test", "meaning": ["reviewed", "tested"], "signature_kind": "ssh""#
    } else {
        ""
    };
    format!(
        r#"{{"protocol": "engineering-assurance.approval-verify-result/v1", "status": "{status}", "tag": "approval/change-1/1", "record_id": "change-1", "record_digest": "{digest}", "trust": {{"allowed_signers": "/keys/allowed_signers"}}{facts}}}"#
    )
}

/// One `approved` decision event, carrying `claim` when given.
fn event(id: &str, sequence: u64, claim: Option<&str>) -> String {
    let claim = claim.map_or(String::new(), |claim| {
        format!(r#", "verified_approval": {claim}"#)
    });
    format!(
        r#"{{"id": "{id}", "ts": "2026-08-31T00:00:00.000Z", "sequence": {sequence},
          "actor": {{"id": "reviewer-1", "kind": "human"}}, "kind": "{KIND}",
          "payload": {{"schema_version": 2, "record_id": "change-1", "revision": 1, "decision": "approved"{claim}}}}}"#
    )
}

/// The scenario under `policy` with one approving event carrying `claim`.
fn input(policy: &str, claim: Option<&str>) -> VerificationInput {
    let mut input = base_input();
    input.record = record_with(policy);
    input.record_digest = digest_under(policy);
    input.decision_history = Some(history(&[event("event-1", 1, claim)]));
    input
}

fn verify(input: &VerificationInput) -> VerificationReceipt {
    verify_change_assurance(input).expect("a verification always produces a receipt")
}

fn reasons(receipt: &VerificationReceipt) -> Vec<&'static str> {
    receipt
        .reasons
        .iter()
        .map(|reason| reason.as_str())
        .collect()
}

/// The `claimed_approval` member of the receipt's JSON form.
fn claimed(receipt: &VerificationReceipt) -> JsonValue {
    let json = receipt.to_json().unwrap();
    let event = member(&json, "decision_event");
    member(event, "claimed_approval").clone()
}

/// Assert the receipt is `outcome` with exactly `expected` reasons and no
/// claim reported.
fn assert_refused(receipt: &VerificationReceipt, outcome: Outcome, expected: &[&str]) {
    assert_eq!(receipt.outcome, outcome, "{:?}", reasons(receipt));
    assert_eq!(reasons(receipt), expected);
    assert_eq!(claimed(receipt), JsonValue::Null);
}

/// An approved claim bound to the stored revision keeps the review check valid
/// and is reported, field for field, as a claim; under a `required` policy the
/// kind and approver are in the narrowing lists.
#[trace("FR-065-AC-13")]
#[test]
fn tc_1511_a_bound_claim_keeps_the_review_valid_and_is_reported() {
    for policy in [POLICY_OFF, POLICY_SSH, POLICY_NAMED] {
        let digest = digest_under(policy);
        let receipt = verify(&input(policy, Some(&claim("approved", &digest))));
        assert_eq!(
            receipt.outcome,
            Outcome::Valid,
            "{policy}: {:?}",
            reasons(&receipt)
        );
        assert_eq!(receipt.checks.review.outcome, Outcome::Valid);
        let reported = claimed(&receipt);
        assert_eq!(
            member(&reported, "protocol"),
            &JsonValue::string("engineering-assurance.approval-verify-result/v1")
        );
        assert_eq!(
            member(&reported, "tag"),
            &JsonValue::string("approval/change-1/1")
        );
        assert_eq!(
            member(&reported, "record_digest"),
            &JsonValue::string(&digest)
        );
        assert_eq!(
            member(&reported, "trust"),
            &json(r#"{"allowed_signers": "/keys/allowed_signers"}"#)
        );
        assert_eq!(
            member(&reported, "approver"),
            &JsonValue::string("reviewer@example.test")
        );
        assert_eq!(
            member(&reported, "meaning"),
            &json(r#"["reviewed", "tested"]"#)
        );
        assert_eq!(
            member(&reported, "signature_kind"),
            &JsonValue::string("ssh")
        );
    }
}

/// A `wrong_record` status, an approved claim for another record, and an
/// approved claim whose digest is not the stored revision's (the record changed
/// after the tag) are each `invalid` with `approval_record_mismatch`.
#[trace("FR-065-AC-14")]
#[test]
fn tc_1511_a_claim_not_bound_to_the_stored_revision_is_a_record_mismatch() {
    let stale = format!("sha256:{}", "0".repeat(64));
    let other_record = claim("approved", &digest_under(POLICY_OFF)).replace(
        "\"change-1\", \"record_digest\"",
        "\"change-9\", \"record_digest\"",
    );
    for claim in [
        claim("wrong_record", &digest_under(POLICY_OFF)),
        claim("approved", &stale),
        other_record,
    ] {
        let receipt = verify(&input(POLICY_OFF, Some(&claim)));
        assert_refused(&receipt, Outcome::Invalid, &["approval_record_mismatch"]);
    }
}

/// `untrusted_signer` and `signed_bad` map to their own reasons; under a
/// `required` policy a kind outside `signature_kinds` or an approver outside a
/// non-empty `approvers` is `approval_signer_untrusted`, while a different
/// `trust` member alone changes nothing.
#[trace("FR-065-AC-15")]
#[test]
fn tc_1511_an_untrusted_or_badly_signed_claim_is_invalid() {
    let digest = digest_under(POLICY_OFF);
    let receipt = verify(&input(
        POLICY_OFF,
        Some(&claim("untrusted_signer", &digest)),
    ));
    assert_refused(&receipt, Outcome::Invalid, &["approval_signer_untrusted"]);
    let receipt = verify(&input(POLICY_OFF, Some(&claim("signed_bad", &digest))));
    assert_refused(&receipt, Outcome::Invalid, &["approval_signature_bad"]);

    let ssh_only = digest_under(POLICY_SSH);
    let gitsign = claim("approved", &ssh_only).replace("\"ssh\"", "\"gitsign\"");
    let receipt = verify(&input(POLICY_SSH, Some(&gitsign)));
    assert_refused(&receipt, Outcome::Invalid, &["approval_signer_untrusted"]);

    let named = digest_under(POLICY_NAMED);
    let stranger =
        claim("approved", &named).replace("reviewer@example.test", "stranger@example.test");
    let receipt = verify(&input(POLICY_NAMED, Some(&stranger)));
    assert_refused(&receipt, Outcome::Invalid, &["approval_signer_untrusted"]);

    let other_trust = claim("approved", &named).replace(
        r#"{"allowed_signers": "/keys/allowed_signers"}"#,
        r#"{"allowed_signers": "/elsewhere/keys", "gitsign": {"certificate_identity": "a@example.test", "certificate_oidc_issuer": "https://issuer.example.test", "principal": "a"}}"#,
    );
    let receipt = verify(&input(POLICY_NAMED, Some(&other_trust)));
    assert_eq!(receipt.outcome, Outcome::Valid, "{:?}", reasons(&receipt));
}

/// `meaning_missing` is `approval_meaning_missing`, `unsigned` and
/// `unverifiable` are `approval_unverified`, all `incomplete`; none is valid,
/// and another invalid check still dominates.
#[trace("FR-065-AC-16")]
#[test]
fn tc_1511_a_claim_that_is_unfinished_is_incomplete_and_never_valid() {
    let digest = digest_under(POLICY_OFF);
    for (status, reason) in [
        ("meaning_missing", "approval_meaning_missing"),
        ("unsigned", "approval_unverified"),
        ("unverifiable", "approval_unverified"),
    ] {
        let receipt = verify(&input(POLICY_OFF, Some(&claim(status, &digest))));
        assert_refused(&receipt, Outcome::Incomplete, &[reason]);

        let mut contradicted = input(POLICY_OFF, Some(&claim(status, &digest)));
        contradicted.candidate_revision = "another-candidate".to_owned();
        let receipt = verify(&contradicted);
        assert_eq!(receipt.outcome, Outcome::Invalid);
        assert!(reasons(&receipt).contains(&reason));
    }
}

/// Two matching events that each carry a claim select none: the exactly-one
/// rule stays `decision_mismatch` and no claim is reported.
#[trace("FR-065-AC-17")]
#[test]
fn tc_1511_two_claiming_events_are_a_decision_mismatch() {
    let digest = digest_under(POLICY_OFF);
    let claim = claim("approved", &digest);
    let mut duplicated = input(POLICY_OFF, Some(&claim));
    duplicated.decision_history = Some(history(&[
        event("event-1", 1, Some(&claim)),
        event("event-2", 2, Some(&claim)),
    ]));
    let receipt = verify(&duplicated);
    assert_eq!(receipt.outcome, Outcome::Invalid);
    assert_eq!(reasons(&receipt), ["decision_mismatch"]);
    assert!(receipt.decision_event.is_none());
}

/// Without a claim and with `required` false the review is as before and
/// `claimed_approval` is null; a version 1 payload cannot approve, a version 1
/// record is `schema_invalid`, and a version 1 receipt is not read.
#[trace("FR-065-AC-18")]
#[test]
fn tc_1511_no_claim_under_an_optional_policy_is_as_before_and_v1_is_refused() {
    let receipt = verify(&input(POLICY_OFF, None));
    assert_eq!(receipt.outcome, Outcome::Valid, "{:?}", reasons(&receipt));
    assert_eq!(claimed(&receipt), JsonValue::Null);

    let mut v1_payload = input(POLICY_OFF, None);
    let v1 = event("event-1", 1, None).replace("\"schema_version\": 2", "\"schema_version\": 1");
    v1_payload.decision_history = Some(history(&[v1]));
    let receipt = verify(&v1_payload);
    assert_eq!(receipt.outcome, Outcome::Invalid);
    assert_eq!(reasons(&receipt), ["decision_mismatch"]);

    let mut v1_record = input(POLICY_OFF, None);
    v1_record.record =
        json(&record_text().replace("\"schema_version\": 2", "\"schema_version\": 1"));
    let receipt = verify(&v1_record);
    assert_eq!(reasons(&receipt), ["schema_invalid"]);

    let valid = verify(&input(POLICY_OFF, None)).to_json().unwrap();
    let v1_receipt = common::replacing(&valid, "schema_version", JsonValue::number(1.0).unwrap());
    assert!(VerificationReceipt::from_json(&v1_receipt).is_err());
}

/// An approved event without a claim, under a `required` policy, is
/// `incomplete` with `approval_missing`; the same record with `required` false
/// is not affected.
#[trace("FR-065-AC-19")]
#[test]
fn tc_1511_a_required_policy_without_a_claim_is_incomplete() {
    for policy in [POLICY_SSH, POLICY_NAMED] {
        let receipt = verify(&input(policy, None));
        assert_refused(&receipt, Outcome::Incomplete, &["approval_missing"]);
    }
    let optional = verify(&input(POLICY_OFF, None));
    assert_eq!(optional.outcome, Outcome::Valid);
}

/// A `signed_bad` claim whose digest also differs from the stored record
/// reports the status first, following Engineering Assurance's order.
#[trace("FR-065-AC-20")]
#[test]
fn tc_1511_a_bad_signature_outranks_a_stale_digest() {
    let stale = format!("sha256:{}", "f".repeat(64));
    let receipt = verify(&input(POLICY_OFF, Some(&claim("signed_bad", &stale))));
    assert_refused(&receipt, Outcome::Invalid, &["approval_signature_bad"]);
}

/// Rule order inside the narrowing: a kind outside `signature_kinds` outranks
/// a stale digest, and a stale digest outranks a missing meaning.
#[trace("FR-065-AC-15", "FR-065-AC-16")]
#[test]
fn tc_1511_policy_narrowing_precedes_binding_and_binding_precedes_meaning() {
    let stale = format!("sha256:{}", "e".repeat(64));
    let gitsign = claim("approved", &stale).replace("\"ssh\"", "\"gitsign\"");
    let receipt = verify(&input(POLICY_SSH, Some(&gitsign)));
    assert_refused(&receipt, Outcome::Invalid, &["approval_signer_untrusted"]);

    let receipt = verify(&input(POLICY_OFF, Some(&claim("meaning_missing", &stale))));
    assert_refused(&receipt, Outcome::Invalid, &["approval_record_mismatch"]);
}
