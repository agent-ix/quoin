// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every FR-061 acceptance criterion the deleted `tests/operational.test.ts`
//! carried, restated against this crate.
//!
//! Trace: FR-061-AC-1, FR-061-AC-2, FR-061-AC-3, FR-061-AC-4, FR-061-AC-5
//! Provenance: quoin#479
//!
//! # Why this file exists
//!
//! The retire-after-parity rule retires `src/measurement/` after parity. `tests/operational.test.ts`
//! goes with it, and with it the only statement of what the GitHub-release
//! operational producer must do. Each criterion is restated here against the
//! Rust producer, one `#[test]` each, tagged at criterion level.
//!
//! # The inputs are synthetic and built here
//!
//! `common/release_evidence.rs` writes a small workflow, run and jobs export
//! into a temporary repository, with a generated head revision. Every refusal
//! case is a mutation of one of those exports, so a refusal is a statement
//! about an input changed in one named way. Nothing here writes into this
//! repository.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

#[path = "common/census.rs"]
mod census;
#[path = "common/release_evidence.rs"]
mod release_evidence;

use census::rust_files;
use release_evidence::{JOBS, PAIR, PREFIX, RUN, WORKFLOW, write_repository};

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use quoin_measurement::intervention::intake::InterventionRefusalCode;
use quoin_measurement::operational::discharge::operational_discharge;
use quoin_measurement::operational::github_release::{
    GitHubReleaseError, GitHubReleaseOperational, GitHubReleaseProducerDefinition,
    produce_github_release_operational,
};
use quoin_measurement::operational::intake::write_operational_pair;
use quoin_measurement::operational::paths::{operational_pairs_root, operational_root};
use quoin_measurement::operational::read::read_operational_records;
use quoin_measurement::operational::record::{OperationalEvidenceRecord, OperationalObligation};
use quoin_measurement::source::SystemClock;

/// A temporary repository carrying the synthetic governance and exports.
fn temporary_repository() -> tempfile::TempDir {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    write_repository(temporary.path());
    temporary
}

fn github_actions(root: &Path) -> PathBuf {
    root.join("spec").join("evidence").join("github-actions")
}

/// The producer definition, as a document.
fn definition_document() -> Value {
    release_evidence::definition_document()
}

fn definition_of(document: &Value) -> GitHubReleaseProducerDefinition {
    serde_json::from_value(document.clone()).expect("the definition reads")
}

/// One retained JSON export from a workspace.
fn export(root: &Path, name: &str) -> Value {
    let path = github_actions(root).join(name);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_slice(&bytes).expect("a retained export is JSON")
}

/// Rewrite one export inside a workspace.
fn rewrite(root: &Path, name: &str, value: &Value) {
    let text = serde_json::to_string_pretty(value).expect("the export serialises");
    std::fs::write(github_actions(root).join(name), text).expect("a writable export");
}

/// Replace the value at `pointer`, which must already be present.
fn patch(base: &Value, pointer: &str, value: Value) -> Value {
    let mut out = base.clone();
    let slot = out
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("{pointer} is present in the synthetic evidence"));
    *slot = value;
    out
}

/// Remove `key` from the object at `pointer` (`""` for the document root).
fn remove(base: &Value, pointer: &str, key: &str) -> Value {
    let mut out = base.clone();
    let slot = out
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("{pointer} is present in the synthetic evidence"));
    slot.as_object_mut()
        .unwrap_or_else(|| panic!("{pointer} is an object"))
        .remove(key)
        .unwrap_or_else(|| panic!("{pointer}/{key} is present"));
    out
}

/// The index of the configured release job in the synthetic jobs export.
fn publish_index(jobs: &Value) -> usize {
    jobs["jobs"]
        .as_array()
        .expect("the jobs export lists jobs")
        .iter()
        .position(|job| job["name"] == json!("Publish"))
        .expect("the synthetic export carries the Publish job")
}

/// Produce into a workspace from the definition.
fn produce(root: &Path) -> Result<GitHubReleaseOperational, GitHubReleaseError> {
    produce_github_release_operational(root, &SystemClock, &definition_of(&definition_document()))
}

/// The obligation a release run is against.
fn obligation() -> OperationalObligation {
    serde_json::from_value(json!({
        "control_kind": "release",
        "subject": {
            "id": "agent-ix/quoin"
        },
        "scope": {
            "service": "agent-ix/quoin",
            "environment": "npm-production",
            "population": "synthetic package release"
        },
        "accepted_modes": ["actual"],
        "clock": {
            "applicability": "operational_with_clock",
            "started_at": "2026-08-29T23:11:00Z",
            "deadline_at": "2026-08-29T23:21:00.000Z"
        }
    }))
    .expect("the obligation reads")
}

/// Quoin's release workflow and synthetic workflow-run/jobs exports produce
/// one linked standing-capability and exercise pair whose workflow, actor,
/// timing, outcome, and observations match the input data.
///
/// Trace: FR-061-AC-1
/// Provenance: quoin#479
#[test]
fn tc_479_450_synthetic_release_inputs_produce_one_exact_linked_pair() {
    let workspace = temporary_repository();
    let root = workspace.path();
    let produced = produce(root).expect("synthetic release evidence produces a pair");

    assert_eq!(
        produced.path.file_name().and_then(|name| name.to_str()),
        Some(PAIR),
        "the pair is not named by the two identities it carries"
    );

    let capability = &produced.capability;
    assert_eq!(
        capability.base.record_id.as_str(),
        &format!("{PREFIX}-capability")
    );
    assert_eq!(
        capability.capability.surface,
        ".github/workflows/release.yml"
    );

    let exercise = &produced.exercise.exercise;
    assert_eq!(
        produced.exercise.base.record_id.as_str(),
        &format!("{PREFIX}-exercise")
    );
    assert_eq!(
        exercise
            .capability_record_id
            .as_ref()
            .expect("the exercise names a capability")
            .as_str(),
        &format!("{PREFIX}-capability"),
        "the pair is not linked"
    );
    assert_eq!(exercise.actor, "release-actor");
    assert_eq!(exercise.trigger, "workflow_dispatch");
    assert_eq!(exercise.started_at.as_str(), "2026-08-29T23:11:00Z");
    assert_eq!(exercise.completed_at.as_str(), "2026-08-29T23:11:37Z");
    assert_eq!(
        exercise.observations,
        [
            "workflow .github/workflows/release.yml",
            "job Publish concluded success"
        ]
    );
    assert!(exercise.state_before.is_empty());
    assert_eq!(
        produced.exercise.base.observed_at.as_str(),
        "2026-08-29T23:11:38Z"
    );
    assert_eq!(read_operational_records(root).expect("read back").len(), 2);
}

/// Missing or malformed YAML/JSON, a workflow/path/event mismatch, or an
/// absent, duplicate, unstarted, incomplete, or mismatched release job is
/// refused without either operational record.
///
/// Trace: FR-061-AC-2
/// Provenance: quoin#479
#[test]
fn tc_479_451_malformed_and_mismatched_inputs_are_refused_without_either_record() {
    let cases = refusal_cases();
    for (label, damage, expected) in &cases {
        let workspace = temporary_repository();
        let root = workspace.path();
        damage(root);
        let error = produce(root).expect_err(&format!("{label} must be refused"));
        let GitHubReleaseError::Input(message) = &error else {
            panic!("{label}: the store refused rather than the input contract: {error}");
        };
        assert!(
            message.contains(expected),
            "{label}: {message:?} does not name {expected:?}"
        );
        assert!(
            std::fs::read_dir(operational_root(root)).is_err()
                || read_operational_records(root)
                    .expect("the store reads back")
                    .is_empty(),
            "{label} retained an operational record"
        );
        assert!(
            !operational_pairs_root(root).exists(),
            "{label} retained a pair"
        );
    }
}

/// Each way the retained input contract can fail, as a mutation of the
/// synthetic evidence.
type RefusalCase = (&'static str, Box<dyn Fn(&Path)>, &'static str);

fn refusal_cases() -> Vec<RefusalCase> {
    let mut cases = export_refusal_cases();
    cases.extend(job_refusal_cases());
    cases
}

/// The refusals the workflow and run exports carry.
fn export_refusal_cases() -> Vec<RefusalCase> {
    fn run_patch(pointer: &'static str, value: Value) -> Box<dyn Fn(&Path)> {
        Box::new(move |root| {
            let damaged = patch(&export(root, RUN), pointer, value.clone());
            rewrite(root, RUN, &damaged);
        })
    }
    vec![
        (
            "a malformed workflow-run export",
            Box::new(|root| {
                std::fs::write(github_actions(root).join(RUN), b"{\"id\":").unwrap();
            }),
            "workflow-run export is malformed",
        ),
        (
            "a malformed workflow-jobs export",
            Box::new(|root| {
                std::fs::write(github_actions(root).join(JOBS), b"not json").unwrap();
            }),
            "workflow-jobs export is malformed",
        ),
        (
            "a workflow whose jobs are not an object",
            Box::new(|root| {
                std::fs::write(
                    github_actions(root).join(WORKFLOW),
                    b"on:\n  workflow_dispatch:\njobs: 3\n",
                )
                .unwrap();
            }),
            "workflow.jobs must be an object",
        ),
        (
            "a workflow that does not declare the accepted event",
            Box::new(|root| {
                let workflow = std::fs::read_to_string(github_actions(root).join(WORKFLOW))
                    .unwrap()
                    .replace("workflow_dispatch:", "schedule:");
                std::fs::write(github_actions(root).join(WORKFLOW), workflow).unwrap();
            }),
            "workflow does not declare accepted event workflow_dispatch",
        ),
        (
            "a run against a different workflow path",
            run_patch("/path", json!(".github/workflows/build-test.yml")),
            "workflow run path or event mismatch",
        ),
        (
            "a run from a different event",
            run_patch("/event", json!("push")),
            "workflow run path or event mismatch",
        ),
        (
            "a run that has not finished",
            run_patch("/status", json!("in_progress")),
            "workflow run is not completed with a conclusion",
        ),
        (
            "a run with no conclusion",
            run_patch("/conclusion", Value::Null),
            "workflow run is not completed with a conclusion",
        ),
        (
            "a run whose id is not a positive safe integer",
            run_patch("/id", json!(0)),
            "workflow-run.id must be a positive safe integer",
        ),
    ]
}

/// The refusals the jobs export carries.
fn job_refusal_cases() -> Vec<RefusalCase> {
    fn job_patch(pointer: &'static str, value: Value) -> Box<dyn Fn(&Path)> {
        Box::new(move |root| {
            let jobs = export(root, JOBS);
            let index = publish_index(&jobs);
            let damaged = patch(&jobs, &format!("/jobs/{index}{pointer}"), value.clone());
            rewrite(root, JOBS, &damaged);
        })
    }
    vec![
        (
            "an absent release job",
            Box::new(|root| {
                let jobs = export(root, JOBS);
                let index = publish_index(&jobs);
                let mut damaged = jobs.clone();
                damaged["jobs"].as_array_mut().unwrap().remove(index);
                rewrite(root, JOBS, &damaged);
            }),
            "jobs export must contain exactly one Publish job",
        ),
        (
            "a duplicated release job",
            Box::new(|root| {
                let jobs = export(root, JOBS);
                let index = publish_index(&jobs);
                let mut damaged = jobs.clone();
                let publish = damaged["jobs"][index].clone();
                damaged["jobs"].as_array_mut().unwrap().push(publish);
                rewrite(root, JOBS, &damaged);
            }),
            "jobs export must contain exactly one Publish job",
        ),
        (
            "an unstarted release job",
            Box::new(|root| {
                let jobs = export(root, JOBS);
                let index = publish_index(&jobs);
                let damaged = remove(&jobs, &format!("/jobs/{index}"), "started_at");
                rewrite(root, JOBS, &damaged);
            }),
            "configured release job is unstarted or incomplete",
        ),
        (
            "an incomplete release job",
            job_patch("/status", json!("in_progress")),
            "configured release job is unstarted or incomplete",
        ),
        (
            "a release job from another run",
            job_patch("/run_id", json!(1)),
            "configured release job run or attempt does not match workflow run",
        ),
        (
            "a release job from another attempt",
            job_patch("/run_attempt", json!(2)),
            "configured release job run or attempt does not match workflow run",
        ),
        (
            "a release job from another revision",
            job_patch("/head_sha", json!("synthetic-different-head")),
            "configured release job revision does not match workflow run",
        ),
        (
            "a release job that completed before it started",
            job_patch("/completed_at", json!("2026-08-29T23:10:00Z")),
            "workflow job/run timestamps are not ordered",
        ),
    ]
}

/// Capability state is derived from the parsed workflow and exercise state and
/// clock status are derived from the retained API exports; caller-supplied
/// replacements cannot change them.
///
/// Trace: FR-061-AC-3
/// Provenance: quoin#479
#[test]
fn tc_479_452_workflow_structure_and_api_exports_decide_state_not_the_caller() {
    // A definition carrying every state member a caller might hope to supply.
    let smuggled = definition_document();
    let mut smuggled = smuggled.as_object().expect("the definition").clone();
    for (member, value) in [
        ("status", json!("unavailable")),
        ("outcome", json!("failed")),
        ("conclusion", json!("failure")),
        ("clock_status", json!("missed")),
        ("observed_at", json!("2001-01-01T00:00:00Z")),
        ("raw_evidence", json!([])),
    ] {
        smuggled.insert(member.to_owned(), value);
    }
    let workspace = temporary_repository();
    let root = workspace.path();
    let produced = produce_github_release_operational(
        root,
        &SystemClock,
        &definition_of(&Value::Object(smuggled)),
    )
    .expect("a definition carrying state members still produces");
    let baseline_workspace = temporary_repository();
    let baseline = produce(baseline_workspace.path()).expect("the baseline produces");
    assert_eq!(
        std::fs::read(&produced.path).unwrap(),
        std::fs::read(&baseline.path).unwrap(),
        "a caller-supplied state member reached the produced records"
    );

    // The exports decide the outcome.
    let workspace = temporary_repository();
    let root = workspace.path();
    let jobs = export(root, JOBS);
    let index = publish_index(&jobs);
    let damaged = patch(
        &jobs,
        &format!("/jobs/{index}/conclusion"),
        json!("failure"),
    );
    rewrite(root, JOBS, &damaged);
    let produced = produce(root).expect("a failed release run still produces a pair");
    assert_eq!(produced.exercise.exercise.outcome.as_str(), "failed");

    // The exports decide the clock, against the definition's own deadline.
    let workspace = temporary_repository();
    let root = workspace.path();
    let jobs = patch(
        &export(root, JOBS),
        &format!("/jobs/{index}/completed_at"),
        json!("2026-08-29T23:31:00Z"),
    );
    rewrite(root, JOBS, &jobs);
    let run = patch(
        &export(root, RUN),
        "/updated_at",
        json!("2026-08-29T23:32:00Z"),
    );
    rewrite(root, RUN, &run);
    let produced = produce(root).expect("a late release run still produces a pair");
    let document =
        serde_json::to_value(produced.exercise.clone()).expect("the exercise serialises");
    assert_eq!(document["exercise"]["clock"]["status"], json!("missed"));
    assert_eq!(
        document["exercise"]["clock"]["deadline_at"],
        json!("2026-08-29T23:21:00.000Z"),
        "the deadline is the definition's, measured from the job's own start"
    );
}

/// Every non-success GitHub conclusion remains a named non-success exercise and
/// cannot discharge the clocked release obligation.
///
/// Trace: FR-061-AC-4
/// Provenance: quoin#479
#[test]
fn tc_479_453_every_non_success_conclusion_is_named_and_cannot_discharge() {
    /// Every conclusion GitHub reports, and the outcome it must become. A
    /// conclusion the producer has not been taught is `partial`, never
    /// `succeeded`.
    const CONCLUSIONS: [(&str, &str); 7] = [
        ("failure", "failed"),
        ("timed_out", "failed"),
        ("action_required", "failed"),
        ("cancelled", "aborted"),
        ("skipped", "partial"),
        ("neutral", "partial"),
        ("startup_failure", "partial"),
    ];

    for (conclusion, outcome) in CONCLUSIONS {
        let workspace = temporary_repository();
        let root = workspace.path();
        let jobs = export(root, JOBS);
        let index = publish_index(&jobs);
        let damaged = patch(
            &jobs,
            &format!("/jobs/{index}/conclusion"),
            json!(conclusion),
        );
        rewrite(root, JOBS, &damaged);

        let produced = produce(root)
            .unwrap_or_else(|error| panic!("{conclusion} must still be recorded: {error}"));
        assert_eq!(
            produced.exercise.exercise.outcome.as_str(),
            outcome,
            "{conclusion} was not named {outcome}"
        );
        assert_eq!(
            produced.exercise.exercise.observations[1],
            format!("job Publish concluded {conclusion}"),
            "{conclusion} was not named in the observations"
        );

        let retained = read_operational_records(root).expect("the pair reads back");
        let exercise = retained
            .iter()
            .find(|record| matches!(record.record(), OperationalEvidenceRecord::Exercise(_)))
            .expect("the exercise is retained");
        let refused = operational_discharge(exercise, &obligation())
            .expect_err(&format!("{conclusion} must not discharge"));
        assert_eq!(refused.reason(), format!("exercise outcome {outcome}"));
    }
}

/// A release-run integration supplies synthetic workflow-run and jobs exports,
/// then the Quoin producer consumes the files without
/// network or process execution and atomically persists the pair through
/// FR-060.
///
/// Trace: FR-061-AC-5
/// Provenance: quoin#479
#[test]
fn tc_479_454_synthetic_exports_are_consumed_offline_and_persisted_all_or_nothing() {
    /// The producer reads files. Anything here would be Quoin running the
    /// release rather than recording one somebody else ran.
    const FORBIDDEN: [&str; 8] = [
        "std::process",
        "Command::new",
        "std::net",
        "TcpStream",
        "reqwest",
        "ureq",
        "octocrab",
        "curl",
    ];

    let census_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("operational")
        .join("github_release");
    for path in rust_files(&census_root) {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: unreadable: {error}", path.display()));
        for needle in FORBIDDEN {
            assert!(
                !text.contains(needle),
                "{}: the producer reaches for `{needle}`",
                path.display()
            );
        }
    }

    // All of the pair, or none of it: the two records land in one file and
    // nothing lands beside it.
    let workspace = temporary_repository();
    let root = workspace.path();
    let produced = produce(root).expect("the retained evidence produces a pair");
    assert_eq!(
        json_files(&operational_pairs_root(root)),
        vec![produced.path]
    );
    assert!(
        json_files(&operational_root(root)).is_empty(),
        "a pair member was published on its own"
    );

    // A pair the store refuses leaves neither record behind.
    let workspace = temporary_repository();
    let root = workspace.path();
    let produced = produce(root).expect("the records to work from");
    let capability = serde_json::to_value(OperationalEvidenceRecord::StandingCapability(
        produced.capability.clone(),
    ))
    .expect("the capability serialises");
    let mut exercise = serde_json::to_value(OperationalEvidenceRecord::Exercise(
        produced.exercise.clone(),
    ))
    .expect("the exercise serialises");
    exercise["exercise"]["control_id"] = json!("some-other-control");
    exercise["record_id"] = json!(format!("{PREFIX}-exercise-unlinked"));

    let fresh = temporary_repository();
    let root = fresh.path();
    let error = write_operational_pair(root, &SystemClock, &capability, &exercise)
        .expect_err("an unlinked pair is refused");
    assert_eq!(error.code(), InterventionRefusalCode::InvalidRecord);
    assert!(
        json_files(&operational_pairs_root(root)).is_empty()
            && json_files(&operational_root(root)).is_empty(),
        "a refused pair retained a record"
    );
}

/// Every `.json` file directly inside a directory, sorted.
fn json_files(directory: &Path) -> Vec<PathBuf> {
    let Ok(listing) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = listing
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|end| end == "json"))
        .collect();
    found.sort();
    found
}
