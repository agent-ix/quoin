// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Retaining and reading evidence, over both evidence stores.
//!
//! # One analysis, two stores
//!
//! Every rule below is asserted through the `EvidenceStore` trait, and the
//! same closure is run against [`MemoryEvidenceStore`] and
//! [`DiskEvidenceStore`]. The memory store is not a stand-in for the disk one:
//! it enforces the same collision and whole-pair rules, so a rule that holds
//! for one and not the other is a defect in the store rather than a property
//! of the filesystem. What only the disk store can show is durability, and the
//! interrupted-publish test below is written against it alone.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use common::{bytes, member, oracle, section, text};
use quoin_change_assurance::error::ChangeAssuranceError;
use quoin_change_assurance::intake::disk::{DiskEvidenceStore, STAGING_PREFIX};
use quoin_change_assurance::intake::memory::MemoryEvidenceStore;
use quoin_change_assurance::{
    EvidenceStore, Published, RetainedPair, intake_attestation, read_attestation,
    read_change_record, seal_change_record, write_change_record,
};
use quoin_store::{JsonValue, canonical_bytes};

/// The first captured attestation and the output it attests to.
fn pair() -> (Vec<u8>, Vec<u8>) {
    let captured = section(&oracle(), "attestations");
    let first = captured
        .iter()
        .find(|entry| !bytes(member(entry, "output")).is_empty())
        .expect("an attestation with a non-empty output");
    (
        canonical_bytes(member(first, "sealed")).unwrap(),
        bytes(member(first, "output")),
    )
}

/// The captured record chain, sealed.
fn chain() -> Vec<JsonValue> {
    section(&oracle(), "records")
        .iter()
        .filter(|entry| text(entry, "name").starts_with("chain-revision-"))
        .map(|entry| member(entry, "sealed").clone())
        .collect()
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
/// The retained bytes must reproduce the declared digest and size, and a pair
/// that does not leaves *neither* artifact: not the attestation, not the
/// output. Changed, absent and extra bytes are each tried, because a store
/// that wrote the attestation first and checked the output second would pass a
/// test that only tried one.
#[test]
fn tc_455_only_bytes_that_reproduce_the_declared_digest_are_retained() {
    let (attestation, output) = pair();
    over_both_stores(|store_name, store| {
        let mut changed = output.clone();
        changed[0] ^= 0xff;
        let extra = {
            let mut longer = output.clone();
            longer.push(0);
            longer
        };
        for (why, candidate) in [
            ("changed bytes", changed),
            ("absent bytes", Vec::new()),
            ("extra bytes", extra),
        ] {
            let refusal = intake_attestation(store, &attestation, &candidate);
            assert!(
                matches!(refusal, Err(ChangeAssuranceError::OutputIntegrity { .. })),
                "{store_name}: {why} must be refused, got {refusal:?}"
            );
        }
        // Nothing above was retained: the digest the attestation declares
        // still names nothing at all.
        let sealed = quoin_change_assurance::attestations::verify_attestation(
            &quoin_store::parse_strict_json(&attestation).unwrap(),
        )
        .unwrap();
        assert!(
            read_attestation(store, &sealed.digest).unwrap().is_none(),
            "{store_name}: a refused intake must leave nothing behind"
        );

        assert_eq!(
            intake_attestation(store, &attestation, &output).unwrap(),
            Published::Retained,
            "{store_name}: the intact pair must be retained"
        );
        let (read, read_output) = read_attestation(store, &sealed.digest)
            .unwrap()
            .expect("the retained pair reads back");
        assert_eq!(read.digest, sealed.digest);
        assert_eq!(
            read_output, output,
            "{store_name}: the output is retained whole"
        );
    });
}

/// Trace: FR-064-AC-6
///
/// Repeated identical intake preserves byte identity and reports that the
/// evidence was already retained, while a different pair under the same digest
/// is a collision that leaves the first pair exactly as it was. The second
/// half is what makes a content-addressed store safe to write to twice.
#[test]
fn tc_455_repeated_intake_is_identical_and_a_collision_preserves_the_first_pair() {
    let (attestation, output) = pair();
    over_both_stores(|store_name, store| {
        let sealed = quoin_change_assurance::attestations::verify_attestation(
            &quoin_store::parse_strict_json(&attestation).unwrap(),
        )
        .unwrap();
        assert_eq!(
            intake_attestation(store, &attestation, &output).unwrap(),
            Published::Retained
        );
        assert_eq!(
            intake_attestation(store, &attestation, &output).unwrap(),
            Published::AlreadyRetained,
            "{store_name}: a repeated identical intake must not rewrite anything"
        );
        let (_, first_output) = read_attestation(store, &sealed.digest).unwrap().unwrap();
        assert_eq!(first_output, output);

        // A pair whose output differs cannot arrive through `intake_attestation`
        // — the integrity check refuses it first — so the collision is stated
        // at the store, which is the layer that has to survive it.
        let colliding = RetainedPair {
            attestation: attestation.clone(),
            output: vec![0x42],
        };
        let refusal = store.publish_attestation(&sealed.digest, &colliding);
        assert!(
            matches!(refusal, Err(ChangeAssuranceError::ContentCollision { .. })),
            "{store_name}: a same-digest different-content pair must collide, got {refusal:?}"
        );
        let (_, after) = read_attestation(store, &sealed.digest).unwrap().unwrap();
        assert_eq!(
            after, output,
            "{store_name}: the first pair must survive the collision unchanged"
        );
    });
}

/// Trace: FR-063-AC-9
///
/// Writing a successor leaves the parent exactly where it was, byte for byte.
/// A revision is a new record, never an edit of the one before, and a reader
/// holding a parent's digest must still find the parent afterwards.
#[test]
fn tc_455_writing_a_successor_leaves_every_parent_byte_untouched() {
    let chain = chain();
    assert_eq!(chain.len(), 3, "the capture must hold a three-link chain");
    over_both_stores(|store_name, store| {
        let mut retained: Vec<(String, Vec<u8>)> = Vec::new();
        for sealed in &chain {
            let record = seal_change_record(sealed).expect("a captured record seals");
            assert_eq!(
                write_change_record(store, &record).unwrap(),
                Published::Retained
            );
            retained.push((
                record.digest.as_hex().to_owned(),
                store.record(&record.digest).unwrap().expect("just written"),
            ));
            // Every earlier revision is still there, and still identical.
            for (digest, bytes) in &retained {
                let found = read_change_record(
                    store,
                    &quoin_store::CanonicalDigest::parse_stored(digest).unwrap(),
                )
                .unwrap()
                .expect("an earlier revision is still addressable");
                assert_eq!(found.digest.as_hex(), digest);
                assert_eq!(
                    &store
                        .record(&quoin_store::CanonicalDigest::parse_stored(digest).unwrap())
                        .unwrap()
                        .unwrap(),
                    bytes,
                    "{store_name}: writing a successor changed {digest}"
                );
            }
        }
        assert_eq!(retained.len(), 3);
    });
}

/// Trace: FR-064-AC-8
///
/// Records and attestations occupy distinct families in the store: the same
/// digest addresses one of each without either standing in for the other.
/// Nothing here reaches outside the change-assurance root, so evidence written
/// by other families is untouched by construction.
#[test]
fn tc_455_records_and_attestations_are_distinct_store_families() {
    let (attestation, output) = pair();
    let sealed_record = seal_change_record(&chain()[0]).expect("a captured record seals");
    over_both_stores(|store_name, store| {
        let sealed = quoin_change_assurance::attestations::verify_attestation(
            &quoin_store::parse_strict_json(&attestation).unwrap(),
        )
        .unwrap();
        write_change_record(store, &sealed_record).unwrap();
        intake_attestation(store, &attestation, &output).unwrap();

        assert!(
            store.attestation(&sealed_record.digest).unwrap().is_none(),
            "{store_name}: a record's digest must not address an attestation"
        );
        assert!(
            store.record(&sealed.digest).unwrap().is_none(),
            "{store_name}: an attestation's digest must not address a record"
        );
        assert!(store.record(&sealed_record.digest).unwrap().is_some());
        assert!(store.attestation(&sealed.digest).unwrap().is_some());
    });
}

/// Trace: FR-100-AC-9
///
/// The durability seam, on disk. A publish interrupted between staging and the
/// rename exposes neither half of the pair — a reader sees no attestation at
/// all rather than half of one — and the staging it left behind is both
/// findable and removable afterwards. A retry then publishes normally.
#[test]
fn tc_455_an_interrupted_publish_exposes_nothing_and_is_recoverable() {
    let (attestation, output) = pair();
    let sealed = quoin_change_assurance::attestations::verify_attestation(
        &quoin_store::parse_strict_json(&attestation).unwrap(),
    )
    .unwrap();
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
        read_attestation(&interrupted, &sealed.digest)
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
    let (_, read_output) = read_attestation(&store, &sealed.digest).unwrap().unwrap();
    assert_eq!(read_output, output);
}
