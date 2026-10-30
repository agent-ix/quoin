// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Retaining and reading evidence, over both evidence stores.
//!
//! Every rule below is asserted through the `EvidenceStore` trait, and the
//! same closure is run against `MemoryEvidenceStore` and `DiskEvidenceStore`.
//! The memory store enforces the same collision and whole-pair rules, so a
//! rule that holds for one and not the other is a defect in the store. What
//! only the disk store can show is durability, and the interrupted-publish
//! test is written against it alone.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use quoin_change_assurance::error::{ChangeAssuranceError, Subject};
use quoin_change_assurance::ids::{AttestationId, RecordId, Revision};
use quoin_change_assurance::intake::disk::{DiskEvidenceStore, STAGING_PREFIX};
use quoin_change_assurance::intake::memory::MemoryEvidenceStore;
use quoin_change_assurance::{
    EvidenceStore, Published, RetainedPair, intake_attestation, prepare_change_record,
    read_attestation, read_change_record, write_change_record,
};
use quoin_store::{JsonValue, canonical_bytes};

use crate::common::{OUTPUT, attestation_text, json, record_text, replacing};

/// The scenario attestation's canonical bytes and the output it attests to.
fn pair() -> (Vec<u8>, Vec<u8>) {
    (
        canonical_bytes(&json(&attestation_text())).unwrap(),
        OUTPUT.to_vec(),
    )
}

fn attestation_id() -> AttestationId {
    AttestationId::parse("att-1", Subject::Attestation, "attestation_id").unwrap()
}

/// The scenario record at `revision`.
fn record_at(revision: u32) -> JsonValue {
    replacing(
        &json(&record_text()),
        "revision",
        JsonValue::number(f64::from(revision)).unwrap(),
    )
}

/// Run one rule against both stores.
fn over_both_stores(rule: impl Fn(&str, &mut dyn EvidenceStore)) {
    let mut memory = MemoryEvidenceStore::new();
    rule("memory", &mut memory);
    let root = tempfile::tempdir().expect("a temporary repository");
    let mut disk = DiskEvidenceStore::new(root.path());
    rule("disk", &mut disk);
}

/// Trace: FR-064-AC-3
///
/// The retained output must have the declared size, and a pair that does not
/// leaves neither artifact: not the attestation, not the output. Changed-size,
/// absent and extra bytes are each tried, because a store that wrote the
/// attestation first and checked the output second would pass a test that
/// only tried one.
#[test]
fn tc_455_only_output_of_the_declared_size_is_retained() {
    let (attestation, output) = pair();
    over_both_stores(|store_name, store| {
        let longer = {
            let mut longer = output.clone();
            longer.push(0);
            longer
        };
        let shorter = output[..output.len() - 1].to_vec();
        for (why, candidate) in [
            ("shorter bytes", shorter),
            ("absent bytes", Vec::new()),
            ("extra bytes", longer),
        ] {
            let refusal = intake_attestation(store, &attestation, &candidate);
            assert!(
                matches!(refusal, Err(ChangeAssuranceError::OutputIntegrity { .. })),
                "{store_name}: {why} must be refused, got {refusal:?}"
            );
        }
        assert!(
            read_attestation(store, &attestation_id())
                .unwrap()
                .is_none(),
            "{store_name}: a refused intake must leave nothing behind"
        );

        assert_eq!(
            intake_attestation(store, &attestation, &output).unwrap(),
            Published::Retained,
            "{store_name}: the intact pair must be retained"
        );
        let (read, read_output) = read_attestation(store, &attestation_id())
            .unwrap()
            .expect("the retained pair reads back");
        assert_eq!(read.attestation_id, attestation_id());
        assert_eq!(
            read_output, output,
            "{store_name}: the output is retained whole"
        );
    });
}

/// Trace: FR-064-AC-6
///
/// Repeated identical intake preserves byte identity and reports that the
/// evidence was already retained, while a different pair under the same id is
/// a collision that leaves the first pair exactly as it was.
#[test]
fn tc_455_repeated_intake_is_identical_and_a_collision_preserves_the_first_pair() {
    let (attestation, output) = pair();
    over_both_stores(|store_name, store| {
        assert_eq!(
            intake_attestation(store, &attestation, &output).unwrap(),
            Published::Retained
        );
        assert_eq!(
            intake_attestation(store, &attestation, &output).unwrap(),
            Published::AlreadyRetained,
            "{store_name}: a repeated identical intake must not rewrite anything"
        );
        let (_, first_output) = read_attestation(store, &attestation_id()).unwrap().unwrap();
        assert_eq!(first_output, output);

        // A pair whose output differs cannot arrive through
        // `intake_attestation` - the size check refuses it first - so the
        // collision is stated at the store, which is the layer that has to
        // survive it.
        let colliding = RetainedPair {
            attestation: attestation.clone(),
            output: vec![0x42],
        };
        let refusal = store.publish_attestation(&attestation_id(), &colliding);
        assert!(
            matches!(refusal, Err(ChangeAssuranceError::ContentCollision { .. })),
            "{store_name}: a same-id different-content pair must collide, got {refusal:?}"
        );
        let (_, after) = read_attestation(store, &attestation_id()).unwrap().unwrap();
        assert_eq!(
            after, output,
            "{store_name}: the first pair must survive the collision unchanged"
        );
    });
}

/// Trace: FR-063-AC-9
///
/// Writing a successor leaves the predecessor exactly where it was, byte for
/// byte. A revision is a new record, never an edit of the one before, and
/// every revision stays addressable by `(record_id, revision)`.
#[test]
fn tc_455_writing_a_successor_leaves_every_parent_byte_untouched() {
    let id = RecordId::parse("change-1", Subject::Record, "record_id").unwrap();
    over_both_stores(|store_name, store| {
        let mut retained: Vec<(Revision, Vec<u8>)> = Vec::new();
        for number in 1..=3_u32 {
            let record = prepare_change_record(&record_at(number)).unwrap();
            assert_eq!(
                write_change_record(store, &record).unwrap(),
                Published::Retained
            );
            let revision = Revision::new(u64::from(number), Subject::Record, "revision").unwrap();
            retained.push((
                revision,
                store.record(&id, revision).unwrap().expect("just written"),
            ));
            for (earlier, bytes) in &retained {
                let found = read_change_record(store, &id, *earlier)
                    .unwrap()
                    .expect("an earlier revision is still addressable");
                assert_eq!(found.revision, *earlier);
                assert_eq!(
                    &store.record(&id, *earlier).unwrap().unwrap(),
                    bytes,
                    "{store_name}: writing a successor changed revision {earlier}"
                );
            }
        }
        assert_eq!(retained.len(), 3);
    });
}

/// Trace: FR-064-AC-8
///
/// Records and attestations occupy distinct families in the store: an
/// attestation's id never addresses a record and a record's identity never
/// addresses an attestation. Nothing here reaches outside the change-assurance
/// root, so evidence written by other families is untouched by construction.
#[test]
fn tc_455_records_and_attestations_are_distinct_store_families() {
    let (attestation, output) = pair();
    let record = prepare_change_record(&record_at(1)).unwrap();
    over_both_stores(|store_name, store| {
        write_change_record(store, &record).unwrap();
        intake_attestation(store, &attestation, &output).unwrap();

        // The record's identity is not an attestation id, and the
        // attestation's id is not a record identity.
        let record_as_attestation =
            AttestationId::parse("change-1", Subject::Attestation, "attestation_id").unwrap();
        let attestation_as_record = RecordId::parse("att-1", Subject::Record, "record_id").unwrap();
        let first = Revision::GENESIS;
        assert!(
            store.attestation(&record_as_attestation).unwrap().is_none(),
            "{store_name}: a record's identity must not address an attestation"
        );
        assert!(
            store
                .record(&attestation_as_record, first)
                .unwrap()
                .is_none(),
            "{store_name}: an attestation's id must not address a record"
        );
        assert!(
            store
                .record(
                    &RecordId::parse("change-1", Subject::Record, "record_id").unwrap(),
                    first
                )
                .unwrap()
                .is_some()
        );
        assert!(store.attestation(&attestation_id()).unwrap().is_some());
    });
}

/// Trace: FR-100-AC-9
///
/// The durability seam, on disk. A publish interrupted between staging and the
/// rename exposes neither half of the pair - a reader sees no attestation at
/// all rather than half of one - and the staging it left behind is both
/// findable and removable afterwards. A retry then publishes normally.
#[test]
fn tc_455_an_interrupted_publish_exposes_nothing_and_is_recoverable() {
    let (attestation, output) = pair();
    let root = tempfile::tempdir().expect("a temporary repository");

    let mut interrupted = DiskEvidenceStore::new(root.path()).interrupting(|| {
        Err(ChangeAssuranceError::Io {
            operation: "publish attestation",
            path: std::path::PathBuf::from("<killed>"),
            source: std::io::Error::other("interrupted before the rename"),
        })
    });
    assert!(
        intake_attestation(&mut interrupted, &attestation, &output).is_err(),
        "the injected interruption must fail the publish"
    );
    assert!(
        read_attestation(&interrupted, &attestation_id())
            .unwrap()
            .is_none(),
        "an interrupted publish must expose neither half of the pair"
    );

    let staging: Vec<_> = std::fs::read_dir(interrupted.attestations_directory())
        .expect("the attestations directory exists")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        staging.len(),
        1,
        "exactly one staging directory must be left behind, saw {staging:?}"
    );
    assert!(
        staging[0].starts_with(STAGING_PREFIX),
        "staging must be named so a reader can never resolve it: {staging:?}"
    );

    let mut store = DiskEvidenceStore::new(root.path());
    assert_eq!(
        store.recover_staging().unwrap(),
        1,
        "recovery must remove exactly the staging that was left behind"
    );
    assert_eq!(
        store.recover_staging().unwrap(),
        0,
        "a second recovery has nothing to do"
    );
    assert_eq!(
        intake_attestation(&mut store, &attestation, &output).unwrap(),
        Published::Retained,
        "the retry must publish normally"
    );
    let (_, read_output) = read_attestation(&store, &attestation_id())
        .unwrap()
        .unwrap();
    assert_eq!(read_output, output);
}
