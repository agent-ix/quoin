// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! An audit report whose finding names an unknown stale `cause` is refused at
//! the graph parse boundary, naming `cause` — not accepted and then failed in
//! canonicalization (FR-115-AC-22).
//!
//! Provenance: PLAT-1760

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use quoin_graph_analysis::AuditEnvelope;
use serde_json::{Value, json};

fn envelope(finding: &Value) -> Value {
    json!({
        "format": "quoin-audit-envelope",
        "format_version": 1,
        "export": { "format": "quire-assurance", "format_version": 1, "modules": [] },
        "report": { "findings": [finding], "healthy": [], "unevaluated": [] },
    })
}

fn stale(cause: &Value) -> Value {
    json!({
        "kind": "stale-evidence", "obligation": "FR-1-AC-1", "severity": "high",
        "summary": "s", "cause": cause,
    })
}

#[trace("FR-115-AC-22")]
#[test]
fn an_unknown_cause_is_refused_at_parse_naming_the_cause_member() {
    for bad in [json!("not-a-cause"), json!(7), json!(null)] {
        let error = AuditEnvelope::parse(&envelope(&stale(&bad))).unwrap_err();
        assert!(
            error.starts_with("report.findings.0.cause:"),
            "{bad}: {error}"
        );
    }
}

#[trace("FR-115-AC-22")]
#[test]
fn a_known_cause_and_an_absent_cause_both_parse() {
    for cause in ["failed-run", "missing-run"] {
        let parsed = AuditEnvelope::parse(&envelope(&stale(&json!(cause))));
        assert!(parsed.is_ok(), "{cause}: {:?}", parsed.err());
    }
    let mut bare = stale(&json!("failed-run"));
    bare.as_object_mut().unwrap().remove("cause");
    let parsed = AuditEnvelope::parse(&envelope(&bare));
    assert!(parsed.is_ok(), "{:?}", parsed.err());
}
