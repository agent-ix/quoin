// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Supplier feature-rating status output and read-only behavior (PLAT-1636).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions report command-contract failures"
)]

use ix_trace_rs::trace;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("fixture has parent")).expect("parent created");
    std::fs::write(path, content).expect("fixture written");
}

fn snapshot(path: &Path, root: &Path, result: &mut BTreeMap<String, Vec<u8>>) {
    let entries = std::fs::read_dir(path).expect("tree readable");
    for entry in entries {
        let entry = entry.expect("entry readable");
        let child = entry.path();
        if child.is_dir() {
            snapshot(&child, root, result);
        } else {
            let relative = child
                .strip_prefix(root)
                .expect("file beneath tree")
                .to_string_lossy()
                .replace('\\', "/");
            result.insert(relative, std::fs::read(child).expect("file readable"));
        }
    }
}

fn invoke(root: &Path, json: bool) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
    command
        .args(["validation", "ratings", "--repo"])
        .arg(root)
        .args(["--repo-id", "agent-ix/quoin"]);
    if json {
        command.arg("--json");
    }
    command.output().expect("quoin binary runs")
}

fn invoke_coverage(root: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["validation", "ratings", "--coverage", "--json", "--repo"])
        .arg(root)
        .args(["--repo-id", "agent-ix/quoin"])
        .output()
        .expect("quoin binary runs")
}

#[trace("FR-235-AC-4")]
#[trace("FR-235-AC-11")]
#[test]
fn tc_235_ratings_coverage_distinguishes_high_not_high_unrated_and_invalid_markers() {
    let scratch = tempfile::tempdir().expect("temporary repository created");
    write(
        scratch.path(),
        "spec/functional/FR-001.md",
        "---\nid: FR-001\ntype: FR\n---\n| FR-001-AC-1 | One. | Test |\n",
    );
    write(
        scratch.path(),
        "spec/functional/FR-002.md",
        "---\nid: FR-002\ntype: FR\n---\n| FR-002-AC-1 | Two. | Test |\n",
    );
    write(
        scratch.path(),
        "spec/functional/FR-003.md",
        "---\nid: FR-003\ntype: FR\n---\n| FR-003-AC-1 | Three. | Test |\n",
    );
    write(
        scratch.path(),
        "spec/ratings/RR-1.md",
        "---\nid: RR-1\ntype: FeatureRiskRating\nscope: supplier\nrating: high_process_risk\nstatus: proposed\nrelationships:\n  - target: ix://agent-ix/quoin/FR-001\n    type: rates_feature\n---\n",
    );
    write(
        scratch.path(),
        "spec/ratings/RR-2.md",
        "---\nid: RR-2\ntype: FeatureRiskRating\nscope: supplier\nrating: not_high_process_risk\nstatus: proposed\nrelationships:\n  - target: ix://agent-ix/quoin/FR-002\n    type: rates_feature\n---\n",
    );
    write(
        scratch.path(),
        "selftest/cases/marked/run.sh",
        "#!/bin/sh\n# Trace: FR-001-AC-1\n# Trace: FR-999-AC-1\n# Trace: malformed\nexit 0\n",
    );
    let output = invoke_coverage(scratch.path());
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).expect("coverage JSON parses");
    let features = report["features"].as_array().expect("features listed");
    let row = |id| {
        features
            .iter()
            .find(|item| item["feature_id"] == id)
            .expect("feature listed")
    };
    assert_eq!(row("FR-001")["coverage"], "covered");
    assert_eq!(row("FR-002")["coverage"], "release_evidence", "{report}");
    assert_eq!(row("FR-003")["coverage"], "unrated");
    assert_eq!(
        report["invalid_markers"]
            .as_array()
            .expect("invalid listed")
            .len(),
        2
    );

    write(
        scratch.path(),
        "selftest/cases/marked/run.sh",
        "#!/bin/sh\n# Trace: FR-001-AC-1\nexit 0\n",
    );
    let output = invoke_coverage(scratch.path());
    assert_eq!(output.status.code(), Some(0));
    let report: Value = serde_json::from_slice(&output.stdout).expect("coverage JSON parses");
    let features = report["features"].as_array().expect("features listed");
    assert_eq!(
        features
            .iter()
            .find(|item| item["feature_id"] == "FR-001")
            .expect("high-risk feature listed")["coverage"],
        "covered"
    );

    write(
        scratch.path(),
        "selftest/cases/marked/run.sh",
        "#!/bin/sh\nexit 0\n",
    );
    let output = invoke_coverage(scratch.path());
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).expect("coverage JSON parses");
    assert_eq!(report["uncovered"], serde_json::json!(["FR-001"]));
    let features = report["features"].as_array().expect("features listed");
    assert_eq!(features[0]["coverage"], "uncovered");
}

#[trace("PLAT-1636-AC-6")]
#[trace("PLAT-1636-AC-7")]
#[test]
fn tc_1636_ratings_reports_each_status_and_is_read_only() {
    let scratch = tempfile::tempdir().expect("temporary repository created");
    assert!(!scratch.path().join(".git").exists());
    write(
        scratch.path(),
        "spec/functional/FR-001-a.md",
        "---\nid: FR-001\ntype: FR\n---\nA\n",
    );
    write(
        scratch.path(),
        "spec/functional/FR-002-b.md",
        "---\nid: FR-002\ntype: FR\n---\nB\n",
    );
    write(
        scratch.path(),
        "spec/functional/FR-003-c.md",
        "---\nid: FR-003\ntype: FR\n---\nC\n",
    );
    write(
        scratch.path(),
        "spec/ratings/RR-1.md",
        "---\nid: RR-1\ntype: FeatureRiskRating\nscope: supplier\nrelationships:\n  - target: ix://agent-ix/quoin/FR-002\n    type: rates_feature\n---\n",
    );
    write(
        scratch.path(),
        "spec/ratings/RR-2.md",
        "---\nid: RR-2\ntype: FeatureRiskRating\nscope: supplier\nrelationships:\n  - target: ix://agent-ix/quoin/FR-002\n    type: rates_feature\n---\n",
    );
    write(
        scratch.path(),
        "spec/ratings/RR-adopter.md",
        "---\nid: RR-A\ntype: FeatureRiskRating\nscope: adopter\nrelationships:\n  - target: ix://agent-ix/quoin/FR-001\n    type: rates_feature\n---\n",
    );
    write(
        scratch.path(),
        "spec/ratings/RR-3.md",
        "---\nid: RR-3\ntype: FeatureRiskRating\nscope: supplier\nrelationships:\n  - target: ix://agent-ix/quoin/FR-003\n    type: rates_feature\n---\n",
    );
    write(
        scratch.path(),
        "spec/ratings/RR-external.md",
        "---\nid: RR-external\ntype: FeatureRiskRating\nscope: supplier\nrelationships:\n  - target: ix://agent-ix/other-repo/FR-001\n    type: rates_feature\n---\n",
    );
    let mut before = BTreeMap::new();
    snapshot(scratch.path(), scratch.path(), &mut before);

    let output = invoke(scratch.path(), true);
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report parses");
    assert_eq!(report.as_object().expect("report is an object").len(), 2);
    let features = report
        .get("features")
        .and_then(Value::as_array)
        .expect("features listed");
    assert_eq!(
        features
            .iter()
            .map(|item| item["feature_id"].as_str().expect("feature ID is string"))
            .collect::<Vec<_>>(),
        ["FR-001", "FR-002", "FR-003"]
    );
    let state = |feature: &str| {
        features
            .iter()
            .find(|item| item.get("feature_id").and_then(Value::as_str) == Some(feature))
            .expect("feature state exists")
    };
    assert_eq!(state("FR-001")["status"], "unrated");
    assert_eq!(state("FR-001")["rating_ids"], serde_json::json!([]));
    assert_eq!(state("FR-002")["status"], "rated_more_than_once");
    assert_eq!(
        state("FR-002")["rating_ids"],
        serde_json::json!(["RR-1", "RR-2"])
    );
    assert_eq!(state("FR-003")["status"], "rated");
    assert_eq!(state("FR-003")["rating_ids"], serde_json::json!(["RR-3"]));

    let mut after = BTreeMap::new();
    snapshot(scratch.path(), scratch.path(), &mut after);
    assert_eq!(
        before, after,
        "validation ratings does not modify the repository tree"
    );
}

#[trace("PLAT-1636-AC-6")]
#[trace("PLAT-1636-AC-7")]
#[test]
fn tc_1636_unreadable_input_is_reported_in_json_and_human_output() {
    let scratch = tempfile::tempdir().expect("temporary repository created");
    assert!(!scratch.path().join(".git").exists());
    write(
        scratch.path(),
        "spec/functional/FR-001-a.md",
        "---\nid: FR-001\ntype: FR\n---\nA\n",
    );
    write(
        scratch.path(),
        "spec/functional/FR-002-broken.md",
        "---\nid: [invalid\ntype: FR\n---\nBroken frontmatter\n",
    );

    let json_output = invoke(scratch.path(), true);
    assert_eq!(json_output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&json_output.stdout).expect("JSON report parses");
    assert_eq!(report["features"][0]["feature_id"], "FR-001");
    assert_eq!(report["features"][0]["status"], "unrated");
    assert_eq!(
        report["unreadable"]
            .as_array()
            .expect("diagnostics listed")
            .len(),
        1
    );
    assert_eq!(
        report["unreadable"][0]["path"],
        "functional/FR-002-broken.md"
    );
    assert!(report["unreadable"][0]["reason"].as_str().is_some());

    let human_output = invoke(scratch.path(), false);
    assert_eq!(human_output.status.code(), Some(1));
    let stdout = String::from_utf8(human_output.stdout).expect("human output is UTF-8");
    assert!(stdout.contains("FR-001: unrated (rating IDs: none)"));
    assert!(stdout.contains("unreadable: functional/FR-002-broken.md: "));
}
