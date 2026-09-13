// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The retained operational pairs read, re-serialise byte-identically, and
//! validate.
//!
//! # Why bytes
//!
//! A pair file is named by the digest of the two record identities it carries
//! and its contents are what a pull-request diff of the evidence tree shows. A
//! port that reads the pairs but writes back different bytes has silently
//! rewritten retained evidence: every diff moves and every reader that compared
//! against a stored copy disagrees. So the assertion is byte equality through
//! [`quoin_store::canonical_json_bytes`], not value equality.
//!
//! # The anti-vacuity floor
//!
//! A walk that stopped finding pair files would pass by measuring nothing, so
//! the count is asserted and the floor is one. Retained evidence is
//! append-only: [`RETAINED_PAIRS`] may grow and may never shrink.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_measurement::operational::paths::operational_pairs_root;
use quoin_measurement::operational::read::read_operational_records;
use quoin_store::{canonical_json_bytes, parse_strict_json};

/// How many pair files `spec/evidence/operational/pairs/` retains.
const RETAINED_PAIRS: usize = 1;

/// Each pair carries exactly one capability and one exercise.
const RECORDS_PER_PAIR: usize = 2;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

fn pairs() -> Vec<(PathBuf, Vec<u8>)> {
    let root = operational_pairs_root(&repo());
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&root).expect("the pairs directory is readable") {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_some_and(|end| end == "json") {
            let bytes = std::fs::read(&path).expect("a readable pair");
            found.push((path, bytes));
        }
    }
    found.sort();
    found
}

/// Trace: FR-100-AC-2, NFR-025-AC-3
/// Provenance: quoin#472
#[test]
fn tc_472_every_retained_pair_round_trips_byte_identically() {
    let pairs = pairs();
    assert_eq!(
        pairs.len(),
        RETAINED_PAIRS,
        "anti-vacuity floor: {RETAINED_PAIRS} retained pair(s) expected, saw {} — a corpus \
         walk that finds nothing proves nothing",
        pairs.len()
    );
    for (path, bytes) in &pairs {
        let stored =
            parse_strict_json(bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let written = canonical_json_bytes(&stored)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(
            written,
            *bytes,
            "{} does not re-serialise to its retained bytes",
            path.display()
        );
    }
}

/// Trace: FR-100-AC-4
/// Provenance: quoin#472
#[test]
fn tc_472_every_retained_pair_is_a_capability_and_an_exercise_that_validate() {
    let records = read_operational_records(&repo()).expect("the retained pairs validate");
    assert_eq!(
        records.len(),
        RETAINED_PAIRS * RECORDS_PER_PAIR,
        "each retained pair carries exactly {RECORDS_PER_PAIR} validated records"
    );
}
