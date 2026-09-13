// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The two records the producer builds, once the checks have passed.
//!
//! `github-release-operational.ts:100-178`. Everything here is assembly: each
//! member is copied from the definition or read out of a retained export, and
//! the only decision made is [`map_conclusion`], which maps a GitHub conclusion
//! onto an exercise outcome and answers `partial` for anything it has not been
//! taught.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::common::identity::{RecordId, WireInstant};
use crate::common::producer::Producer;
use crate::common::recorded_evidence::RecordedEvidenceReference;
use crate::common::scalar::EnvironmentValue;
use crate::operational::record::{
    ExerciseMode, ExerciseOutcome, OperationalBase, OperationalControlKind, OperationalExercise,
    OperationalRecordType,
};
use crate::raw_evidence::raw_evidence_for;
use crate::source::DiskMeasurement;

use super::checks::{RunIdentity, Timing};
use super::{GitHubReleaseError, GitHubReleaseProducerDefinition, media_type_for};

/// `github-release-operational.ts:100-108`: the definition's producer, with the
/// run it observed named in its environment.
pub(super) fn producer_for(
    definition: &GitHubReleaseProducerDefinition,
    identity: &RunIdentity,
    run: &Map<String, Value>,
) -> Producer {
    let mut environment: BTreeMap<String, EnvironmentValue> =
        definition.producer.environment.clone();
    environment.insert(
        "github_run_id".to_owned(),
        EnvironmentValue::Number(identity.id.into()),
    );
    environment.insert("github_run_url".to_owned(), scalar(run.get("html_url")));
    Producer {
        environment,
        ..definition.producer.clone()
    }
}

/// The three retained exports, as the records reference them.
pub(super) fn raw_evidence(
    source: &DiskMeasurement,
    definition: &GitHubReleaseProducerDefinition,
) -> Result<Vec<RecordedEvidenceReference>, GitHubReleaseError> {
    [
        &definition.workflow_evidence_path,
        &definition.run_evidence_path,
        &definition.jobs_evidence_path,
    ]
    .into_iter()
    .map(|path| {
        // `From<RawEvidenceReference>` (quoin#471), not a widening written out
        // here: minted-to-recorded is one conversion and it has one home.
        raw_evidence_for(source, path.as_str(), media_type_for(path))
            .map(RecordedEvidenceReference::from)
            .map_err(|error| GitHubReleaseError::Input(error.to_string()))
    })
    .collect()
}

/// Everything both produced records carry.
pub(super) fn base_for(
    definition: &GitHubReleaseProducerDefinition,
    record_id: &RecordId,
    timing: &Timing,
    producer: &Producer,
    raw: &[RecordedEvidenceReference],
) -> OperationalBase {
    OperationalBase {
        schema_version: 1,
        record_type: OperationalRecordType::OperationalEvidence,
        record_id: record_id.clone(),
        observed_at: WireInstant::from_stored(timing.observed_at.clone()),
        control_kind: OperationalControlKind::Release,
        subject: definition.subject.clone(),
        producer: producer.clone(),
        scope: definition.scope.clone(),
        configuration: definition.configuration.clone(),
        owner: definition.owner.clone(),
        gaps: definition.gaps.clone(),
        actions: definition.actions.clone(),
        raw_evidence: raw.to_vec(),
    }
}

/// `github-release-operational.ts:141-178`.
pub(super) fn exercise_for(
    definition: &GitHubReleaseProducerDefinition,
    capability_id: &RecordId,
    timing: &Timing,
    run: &Map<String, Value>,
    job: &Map<String, Value>,
) -> OperationalExercise {
    let conclusion = job
        .get("conclusion")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let head_sha = run
        .get("head_sha")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let mut state_before = Map::new();
    state_before.insert("source_revision".to_owned(), Value::String(head_sha));
    let mut state_after = Map::new();
    state_after.insert("conclusion".to_owned(), Value::String(conclusion.clone()));
    state_after.insert(
        "run_id".to_owned(),
        run.get("id").cloned().unwrap_or(Value::Null),
    );
    state_after.insert(
        "url".to_owned(),
        run.get("html_url")
            .filter(|value| !value.is_array() && !value.is_object())
            .cloned()
            .unwrap_or(Value::Null),
    );
    OperationalExercise {
        control_id: definition.control_id.clone(),
        capability_record_id: Some(capability_id.clone()),
        mode: ExerciseMode::Actual,
        started_at: WireInstant::from_stored(timing.started_at.clone()),
        completed_at: WireInstant::from_stored(timing.completed_at.clone()),
        actor: run
            .get("actor")
            .and_then(Value::as_object)
            .and_then(|actor| actor.get("login"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        trigger: run
            .get("event")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        outcome: map_conclusion(&conclusion),
        state_before,
        state_after,
        observations: vec![
            format!("workflow {}", definition.workflow_path),
            format!("job {} concluded {conclusion}", definition.release_job),
        ],
        clock: crate::operational::record::ExerciseClock::WithClock {
            started_at: WireInstant::from_stored(timing.started_at.clone()),
            deadline_at: WireInstant::from_stored(timing.deadline_at.clone()),
            completed_at: Some(WireInstant::from_stored(timing.completed_at.clone())),
            status: timing.status,
        },
    }
}

/// `github-release-operational.ts:255-268`. A conclusion this producer has not
/// been taught is `partial`, never `succeeded`: an unknown answer is not a good
/// one.
pub(super) fn map_conclusion(conclusion: &str) -> ExerciseOutcome {
    match conclusion {
        "success" => ExerciseOutcome::Succeeded,
        "failure" | "timed_out" | "action_required" => ExerciseOutcome::Failed,
        "cancelled" => ExerciseOutcome::Aborted,
        _ => ExerciseOutcome::Partial,
    }
}

/// `github-release-operational.ts:270-277`: a composite is not a scalar.
pub(super) fn scalar(value: Option<&Value>) -> EnvironmentValue {
    match value {
        Some(Value::Bool(flag)) => EnvironmentValue::Bool(*flag),
        Some(Value::Number(number)) => EnvironmentValue::Number(number.clone()),
        Some(Value::String(text)) => EnvironmentValue::String(text.clone()),
        _ => EnvironmentValue::Null,
    }
}
