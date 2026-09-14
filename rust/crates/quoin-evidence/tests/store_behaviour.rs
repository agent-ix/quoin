// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The FR-030/FR-031/FR-048/FR-093/FR-094 criteria the retained TypeScript
//! suites state, restated against the Rust port (quoin#456).
//!
//! These are behaviour assertions, not golden comparisons — `store_parity.rs`
//! carries the byte-level agreement with the oracle. What is restated here is
//! what each criterion *says*, so the port is gated on the criterion and not
//! only on the previous implementation's output.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use quoin_evidence::assurance_records::{
    read_experiment_record, write_experiment_record, write_operational_evidence_record,
};
use quoin_evidence::independence::{
    assess_independence, require_known_policy_obligations, validate_independence_policy,
};
use quoin_evidence::mock_inspection::{inspect_mock_injections, mock_inspection_input};
use quoin_evidence::paths::{bindings_path, run_path, scan_path, trust_decision_path};
use quoin_evidence::record::{RecordRequest, record_run};
use quoin_evidence::store::{
    affirm, gc, list_recorded_suites, read_bindings, read_runs, read_trust_decisions,
    write_bindings, write_mock_inspection, write_run, write_trust_decision,
};
use quoin_evidence::trust::{assess_trust, validate_trust_decision};
use quoin_evidence::types::{
    Affirmation, Binding, EvidenceLineage, IndependenceDimension, IndependencePolicy,
    IndependenceRequirement, IndependenceStatus, MockInspectionRecord, Outcome, ProducerContext,
    RunEntry, RunRecord, STORE_SCHEMA_VERSION, TrustAdapter, TrustDecision, TrustDecisionKind,
    TrustEvidenceReference, TrustStatus, TrustTrigger, TrustUse,
};
use quoin_evidence::{
    Commit, DiskEvidence, EvidenceSource, MemoryEvidence, ObligationId, ProfileId, StatementHash,
    SuiteId, SymbolId, TrustDecisionId,
};
use quoin_quire_types::Obligation;
use serde_json::json;

const COMMIT: &str = "aaaaaaaaaaaa0000";
const OTHER_COMMIT: &str = "bbbbbbbbbbbb0000";
const HASH_A: &str = "sha256:aaaa";
const HASH_B: &str = "sha256:bbbb";

fn suite(name: &str) -> SuiteId {
    SuiteId::new(name)
}

fn commit(value: &str) -> Commit {
    Commit::new(value)
}

fn obligation(id: &str, hash: &str) -> Obligation {
    Obligation {
        id: id.to_owned(),
        statement: format!("the system shall {id}"),
        statement_hash: hash.to_owned(),
        target_ids: None,
        // The three fields quoin#383 added for the auditor. Absent here, which
        // is what an obligation with no `Verification` cell, no criticality and
        // no parameters looks like.
        method: None,
        criticality: None,
        parameters: None,
    }
}

fn entry(symbol: &str, outcome: Outcome, traces: &[&str]) -> RunEntry {
    RunEntry {
        symbol: SymbolId::new(symbol),
        outcome,
        score: None,
        metric: None,
        trace_ids: Some(traces.iter().map(|id| (*id).to_owned()).collect()),
        config: None,
    }
}

fn run(suite_name: &str, commit_value: &str, timestamp: &str, entries: Vec<RunEntry>) -> RunRecord {
    RunRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: suite(suite_name),
        commit: commit(commit_value),
        tool: "cargo test".to_owned(),
        evidence_kind: None,
        timestamp: timestamp.to_owned(),
        entries,
    }
}

fn request(suite_name: &str, commit_value: &str, entries: Vec<RunEntry>) -> RecordRequest {
    RecordRequest {
        suite: suite(suite_name),
        commit: commit(commit_value),
        tool: "cargo test".to_owned(),
        evidence_kind: None,
        lineage: None,
        timestamp: "2026-08-17T00:00:00Z".to_owned(),
        entries,
    }
}

fn binding(obligation_id: &str, suite_name: &str, hash: &str) -> Binding {
    Binding {
        obligation: ObligationId::new(obligation_id),
        statement_hash_at_binding: StatementHash::new(hash),
        suite: suite(suite_name),
        commit: commit(COMMIT),
        symbols: vec![SymbolId::new("t")],
        lineage: None,
        affirmations: None,
    }
}

fn context(name: &str) -> ProducerContext {
    ProducerContext {
        name: name.to_owned(),
        version: "1.0".to_owned(),
        configuration_digest: "sha256:aa".to_owned(),
        validation_corpus_digest: "sha256:bb".to_owned(),
        input_contract: "junit-xml".to_owned(),
        environment: "ci".to_owned(),
        adapter: None,
    }
}

fn decision(id: &str) -> TrustDecision {
    TrustDecision {
        schema_version: STORE_SCHEMA_VERSION,
        id: TrustDecisionId::parse(id).unwrap(),
        r#use: TrustUse {
            id: "U-1".to_owned(),
            intended_function: "transcribe junit".to_owned(),
            permitted_decisions: vec!["record-run".to_owned()],
        },
        decision: TrustDecisionKind::ReliedUpon,
        accepted_context: context("junit"),
        observed_context: Some(context("junit")),
        revalidate_on: vec![
            TrustTrigger::ProducerVersion,
            TrustTrigger::Configuration,
            TrustTrigger::ValidationCorpus,
            TrustTrigger::InputContract,
            TrustTrigger::Environment,
        ],
        validation_evidence: vec![TrustEvidenceReference {
            id: "V-1".to_owned(),
            digest: Some("sha256:abcdef".to_owned()),
            reference: None,
        }],
        limitations: Vec::new(),
        owner: "peter".to_owned(),
        decided_at: "2026-01-01T00:00:00Z".to_owned(),
    }
}

fn provenance() -> serde_json::Value {
    json!({
        "schemaVersion": "producer-provenance-v1",
        "identity": "quoin",
        "version": "0.1.0",
        "sourceRevision": "a".repeat(40),
        "sourceState": "clean",
        "executableDigest": format!("sha256:{}", "1".repeat(64)),
        "configurationDigest": format!("sha256:{}", "2".repeat(64)),
        "capabilities": ["record"],
        "artifacts": [{ "name": "report.json", "digest": format!("sha256:{}", "3".repeat(64)) }],
    })
}

fn experiment() -> serde_json::Value {
    json!({
        "schemaVersion": "experiment-record-v1",
        "subject": { "kind": "requirement", "id": "FR-001", "sourceRevision": "b".repeat(40) },
        "recordedAt": "2026-01-01T00:00:00Z",
        "hypothesis": "the adapter reads real output",
        "design": {
            "timeBox": "1d",
            "corpusRefs": ["corpus/a"],
            "comparisonMethod": "A/B",
            "decisionRule": "p<0.05",
        },
        "result": { "status": "supported", "summary": "it did", "evidenceRefs": ["runs/SUITE-A"] },
        "producerProvenance": provenance(),
    })
}

fn operational() -> serde_json::Value {
    json!({
        "schemaVersion": "operational-evidence-record-v1",
        "subject": { "kind": "service", "id": "quoin-core", "sourceRevision": "c".repeat(40) },
        "recordedAt": "2026-01-01T00:00:00Z",
        "window": { "startedAt": "2026-01-01T00:00:00Z", "endedAt": "2026-01-02T00:00:00Z" },
        "environment": "production",
        "observations": [{
            "signal": "latency",
            "value": "12",
            "unit": "ms",
            "interpretation": "within bounds",
            "evidenceRefs": ["metrics/1"],
        }],
        "outcome": "within_bounds",
        "producerProvenance": provenance(),
    })
}

// ---------------------------------------------------------------- FR-030 -----

/// Trace: FR-030-AC-1
#[test]
fn tc_456_200_a_run_file_is_named_by_suite_and_twelve_character_commit_prefix() {
    assert_eq!(
        run_path(&suite("SUITE-001"), &commit(COMMIT)),
        "runs/SUITE-001/aaaaaaaaaaaa.json"
    );
    assert_eq!(
        scan_path(&suite("SUITE-001"), &commit(COMMIT)),
        "scans/SUITE-001/aaaaaaaaaaaa.json"
    );
    assert_eq!(bindings_path(), "bindings.json");
    assert_eq!(
        trust_decision_path(&TrustDecisionId::parse("ETD-7").unwrap()),
        "trust/ETD-7.json"
    );
}

/// Trace: FR-030-AC-2
#[test]
fn tc_456_201_records_sort_keys_at_every_level_and_end_with_a_newline() {
    let mut store = MemoryEvidence::new();
    let record = run(
        "SUITE-001",
        COMMIT,
        "2026-08-17T00:00:00Z",
        vec![entry("tests::ok", Outcome::Pass, &["FR-001-AC-1"])],
    );
    let path = write_run(&mut store, &record).unwrap();
    let first = store.read(&path).unwrap().unwrap();
    assert!(
        first.ends_with('\n'),
        "the record does not end with a newline"
    );
    // Every contiguous run of keys at one indentation must already be sorted.
    let mut previous: Option<(usize, &str)> = None;
    let mut checked = 0_usize;
    for line in first.lines() {
        let indent = line.len() - line.trim_start().len();
        let Some(rest) = line.trim_start().strip_prefix('"') else {
            previous = None;
            continue;
        };
        let Some((key, tail)) = rest.split_once('"') else {
            previous = None;
            continue;
        };
        if !tail.starts_with(':') {
            previous = None;
            continue;
        }
        if let Some((last_indent, last_key)) = previous
            && last_indent == indent
        {
            assert!(last_key < key, "{last_key} precedes {key} out of order");
            checked += 1;
        }
        previous = Some((indent, key));
    }
    assert!(checked >= 5, "too few key pairs were compared: {checked}");

    // Serializing the same record twice must produce the same bytes.
    let mut second_store = MemoryEvidence::new();
    write_run(&mut second_store, &record).unwrap();
    assert_eq!(first, second_store.read(&path).unwrap().unwrap());
}

/// Trace: FR-030-AC-3
#[test]
fn tc_456_202_one_file_per_suite_and_commit_and_the_last_write_wins() {
    let mut store = MemoryEvidence::new();
    write_run(
        &mut store,
        &run("SUITE-001", COMMIT, "2026-08-17T00:00:00Z", vec![]),
    )
    .unwrap();
    write_run(
        &mut store,
        &run("SUITE-002", COMMIT, "2026-08-17T00:00:00Z", vec![]),
    )
    .unwrap();
    assert_eq!(
        store.paths(),
        vec![
            "runs/SUITE-001/aaaaaaaaaaaa.json",
            "runs/SUITE-002/aaaaaaaaaaaa.json"
        ],
        "two suites at one commit must not merge into one file"
    );

    write_run(
        &mut store,
        &run(
            "SUITE-001",
            COMMIT,
            "2026-08-18T00:00:00Z",
            vec![entry("tests::later", Outcome::Pass, &[])],
        ),
    )
    .unwrap();
    assert_eq!(store.paths().len(), 2, "the rewrite added a file");
    let text = store
        .read("runs/SUITE-001/aaaaaaaaaaaa.json")
        .unwrap()
        .unwrap();
    assert!(text.contains("tests::later"), "the later write did not win");
}

/// Trace: FR-030-AC-4
#[test]
fn tc_456_203_only_a_passing_symbol_discharges_an_obligation() {
    let obligations = [obligation("FR-001-AC-1", HASH_A)];
    for (outcome, expected) in [
        (Outcome::Pass, vec![ObligationId::new("FR-001-AC-1")]),
        (Outcome::Fail, vec![]),
        (Outcome::Skip, vec![]),
        (Outcome::Error, vec![]),
    ] {
        let mut store = MemoryEvidence::new();
        let outcome = record_run(
            &mut store,
            &request(
                "SUITE-001",
                COMMIT,
                vec![entry("tests::tc001", outcome, &["FR-001-AC-1"])],
            ),
            &obligations,
        )
        .unwrap();
        assert_eq!(outcome.bound, expected);
    }
}

/// Trace: FR-030-AC-5
#[test]
fn tc_456_204_a_changed_statement_is_reported_suspect_and_the_hash_is_not_moved() {
    let mut store = MemoryEvidence::new();
    record_run(
        &mut store,
        &request(
            "SUITE-001",
            COMMIT,
            vec![entry("tests::tc001", Outcome::Pass, &["FR-001-AC-1"])],
        ),
        &[obligation("FR-001-AC-1", HASH_A)],
    )
    .unwrap();
    let outcome = record_run(
        &mut store,
        &request(
            "SUITE-001",
            OTHER_COMMIT,
            vec![entry("tests::tc001", Outcome::Pass, &["FR-001-AC-1"])],
        ),
        &[obligation("FR-001-AC-1", HASH_B)],
    )
    .unwrap();
    assert_eq!(outcome.suspect, vec![ObligationId::new("FR-001-AC-1")]);
    assert!(
        outcome.bound.is_empty(),
        "a re-discharge is not a new binding"
    );
    assert_eq!(
        read_bindings(&store).unwrap().bindings[0]
            .statement_hash_at_binding
            .as_str(),
        HASH_A,
        "re-running a test must not re-affirm a reworded requirement"
    );
}

/// Trace: FR-030-AC-6
#[test]
fn tc_456_205_affirming_moves_the_hash_forward_and_records_who_and_where() {
    let existing = [binding("FR-001-AC-1", "SUITE-001", HASH_A)];
    let affirmation = Affirmation {
        who: "peter".to_owned(),
        commit: commit(OTHER_COMMIT),
        note: Some("read the new statement".to_owned()),
    };
    let outcome = affirm(
        &existing,
        &ObligationId::new("FR-001-AC-1"),
        None,
        &StatementHash::new(HASH_B),
        &affirmation,
    );
    assert!(outcome.found);
    assert_eq!(
        outcome.bindings[0].statement_hash_at_binding.as_str(),
        HASH_B
    );
    assert_eq!(
        outcome.bindings[0].affirmations.as_ref().unwrap()[0].who,
        "peter"
    );

    let unknown = affirm(
        &existing,
        &ObligationId::new("FR-999-AC-9"),
        None,
        &StatementHash::new(HASH_B),
        &affirmation,
    );
    assert!(!unknown.found, "an unknown obligation must not be invented");
    assert_eq!(unknown.bindings, existing, "the graph must be untouched");
}

/// Trace: FR-030-AC-7
#[test]
fn tc_456_206_a_trace_id_no_obligation_states_is_named_rather_than_dropped() {
    let mut store = MemoryEvidence::new();
    let outcome = record_run(
        &mut store,
        &request(
            "SUITE-001",
            COMMIT,
            vec![entry("tests::tc001", Outcome::Pass, &["FR-404-AC-1"])],
        ),
        &[obligation("FR-001-AC-1", HASH_A)],
    )
    .unwrap();
    assert_eq!(outcome.unmatched, vec!["FR-404-AC-1".to_owned()]);
    assert!(outcome.bound.is_empty());
}

/// Trace: FR-030-AC-8
#[test]
fn tc_456_207_gc_deletes_only_unreferenced_older_runs_and_dry_run_changes_nothing() {
    let mut store = MemoryEvidence::new();
    write_run(
        &mut store,
        &run("SUITE-001", COMMIT, "2026-08-17T00:00:00Z", vec![]),
    )
    .unwrap();
    write_run(
        &mut store,
        &run("SUITE-001", OTHER_COMMIT, "2026-08-18T00:00:00Z", vec![]),
    )
    .unwrap();
    write_run(
        &mut store,
        &run(
            "SUITE-001",
            "cccccccccccc0000",
            "2026-08-16T00:00:00Z",
            vec![],
        ),
    )
    .unwrap();
    let mut referenced = binding("FR-001-AC-1", "SUITE-001", HASH_A);
    referenced.commit = commit(COMMIT);
    write_bindings(&mut store, &[referenced]).unwrap();

    let before = store.paths().len();
    let dry = gc(&mut store, true).unwrap();
    assert_eq!(store.paths().len(), before, "--dry-run deleted something");
    let wet = gc(&mut store, false).unwrap();
    assert_eq!(dry, wet, "the dry run and the real run disagree");
    assert_eq!(
        wet,
        vec!["runs/SUITE-001/cccccccccccc.json".to_owned()],
        "gc kept the newest and the referenced, and collected the rest"
    );
    assert!(
        store
            .read("runs/SUITE-001/aaaaaaaaaaaa.json")
            .unwrap()
            .is_some()
    );
    assert!(
        store
            .read("runs/SUITE-001/bbbbbbbbbbbb.json")
            .unwrap()
            .is_some()
    );
}

/// Trace: FR-030-AC-9
#[test]
fn tc_456_208_an_unrecorded_store_reads_as_an_empty_graph_not_an_error() {
    let store = MemoryEvidence::new();
    let graph = read_bindings(&store).unwrap();
    assert!(graph.bindings.is_empty());
    assert_eq!(graph.schema_version, STORE_SCHEMA_VERSION);
    assert!(list_recorded_suites(&store).unwrap().is_empty());
}

/// Trace: FR-030-AC-10, FR-030-CON-2
#[test]
fn tc_456_209_the_graph_keeps_the_hash_and_never_the_statement() {
    let mut store = MemoryEvidence::new();
    record_run(
        &mut store,
        &request(
            "SUITE-001",
            COMMIT,
            vec![entry("tests::tc001", Outcome::Pass, &["FR-001-AC-1"])],
        ),
        &[obligation("FR-001-AC-1", HASH_A)],
    )
    .unwrap();
    let text = store.read(&bindings_path()).unwrap().unwrap();
    assert!(text.contains(HASH_A), "the hash is not recorded");
    assert!(
        !text.contains("the system shall"),
        "the statement text leaked into the store"
    );
}

/// Trace: FR-030-AC-15
#[test]
fn tc_456_210_a_test_case_id_binds_to_every_criterion_whose_cell_names_it() {
    let mut store = MemoryEvidence::new();
    let mut first = obligation("FR-038-AC-1", HASH_A);
    first.target_ids = Some(vec!["TC-EV-054".to_owned()]);
    let mut second = obligation("FR-038-AC-2", HASH_A);
    second.target_ids = Some(vec!["TC-EV-054".to_owned()]);
    let outcome = record_run(
        &mut store,
        &request(
            "EVAL-001",
            COMMIT,
            vec![entry("TC-EV-054", Outcome::Pass, &["TC-EV-054"])],
        ),
        &[first, second],
    )
    .unwrap();
    assert_eq!(
        outcome.bound,
        vec![
            ObligationId::new("FR-038-AC-1"),
            ObligationId::new("FR-038-AC-2")
        ]
    );
    assert!(outcome.unmatched.is_empty());
}

/// Trace: FR-030-AC-15
#[test]
fn tc_456_211_a_direct_obligation_id_beats_the_indirect_test_case_route() {
    let mut store = MemoryEvidence::new();
    let mut sibling = obligation("FR-001-AC-1", HASH_A);
    sibling.target_ids = Some(vec!["FR-001-AC-2".to_owned()]);
    let outcome = record_run(
        &mut store,
        &request(
            "SUITE-001",
            COMMIT,
            vec![entry("tests::tc001", Outcome::Pass, &["FR-001-AC-2"])],
        ),
        &[sibling, obligation("FR-001-AC-2", HASH_A)],
    )
    .unwrap();
    assert_eq!(outcome.bound, vec![ObligationId::new("FR-001-AC-2")]);
}

/// Trace: FR-030-AC-16
#[test]
fn tc_456_212_a_completed_empty_inspection_is_recorded_and_read_at_the_exact_commit() {
    let mut store = MemoryEvidence::new();
    write_mock_inspection(
        &mut store,
        &MockInspectionRecord {
            schema_version: STORE_SCHEMA_VERSION,
            suite: suite("SUITE-001"),
            commit: commit(COMMIT),
            tool: "quoin".to_owned(),
            timestamp: "2026-08-17T00:00:00Z".to_owned(),
            injections: Vec::new(),
        },
    )
    .unwrap();
    let (suites, injections) = mock_inspection_input(&store, Some(&commit(COMMIT))).unwrap();
    assert_eq!(suites, vec![suite("SUITE-001")]);
    assert!(
        injections.is_empty(),
        "an empty inspection is a completed inspection"
    );

    let (other_suites, _) = mock_inspection_input(&store, Some(&commit(OTHER_COMMIT))).unwrap();
    assert!(
        other_suites.is_empty(),
        "a record at another commit must not be read as this one's"
    );
    let (none, _) = mock_inspection_input(&store, None).unwrap();
    assert!(none.is_empty());
}

// ---------------------------------------------------------------- FR-031 -----

/// Trace: FR-031-AC-1
#[test]
fn tc_456_220_the_graph_is_cross_suite_and_ordered_by_obligation_then_suite() {
    let mut store = MemoryEvidence::new();
    write_bindings(
        &mut store,
        &[
            binding("FR-001-AC-2", "SUITE-001", HASH_B),
            binding("FR-001-AC-1", "SUITE-002", HASH_A),
            binding("FR-001-AC-1", "SUITE-001", HASH_A),
        ],
    )
    .unwrap();
    let graph = read_bindings(&store).unwrap();
    let order: Vec<(&str, &str)> = graph
        .bindings
        .iter()
        .map(|item| (item.obligation.as_str(), item.suite.as_str()))
        .collect();
    assert_eq!(
        order,
        vec![
            ("FR-001-AC-1", "SUITE-001"),
            ("FR-001-AC-1", "SUITE-002"),
            ("FR-001-AC-2", "SUITE-001"),
        ]
    );
    assert!(
        store
            .read(&bindings_path())
            .unwrap()
            .unwrap()
            .ends_with('\n')
    );
}

/// Trace: FR-031-AC-1
#[test]
fn tc_456_221_affirming_clears_every_suite_not_only_the_first() {
    let existing = [
        binding("FR-001-AC-1", "SUITE-001", HASH_A),
        binding("FR-001-AC-1", "SUITE-002", HASH_A),
    ];
    let outcome = affirm(
        &existing,
        &ObligationId::new("FR-001-AC-1"),
        None,
        &StatementHash::new(HASH_B),
        &Affirmation {
            who: "peter".to_owned(),
            commit: commit(COMMIT),
            note: None,
        },
    );
    assert!(
        outcome
            .bindings
            .iter()
            .all(|item| item.statement_hash_at_binding.as_str() == HASH_B),
        "a sibling suite was left suspect for a reason nobody could act on"
    );

    let narrowed = affirm(
        &existing,
        &ObligationId::new("FR-001-AC-1"),
        Some(&suite("SUITE-002")),
        &StatementHash::new(HASH_B),
        &Affirmation {
            who: "peter".to_owned(),
            commit: commit(COMMIT),
            note: None,
        },
    );
    assert_eq!(
        narrowed.bindings[0].statement_hash_at_binding.as_str(),
        HASH_A
    );
    assert_eq!(
        narrowed.bindings[1].statement_hash_at_binding.as_str(),
        HASH_B
    );
}

/// Trace: FR-031-AC-2
#[test]
fn tc_456_222_the_newest_record_is_the_newest_by_timestamp_not_by_filename() {
    let mut store = MemoryEvidence::new();
    // `aaaa…` sorts first but ran last.
    write_run(
        &mut store,
        &run("SUITE-001", COMMIT, "2026-08-20T00:00:00Z", vec![]),
    )
    .unwrap();
    write_run(
        &mut store,
        &run("SUITE-001", OTHER_COMMIT, "2026-08-01T00:00:00Z", vec![]),
    )
    .unwrap();
    let mut skipped = Vec::new();
    let runs = read_runs(&store, &suite("SUITE-001"), &mut skipped).unwrap();
    assert_eq!(
        runs.last().unwrap().commit.as_str(),
        COMMIT,
        "records are not ordered by timestamp"
    );
    assert!(skipped.is_empty());
}

/// Trace: FR-031-AC-2
#[test]
fn tc_456_223_two_records_sharing_a_timestamp_are_ordered_by_commit() {
    let mut store = MemoryEvidence::new();
    write_run(
        &mut store,
        &run("SUITE-001", OTHER_COMMIT, "2026-08-17T00:00:00Z", vec![]),
    )
    .unwrap();
    write_run(
        &mut store,
        &run("SUITE-001", COMMIT, "2026-08-17T00:00:00Z", vec![]),
    )
    .unwrap();
    let mut skipped = Vec::new();
    let runs = read_runs(&store, &suite("SUITE-001"), &mut skipped).unwrap();
    let commits: Vec<&str> = runs.iter().map(|record| record.commit.as_str()).collect();
    assert_eq!(commits, vec![COMMIT, OTHER_COMMIT]);
}

/// Trace: FR-031-AC-3
#[test]
fn tc_456_224_an_unreadable_binding_graph_names_the_file_and_the_cause() {
    let store = MemoryEvidence::new().with_file(
        bindings_path(),
        "<<<<<<< HEAD\n{\"bindings\": []}\n=======\n",
    );
    let error = read_bindings(&store).unwrap_err().to_string();
    assert!(
        error.contains("bindings.json"),
        "the file is not named: {error}"
    );
    assert!(
        error.contains("merge conflict"),
        "the cause is not named: {error}"
    );
}

/// Trace: FR-031-AC-3
#[test]
fn tc_456_225_one_corrupt_run_file_is_skipped_rather_than_hiding_every_finding() {
    let mut store = MemoryEvidence::new();
    write_run(
        &mut store,
        &run(
            "SUITE-001",
            COMMIT,
            "2026-08-17T00:00:00Z",
            vec![entry("tests::ok", Outcome::Pass, &[])],
        ),
    )
    .unwrap();
    store
        .write("runs/SUITE-001/bbbbbbbbbbbb.json", b"{ truncated")
        .unwrap();

    let mut skipped = Vec::new();
    let runs = read_runs(&store, &suite("SUITE-001"), &mut skipped).unwrap();
    assert_eq!(
        runs.iter().map(|r| r.commit.as_str()).collect::<Vec<_>>(),
        vec![COMMIT],
        "the good record did not reach the caller"
    );
    assert_eq!(skipped.len(), 1);
    assert!(skipped[0].contains("bbbbbbbbbbbb.json"));
}

// ---------------------------------------------------------------- FR-048 -----

/// Trace: FR-048-AC-1, FR-048-AC-2
#[test]
fn tc_456_240_an_assurance_record_is_identified_by_its_canonical_content() {
    let mut store = MemoryEvidence::new();
    let first = write_experiment_record(&mut store, &experiment()).unwrap();
    assert!(first.created);
    assert!(
        first.record.record_id.starts_with("sha256:"),
        "identity is not content derived"
    );
    assert_eq!(
        first.record.input.producer_provenance.identity, "quoin",
        "the record does not carry a producer provenance tuple"
    );

    let again = write_experiment_record(&mut store, &experiment()).unwrap();
    assert!(!again.created, "an identical republish is not a new record");
    assert_eq!(again.record.record_id, first.record.record_id);
    assert_eq!(store.paths().len(), 1);
}

/// Trace: FR-048-AC-3
#[test]
fn tc_456_241_invalid_provenance_is_refused_before_any_record_is_published() {
    let mut store = MemoryEvidence::new();
    let mut bad = experiment();
    bad["producerProvenance"]["executableDigest"] = json!("sha256:not-hex");
    let error = write_experiment_record(&mut store, &bad)
        .unwrap_err()
        .to_string();
    assert!(error.contains("executableDigest"), "{error}");
    assert!(
        store.paths().is_empty(),
        "a refused record still reached the store"
    );
}

/// Trace: FR-048-AC-4, FR-048-AC-6
#[test]
fn tc_456_242_differing_bytes_at_an_immutable_path_are_never_overwritten() {
    let mut store = MemoryEvidence::new();
    let stored = write_experiment_record(&mut store, &experiment()).unwrap();
    store
        .write(&stored.path, b"{\"recordId\":\"tampered\"}\n")
        .unwrap();
    let error = write_experiment_record(&mut store, &experiment())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("existing bytes were not overwritten"),
        "{error}"
    );
    assert_eq!(
        store.read(&stored.path).unwrap().unwrap(),
        "{\"recordId\":\"tampered\"}\n",
        "the store overwrote the differing bytes"
    );
}

/// Trace: FR-048-AC-5
#[test]
fn tc_456_243_operational_evidence_is_content_addressed_and_its_window_is_checked() {
    let mut store = MemoryEvidence::new();
    let stored = write_operational_evidence_record(&mut store, &operational()).unwrap();
    assert!(stored.path.starts_with("operational/sha256-"));

    let mut inverted = operational();
    inverted["window"]["endedAt"] = json!("2025-12-31T00:00:00Z");
    let error = write_operational_evidence_record(&mut store, &inverted)
        .unwrap_err()
        .to_string();
    assert!(error.contains("must be after"), "{error}");
}

/// Trace: FR-048-AC-6
#[test]
fn tc_456_244_content_tampering_is_detected_on_read() {
    let mut store = MemoryEvidence::new();
    let stored = write_experiment_record(&mut store, &experiment()).unwrap();
    let text = store.read(&stored.path).unwrap().unwrap();
    store
        .write(
            &stored.path,
            text.replace("it did", "it did not").as_bytes(),
        )
        .unwrap();
    let error = read_experiment_record(&store, &stored.record.record_id)
        .unwrap_err()
        .to_string();
    assert!(error.contains("digest does not match content"), "{error}");
}

// ---------------------------------------------------------------- FR-093 -----

/// Trace: FR-093-AC-1, FR-093-AC-2
#[test]
fn tc_456_260_a_decision_is_scoped_to_one_use_and_is_never_a_global_tool_badge() {
    let assessment = assess_trust(&decision("ETD-1")).unwrap();
    assert_eq!(assessment.status, TrustStatus::Accepted);
    assert_eq!(assessment.use_id, "U-1");
    assert_eq!(assessment.producer, "junit");
    assert_eq!(
        assessment.permitted_decisions,
        vec!["record-run".to_owned()]
    );
    assert_eq!(assessment.owner, "peter");
}

/// Trace: FR-093-AC-3, FR-093-CON-3
#[test]
fn tc_456_261_a_changed_context_names_every_trigger_and_never_falls_back_to_accepted() {
    let mut changed = decision("ETD-1");
    let observed = changed.observed_context.as_mut().unwrap();
    observed.version = "2.0".to_owned();
    observed.environment = "laptop".to_owned();
    let assessment = assess_trust(&changed).unwrap();
    assert_eq!(assessment.status, TrustStatus::Invalidated);
    assert_eq!(
        assessment.triggered_by,
        vec![TrustTrigger::ProducerVersion, TrustTrigger::Environment]
    );
}

/// Trace: FR-093-AC-4, FR-093-CON-3
#[test]
fn tc_456_262_an_unobserved_decision_is_neither_accepted_nor_rejected() {
    let mut unobserved = decision("ETD-1");
    unobserved.observed_context = None;
    let assessment = assess_trust(&unobserved).unwrap();
    assert_eq!(assessment.status, TrustStatus::Unobserved);
    assert!(assessment.triggered_by.is_empty());
}

/// Trace: FR-093-AC-5
#[test]
fn tc_456_263_one_producer_carries_different_decisions_for_different_uses() {
    let mut relied = decision("ETD-1");
    relied.r#use.id = "U-record".to_owned();
    let mut refused = decision("ETD-2");
    refused.r#use.id = "U-certify".to_owned();
    refused.decision = TrustDecisionKind::NotReliedUpon;

    let first = assess_trust(&relied).unwrap();
    let second = assess_trust(&refused).unwrap();
    assert_eq!(first.producer, second.producer);
    assert_ne!(first.use_id, second.use_id);
    assert_eq!(first.status, TrustStatus::Accepted);
    assert_eq!(second.status, TrustStatus::NotAccepted);
}

/// Trace: FR-093-AC-6
#[test]
fn tc_456_264_valid_decisions_round_trip_and_an_invalid_file_is_reported_not_hidden() {
    let mut store = MemoryEvidence::new();
    write_trust_decision(&mut store, &decision("ETD-1")).unwrap();
    store
        .write("trust/ETD-2.json", b"{\"schemaVersion\": 1}\n")
        .unwrap();

    let mut skipped = Vec::new();
    let decisions = read_trust_decisions(&store, &mut skipped).unwrap();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].id.as_str(), "ETD-1");
    assert_eq!(skipped, vec!["trust/ETD-2.json".to_owned()]);
}

/// Trace: FR-093-CON-3
#[test]
fn tc_456_265_a_decision_missing_a_required_trigger_or_its_evidence_is_refused() {
    let mut missing = decision("ETD-1");
    missing
        .revalidate_on
        .retain(|trigger| *trigger != TrustTrigger::Environment);
    let error = validate_trust_decision(&missing).unwrap_err().to_string();
    assert!(error.contains("must include environment"), "{error}");

    let mut unevidenced = decision("ETD-1");
    unevidenced.validation_evidence.clear();
    let error = validate_trust_decision(&unevidenced)
        .unwrap_err()
        .to_string();
    assert!(error.contains("validationEvidence"), "{error}");

    let mut adapted = decision("ETD-1");
    adapted.accepted_context.adapter = Some(TrustAdapter {
        name: "junit".to_owned(),
        version: "3".to_owned(),
    });
    let error = validate_trust_decision(&adapted).unwrap_err().to_string();
    assert!(
        error.contains("must include adapter when an adapter is relied upon"),
        "{error}"
    );
}

// ---------------------------------------------------------------- FR-094 -----

fn requirement(dimensions: &[IndependenceDimension]) -> IndependenceRequirement {
    IndependenceRequirement {
        id: "IR-1".to_owned(),
        obligation: ObligationId::new("FR-001-AC-1"),
        dimensions: dimensions.to_vec(),
        rationale: "two independent lines".to_owned(),
    }
}

fn lineage(actor: Option<&str>, technique: Option<&str>) -> EvidenceLineage {
    EvidenceLineage {
        actor: actor.map(str::to_owned),
        implementation_toolchain: None,
        technique: technique.map(str::to_owned),
        data_source: None,
        review_path: None,
    }
}

fn lineaged(suite_name: &str, value: EvidenceLineage) -> Binding {
    let mut item = binding("FR-001-AC-1", suite_name, HASH_A);
    item.lineage = Some(value);
    item
}

/// Trace: FR-094-AC-1
#[test]
fn tc_456_280_a_policy_naming_no_dimension_is_refused() {
    let policy = IndependencePolicy {
        schema_version: STORE_SCHEMA_VERSION,
        profile: ProfileId::parse("AP-1").unwrap(),
        requirements: vec![requirement(&[])],
    };
    let error = validate_independence_policy(&policy)
        .unwrap_err()
        .to_string();
    assert!(error.contains("dimensions"), "{error}");

    let good = IndependencePolicy {
        requirements: vec![requirement(&[IndependenceDimension::Actor])],
        ..policy
    };
    validate_independence_policy(&good).unwrap();
}

/// Trace: FR-094-AC-2
#[test]
fn tc_456_281_a_policy_naming_an_unknown_obligation_is_refused_not_silently_skipped() {
    let policy = IndependencePolicy {
        schema_version: STORE_SCHEMA_VERSION,
        profile: ProfileId::parse("AP-1").unwrap(),
        requirements: vec![requirement(&[IndependenceDimension::Actor])],
    };
    let known = ["FR-002-AC-1"];
    let error = require_known_policy_obligations(&policy, known.iter().copied())
        .unwrap_err()
        .to_string();
    assert!(error.contains("FR-001-AC-1"), "{error}");

    let known = ["FR-001-AC-1"];
    require_known_policy_obligations(&policy, known.iter().copied()).unwrap();
}

/// Trace: FR-094-AC-3, FR-094-CON-2
#[test]
fn tc_456_282_independence_needs_two_lines_differing_on_every_selected_dimension() {
    let requirement = requirement(&[
        IndependenceDimension::Actor,
        IndependenceDimension::Technique,
    ]);
    let satisfied = assess_independence(
        &ProfileId::parse("AP-1").unwrap(),
        &requirement,
        &[
            lineaged("SUITE-A", lineage(Some("alice"), Some("unit"))),
            lineaged("SUITE-B", lineage(Some("bob"), Some("property"))),
        ],
    );
    assert_eq!(satisfied.status, IndependenceStatus::Satisfied);
    assert_eq!(
        satisfied.satisfied_by,
        Some((suite("SUITE-A"), suite("SUITE-B")))
    );

    let shared = assess_independence(
        &ProfileId::parse("AP-1").unwrap(),
        &requirement,
        &[
            lineaged("SUITE-A", lineage(Some("alice"), Some("unit"))),
            lineaged("SUITE-B", lineage(Some("bob"), Some("unit"))),
        ],
    );
    assert_eq!(
        shared.status,
        IndependenceStatus::Insufficient,
        "two suites sharing a technique are still common-mode"
    );
}

/// Trace: FR-094-AC-4
#[test]
fn tc_456_283_a_common_actor_and_a_missing_dimension_stay_visible_in_the_assessment() {
    let assessment = assess_independence(
        &ProfileId::parse("AP-1").unwrap(),
        &requirement(&[IndependenceDimension::Actor]),
        &[
            lineaged("SUITE-A", lineage(Some("alice"), None)),
            binding("FR-001-AC-1", "SUITE-B", HASH_A),
        ],
    );
    assert_eq!(assessment.status, IndependenceStatus::Insufficient);
    assert_eq!(assessment.dimensions[0].values, vec!["alice".to_owned()]);
    assert_eq!(
        assessment.dimensions[0].missing_suites,
        vec![suite("SUITE-B")],
        "a dimension absent on one side must be named"
    );
    assert!(assessment.summary.contains("missing on SUITE-B"));
}

/// Trace: FR-094-AC-7
#[test]
fn tc_456_284_a_later_run_that_states_no_lineage_clears_the_old_claim() {
    let mut store = MemoryEvidence::new();
    let mut first = request(
        "SUITE-001",
        COMMIT,
        vec![entry("tests::tc001", Outcome::Pass, &["FR-001-AC-1"])],
    );
    first.lineage = Some(lineage(Some("alice"), None));
    record_run(&mut store, &first, &[obligation("FR-001-AC-1", HASH_A)]).unwrap();
    assert_eq!(
        read_bindings(&store).unwrap().bindings[0]
            .lineage
            .as_ref()
            .unwrap()
            .actor
            .as_deref(),
        Some("alice")
    );

    let second = request(
        "SUITE-001",
        OTHER_COMMIT,
        vec![entry("tests::tc001", Outcome::Pass, &["FR-001-AC-1"])],
    );
    record_run(&mut store, &second, &[obligation("FR-001-AC-1", HASH_A)]).unwrap();
    assert!(
        read_bindings(&store).unwrap().bindings[0].lineage.is_none(),
        "an old independence claim was carried into evidence that did not state one"
    );
}

// ---------------------------------------------------------------- FR-032 -----

/// Trace: FR-032-AC-16
#[test]
fn tc_456_300_the_same_explicit_stand_in_shape_is_found_in_rust_python_and_typescript() {
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(
        repo.path().join("lib.rs"),
        "#[test]\nfn tc_001_a() {\n    let c = MockClock::new();\n}\n",
    )
    .unwrap();
    std::fs::write(
        repo.path().join("test_gate.py"),
        "def test_b():\n    g = FakeGate.build()\n",
    )
    .unwrap();
    std::fs::write(
        repo.path().join("gate.test.ts"),
        "test(\"c\", () => {\n  const s = StubStore.create();\n});\n",
    )
    .unwrap();

    let found =
        inspect_mock_injections(&DiskEvidence::new(repo.path()), &suite("SUITE-001")).unwrap();
    let symbols: Vec<&str> = found.iter().map(|item| item.symbol.as_str()).collect();
    assert_eq!(
        symbols,
        vec!["c", "tc_001_a", "test_b"],
        "the same shape was not located in all three languages"
    );
    assert!(
        found
            .iter()
            .all(|item| item.suite.as_str() == "SUITE-001" && item.line.is_some()),
        "an injection is missing its suite or its line"
    );
}

/// Trace: FR-032-AC-16
#[test]
fn tc_456_301_ordinary_constructors_and_production_calls_are_not_reported() {
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(
        repo.path().join("lib.rs"),
        "fn main() { let c = MockClock::new(); }\n\
         #[test]\nfn tc_002_b() {\n    let c = SystemClock::new();\n    Vec::with_capacity(4);\n}\n",
    )
    .unwrap();
    let found =
        inspect_mock_injections(&DiskEvidence::new(repo.path()), &suite("SUITE-001")).unwrap();
    assert!(
        found.is_empty(),
        "an ordinary constructor or a production call was reported: {found:?}"
    );
}
