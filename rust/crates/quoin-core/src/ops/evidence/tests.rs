// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Unit tests for the `evidence` domain, over an in-memory store.
//!
//! Nothing here touches a disk. The host is a `MemoryEvidence` behind a
//! `RefCell`, which is the same seam `main.rs` fills with `DiskEvidence` — so
//! the refusal, mapping and payload paths are exercised without a temporary
//! directory, and the containment audit stays true of this file too.
//!
//! These are HANDLER tests. They cannot see a swapped match arm in
//! `dispatch.rs`, which is why every operation is ALSO driven by its wire
//! spelling through the real binary in `tests/tc_458_evidence_boundary.rs`
//! (the quoin#447 trap).

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use quoin_evidence::{EvidenceSource, MemoryEvidence};
use serde_json::json;

use super::*;

/// An evidence host over a store held in memory.
///
/// `store_root` answers a fixed absolute path rather than resolving one: the
/// property under test is that a store-relative path gets JOINED to whatever
/// the host reports, and a host that reported a real directory would let a
/// wrong join pass by accident.
struct TestHost {
    store: RefCell<MemoryEvidence>,
    root: PathBuf,
}

impl TestHost {
    fn new(store: MemoryEvidence) -> Self {
        Self {
            store: RefCell::new(store),
            root: PathBuf::from("/somewhere/spec/evidence"),
        }
    }

    fn empty() -> Self {
        Self::new(MemoryEvidence::new())
    }
}

impl EvidenceHost for TestHost {
    fn with_store(
        &self,
        _repo: &Path,
        action: &mut dyn FnMut(&mut dyn EvidenceSource) -> Result<serde_json::Value, CoreError>,
    ) -> Result<serde_json::Value, CoreError> {
        let mut store = self.store.borrow_mut();
        action(&mut *store)
    }

    fn store_root(&self, _repo: &Path) -> PathBuf {
        self.root.clone()
    }
}

/// A host that is never consulted. Handing one of these to an operation and
/// asserting the store is untouched is how "refused before the host" is
/// distinguished from "refused after doing the work".
struct RefusingHost;

impl EvidenceHost for RefusingHost {
    fn with_store(
        &self,
        _repo: &Path,
        _action: &mut dyn FnMut(&mut dyn EvidenceSource) -> Result<serde_json::Value, CoreError>,
    ) -> Result<serde_json::Value, CoreError> {
        panic!("the host was consulted; the bound did not refuse first");
    }

    fn store_root(&self, _repo: &Path) -> PathBuf {
        panic!("the host was consulted; the bound did not refuse first");
    }
}

fn run_record(suite: &str, commit: &str, timestamp: &str) -> String {
    json!({
        "schemaVersion": 1,
        "suite": suite,
        "commit": commit,
        "tool": "vitest 1.0.0",
        "timestamp": timestamp,
        "entries": [{"symbol": "a", "outcome": "pass"}],
    })
    .to_string()
}

/// A string of `size` bytes that is still a legal JSON string value.
fn filler(size: usize) -> String {
    "x".repeat(size)
}

/// Trace: FR-101-AC-1
///
/// The facts the command layer needs before it can build any other request,
/// served from the crate rather than restated in TypeScript.
#[test]
fn store_facts_serves_the_constants_the_command_layer_would_otherwise_restate() {
    let response = store_facts(&json!({})).unwrap();
    let payload = response.payload;
    assert_eq!(payload["mutation_score_metric"], "mutation-score");
    assert_eq!(payload["store_schema_version"], 1);
    assert_eq!(payload["bindings_path"], "bindings.json");
    assert_eq!(payload["baseline_path"], "baseline.json");
    assert_eq!(payload["suites_path"], "suites.md");
    assert_eq!(payload["inspections_path"], "inspections.md");
    let families = payload["collected_families"].as_array().unwrap();
    assert_eq!(families.len(), 2, "{families:?}");
    // Anti-vacuity: a registry that answered an empty list would satisfy every
    // "is each name a string" check ever written against it.
    let adapters = payload["adapter_names"].as_array().unwrap();
    assert!(adapters.len() >= 5, "adapter names: {adapters:?}");
    assert!(adapters.iter().any(|name| name == "junit"), "{adapters:?}");
    let triggers = payload["required_triggers"].as_array().unwrap();
    assert_eq!(triggers.len(), 5, "{triggers:?}");
}

/// Trace: FR-101-AC-1
#[test]
fn store_facts_takes_no_arguments() {
    let error = store_facts(&json!({"repo": "/repo"})).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert_eq!(error.outcome().code(), 3);
}

/// Trace: FR-101-AC-2
///
/// The declared divergence: `quoin_evidence::store::gc` returns STORE-RELATIVE
/// paths because it names no host capability, and the retained `gc()` returned
/// absolute ones. The join is this module's, against the root the host reports.
#[test]
fn tc_458_gc_reports_absolute_paths_by_joining_the_store_root() {
    let store = MemoryEvidence::new()
        .with_file(
            "runs/alpha/aaaaaaaaaaaa.json",
            run_record("alpha", "a".repeat(40).as_str(), "2026-01-01T00:00:00Z"),
        )
        .with_file(
            "runs/alpha/bbbbbbbbbbbb.json",
            run_record("alpha", "b".repeat(40).as_str(), "2026-02-01T00:00:00Z"),
        );
    let host = TestHost::new(store);
    let capabilities = Capabilities::with_evidence(&host);

    let response = gc(
        &json!({"repo": "/somewhere", "dry_run": true}),
        &capabilities,
    )
    .unwrap();
    let deleted = response.payload["deleted"].as_array().unwrap();
    // Anti-vacuity: an empty list would pass any "every path is absolute" check.
    assert_eq!(deleted.len(), 1, "{deleted:?}");
    assert_eq!(
        deleted[0], "/somewhere/spec/evidence/runs/alpha/aaaaaaaaaaaa.json",
        "the store-relative path was reported without the root joined onto it"
    );
}

/// Trace: FR-101-AC-2
#[test]
fn a_dry_run_gc_removes_nothing() {
    let store = MemoryEvidence::new()
        .with_file(
            "runs/alpha/aaaaaaaaaaaa.json",
            run_record("alpha", "a".repeat(40).as_str(), "2026-01-01T00:00:00Z"),
        )
        .with_file(
            "runs/alpha/bbbbbbbbbbbb.json",
            run_record("alpha", "b".repeat(40).as_str(), "2026-02-01T00:00:00Z"),
        );
    let host = TestHost::new(store);
    let capabilities = Capabilities::with_evidence(&host);

    gc(
        &json!({"repo": "/somewhere", "dry_run": true}),
        &capabilities,
    )
    .unwrap();
    assert_eq!(host.store.borrow().paths().len(), 2);

    gc(
        &json!({"repo": "/somewhere", "dry_run": false}),
        &capabilities,
    )
    .unwrap();
    assert_eq!(
        host.store.borrow().paths(),
        vec!["runs/alpha/bbbbbbbbbbbb.json"]
    );
}

/// Trace: FR-101-AC-2
///
/// The second declared divergence. The retained reader handed back a
/// structurally malformed record half-formed; typed deserialization refuses it,
/// skips it and NAMES it. The behaviour change is observable and reported.
#[test]
fn tc_458_a_malformed_record_is_skipped_and_named_rather_than_half_read() {
    let store = MemoryEvidence::new()
        .with_file(
            "runs/alpha/aaaaaaaaaaaa.json",
            run_record("alpha", "a".repeat(40).as_str(), "2026-01-01T00:00:00Z"),
        )
        .with_file(
            "runs/beta/cccccccccccc.json",
            r#"{"schemaVersion":1,"suite":"beta"}"#,
        );
    let host = TestHost::new(store);
    let capabilities = Capabilities::with_evidence(&host);

    let response = audit_inputs(&json!({"repo": "/somewhere"}), &capabilities).unwrap();
    let skipped = response.payload["skipped"].as_array().unwrap();
    assert_eq!(skipped.len(), 1, "{skipped:?}");
    assert!(
        skipped[0].as_str().unwrap().contains("beta"),
        "skipped: {skipped:?}"
    );
    // And the readable record is still there: skipping one is not refusing all.
    let runs = response.payload["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1, "{runs:?}");
    assert_eq!(runs[0]["suite"], "alpha");
}

/// Trace: FR-101-AC-4
///
/// One subprocess answers everything the pure auditor reads, including the two
/// questions it would otherwise ask inside per-obligation loops.
#[test]
fn audit_inputs_answers_vacuity_and_independence_alongside_the_records() {
    let scan = json!({
        "schemaVersion": 1,
        "suite": "scan-suite",
        "commit": "c".repeat(40),
        "tool": "semgrep",
        "timestamp": "2026-01-01T00:00:00Z",
        "rulesEvaluated": 0,
        "findings": [],
    })
    .to_string();
    let host =
        TestHost::new(MemoryEvidence::new().with_file("scans/scan-suite/cccccccccccc.json", scan));
    let capabilities = Capabilities::with_evidence(&host);

    let response = audit_inputs(&json!({"repo": "/somewhere"}), &capabilities).unwrap();
    let vacuous = response.payload["vacuous_scan_suites"].as_array().unwrap();
    assert_eq!(vacuous.len(), 1, "{vacuous:?}");
    assert_eq!(vacuous[0], "scan-suite");
    assert!(
        response.payload["independence"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

/// Trace: FR-101-AC-3
#[test]
fn an_unknown_adapter_is_a_caller_mistake_not_a_refusal() {
    let error = parse_results(&json!({"text": "{}", "adapter": "nosuch"})).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert_eq!(error.outcome().code(), 3);
    assert_eq!(error.context["evidence_code"], "QE-E002");
    assert_eq!(error.context["op"], "evidence.parse_results");
}

/// Trace: FR-101-AC-3
#[test]
fn a_store_that_cannot_be_read_is_refused_not_reported_as_empty() {
    let host = TestHost::new(MemoryEvidence::new().with_file("bindings.json", "<<<<<<< HEAD"));
    let capabilities = Capabilities::with_evidence(&host);
    let error = audit_inputs(&json!({"repo": "/somewhere"}), &capabilities).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.outcome().code(), 2);
    assert_eq!(error.context["evidence_code"], "QE-E003");
}

/// Trace: FR-101-AC-5
///
/// A missing host is this build's fault, not the caller's: Internal (4), never
/// Refused (2). Every host-taking operation is driven, because the mapping is
/// per-operation code.
#[test]
fn an_operation_dispatched_without_a_host_is_an_internal_fault_not_a_refusal() {
    let none = Capabilities::none();
    let cases: Vec<(&str, Result<Response, CoreError>)> = vec![
        ("evidence.gc", gc(&json!({"repo": "/r"}), &none)),
        (
            "evidence.affirm",
            affirm_binding(
                &json!({"repo": "/r", "obligation": "FR-1-AC-1", "statement_hash": "h", "who": "p", "commit": "c"}),
                &none,
            ),
        ),
        (
            "evidence.record",
            record(
                &json!({"repo": "/r", "suite": "s", "commit": "c", "tool": "junit", "timestamp": "t", "results": "<testsuite/>"}),
                &none,
            ),
        ),
        (
            "evidence.trust_assessments",
            trust_assessments(&json!({"repo": "/r"}), &none),
        ),
        (
            "evidence.inspect_mocks",
            inspect_mocks(
                &json!({"repo": "/r", "suite": "s", "commit": "c", "tool": "t", "timestamp": "ts"}),
                &none,
            ),
        ),
        (
            "evidence.audit_inputs",
            audit_inputs(&json!({"repo": "/r"}), &none),
        ),
        (
            "evidence.read_baseline",
            read_baseline(&json!({"repo": "/r"}), &none),
        ),
        (
            "evidence.write_baseline",
            write_baseline(&json!({"repo": "/r", "commit": "c", "accepted": []}), &none),
        ),
        (
            "evidence.record_experiment",
            record_experiment(&json!({"repo": "/r", "document": {}}), &none),
        ),
        (
            "evidence.record_operational",
            record_operational(&json!({"repo": "/r", "document": {}}), &none),
        ),
    ];
    assert!(cases.len() >= 10, "the census shrank: {}", cases.len());
    for (op, outcome) in cases {
        let error = outcome.unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Io, "{op}");
        assert_eq!(error.outcome().code(), 4, "{op}");
        assert_eq!(error.context["op"], op);
    }
}

/// Trace: FR-101-AC-6
///
/// Every bound refuses BEFORE the host is consulted. The host panics if it is
/// reached, so a bound applied after the store was opened fails here rather
/// than passing with a remark.
#[test]
fn tc_412_each_bound_refuses_before_the_host_is_consulted() {
    let host = RefusingHost;
    let capabilities = Capabilities::with_evidence(&host);
    let cases: Vec<(&str, usize, Result<Response, CoreError>)> = vec![
        (
            "evidence.gc",
            MAX_GC_BYTES,
            gc(&json!({"repo": filler(MAX_GC_BYTES)}), &capabilities),
        ),
        (
            "evidence.read_baseline",
            MAX_READ_BASELINE_BYTES,
            read_baseline(
                &json!({"repo": filler(MAX_READ_BASELINE_BYTES)}),
                &capabilities,
            ),
        ),
        (
            "evidence.trust_assessments",
            MAX_TRUST_ASSESSMENTS_BYTES,
            trust_assessments(
                &json!({"repo": filler(MAX_TRUST_ASSESSMENTS_BYTES)}),
                &capabilities,
            ),
        ),
        (
            "evidence.inspect_mocks",
            MAX_INSPECT_MOCKS_BYTES,
            inspect_mocks(
                &json!({"repo": filler(MAX_INSPECT_MOCKS_BYTES)}),
                &capabilities,
            ),
        ),
        (
            "evidence.affirm",
            MAX_AFFIRM_BYTES,
            affirm_binding(&json!({"repo": filler(MAX_AFFIRM_BYTES)}), &capabilities),
        ),
        (
            "evidence.audit_inputs",
            MAX_AUDIT_INPUTS_BYTES,
            audit_inputs(
                &json!({"repo": filler(MAX_AUDIT_INPUTS_BYTES)}),
                &capabilities,
            ),
        ),
    ];
    assert!(cases.len() >= 6, "the census shrank: {}", cases.len());
    for (op, limit, outcome) in cases {
        let error = outcome.unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Refused, "{op}");
        assert_eq!(error.outcome().code(), 2, "{op}");
        assert_eq!(error.context["op"], op);
        assert_eq!(error.context["limit_bytes"], limit.to_string(), "{op}");
    }
}

/// Trace: FR-101-AC-6
///
/// A scalar field is measured on its own, so a field that fits inside the
/// request's ceiling but is absurd on its own is still refused before any work.
#[test]
fn an_oversized_scalar_is_refused_before_the_host_is_consulted() {
    let host = RefusingHost;
    let capabilities = Capabilities::with_evidence(&host);
    let error = audit_inputs(
        &json!({"repo": "/r", "head_commit": filler(MAX_SCALAR_BYTES + 1)}),
        &capabilities,
    )
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.context["field"], "head_commit");
    assert_eq!(error.context["limit_bytes"], MAX_SCALAR_BYTES.to_string());
}

/// Trace: FR-101-AC-4
///
/// Nothing is written when no binding matched: an unmatched affirmation must
/// not rewrite `bindings.json` with identical content and a fresh mtime.
#[test]
fn an_affirmation_that_matches_nothing_writes_nothing() {
    let host = TestHost::empty();
    let capabilities = Capabilities::with_evidence(&host);
    let response = affirm_binding(
        &json!({
            "repo": "/somewhere",
            "obligation": "FR-1-AC-1",
            "statement_hash": "abc",
            "who": "peter",
            "commit": "c".repeat(40),
        }),
        &capabilities,
    )
    .unwrap();
    assert_eq!(response.payload["found"], false);
    assert!(
        host.store.borrow().paths().is_empty(),
        "an unmatched affirmation touched the store: {:?}",
        host.store.borrow().paths()
    );
}

/// Trace: FR-101-AC-4
#[test]
fn a_run_shaped_record_writes_a_run_and_a_finding_shaped_one_writes_a_scan() {
    let host = TestHost::empty();
    let capabilities = Capabilities::with_evidence(&host);

    let response = record(
        &json!({
            "repo": "/somewhere",
            "suite": "unit",
            "commit": "a".repeat(40),
            "tool": "vitest 1.0.0",
            "timestamp": "2026-01-01T00:00:00Z",
            "adapter": "junit",
            "results": r#"<testsuite name="unit"><testcase name="works"/></testsuite>"#,
        }),
        &capabilities,
    )
    .unwrap();
    assert!(
        response.payload.get("run_path").is_some(),
        "{:?}",
        response.payload
    );

    let response = record(
        &json!({
            "repo": "/somewhere",
            "suite": "scan",
            "commit": "b".repeat(40),
            "tool": "sarif",
            "timestamp": "2026-01-01T00:00:00Z",
            "adapter": "sarif",
            "results": json!({
                "version": "2.1.0",
                "runs": [{
                    "tool": {"driver": {"name": "sarif", "rules": [{"id": "R1"}]}},
                    "results": [],
                }],
            })
            .to_string(),
        }),
        &capabilities,
    )
    .unwrap();
    assert!(
        response.payload.get("scan_path").is_some(),
        "a finding-shaped adapter fell through to the run path: {:?}",
        response.payload
    );
}

/// Trace: FR-101-AC-4
#[test]
fn write_baseline_reports_an_absolute_path() {
    let host = TestHost::empty();
    let capabilities = Capabilities::with_evidence(&host);
    let response = write_baseline(
        &json!({"repo": "/somewhere", "commit": "a".repeat(40), "accepted": ["b", "a"]}),
        &capabilities,
    )
    .unwrap();
    assert_eq!(
        response.payload["path"],
        "/somewhere/spec/evidence/baseline.json"
    );

    let response = read_baseline(&json!({"repo": "/somewhere"}), &capabilities).unwrap();
    // Sorted by the store, not by the caller.
    assert_eq!(response.payload["baseline"]["accepted"], json!(["a", "b"]));
}

/// Trace: FR-101-AC-3
#[test]
fn a_policy_naming_an_underived_obligation_is_refused_with_its_own_code() {
    let policy = json!({
        "schemaVersion": 1,
        "profile": "AP-001",
        "requirements": [{
            "id": "IR-1",
            "obligation": "FR-999-AC-1",
            "dimensions": ["actor"],
            "rationale": "because",
        }],
    })
    .to_string();
    let error =
        parse_policy(&json!({"text": policy, "known_obligations": ["FR-1-AC-1"]})).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert_eq!(error.context["evidence_code"], "QE-E007");
}

/// Trace: FR-101-AC-4
///
/// The assurance family and its wire spelling round trip, both ways.
///
/// An anti-vacuity floor on the census: two DISTINCT spellings, and neither is
/// accepted for the other. A table that answered one spelling for both
/// variants would satisfy `from_op(f.op()) == Some(f)` for whichever it named
/// and fail here.
#[test]
fn tc_458_the_assurance_family_round_trips_through_its_wire_spelling() {
    use super::records::AssuranceFamily;

    let families = [AssuranceFamily::Experiment, AssuranceFamily::Operational];
    for family in families {
        assert_eq!(AssuranceFamily::from_op(family.op()), Some(family));
    }
    assert_eq!(families[0].op(), "evidence.record_experiment");
    assert_eq!(families[1].op(), "evidence.record_operational");
    assert_ne!(families[0].op(), families[1].op());
    assert_eq!(AssuranceFamily::from_op("evidence.record"), None);
}
