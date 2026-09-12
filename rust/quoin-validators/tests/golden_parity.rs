// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Verdict parity with the retained TypeScript (quoin#377).
//!
//! `tests/golden/cases.json` is hand-authored input; `tests/golden/expected.json`
//! was captured once from `src/validators/` at quoin `4d27dcf` and is the only
//! oracle consulted here. See `tests/golden/PROVENANCE.md`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use quoin_validators::{GateReport, Verdict, inspect_empty_gates};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Corpus {
    cases: Vec<CaseInput>,
}

#[derive(Debug, Deserialize)]
struct CaseInput {
    name: String,
    #[allow(dead_code, reason = "documentation carried in the corpus file")]
    covers: String,
    /// Relative path -> lines, joined with `\n` exactly as the capture did.
    files: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Expectations {
    cases: Vec<CaseExpectation>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CaseExpectation {
    name: String,
    findings: serde_json::Value,
    json: String,
    human: Vec<String>,
    strict_exit: u8,
}

fn golden_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden"))
}

fn load() -> (Corpus, Expectations) {
    let corpus: Corpus =
        serde_json::from_str(&fs::read_to_string(golden_dir().join("cases.json")).unwrap())
            .unwrap();
    let expected: Expectations =
        serde_json::from_str(&fs::read_to_string(golden_dir().join("expected.json")).unwrap())
            .unwrap();
    (corpus, expected)
}

fn materialise(case: &CaseInput, root: &Path) {
    for (relative, lines) in &case.files {
        let target = root.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, lines.join("\n")).unwrap();
    }
}

/// Exact verdict parity on the golden corpus: the serialised payload, the
/// operator-facing lines, and the strict exit code, case by case.
///
/// Trace: FR-096, TC-1067, TC-1068, TC-1069, TC-1070, TC-1071
/// Provenance: quoin#377
#[test]
fn tc_377_019_golden_corpus_matches_the_typescript_verdicts() {
    let (corpus, expected) = load();
    assert_eq!(
        corpus.cases.len(),
        expected.cases.len(),
        "cases.json and expected.json disagree on how many cases exist"
    );

    for (case, expectation) in corpus.cases.iter().zip(&expected.cases) {
        assert_eq!(case.name, expectation.name, "corpus files are out of order");

        let scratch = tempfile::tempdir().unwrap();
        materialise(case, scratch.path());

        let report = GateReport::new(
            inspect_empty_gates(scratch.path())
                .unwrap_or_else(|error| panic!("case {}: {error}", case.name)),
        );

        // Structural parity: the finding list, field for field.
        let actual = serde_json::to_value(&report.findings).unwrap();
        assert_eq!(
            actual, expectation.findings,
            "case {}: finding payload differs from the TypeScript",
            case.name
        );

        // Byte parity of `quoin validate --json`.
        let json = serde_json::to_string_pretty(&report).unwrap();
        assert_eq!(
            json, expectation.json,
            "case {}: --json stdout differs from the TypeScript",
            case.name
        );

        // The operator-facing lines and the strict verdict.
        assert_eq!(
            report.human_lines(),
            expectation.human,
            "case {}: human output differs from the TypeScript",
            case.name
        );
        assert_eq!(
            report.verdict(true).exit_code(),
            expectation.strict_exit,
            "case {}: --strict exit code differs from the TypeScript",
            case.name
        );
        assert_eq!(
            report.verdict(false).exit_code(),
            0,
            "case {}: findings are advisory unless --strict",
            case.name
        );
    }
}

/// The corpus asserts over a non-empty population on both sides of the verdict.
///
/// Without this, trimming `cases.json` down to only-clean repositories would
/// leave `tc_377_019` green while it measured nothing.
///
/// Trace: FR-096
/// Provenance: quoin#377
#[test]
fn tc_377_020_corpus_covers_both_verdicts() {
    let (corpus, expected) = load();
    let with_findings = expected
        .cases
        .iter()
        .filter(|case| case.strict_exit == 1)
        .count();
    let clean = expected.cases.len() - with_findings;
    let total_findings: usize = expected
        .cases
        .iter()
        .filter_map(|case| case.findings.as_array())
        .map(Vec::len)
        .sum();

    assert!(corpus.cases.len() >= 46, "corpus shrank below its floor");
    assert!(with_findings >= 23, "too few cases expect a finding");
    assert!(clean >= 23, "too few cases expect a clean verdict");
    assert!(total_findings >= 28, "too few findings asserted overall");
}

/// A repository root that does not exist is a refusal, not a clean verdict.
///
/// This is the one place the port deliberately improves on the TypeScript,
/// which lets a raw `ENOENT` escape: the caller now gets a stable code.
///
/// Trace: FR-096
/// Provenance: quoin#377
#[test]
fn tc_377_021_missing_root_is_a_coded_refusal() {
    let scratch = tempfile::tempdir().unwrap();
    let missing = scratch.path().join("nope");
    let error = inspect_empty_gates(&missing).unwrap_err();
    assert_eq!(
        error.code(),
        quoin_validators::ErrorCode::RepoRootUnreadable
    );
    assert_eq!(error.path(), missing);
}

/// An empty report is clean, advisory-clean, and prints the no-findings line.
///
/// Trace: TC-1070
/// Provenance: quoin#377
#[test]
fn tc_377_022_empty_report_shape() {
    let report = GateReport::default();
    assert_eq!(report.verdict(false), Verdict::Clean);
    assert_eq!(report.verdict(true), Verdict::Clean);
    assert_eq!(
        report.human_lines(),
        vec!["repository QA gates: no findings"]
    );
    assert_eq!(
        serde_json::to_string_pretty(&report).unwrap(),
        "{\n  \"findings\": []\n}"
    );
}
