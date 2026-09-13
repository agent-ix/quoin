// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The retained corpus reads, re-serialises byte-identically, and validates.
//!
//! # Why bytes and not values
//!
//! A port that can read the evidence store but cannot write back the same bytes
//! has changed the store's contents without saying so — every digest over a
//! collection, and every `git diff` of the evidence tree, would move. So the
//! check is byte equality through [`quoin_store::canonical_json_bytes`], over
//! all 48 collections retained under `spec/evidence/measurements/`, not a
//! value comparison that a whitespace or key-order change would pass.
//!
//! The count is asserted, and the census fails at zero: a corpus walk that
//! stopped finding files would otherwise report success by measuring nothing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_measurement::{
    DiskMeasurement, read_measurement_collection_results, read_measurement_collections,
    stored_measurement_collection,
};
use quoin_store::{canonical_json_bytes, parse_strict_json};

/// How many collections `spec/evidence/measurements/` retains.
///
/// Retained evidence is append-only, so this number may grow and may never
/// shrink. A change here is a claim that the store gained a collection.
const RETAINED_COLLECTIONS: usize = 48;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

fn collections() -> Vec<(String, Vec<u8>)> {
    let root = repo().join("spec").join("evidence").join("measurements");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&root).expect("the measurements directory is readable") {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_some_and(|end| end == "json") {
            let name = path
                .file_name()
                .expect("a file name")
                .to_string_lossy()
                .into_owned();
            out.push((name, std::fs::read(&path).expect("a readable collection")));
        }
    }
    out.sort();
    out
}

/// Trace: FR-100-AC-2, NFR-025-AC-3
/// Provenance: quoin#468
#[test]
fn tc_468_every_retained_collection_re_serialises_byte_identically() {
    let collections = collections();
    assert!(
        !collections.is_empty(),
        "anti-vacuity floor: the retained corpus read as empty, so this test proved nothing"
    );
    assert_eq!(
        collections.len(),
        RETAINED_COLLECTIONS,
        "the retained corpus changed size; evidence is append-only, so say which collection \
         arrived"
    );

    for (name, bytes) in &collections {
        let value = parse_strict_json(bytes)
            .unwrap_or_else(|error| panic!("{name}: not strict JSON: {error}"));
        let round_tripped = canonical_json_bytes(&value)
            .unwrap_or_else(|error| panic!("{name}: could not re-serialise: {error}"));
        assert_eq!(
            round_tripped, *bytes,
            "{name}: re-serialising through canonical_json_bytes changed the file's bytes"
        );
    }
}

/// Trace: FR-100-AC-2
/// Provenance: quoin#468
#[test]
fn tc_468_every_retained_collection_passes_the_ported_validator() {
    let collections = collections();
    assert_eq!(collections.len(), RETAINED_COLLECTIONS);
    for (name, bytes) in &collections {
        let value = parse_strict_json(bytes).expect("strict JSON");
        stored_measurement_collection(&value)
            .unwrap_or_else(|error| panic!("{name}: the ported validator refused it: {error}"));
    }
}

/// Trace: FR-100-AC-2, FR-100-AC-4
/// Provenance: quoin#468
#[test]
fn tc_468_the_disk_source_reads_the_whole_retained_corpus() {
    let source = DiskMeasurement::new(repo());
    let results = read_measurement_collection_results(&source).expect("the store lists");
    assert_eq!(results.len(), RETAINED_COLLECTIONS);
    for result in &results {
        assert!(
            result.collection.is_ok(),
            "{}: {:?}",
            result.path,
            result.collection.as_ref().err()
        );
    }

    let ordered = read_measurement_collections(&source).expect("every collection reads");
    assert_eq!(ordered.len(), RETAINED_COLLECTIONS);
    // `collectionOrder` sorts by instant, so the read-back is non-decreasing in
    // the timestamp text for this corpus, whose timestamps are all zulu.
    let mut previous: Option<&str> = None;
    for collection in &ordered {
        if let Some(earlier) = previous {
            assert!(
                earlier <= collection.timestamp.as_str(),
                "collections came back out of order at {}",
                collection.collection_id
            );
        }
        previous = Some(collection.timestamp.as_str());
    }
}
