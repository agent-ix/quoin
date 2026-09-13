// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The two record renderers agree with the TypeScript they were ported from.
//!
//! # The oracle is a file, not a runtime
//!
//! `tests/fixtures/report-oracle.json` was captured once, by
//! `oracle/capture-report-oracle.mjs`, running the real
//! `buildInterventionReport` / `renderInterventionReport` and
//! `buildOperationalReport` / `renderOperationalReport` at the revision the file
//! records (FR-101-AC-11). It is committed and frozen. Nothing here spawns
//! node: a live TypeScript runtime as a test-time oracle makes the gate a claim
//! about whatever is installed that morning, and makes the suite unrunnable
//! once the TypeScript is deleted at cutover.
//!
//! # What each case proves
//!
//! Three assertions per case, and they are not the same assertion:
//!
//! 1. the records deserialize — the Rust types accept the wire shape the
//!    TypeScript wrote;
//! 2. the projection matches — `build*Report`'s entries, compared as JSON, so
//!    ordering, list sorting and the gap/claim/counterevidence routing are all
//!    under the comparison;
//! 3. the rendered text is byte-identical.
//!
//! A fourth assertion covers the records themselves: re-serializing a
//! deserialized record reproduces the JSON it came from, so the type model
//! drops no field on the way through.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_measurement::intervention::record::InterventionExperimentRecord;
use quoin_measurement::intervention::report::{
    build_intervention_report, render_intervention_report,
};
use quoin_measurement::operational::record::OperationalEvidenceRecord;
use quoin_measurement::operational::report::{build_operational_report, render_operational_report};
use serde::Deserialize;

/// The committed capture.
const ORACLE: &str = include_str!("fixtures/report-oracle.json");

/// The anti-vacuity floor: a capture that lost its cases would otherwise pass.
const INTERVENTION_CASE_FLOOR: usize = 4;
/// The same, for the operational half, which has the larger verdict table.
const OPERATIONAL_CASE_FLOOR: usize = 11;

#[derive(Deserialize)]
struct Capture {
    produced_from_revision: String,
    intervention: Vec<Case>,
    operational: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    records: Vec<serde_json::Value>,
    entries: serde_json::Value,
    rendered: String,
}

fn capture() -> Capture {
    serde_json::from_str(ORACLE).expect("the committed oracle capture parses")
}

/// Deserializes a case's records and asserts each one re-serializes unchanged.
fn records<R>(case: &Case) -> Vec<R>
where
    R: serde::de::DeserializeOwned + serde::Serialize,
{
    case.records
        .iter()
        .map(|written| {
            let record: R = serde_json::from_value(written.clone()).unwrap_or_else(|error| {
                panic!(
                    "case {}: the record does not deserialize: {error}",
                    case.name
                )
            });
            let round_tripped = serde_json::to_value(&record).unwrap();
            assert_eq!(
                &round_tripped, written,
                "case {}: re-serializing the record did not reproduce it",
                case.name
            );
            record
        })
        .collect()
}

/// Trace: FR-100-AC-4
/// Provenance: quoin#469, quoin#373
#[test]
fn tc_469_the_capture_names_the_revision_it_was_taken_at() {
    let capture = capture();
    assert_eq!(
        capture.produced_from_revision.len(),
        40,
        "the capture must name the revision of the TypeScript it ran, so the \
         fixture is evidence rather than a file someone once generated"
    );
    assert!(
        capture.intervention.len() >= INTERVENTION_CASE_FLOOR,
        "anti-vacuity floor: at least {INTERVENTION_CASE_FLOOR} intervention cases expected, saw {}",
        capture.intervention.len()
    );
    assert!(
        capture.operational.len() >= OPERATIONAL_CASE_FLOOR,
        "anti-vacuity floor: at least {OPERATIONAL_CASE_FLOOR} operational cases expected, saw {}",
        capture.operational.len()
    );
}

/// Trace: FR-100-AC-4
/// Provenance: quoin#469
#[test]
fn tc_469_the_intervention_report_matches_the_typescript_oracle() {
    for case in &capture().intervention {
        let entries = build_intervention_report(&records::<InterventionExperimentRecord>(case));
        assert_eq!(
            serde_json::to_value(&entries).unwrap(),
            case.entries,
            "case {}: buildInterventionReport disagrees",
            case.name
        );
        assert_eq!(
            render_intervention_report(&entries),
            case.rendered,
            "case {}: renderInterventionReport disagrees",
            case.name
        );
    }
}

/// Trace: FR-100-AC-4
/// Provenance: quoin#469
#[test]
fn tc_469_the_operational_report_matches_the_typescript_oracle() {
    for case in &capture().operational {
        let entries = build_operational_report(&records::<OperationalEvidenceRecord>(case));
        assert_eq!(
            serde_json::to_value(&entries).unwrap(),
            case.entries,
            "case {}: buildOperationalReport disagrees",
            case.name
        );
        assert_eq!(
            render_operational_report(&entries),
            case.rendered,
            "case {}: renderOperationalReport disagrees",
            case.name
        );
    }
}

/// An empty report is the empty string, not a heading with nothing under it.
///
/// Trace: FR-100-AC-4
/// Provenance: quoin#469
#[test]
fn tc_469_an_empty_report_renders_as_nothing_at_all() {
    assert_eq!(render_intervention_report(&[]), "");
    assert_eq!(render_operational_report(&[]), "");
    let capture = capture();
    let intervention_empty = capture
        .intervention
        .iter()
        .find(|case| case.name == "empty")
        .expect("the capture carries an empty intervention case");
    assert_eq!(intervention_empty.rendered, "");
    let operational_empty = capture
        .operational
        .iter()
        .find(|case| case.name == "empty")
        .expect("the capture carries an empty operational case");
    assert_eq!(operational_empty.rendered, "");
}
