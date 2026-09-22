// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The projection every measurement view is rendered from.
//!
//! Ports `MeasurementReport`, `buildMeasurementReport` and
//! `buildMeasurementReportFrom` (`report.ts:24-103`), plus the two gap readers
//! at `report.ts:370-384`.
//!
//! # Two entry points, and only one of them reads
//!
//! `buildMeasurementReportFrom` exists in the retained code so the portfolio
//! view can load a repository's store once and derive both the human and the
//! JSON report from it. Here that split is sharper: [`build_measurement_report_from`]
//! takes every input as a value and touches nothing, and
//! [`build_measurement_report`] is the one that reads. A caller that already
//! holds the records — which is every caller inside
//! [`crate::portfolio`] — never goes near a filesystem.

use std::path::Path;

use quoin_store::JsonValue;

use crate::error::MeasurementError;
use crate::intervention::intake::read_intervention_records;
use crate::intervention::record::InterventionExperimentRecord;
use crate::intervention::report::{InterventionReportEntry, build_intervention_report};
use crate::operational::read::read_operational_records;
use crate::operational::record::OperationalEvidenceRecord;
use crate::operational::report::{OperationalReportEntry, build_operational_report};
use crate::plans::{PlanLoadOptions, load_measurement_plans};
use crate::source::MeasurementSource;
use crate::store::paths::measurement_path;
use crate::store::read::read_measurement_collections;
use crate::types::collection::MeasurementCollection;
use crate::types::ids::CollectionId;
use crate::types::observation::MeasurementObservation;
use crate::types::plan::{GroundTruthKind, LifecycleStatus, MeasurementPlan, MeasurementStage};

/// The collection members a report row quotes, and where the record lives.
///
/// `report.ts:34-43`'s `Pick<…> & { path: string }`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionSummary {
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
    /// The revision of the measured corpus, when the collection states one.
    pub corpus_revision: Option<String>,
    /// The collection's path, as [`measurement_path`] spells it.
    pub path: String,
    /// `verificationStack.artifacts` names the collection states with no
    /// local filesystem entry, sorted; empty when the collection states none
    /// or carries no verification stack at all (PLAT-969's ruling — a label
    /// is admitted, and the report says so next to the collection's
    /// provenance rather than staying silent about it).
    pub unverified_artifacts: Vec<String>,
}

/// One row of the "what this repository measures" table.
///
/// `report.ts:26-44`. A plan with no computed observation still gets a row,
/// carrying `None` — that absence is the report's most important output.
#[derive(Clone, Debug, PartialEq)]
pub struct CurrentRow {
    /// The metric the governing plan names.
    pub metric: String,
    /// The governing plan's identity.
    pub plan_id: String,
    /// Where the governing plan is authored.
    pub plan_path: String,
    /// The definition version the governing plan requires.
    pub plan_definition_version: String,
    /// How far along the measurement ladder the plan sits.
    pub stage: MeasurementStage,
    /// How the governing plan's ground truth was produced, when it says
    /// (PLAT-960).
    pub plan_ground_truth_kind: Option<GroundTruthKind>,
    /// The observation, when one was computed.
    pub observation: Option<MeasurementObservation>,
    /// The collection it was computed in, when there is one.
    pub collection: Option<CollectionSummary>,
}

/// Everything the per-repository views render.
///
/// `report.ts:24-46`.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementReport {
    /// The active plans, in load order.
    pub plans: Vec<MeasurementPlan>,
    /// One row per active plan and computed observation.
    pub current: Vec<CurrentRow>,
    /// The newest stated `bounds.gap_count`, when any collection states one.
    pub corpus_gaps: Option<f64>,
    /// The intervention experiments, projected.
    pub interventions: Vec<InterventionReportEntry>,
    /// The operational evidence, projected.
    pub operational: Vec<OperationalReportEntry>,
}

/// Read a repository and build its report.
///
/// `buildMeasurementReport` (`report.ts:48-55`). `repo` is the repository root,
/// used for the record paths the report quotes; `source` is where the bytes
/// come from, so a caller with no filesystem can still build a report.
///
/// # Errors
///
/// Whatever the plan walk, the collection read-back, the intervention read or
/// the operational read refuses, and
/// [`crate::MeasurementErrorCode::CollectionIdUnsafe`] for a retained
/// collection whose id does not name a file.
pub fn build_measurement_report<S: MeasurementSource + ?Sized>(
    source: &S,
    repo: &Path,
) -> Result<MeasurementReport, MeasurementError> {
    let plans = load_measurement_plans(source, PlanLoadOptions::default())?;
    let collections = read_measurement_collections(source)?;
    let interventions = read_intervention_records(source)?;
    let operational: Vec<OperationalEvidenceRecord> = read_operational_records(repo)?
        .into_iter()
        .map(|valid| valid.record().clone())
        .collect();
    build_measurement_report_from(repo, &plans, &collections, &interventions, &operational)
}

/// Build every view from one already-loaded snapshot.
///
/// `buildMeasurementReportFrom` (`report.ts:57-103`). Pure: the records the
/// retained function reads for itself are arguments here, because the portfolio
/// view already holds them and reading them twice is how a report and its
/// summary end up disagreeing.
///
/// # Errors
///
/// [`crate::MeasurementErrorCode::CollectionIdUnsafe`] for a retained
/// collection whose id does not name a file — `measurementPath` re-checks the
/// same grammar on every call (`store.ts:26`).
pub fn build_measurement_report_from(
    repo: &Path,
    all_plans: &[MeasurementPlan],
    collections: &[MeasurementCollection],
    interventions: &[InterventionExperimentRecord],
    operational: &[OperationalEvidenceRecord],
) -> Result<MeasurementReport, MeasurementError> {
    let plans: Vec<MeasurementPlan> = all_plans
        .iter()
        .filter(|plan| plan.status == LifecycleStatus::Active)
        .cloned()
        .collect();

    let mut current = Vec::new();
    for plan in &plans {
        // `[...collections].reverse().find(…)` — the newest collection that
        // computed this metric at all, not the newest collection.
        let collection = collections
            .iter()
            .rev()
            .find(|candidate| observes(candidate, plan.metric.as_str()));
        let summary = collection
            .map(|found| summary_of(repo, found))
            .transpose()?;
        let observations: Vec<&MeasurementObservation> = collection
            .map(|found| {
                found
                    .observations
                    .iter()
                    .filter(|observation| observation.metric.as_str() == plan.metric.as_str())
                    .collect()
            })
            .unwrap_or_default();
        // `observations.length > 0 ? observations : [null]`: a plan with no
        // observation is still a row.
        let row = |observation: Option<&MeasurementObservation>| CurrentRow {
            metric: plan.metric.as_str().to_owned(),
            plan_id: plan.id.as_str().to_owned(),
            plan_path: plan.path.clone(),
            plan_definition_version: plan.definition_version.as_str().to_owned(),
            stage: plan.stage,
            plan_ground_truth_kind: plan.ground_truth_kind,
            observation: observation.cloned(),
            collection: summary.clone(),
        };
        if observations.is_empty() {
            current.push(row(None));
        } else {
            current.extend(observations.into_iter().map(|found| row(Some(found))));
        }
    }

    Ok(MeasurementReport {
        plans,
        current,
        corpus_gaps: latest_gap_count(collections),
        interventions: build_intervention_report(interventions),
        operational: build_operational_report(operational),
    })
}

/// Whether a collection computed `metric` at all.
fn observes(collection: &MeasurementCollection, metric: &str) -> bool {
    collection
        .observations
        .iter()
        .any(|observation| observation.metric.as_str() == metric)
}

/// `report.ts:85-96`'s collection quote.
fn summary_of(
    repo: &Path,
    collection: &MeasurementCollection,
) -> Result<CollectionSummary, MeasurementError> {
    let id = CollectionId::parse(collection.collection_id.as_str())?;
    Ok(CollectionSummary {
        collection_id: collection.collection_id.as_str().to_owned(),
        timestamp: collection.timestamp.as_str().to_owned(),
        tool_identity: collection.tool_identity.as_str().to_owned(),
        tool_version: collection.tool_version.as_str().to_owned(),
        config_digest: collection.config_digest.as_str().to_owned(),
        source_revision: collection.source_revision.as_str().to_owned(),
        corpus_revision: collection.corpus_revision.clone(),
        path: measurement_path(repo, &id).to_string_lossy().into_owned(),
        unverified_artifacts: collection
            .verification_stack
            .as_ref()
            .map(|stack| stack.unverified_artifacts.clone())
            .unwrap_or_default(),
    })
}

/// The `bounds.gap_count` a collection's raw evidence states, if it states one
/// as a number.
///
/// `gapCount` (`report.ts:370-377`). `typeof x === "number"` admits any double,
/// including a fractional one, so this does too.
#[must_use]
pub fn gap_count(collection: &MeasurementCollection) -> Option<f64> {
    let JsonValue::Object(raw) = &collection.raw_evidence else {
        return None;
    };
    let Some(JsonValue::Object(bounds)) = raw.get("bounds") else {
        return None;
    };
    bounds.get("gap_count").and_then(JsonValue::as_f64)
}

/// The newest stated gap count, scanning back from the newest collection.
///
/// `latestGapCount` (`report.ts:379-384`).
#[must_use]
pub fn latest_gap_count(collections: &[MeasurementCollection]) -> Option<f64> {
    collections.iter().rev().find_map(gap_count)
}
