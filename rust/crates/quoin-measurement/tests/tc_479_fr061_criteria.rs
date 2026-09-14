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
//! FR-101 retires `src/measurement/` after parity. `tests/operational.test.ts`
//! goes with it, and with it the only statement of what the GitHub-release
//! operational producer must do. Each criterion is restated here against the
//! Rust producer, one `#[test]` each, tagged at criterion level.
//!
//! # The oracle is the committed evidence, not a fixture written here
//!
//! `spec/evidence/github-actions/` holds the definition and the three exports
//! of one real release run — captured from GitHub, outside Quoin — and
//! `spec/evidence/operational/pairs/c1b3….json` holds the pair the retained
//! TypeScript produced from them. The happy path asserts the Rust producer
//! writes those same bytes under that same name. Every refusal case is a
//! **mutation of one of those committed exports**, copied into a temporary
//! repository first: a refusal is then a statement about real evidence that has
//! been changed in one named way, not about a fixture written to be refused.
//! Nothing here writes into this repository.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

#[path = "common/census.rs"]
mod census;
#[path = "common/copy.rs"]
mod copy;
#[path = "common/paths.rs"]
mod paths;

use census::rust_files;
use copy::copy_tree;
use paths::repo_root;

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

/// The pair the retained TypeScript produced from the retained exports.
const RETAINED_PAIR: &str = "c1b30a188d4d03bbe316e0fdb7582eff3fa314c55268a5f71f81f99f8ea2acf8.json";

/// The definition that produced it, under `spec/evidence/`.
const DEFINITION: &str = "quoin-271-release-v0.22.5-definition.json";

/// The retained workflow export, under `spec/evidence/github-actions/`.
const WORKFLOW: &str = "quoin-271-release-v0.22.5-workflow.yml";

/// The retained workflow-run export.
const RUN: &str = "quoin-271-release-v0.22.5-run.json";

/// The retained workflow-jobs export.
const JOBS: &str = "quoin-271-release-v0.22.5-jobs.json";

/// A temporary repository carrying the committed governance and exports.
fn temporary_repository() -> tempfile::TempDir {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let root = temporary.path();
    copy_tree(
        &repo_root().join("spec").join("assurance"),
        &root.join("spec").join("assurance"),
    );
    copy_tree(
        &repo_root()
            .join("spec")
            .join("evidence")
            .join("github-actions"),
        &root.join("spec").join("evidence").join("github-actions"),
    );
    temporary
}

fn github_actions(root: &Path) -> PathBuf {
    root.join("spec").join("evidence").join("github-actions")
}

/// The committed producer definition, as a document.
fn definition_document() -> Value {
    let path = repo_root()
        .join("spec")
        .join("evidence")
        .join("github-actions")
        .join(DEFINITION);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    serde_json::from_slice(&bytes).expect("the committed definition is JSON")
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
        .unwrap_or_else(|| panic!("{pointer} is present in the committed evidence"));
    *slot = value;
    out
}

/// Remove `key` from the object at `pointer` (`""` for the document root).
fn remove(base: &Value, pointer: &str, key: &str) -> Value {
    let mut out = base.clone();
    let slot = out
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("{pointer} is present in the committed evidence"));
    slot.as_object_mut()
        .unwrap_or_else(|| panic!("{pointer} is an object"))
        .remove(key)
        .unwrap_or_else(|| panic!("{pointer}/{key} is present"));
    out
}

/// The index of the configured release job in the committed jobs export.
fn publish_index(jobs: &Value) -> usize {
    jobs["jobs"]
        .as_array()
        .expect("the jobs export lists jobs")
        .iter()
        .position(|job| job["name"] == json!("Publish"))
        .expect("the committed export carries the Publish job")
}

/// Produce into a workspace from the committed definition.
fn produce(root: &Path) -> Result<GitHubReleaseOperational, GitHubReleaseError> {
    produce_github_release_operational(root, &SystemClock, &definition_of(&definition_document()))
}

/// The obligation a release run is against.
fn obligation() -> OperationalObligation {
    serde_json::from_value(json!({
        "control_kind": "release",
        "subject": {
            "id": "agent-ix/quoin",
            "revision": "a9808be18b61f8e4d44e3b74de27e90f17c5c76b"
        },
        "scope": {
            "service": "agent-ix/quoin",
            "environment": "npm-production",
            "population": "v0.22.5 package release"
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

/// Quoin's retained release workflow and a real completed workflow-run/jobs
/// export produce one linked standing-capability and exercise pair whose
/// workflow, revision, actor, timing, outcome, and observations match the
/// source artifacts.
///
/// Trace: FR-061-AC-1
/// Provenance: quoin#479
#[test]
fn tc_479_450_the_retained_release_evidence_persists_one_exact_linked_pair() {
    let workspace = temporary_repository();
    let root = workspace.path();
    let produced = produce(root).expect("the retained release evidence produces a pair");

    assert_eq!(
        produced.path.file_name().and_then(|name| name.to_str()),
        Some(RETAINED_PAIR),
        "the pair is not named by the two identities it carries"
    );
    // The committed pair is what the retained TypeScript wrote from these same
    // four files. Byte equality is the parity statement.
    let committed = repo_root()
        .join("spec")
        .join("evidence")
        .join("operational")
        .join("pairs")
        .join(RETAINED_PAIR);
    assert_eq!(
        std::fs::read(&produced.path).expect("the produced pair is readable"),
        std::fs::read(&committed).expect("the committed pair is readable"),
        "the produced pair does not carry the retained bytes"
    );

    let capability = &produced.capability;
    assert_eq!(
        capability.base.record_id.as_str(),
        "quoin-271-release-v0.22.5-capability"
    );
    assert_eq!(
        capability.capability.surface,
        ".github/workflows/release.yml"
    );
    assert_eq!(
        capability.base.subject.revision.as_str(),
        "a9808be18b61f8e4d44e3b74de27e90f17c5c76b"
    );

    let exercise = &produced.exercise.exercise;
    assert_eq!(
        produced.exercise.base.record_id.as_str(),
        "quoin-271-release-v0.22.5-exercise"
    );
    assert_eq!(
        exercise
            .capability_record_id
            .as_ref()
            .expect("the exercise names a capability")
            .as_str(),
        "quoin-271-release-v0.22.5-capability",
        "the pair is not linked"
    );
    assert_eq!(exercise.actor, "kreneskyp");
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
    assert_eq!(
        exercise.state_before["source_revision"],
        json!("a9808be18b61f8e4d44e3b74de27e90f17c5c76b")
    );
    assert_eq!(
        produced.exercise.base.observed_at.as_str(),
        "2026-08-29T23:11:38Z"
    );
    assert_eq!(read_operational_records(root).expect("read back").len(), 2);
}

/// Missing or malformed YAML/JSON, a workflow/path/event/revision mismatch, or
/// an absent, duplicate, unstarted, or incomplete release job is refused
/// without either operational record.
///
/// Trace: FR-061-AC-2
/// Provenance: quoin#479
#[test]
fn tc_479_451_malformed_and_mismatched_inputs_are_refused_without_either_record() {
    /// Below this the case walk is asserting nothing.
    const CASE_FLOOR: usize = 12;

    let cases = refusal_cases();
    assert!(
        cases.len() >= CASE_FLOOR,
        "anti-vacuity floor: {} refusal cases, below {CASE_FLOOR}",
        cases.len()
    );
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
/// committed evidence.
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
            "workflow run path, event, or immutable source revision mismatch",
        ),
        (
            "a run from a different event",
            run_patch("/event", json!("push")),
            "workflow run path, event, or immutable source revision mismatch",
        ),
        (
            "a run at a different source revision",
            run_patch(
                "/head_sha",
                json!("0000000000000000000000000000000000000000"),
            ),
            "workflow run path, event, or immutable source revision mismatch",
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
            "configured release job run, revision, or attempt does not match workflow run",
        ),
        (
            "a release job from another attempt",
            job_patch("/run_attempt", json!(2)),
            "configured release job run, revision, or attempt does not match workflow run",
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
    let committed = repo_root()
        .join("spec")
        .join("evidence")
        .join("operational")
        .join("pairs")
        .join(RETAINED_PAIR);
    assert_eq!(
        std::fs::read(&produced.path).unwrap(),
        std::fs::read(&committed).unwrap(),
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

/// A real release-run integration captures workflow-run and jobs responses
/// outside Quoin, then the Quoin producer consumes the retained files without
/// network or process execution and atomically persists the pair through
/// FR-060.
///
/// Trace: FR-061-AC-5
/// Provenance: quoin#479
#[test]
fn tc_479_454_a_real_run_is_consumed_offline_and_persisted_all_or_nothing() {
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

    /// Below this the census is reading an empty tree and proving nothing.
    const SOURCE_FLOOR: usize = 3;

    // The retained run is a real one, captured outside Quoin: its own export
    // says which run it was and how GitHub concluded it.
    let run = export(&repo_root(), RUN);
    assert_eq!(run["id"], json!(33_280_266_874_u64));
    assert_eq!(
        run["html_url"],
        json!("https://github.com/agent-ix/quoin/actions/runs/33280266874")
    );
    assert_eq!(run["conclusion"], json!("success"));

    let census_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("operational")
        .join("github_release");
    let mut measured = 0_usize;
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
        measured += 1;
    }
    assert!(
        measured >= SOURCE_FLOOR,
        "the census read {measured} producer modules, below the floor of {SOURCE_FLOOR}"
    );

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
    exercise["record_id"] = json!("quoin-271-release-v0.22.5-exercise-unlinked");

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
