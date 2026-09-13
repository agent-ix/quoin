// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The GitHub-release producer: three retained exports in, one pair out.
//!
//! Ports `operational-types.ts:112-136` (the definition) and the whole of
//! `src/measurement/github-release-operational.ts` (the producer).
//!
//! # The producer decides nothing it is not shown
//!
//! Every value in the two records it writes comes from the definition or from
//! one of the three retained exports, and every cross-check the retained code
//! makes is kept: the run's workflow path, event and head revision against the
//! definition, the job's run id, revision and attempt against the run, and the
//! three instants against each other. A producer that filled a gap with a
//! default would be producing evidence rather than recording it.
//!
//! # The workflow export is read as YAML 1.2, and `on:` is a key
//!
//! `workflow.on` is looked up by the **string** `"on"`. Under YAML 1.1 —
//! which every libyaml binding implements — the plain scalar `on` resolves to
//! the boolean `true`, and this lookup would miss on every GitHub workflow file
//! there is. [`quoin_yaml::from_str`] is YAML 1.2 core, where `on` is the
//! string it looks like, and `tests/tc_472_github_release.rs` asserts that
//! against the retained export rather than trusting it.
//!
//! # Two error shapes, because there are two failures
//!
//! `github-release-operational.ts` throws a bare `Error` when the retained
//! exports do not satisfy the producer's input contract, and lets
//! `writeOperationalPair`'s `InterventionIntakeError` propagate when the store
//! refuses the result. They are different failures with different audiences, so
//! they stay different here: [`GitHubReleaseError::Input`] and
//! [`GitHubReleaseError::Intake`].

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::common::identity::{ControlId, EvidencePath, RecordId};
use crate::common::producer::{Producer, Subject};
use crate::intervention::intake::InterventionIntakeError;
use crate::json_bridge::to_serde;
use crate::operational::intake::write_operational_pair;
use crate::operational::record::{
    CapabilityStatus, ClockSupport, OperationalConfiguration, OperationalExerciseRecord,
    OperationalScope, StandingCapability, StandingCapabilityRecord,
};
use crate::raw_evidence::{RawEvidencePath, raw_evidence_for};
use crate::source::{Clock, DiskMeasurement, MeasurementSource};

mod checks;
mod records;

use checks::{check_job, check_run, check_workflow, select_job, validate_definition};
use records::{base_for, exercise_for, producer_for, raw_evidence};

/// What a GitHub release run needs in order to become an operational pair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitHubReleaseProducerDefinition {
    /// The prefix the produced record identities carry.
    pub record_prefix: String,
    /// Which workflow file governs the release.
    pub workflow_path: EvidencePath,
    /// Which job in it performs the release.
    pub release_job: String,
    /// Which workflow event the release is accepted from.
    pub accepted_event: String,
    /// The control the produced records are about.
    pub control_id: ControlId,
    /// What the control governs.
    pub subject: Subject,
    /// What produces the records.
    pub producer: Producer,
    /// Where the control is observed.
    pub scope: OperationalScope,
    /// What the control runs under.
    pub configuration: OperationalConfiguration,
    /// The state transition the control supports.
    pub supported_transition: String,
    /// Who may use it.
    pub authorized_roles: Vec<String>,
    /// What it covers.
    pub coverage: String,
    /// What it does not.
    pub limitations: Vec<String>,
    /// Who owns the produced records.
    pub owner: String,
    /// Declared gaps.
    pub gaps: Vec<String>,
    /// Declared actions.
    pub actions: Vec<String>,
    /// How long the control has, in seconds.
    pub clock_deadline_seconds: u64,
    /// Where the retained workflow export is.
    pub workflow_evidence_path: EvidencePath,
    /// Where the retained run export is.
    pub run_evidence_path: EvidencePath,
    /// Where the retained jobs export is.
    pub jobs_evidence_path: EvidencePath,
}

/// What the producer wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct GitHubReleaseOperational {
    /// The pair file the two records are retained in.
    pub path: PathBuf,
    /// The standing capability.
    pub capability: StandingCapabilityRecord,
    /// The exercise.
    pub exercise: OperationalExerciseRecord,
}

/// Why a GitHub release run did not become an operational pair.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GitHubReleaseError {
    /// The definition or the retained exports do not satisfy the input
    /// contract. `github-release-operational.ts` raises a bare `Error` here.
    #[error("{0}")]
    Input(String),
    /// The store refused the produced pair.
    #[error(transparent)]
    Intake(#[from] InterventionIntakeError),
}

fn input<T>(detail: impl Into<String>) -> Result<T, GitHubReleaseError> {
    Err(GitHubReleaseError::Input(detail.into()))
}

/// Produce and retain the capability/exercise pair for one release run.
///
/// # Errors
///
/// [`GitHubReleaseError::Input`] for every disagreement between the definition
/// and the three retained exports, and [`GitHubReleaseError::Intake`] when the
/// store refuses the result.
pub fn produce_github_release_operational(
    repo: &Path,
    clock: &dyn Clock,
    definition: &GitHubReleaseProducerDefinition,
) -> Result<GitHubReleaseOperational, GitHubReleaseError> {
    validate_definition(definition)?;
    let source = DiskMeasurement::new(repo);
    let workflow = read_workflow(&source, definition)?;
    let run = read_export(&source, &definition.run_evidence_path, "workflow-run")?;
    let jobs = read_export(&source, &definition.jobs_evidence_path, "workflow-jobs")?;
    check_workflow(&workflow, definition)?;
    let run_identity = check_run(&run, definition)?;
    let job = select_job(&jobs, definition)?;
    let timing = check_job(job, &run, &run_identity, definition)?;

    let producer = producer_for(definition, &run_identity, &run);
    let raw = raw_evidence(&source, definition)?;
    let capability_id = RecordId::from_stored(format!("{}-capability", definition.record_prefix));
    let exercise_id = RecordId::from_stored(format!("{}-exercise", definition.record_prefix));
    let capability = StandingCapabilityRecord {
        base: base_for(definition, &capability_id, &timing, &producer, &raw),
        capability: StandingCapability {
            control_id: definition.control_id.clone(),
            status: CapabilityStatus::Available,
            surface: definition.workflow_path.as_str().to_owned(),
            authorized_roles: definition.authorized_roles.clone(),
            coverage: definition.coverage.clone(),
            limitations: definition.limitations.clone(),
            supported_transitions: vec![definition.supported_transition.clone()],
            clock_support: ClockSupport::Supported {
                supported: crate::common::literal_bool::LiteralBool::VALUE,
                start_event: "release_job_started".to_owned(),
                completion_event: "release_job_completed".to_owned(),
                deadline_seconds: definition.clock_deadline_seconds,
            },
        },
    };
    let exercise = OperationalExerciseRecord {
        base: base_for(definition, &exercise_id, &timing, &producer, &raw),
        exercise: exercise_for(definition, &capability_id, &timing, &run, job),
    };

    let capability_document = serde_json::to_value(
        crate::operational::record::OperationalEvidenceRecord::StandingCapability(
            capability.clone(),
        ),
    )
    .map_err(|error| GitHubReleaseError::Input(error.to_string()))?;
    let exercise_document = serde_json::to_value(
        crate::operational::record::OperationalEvidenceRecord::Exercise(exercise.clone()),
    )
    .map_err(|error| GitHubReleaseError::Input(error.to_string()))?;
    let path = write_operational_pair(repo, clock, &capability_document, &exercise_document)?;
    Ok(GitHubReleaseOperational {
        path,
        capability,
        exercise,
    })
}

/// The retained workflow export, accounted for and then read as YAML 1.2.
fn read_workflow(
    source: &DiskMeasurement,
    definition: &GitHubReleaseProducerDefinition,
) -> Result<Value, GitHubReleaseError> {
    let text = retained_text(source, &definition.workflow_evidence_path)?;
    quoin_yaml::from_str(&text).map_err(|error| GitHubReleaseError::Input(error.to_string()))
}

/// One retained JSON export, accounted for and then read.
fn read_export(
    source: &DiskMeasurement,
    path: &EvidencePath,
    label: &str,
) -> Result<Map<String, Value>, GitHubReleaseError> {
    let text = retained_text(source, path)?;
    let stored = quoin_store::parse_strict_json_str(&text).map_err(|error| {
        GitHubReleaseError::Input(format!("{label} export is malformed: {error}"))
    })?;
    let value = to_serde(&stored).map_err(|error| {
        GitHubReleaseError::Input(format!("{label} export is malformed: {error}"))
    })?;
    match value {
        Value::Object(members) => Ok(members),
        _ => input(format!("{label} must be an object")),
    }
}

/// `github-release-operational.ts:216-223`: account for the file, then read it.
///
/// The accounting is not incidental — it is what puts the file's size and
/// digest under the same guards the record's `raw_evidence` is checked against,
/// so a producer cannot read a file the record could not then reference.
fn retained_text(
    source: &DiskMeasurement,
    path: &EvidencePath,
) -> Result<String, GitHubReleaseError> {
    let parsed = RawEvidencePath::parse(path.as_str())
        .map_err(|error| GitHubReleaseError::Input(error.to_string()))?;
    raw_evidence_for(source, path.as_str(), media_type_for(path))
        .map_err(|error| GitHubReleaseError::Input(error.to_string()))?;
    source
        .retained_evidence_text(&parsed)
        .map_err(|error| GitHubReleaseError::Input(error.to_string()))
}

/// `github-release-operational.ts:218`: the extension decides, and there are
/// two.
fn media_type_for(path: &EvidencePath) -> &'static str {
    if std::path::Path::new(path.as_str())
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
    {
        "application/json"
    } else {
        "application/yaml"
    }
}
