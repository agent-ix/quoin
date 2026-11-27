// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PLAT-1791: the binding graph and the run records never silently lose an
//! entry (FR-030; 21 CFR 11.10(e) audit trail).
//!
//! `record_run` reads `bindings.json`, adds its bindings and writes the whole
//! file back. Two recorders that both read before either wrote used to leave
//! only the last writer's bindings.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use ix_trace_rs::trace;
use quoin_evidence::record::{RecordRequest, record_run, record_run_with_analysis_projection};
use quoin_evidence::store::{
    gc, read_bindings, read_runs, write_mock_inspection, write_run, write_scan,
};
use quoin_evidence::types::{Outcome, RunEntry, RunRecord, STORE_SCHEMA_VERSION};
use quoin_evidence::{
    DiskEvidence, EvidenceError, EvidenceErrorCode, EvidenceSource, StoreLock, SuiteId, SymbolId,
};
use quoin_quire_types::Obligation;

fn obligation(id: &str) -> Obligation {
    Obligation {
        id: id.to_owned(),
        identity: None,
        document: None,
        statement: format!("the system shall {id}"),
        target_ids: None,
        method: None,
        criticality: None,
        parameters: None,
    }
}

fn entry(symbol: &str, outcome: Outcome, traces: &[&str]) -> RunEntry {
    RunEntry {
        symbol: SymbolId::new(symbol),
        outcome,
        path: None,
        score: None,
        metric: None,
        trace_ids: Some(traces.iter().map(|id| (*id).to_owned()).collect()),
        config: None,
        survivors: None,
    }
}

fn request(suite: &str, timestamp: &str, entries: Vec<RunEntry>) -> RecordRequest {
    RecordRequest {
        suite: SuiteId::new(suite),
        tool: "cargo test".to_owned(),
        evidence_kind: None,
        lineage: None,
        timestamp: timestamp.to_owned(),
        entries,
    }
}

fn run(suite: &str, timestamp: &str, entries: Vec<RunEntry>) -> RunRecord {
    RunRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new(suite),
        tool: "cargo test".to_owned(),
        evidence_kind: None,
        timestamp: timestamp.to_owned(),
        entries,
    }
}

/// Holds each recorder at its read of the graph until both have read, or until
/// the wait times out. A store that serialises recorders makes the second one
/// time out, which is the healthy outcome; a store that does not has both read
/// the same graph before either writes.
struct Rendezvous {
    arrived: Mutex<u32>,
    both: Condvar,
}

impl Rendezvous {
    fn wait(&self) {
        let mut arrived = self.arrived.lock().unwrap();
        *arrived += 1;
        self.both.notify_all();
        let _ = self
            .both
            .wait_timeout_while(arrived, Duration::from_secs(2), |count| *count < 2)
            .unwrap();
    }
}

struct Barriered {
    inner: DiskEvidence,
    rendezvous: Arc<Rendezvous>,
    forward_lock: bool,
}

impl EvidenceSource for Barriered {
    fn read(&self, path: &str) -> Result<Option<String>, EvidenceError> {
        let text = self.inner.read(path)?;
        if path == "bindings.json" {
            self.rendezvous.wait();
        }
        Ok(text)
    }
    fn list_files(&self, directory: &str) -> Result<Vec<String>, EvidenceError> {
        self.inner.list_files(directory)
    }
    fn list_directories(&self, directory: &str) -> Result<Vec<String>, EvidenceError> {
        self.inner.list_directories(directory)
    }
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), EvidenceError> {
        self.inner.write(path, bytes)
    }
    fn publish(&mut self, path: &str, bytes: &[u8]) -> Result<bool, EvidenceError> {
        self.inner.publish(path, bytes)
    }
    fn remove(&mut self, path: &str) -> Result<(), EvidenceError> {
        self.inner.remove(path)
    }
    fn lock_store(&mut self) -> Result<StoreLock, EvidenceError> {
        if self.forward_lock {
            self.inner.lock_store()
        } else {
            Ok(StoreLock::none())
        }
    }
    fn source_files(&self) -> Result<Vec<String>, EvidenceError> {
        self.inner.source_files()
    }
    fn source_text(&self, path: &str) -> Result<String, EvidenceError> {
        self.inner.source_text(path)
    }
}

/// Two independent recorders over one on-disk graph; returns the obligations
/// bound after reopening the store.
fn race(forward_lock: bool) -> Vec<(String, String)> {
    let repo = tempfile::tempdir().unwrap();
    let obligations = vec![obligation("FR-001-AC-1"), obligation("FR-002-AC-1")];
    let rendezvous = Arc::new(Rendezvous {
        arrived: Mutex::new(0),
        both: Condvar::new(),
    });
    let handles: Vec<_> = [("SUITE-A", "FR-001-AC-1"), ("SUITE-B", "FR-002-AC-1")]
        .into_iter()
        .map(|(suite, id)| {
            let mut source = Barriered {
                inner: DiskEvidence::new(repo.path()),
                rendezvous: Arc::clone(&rendezvous),
                forward_lock,
            };
            let obligations = obligations.clone();
            std::thread::spawn(move || {
                let request = request(
                    suite,
                    "2026-11-22T00:00:00Z",
                    vec![entry("tests::a", Outcome::Pass, &[id])],
                );
                record_run(&mut source, &request, &obligations).unwrap();
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    let reopened = DiskEvidence::new(repo.path());
    read_bindings(&reopened)
        .unwrap()
        .bindings
        .into_iter()
        .map(|b| {
            (
                b.obligation.as_str().to_owned(),
                b.suite.as_str().to_owned(),
            )
        })
        .collect()
}

#[trace("FR-030-AC-20")]
#[test]
fn two_recorders_sharing_one_graph_keep_both_bindings() {
    let bound = race(true);
    assert_eq!(
        bound,
        [
            ("FR-001-AC-1".to_owned(), "SUITE-A".to_owned()),
            ("FR-002-AC-1".to_owned(), "SUITE-B".to_owned()),
        ],
        "a successful recording was lost from the graph"
    );
}

/// Control: with the store lock withheld the same race loses a binding, so the
/// test above is observing the lock and not an accident of timing.
#[trace("FR-030-AC-20")]
#[test]
fn without_the_store_lock_the_same_race_loses_a_binding() {
    assert_eq!(race(false).len(), 1);
}

/// A source that fails the graph write once, as a process killed between the
/// run publication and the graph write would.
struct FailsGraphWrite {
    inner: DiskEvidence,
}

impl EvidenceSource for FailsGraphWrite {
    fn read(&self, path: &str) -> Result<Option<String>, EvidenceError> {
        self.inner.read(path)
    }
    fn list_files(&self, directory: &str) -> Result<Vec<String>, EvidenceError> {
        self.inner.list_files(directory)
    }
    fn list_directories(&self, directory: &str) -> Result<Vec<String>, EvidenceError> {
        self.inner.list_directories(directory)
    }
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), EvidenceError> {
        if path == "bindings.json" {
            return Err(EvidenceError::StoreIo {
                operation: "write",
                path: path.to_owned(),
                detail: "injected: process killed before the graph write".to_owned(),
            });
        }
        self.inner.write(path, bytes)
    }
    fn publish(&mut self, path: &str, bytes: &[u8]) -> Result<bool, EvidenceError> {
        self.inner.publish(path, bytes)
    }
    fn remove(&mut self, path: &str) -> Result<(), EvidenceError> {
        self.inner.remove(path)
    }
    fn lock_store(&mut self) -> Result<StoreLock, EvidenceError> {
        self.inner.lock_store()
    }
    fn source_files(&self) -> Result<Vec<String>, EvidenceError> {
        self.inner.source_files()
    }
    fn source_text(&self, path: &str) -> Result<String, EvidenceError> {
        self.inner.source_text(path)
    }
}

#[trace("FR-030-AC-21")]
#[test]
fn a_retry_after_an_interrupted_recording_binds_once_and_leaves_one_run() {
    let repo = tempfile::tempdir().unwrap();
    let obligations = vec![obligation("FR-001-AC-1")];
    let req = request(
        "SUITE-A",
        "2026-11-22T00:00:00Z",
        vec![entry("tests::a", Outcome::Pass, &["FR-001-AC-1"])],
    );

    let mut failing = FailsGraphWrite {
        inner: DiskEvidence::new(repo.path()),
    };
    let error = record_run(&mut failing, &req, &obligations).unwrap_err();
    assert_eq!(error.code(), EvidenceErrorCode::StoreIo);

    // Restart: a fresh handle, the same request.
    let mut restarted = DiskEvidence::new(repo.path());
    assert!(
        read_bindings(&restarted).unwrap().bindings.is_empty(),
        "the interrupted recording must not have bound anything"
    );
    let outcome = record_run(&mut restarted, &req, &obligations).unwrap();
    assert_eq!(
        outcome.bound.len(),
        1,
        "the retry must complete the binding"
    );
    assert_eq!(read_bindings(&restarted).unwrap().bindings.len(), 1);
    assert_eq!(
        restarted.list_files("runs/SUITE-A").unwrap().len(),
        1,
        "the retry must reuse the run record, not add an orphan"
    );

    // Replaying a completed recording changes nothing.
    let again = record_run(&mut restarted, &req, &obligations).unwrap();
    assert_eq!(again.bound, [] as [quoin_evidence::ObligationId; 0]);
    assert_eq!(read_bindings(&restarted).unwrap().bindings.len(), 1);
    assert_eq!(restarted.list_files("runs/SUITE-A").unwrap().len(), 1);
}

#[trace("FR-030-AC-22")]
#[test]
fn rewriting_a_run_key_with_different_content_is_refused_and_keeps_the_original() {
    let repo = tempfile::tempdir().unwrap();
    let mut store = DiskEvidence::new(repo.path());
    let failing = run(
        "SUITE-A",
        "2026-11-22T00:00:00Z",
        vec![entry("tests::a", Outcome::Fail, &[])],
    );
    let path = write_run(&mut store, &failing).unwrap();
    let original = store.read(&path).unwrap().unwrap();

    let passing = run(
        "SUITE-A",
        "2026-11-22T00:00:00Z",
        vec![entry("tests::a", Outcome::Pass, &[])],
    );
    let error = write_run(&mut store, &passing).unwrap_err();
    assert_eq!(error.code(), EvidenceErrorCode::RecordIntegrity);
    assert_eq!(
        store.read(&path).unwrap().unwrap(),
        original,
        "the failing record was replaced"
    );

    // Healthy control: an identical replay and a different key both succeed.
    assert_eq!(write_run(&mut store, &failing).unwrap(), path);
    let later = run("SUITE-A", "2026-11-22T00:00:01Z", vec![]);
    write_run(&mut store, &later).unwrap();
    assert_eq!(
        read_runs(&store, &SuiteId::new("SUITE-A"), &mut Vec::new())
            .unwrap()
            .len(),
        2
    );
}

#[trace("FR-030-AC-23")]
#[test]
fn gc_leaves_a_retirement_record_naming_what_it_deleted() {
    let repo = tempfile::tempdir().unwrap();
    let mut store = DiskEvidence::new(repo.path());
    let old = run(
        "SUITE-A",
        "2026-11-22T00:00:00Z",
        vec![entry("tests::a", Outcome::Fail, &[])],
    );
    let new = run(
        "SUITE-A",
        "2026-11-22T00:00:09Z",
        vec![entry("tests::a", Outcome::Pass, &[])],
    );
    let old_path = write_run(&mut store, &old).unwrap();
    write_run(&mut store, &new).unwrap();

    // Dry run: nothing deleted, nothing logged.
    assert_eq!(
        gc(&mut store, true, &mut Vec::new()).unwrap(),
        std::slice::from_ref(&old_path)
    );
    assert_eq!(
        store.list_files("retired").unwrap(),
        [] as [std::string::String; 0]
    );
    assert!(store.read(&old_path).unwrap().is_some());

    let deleted = gc(&mut store, false, &mut Vec::new()).unwrap();
    assert!(!deleted.contains(&"retired".to_owned()));
    assert_eq!(deleted, std::slice::from_ref(&old_path));
    assert!(store.read(&old_path).unwrap().is_none());

    let logs = store.list_files("retired").unwrap();
    assert_eq!(logs.len(), 1, "gc deleted a run and left no trace");
    let text = store
        .read(&format!("retired/{}", logs[0]))
        .unwrap()
        .unwrap();
    let log: serde_json::Value = serde_json::from_str(&text).unwrap();
    let retired = &log["retired"][0];
    assert_eq!(retired["path"], old_path);
    assert_eq!(retired["suite"], "SUITE-A");
    assert_eq!(retired["timestamp"], "2026-11-22T00:00:00Z");
    assert_eq!(retired["failed"], 1);
    assert_eq!(retired["passed"], 0);

    // Nothing left to collect: no second log, and the first is untouched.
    assert_eq!(
        gc(&mut store, false, &mut Vec::new()).unwrap(),
        [] as [std::string::String; 0]
    );
    assert_eq!(store.list_files("retired").unwrap().len(), 1);
}

#[trace("FR-030-AC-24")]
#[test]
fn removal_cannot_reach_outside_the_store_root() {
    let repo = tempfile::tempdir().unwrap();
    let outside = repo.path().join("keep.txt");
    std::fs::write(&outside, "evidence").unwrap();
    let mut store = DiskEvidence::new(repo.path());
    let error = store.remove("../../keep.txt").unwrap_err();
    assert_eq!(error.code(), EvidenceErrorCode::StoreIo);
    assert!(outside.exists(), "remove escaped the store root");

    // Healthy control: a path inside the root is removed.
    let mut store = DiskEvidence::new(repo.path());
    store.write("runs/SUITE-A/x.json", b"{}").unwrap();
    store.remove("runs/SUITE-A/x.json").unwrap();
    assert!(store.read("runs/SUITE-A/x.json").unwrap().is_none());
}

#[trace("FR-030-AC-23")]
#[test]
fn gc_keeps_a_record_it_cannot_read() {
    let repo = tempfile::tempdir().unwrap();
    let mut store = DiskEvidence::new(repo.path());
    write_run(&mut store, &run("SUITE-A", "2026-11-22T00:00:09Z", vec![])).unwrap();
    store
        .write("runs/SUITE-A/corrupt.json", b"{ not json")
        .unwrap();

    assert_eq!(
        gc(&mut store, false, &mut Vec::new()).unwrap(),
        [] as [std::string::String; 0]
    );
    assert!(
        store.read("runs/SUITE-A/corrupt.json").unwrap().is_some(),
        "gc deleted a record it could not read"
    );
}

#[trace("FR-030-AC-22")]
#[test]
fn scan_and_mock_inspection_rewrites_with_different_content_are_refused() {
    use quoin_evidence::types::{FindingRecord, MockInspectionRecord};
    let repo = tempfile::tempdir().unwrap();
    let mut store = DiskEvidence::new(repo.path());
    let scan = |ruleset: &str| FindingRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new("SUITE-A"),
        tool: "scanner".to_owned(),
        evidence_kind: None,
        timestamp: "2026-11-22T00:00:00Z".to_owned(),
        ruleset: Some(ruleset.to_owned()),
        rules_evaluated: Some(1),
        findings: Vec::new(),
        evaluated: None,
    };
    write_scan(&mut store, &scan("one")).unwrap();
    write_scan(&mut store, &scan("one")).unwrap();
    let error = write_scan(&mut store, &scan("two")).unwrap_err();
    assert_eq!(error.code(), EvidenceErrorCode::RecordIntegrity);

    let inspection = |tool: &str| MockInspectionRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new("SUITE-A"),
        tool: tool.to_owned(),
        timestamp: "2026-11-22T00:00:00Z".to_owned(),
        injections: Vec::new(),
    };
    write_mock_inspection(&mut store, &inspection("inspector 1.0.0")).unwrap();
    let error = write_mock_inspection(&mut store, &inspection("inspector 2.0.0")).unwrap_err();
    assert_eq!(error.code(), EvidenceErrorCode::RecordIntegrity);
}

#[trace("FR-030-AC-23")]
#[test]
fn a_repeated_gc_that_retires_a_different_set_under_the_same_first_path_still_collects() {
    let repo = tempfile::tempdir().unwrap();
    let mut store = DiskEvidence::new(repo.path());
    let at = |second: u32| run("SUITE-A", &format!("2026-11-22T00:00:0{second}Z"), vec![]);
    for second in [0, 1] {
        write_run(&mut store, &at(second)).unwrap();
    }
    gc(&mut store, false, &mut Vec::new()).unwrap();
    // The key is recorded again and a newer run supersedes it: the first
    // retired path, and so the log's name, is the same as before.
    write_run(&mut store, &at(0)).unwrap();
    write_run(&mut store, &at(2)).unwrap();
    let deleted = gc(&mut store, false, &mut Vec::new()).unwrap();
    assert_eq!(deleted.len(), 2, "gc was blocked by its own earlier log");
    assert_eq!(store.list_files("retired").unwrap().len(), 2);
}

#[trace("FR-030-AC-24")]
#[test]
fn writes_and_publishes_cannot_reach_outside_the_store_root() {
    let repo = tempfile::tempdir().unwrap();
    let mut store = DiskEvidence::new(repo.path());
    assert_eq!(
        store.write("../escaped.json", b"{}").unwrap_err().code(),
        EvidenceErrorCode::StoreIo
    );
    assert_eq!(
        store.publish("../escaped.json", b"{}").unwrap_err().code(),
        EvidenceErrorCode::StoreIo
    );
    assert!(!repo.path().join("spec/escaped.json").exists());
    assert!(store.publish("inside.json", b"{}").unwrap());
}

#[trace("FR-030-AC-21")]
#[test]
fn a_retry_completes_a_differential_pair_whose_projection_was_not_written() {
    let repo = tempfile::tempdir().unwrap();
    let mut store = DiskEvidence::new(repo.path());
    let entries = vec![entry("tests::a", Outcome::Pass, &[])];
    let req = request("SUITE-A", "2026-11-22T00:00:00Z", entries.clone());
    // The run alone, as a process killed before the projection leaves it.
    write_run(&mut store, &run("SUITE-A", "2026-11-22T00:00:00Z", entries)).unwrap();
    let projection = serde_json::json!({ "disposition": "same" });
    record_run_with_analysis_projection(&mut store, &req, &[], &projection).unwrap();
    assert_eq!(
        store.list_files("analysis-reports/SUITE-A").unwrap().len(),
        1
    );
    // A different projection at the occupied key is still refused.
    let other = serde_json::json!({ "disposition": "different" });
    record_run_with_analysis_projection(&mut store, &req, &[], &other).unwrap_err();
}

#[trace("FR-030-AC-23")]
#[test]
fn gc_names_an_unreadable_record_and_collects_nothing_around_it() {
    let repo = tempfile::tempdir().unwrap();
    let mut store = DiskEvidence::new(repo.path());
    write_run(&mut store, &run("SUITE-A", "2026-11-22T00:00:01Z", vec![])).unwrap();
    write_run(&mut store, &run("SUITE-A", "2026-11-22T00:00:02Z", vec![])).unwrap();
    store
        .write("runs/SUITE-A/z-corrupt.json", b"{ not json")
        .unwrap();

    let mut skipped = Vec::new();
    let deleted = gc(&mut store, false, &mut skipped).unwrap();
    assert!(
        deleted.is_empty(),
        "gc deleted readable evidence around an unreadable record"
    );
    assert_eq!(skipped, ["runs/SUITE-A/z-corrupt.json"]);
    assert_eq!(store.list_files("runs/SUITE-A").unwrap().len(), 3);

    // Healthy control: another suite with no unreadable record is collected.
    write_run(&mut store, &run("SUITE-B", "2026-11-22T00:00:01Z", vec![])).unwrap();
    write_run(&mut store, &run("SUITE-B", "2026-11-22T00:00:02Z", vec![])).unwrap();
    let mut skipped = Vec::new();
    assert_eq!(gc(&mut store, false, &mut skipped).unwrap().len(), 1);
}

#[trace("FR-030-AC-25")]
#[test]
fn taking_the_store_lock_leaves_no_file_in_the_store_on_unix() {
    let repo = tempfile::tempdir().unwrap();
    let mut store = DiskEvidence::new(repo.path());
    store.write("runs/SUITE-A/x.json", b"{}").unwrap();
    let before = store.list_files("").unwrap();
    let _lock = store.lock_store().unwrap();
    if cfg!(unix) {
        let entries = std::fs::read_dir(repo.path().join("spec/evidence"))
            .unwrap()
            .count();
        assert_eq!(entries, 1, "the lock created a file in the store");
        assert_eq!(store.list_files("").unwrap(), before);
    }
}
