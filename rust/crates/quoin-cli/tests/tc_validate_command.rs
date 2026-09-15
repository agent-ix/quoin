// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Shipped-command replay of the captured validator corpus.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions report command-contract failures"
)]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Corpus {
    cases: Vec<CaseInput>,
}

#[derive(Debug, Deserialize)]
struct CaseInput {
    name: String,
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

fn materialize(root: &Path, files: &BTreeMap<String, Vec<String>>) {
    for (relative, lines) in files {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("fixture path has a parent"))
            .expect("fixture parent is created");
        std::fs::write(path, lines.join("\n")).expect("fixture file is written");
    }
}

fn invoke(root: &Path, strict: bool) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
    command.args(["validate", "--repo"]);
    command.arg(root);
    command.arg("--json");
    if strict {
        command.arg("--strict");
    }
    command.output().expect("the native quoin binary runs")
}

/// The captured TypeScript verdict corpus must be observable through the
/// shipped `quoin validate` command, not a retired private protocol binary.
///
/// Trace: FR-098, FR-101, FR-102
/// Provenance: quoin#412, quoin#377, quoin#521
#[test]
fn tc_521_validate_replays_every_captured_validator_verdict() {
    let corpus: Corpus =
        serde_json::from_str(&std::fs::read_to_string(golden_dir().join("cases.json")).unwrap())
            .unwrap();
    let expected: Expectations =
        serde_json::from_str(&std::fs::read_to_string(golden_dir().join("expected.json")).unwrap())
            .unwrap();
    assert_eq!(corpus.cases.len(), expected.cases.len());
    assert!(
        corpus.cases.len() >= 40,
        "the captured corpus is empty or unreadable"
    );

    for (case, expectation) in corpus.cases.iter().zip(&expected.cases) {
        assert_eq!(case.name, expectation.name);
        let scratch = tempfile::tempdir().unwrap();
        materialize(scratch.path(), &case.files);
        let output = invoke(scratch.path(), false);
        assert!(output.status.success(), "{}: {:?}", case.name, output);
        assert!(
            output.stderr.is_empty(),
            "{}: stderr was not empty",
            case.name
        );
        let observed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            observed
                .get("findings")
                .expect("validate JSON names findings"),
            &expectation.findings,
            "{} diverged from the captured TypeScript verdict",
            case.name
        );
    }
}

/// Findings remain advisory unless the explicit strict flag turns them into a
/// partial command result.
///
/// Trace: FR-102
#[test]
fn tc_521_validate_keeps_findings_advisory_until_strict() {
    let scratch = tempfile::tempdir().unwrap();
    materialize(
        scratch.path(),
        &BTreeMap::from([
            (
                "Makefile".to_owned(),
                vec!["gate:".to_owned(), "\t./scripts/check.sh".to_owned()],
            ),
            (
                "scripts/check.sh".to_owned(),
                vec![
                    "#!/bin/sh".to_owned(),
                    "# Gate for FR-001-AC-1: no production symbol shall call `unwrap`.".to_owned(),
                    "grep -rn \"unwrap()\" src/ | wc -l".to_owned(),
                ],
            ),
        ]),
    );

    assert!(invoke(scratch.path(), false).status.success());
    assert_eq!(invoke(scratch.path(), true).status.code(), Some(1));
}
