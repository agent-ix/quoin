// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `matrix.build` through the production runtime, by its wire name (FR-115).
//!
//! What the library tests cannot see is asserted here: that the name reaches
//! the handler, the request refusals and their exit classes, the
//! contradictory-audit refusal's context, and that the canonical payload is
//! byte-identical across two runs over byte-identical input.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_core::runtime::{RuntimeSettings, dispatch};
use serde_json::{Value, json};

struct Run {
    stdout: String,
    diagnostics: Vec<Diagnostic>,
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
        diagnostics: response.diagnostics,
        status: i32::from(response.outcome.code()),
    }
}

/// Two requirements, one bound and one stale criterion, one criterion with no
/// method and no evidence at all.
fn request() -> Value {
    json!({
        "coverage": [{ "document": "spec/FR.md", "criteria": [
            { "id": "FR-002-AC-1", "statement": "s", "method": "Test", "binders": [], "status": "untagged" },
            { "id": "FR-001-AC-1", "statement": "s", "method": "Test", "binders": [
                { "path": "t.rs", "line": 1, "column": 1, "qualified_name": "tc_1", "kind": "function" }
            ], "status": "tagged" },
            { "id": "FR-001-AC-2", "statement": "s", "binders": [], "status": "untagged" },
        ]}],
        "bindings": { "schemaVersion": 1, "bindings": [
            { "obligation": "FR-001-AC-1", "statementHashAtBinding": "h", "suite": "unit", "commit": "c1", "symbols": ["tc_1"] },
            { "obligation": "FR-002-AC-1", "statementHashAtBinding": "h", "suite": "unit", "commit": "c0", "symbols": ["tc_2"] },
        ]},
        "audit": {
            "findings": [{ "kind": "stale-evidence", "obligation": "FR-002-AC-1", "severity": "medium", "summary": "behind HEAD" }],
            "healthy": ["FR-001-AC-1"],
            "unevaluated": [],
        },
    })
}

/// The whole payload, as bytes, twice.
///
/// Trace: FR-115-AC-1, FR-115-AC-10
#[test]
fn tc_1080_100_two_runs_over_one_input_emit_identical_canonical_bytes() {
    let stdin = request().to_string();
    let first = run("matrix.build", &stdin);
    let second = run("matrix.build", &stdin);
    assert_eq!(first.status, 0, "{:?}", first.diagnostics);
    assert!(first.diagnostics.is_empty());
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(
        first.stdout,
        concat!(
            r#"{"requirements":[{"criteria":["#,
            r#"{"binders":[{"column":1,"kind":"function","line":1,"path":"t.rs","qualified_name":"tc_1"}],"#,
            r#""evidence_detail":{"bindings":[{"commit":"c1","suite":"unit"}],"findings":[],"unevaluated":[]},"#,
            r#""evidence_status":"bound","id":"FR-001-AC-1","method":"Test","static_status":"tagged"},"#,
            r#"{"binders":[],"evidence_detail":{"bindings":[],"findings":[],"unevaluated":[]},"#,
            r#""evidence_status":"undischarged","id":"FR-001-AC-2","static_status":"untagged"}],"id":"FR-001"},"#,
            r#"{"criteria":[{"binders":[],"evidence_detail":{"bindings":[{"commit":"c0","suite":"unit"}],"#,
            r#""findings":[{"kind":"stale-evidence","summary":"behind HEAD"}],"unevaluated":[]},"#,
            r#""evidence_status":"stale","id":"FR-002-AC-1","method":"Test","static_status":"untagged"}],"id":"FR-002"}]}"#,
        )
    );
}

/// A missing required field or an unknown one is a bad request naming the
/// operation; a request over the ceiling is refused with its size.
///
/// Trace: FR-115-AC-2
#[test]
fn tc_1080_101_the_request_shape_is_enforced() {
    for broken in [
        {
            let mut request = request();
            request.as_object_mut().unwrap().remove("bindings");
            request
        },
        {
            let mut request = request();
            request.as_object_mut().unwrap().remove("audit");
            request
        },
        {
            let mut request = request();
            request["extra"] = json!(true);
            request
        },
    ] {
        let result = run("matrix.build", &broken.to_string());
        assert_eq!(result.status, 3, "{broken}");
        assert_eq!(result.stdout, "");
        assert_eq!(result.diagnostics[0].code, "CORE_BAD_REQUEST");
        assert_eq!(result.diagnostics[0].context["op"], "matrix.build");
    }

    let mut oversized = request();
    oversized["audit"]["healthy"] = json!(["x".repeat(17 * 1024 * 1024)]);
    let result = run("matrix.build", &oversized.to_string());
    assert_eq!(result.status, 2);
    assert_eq!(result.stdout, "");
    let context = &result.diagnostics[0].context;
    assert_eq!(result.diagnostics[0].code, "CORE_REFUSED");
    assert_eq!(context["op"], "matrix.build");
    assert_eq!(context["limit_bytes"], (16 * 1024 * 1024).to_string());
    assert!(context["observed_bytes"].parse::<usize>().unwrap() > 16 * 1024 * 1024);
}

/// A contradictory audit exits 2 with `reason: contradictory-audit` and the
/// offending id, and emits no partial matrix.
///
/// Trace: FR-115-AC-5, FR-115-CON-4
#[test]
fn tc_1080_102_a_contradictory_audit_is_refused_with_its_reason() {
    for contradiction in [
        json!({ "findings": [], "unevaluated": [
            { "check": "mocked-confirmation", "obligation": "FR-001-AC-1", "suites": ["unit"], "reason": "r" }
        ]}),
        json!({ "unevaluated": [], "findings": [
            { "kind": "undischarged", "obligation": "FR-001-AC-1", "summary": "s" }
        ]}),
    ] {
        let mut request = request();
        for (key, value) in contradiction.as_object().unwrap() {
            request["audit"][key] = value.clone();
        }
        let result = run("matrix.build", &request.to_string());
        assert_eq!(result.status, 2);
        assert_eq!(result.stdout, "", "no partial matrix");
        let diagnostic = &result.diagnostics[0];
        assert_eq!(diagnostic.code, "CORE_REFUSED");
        assert_eq!(diagnostic.context["reason"], "contradictory-audit");
        assert_eq!(diagnostic.context["obligation"], "FR-001-AC-1");
        assert_eq!(diagnostic.context["op"], "matrix.build");
    }
}
