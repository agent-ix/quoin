// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Two collections of one repository, compared.
//!
//! Ports `comparisonFor`, `renderMeasurementComparison`, `collectionReference`
//! and `renderReference` (`report.ts:223-330,332-360`).
//!
//! # Why the status carries the reason
//!
//! The retained report is `{ status, reason: string | null, … }` with the
//! invariant that a `compared` report has no reason and a `not_computed` one
//! always has one — `report.ts:308` branches on `report.reason` alone and would
//! print a table for a `not_computed` report that forgot its reason. Here the
//! two travel together in [`MeasurementComparisonStatus`], so there is no value
//! of this type that spells the state `report.ts:308` mishandles.

use std::path::Path;

use serde::Serialize;

use crate::common::scalar::js_f64_string;
use crate::error::MeasurementError;
use crate::report::build::gap_count;
use crate::report::render::metric_label;
use crate::report::wire::{ComparisonWire, canonical_json_of};
use crate::source::MeasurementSource;
use crate::store::paths::measurement_path;
use crate::store::read::read_measurement_collections;
use crate::types::collection::MeasurementCollection;
use crate::types::comparison::MeasurementComparison;
use crate::types::ids::CollectionId;

/// A collection, as the comparison report quotes it. `report.ts:243-253`.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementCollectionReference {
    /// The collection's identity.
    pub collection_id: String,
    /// When it was produced.
    pub timestamp: String,
    /// Which tool produced it.
    pub tool_identity: String,
    /// Which version of that tool.
    pub tool_version: String,
    /// The digest of the configuration it ran under.
    pub config_digest: String,
    /// The revision of the measured source.
    pub source_revision: String,
    /// The measured corpus revision, when the collection states one.
    pub corpus_revision: Option<String>,
    /// The stated corpus gap count, when the collection states one.
    pub corpus_gaps: Option<f64>,
    /// Where the record lives.
    pub path: String,
}

impl MeasurementCollectionReference {
    /// `collectionReference` (`report.ts:332-347`).
    fn of(repo: &Path, collection: &MeasurementCollection) -> Result<Self, MeasurementError> {
        let id = CollectionId::parse(collection.collection_id.as_str())?;
        Ok(Self {
            collection_id: collection.collection_id.as_str().to_owned(),
            timestamp: collection.timestamp.as_str().to_owned(),
            tool_identity: collection.tool_identity.as_str().to_owned(),
            tool_version: collection.tool_version.as_str().to_owned(),
            config_digest: collection.config_digest.as_str().to_owned(),
            source_revision: collection.source_revision.as_str().to_owned(),
            corpus_revision: collection.corpus_revision.clone(),
            corpus_gaps: gap_count(collection),
            path: measurement_path(repo, &id).to_string_lossy().into_owned(),
        })
    }

    /// `renderReference` (`report.ts:349-360`).
    fn rendered(&self) -> String {
        format!(
            "{} — {} {}; source {}; corpus {}; gaps {}; config {}; record {}",
            self.collection_id,
            self.tool_identity,
            self.tool_version,
            self.source_revision,
            self.corpus_revision.as_deref().unwrap_or("n/a"),
            self.corpus_gaps
                .map_or_else(|| "not_computed".to_owned(), js_f64_string),
            self.config_digest,
            self.path
        )
    }
}

/// Whether the two collections were compared, and why not when they were not.
///
/// `report.ts:255-256`, with the report's `reason` carried by the variant that
/// always has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeasurementComparisonStatus {
    /// Both collections were found and compared.
    Compared,
    /// They were not, for this stated reason.
    NotComputed(String),
}

impl MeasurementComparisonStatus {
    /// The stable wire spelling.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match *self {
            Self::Compared => "compared",
            Self::NotComputed(_) => "not_computed",
        }
    }

    /// The reason, which only a `not_computed` status has.
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        match *self {
            Self::Compared => None,
            Self::NotComputed(ref reason) => Some(reason),
        }
    }
}

/// One repository's before-and-after comparison. `report.ts:255-262`.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementComparisonReport {
    /// Whether the comparison happened, and why not when it did not.
    pub status: MeasurementComparisonStatus,
    /// The baseline collection, when one was found.
    pub before: Option<MeasurementCollectionReference>,
    /// The current collection, when one exists.
    pub after: Option<MeasurementCollectionReference>,
    /// The compared slices; empty unless the status is `compared`.
    pub comparisons: Vec<MeasurementComparison>,
}

/// Compare the newest collection against the newest one whose source revision
/// starts with `before_revision`.
///
/// `comparisonFor` (`report.ts:264-306`).
///
/// # Errors
///
/// Whatever the collection read-back refuses, and
/// [`crate::MeasurementErrorCode::CollectionIdUnsafe`] for a retained
/// collection whose id does not name a file.
pub fn comparison_for<S: MeasurementSource + ?Sized>(
    source: &S,
    repo: &Path,
    before_revision: &str,
) -> Result<MeasurementComparisonReport, MeasurementError> {
    let collections = read_measurement_collections(source)?;
    let Some(after) = collections.last() else {
        return Ok(MeasurementComparisonReport {
            status: MeasurementComparisonStatus::NotComputed(
                "no current measurement collection is recorded".to_owned(),
            ),
            before: None,
            after: None,
            comparisons: Vec::new(),
        });
    };
    let after_reference = MeasurementCollectionReference::of(repo, after)?;
    let Some(before) = collections.iter().rev().find(|candidate| {
        candidate
            .source_revision
            .as_str()
            .starts_with(before_revision)
    }) else {
        return Ok(MeasurementComparisonReport {
            status: MeasurementComparisonStatus::NotComputed(format!(
                "no baseline measurement collection for source revision {before_revision}"
            )),
            before: None,
            after: Some(after_reference),
            comparisons: Vec::new(),
        });
    };
    Ok(MeasurementComparisonReport {
        status: MeasurementComparisonStatus::Compared,
        before: Some(MeasurementCollectionReference::of(repo, before)?),
        after: Some(after_reference),
        comparisons: crate::compare::compare_measurement_collections(before, after)?,
    })
}

/// Render the comparison as markdown.
///
/// `renderMeasurementComparison` (`report.ts:308-330`).
///
/// # Errors
///
/// As [`crate::report::render_measurement_report`].
pub fn render_measurement_comparison(
    report: &MeasurementComparisonReport,
) -> Result<String, MeasurementError> {
    let mut lines = vec![
        "# QA measurement comparison".to_owned(),
        String::new(),
        format!("Status: {}", report.status.as_str()),
        String::new(),
    ];
    lines.push(report.before.as_ref().map_or_else(
        || "Before: not_computed — no baseline".to_owned(),
        |reference| format!("Before: {}", reference.rendered()),
    ));
    lines.push(report.after.as_ref().map_or_else(
        || "After: not_computed — no current record".to_owned(),
        |reference| format!("After: {}", reference.rendered()),
    ));
    lines.push(String::new());
    if let Some(reason) = report.status.reason() {
        lines.push(format!("not_computed: {reason}"));
        lines.push(String::new());
        return Ok(lines.join("\n"));
    }
    lines.push("| Metric | Status | Before | After | Delta |".to_owned());
    lines.push("| --- | --- | ---: | ---: | ---: |".to_owned());
    for comparison in &report.comparisons {
        lines.push(format!(
            "| {} | {} | {} | {} | {} |",
            metric_label(&comparison.metric, &comparison.dimensions)?,
            comparison.status.as_str(),
            cell(comparison.before),
            cell(comparison.after),
            cell(comparison.delta)
        ));
        for reason in &comparison.reasons {
            lines.push(format!(
                "| ↳ {} | {} |  |  | {} |",
                reason.code.as_str(),
                if reason.blocking {
                    "blocks delta"
                } else {
                    "context"
                },
                reason.message
            ));
        }
    }
    lines.push(String::new());
    Ok(lines.join("\n"))
}

/// `${value ?? "n/a"}`.
fn cell(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_owned(), js_f64_string)
}

/// The comparison report as canonical JSON, trailing newline included.
///
/// `src/commands/report.ts:100-136` is the caller that renders it.
///
/// # Errors
///
/// As [`crate::report::render_measurement_report_json`].
pub fn render_measurement_comparison_json(
    report: &MeasurementComparisonReport,
) -> Result<String, MeasurementError> {
    canonical_json_of(&ComparisonReportWire {
        status: report.status.as_str(),
        reason: report.status.reason(),
        before: report.before.as_ref().map(ReferenceWire::of),
        after: report.after.as_ref().map(ReferenceWire::of),
        comparisons: ComparisonWire::all(&report.comparisons)?,
    })
}

/// `report.ts:255-262`'s object.
#[derive(Debug, Serialize)]
struct ComparisonReportWire<'a> {
    status: &'static str,
    /// `| null`: stated as `null` on a compared report.
    reason: Option<&'a str>,
    /// As `reason`.
    before: Option<ReferenceWire<'a>>,
    /// As `reason`.
    after: Option<ReferenceWire<'a>>,
    comparisons: Vec<ComparisonWire<'a>>,
}

/// `report.ts:243-253`'s object.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReferenceWire<'a> {
    collection_id: &'a str,
    timestamp: &'a str,
    tool_identity: &'a str,
    tool_version: &'a str,
    config_digest: &'a str,
    source_revision: &'a str,
    /// `?? null`: stated as `null`, not dropped.
    corpus_revision: Option<&'a str>,
    /// As `corpus_revision`.
    corpus_gaps: Option<f64>,
    path: &'a str,
}

impl<'a> ReferenceWire<'a> {
    /// The wire view of one reference.
    fn of(reference: &'a MeasurementCollectionReference) -> Self {
        Self {
            collection_id: &reference.collection_id,
            timestamp: &reference.timestamp,
            tool_identity: &reference.tool_identity,
            tool_version: &reference.tool_version,
            config_digest: &reference.config_digest,
            source_revision: &reference.source_revision,
            corpus_revision: reference.corpus_revision.as_deref(),
            corpus_gaps: reference.corpus_gaps,
            path: &reference.path,
        }
    }
}
