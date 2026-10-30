// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A written receipt records `governing_plan_id` and `diff_paths_supplied` as
//! explicit facts (PLAT-1015, FR-111).
//!
//! `tc_964_apparatus.rs::tc_1015_005_…` already exercises both members through
//! `verify_change_assurance` end to end; this file is the receipt SHAPE's own
//! coverage - what the JSON carries - against the crate's public API only, so it lives beside the shape it
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
use quoin_change_assurance::ids::{NonEmptyText, RecordId, Revision};
use quoin_change_assurance::model::outcome::{Check, Outcome};
use quoin_change_assurance::model::receipt::{ReceiptChecks, VerificationReceipt};
use quoin_store::JsonValue;

/// A minimal, otherwise-valid receipt, written as `write_receipt` always seals
/// one from here on — `governing_plan_id` and `diff_paths_supplied` present.
fn minimal() -> VerificationReceipt {
    VerificationReceipt {
        record_id: RecordId::parse("change-1", Subject::Receipt, "record_id").unwrap(),
        record_revision: Revision::new(1, Subject::Receipt, "revision").unwrap(),
        candidate_revision: NonEmptyText::parse(
            "candidate-1",
            Subject::Receipt,
            "candidate revision",
        )
        .unwrap(),
        decision_event: None,
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

/// Trace: FR-111-AC-5, TC-1915
///
/// A receipt written with no plan linked and no diff supplied records both
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

/// Trace: FR-111-AC-5, TC-1915
///
/// A receipt whose `governing_plan_id` and `diff_paths` were both supplied
/// round-trips both through sealing and `verify_receipt`.
#[test]
fn tc_1015_002_a_linked_receipt_records_the_plan_id_and_that_a_diff_was_supplied() {
    let mut body = minimal();
    body.governing_plan_id =
        Some(NonEmptyText::parse("MP-1015-A", Subject::Receipt, "governing plan id").unwrap());
    body.diff_paths_supplied = true;
    let verified =
        VerificationReceipt::from_json(&body.to_json().unwrap()).expect("the receipt reads back");
    assert_eq!(
        verified.governing_plan_id.map(|id| id.as_str().to_owned()),
        Some("MP-1015-A".to_owned())
    );
    assert!(verified.diff_paths_supplied);
}
