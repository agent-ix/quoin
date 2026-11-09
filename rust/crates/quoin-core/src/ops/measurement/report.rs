// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The single-store reports: the plan report and one metric's
//! series.
//!
//! Five routes over one store. Each `build_*` answers with the canonical JSON
//! document its crate's `render_json` defines; each `render_*` answers with the
//! markdown its crate's `render` defines. Both halves read the store, for the
//! reason [`super`]'s header states.

use std::path::PathBuf;

use quoin_measurement::{
    DiskMeasurement, build_measurement_report, render_measurement_report,
    render_measurement_report_json, render_series_json, series_for,
};

use crate::error::CoreError;
use crate::protocol::Response;

use super::taxonomy::map_measurement;
use super::wire::{
    MAX_ASSURANCE_DOCUMENT_BYTES, MAX_METRIC_NAME_BYTES, RepoRequest, SeriesRequest,
};
use super::{bound, bound_field, document, parse, rendered, string_len};

/// Answer a `measurement.build_report`.
///
/// # Errors
///
/// - [`crate::error::CoreErrorCode::BadRequest`] when stdin is not a
///   [`RepoRequest`].
/// - [`crate::error::CoreErrorCode::Refused`] when the ceiling is exceeded, or
///   when the store cannot be read.
pub fn build_report(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "measurement.build_report";
    let repo = locate(request, OP)?;
    let report = build_measurement_report(&DiskMeasurement::new(&repo), &repo)
        .map_err(|e| map_measurement(&e, OP))?;
    let canonical = render_measurement_report_json(&report).map_err(|e| map_measurement(&e, OP))?;
    document(&canonical, OP)
}

/// Answer a `measurement.render_report`.
///
/// # Errors
///
/// As [`build_report`].
pub fn render_report(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "measurement.render_report";
    let repo = locate(request, OP)?;
    let report = build_measurement_report(&DiskMeasurement::new(&repo), &repo)
        .map_err(|e| map_measurement(&e, OP))?;
    let markdown = render_measurement_report(&report).map_err(|e| map_measurement(&e, OP))?;
    rendered(markdown, OP)
}

/// Answer a `measurement.build_series`.
///
/// There is no `measurement.render_series` beside it, and that is the ticket's
/// list rather than an omission: the retained `report.ts` renders a series with
/// a local helper over the same rows this route returns, and porting that
/// helper is #479's cutover work, not this wave's wiring.
///
/// # Errors
///
/// - [`crate::error::CoreErrorCode::BadRequest`] when stdin is not a
///   [`SeriesRequest`].
/// - [`crate::error::CoreErrorCode::Refused`] when a ceiling is exceeded, or
///   when the store cannot be read.
pub fn build_series(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "measurement.build_series";
    bound(request, OP, MAX_ASSURANCE_DOCUMENT_BYTES)?;
    bound_field(
        OP,
        "metric",
        string_len(request, "metric"),
        MAX_METRIC_NAME_BYTES,
    )?;

    let request: SeriesRequest = parse(request, OP)?;
    let points = series_for(&DiskMeasurement::new(&request.repo), &request.metric)
        .map_err(|e| map_measurement(&e, OP))?;
    let canonical = render_series_json(&points).map_err(|e| map_measurement(&e, OP))?;
    document(&canonical, OP)
}

/// Bound and parse a request that names one store, and nothing else.
fn locate(request: &serde_json::Value, op: &'static str) -> Result<PathBuf, CoreError> {
    bound(request, op, MAX_ASSURANCE_DOCUMENT_BYTES)?;
    let request: RepoRequest = parse(request, op)?;
    Ok(PathBuf::from(request.repo))
}
