// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A sealed receipt records `governing_plan_id` and `diff_paths_supplied` as
//! explicit facts, and reading one sealed before either member existed still
//! re-verifies (PLAT-1015, FR-111).
//!
//! `tc_964_apparatus.rs::tc_1015_005_…` already exercises both members through
//! `verify_change_assurance` end to end; this file is the receipt SHAPE's own
//! coverage — what the JSON carries, and that a pre-PLAT-1015 document (no
//! `governing_plan_id`, no `diff_paths_supplied` at all) is still readable —
//! against the crate's public API only, so it lives beside the shape it
//! covers rather than inside `model/receipt.rs`, which is at the module-size
//! ceiling (quoin#464).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_change_assurance::error::Subject;
use quoin_change_assurance::ids::NonEmptyText;
use quoin_change_assurance::model::outcome::{Check, Outcome};
use quoin_change_assurance::model::receipt::{ReceiptChecks, UnsealedReceipt};
use quoin_change_assurance::verify::verify_receipt;
use quoin_store::{CanonicalDigest, JsonValue, digest_canonical_value};

/// A minimal, otherwise-valid receipt, sealed as `write_receipt` always seals
/// one from here on — `governing_plan_id` and `diff_paths_supplied` present.
fn minimal() -> UnsealedReceipt {
    UnsealedReceipt {
        record_digest: CanonicalDigest::parse_stored(&"a".repeat(64)).unwrap(),
        candidate_revision: NonEmptyText::parse(
            "candidate-1",
            Subject::Receipt,
            "candidate revision",
        )
        .unwrap(),
        decision_event: None,
        parent_digests: Vec::new(),
        checks: ReceiptChecks {
            record: Check::valid(),
            lineage: Check::valid(),
            review: Check::valid(),
            impact: Check::valid(),
        },
        proofs: Vec::new(),
        unknowns: Vec::new(),
        outcome: Outcome::Valid,
        reasons: Vec::new(),
        governing_plan_id: None,
        diff_paths_supplied: false,
    }
}

/// Trace: FR-111-AC-5
///
/// A receipt sealed with no plan linked and no diff supplied records both
/// facts explicitly rather than omitting the members: `governing_plan_id:
/// null`, `diff_paths_supplied: false`, not silence.
#[test]
fn tc_1015_001_an_unlinked_receipt_records_the_omission_explicitly() {
    let value = minimal().to_json().unwrap();
    let object = value.as_object().unwrap();
    assert!(
        object.contains("governing_plan_id"),
        "the member itself must be present, not merely absent-and-implied"
    );
    assert!(object.contains("diff_paths_supplied"));
    assert_eq!(object.get("governing_plan_id"), Some(&JsonValue::Null));
    assert_eq!(
        object.get("diff_paths_supplied"),
        Some(&JsonValue::Bool(false))
    );
}

/// Trace: FR-111-AC-5
///
/// A receipt whose `governing_plan_id` and `diff_paths` were both supplied
/// round-trips both through sealing and `verify_receipt`.
#[test]
fn tc_1015_002_a_linked_receipt_records_the_plan_id_and_that_a_diff_was_supplied() {
    let mut body = minimal();
    body.governing_plan_id =
        Some(NonEmptyText::parse("MP-1015-A", Subject::Receipt, "governing plan id").unwrap());
    body.diff_paths_supplied = true;
    let digest = digest_canonical_value(&body.to_json().unwrap()).unwrap();
    let sealed = body.seal_with(digest);
    let verified = verify_receipt(&sealed.to_json().unwrap()).expect("the receipt re-verifies");
    assert_eq!(
        verified
            .body
            .governing_plan_id
            .map(|id| id.as_str().to_owned()),
        Some("MP-1015-A".to_owned())
    );
    assert!(verified.body.diff_paths_supplied);
}

/// Trace: FR-111-AC-5
///
/// A receipt sealed before PLAT-1015 carries neither `governing_plan_id` nor
/// `diff_paths_supplied` at all. `verify_receipt` still reads and verifies
/// it — the two members are read as optional, so evidence retained before
/// this ticket does not become unreadable the day it ships. Absence reads
/// back as the pre-PLAT-1015 default: no plan link, no diff, because that
/// receipt genuinely never tracked either.
#[test]
fn tc_1015_003_a_receipt_sealed_before_plat_1015_still_re_verifies() {
    let full = minimal().to_json().unwrap();
    let mut stripped = full.as_object().unwrap().clone();
    stripped.remove("governing_plan_id");
    stripped.remove("diff_paths_supplied");
    let stripped_value = JsonValue::Object(stripped);
    let digest = digest_canonical_value(&stripped_value).unwrap();
    let mut with_digest = stripped_value.as_object().unwrap().clone();
    with_digest.set("digest", JsonValue::string(digest.as_hex()));
    let sealed = JsonValue::Object(with_digest);

    let verified =
        verify_receipt(&sealed).expect("a receipt sealed before PLAT-1015 still re-verifies");
    assert_eq!(verified.body.governing_plan_id, None);
    assert!(!verified.body.diff_paths_supplied);
}
