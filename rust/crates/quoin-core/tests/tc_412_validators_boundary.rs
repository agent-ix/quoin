// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `validators.run` across a real pipe, over the captured TypeScript verdicts
//! (quoin#412).
//!
//! # Why this test exists after the difftest already passed
//!
//! `quoin-difftest` compares the binary against the RETAINED TypeScript, and
//! this ticket deletes that TypeScript — so the difftest's `validators.run`
//! cases are retired in the same commit as the code they oracle. FR-101 is
//! satisfied at the cutover revision by construction, and what remains
//! afterwards must be a gate that does not need Node.
//!
//! That gate is this one. `quoin-validators`' golden corpus was captured from
//! `src/validators/` at quoin `4d27dcf` and is checked in; driving the real
//! `quoin-core` binary over it asserts that the operator-visible answer still
//! matches the TypeScript's, at the boundary rather than in the library, for
//! as long as the corpus is kept. `quoin-validators`' own `tc_377_019` runs
//! the same corpus through the library over a temp directory; this one runs it
//! through the request shape, so the two together say the seam did not change
//! the answer.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Corpus {
    cases: Vec<CaseInput>,
}

#[derive(Debug, Deserialize)]
struct CaseInput {
    name: String,
    /// Relative path -> lines. This IS `RunRequest::files`' value shape, which
    /// is why the request carries a file map rather than a repository path.
    files: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Expectations {
    cases: Vec<CaseExpectation>,
}

#[derive(Debug, Deserialize)]
struct CaseExpectation {
    name: String,
    findings: serde_json::Value,
}

fn golden_dir() -> &'static Path {
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quoin-validators/tests/golden"
    ))
}

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

/// Every captured TypeScript verdict, reproduced by the real binary from a
/// request carrying the file map.
///
/// Trace: FR-096, FR-101
/// Provenance: quoin#412, quoin#377
#[test]
fn tc_412_the_boundary_reproduces_every_captured_typescript_verdict() {
    let corpus: Corpus =
        serde_json::from_str(&std::fs::read_to_string(golden_dir().join("cases.json")).unwrap())
            .unwrap();
    let expected: Expectations =
        serde_json::from_str(&std::fs::read_to_string(golden_dir().join("expected.json")).unwrap())
            .unwrap();
    assert_eq!(corpus.cases.len(), expected.cases.len());
    // A comparison over an empty population is green and says nothing; the
    // corpus is 46 cases and a path typo would otherwise read as a pass.
    assert!(
        corpus.cases.len() >= 40,
        "the golden corpus came back with {} cases; it is not being read",
        corpus.cases.len()
    );

    for (case, expectation) in corpus.cases.iter().zip(&expected.cases) {
        assert_eq!(case.name, expectation.name);
        let request = serde_json::json!({ "files": case.files });
        let result = run("validators.run", &request.to_string());

        assert_eq!(result.status, 0, "{}: {}", case.name, result.stderr);
        assert_eq!(result.stderr, "", "{}", case.name);
        let payload: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
        assert_eq!(
            payload["findings"], expectation.findings,
            "{} diverged from the captured TypeScript verdict",
            case.name
        );
    }
}

/// Findings are a successful answer: exit 0, whatever the corpus holds.
///
/// The taxonomy decision of `ops::validators`, observed where a caller observes
/// it — the process exit status — rather than only in a library return value.
/// `Verdict::Failing` is also 1 and `Outcome::Partial` is also 1; conflating
/// them would make `quoin validate` fail CI without `--strict`.
///
/// Trace: FR-096
/// Provenance: quoin#412
#[test]
fn tc_412_a_repository_with_findings_still_exits_zero() {
    let request = serde_json::json!({
        "files": {
            "Makefile": ["gate:", "\t./scripts/check.sh"],
            "scripts/check.sh": [
                "#!/bin/sh",
                "# Gate for FR-001-AC-1: no production symbol shall call `unwrap`.",
                "grep -rn \"unwrap()\" src/ | wc -l"
            ]
        }
    });
    let result = run("validators.run", &request.to_string());
    assert_eq!(result.status, 0);
    assert_eq!(result.stderr, "");
    let payload: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(payload["findings"].as_array().unwrap().len(), 1);
}

/// The refusal taxonomy as a caller meets it: a root the caller could not list
/// exits 2, a subtree it could not list exits 4, and neither writes a payload.
///
/// Trace: FR-096
/// Provenance: quoin#412
#[test]
fn tc_412_unreadable_inputs_exit_by_whose_mistake_it_was() {
    let root = run("validators.run", r#"{"files":{},"unlistable":[""]}"#);
    assert_eq!(root.status, 2);
    assert_eq!(root.stdout, "");
    let diagnostics: serde_json::Value = serde_json::from_str(&root.stderr).unwrap();
    assert_eq!(diagnostics[0]["code"], "CORE_REFUSED");
    assert_eq!(diagnostics[0]["context"]["validator_code"], "QV-E001");

    let subtree = run("validators.run", r#"{"files":{},"unlistable":["scripts"]}"#);
    assert_eq!(subtree.status, 4);
    assert_eq!(subtree.stdout, "");
    let diagnostics: serde_json::Value = serde_json::from_str(&subtree.stderr).unwrap();
    assert_eq!(diagnostics[0]["code"], "CORE_IO");
    assert_eq!(diagnostics[0]["context"]["validator_code"], "QV-E002");
}
