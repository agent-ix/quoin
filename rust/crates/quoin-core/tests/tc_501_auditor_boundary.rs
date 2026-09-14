// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The three `auditor.*` routes, each driven through the real binary.
//!
//! # Why by name, through the process
//!
//! `auditor.audit` and `auditor.baseline` run the SAME audit over the SAME
//! `input` member and differ only in what they answer with, so a swapped match
//! arm in `dispatch` would return a well-formed payload of the wrong shape and
//! leave every unit test in `src/ops/auditor/tests.rs` green — the condition
//! `tc_447_operation_census.rs` exists to make impossible. Each route is run by
//! its own name here and asserted on a member only its own handler produces.
//!
//! # The store rides on stdin
//!
//! There is no fixture tree and no repo path, because there is nothing for one
//! to hold: `quoin-auditor` is `Inert`. It opens no path, spawns nothing and
//! reads no environment, so the whole evidence store arrives in the request and
//! the domain is granted no host capability at all.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// One invocation of the boundary binary.
struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

fn run(op: &str, stdin: &str) -> Run {
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
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    Run {
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
        status: out.status.code().unwrap(),
    }
}

fn payload(result: &Run) -> Value {
    assert_eq!(result.status, 0, "{}", result.stderr);
    serde_json::from_str(&result.stdout).unwrap()
}

/// One obligation with no binding: an `undischarged` finding, and nothing else.
///
/// Deliberately the smallest input that produces a finding at all. A request
/// that produced none would assert over an empty population, which is the
/// vacuity FR-101 names.
fn input() -> Value {
    json!({
        "obligations": [{
            "id": "FR-001-AC-1",
            "statement": "The parser rejects an unterminated string.",
            "statement_hash": "a1b2c3",
        }],
        "bindings": [],
        "runs": [],
    })
}

/// `auditor.audit` reports the unbound obligation, and says no ratchet applied.
///
/// Trace: FR-096-AC-1, FR-101-AC-2
/// Provenance: agent-ix/quoin#501
#[test]
fn tc_501_600_audit_reports_an_unbound_obligation_through_the_binary() {
    let result = run("auditor.audit", &json!({ "input": input() }).to_string());
    let payload = payload(&result);
    let findings = payload["report"]["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{payload}");
    assert_eq!(findings[0]["kind"], "undischarged");
    assert_eq!(findings[0]["obligation"], "FR-001-AC-1");
    // Absent, not `[]`. No baseline was sent, so the caller must be able to
    // tell "the whole backlog" from "nothing was new" (agent-ix/quoin#169).
    assert!(payload.get("reported").is_none(), "{payload}");
}

/// A baseline naming that finding suppresses it, and an empty one does not.
///
/// Trace: FR-096-AC-1
/// Provenance: agent-ix/quoin#169, agent-ix/quoin#501
#[test]
fn tc_501_601_a_baseline_suppresses_exactly_the_keys_it_names() {
    let accepted = run("auditor.baseline", &json!({ "input": input() }).to_string());
    let keys: Vec<String> = serde_json::from_value(payload(&accepted)["accepted"].clone()).unwrap();
    assert_eq!(keys, vec!["undischarged:FR-001-AC-1".to_owned()]);

    let suppressed = run(
        "auditor.audit",
        &json!({ "input": input(), "accepted": keys }).to_string(),
    );
    assert_eq!(
        payload(&suppressed)["reported"].as_array().unwrap().len(),
        0
    );

    // An empty baseline WAS read and accepted nothing, so the finding is new.
    let empty = run(
        "auditor.audit",
        &json!({ "input": input(), "accepted": [] }).to_string(),
    );
    assert_eq!(payload(&empty)["reported"].as_array().unwrap().len(), 1);
}

/// `auditor.advise` answers one row per obligation, in the order they arrived.
///
/// Trace: FR-096-AC-1, FR-054
/// Provenance: agent-ix/quoin#501
#[test]
fn tc_501_602_advise_answers_one_row_per_obligation_through_the_binary() {
    let request = json!({
        "catalog": { "methods": [], "classes": [], "unreadable": [], "duplicates": [] },
        "obligations": [
            {
                "id": "FR-001-AC-1",
                "statement": "The parser rejects an unterminated string.",
                "statement_hash": "a1b2c3",
            },
            {
                "id": "FR-002-AC-1",
                "statement": "The renderer emits stable output.",
                "statement_hash": "d4e5f6",
            },
        ],
    });
    let payload = payload(&run("auditor.advise", &request.to_string()));
    let advice = payload["advice"].as_array().unwrap();
    assert_eq!(advice.len(), 2, "{payload}");
    assert_eq!(advice[0]["obligation"], "FR-001-AC-1");
    assert_eq!(advice[1]["obligation"], "FR-002-AC-1");
}

/// `auditor.vocabulary` states what the fact set can mint, non-empty and sorted.
///
/// The floor is the point: an empty vocabulary would make the catalog↔fact-set
/// census (agent-ix/quoin#128) pass over nothing, which is the vacuity FR-101
/// names. It is a floor and not an equality — the set grows as rules are added,
/// and pinning it exactly would make every new rule a test edit.
///
/// Trace: FR-096-AC-1, FR-054
/// Provenance: agent-ix/quoin#128, agent-ix/quoin#501
#[test]
fn tc_501_603_vocabulary_states_what_the_fact_set_can_mint() {
    let payload = payload(&run("auditor.vocabulary", "{}"));
    let minted: Vec<String> =
        serde_json::from_value(payload["mintableCharacteristics"].clone()).unwrap();
    assert!(minted.len() >= 20, "{minted:?}");
    let mut sorted = minted.clone();
    sorted.sort();
    assert_eq!(minted, sorted, "the vocabulary is not sorted");
    // Three values nothing lexical produces: they are read from the
    // obligation's criticality and from the evidence store, and a fact set
    // that dropped them would still look like a full vocabulary.
    for expected in [
        "fault-detection-failed",
        "fault-detection-unmeasured",
        "high-criticality",
    ] {
        assert!(minted.iter().any(|v| v == expected), "missing {expected}");
    }
}
