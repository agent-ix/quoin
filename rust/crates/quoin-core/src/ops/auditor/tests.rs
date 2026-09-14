// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `auditor` domain, decided over payloads and nothing else.
//!
//! No test here touches disk, and none can: the domain is granted no
//! capability, so there is nothing to substitute and nothing to stub. That is
//! the boundary property worth pinning — `quoin_auditor` is `Inert`, and this
//! module inherits it.
//!
//! What the audit and the advice DECIDE is `quoin-auditor`'s and is pinned
//! there against the captured oracle corpus. What is pinned here is the
//! boundary: that a request reaches those functions unchanged, that the
//! baseline-absent and baseline-empty cases stay distinguishable, and that a
//! refusal reaches the caller as the status the retained commands exited with.
//!
//! Provenance: quoin#501

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use serde_json::{Value, json};

use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::{Outcome, Response};

use super::{MAX_AUDITOR_REQUEST_BYTES, advise, audit, baseline};

/// An obligation nothing is bound to, which the audit reports as unbound.
fn unbound(id: &str) -> Value {
    json!({
        "id": id,
        "statement": "The system shall do the thing.",
        "statement_hash": "sha256:0000",
    })
}

/// The payload of a clean answer.
fn payload(response: Response) -> Value {
    assert_eq!(
        response.outcome,
        Outcome::Ok,
        "the operation answered, so the outcome is clean"
    );
    assert!(
        response.diagnostics.is_empty(),
        "a clean answer carries no diagnostic"
    );
    response.payload
}

/// Trace: FR-101-AC-1
/// Provenance: quoin#501
#[test]
fn an_audit_reports_an_unbound_obligation_without_a_baseline() {
    let answer =
        payload(audit(&json!({ "input": { "obligations": [unbound("FR-001-AC-1")] } })).unwrap());
    assert!(
        !answer["report"]["findings"].as_array().unwrap().is_empty(),
        "an obligation with no binding is a finding"
    );
    assert!(
        answer.get("reported").is_none(),
        "no baseline was read, so there is no ratcheted list to carry; the \
         caller reports `report.findings` rather than the same list twice"
    );
}

/// An empty baseline is not an absent one (agent-ix/quoin#169).
///
/// Trace: FR-101-AC-1
/// Provenance: quoin#501
#[test]
fn an_empty_baseline_is_read_and_accepts_nothing() {
    let answer = payload(
        audit(&json!({
            "input": { "obligations": [unbound("FR-001-AC-1")] },
            "accepted": [],
        }))
        .unwrap(),
    );
    let reported = answer["reported"].as_array().unwrap();
    let all = answer["report"]["findings"].as_array().unwrap();
    assert_eq!(
        reported.len(),
        all.len(),
        "a baseline that accepted nothing suppresses nothing"
    );
    assert!(
        !reported.is_empty(),
        "the empty-baseline case must not be mistaken for the absent one, \
         which carries no `reported` at all"
    );
}

/// Trace: FR-101-AC-1
/// Provenance: quoin#501
#[test]
fn a_baseline_suppresses_exactly_the_keys_it_accepted() {
    let input = json!({ "obligations": [unbound("FR-001-AC-1")] });
    let keys = payload(baseline(&json!({ "input": input.clone() })).unwrap());
    let accepted = keys["accepted"].as_array().unwrap().clone();
    assert!(!accepted.is_empty(), "the baseline accepted the finding");

    let answer =
        payload(audit(&json!({ "input": input, "accepted": Value::Array(accepted) })).unwrap());
    assert!(
        answer["reported"].as_array().unwrap().is_empty(),
        "every finding was in the baseline, so a ratcheted run reports none"
    );
}

/// Trace: FR-101-AC-1
/// Provenance: quoin#501
#[test]
fn baseline_keys_come_back_sorted() {
    let answer = payload(
        baseline(&json!({
            "input": {
                "obligations": [
                    unbound("FR-003-AC-1"),
                    unbound("FR-001-AC-1"),
                    unbound("FR-002-AC-1"),
                ],
            },
        }))
        .unwrap(),
    );
    let keys: Vec<&str> = answer["accepted"]
        .as_array()
        .unwrap()
        .iter()
        .map(|key| key.as_str().unwrap())
        .collect();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    assert_eq!(keys, sorted, "the baseline file records keys in order");
}

/// Trace: FR-101-AC-1
/// Provenance: quoin#501
#[test]
fn advice_comes_back_one_row_per_obligation_in_the_order_given() {
    let answer = payload(
        advise(&json!({
            "catalog": { "methods": [], "duplicates": [], "unreadable": [] },
            "obligations": [unbound("FR-002-AC-1"), unbound("FR-001-AC-1")],
        }))
        .unwrap(),
    );
    let advice = answer["advice"].as_array().unwrap();
    assert_eq!(advice.len(), 2);
    assert_eq!(advice[0]["obligation"], "FR-002-AC-1");
    assert_eq!(advice[1]["obligation"], "FR-001-AC-1");
    assert_eq!(
        answer["degraded"], false,
        "no diagnostic carried the reason, so nothing degraded"
    );
}

/// An engine predating quire-rs CR-091 emits the reason with no `value`.
///
/// Trace: FR-101-AC-1
/// Provenance: quoin#501
#[test]
fn advice_says_when_the_engine_could_not_classify_the_vocabulary() {
    let answer = payload(
        advise(&json!({
            "catalog": { "methods": [], "duplicates": [], "unreadable": [] },
            "obligations": [],
            "diagnostics": [{ "reason": "uncatalogued-verification-method" }],
        }))
        .unwrap(),
    );
    assert_eq!(
        answer["degraded"], true,
        "the caller warns on this; it is a fact about the engine, not a row"
    );
}

/// Trace: FR-101-AC-3
/// Provenance: quoin#501
#[test]
fn a_field_the_boundary_would_drop_is_refused_rather_than_ignored() {
    for request in [
        json!({ "input": {}, "acceptd": [] }),
        json!({ "input": {}, "extra": 1 }),
    ] {
        let error = audit(&request).unwrap_err();
        assert_eq!(
            error.code,
            CoreErrorCode::BadRequest,
            "a misspelled field the caller believes it sent must not be \
             silently dropped"
        );
        assert_eq!(
            error.context.get("op").map(String::as_str),
            Some("auditor.audit")
        );
    }
}

/// Trace: FR-101-AC-3
/// Provenance: quoin#501
#[test]
fn every_operation_names_itself_on_a_bad_request() {
    /// One route: its wire spelling and the handler `dispatch` sends it to.
    type Route = (&'static str, fn(&Value) -> Result<Response, CoreError>);

    let cases: [Route; 3] = [
        ("auditor.audit", audit),
        ("auditor.baseline", baseline),
        ("auditor.advise", advise),
    ];
    for (op, operation) in cases {
        let error = operation(&json!("not an object")).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest);
        assert_eq!(error.context.get("op").map(String::as_str), Some(op));
    }
}

/// Trace: FR-101-AC-3
/// Provenance: quoin#501
#[test]
fn the_request_ceiling_is_a_store_and_not_a_filename() {
    // The number itself, asserted rather than recomputed: a ceiling that
    // followed the code it bounds would agree with any change to it.
    assert_eq!(MAX_AUDITOR_REQUEST_BYTES, 32 * 1024 * 1024);
}
