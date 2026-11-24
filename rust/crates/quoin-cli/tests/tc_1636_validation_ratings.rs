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

fn invoke(root: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["validation", "ratings", "--repo"])
        .arg(root)
        .arg("--json")
        .output()
        .expect("quoin binary runs")
}

#[trace("PLAT-1636-AC-6")]
#[trace("PLAT-1636-AC-7")]
#[test]
fn tc_1636_ratings_reports_each_status_and_is_read_only() {
    let scratch = tempfile::tempdir().expect("temporary repository created");
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
    let mut before = BTreeMap::new();
    snapshot(scratch.path(), scratch.path(), &mut before);

    let output = invoke(scratch.path());
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report parses");
    let features = report
        .get("features")
        .and_then(Value::as_array)
        .expect("features listed");
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
