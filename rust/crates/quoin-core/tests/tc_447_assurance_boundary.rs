// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `assurance.build_case` feeds `assurance.render_case` through the production
//! runtime (quoin#447).
//!
//! The inputs are the hand-authored cases in
//! `quoin-assurance/tests/golden/cases.json`; both operations are live code and
//! no recorded answer is read. Error PROSE is never asserted: a refusal asserts
//! its exit class and its diagnostic code and stops there.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::path::Path;

use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_core::runtime::{RuntimeSettings, dispatch};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Corpus {
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    name: String,
    op: String,
    /// Marks an input the Rust REQUEST SCHEMA refuses.
    #[serde(default)]
    boundary: Option<String>,
    #[serde(default)]
    input: Option<Value>,
}

fn golden_dir() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quoin-assurance/tests/golden"
    ))
}

fn read<T: for<'de> Deserialize<'de>>(file: &str) -> T {
    serde_json::from_str(&std::fs::read_to_string(golden_dir().join(file)).unwrap()).unwrap()
}

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
    let stdout = if response.outcome.carries_payload() {
        canonical_json(&response.payload).unwrap()
    } else {
        String::new()
    };
    let stderr = if response.diagnostics.is_empty() {
        String::new()
    } else {
        canonical_json(&response.diagnostics).unwrap()
    };
    Run {
        stdout,
        stderr,
        status: i32::from(response.outcome.code()),
    }
}

/// A built case is a renderable case, except where its assessments are
/// untyped.
///
/// `assurance.render_case`'s input IS `assurance.build_case`'s output, and the
/// two handlers deserialise it with different types (`CaseInput` against
/// `RenderableCase`). Feeding one straight into the other is the only check
/// that those two agree at the boundary.
///
/// `build_case` treats `producer_trust` and `evidence_independence` as OPAQUE
/// (`serde_json::Value`); the renderer's `TrustAssessment` and
/// `IndependenceAssessment` are typed, so a case whose assessment is malformed
/// is refused by the renderer's request schema and nowhere else.
///
/// Trace: FR-040
/// Provenance: quoin#447
#[test]
fn tc_447_512_a_built_case_is_a_renderable_case_unless_its_assessments_are_untyped() {
    let corpus: Corpus = read("cases.json");

    for case in &corpus.cases {
        if case.op != "build_case" || case.boundary.is_some() {
            continue;
        }
        let input = case.input.clone().unwrap();
        let built = run("assurance.build_case", &input.to_string());
        assert_eq!(built.status, 0, "{}: {}", case.name, built.stderr);

        let result = run("assurance.render_case", &built.stdout);
        if result.status == 0 {
            continue;
        }

        // Only the assessments are opaque to `build_case`; every other part of
        // its payload is typed on both sides, so a case carrying neither
        // assessment must round-trip.
        assert!(
            input.get("producer_trust").is_some() || input.get("evidence_independence").is_some(),
            "{}: build_case produced a payload render_case refuses, \
             and the input carries no assessment to explain it: {}",
            case.name,
            result.stderr
        );
        assert_eq!(result.status, 3, "{}", case.name);
        assert_eq!(result.stdout, "", "{}", case.name);
        let diagnostics: Value = serde_json::from_str(&result.stderr).unwrap();
        assert_eq!(diagnostics[0]["code"], "CORE_BAD_REQUEST", "{}", case.name);
        assert_eq!(
            diagnostics[0]["context"]["op"], "assurance.render_case",
            "{}: the refusal came from somewhere other than the renderer's request schema",
            case.name
        );
    }
}
