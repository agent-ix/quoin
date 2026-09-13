// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `evidence` operations, reached by their wire names (quoin#458).
//!
//! `tc_447_600_every_operation_is_invoked_by_name_by_some_test` requires every
//! entry in `OPERATIONS` to be handed to the REAL BINARY, spelled, by some
//! integration test — because a swapped match arm leaves the drift guard, the
//! unit tests and `quoin-difftest` all green and breaks only for a user. A unit
//! test that calls the handler function directly cannot see that swap at all.
//!
//! So each test below asserts a field **only its own handler writes**. Where
//! two operations would otherwise answer the same shape (`evidence.record` and
//! `evidence.record_experiment` both report a `path`, say), the assertion is on
//! the discriminating key, never on the shared one.
//!
//! This file is also the one place the whole crossing runs end to end: a real
//! evidence store on a real temporary repository, a real subprocess, canonical
//! JSON on stdout and the exit taxonomy on the status.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

/// Invoke the binary with `op` on argv and `request` on stdin.
fn run(op: &str, request: &Value) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_quoin-core"))
        .arg(op)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    Run {
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
        status: out.status.code().unwrap(),
    }
}

/// The payload of a run that must have succeeded outright.
fn ok(result: &Run) -> Value {
    assert_eq!(result.status, 0, "{}", result.stderr);
    assert_eq!(result.stderr, "");
    serde_json::from_str(&result.stdout).unwrap()
}

/// A temporary repository with an evidence store under `spec/evidence`.
fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("spec/evidence")).unwrap();
    dir
}

fn write_store(root: &Path, relative: &str, text: &str) {
    let path = root.join("spec/evidence").join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn store_path(root: &Path, relative: &str) -> PathBuf {
    root.join("spec/evidence").join(relative)
}

/// One payload path, checked to be the absolute path the contract says it is.
///
/// Every path an `evidence.*` payload carries is absolute — the commands print
/// them for a person to open, and `quoin_evidence` returns store-relative ones.
/// Asserted as an equality against the store root rather than by handing the
/// reported string to `is_file()`, which a relative path would also satisfy
/// whenever the test process happened to be running in the repository.
fn reported_store_path(root: &Path, reported: &str, relative: &str) -> PathBuf {
    let expected = store_path(root, relative);
    assert_eq!(
        Path::new(reported),
        expected,
        "the payload path must be absolute and inside the store"
    );
    assert!(
        expected.is_file(),
        "the payload named a record the store does not hold"
    );
    expected
}

/// As [`reported_store_path`], where only the record's FAMILY is predictable —
/// a content-addressed record's file name is derived from its own bytes.
fn reported_store_family(root: &Path, reported: &str, family: &str) -> PathBuf {
    let prefix = store_path(root, family);
    let path = Path::new(reported);
    assert!(
        path.starts_with(&prefix),
        "{reported} is not an absolute path under {}",
        prefix.display()
    );
    assert!(
        path.is_file(),
        "the payload named a record the store does not hold"
    );
    path.to_path_buf()
}

fn commit(seed: char) -> String {
    std::iter::repeat_n(seed, 40).collect()
}

fn junit(case: &str) -> String {
    format!(r#"<testsuite name="unit"><testcase name="{case}"/></testsuite>"#)
}

/// Trace: FR-101-AC-1
///
/// `store_facts` is the only operation that answers `mutation_score_metric`,
/// and the only one that takes no arguments at all.
#[test]
fn tc_458_010_store_facts_serves_the_store_constants() {
    let payload = ok(&run("evidence.store_facts", &json!({})));
    assert_eq!(payload["mutation_score_metric"], "mutation-score");
    assert_eq!(payload["bindings_path"], "bindings.json");
    assert!(
        payload["adapter_names"].as_array().unwrap().len() >= 5,
        "{payload}"
    );
}

/// Trace: FR-101-AC-3
///
/// `parse_results` is the only operation that answers a bare `entries` list
/// with no store path beside it.
#[test]
fn tc_458_020_parse_results_transcribes_a_producer_document() {
    let payload = ok(&run(
        "evidence.parse_results",
        &json!({"text": junit("works"), "adapter": "junit"}),
    ));
    assert_eq!(payload["kind"], "run");
    let entries = payload["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "{payload}");
    assert_eq!(entries[0]["outcome"], "pass");
    assert!(payload.get("run_path").is_none(), "{payload}");
}

/// Trace: FR-101-AC-3
#[test]
fn tc_458_030_parse_lineage_reads_a_lineage_document() {
    let payload = ok(&run(
        "evidence.parse_lineage",
        &json!({"text": json!({"actor": "peter", "technique": "unit"}).to_string()}),
    ));
    assert_eq!(payload["lineage"]["actor"], "peter");
}

/// Trace: FR-101-AC-3
#[test]
fn tc_458_040_parse_policy_reads_a_policy_and_checks_its_obligations() {
    let policy = json!({
        "schemaVersion": 1,
        "profile": "AP-001",
        "requirements": [{
            "id": "IR-1",
            "obligation": "FR-1-AC-1",
            "dimensions": ["actor"],
            "rationale": "two lines",
        }],
    })
    .to_string();
    let payload = ok(&run(
        "evidence.parse_policy",
        &json!({"text": policy, "known_obligations": ["FR-1-AC-1"]}),
    ));
    assert_eq!(payload["policy"]["profile"], "AP-001");

    // And the check is not decorative.
    let refused = run(
        "evidence.parse_policy",
        &json!({"text": policy, "known_obligations": ["FR-2-AC-1"]}),
    );
    assert_eq!(refused.status, 3, "{}", refused.stdout);
    assert!(refused.stderr.contains("QE-E007"), "{}", refused.stderr);
}

/// Trace: FR-101-AC-4
///
/// `record` is the only operation that answers `run_path`.
#[test]
fn tc_458_050_record_writes_a_run_record_through_the_real_store() {
    let dir = repo();
    let payload = ok(&run(
        "evidence.record",
        &json!({
            "repo": dir.path(),
            "suite": "unit",
            "commit": commit('a'),
            "tool": "vitest 1.0.0",
            "timestamp": "2026-01-01T00:00:00Z",
            "adapter": "junit",
            "results": junit("works"),
        }),
    ));
    assert_eq!(payload["kind"], "run");
    let run_path = payload["run_path"].as_str().unwrap();
    reported_store_path(dir.path(), run_path, "runs/unit/aaaaaaaaaaaa.json");
}

/// Trace: FR-101-AC-4
///
/// `affirm` is the only operation that answers `found`.
#[test]
fn tc_458_060_affirm_clears_suspicion_on_a_binding_that_exists() {
    let dir = repo();
    write_store(
        dir.path(),
        "bindings.json",
        &json!({
            "schemaVersion": 1,
            "bindings": [{
                "obligation": "FR-1-AC-1",
                "statementHashAtBinding": "old",
                "suite": "unit",
                "commit": commit('a'),
                "symbols": [],
            }],
        })
        .to_string(),
    );
    let payload = ok(&run(
        "evidence.affirm",
        &json!({
            "repo": dir.path(),
            "obligation": "FR-1-AC-1",
            "statement_hash": "new",
            "who": "peter",
            "commit": commit('b'),
        }),
    ));
    assert_eq!(payload["found"], true);
    let written: Value =
        serde_json::from_str(&fs::read_to_string(store_path(dir.path(), "bindings.json")).unwrap())
            .unwrap();
    assert_eq!(
        written["bindings"][0]["affirmations"][0]["who"], "peter",
        "{written}"
    );
}

/// Trace: FR-101-AC-2
///
/// `gc` is the only operation that answers `deleted` — and the divergence this
/// ticket declares is asserted HERE, through the binary, because the join onto
/// the store root is what makes the path the caller prints a real one.
#[test]
fn tc_458_070_gc_reports_absolute_paths_and_removes_only_the_superseded() {
    let dir = repo();
    for (seed, timestamp) in [('a', "2026-01-01T00:00:00Z"), ('b', "2026-02-01T00:00:00Z")] {
        ok(&run(
            "evidence.record",
            &json!({
                "repo": dir.path(),
                "suite": "unit",
                "commit": commit(seed),
                "tool": "vitest 1.0.0",
                "timestamp": timestamp,
                "adapter": "junit",
                "results": junit("works"),
            }),
        ));
    }
    let payload = ok(&run(
        "evidence.gc",
        &json!({"repo": dir.path(), "dry_run": false}),
    ));
    let deleted = payload["deleted"].as_array().unwrap();
    assert_eq!(deleted.len(), 1, "{payload}");
    let expected = store_path(dir.path(), "runs/unit/aaaaaaaaaaaa.json");
    assert_eq!(
        deleted[0].as_str().unwrap(),
        expected.to_string_lossy(),
        "a store-relative path reached the caller"
    );
    assert!(!expected.exists());
    assert!(store_path(dir.path(), "runs/unit/bbbbbbbbbbbb.json").is_file());
}

/// Trace: FR-101-AC-4
///
/// `trust_decision` is the only operation that answers both a `path` and an
/// `assessment`.
#[test]
fn tc_458_080_trust_decision_is_assessed_before_it_is_stored() {
    let dir = repo();
    let context = json!({
        "name": "ripgrep",
        "version": "14.0.0",
        "configurationDigest": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        "validationCorpusDigest": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
        "inputContract": "utf-8 text",
        "environment": "linux-x86_64",
    });
    let decision = json!({
        "schemaVersion": 1,
        "id": "ETD-001",
        "use": {
            "id": "TU-001",
            "intendedFunction": "find literal matches in the corpus",
            "permittedDecisions": ["a finding is reported"],
        },
        "decision": "relied-upon",
        "acceptedContext": context,
        "revalidateOn": [
            "producer-version",
            "configuration",
            "validation-corpus",
            "input-contract",
            "environment",
        ],
        "validationEvidence": [{"id": "EV-1", "digest": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}],
        "limitations": [],
        "owner": "peter",
        "decidedAt": "2026-01-01T00:00:00Z",
    });
    let result = run(
        "evidence.trust_decision",
        &json!({"repo": dir.path(), "decision": decision}),
    );
    // The shape of a trust decision is `quoin-evidence`'s to define; this test
    // is about the ROUTE, so a refusal is reported with its code rather than
    // silently accepted as "some answer came back".
    assert_eq!(result.status, 0, "{} / {}", result.status, result.stderr);
    let payload = ok(&result);
    assert!(payload.get("assessment").is_some(), "{payload}");
    let path = payload["path"].as_str().unwrap();
    reported_store_family(dir.path(), path, "trust");
}

/// Trace: FR-101-AC-4
///
/// `trust_assessments` is the only operation that answers `unreadable`.
#[test]
fn tc_458_090_trust_assessments_reads_an_empty_store_as_empty() {
    let dir = repo();
    let payload = ok(&run(
        "evidence.trust_assessments",
        &json!({"repo": dir.path()}),
    ));
    assert_eq!(payload["assessments"], json!([]));
    assert_eq!(payload["unreadable"], json!([]));
}

/// Trace: FR-101-AC-4
///
/// `inspect_mocks` is the only operation that answers `injections`. An empty
/// COMPLETED inspection is still recorded — that is how audit tells "looked and
/// found none" from "nobody looked" (agent-ix/quoin#204).
#[test]
fn tc_458_100_inspect_mocks_records_an_empty_inspection() {
    let dir = repo();
    let payload = ok(&run(
        "evidence.inspect_mocks",
        &json!({
            "repo": dir.path(),
            "suite": "unit",
            "commit": commit('a'),
            "tool": "grep",
            "timestamp": "2026-01-01T00:00:00Z",
            "dry_run": false,
        }),
    ));
    assert_eq!(payload["injections"], json!([]));
    let path = payload["path"].as_str().unwrap();
    reported_store_path(dir.path(), path, "mock-inspections/unit/aaaaaaaaaaaa.json");
}

/// Trace: FR-101-AC-4
///
/// `record_experiment` and `record_operational` are the only operations that
/// answer `created`, and they write to different families — which is the fact
/// that tells one from the other if an arm were swapped.
#[test]
fn tc_458_110_the_two_assurance_record_operations_write_different_families() {
    let dir = repo();
    let provenance = json!({
        "schemaVersion": "producer-provenance-v1",
        "identity": "quoin",
        "version": "0.1.0",
        "sourceRevision": commit('a'),
        "sourceState": "clean",
        "executableDigest": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        "configurationDigest": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
        "capabilities": ["record"],
        "artifacts": [{
            "name": "report.json",
            "digest": "sha256:3333333333333333333333333333333333333333333333333333333333333333",
        }],
    });

    let payload = ok(&run(
        "evidence.record_experiment",
        &json!({
            "repo": dir.path(),
            "document": {
                "schemaVersion": "experiment-record-v1",
                "subject": {"kind": "requirement", "id": "FR-001", "sourceRevision": commit('b')},
                "recordedAt": "2026-01-01T00:00:00Z",
                "hypothesis": "the adapter reads real output",
                "design": {
                    "timeBox": "1d",
                    "corpusRefs": ["corpus/a"],
                    "comparisonMethod": "A/B",
                    "decisionRule": "p<0.05",
                },
                "result": {
                    "status": "supported",
                    "summary": "it did",
                    "evidenceRefs": ["runs/SUITE-A"],
                },
                "producerProvenance": provenance,
            },
        }),
    ));
    assert_eq!(payload["created"], true);
    let experiment_path = payload["path"].as_str().unwrap().to_owned();
    reported_store_family(dir.path(), &experiment_path, "experiments");

    // Append-only: publishing the same document again finds identical bytes
    // and says so, rather than reporting a second creation.
    let payload = ok(&run(
        "evidence.record_experiment",
        &json!({
            "repo": dir.path(),
            "document": {
                "schemaVersion": "experiment-record-v1",
                "subject": {"kind": "requirement", "id": "FR-001", "sourceRevision": commit('b')},
                "recordedAt": "2026-01-01T00:00:00Z",
                "hypothesis": "the adapter reads real output",
                "design": {
                    "timeBox": "1d",
                    "corpusRefs": ["corpus/a"],
                    "comparisonMethod": "A/B",
                    "decisionRule": "p<0.05",
                },
                "result": {
                    "status": "supported",
                    "summary": "it did",
                    "evidenceRefs": ["runs/SUITE-A"],
                },
                "producerProvenance": provenance,
            },
        }),
    ));
    assert_eq!(payload["created"], false);
    assert_eq!(payload["path"].as_str().unwrap(), experiment_path);

    let payload = ok(&run(
        "evidence.record_operational",
        &json!({
            "repo": dir.path(),
            "document": {
                "schemaVersion": "operational-evidence-record-v1",
                "subject": {"kind": "service", "id": "quoin-core", "sourceRevision": commit('c')},
                "recordedAt": "2026-01-01T00:00:00Z",
                "window": {
                    "startedAt": "2026-01-01T00:00:00Z",
                    "endedAt": "2026-01-02T00:00:00Z",
                },
                "environment": "production",
                "observations": [{
                    "signal": "latency",
                    "value": "12",
                    "unit": "ms",
                    "interpretation": "within bounds",
                    "evidenceRefs": ["metrics/1"],
                }],
                "outcome": "within_bounds",
                "producerProvenance": provenance,
            },
        }),
    ));
    // The discriminating fact: the same payload SHAPE, a different family. An
    // arm routed to the sibling handler would refuse the document outright, and
    // one that somehow accepted it would file it under `experiments/`.
    reported_store_family(dir.path(), payload["path"].as_str().unwrap(), "operational");
    assert_eq!(payload["created"], true);
}

/// Trace: FR-101-AC-4
///
/// `audit_inputs` is the only operation that answers `vacuous_scan_suites`.
#[test]
fn tc_458_120_audit_inputs_answers_everything_the_pure_auditor_reads() {
    let dir = repo();
    ok(&run(
        "evidence.record",
        &json!({
            "repo": dir.path(),
            "suite": "unit",
            "commit": commit('a'),
            "tool": "vitest 1.0.0",
            "timestamp": "2026-01-01T00:00:00Z",
            "adapter": "junit",
            "results": junit("works"),
        }),
    ));
    let payload = ok(&run("evidence.audit_inputs", &json!({"repo": dir.path()})));
    assert_eq!(payload["runs"].as_array().unwrap().len(), 1, "{payload}");
    assert_eq!(payload["vacuous_scan_suites"], json!([]));
    assert_eq!(payload["skipped"], json!([]));
    assert_eq!(payload["independence"], json!([]));
}

/// Trace: FR-101-AC-4
///
/// `write_baseline` is the only operation that answers a bare `path`, and
/// `read_baseline` the only one that answers `baseline`. Driven together
/// because the round trip is the property: what one writes the other reads.
#[test]
fn tc_458_130_the_baseline_round_trips_through_the_store() {
    let dir = repo();
    let payload = ok(&run(
        "evidence.write_baseline",
        &json!({"repo": dir.path(), "commit": commit('a'), "accepted": ["b", "a"]}),
    ));
    assert_eq!(
        payload["path"].as_str().unwrap(),
        store_path(dir.path(), "baseline.json").to_string_lossy()
    );

    let payload = ok(&run("evidence.read_baseline", &json!({"repo": dir.path()})));
    assert_eq!(payload["baseline"]["accepted"], json!(["a", "b"]));
    assert_eq!(payload["baseline"]["commit"], commit('a'));
}

/// Trace: FR-101-AC-3
///
/// The exit taxonomy, through the binary: a caller mistake is 3, a refusal is
/// 2, and neither writes a payload to stdout.
#[test]
fn tc_458_140_the_exit_taxonomy_reaches_the_status() {
    let bad = run(
        "evidence.parse_results",
        &json!({"text": "", "adapter": "nope"}),
    );
    assert_eq!(bad.status, 3, "{}", bad.stderr);
    assert_eq!(bad.stdout, "");
    assert!(bad.stderr.contains("QE-E002"), "{}", bad.stderr);

    let dir = repo();
    write_store(dir.path(), "bindings.json", "<<<<<<< HEAD");
    let refused = run("evidence.audit_inputs", &json!({"repo": dir.path()}));
    assert_eq!(refused.status, 2, "{}", refused.stderr);
    assert_eq!(refused.stdout, "");
    assert!(refused.stderr.contains("QE-E003"), "{}", refused.stderr);
}

/// Trace: FR-034-AC-10, FR-034-AC-11
///
/// The half of FR-034's vacuity rule that moved into `quoin-evidence`: which
/// suites' newest scan evaluated no rules. The auditor no longer asks the
/// question per obligation — it is handed the answer as `vacuous_scan_suites`
/// — so the rule `rulesEvaluated == 0` is stated here and what the auditor
/// does with it stays on `tests/finding-record.test.ts`.
///
/// Both halves of the distinction in one store, because the defect this guards
/// is collapsing them: a scan that declared zero rules is vacuous, and a scan
/// whose tool reported no count at all is a question that cannot be asked, so
/// it must be absent from the list rather than in it or assumed clean.
#[test]
fn tc_458_150_audit_inputs_names_the_scans_that_evaluated_no_rules() {
    let dir = repo();
    let scan = |suite: &str, driver: &str| {
        ok(&run(
            "evidence.record",
            &json!({
                "repo": dir.path(),
                "suite": suite,
                "commit": commit('a'),
                "tool": "semgrep 1.2.3",
                "timestamp": "2026-01-01T00:00:00Z",
                "adapter": "sarif",
                "results": format!(
                    r#"{{"runs":[{{"tool":{{"driver":{driver}}},"results":[]}}]}}"#
                ),
            }),
        ))
    };
    // `rules: []` — the tool said it evaluated zero rules.
    scan("SUITE-VACUOUS", r#"{"name":"semgrep","rules":[]}"#);
    // No `rules` key — the tool said nothing about how many it evaluated.
    scan("SUITE-SILENT", r#"{"name":"semgrep"}"#);

    let payload = ok(&run("evidence.audit_inputs", &json!({"repo": dir.path()})));
    assert_eq!(payload["scans"].as_array().unwrap().len(), 2, "{payload}");
    assert_eq!(payload["vacuous_scan_suites"], json!(["SUITE-VACUOUS"]));
}

/// Trace: FR-101-AC-2
///
/// The second declared divergence, through the real binary.
///
/// The retained TypeScript reader handed a structurally malformed record back
/// half-formed — whatever fields happened to parse, with the rest absent — so a
/// run with no `entries` reached the auditor as a run that discharged nothing.
/// Typed deserialization refuses it. The behaviour change is observable, so it
/// is REPORTED: the record is skipped and its path is named in `skipped`.
///
/// Driven here as well as in the handler unit test because what a user can act
/// on is the payload the binary writes, and `skipped` is the whole point of the
/// divergence: an unreadable record must not become a silent absence.
#[test]
fn tc_458_160_a_malformed_record_is_skipped_and_named_through_the_binary() {
    let dir = repo();
    ok(&run(
        "evidence.record",
        &json!({
            "repo": dir.path(),
            "suite": "readable",
            "commit": commit('a'),
            "tool": "vitest 1.0.0",
            "timestamp": "2026-01-01T00:00:00Z",
            "adapter": "junit",
            "results": junit("works"),
        }),
    ));
    // Well-formed JSON, and not a run record: no `entries`, no `commit`.
    write_store(
        dir.path(),
        "runs/broken/cccccccccccc.json",
        r#"{"schemaVersion":1,"suite":"broken"}"#,
    );

    let payload = ok(&run("evidence.audit_inputs", &json!({"repo": dir.path()})));
    let skipped = payload["skipped"].as_array().unwrap();
    assert_eq!(skipped.len(), 1, "{payload}");
    assert!(skipped[0].as_str().unwrap().contains("broken"), "{payload}");
    // Skipping one record is not refusing the store: the readable run survives.
    assert_eq!(payload["runs"].as_array().unwrap().len(), 1, "{payload}");
    assert_eq!(payload["runs"][0]["suite"], "readable");
}
