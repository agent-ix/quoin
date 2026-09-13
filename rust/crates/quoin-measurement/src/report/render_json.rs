// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The measurement report as canonical JSON.
//!
//! `renderMeasurementReportJson` (`report.ts:202-204`) is one expression —
//! `canonicalJson(report)` — over an object whose exact members are
//! [`crate::report::wire`]'s subject.

use crate::error::MeasurementError;
use crate::report::build::MeasurementReport;
use crate::report::wire::{MeasurementReportWire, canonical_json_of};

/// Render the report as the store's canonical JSON, trailing newline included.
///
/// # Errors
///
/// [`crate::MeasurementErrorCode::Store`] when a stored dimension or population
/// value cannot cross [`crate::json_bridge`].
pub fn render_measurement_report_json(
    report: &MeasurementReport,
) -> Result<String, MeasurementError> {
    canonical_json_of(&MeasurementReportWire::of(report)?)
}
