// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin evidence record --adapter licence-policy --policy` through the real
//! binary and a real store (FR-150).
//!
//! The SBOM is the Syft 1.57.1 capture quoin keeps with the evidence adapter
//! tests; the policy is a person-authored file written here.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const MODULE: &str = "name: licence-fixture\nversion: 0.1.0\ndescription: Fixture module\nartifact_types:\n  - name: FR\n    grammar_ref: iso-spec-core\n    defaults:\n      id_pattern: FR-{next:03d}\n";
const SPEC: &str = "---\nid: FR-001\ntype: FR\ntitle: One\n---\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n|----|----------|--------------|\n| FR-001-AC-1 | It works. |  |\n";
const POLICY: &str = "format: licence-policy-v1\nallow: [MIT]\ndeny: [GPL-3.0-only]\nban:\n  - purl: pkg:npm/fx-vale\n";

struct Fixture {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self {
            dir: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
        };
        std::fs::write(fixture.dir.path().join("manifest.yaml"), MODULE).unwrap();
        std::fs::create_dir_all(fixture.dir.path().join("spec")).unwrap();
        std::fs::write(fixture.dir.path().join("spec/FR-001-example.md"), SPEC).unwrap();
        fixture
    }

    fn sbom() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../quoin-evidence/tests/fixtures/licence-npm.syft.cdx.json")
    }

    fn policy(&self) -> PathBuf {
        let path = self.home.path().join("policy.yaml");
        std::fs::write(&path, POLICY).unwrap();
        path
    }

    fn record(&self, extra: &[&str]) -> Output {
        let sbom = Self::sbom();
        let mut arguments = vec![
            "evidence",
            "record",
            "--repo",
            ".",
            "--module",
            ".",
            "--suite",
            "SUITE-LIC",
            "--tool",
            "licence-policy",
            "--results",
            sbom.to_str().unwrap(),
            "--timestamp",
            "2026-01-01T00:00:00Z",
            "--json",
        ];
        arguments.extend_from_slice(extra);
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(&arguments)
            .current_dir(self.dir.path())
            .env("IX_HOME", self.home.path())
            .env("GIT_CEILING_DIRECTORIES", self.dir.path().parent().unwrap())
            .output()
            .unwrap()
    }

    fn files(&self, family: &str) -> bool {
        self.dir.path().join("spec/evidence").join(family).exists()
    }
}

/// The recorded scan holds the four licence and ban finding kinds and nothing
/// lands under `runs/`.
#[trace("FR-150-AC-1", "FR-150-AC-12")]
#[test]
fn tc_1671_040_the_command_records_a_scan_from_the_syft_capture() {
    let fixture = Fixture::new();
    let policy = fixture.policy();
    let output = fixture.record(&["--policy", policy.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["kind"], "scan");
    let mut rules: Vec<&str> = payload["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["ruleId"].as_str().unwrap())
        .collect();
    rules.sort_unstable();
    rules.dedup();
    assert_eq!(
        rules,
        [
            "licence-denied",
            "licence-not-allowed",
            "licence-unknown",
            "package-banned"
        ]
    );
    assert!(fixture.files("scans"));
    assert!(!fixture.files("runs"));
}

/// Omitting `--policy` exits non-zero with a usage message and records
/// nothing; `--policy` on another adapter names that adapter and records
/// nothing.
#[trace("FR-150-AC-6", "FR-150-AC-13")]
#[test]
fn tc_1671_041_a_missing_or_misplaced_policy_exits_non_zero_and_writes_nothing() {
    let fixture = Fixture::new();
    let missing = fixture.record(&[]);
    assert!(!missing.status.success());
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert!(stderr.contains("--policy <file> is required"), "{stderr}");

    let policy = fixture.policy();
    let misplaced = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "evidence",
            "record",
            "--repo",
            ".",
            "--module",
            ".",
            "--suite",
            "SUITE-X",
            "--tool",
            "syft",
            "--adapter",
            "sbom",
            "--results",
            Fixture::sbom().to_str().unwrap(),
            "--policy",
            policy.to_str().unwrap(),
        ])
        .current_dir(fixture.dir.path())
        .env("IX_HOME", fixture.home.path())
        .env(
            "GIT_CEILING_DIRECTORIES",
            fixture.dir.path().parent().unwrap(),
        )
        .output()
        .unwrap();
    assert!(!misplaced.status.success());
    let stderr = String::from_utf8_lossy(&misplaced.stderr);
    assert!(stderr.contains("sbom: --policy applies only"), "{stderr}");
    assert!(!fixture.files("scans") && !fixture.files("runs"));
}
