// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PLAT-1021: a graded run recorded through `quoin measurement record` is
//! decided by `quoin measurement verify` against the recomputed
//! constant-predictor baseline.
//!
//! The corpus is a small graded fixture authored for this test: 11
//! `weakness_kind` and 4 `coverage` items whose best constants give a
//! baseline of `(9 + 3) / 15 = 0.8`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use std::path::Path;
use std::process::{Command, Output};

use quoin_jev::{ConstantPredictorItem, GradedRun, record_input};
use serde_json::Value;

const PLAN: &str = "---\n\
id: MP-1021\n\
title: Graded run over the constant predictor\n\
type: MeasurementPlan\n\
status: active\n\
owner: test\n\
stage: gate\n\
metric: jev.criterion-strength.agreement-rate\n\
definition_version: graded-run-v1\n\
protected_apparatus:\n\
\x20\x20- spec/assurance/MP-1021.md\n\
negative_controls:\n\
\x20\x20- kind: apparatus-edit\n\
\x20\x20\x20\x20description: the plan document defines the measurement\n\
ground_truth_kind: human-labelled\n\
statistical_design:\n\
\x20\x20population: every graded item\n\
\x20\x20minimum_population: 10\n\
\x20\x20sampling: exhaustive\n\
\x20\x20repetitions: 1\n\
\x20\x20estimator: proportion\n\
\x20\x20error_model: binomial\n\
\x20\x20uncertainty: wilson interval\n\
\x20\x20decision_rule:\n\
\x20\x20\x20\x20comparator: gt\n\
\x20\x20\x20\x20baseline: constant-predictor\n\
\x20\x20\x20\x20margin: 0.05\n\
\x20\x20answer_families:\n\
\x20\x20\x20\x20- weakness_kind\n\
\x20\x20\x20\x20- coverage\n\
---\n\
\n\
# Graded run\n";

/// The fixture corpus. The tool answers each item correctly except the
/// first `wrong` items, which it answers `unknown`.
fn corpus(wrong: usize) -> Vec<ConstantPredictorItem> {
    let weakness: [(&str, &[&str]); 11] = [
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("sound", &["sound"]),
        ("gap", &["gap", "sound"]),
        ("gap", &["gap", "sound"]),
        ("gap", &["gap", "sound"]),
        ("weakness", &["weakness"]),
        ("weakness", &["weakness"]),
    ];
    let coverage: [(&str, &[&str]); 4] =
        [("2", &["2"]), ("2", &["2"]), ("2", &["2"]), ("1", &["1"])];
    let rows = weakness
        .into_iter()
        .enumerate()
        .map(|(index, row)| (format!("w{index}"), "weakness_kind", row))
        .chain(
            coverage
                .into_iter()
                .enumerate()
                .map(|(index, row)| (format!("c{index}"), "coverage", row)),
        );
    rows.enumerate()
        .map(
            |(index, (item_id, family, (expected, contested)))| ConstantPredictorItem {
                item_id,
                family: family.to_owned(),
                expected: expected.to_owned(),
                contested: contested.iter().map(|label| (*label).to_owned()).collect(),
                actual: if index < wrong { "unknown" } else { expected }.to_owned(),
            },
        )
        .collect()
}

fn quoin(repo: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(arguments)
        .args(["--repo", repo.to_str().unwrap()])
        .output()
        .unwrap()
}

fn git(repo: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args([
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .arg("-C")
        .arg(repo)
        .args(arguments)
        .status()
        .unwrap();
    assert!(status.success(), "git {arguments:?}");
}

/// Grade `wrong` items wrong, record the run, commit it, and return the
/// verify exit code and verdict document.
fn verdict_of(wrong: usize) -> (Option<i32>, Value) {
    let repo = tempfile::tempdir().unwrap();
    let assurance = repo.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).unwrap();
    std::fs::write(assurance.join("MP-1021.md"), PLAN).unwrap();
    git(repo.path(), &["init", "--quiet"]);
    git(repo.path(), &["add", "spec"]);
    git(repo.path(), &["commit", "--quiet", "-m", "plan"]);

    let run = GradedRun {
        plan_id: "MP-1021",
        definition_version: "graded-run-v1",
        metric: "jev.criterion-strength.agreement-rate",
        collection_id: "run-1021",
        subject: "fixture corpus",
        tool_version: "0.0.0",
        timestamp: "2026-11-11T00:00:00.000Z",
        plan_path: "spec/assurance/MP-1021.md",
    };
    let input = repo.path().join("input.json");
    std::fs::write(
        &input,
        record_input(&run, &corpus(wrong)).unwrap().to_string(),
    )
    .unwrap();
    let recorded = quoin(
        repo.path(),
        &["measurement", "record", "--input", input.to_str().unwrap()],
    );
    assert!(
        recorded.status.success(),
        "{}",
        String::from_utf8_lossy(&recorded.stderr)
    );
    std::fs::remove_file(input).unwrap();
    git(repo.path(), &["add", "spec"]);
    git(repo.path(), &["commit", "--quiet", "-m", "run"]);

    let output = quoin(repo.path(), &["measurement", "verify", "--plan", "MP-1021"]);
    let payload = serde_json::from_slice(&output.stdout).unwrap();
    (output.status.code(), payload)
}

/// Provenance: PLAT-1021
#[trace("FR-134-AC-2", "TC-2801")]
#[test]
fn tc_1021_101_a_recorded_graded_run_is_decided_against_the_recomputed_baseline() {
    // 15/15 agreement clears the 0.8 baseline plus the 0.05 margin.
    let (status, accepted) = verdict_of(0);
    assert_eq!(accepted["verdict"], "accept", "{accepted}");
    assert_eq!(status, Some(0));
    assert_eq!(accepted["decisions"][0]["baseline"], 0.8);

    // 12/15 equals the baseline, so the margin is not cleared.
    let (status, rejected) = verdict_of(3);
    assert_eq!(rejected["verdict"], "reject", "{rejected}");
    assert_eq!(status, Some(1));
    assert!(
        rejected["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason == "rule_not_met"),
        "{rejected}"
    );
    assert_eq!(rejected["decisions"][0]["baseline"], 0.8);
}
