// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The change-assurance store path, replayed against a store the oracle wrote.
//!
//! The ecosystem-wide replay found **no reachable repository containing a
//! change-assurance record or a proof attestation**. Every real store holds
//! runs, scans, bindings, baselines, measurements and interventions, all in the
//! pretty form. So the RFC 8785 on-disk form, the digest that *names* a record,
//! and the attestation/retained-output pairing had no production instance to
//! replay against.
//!
//! `oracle/build-change-assurance-fixture.mjs` writes one through the shipped
//! TypeScript writers — `sealChangeRecord`, `writeChangeRecord`,
//! `sealAttestation`, `intakeAttestation` — and the result is committed at
//! `tests/fixtures/change-assurance-store/`. Every byte, filename and digest in
//! it was produced by the oracle. This suite asserts that Rust reads it,
//! reproduces its bytes exactly, and recomputes every digest it carries.
//!
//! This is a fixture, not a production store, and the distinction matters: it
//! proves the two implementations agree on this shape, not that the shape
//! occurs in the wild. It does not.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_store::replay::{ReplayReport, StoreForm, replay_repository};
use quoin_store::{
    CanonicalDigest, RawBytesDigest, canonicalize_jcs, digest_raw_bytes, parse_strict_json,
    verify_record_digest,
};

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("change-assurance-store")
}

fn files_under(root: &Path, name_suffix: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.to_string_lossy().ends_with(name_suffix) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Every sealed record reproduces its own digest, and its filename is that
/// digest.
///
/// Trace: FR-098-CON-1
#[test]
fn tc_380_every_sealed_record_verifies_and_is_named_by_its_digest() {
    let records = files_under(
        &fixture_root().join("spec/evidence/change-assurance/records"),
        ".json",
    );
    assert_eq!(records.len(), 3, "the fixture must not lose records");
    for path in records {
        let bytes = std::fs::read(&path).expect("read");
        let value = parse_strict_json(&bytes).expect("the oracle wrote strict JSON");
        let object = value.as_object().expect("a record is an object");
        let digest = verify_record_digest(object).expect("record verifies against its own digest");
        let stem = path
            .file_stem()
            .expect("stem")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            stem,
            digest.as_hex(),
            "{}: filename is the digest",
            path.display()
        );
        assert_eq!(
            CanonicalDigest::parse_stored(&stem).expect("parses"),
            digest
        );
    }
}

/// The change-assurance family is stored as RFC 8785 bytes, not as the pretty
/// form, and Rust reproduces those bytes exactly.
///
/// Trace: FR-100-AC-2
#[test]
fn tc_380_change_assurance_files_are_jcs_bytes_and_round_trip_exactly() {
    let root = fixture_root().join("spec/evidence/change-assurance");
    let files = files_under(&root, ".json");
    assert_eq!(files.len(), 7, "3 records plus 4 attestations");
    for path in files {
        assert_eq!(StoreForm::for_path(&path), StoreForm::Jcs);
        let bytes = std::fs::read(&path).expect("read");
        let value = parse_strict_json(&bytes).expect("strict JSON");
        assert_eq!(
            canonicalize_jcs(&value)
                .expect("depth within budget")
                .into_bytes(),
            bytes,
            "{}: re-serialization is not byte-identical",
            path.display()
        );
        assert!(
            !bytes.ends_with(b"\n"),
            "{}: the JCS form carries no trailing newline",
            path.display()
        );
    }
}

/// Every retained output's raw-bytes digest and size match its attestation,
/// including an empty output and one that is not JSON at all.
///
/// Trace: FR-098-CON-1
#[test]
fn tc_380_retained_outputs_match_their_attestations_in_the_raw_domain() {
    let root = fixture_root().join("spec/evidence/change-assurance/attestations");
    let outputs = files_under(&root, "output.bin");
    assert_eq!(outputs.len(), 4);
    let mut sizes = Vec::new();
    for output_path in outputs {
        let attestation_path = output_path
            .parent()
            .expect("parent")
            .join("attestation.json");
        let attestation = parse_strict_json(&std::fs::read(&attestation_path).expect("read"))
            .expect("strict JSON");
        let object = attestation.as_object().expect("object");
        let retained = object
            .get("retained_output")
            .expect("retained_output")
            .as_object()
            .expect("object");

        let bytes = std::fs::read(&output_path).expect("read");
        let actual = digest_raw_bytes(&bytes);
        let stored = RawBytesDigest::parse_stored(
            retained
                .get("digest")
                .and_then(|v| v.as_str())
                .expect("digest"),
        )
        .expect("parses");
        assert_eq!(stored, actual, "{}", output_path.display());

        let declared = retained
            .get("size_bytes")
            .and_then(quoin_store::JsonValue::as_f64)
            .expect("size_bytes");
        #[allow(clippy::cast_precision_loss, reason = "compared, never stored")]
        let observed = bytes.len() as f64;
        #[allow(
            clippy::float_cmp,
            reason = "an integral size must match exactly; a tolerance would hide corruption"
        )]
        {
            assert_eq!(declared, observed, "{}", output_path.display());
        }
        sizes.push(bytes.len());
    }
    sizes.sort_unstable();
    assert_eq!(sizes[0], 0, "one retained output must be empty");
    assert!(sizes[3] > 1000, "one retained output must be substantial");
}

/// The replay tool, run with no oracle, reports the fixture's self-consistency
/// and finds nothing wrong with it.
///
/// Trace: FR-098-CON-3, FR-098-AC-7
#[test]
fn tc_380_replay_reports_the_fixture_as_internally_consistent() {
    let mut report = ReplayReport::default();
    replay_repository(&fixture_root(), "fixture", None, &mut report).expect("replay runs");
    assert_eq!(report.files_found, 7);
    assert_eq!(report.files_parsed, 7);
    assert_eq!(report.round_trip_identical, 7);
    assert_eq!(report.sealed_records_verified, 7);
    assert_eq!(report.retained_outputs_verified, 4);
    assert_eq!(report.raw_outputs_digested, 4);
    assert!(report.integrity_findings.is_empty());
    assert!(report.proto_member_files.is_empty());
    // Self-consistency is NOT the gate. No oracle was supplied, so nothing was
    // compared, and a report that compared nothing must never claim PASS.
    assert!(!report.oracle_supplied);
    assert_eq!(report.oracle_comparisons_performed, 0);
    assert!(
        !report.gate_passes(),
        "a replay with no oracle compares nothing and cannot pass the gate"
    );
    // 7 documents' worth of nodes, plus one self-verification digest per
    // sealed record. The exact node total is a property of the fixture; assert
    // that it is substantial rather than pinning a number the fixture owns.
    assert!(report.digests_replayed > 200, "{}", report.digests_replayed);
}

/// A record whose bytes are altered is refused, not silently accepted.
///
#[test]
fn tc_380_a_tampered_record_fails_its_own_digest() {
    let records = files_under(
        &fixture_root().join("spec/evidence/change-assurance/records"),
        ".json",
    );
    let bytes = std::fs::read(&records[0]).expect("read");
    let text = String::from_utf8(bytes).expect("utf8");
    let tampered = text.replace("\"abc123\"", "\"abc124\"");
    assert_ne!(tampered, text, "the fixture must contain the edited value");
    let value = parse_strict_json(tampered.as_bytes()).expect("still strict JSON");
    let error = verify_record_digest(value.as_object().expect("object"))
        .expect_err("a tampered record must not verify");
    assert_eq!(error.code(), quoin_store::StoreErrorCode::DigestMismatch);
}
