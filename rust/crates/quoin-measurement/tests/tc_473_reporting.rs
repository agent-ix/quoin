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
    assert_eq!(
        normalise(&capture, &render_portfolio_report(&report).unwrap()),
        capture.portfolio.rendered,
        "the rendered portfolio is not what the TypeScript wrote"
    );
    assert_eq!(
        normalise(&capture, &render_portfolio_report_json(&report).unwrap()),
        capture.portfolio.json,
        "the portfolio JSON is not what the TypeScript wrote"
    );
}
