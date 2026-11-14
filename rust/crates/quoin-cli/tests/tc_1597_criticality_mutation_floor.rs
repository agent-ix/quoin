// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic is the failure report"
)]

//! Criticality reaches the mutation floor through Quoin's production assembly
//! and the real spec-artifacts-process manifest (PLAT-1597).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::Value;

const REQUIREMENT: &str = "---\nid: FR-1597\ntitle: Rated obligation fixture\ntype: FR\n{criticality}---\n\
## Acceptance Criteria\n\n\
| ID | Criteria | Verification |\n\
|----|----------|--------------|\n\
| FR-1597-AC-1 | The audited obligation keeps its declared rating. | mutation-testing |\n";

fn process_module() -> PathBuf {
    let checkout = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-artifacts-process"),
        PathBuf::from,
    );
    let module = checkout.join("spec_artifacts_process");
    assert!(
        module.join("manifest.yaml").is_file(),
        "the real spec-artifacts-process module is not at {}; set \
         SPEC_ARTIFACTS_PROCESS_DIR to its repository checkout",
        module.display()
    );
    std::fs::canonicalize(module).unwrap()
}

struct Fixture {
    repo: tempfile::TempDir,
    home: tempfile::TempDir,
    results: tempfile::TempDir,
}

impl Fixture {
    fn new(criticality: &str) -> Self {
        let repo = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let results = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(repo.path().join("spec/functional")).unwrap();
        let frontmatter = if criticality.is_empty() {
            String::new()
        } else {
            format!("criticality: {criticality}\n")
        };
        std::fs::write(
            repo.path()
                .join("spec/functional/FR-1597-rated-obligation.md"),
            REQUIREMENT.replace("{criticality}", &frontmatter),
        )
        .unwrap();
        Self {
            repo,
            home,
            results,
        }
    }

    fn invoke(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(args)
            .current_dir(self.repo.path())
            .env("IX_HOME", self.home.path())
            .env(
                "GIT_CEILING_DIRECTORIES",
                self.repo.path().parent().unwrap(),
            )
            .output()
            .unwrap()
    }

    fn record_score(&self) {
        let path = self.results.path().join("score.json");
        std::fs::write(
            &path,
            r#"{"entries":[{"symbol":"mutation::sample","outcome":"pass","traceIds":["FR-1597-AC-1"],"score":0.5,"metric":"mutation-score"}]}"#,
        )
        .unwrap();
        let module = process_module();
        let output = self.invoke(&[
            "evidence",
            "record",
            "--repo",
            ".",
            "--module",
            module.to_str().unwrap(),
            "--suite",
            "SUITE-1597",
            "--tool",
            "fixture",
            "--adapter",
            "entries",
            "--results",
            path.to_str().unwrap(),
            "--timestamp",
            "2026-01-01T00:00:00Z",
        ]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "recording the score: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let output = self.invoke(&[
            "evidence",
            "inspect-mocks",
            "--repo",
            ".",
            "--suite",
            "SUITE-1597",
            "--timestamp",
            "2026-01-01T00:00:00Z",
        ]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "recording the empty mock inspection: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn audit(&self) -> Value {
        let module = process_module();
        let output = self.invoke(&[
            "evidence",
            "audit",
            "--repo",
            ".",
            "--module",
            module.to_str().unwrap(),
            "--mutation-floor",
            "high=0.8",
            "--json",
        ]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "audit: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

#[trace("FR-039-AC-2")]
#[test]
fn tc_1597_001_real_manifest_rating_derives_floor_finding() {
    let fixture = Fixture::new("high");
    fixture.record_score();

    let report = fixture.audit();
    let findings = report["findings"].as_array().unwrap();
    let floor_finding: Vec<_> = findings
        .iter()
        .filter(|finding| {
            finding["kind"] == "insufficient-mutation-score"
                && finding["obligation"] == "FR-1597-AC-1"
        })
        .collect();
    assert_eq!(
        floor_finding.len(),
        1,
        "a high-rated derived obligation with score 0.5 must fail high=0.8: {report}"
    );
}

#[trace("FR-039-AC-7")]
#[test]
fn tc_1597_002_absent_rating_does_not_apply_high_floor() {
    let fixture = Fixture::new("");
    fixture.record_score();

    let report = fixture.audit();
    assert!(
        !report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["kind"] == "insufficient-mutation-score"
                    && finding["obligation"] == "FR-1597-AC-1"
            }),
        "an unrated obligation has no high-specific floor: {report}"
    );
}
