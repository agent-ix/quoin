// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The replay gate's own sensitivity.
//!
//! Every other suite asks whether Rust agrees with the oracle. This one asks
//! the prior question: **would the gate notice if it did not?**
//!
//! A gate that compares nothing reports zero mismatches, and zero mismatches
//! reads as agreement. The three ways to compare nothing — no oracle at all, an
//! empty oracle, and an oracle whose keys are spelled differently from this
//! side's — must each be a FAIL, and the compared population must be counted
//! and reported rather than inferred from a mismatch count of zero.
//!
//! The oracle used here is constructed from the fixture's own bytes rather than
//! captured from Node. That is deliberate and is not circular: these tests
//! assert what the *gate mechanism* does with a given capture, and whether the
//! capture's contents match TypeScript is what
//! `tc_change_assurance_store.rs` and `tc_jcs_adversarial.rs` assert.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_store::replay::{Oracle, OracleFile, StoreForm, node_digests, oracle_label, replay};
use quoin_store::{digest_raw_bytes, parse_strict_json};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("change-assurance-store")
}

fn walk(root: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(root)
        .map(|read| read.flatten().map(|entry| entry.path()).collect())
        .unwrap_or_default();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(&path, out);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
            || path.file_name().is_some_and(|name| name == "output.bin")
        {
            out.push(path);
        }
    }
}

/// Build the capture a faithful oracle would have produced for `repo`.
fn oracle_for(repo: &Path, label: &str) -> Oracle {
    let root = repo.join("spec").join("evidence");
    let mut files = Vec::new();
    walk(&root, &mut files);
    let mut oracle = Oracle::default();
    for path in files {
        let relative = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let key = format!("{label}/{relative}");
        let bytes = std::fs::read(&path).unwrap();
        if path.file_name().is_some_and(|name| name == "output.bin") {
            oracle
                .raw_files
                .insert(key, digest_raw_bytes(&bytes).as_hex().to_owned());
            continue;
        }
        let value = parse_strict_json(&bytes).expect("the fixture parses");
        let serialized = StoreForm::for_path(&path).serialize(&value).unwrap();
        oracle.files.insert(
            key,
            OracleFile {
                path: relative,
                parsed: true,
                error: None,
                serialization_digest: Some(digest_raw_bytes(&serialized).as_hex().to_owned()),
                round_trip_identical: Some(serialized == bytes),
                node_digests: node_digests(&value).unwrap(),
            },
        );
    }
    oracle
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let source = entry.path();
        let target = to.join(entry.file_name());
        if source.is_dir() {
            copy_tree(&source, &target);
        } else {
            std::fs::copy(&source, &target).unwrap();
        }
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "quoin-store-gate-{}-{name}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A faithful oracle compares every store entity and the gate passes.
///
/// The compared population is asserted explicitly: it is the number every
/// "zero mismatches" claim is a claim *about*, and the gate requires it to
/// cover every entity walked.
///
/// Trace: FR-098-AC-3
#[test]
fn tc_380_a_faithful_oracle_compares_every_entity_and_the_gate_passes() {
    let repo = fixture_root();
    let label = oracle_label(&repo);
    let oracle = oracle_for(&repo, &label);
    let report = replay(&[repo], Some(&oracle)).expect("replay runs");

    assert!(report.oracle_supplied);
    assert_eq!(report.files_found, 7);
    assert_eq!(report.raw_outputs_digested, 4);
    assert_eq!(report.store_entities_walked(), 11);
    assert_eq!(report.oracle_comparisons_performed, 11);
    assert!(report.oracle_entries_unmatched.is_empty());
    assert!(report.store_entries_unmatched.is_empty());
    assert!(report.divergences.is_empty());
    assert!(report.gate_passes());
}

/// A single planted byte makes the replay fail.
///
/// This is the test that would have caught a gate reporting PASS over an empty
/// comparison: if the gate cannot fail on a one-byte change, it is not
/// measuring anything.
///
/// Trace: FR-098-AC-3
#[test]
fn tc_380_a_planted_single_byte_change_makes_the_replay_fail() {
    let source = fixture_root();
    let label = oracle_label(&source);
    let oracle = oracle_for(&source, &label);

    let scratch = scratch("planted");
    let repo = scratch.join("repo");
    copy_tree(&source, &repo);

    // The oracle is keyed on the pristine label, so replay the copy under that
    // same label: the only difference between the two runs is the planted byte.
    let records = repo.join("spec/evidence/change-assurance/records");
    let victim = std::fs::read_dir(&records)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .min()
        .expect("the fixture has records");
    let mut bytes = std::fs::read(&victim).unwrap();
    // Flip one byte of one ASCII value character: still valid JSON, still
    // parses, and changes exactly one canonical byte.
    let position = bytes
        .iter()
        .rposition(u8::is_ascii_digit)
        .expect("a record carries digits");
    bytes[position] = if bytes[position] == b'9' { b'8' } else { b'9' };
    std::fs::write(&victim, &bytes).unwrap();

    let mut report = quoin_store::replay::ReplayReport {
        oracle_supplied: true,
        ..quoin_store::replay::ReplayReport::default()
    };
    quoin_store::replay::replay_repository(&repo, &label, Some(&oracle), &mut report)
        .expect("replay runs");

    assert!(
        !report.divergences.is_empty(),
        "a planted byte must surface as a divergence"
    );
    assert!(
        !report.gate_passes(),
        "a planted byte must fail the gate, not be absorbed by it"
    );
    std::fs::remove_dir_all(&scratch).ok();
}

/// A run that compared nothing is inconclusive, never a pass.
///
/// Trace: FR-098-CON-3, FR-098-AC-7
#[test]
fn tc_380_a_run_whose_compared_population_is_zero_cannot_pass_the_gate() {
    let repo = fixture_root();

    // No oracle at all.
    let report = replay(std::slice::from_ref(&repo), None).expect("replay runs");
    assert_eq!(report.oracle_comparisons_performed, 0);
    assert!(!report.gate_passes());
    assert!(report.divergences.is_empty(), "nothing was compared");

    // An oracle that happens to be empty: same zero population, same verdict.
    let empty = Oracle::default();
    let report = replay(&[repo], Some(&empty)).expect("replay runs");
    assert_eq!(report.oracle_comparisons_performed, 0);
    assert_eq!(report.store_entries_unmatched.len(), 11);
    assert!(
        !report.gate_passes(),
        "zero mismatches over zero comparisons is not agreement"
    );
}

/// An oracle whose keys are spelled differently is a FAILURE, not a pass.
///
/// The capture keys on the resolved repository path and this side used to key
/// on the argument as typed. A relative argument therefore produced a total key
/// mismatch, every lookup missed, `divergences` stayed empty and the gate said
/// PASS over nothing. Both unmatched populations must now be non-empty and the
/// gate must fail.
///
/// Trace: FR-098-CON-3, FR-098-AC-7
#[test]
fn tc_380_a_key_shape_mismatch_fails_the_gate_instead_of_passing_silently() {
    let repo = fixture_root();
    let oracle = oracle_for(&repo, "a-label-this-side-will-never-produce");
    let report = replay(&[repo], Some(&oracle)).expect("replay runs");

    assert_eq!(report.oracle_comparisons_performed, 0);
    assert_eq!(report.store_entries_unmatched.len(), 11);
    assert_eq!(report.oracle_entries_unmatched.len(), 11);
    assert!(report.divergences.is_empty(), "nothing was compared");
    assert!(
        !report.gate_passes(),
        "a key-shape mismatch is a failure to compare, and a failure to compare is a gate failure"
    );
}

/// A capture that covers only part of the store fails the gate.
///
/// The capture script can die partway through and leave a truncated ndjson.
/// The Rust side must refuse a capture whose entries do not cover the files it
/// walked, rather than comparing the prefix and reporting a pass.
///
/// Trace: FR-098-CON-3, FR-098-AC-7
#[test]
fn tc_380_a_truncated_capture_does_not_pass_the_gate() {
    let repo = fixture_root();
    let label = oracle_label(&repo);
    let mut oracle = oracle_for(&repo, &label);
    let dropped = oracle.files.keys().next().cloned().expect("entries");
    oracle.files.remove(&dropped);

    let report = replay(&[repo], Some(&oracle)).expect("replay runs");
    assert_eq!(report.oracle_comparisons_performed, 10);
    assert_eq!(report.store_entities_walked(), 11);
    assert_eq!(report.store_entries_unmatched, vec![dropped]);
    assert!(report.divergences.is_empty());
    assert!(!report.gate_passes());
}

/// The label this side builds matches Node's `path.resolve` for the same
/// argument, so the two sides key on the same string.
///
/// Trace: FR-098-CON-3
#[test]
fn tc_380_the_oracle_label_resolves_the_way_node_resolves() {
    let cwd = std::env::current_dir().unwrap();
    let expected = cwd.join("a").join("b").to_string_lossy().into_owned();

    assert_eq!(oracle_label(Path::new("a/b")), expected);
    assert_eq!(oracle_label(Path::new("./a/b")), expected);
    assert_eq!(oracle_label(Path::new("a/b/")), expected);
    assert_eq!(oracle_label(Path::new("a/c/../b")), expected);
    assert_eq!(oracle_label(Path::new("./a/./b/")), expected);
    assert_eq!(oracle_label(&cwd.join("a/b")), expected);
    assert_eq!(oracle_label(Path::new("/x/y/../z")), "/x/z");
}
