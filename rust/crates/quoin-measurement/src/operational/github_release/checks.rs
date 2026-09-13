// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every cross-check `github-release-operational.ts` makes before it will write.
//!
//! The definition against itself (`:185-213`), the workflow against the
//! definition (`:29-46`), the run against the definition (`:47-64`), the job
//! against the run (`:65-98`), and the three instants against each other. None
//! of these produces a value the records carry — they only decide whether there
//! is anything to produce — so they are the part of the producer that can be
//! read without knowing the record shapes at all.

use serde_json::{Map, Value};

use crate::date_time::{Rfc3339DateTime, to_iso_string};
use crate::operational::record::ClockStatus;

use super::{GitHubReleaseError, GitHubReleaseProducerDefinition, input};

/// `github-release-operational.ts:185-213`.
pub(super) fn validate_definition(
    definition: &GitHubReleaseProducerDefinition,
) -> Result<(), GitHubReleaseError> {
    for (name, value) in [
        ("record_prefix", definition.record_prefix.as_str()),
        ("workflow_path", definition.workflow_path.as_str()),
        ("release_job", definition.release_job.as_str()),
        ("accepted_event", definition.accepted_event.as_str()),
        ("control_id", definition.control_id.as_str()),
        (
            "workflow_evidence_path",
            definition.workflow_evidence_path.as_str(),
        ),
        ("run_evidence_path", definition.run_evidence_path.as_str()),
        ("jobs_evidence_path", definition.jobs_evidence_path.as_str()),
    ] {
        if value.is_empty() {
            return input(format!("producer definition requires {name}"));
        }
    }
    if definition.clock_deadline_seconds < 1 {
        return input("producer definition requires a positive clock_deadline_seconds");
    }
    Ok(())
}

/// `github-release-operational.ts:29-46`: exactly one configured release job,
/// and the accepted event is declared.
pub(super) fn check_workflow(
    workflow: &Value,
    definition: &GitHubReleaseProducerDefinition,
) -> Result<(), GitHubReleaseError> {
    let Some(root) = workflow.as_object() else {
        return input("workflow must be an object");
    };
    let Some(jobs) = root.get("jobs").and_then(Value::as_object) else {
        return input("workflow.jobs must be an object");
    };
    let configured = jobs
        .iter()
        .filter(|(key, value)| {
            value
                .as_object()
                .and_then(|job| job.get("name"))
                .and_then(Value::as_str)
                == Some(definition.release_job.as_str())
                || key.as_str() == definition.release_job
        })
        .count();
    if configured != 1 {
        return input(format!(
            "workflow must contain exactly one configured release job {}",
            definition.release_job
        ));
    }
    // `on` is looked up as a string. See the module header for why that is a
    // statement about the YAML version and not about this lookup.
    let Some(triggers) = root.get("on").and_then(Value::as_object) else {
        return input("workflow.on must be an object");
    };
    if !triggers.contains_key(&definition.accepted_event) {
        return input(format!(
            "workflow does not declare accepted event {}",
            definition.accepted_event
        ));
    }
    Ok(())
}

/// The run's identity, once it has agreed with the definition.
pub(super) struct RunIdentity {
    pub(super) id: u64,
    pub(super) attempt: u64,
}

/// `github-release-operational.ts:47-64`.
pub(super) fn check_run(
    run: &Map<String, Value>,
    definition: &GitHubReleaseProducerDefinition,
) -> Result<RunIdentity, GitHubReleaseError> {
    let matches = run.get("path").and_then(Value::as_str)
        == Some(definition.workflow_path.as_str())
        && run.get("event").and_then(Value::as_str) == Some(definition.accepted_event.as_str())
        && run.get("head_sha").and_then(Value::as_str)
            == Some(definition.subject.revision.as_str());
    if !matches {
        return input("workflow run path, event, or immutable source revision mismatch");
    }
    if run.get("status").and_then(Value::as_str) != Some("completed")
        || run.get("conclusion").and_then(Value::as_str).is_none()
    {
        return input("workflow run is not completed with a conclusion");
    }
    Ok(RunIdentity {
        id: positive_safe_integer(run.get("id"), "workflow-run.id")?,
        attempt: positive_integer(run.get("run_attempt"), "workflow-run.run_attempt")?,
    })
}

/// `github-release-operational.ts:65-73`: exactly one job by that name.
pub(super) fn select_job<'a>(
    jobs: &'a Map<String, Value>,
    definition: &GitHubReleaseProducerDefinition,
) -> Result<&'a Map<String, Value>, GitHubReleaseError> {
    let Some(listed) = jobs.get("jobs").and_then(Value::as_array) else {
        return input("workflow-jobs export requires jobs array");
    };
    let mut selected = listed.iter().filter_map(|value| {
        value
            .as_object()
            .filter(|job| job.get("name").and_then(Value::as_str) == Some(&definition.release_job))
    });
    match (selected.next(), selected.next()) {
        (Some(job), None) => Ok(job),
        _ => input(format!(
            "jobs export must contain exactly one {} job",
            definition.release_job
        )),
    }
}

/// The three instants and the deadline, once they have agreed with each other.
pub(super) struct Timing {
    pub(super) started_at: String,
    pub(super) completed_at: String,
    pub(super) observed_at: String,
    pub(super) deadline_at: String,
    pub(super) status: ClockStatus,
}

/// `github-release-operational.ts:74-98`.
pub(super) fn check_job(
    job: &Map<String, Value>,
    run: &Map<String, Value>,
    identity: &RunIdentity,
    definition: &GitHubReleaseProducerDefinition,
) -> Result<Timing, GitHubReleaseError> {
    let (Some(started_at), Some(completed_at)) = (
        job.get("started_at").and_then(Value::as_str),
        job.get("completed_at").and_then(Value::as_str),
    ) else {
        return input("configured release job is unstarted or incomplete");
    };
    if job.get("status").and_then(Value::as_str) != Some("completed")
        || job.get("conclusion").and_then(Value::as_str).is_none()
    {
        return input("configured release job is unstarted or incomplete");
    }
    let same_run = positive_safe_integer(job.get("run_id"), "job.run_id")? == identity.id
        && job.get("head_sha").and_then(Value::as_str)
            == run.get("head_sha").and_then(Value::as_str)
        && positive_integer(job.get("run_attempt"), "job.run_attempt")? == identity.attempt;
    if !same_run {
        return input(
            "configured release job run, revision, or attempt does not match workflow run",
        );
    }
    let started = instant(
        Some(&Value::String(started_at.to_owned())),
        "job.started_at",
    )?;
    let completed = instant(
        Some(&Value::String(completed_at.to_owned())),
        "job.completed_at",
    )?;
    // `run.updated_at ?? job.completed_at`: the run's own last word, falling
    // back to the job's.
    let observed_at = run
        .get("updated_at")
        .and_then(Value::as_str)
        .unwrap_or(completed_at)
        .to_owned();
    let observed = instant(Some(&Value::String(observed_at.clone())), "run.updated_at")?;
    if completed < started || observed < completed {
        return input("workflow job/run timestamps are not ordered");
    }
    let deadline = started.saturating_add(
        i64::try_from(definition.clock_deadline_seconds)
            .unwrap_or(i64::MAX)
            .saturating_mul(1_000),
    );
    let Some(deadline_at) = to_iso_string(deadline) else {
        return input("release deadline falls outside the four-digit years");
    };
    Ok(Timing {
        started_at: started_at.to_owned(),
        completed_at: completed_at.to_owned(),
        observed_at,
        deadline_at,
        status: if completed <= deadline {
            ClockStatus::Met
        } else {
            ClockStatus::Missed
        },
    })
}

pub(super) fn instant(value: Option<&Value>, label: &str) -> Result<i64, GitHubReleaseError> {
    value
        .and_then(Value::as_str)
        .and_then(|text| Rfc3339DateTime::parse(text).ok())
        .map(|parsed| parsed.epoch_millis())
        .ok_or_else(|| GitHubReleaseError::Input(format!("{label} must be an RFC 3339 date-time")))
}

/// `Number.isSafeInteger(value) && value >= 1`.
pub(super) fn positive_safe_integer(
    value: Option<&Value>,
    label: &str,
) -> Result<u64, GitHubReleaseError> {
    const SAFE: f64 = 9_007_199_254_740_991.0;
    let number = value
        .and_then(Value::as_f64)
        .filter(|number| number.fract() == 0.0 && *number >= 1.0 && *number <= SAFE);
    match number {
        // The filter above proves the cast is exact.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the value is a positive integer at most 2^53-1, which u64 holds exactly"
        )]
        Some(number) => Ok(number as u64),
        None => input(format!("{label} must be a positive safe integer")),
    }
}

/// `Number.isInteger(value) && value >= 1`.
pub(super) fn positive_integer(
    value: Option<&Value>,
    label: &str,
) -> Result<u64, GitHubReleaseError> {
    positive_safe_integer(value, label)
        .map_err(|_| GitHubReleaseError::Input(format!("{label} must be a positive integer")))
}
