// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The six reporting functions agree with the TypeScript they were ported from.
//!
//! `renderMeasurementReport`, `renderMeasurementReportJson`,
//! `renderPortfolioReport`, `renderPortfolioReportJson`, `comparisonFor` and
//! `seriesFor` — the whole of the wave-6 gate (FR-100-AC-4), each compared
//! byte-for-byte against what the retained TypeScript produced over the
//! committed fixture tree under `tests/fixtures/portfolio-tree/`.
//!
//! # The oracle is a file, not a runtime
//!
//! `tests/fixtures/portfolio-oracle.json` was captured once, by
//! `oracle/capture-portfolio-oracle.mjs` (deleted at cutover, recoverable from
//! the revision the fixture records), at the revision the file records
//! (FR-101-AC-11). It is committed and frozen. Nothing here spawns node:
//! FR-101-AC-5 forbids a live TypeScript runtime as a test-time oracle, and the
//! suite has to keep running once the TypeScript is deleted at cutover.
//!
//! The fixture tree is not a copy of anything this crate produced — it is
//! authored markdown and canonical JSON, and its intervention record and
//! operational pair are byte-for-byte copies of retained artifacts under
//! `spec/evidence/`.
//!
//! # The one normalisation
//!
//! `root` and `path` are absolute, so the captured bytes would otherwise record
//! the checkout that produced them. Both sides replace the fixture tree's own
//! absolute root with `@@TREE@@` before comparing. Nothing else is edited. The
//! capture used node's `path.resolve`, which does not follow symlinks, so the
//! Rust side must not canonicalise either.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_measurement::portfolio::{
    build_portfolio_report, render_portfolio_report, render_portfolio_report_json,
};
use quoin_measurement::report::{
    build_measurement_report, comparison_for, render_measurement_comparison_json,
    render_measurement_report, render_measurement_report_json, render_series_json, series_for,
};
use quoin_measurement::source::DiskMeasurement;
use serde::Deserialize;

/// The committed capture.
const ORACLE: &str = include_str!("fixtures/portfolio-oracle.json");

/// The anti-vacuity floor for each case list. A capture that lost its cases
/// would otherwise pass every comparison below by comparing nothing.
const REPORT_CASE_FLOOR: usize = 5;
const COMPARISON_CASE_FLOOR: usize = 4;
const SERIES_CASE_FLOOR: usize = 4;
/// The portfolio walk's floor is its location list: the report is one value,
/// so the thing that can silently shrink is what it was built over.
const PORTFOLIO_LOCATION_FLOOR: usize = 8;

#[derive(Deserialize)]
struct Capture {
    produced_from_revision: String,
    tree_token: String,
    reports: Vec<ReportCase>,
    comparisons: Vec<ComparisonCase>,
    series: Vec<SeriesCase>,
    portfolio: PortfolioCase,
}

#[derive(Deserialize)]
struct ReportCase {
    name: String,
    rendered: String,
    json: String,
}

#[derive(Deserialize)]
struct ComparisonCase {
    name: String,
    repository: String,
    before: String,
    json: String,
}

#[derive(Deserialize)]
struct SeriesCase {
    name: String,
    repository: String,
    metric: String,
    json: String,
}

#[derive(Deserialize)]
struct PortfolioCase {
    locations: Vec<String>,
    rendered: String,
    json: String,
}

fn capture() -> Capture {
    serde_json::from_str(ORACLE).expect("the committed oracle capture parses")
}

/// The fixture tree's absolute root in this checkout.
///
/// Deliberately unresolved: `path.resolve` absolutises and normalises
/// lexically, and `CARGO_MANIFEST_DIR` is already absolute, so the two spell the
/// same path. `canonicalize` would follow symlinks and could not.
fn tree() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("portfolio-tree")
}

/// Replaces this checkout's tree root with the token the capture recorded.
fn normalise(capture: &Capture, text: &str) -> String {
    text.replace(
        tree().to_str().expect("the fixture path is UTF-8"),
        &capture.tree_token,
    )
}

/// Restores a captured location to a path in this checkout.
fn denormalise(capture: &Capture, location: &str) -> PathBuf {
    PathBuf::from(location.replace(
        &capture.tree_token,
        tree().to_str().expect("the fixture path is UTF-8"),
    ))
}

/// PLAT-968 appended an advisory "Priority ranking" section to the rendered
/// portfolio, and a `ranking` member to its JSON, with no counterpart in the
/// frozen TypeScript oracle: `portfolio.ts` never ranked anything, and
/// FR-101-AC-5 forbids re-capturing this oracle from a live TypeScript
/// runtime to add one. This strips exactly that appended, Rust-only content
/// before the byte comparison — the one place this suite deliberately does
/// not assert parity with an oracle that cannot state an opinion about
/// something it predates.
fn without_ranking_section(rendered: &str) -> String {
    // The ranking section is always preceded by exactly one blank line, which
    // `split_once` consumes along with the header; put back the single
    // trailing newline every prior repository section already ends its own
    // lines with, so the prefix reads as though `render_portfolio_report` had
    // never called `ranking_section` at all.
    let (before, section) = rendered
        .split_once("\n\n## Priority ranking\n")
        .expect("the ranking section is rendered unconditionally");
    assert_only_ranking_lines(section);
    format!("{before}\n")
}

/// Fail unless every line of `section` (the ranking section after its
/// header) is one `ranking_section` writes: blank, the advisory note, a
/// table row, the "Not ranked:" label or an unranked bullet. This is what
/// keeps the strip from hiding any other drift appended after the header.
fn assert_only_ranking_lines(section: &str) {
    use quoin_measurement::portfolio::RANKING_ADVISORY_NOTE;
    for line in section.lines() {
        assert!(
            line.is_empty()
                || line == RANKING_ADVISORY_NOTE
                || line == "Not ranked:"
                || line.starts_with("| ")
                || line.starts_with("- "),
            "unexpected line inside the stripped ranking section: {line:?}"
        );
    }
}

/// As [`without_ranking_section`], for the JSON form's `ranking` member.
///
/// Round-trips through `quoin_store`'s own strict reader and canonical
/// pretty-printer (not `serde_json::to_string`) so the surrounding bytes stay
/// exactly what [`render_portfolio_report_json`] would have written without
/// the member — same two-space indent, same ECMAScript own-property key
/// order, same trailing newline.
fn without_ranking_member(json: &str) -> String {
    let mut value = quoin_store::parse_strict_json_str(json).expect("the rendered JSON parses");
    let quoin_store::JsonValue::Object(ref mut object) = value else {
        panic!("the portfolio JSON is an object");
    };
    assert!(
        object.remove("ranking").is_some(),
        "the ranking member is rendered unconditionally"
    );
    quoin_store::canonical_json(&value).expect("the trimmed value re-serializes")
}

/// Trace: FR-101-AC-11
/// Provenance: quoin#473
#[test]
fn tc_473_the_capture_names_the_revision_it_was_taken_at() {
    let capture = capture();
    assert_eq!(
        capture.produced_from_revision.len(),
        40,
        "the capture must name the revision of the TypeScript it ran, so the \
         fixture is evidence rather than a file someone once generated"
    );
    assert!(
        capture.reports.len() >= REPORT_CASE_FLOOR,
        "anti-vacuity floor: at least {REPORT_CASE_FLOOR} report cases expected, saw {}",
        capture.reports.len()
    );
    assert!(
        capture.comparisons.len() >= COMPARISON_CASE_FLOOR,
        "anti-vacuity floor: at least {COMPARISON_CASE_FLOOR} comparison cases expected, saw {}",
        capture.comparisons.len()
    );
    assert!(
        capture.series.len() >= SERIES_CASE_FLOOR,
        "anti-vacuity floor: at least {SERIES_CASE_FLOOR} series cases expected, saw {}",
        capture.series.len()
    );
    assert!(
        capture.portfolio.locations.len() >= PORTFOLIO_LOCATION_FLOOR,
        "anti-vacuity floor: at least {PORTFOLIO_LOCATION_FLOOR} portfolio locations expected, \
         saw {}",
        capture.portfolio.locations.len()
    );
}

/// Trace: FR-100-AC-4
/// Provenance: quoin#473
#[test]
fn tc_473_render_measurement_report_is_byte_identical() {
    let capture = capture();
    for case in &capture.reports {
        let repo = tree().join(&case.name);
        let report =
            build_measurement_report(&DiskMeasurement::new(&repo), &repo).unwrap_or_else(|error| {
                panic!("case {}: the report does not build: {error}", case.name)
            });
        assert_eq!(
            normalise(&capture, &render_measurement_report(&report).unwrap()),
            case.rendered,
            "case {}: the rendered report is not what the TypeScript wrote",
            case.name
        );
    }
}

/// Trace: FR-100-AC-4
/// Provenance: quoin#473
#[test]
fn tc_473_render_measurement_report_json_is_byte_identical() {
    let capture = capture();
    for case in &capture.reports {
        let repo = tree().join(&case.name);
        let report =
            build_measurement_report(&DiskMeasurement::new(&repo), &repo).unwrap_or_else(|error| {
                panic!("case {}: the report does not build: {error}", case.name)
            });
        assert_eq!(
            normalise(&capture, &render_measurement_report_json(&report).unwrap()),
            case.json,
            "case {}: the report JSON is not what the TypeScript wrote",
            case.name
        );
    }
}

/// Trace: FR-100-AC-4
/// Provenance: quoin#473
#[test]
fn tc_473_comparison_for_is_byte_identical() {
    let capture = capture();
    for case in &capture.comparisons {
        let repo = tree().join(&case.repository);
        let report = comparison_for(&DiskMeasurement::new(&repo), &repo, &case.before)
            .unwrap_or_else(|error| panic!("case {}: the comparison refused: {error}", case.name));
        assert_eq!(
            normalise(
                &capture,
                &render_measurement_comparison_json(&report).unwrap()
            ),
            case.json,
            "case {}: the comparison is not what the TypeScript computed",
            case.name
        );
    }
}

/// Trace: FR-100-AC-4
/// Provenance: quoin#473
#[test]
fn tc_473_series_for_is_byte_identical() {
    let capture = capture();
    for case in &capture.series {
        let repo = tree().join(&case.repository);
        let points = series_for(&DiskMeasurement::new(&repo), &case.metric)
            .unwrap_or_else(|error| panic!("case {}: the series refused: {error}", case.name));
        assert_eq!(
            normalise(&capture, &render_series_json(&points).unwrap()),
            case.json,
            "case {}: the series is not what the TypeScript computed",
            case.name
        );
    }
}

/// Trace: FR-100-AC-4
/// Provenance: quoin#473
#[test]
fn tc_473_render_portfolio_report_is_byte_identical() {
    let capture = capture();
    let locations: Vec<PathBuf> = capture
        .portfolio
        .locations
        .iter()
        .map(|location| denormalise(&capture, location))
        .collect();
    let report = build_portfolio_report(&locations);
    let rendered = normalise(&capture, &render_portfolio_report(&report).unwrap());
    assert_eq!(
        without_ranking_section(&rendered),
        capture.portfolio.rendered,
        "the rendered portfolio is not what the TypeScript wrote, once PLAT-968's \
         Rust-only ranking section is set aside"
    );
    let json = normalise(&capture, &render_portfolio_report_json(&report).unwrap());
    assert_eq!(
        without_ranking_member(&json),
        capture.portfolio.json,
        "the portfolio JSON is not what the TypeScript wrote, once PLAT-968's \
         Rust-only ranking member is set aside"
    );
}
