// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The record-id encoding, and what `resolveRawPath` refuses.
//!
//! # Why these two together
//!
//! They are the two places intake turns caller-supplied text into a path. One
//! decides what file a record is written to; the other decides what file a
//! record is allowed to rest on. A defect in either is a store that writes
//! outside itself, and neither is visible in a happy-path corpus test, so both
//! are measured here against something other than themselves — RFC 4648's own
//! published vectors for the encoder, and a real symlink on a real filesystem
//! for the escape refusals.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::Path;

use quoin_measurement::{
    DiskMeasurement, InterventionRecordId, MAX_RECORD_ID_BYTES, MeasurementErrorCode,
    RecordIdNamespace, intervention_path, raw_evidence_for,
};

/// Trace: FR-100-AC-2
/// Provenance: quoin#471
///
/// The grammar of `intervention.ts:34`, including the byte the cap is written
/// in terms of: 128 characters pass, 129 do not.
#[test]
fn tc_471_the_record_id_grammar_is_the_retained_one() {
    for accepted in [
        "a",
        "quoin-270-cli-eval-sentinel-contract",
        "agent-ix/quoin:270",
        "A.b_c-d:e/f",
        "0",
        &"z".repeat(MAX_RECORD_ID_BYTES),
    ] {
        assert!(
            InterventionRecordId::parse(accepted).is_ok(),
            "{accepted} should parse"
        );
    }
    for refused in [
        "",
        "-leading-dash",
        ".leading-dot",
        "/leading-slash",
        "has space",
        "has\\backslash",
        "has\"quote",
        "café",
        &"z".repeat(MAX_RECORD_ID_BYTES + 1),
    ] {
        let error = InterventionRecordId::parse(refused).unwrap_err();
        assert_eq!(
            error.findings().len(),
            1,
            "{refused} should refuse with one finding"
        );
        assert!(
            error.findings()[0].starts_with("/record_id: unsafe record id "),
            "{refused}: {:?}",
            error.findings()
        );
    }
}

/// Trace: FR-100-AC-2
/// Provenance: quoin#471
///
/// The property the retained comment states and does not check: no portable
/// basename can alias an encoded one. Asserted over the ids that actually
/// stress it — a portable id that *looks* like base64url, and the encoding of
/// an id that differs from it only in the characters that force encoding.
#[test]
fn tc_471_the_two_basename_namespaces_are_disjoint() {
    let ids = [
        "quoin-270",
        "quoin_270",
        "agent-ix/quoin:270",
        "a/b",
        "a:b",
        "ab",
        "YWdlbnQtaXgvcXVvaW46Mjcw",
        &"z".repeat(MAX_RECORD_ID_BYTES),
        &format!("{}/x", "z".repeat(MAX_RECORD_ID_BYTES - 2)),
    ];
    let mut seen: Vec<(String, &str)> = Vec::new();
    for id in ids {
        let parsed = InterventionRecordId::parse(id).unwrap();
        let basename = parsed.basename();
        // Portable exactly when the id needs no encoding.
        let expected = if id.contains(['/', ':']) {
            RecordIdNamespace::Encoded
        } else {
            RecordIdNamespace::Portable
        };
        assert_eq!(basename.namespace(), expected, "{id}");
        assert!(
            basename.as_str().starts_with(expected.prefix()),
            "{id}: {basename}"
        );
        if let Some((_, other)) = seen.iter().find(|(name, _)| name == basename.as_str()) {
            panic!("{id} and {other} share the basename {basename}");
        }
        seen.push((basename.as_str().to_owned(), id));
    }
    assert_eq!(seen.len(), ids.len());
    // Anti-vacuity: both namespaces were actually exercised.
    for namespace in RecordIdNamespace::ALL {
        assert!(
            seen.iter()
                .any(|(name, _)| name.starts_with(namespace.prefix())),
            "no id encoded into {namespace:?}"
        );
    }
}

/// Trace: FR-100-AC-2
/// Provenance: quoin#471
///
/// The arithmetic the module header states: at the 128-byte cap the longer of
/// the two basenames is 173 bytes, 178 with `.json`, which is inside the
/// 255-byte `NAME_MAX` every filesystem this runs on enforces.
#[test]
fn tc_471_the_longest_basename_fits_a_file_name() {
    const NAME_MAX: usize = 255;
    let longest = format!("{}/x", "z".repeat(MAX_RECORD_ID_BYTES - 2));
    assert_eq!(longest.len(), MAX_RECORD_ID_BYTES);
    let basename = InterventionRecordId::parse(&longest).unwrap().basename();
    assert_eq!(basename.namespace(), RecordIdNamespace::Encoded);
    assert_eq!(
        basename.as_str().len(),
        2 + (MAX_RECORD_ID_BYTES * 4).div_ceil(3)
    );
    assert_eq!(basename.as_str().len(), 173);
    assert_eq!(basename.file_name().len(), 178);
    assert!(basename.file_name().len() <= NAME_MAX);
}

/// Trace: FR-100-AC-2
/// Provenance: quoin#471
///
/// Measured against RFC 4648 §10's published vectors, not against this
/// encoder's own output — with the two §5 substitutions (`+` becomes `-`, `/`
/// becomes `_`) and the padding stripped, which is what Node's `"base64url"`
/// emits.
#[test]
fn tc_471_the_encoder_reproduces_the_rfc_4648_vectors() {
    // RFC 4648 §10 lists these for base64; §5 defines base64url as the same
    // encoding over the URL-safe alphabet, and none of these vectors contains
    // a `+` or `/` output symbol, so the last two are carried from a value
    // that does: `Buffer.from("~~~~", "utf8").toString("base64url")`.
    let vectors = [
        ("", ""),
        ("f", "Zg"),
        ("fo", "Zm8"),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg"),
        ("fooba", "Zm9vYmE"),
        ("foobar", "Zm9vYmFy"),
        ("~~~~", "fn5-fg"),
        ("agent-ix/quoin:270", "YWdlbnQtaXgvcXVvaW46Mjcw"),
    ];
    for (plain, expected) in vectors {
        assert_eq!(
            quoin_measurement::intervention::ids::base64url_no_pad(plain.as_bytes()),
            expected,
            "base64url({plain:?})"
        );
    }
}

/// Trace: FR-100-AC-2
/// Provenance: quoin#471
///
/// The encoder's alphabet is RFC 4648 §5's table, in order, and every symbol
/// it can emit comes from it — so the non-panicking fallback in `symbol` is
/// dead code rather than a silent replacement character in a file name.
#[test]
fn tc_471_the_encoder_alphabet_is_the_url_safe_table() {
    let alphabet = quoin_measurement::intervention::ids::base64url_alphabet();
    assert_eq!(
        core::str::from_utf8(alphabet).unwrap(),
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
    );
    let mut sorted = alphabet.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 64, "the alphabet repeats a symbol");
    // Every byte value, so every 6-bit group the encoder can form is covered.
    let all: Vec<u8> = (0..=255).collect();
    let encoded = quoin_measurement::intervention::ids::base64url_no_pad(&all);
    assert!(
        encoded.bytes().all(|byte| alphabet.contains(&byte)),
        "the encoder emitted a symbol outside its alphabet"
    );
}

/// Trace: FR-100-AC-2
/// Provenance: quoin#471
#[test]
fn tc_471_a_record_id_names_a_file_under_the_interventions_directory() {
    let repo = Path::new("/repo");
    let id = InterventionRecordId::parse("agent-ix/quoin:270").unwrap();
    assert_eq!(
        intervention_path(repo, &id),
        Path::new("/repo/spec/evidence/interventions/b-YWdlbnQtaXgvcXVvaW46Mjcw.json")
    );
}

/// Trace: FR-100-AC-2, FR-100-AC-4
/// Provenance: quoin#471
///
/// `resolveRawPath`'s filesystem half, on a real tree: a symlink that leaves
/// the store is refused even though every lexical guard passes, and an absent
/// path and a directory are refused with the oracle's own sentence.
#[test]
fn tc_471_raw_evidence_resolution_refuses_every_escape() {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let repo = temporary.path();
    let store = repo.join("spec").join("evidence");
    std::fs::create_dir_all(store.join("raw")).expect("the store is creatable");
    std::fs::write(store.join("raw").join("run.json"), b"{}").expect("a retained file");
    std::fs::create_dir_all(repo.join("outside")).expect("a directory outside the store");
    let secret = repo.join("outside").join("secret.json");
    std::fs::write(&secret, b"{}").expect("a file outside the store");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&secret, store.join("raw").join("escape.json"))
        .expect("a symlink out of the store");
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        store.join("raw").join("run.json"),
        store.join("raw").join("inside.json"),
    )
    .expect("a symlink within the store");

    let source = DiskMeasurement::new(repo);
    assert!(
        raw_evidence_for(&source, "raw/run.json", "application/json").is_ok(),
        "the retained file itself must still resolve"
    );

    for (path, expected) in [
        (
            "raw/absent.json",
            "retained evidence file is absent: raw/absent.json",
        ),
        ("raw", "retained evidence file is absent: raw"),
    ] {
        let error = raw_evidence_for(&source, path, "application/json").unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::RawEvidencePathUnsafe);
        assert_eq!(error.subject(), expected);
    }

    #[cfg(unix)]
    {
        let error = raw_evidence_for(&source, "raw/escape.json", "application/json").unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::RawEvidencePathUnsafe);
        assert_eq!(
            error.subject(),
            "path resolves outside evidence store: raw/escape.json"
        );
        // The mirror case, so the refusal above is about *escaping* and not
        // about symlinks: one that stays inside resolves, and resolves to the
        // same bytes as its target (`intervention.ts` reads through it too).
        let followed = raw_evidence_for(&source, "raw/inside.json", "application/json")
            .expect("a symlink inside the store resolves");
        let direct = raw_evidence_for(&source, "raw/run.json", "application/json").unwrap();
        assert_eq!(followed.digest, direct.digest);
        assert_eq!(followed.size_bytes, direct.size_bytes);
    }

    // The lexical guards still hold on this source, and they refuse before the
    // filesystem is touched at all.
    for lexical in ["../outside/secret.json", "/etc/passwd", "raw/../../x", ""] {
        let error = raw_evidence_for(&source, lexical, "application/json").unwrap_err();
        assert_eq!(error.code(), MeasurementErrorCode::RawEvidencePathUnsafe);
        assert!(
            error.subject().starts_with("unsafe relative path "),
            "{lexical}: {}",
            error.subject()
        );
    }
}
