// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use serde_json::{Value, json};

use super::{CustomerArtifactRefusal, read_customer_artifact, write_customer_artifact};
use crate::error::{EvidenceError, EvidenceErrorCode};
use crate::ids::SuiteId;
use crate::source::{DiskEvidence, EvidenceSource, MemoryEvidence};
use crate::store::{gc, write_run};
use crate::types::{RunRecord, STORE_SCHEMA_VERSION};

fn document() -> Value {
    json!({
        "artifactId": "Complaint-2026-001",
        "title": "Complaint record 001",
        "location": "https://dms.customer.example/records/4711",
        "suppliedBy": "Quality manager",
        "suppliedAt": "2026-11-20T09:30:00Z"
    })
}

fn refusal(error: EvidenceError) -> CustomerArtifactRefusal {
    match error {
        EvidenceError::CustomerArtifactRefused { kind, .. } => kind,
        other => panic!("expected a customer artifact refusal, got {other}"),
    }
}

#[test]
#[trace("FR-030-AC-36")]
fn a_document_is_filed_canonical_under_a_lowercased_name_and_reads_back() {
    let mut store = MemoryEvidence::new();
    let stored = write_customer_artifact(&mut store, &document()).unwrap();
    assert!(stored.created);
    assert_eq!(stored.path, "customer-artifacts/complaint-2026-001.json");
    assert_eq!(
        store.read(&stored.path).unwrap().unwrap(),
        concat!(
            "{\n",
            "  \"artifactId\": \"Complaint-2026-001\",\n",
            "  \"location\": \"https://dms.customer.example/records/4711\",\n",
            "  \"schemaVersion\": 1,\n",
            "  \"suppliedAt\": \"2026-11-20T09:30:00Z\",\n",
            "  \"suppliedBy\": \"Quality manager\",\n",
            "  \"title\": \"Complaint record 001\"\n",
            "}\n"
        )
    );
    // Any letter case finds the one record, and the id comes back as submitted.
    let read = read_customer_artifact(&store, "COMPLAINT-2026-001")
        .unwrap()
        .unwrap();
    assert_eq!(read, stored.record);
    assert_eq!(read.artifact_id, "Complaint-2026-001");
    assert_eq!(read_customer_artifact(&store, "absent").unwrap(), None);
}

#[test]
#[trace("FR-030-AC-37")]
fn an_invalid_document_is_refused_before_any_file_is_written() {
    let mut cases: Vec<(&str, Value)> = Vec::new();
    for member in [
        "artifactId",
        "title",
        "location",
        "suppliedBy",
        "suppliedAt",
    ] {
        let mut missing = document();
        missing.as_object_mut().unwrap().remove(member);
        cases.push((member, missing));
        let mut non_string = document();
        non_string[member] = json!(7);
        cases.push((member, non_string));
    }
    for member in ["title", "location", "suppliedBy", "suppliedAt"] {
        let mut blank = document();
        blank[member] = json!("  ");
        cases.push((member, blank));
    }
    let mut unknown = document();
    unknown["digest"] = json!("sha256:00");
    cases.push(("digest", unknown));
    for (member, limit) in [("title", 256), ("suppliedBy", 256), ("location", 2048)] {
        let mut long = document();
        long[member] = json!("x".repeat(limit + 1));
        cases.push((member, long));
        let mut at_limit = document();
        at_limit[member] = json!("x".repeat(limit));
        write_customer_artifact(&mut MemoryEvidence::new(), &at_limit).unwrap();
    }
    for stamp in [
        "2026-11-20T09:30:00+01:00",
        "2026-11-20 09:30:00Z",
        "2026-02-30T00:00:00Z",
    ] {
        let mut bad = document();
        bad["suppliedAt"] = json!(stamp);
        cases.push(("suppliedAt", bad));
    }
    cases.push(("document", json!(["not", "an", "object"])));
    for (member, case) in cases {
        let mut store = MemoryEvidence::new();
        let error = write_customer_artifact(&mut store, &case).unwrap_err();
        assert_eq!(error.code(), EvidenceErrorCode::InvalidRecord, "{member}");
        assert!(store.paths().is_empty(), "{member} wrote a file");
    }
    for id in [
        "",
        " ",
        "has space",
        "under_score",
        "a/b",
        "..",
        "é",
        &"a".repeat(65),
    ] {
        let mut bad = document();
        bad["artifactId"] = json!(id);
        let mut store = MemoryEvidence::new();
        let error = write_customer_artifact(&mut store, &bad).unwrap_err();
        assert_eq!(refusal(error), CustomerArtifactRefusal::InvalidIdentifier);
        assert!(store.paths().is_empty(), "{id:?} wrote a file");
    }
    let mut longest = document();
    longest["artifactId"] = json!("a".repeat(64));
    write_customer_artifact(&mut MemoryEvidence::new(), &longest).unwrap();
}

#[test]
#[trace("FR-030-AC-38")]
fn the_store_is_append_only_at_a_file_name() {
    let mut store = MemoryEvidence::new();
    write_customer_artifact(&mut store, &document()).unwrap();

    let replay = write_customer_artifact(&mut store, &document()).unwrap();
    assert!(!replay.created, "identical replay is a no-op");

    let mut other_case = document();
    other_case["artifactId"] = json!("complaint-2026-001");
    let error = write_customer_artifact(&mut store, &other_case).unwrap_err();
    assert_eq!(refusal(error), CustomerArtifactRefusal::CaseCollision);

    let before = store
        .read("customer-artifacts/complaint-2026-001.json")
        .unwrap();
    let mut changed = document();
    changed["title"] = json!("Another title");
    let error = write_customer_artifact(&mut store, &changed).unwrap_err();
    assert_eq!(error.code(), EvidenceErrorCode::RecordIntegrity);
    assert_eq!(
        store
            .read("customer-artifacts/complaint-2026-001.json")
            .unwrap(),
        before
    );
}

/// FR-030-CON-4 and CON-5: a location the host cannot reach is still recorded,
/// and nothing about the artifact itself is stored.
#[test]
#[trace("FR-030-AC-39", "FR-030-CON-4", "FR-030-CON-5")]
fn the_record_holds_only_the_submitted_members_and_never_touches_the_location() {
    let repo = tempfile::tempdir().unwrap();
    let mut store = DiskEvidence::new(repo.path());
    for location in [
        "/nonexistent/path/to/the/artifact.pdf",
        "https://unreachable.invalid/a",
    ] {
        let mut submitted = document();
        submitted["location"] = json!(location);
        submitted["artifactId"] = json!(format!("a-{}", location.len()));
        let stored = write_customer_artifact(&mut store, &submitted).unwrap();
        let bytes = store.read(&stored.path).unwrap().unwrap();
        let mut expected = submitted.clone();
        expected["schemaVersion"] = json!(1);
        assert_eq!(serde_json::from_str::<Value>(&bytes).unwrap(), expected);
        for forbidden in ["digest", "hash", "sha256", "size", "content", "bytes"] {
            assert!(!bytes.contains(forbidden), "{forbidden} in {bytes}");
        }
    }
}

#[test]
#[trace("FR-030-AC-37")]
fn the_refusal_kinds_keep_their_contractual_spellings() {
    let spellings: Vec<&str> = CustomerArtifactRefusal::ALL
        .iter()
        .map(|kind| kind.as_str())
        .collect();
    assert_eq!(spellings, ["invalid-identifier", "case-collision"]);
}

#[test]
#[trace("FR-030-AC-40")]
fn gc_never_collects_a_customer_artifact() {
    let mut store = MemoryEvidence::new();
    // Two runs of one suite, so gc has a superseded record to retire and
    // writes a retirement log: the log check below can fail.
    for timestamp in ["2026-11-01T00:00:00Z", "2026-11-02T00:00:00Z"] {
        write_run(
            &mut store,
            &RunRecord {
                schema_version: STORE_SCHEMA_VERSION,
                suite: SuiteId::new("SUITE-001"),
                tool: "runner 1".to_owned(),
                evidence_kind: None,
                timestamp: timestamp.to_owned(),
                entries: Vec::new(),
            },
        )
        .unwrap();
    }
    write_customer_artifact(&mut store, &document()).unwrap();
    let mut other = document();
    other["artifactId"] = json!("second");
    write_customer_artifact(&mut store, &other).unwrap();
    for dry_run in [true, false] {
        let mut skipped = Vec::new();
        let deleted = gc(&mut store, dry_run, &mut skipped).unwrap();
        assert_eq!(
            deleted.len(),
            1,
            "gc collects the superseded run: {deleted:?}"
        );
        assert!(deleted.iter().all(|path| path.starts_with("runs/")));
    }
    assert!(
        read_customer_artifact(&store, "complaint-2026-001")
            .unwrap()
            .is_some()
    );
    assert!(read_customer_artifact(&store, "second").unwrap().is_some());
    let log: Vec<String> = store
        .paths()
        .into_iter()
        .filter(|path| path.starts_with("retired/"))
        .map(str::to_owned)
        .collect();
    assert_eq!(log.len(), 1, "gc wrote one retirement log: {log:?}");
    let logged = store.read(&log[0]).unwrap().unwrap();
    assert!(logged.contains("runs/SUITE-001"), "{logged}");
    assert!(!logged.contains("customer-artifacts"), "{logged}");
}

#[test]
#[trace("FR-030-AC-41")]
fn an_invalid_stored_file_is_an_error_naming_it_never_an_absent_record() {
    let good = {
        let mut store = MemoryEvidence::new();
        let stored = write_customer_artifact(&mut store, &document()).unwrap();
        store.read(&stored.path).unwrap().unwrap()
    };
    let path = "customer-artifacts/complaint-2026-001.json";

    // A mixed-case artifactId in a lowercase file name is valid.
    let valid = MemoryEvidence::new().with_file(path, good.clone());
    assert!(
        read_customer_artifact(&valid, "complaint-2026-001")
            .unwrap()
            .is_some()
    );

    let renamed = good.replace("Complaint-2026-001", "Complaint-2026-002");
    let unknown_member = good.replace("\"title\"", "\"extra\": 1,\n  \"title\"");
    let blank_title = good.replace("Complaint record 001", " ");
    let bad_charset = good.replace("Complaint-2026-001", "Complaint_2026_001");
    let broken: [(&str, String, EvidenceErrorCode); 5] = [
        (
            "id outside the charset",
            bad_charset,
            EvidenceErrorCode::InvalidRecord,
        ),
        (
            "not json",
            "<<<<<<< HEAD".to_owned(),
            EvidenceErrorCode::StoreRead,
        ),
        ("id differs", renamed, EvidenceErrorCode::RecordIntegrity),
        (
            "unknown member",
            unknown_member,
            EvidenceErrorCode::InvalidRecord,
        ),
        ("blank title", blank_title, EvidenceErrorCode::InvalidRecord),
    ];
    for (what, text, code) in broken {
        let store = MemoryEvidence::new().with_file(path, text);
        let error = read_customer_artifact(&store, "complaint-2026-001").unwrap_err();
        assert_eq!(error.code(), code, "{what}");
        assert!(error.to_string().contains(path), "{what}: {error}");
        // Recording over it is refused too, not treated as a free name.
        let mut store = store;
        assert!(
            write_customer_artifact(&mut store, &document()).is_err(),
            "{what}"
        );
    }
}
