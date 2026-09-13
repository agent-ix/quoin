// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The agent-ix/quoin#164 payload, read through the Cargo edge.
//!
//! `tests/fixtures/coverage-volume/PROVENANCE.md` records where the bytes came
//! from, what the TypeScript oracle said about them once, and why the file is
//! checked in rather than regenerated. Nothing here runs `node`, `quire`, or
//! any other process: the oracle ran once, its answer is in that note, and
//! these assertions stand on their own against the same bytes.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_quire::{CoveragePayload, ErrorCode, PayloadLimit, engine, payload};

/// The captured payload.
const VOLUME: &[u8] = include_bytes!("fixtures/coverage-volume/filament-ide-rs-coverage.json");

/// Exactly what the file holds, so a silent truncation on checkout or a
/// line-ending rewrite fails here and not as a puzzling parse error.
const VOLUME_BYTES: usize = 1_632_821;

/// The payload size that caused #164, for the comparison the bound exists for.
const INCIDENT_BYTES: usize = 1_090_714;

/// Trace: NFR-024
///
/// The headline: a payload half again the size of the one that killed six
/// commands is read whole, by value, with no ceiling anywhere near it.
#[test]
fn tc_379_100_the_incident_payload_is_read_whole() {
    assert_eq!(VOLUME.len(), VOLUME_BYTES, "the fixture changed size");
    assert!(
        VOLUME.len() > INCIDENT_BYTES,
        "the volume fixture must be at least as large as the payload that \
         caused #164 ({INCIDENT_BYTES} bytes), or it is not a volume test",
    );

    let parsed: CoveragePayload = payload::from_slice(
        "the #164 coverage payload",
        "CoverageReport",
        VOLUME,
        PayloadLimit::DEFAULT,
    )
    .expect("the payload reads");

    // The figures the TypeScript oracle reported once, in PROVENANCE.md.
    // Asserted against the engine's own struct here: if the Cargo edge read
    // this payload differently from the way ajv-plus-hand-written-interfaces
    // read it, these numbers move.
    assert_eq!(parsed.report.totals.backed, 1555);
    assert_eq!(parsed.report.totals.total, 2749);
    assert_eq!(parsed.report.totals.criteria, Some(1007));
    assert_eq!(parsed.report.totals.property_shaped, Some(553));
    assert_eq!(parsed.report.totals.specific_shaped, Some(79));
    assert_eq!(parsed.report.unbacked_rows.len(), 211);
    assert_eq!(parsed.report.obligations.len(), 1215);
    assert_eq!(parsed.report.criteria.len(), 139);

    // The provenance block survived the read rather than being dropped by the
    // flattened envelope — the field `quire_rs::CoverageReport` does not have
    // and every stored artifact does.
    let provenance = parsed
        .engine
        .as_ref()
        .expect("the payload carries provenance");
    assert_eq!(provenance.cli, "0.32.0");
    assert_eq!(
        provenance.engine,
        "a874fb641cb70da83c8c8b23f9fea0a44255b88a"
    );
}

/// Trace: NFR-025
///
/// Round-trip, in the sense that matters for an evidence store: re-encoding
/// the value and reading it again must yield the same value. NFR-025 makes
/// stored bytes immutable across the port, so a reader that quietly drops a
/// field would be unrecoverable — and a field dropped on the way in is
/// invisible in a single read.
#[test]
fn tc_379_101_the_incident_payload_round_trips() {
    let first: CoveragePayload = payload::from_slice(
        "the #164 coverage payload",
        "CoverageReport",
        VOLUME,
        PayloadLimit::DEFAULT,
    )
    .expect("the payload reads");

    let encoded = payload::encode("re-encoded coverage", &first, PayloadLimit::DEFAULT)
        .expect("the value encodes under the ceiling");
    let second: CoveragePayload = payload::from_slice(
        "re-encoded coverage",
        "CoverageReport",
        encoded.as_bytes(),
        PayloadLimit::DEFAULT,
    )
    .expect("the re-encoded payload reads");

    assert_eq!(first, second, "the value did not survive a round trip");

    // Re-encoding twice is byte-stable. Not asserted against the *original*
    // bytes: the engine omits empty collections, so a payload whose producer
    // emitted `"no_symbol_rows": []` would re-encode shorter while meaning the
    // same thing. Value equality above is the claim; this is determinism.
    let again =
        payload::encode("re-encoded coverage", &second, PayloadLimit::DEFAULT).expect("encodes");
    assert_eq!(encoded, again);
}

/// Trace: NFR-024
///
/// The bound is real, and it names itself. `src/quire/exec.ts` could only
/// report this as a process killed by `ENOBUFS`; here it is a refusal that
/// says which payload, how big, and against what ceiling.
#[test]
fn tc_379_102_a_ceiling_below_the_payload_refuses_it_by_name() {
    let limit = PayloadLimit::bytes(1_000_000).expect("non-zero");
    let error = payload::from_slice::<CoveragePayload>(
        "the #164 coverage payload",
        "CoverageReport",
        VOLUME,
        limit,
    )
    .expect_err("a payload over the ceiling must be refused");

    assert_eq!(error.code(), ErrorCode::PayloadTooLarge);
    let message = error.to_string();
    assert!(message.contains("1632821"), "{message}");
    assert!(message.contains("1000000"), "{message}");
    assert!(
        message.contains("the #164 coverage payload"),
        "the refusal must name what was being read: {message}",
    );
}

/// Trace: FR-097
///
/// The engine-identity finding this fixture exposed: a real payload's
/// `engine.engine` is a 40-character **object id**, not a version. Anything
/// reading it as a version reports every stored artifact as unknown
/// provenance.
#[test]
fn tc_379_103_stored_provenance_is_a_revision_and_is_treated_as_one() {
    let parsed: CoveragePayload = payload::from_slice(
        "the #164 coverage payload",
        "CoverageReport",
        VOLUME,
        PayloadLimit::DEFAULT,
    )
    .expect("the payload reads");
    let found = parsed.engine.as_ref().expect("provenance").engine.clone();

    assert_eq!(
        engine::identify(Some(&found)),
        engine::InstrumentId::Revision(found.clone()),
        "a 40-hex engine field is a revision, not an unparseable version",
    );

    // This build links a different object id, and that is reported as a
    // mismatch rather than as "older": revisions are not ordered.
    let error = engine::check_premise("the #164 coverage payload", Some(&found))
        .expect_err("a foreign engine revision must be reported");
    assert_eq!(error.code(), ErrorCode::EngineRevisionMismatch);

    // ...and this build's own revision passes.
    engine::check_premise("a payload this build wrote", Some(engine::ENGINE_REVISION))
        .expect("this build's own revision satisfies the premise");
}
