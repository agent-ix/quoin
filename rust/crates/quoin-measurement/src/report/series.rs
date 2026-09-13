// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One metric's history across every retained collection.
//!
//! Ports `seriesFor` (`report.ts:206-221`). The retained function returns
//! `unknown` and its only caller renders it as canonical JSON
//! (`src/commands/report.ts:100-136`), so the shape is stated here as a type
//! and [`render_series_json`] is the rendering that caller performs.

use serde::Serialize;

use crate::error::MeasurementError;
use crate::report::build::gap_count;
use crate::report::wire::{ObservationWire, canonical_json_of};
use crate::source::MeasurementSource;
use crate::store::read::read_measurement_collections;
use crate::types::collection::MeasurementCollection;
use crate::types::observation::MeasurementObservation;

/// One collection's reading of the metric, with the provenance it was read
/// under.
#[derive(Clone, Debug, PartialEq)]
pub struct SeriesPoint {
    /// When the collection was produced.
    pub timestamp: String,
    /// The revision of the measured source.
    pub source_revision: String,
    /// The producing tool's version.
    pub tool_version: String,
    /// The producing tool's identity.
    pub tool_identity: String,
    /// The digest of the configuration it ran under.
    pub config_digest: String,
    /// The measured corpus revision, when the collection states one.
    pub corpus_revision: Option<String>,
    /// The collection's stated corpus gap count, when it states one.
    pub corpus_gaps: Option<f64>,
    /// The observation itself.
    pub observation: MeasurementObservation,
}

/// Every observation of `metric`, oldest collection first.
///
/// # Errors
///
/// Whatever the collection read-back refuses.
pub fn series_for<S: MeasurementSource + ?Sized>(
    source: &S,
    metric: &str,
) -> Result<Vec<SeriesPoint>, MeasurementError> {
    Ok(read_measurement_collections(source)?
        .iter()
        .flat_map(|collection| points_of(collection, metric))
        .collect())
}

/// One collection's points.
fn points_of<'a>(
    collection: &'a MeasurementCollection,
    metric: &'a str,
) -> impl Iterator<Item = SeriesPoint> + 'a {
    let gaps = gap_count(collection);
    collection
        .observations
        .iter()
        .filter(move |observation| observation.metric.as_str() == metric)
        .map(move |observation| SeriesPoint {
            timestamp: collection.timestamp.as_str().to_owned(),
            source_revision: collection.source_revision.as_str().to_owned(),
            tool_version: collection.tool_version.as_str().to_owned(),
            tool_identity: collection.tool_identity.as_str().to_owned(),
            config_digest: collection.config_digest.as_str().to_owned(),
            corpus_revision: collection.corpus_revision.clone(),
            corpus_gaps: gaps,
            observation: observation.clone(),
        })
}

/// The series as canonical JSON, trailing newline included.
///
/// `src/commands/report.ts:100-136` renders it with `canonicalJson(…)` and then
/// trims the trailing newline for terminal output; the newline is kept here
/// because it is what `canonicalJson` states and trimming is the caller's.
///
/// # Errors
///
/// As [`crate::report::render_measurement_report_json`].
pub fn render_series_json(points: &[SeriesPoint]) -> Result<String, MeasurementError> {
    let wire: Vec<SeriesPointWire> = points
        .iter()
        .map(SeriesPointWire::of)
        .collect::<Result<Vec<_>, MeasurementError>>()?;
    canonical_json_of(&wire)
}

/// One point, as `report.ts:212-220` spells it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SeriesPointWire<'a> {
    timestamp: &'a str,
    source_revision: &'a str,
    tool_version: &'a str,
    tool_identity: &'a str,
    config_digest: &'a str,
    /// `?? null`: stated as `null`, not dropped.
    corpus_revision: Option<&'a str>,
    /// As `corpus_revision`.
    corpus_gaps: Option<f64>,
    observation: ObservationWire,
}

impl<'a> SeriesPointWire<'a> {
    /// The wire view of one point.
    fn of(point: &'a SeriesPoint) -> Result<Self, MeasurementError> {
        Ok(Self {
            timestamp: &point.timestamp,
            source_revision: &point.source_revision,
            tool_version: &point.tool_version,
            tool_identity: &point.tool_identity,
            config_digest: &point.config_digest,
            corpus_revision: point.corpus_revision.as_deref(),
            corpus_gaps: point.corpus_gaps,
            observation: ObservationWire::of(&point.observation)?,
        })
    }
}
