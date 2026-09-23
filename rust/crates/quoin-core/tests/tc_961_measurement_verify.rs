// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `measurement.verify` end to end through the production runtime (FR-108,
//! PLAT-961): a gate-stage plan with an objective and a decision rule, a
//! collection published through `measurement.record`, a verdict that accepts
//! it, and the same store with one observation tampered, which it rejects
//! with a typed reason.
//!
//! The plan and the collection are the committed fixtures the checker's own
//! corpus uses (`quoin-measurement/tests/fixtures/verify/`), copied into a
//! temporary repository so the store may be written to.
//!
//! Provenance: PLAT-961

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::path::{Path, PathBuf};

use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_core::runtime::{RuntimeSettings, dispatch};
use serde_json::{Value, json};

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

fn run(op: &str, stdin: &str) -> Run {
    let request: Value = serde_json::from_str(stdin).unwrap();
    let response =
        dispatch(op, &request, &RuntimeSettings::default()).unwrap_or_else(|error| Response {
            payload: Value::Null,
            diagnostics: vec![Diagnostic::from(&error)],
            outcome: error.outcome(),
        });
    Run {
        stdout: if response.outcome.carries_payload() {
            canonical_json(&response.payload).unwrap()
        } else {
            String::new()
        },
        stderr: if response.diagnostics.is_empty() {
            String::new()
        } else {
            canonical_json(&response.diagnostics).unwrap()
        },
        status: i32::from(response.outcome.code()),
    }
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../quoin-measurement/tests/fixtures/verify")
}

/// A repository holding the gate plan and nothing else.
fn repository() -> tempfile::TempDir {
    let temporary = tempfile::tempdir().unwrap();
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).unwrap();
    std::fs::copy(
        fixtures().join("plans/MP-961-gate.md"),
        assurance.join("MP-961-gate.md"),
    )
    .unwrap();
    temporary
}

fn verify(repo: &Path, claimed: Option<&str>) -> (i32, Value, String) {
    let result = run(
        "measurement.verify",
        &json!({ "repo": repo.to_str().unwrap(), "plan": "MP-961", "claimed": claimed })
            .to_string(),
    );
    let payload = serde_json::from_str(&result.stdout).unwrap_or(Value::Null);
    (result.status, payload, result.stderr)
}

/// Trace: FR-108-AC-2, FR-108-AC-4, FR-108-AC-7
/// Provenance: PLAT-961
#[test]
fn tc_961_018_record_verify_accepts_then_a_tampered_observation_is_rejected() {
    let repo = repository();
    let mut record: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("right/accept/1.json")).unwrap(),
    )
    .unwrap();
    // The fixture is the stored form; intake computes `protectedApparatus`
    // itself and refuses a candidate that states it (PLAT-975).
    record["verificationStack"]
        .as_object_mut()
        .unwrap()
        .remove("protectedApparatus");
    let recorded = run(
        "measurement.record",
        &json!({ "repo": repo.path().to_str().unwrap(), "record": record }).to_string(),
    );
    assert_eq!(recorded.status, 0, "{}", recorded.stderr);

    let (status, accepted, stderr) = verify(repo.path(), Some("accept"));
    assert_eq!(status, 0, "{stderr}");
    assert_eq!(accepted["verdict"], "accept");
    assert_eq!(accepted["reasons"], json!([]));
    assert_eq!(accepted["counts"]["observationsRecomputed"], 1);
    assert_eq!(accepted["counts"]["collectionsConsidered"], 1);

    // Tamper with the stored observation: the rows still say 9 of 10, and
    // 0.5 is not 0.9 at the one decimal it states.
    let stored = repo
        .path()
        .join("spec/evidence/measurements/verify-accept.json");
    let mut permissions = std::fs::metadata(&stored).unwrap().permissions();
    #[allow(
        clippy::permissions_set_readonly_false,
        reason = "the test deliberately edits a published collection to tamper with it"
    )]
    permissions.set_readonly(false);
    std::fs::set_permissions(&stored, permissions).unwrap();
    let mut collection: Value =
        serde_json::from_str(&std::fs::read_to_string(&stored).unwrap()).unwrap();
    collection["observations"][0]["value"] = json!(0.5);
    std::fs::write(&stored, collection.to_string()).unwrap();

    let (status, rejected, stderr) = verify(repo.path(), Some("accept"));
    assert_eq!(status, 1, "a rejection carries its payload with exit 1");
    assert_eq!(rejected["verdict"], "reject");
    assert_eq!(
        rejected["reasons"],
        json!(["value_disagrees_with_rows", "claimed_verdict_disagrees"])
    );
    let diagnostics: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(diagnostics[0]["code"], "CORE_REJECTED");
    assert_eq!(diagnostics[0]["context"]["verdict"], "reject");
}

/// Trace: FR-108-AC-7
/// Provenance: PLAT-961
#[test]
fn tc_961_019_an_unknown_plan_or_claim_is_refused_without_a_verdict() {
    let repo = repository();
    let unknown = run(
        "measurement.verify",
        &json!({ "repo": repo.path().to_str().unwrap(), "plan": "MP-0" }).to_string(),
    );
    assert_eq!(unknown.status, 2, "{}", unknown.stderr);
    assert!(unknown.stdout.is_empty());
    let claim = run(
        "measurement.verify",
        &json!({ "repo": repo.path().to_str().unwrap(), "plan": "MP-961", "claimed": "pass" })
            .to_string(),
    );
    assert_eq!(claim.status, 3, "{}", claim.stderr);
    // No collection yet: inconclusive, with the payload, never accept.
    let (status, empty, stderr) = verify(repo.path(), None);
    assert_eq!(status, 1);
    assert_eq!(empty["verdict"], "inconclusive");
    assert_eq!(empty["reasons"], json!(["no_collections"]));
    assert_eq!(empty["orderSource"], "none");
    let diagnostics: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(diagnostics[0]["code"], "CORE_INCONCLUSIVE");
}

/// Trace: FR-108-AC-2, FR-108-AC-7
/// Provenance: PLAT-961
#[test]
fn tc_961_029_a_collection_filed_under_another_id_is_refused() {
    let repo = repository();
    let measurements = repo.path().join("spec/evidence/measurements");
    std::fs::create_dir_all(&measurements).unwrap();
    std::fs::copy(
        fixtures().join("right/accept/1.json"),
        measurements.join("not-its-id.json"),
    )
    .unwrap();
    let refused = run(
        "measurement.verify",
        &json!({ "repo": repo.path().to_str().unwrap(), "plan": "MP-961" }).to_string(),
    );
    assert_eq!(refused.status, 2, "{}", refused.stderr);
    assert!(refused.stdout.is_empty());
    assert!(refused.stderr.contains("not-its-id"), "{}", refused.stderr);
}
