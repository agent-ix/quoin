// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The identity-addressed contract of FR-063, FR-064 and FR-065: records by
//! `(record_id, revision)`, attestations by `attestation_id`, decision events
//! by `sequence`, receipts re-derived and compared byte for byte.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use quoin_change_assurance::attestations::read_attestation_json;
use quoin_change_assurance::error::{ChangeAssuranceError, Subject};
use quoin_change_assurance::ids::{RecordId, Revision};
use quoin_change_assurance::intake::memory::MemoryEvidenceStore;
use quoin_change_assurance::intake::{Published, read_change_record, write_change_record};
use quoin_change_assurance::model::outcome::{Outcome, Reason};
use quoin_change_assurance::records::prepare_change_record;
use quoin_change_assurance::verify::{verify_change_assurance, verify_receipt};
use quoin_store::JsonValue;

use crate::common::{attestation_text, base_input, event_text, history, json, record_text};

/// Trace: FR-063-AC-1
///
/// An undeclared member - a `digest` among them - is refused, on a record and
/// on an attestation.
#[test]
fn tc_455_001_a_digest_member_is_refused() {
    let mut record = json(&record_text()).as_object().unwrap().clone();
    record.set("digest", JsonValue::string("a".repeat(64)));
    assert!(prepare_change_record(&JsonValue::Object(record)).is_err());

    let mut attestation = json(&attestation_text()).as_object().unwrap().clone();
    attestation.set("digest", JsonValue::string("a".repeat(64)));
    assert!(read_attestation_json(&JsonValue::Object(attestation)).is_err());
}

/// Trace: FR-063-AC-1
///
/// A record is retained at `(record_id, revision)` and read back from there;
/// rewriting the same bytes is idempotent, different bytes are a collision.
#[test]
fn tc_455_002_records_are_addressed_by_identity_and_revision() {
    let record = prepare_change_record(&json(&record_text())).unwrap();
    let mut store = MemoryEvidenceStore::new();
    assert_eq!(
        write_change_record(&mut store, &record).unwrap(),
        Published::Retained
    );
    assert_eq!(
        write_change_record(&mut store, &record).unwrap(),
        Published::AlreadyRetained
    );
    let id = RecordId::parse("change-1", Subject::Record, "record_id").unwrap();
    let revision = Revision::new(1, Subject::Record, "revision").unwrap();
    assert_eq!(
        read_change_record(&store, &id, revision).unwrap(),
        Some(record.clone())
    );

    let mut changed = json(&record_text()).as_object().unwrap().clone();
    changed.set("impact_snapshot", {
        let mut impact = changed
            .get("impact_snapshot")
            .unwrap()
            .as_object()
            .unwrap()
            .clone();
        impact.set("revision", JsonValue::string("2"));
        JsonValue::Object(impact)
    });
    let changed = prepare_change_record(&JsonValue::Object(changed)).unwrap();
    assert!(matches!(
        write_change_record(&mut store, &changed),
        Err(ChangeAssuranceError::ContentCollision { .. })
    ));
}

/// Trace: FR-065-AC-1
///
/// Everything agreeing verifies `valid` with no reasons, and the receipt is
/// re-derived byte for byte; a receipt that differs is refused.
#[test]
fn tc_455_003_a_receipt_is_re_derived_and_compared_byte_for_byte() {
    let input = base_input();
    let receipt = verify_change_assurance(&input).unwrap();
    assert_eq!(receipt.outcome, Outcome::Valid);
    assert_eq!(receipt.reasons, Vec::new());
    let stored = receipt.to_json().unwrap();
    assert_eq!(verify_receipt(&stored, &input).unwrap(), receipt);

    let mut tampered = stored.as_object().unwrap().clone();
    tampered.set("candidate_revision", JsonValue::string("candidate-2"));
    assert!(matches!(
        verify_receipt(&JsonValue::Object(tampered), &input),
        Err(ChangeAssuranceError::ReceiptMismatch)
    ));
}

/// Trace: FR-065-AC-1
///
/// Decision events are read in `sequence` order; a gap or a repeat is an
/// invalid history, and two matching decisions are a mismatch.
#[test]
fn tc_455_004_decision_events_are_ordered_by_sequence() {
    let kind = "change_assurance.review_decided";
    let mut input = base_input();

    input.decision_history = Some(history(&[
        event_text("event-1", 1, "other"),
        event_text("event-2", 3, kind),
    ]));
    let receipt = verify_change_assurance(&input).unwrap();
    assert_eq!(receipt.reasons, vec![Reason::EventChainInvalid]);

    input.decision_history = Some(history(&[
        event_text("event-2", 2, kind),
        event_text("event-1", 1, "other"),
    ]));
    let receipt = verify_change_assurance(&input).unwrap();
    assert_eq!(receipt.outcome, Outcome::Valid);
    assert_eq!(receipt.decision_event.unwrap().sequence, 2);

    input.decision_history = Some(history(&[
        event_text("event-1", 1, kind),
        event_text("event-2", 2, kind),
    ]));
    let receipt = verify_change_assurance(&input).unwrap();
    assert_eq!(receipt.reasons, vec![Reason::DecisionMismatch]);
}

/// Trace: FR-065-AC-1
///
/// Retained output is checked by its declared size alone.
#[test]
fn tc_455_005_retained_output_is_checked_by_size() {
    let mut input = base_input();
    input.attestations[0].output = Some(b"short".to_vec());
    let receipt = verify_change_assurance(&input).unwrap();
    assert_eq!(receipt.reasons, vec![Reason::OutputSizeMismatch]);
}

/// Trace: FR-063-AC-1
///
/// Revision 2 with no retained revision 1 has a gap in its lineage.
#[test]
fn tc_455_006_a_later_revision_needs_its_predecessor() {
    let mut input = base_input();
    let mut record = json(&record_text()).as_object().unwrap().clone();
    record.set("revision", JsonValue::number(2.0).unwrap());
    input.record = JsonValue::Object(record);
    input.record_revision = Revision::new(2, Subject::Record, "revision").unwrap();
    let receipt = verify_change_assurance(&input).unwrap();
    assert!(receipt.reasons.contains(&Reason::RevisionGap));
}
