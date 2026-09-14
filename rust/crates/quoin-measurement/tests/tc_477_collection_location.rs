// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! An unreadable collection is named by its whole path, not by its file name.
//!
//! Trace: FR-100-AC-4
//! Provenance: quoin#477
//!
//! # The claim under test
//!
//! `store.ts:84` builds every read result's `path` as
//! `join(measurementsRoot(repo), name)`, and `store.ts:63` renders that whole
//! path in `${result.path}: unreadable measurement collection`. The port's
//! result carried the **bare file name** the [`MeasurementSource`] seam listed,
//! so the same refusal read `c-broken.json: unreadable measurement collection`
//! where the retained reader says
//! `<repo>/spec/evidence/measurements/c-broken.json: …`.
//!
//! A source that is not a directory has no such path to build, so the join
//! belongs to the seam rather than to each caller:
//! [`MeasurementSource::collection_location`] answers it, a disk source joins
//! and an in-memory source returns the name it was given.
//!
//! # Why this file exists at all
//!
//! The fix changes a sentence a person reads and adds a method to a trait, and
//! nothing else in the workspace asserted either. A correctness claim the gates
//! cannot evaluate is not a correctness claim.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::Path;

use quoin_measurement::source::{DiskMeasurement, MeasurementSource, MemoryMeasurement};
use quoin_measurement::store::{
    measurements_root, read_measurement_collection_results, read_measurement_collections,
};

/// The file every case below makes unreadable.
const BROKEN: &str = "c-broken.json";

/// The bytes that make it unreadable: valid UTF-8, not a collection.
const NOT_A_COLLECTION: &[u8] = b"{ \"schemaVersion\": 1 }";

/// The sentence a refusal ends with, and what is read back from in front of it.
const SENTENCE: &str = ": unreadable measurement collection";

/// The lowest number of results the store must list for this test to mean
/// anything. One: a store that listed nothing would satisfy every `for` loop
/// below without entering it.
const RESULT_FLOOR: usize = 1;

/// A repository whose measurement store holds exactly one, unreadable,
/// collection.
fn repository_with_one_broken_collection(root: &Path) {
    let store = measurements_root(root);
    std::fs::create_dir_all(&store).expect("the store directory is creatable");
    std::fs::write(store.join(BROKEN), NOT_A_COLLECTION).expect("the collection is writable");
}

/// The disk seam names a collection by the whole path, which is what the
/// refusal then renders.
///
/// Trace: FR-100-AC-4
/// Provenance: quoin#477
#[test]
fn tc_477_020_a_disk_refusal_names_the_whole_path() {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let root = temporary.path();
    repository_with_one_broken_collection(root);
    let source = DiskMeasurement::new(root);

    let expected = measurements_root(root)
        .join(BROKEN)
        .to_string_lossy()
        .into_owned();

    let results =
        read_measurement_collection_results(&source).expect("the store lists its collections");
    assert!(
        results.len() >= RESULT_FLOOR,
        "the store listed {} collections, below the floor of {RESULT_FLOOR}",
        results.len()
    );
    for result in &results {
        assert_eq!(
            result.path, expected,
            "a read result must carry the whole path, as `store.ts:84` does"
        );
    }

    let refused = read_measurement_collections(&source)
        .expect_err("a collection that is not a collection refuses the read");
    let message = refused.to_string();

    // `contains(&expected)` alone would not catch the mutation this test exists
    // for: the whole path *ends with* the bare name, so a message rendering
    // only the name is a substring of what a correct one would say. Read what
    // actually precedes the sentence instead.
    let named = message
        .split_once(SENTENCE)
        .unwrap_or_else(|| panic!("the refusal is {message:?}, which does not say {SENTENCE:?}"))
        .0;
    assert!(
        named.ends_with(&expected),
        "the refusal names {named:?}, not the whole path {expected:?} — removing the join in \
         `MeasurementSource::collection_location` is exactly what this reads like"
    );
    assert!(
        expected.contains("spec/evidence/measurements/"),
        "the expected path {expected:?} is not under the measurement store, so this test would \
         pass without the join"
    );
}

/// A source with no directory answers with the name it was given, and says so.
///
/// The seam is asked rather than assumed precisely because this answer is not
/// a path. Asserting it keeps the two implementations from converging on a
/// join an in-memory store cannot make.
///
/// Trace: FR-100-AC-4
/// Provenance: quoin#477
#[test]
fn tc_477_021_an_in_memory_source_names_what_it_was_given() {
    let source = MemoryMeasurement::new().with_collection(BROKEN, NOT_A_COLLECTION);
    assert_eq!(source.collection_location(BROKEN), BROKEN);

    let results =
        read_measurement_collection_results(&source).expect("the map lists its collections");
    assert!(
        results.len() >= RESULT_FLOOR,
        "the map listed {} collections, below the floor of {RESULT_FLOOR}",
        results.len()
    );
    assert_eq!(results[0].path, BROKEN);

    let refused = read_measurement_collections(&source)
        .expect_err("a collection that is not a collection refuses the read");
    assert!(
        refused.to_string().contains(&format!("{BROKEN}{SENTENCE}")),
        "the refusal is {:?}",
        refused.to_string()
    );
}
