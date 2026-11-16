// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The verification report's optional module selection matches the matrix
//! command (FR-191, FR-196).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use ix_trace_rs::trace;
use quoin_assurance::matrix::MatrixOutput;

#[trace("FR-191-AC-1", "FR-196-AC-1")]
#[test]
fn omitting_module_uses_the_scoped_module_as_the_matrix_does() {
    let scope =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../quoin-core/tests/fixtures/quire-repo");
    let work = tempfile::tempdir().unwrap();
    for explicit in [false, true] {
        let mut matrix = Command::new(env!("CARGO_BIN_EXE_quoin"));
        matrix
            .current_dir(work.path())
            .args(["matrix", "--json", "--repo"])
            .arg(&scope);
        if explicit {
            matrix.arg("--module").arg(&scope);
        }
        let output = matrix.output().expect("quoin matrix starts");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let source: MatrixOutput = serde_json::from_slice(&output.stdout).unwrap();
        let out = work
            .path()
            .join(if explicit { "explicit" } else { "scoped" });
        let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
        command.current_dir(work.path()).args([
            "dossier",
            "generate",
            "--doc",
            "verification-report",
            "--version",
            "1.0",
            "--json",
            "--scope",
        ]);
        command.arg(&scope).arg("--out").arg(&out);
        if explicit {
            command.arg("--module").arg(&scope);
        }
        let output = command.output().expect("quoin starts");
        assert_eq!(
            output.status.code(),
            Some(1),
            "explicit module: {explicit}; stdout: {}; stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let text = fs::read_to_string(out.join("DOS-VR.md")).unwrap();
        assert!(text.contains("## FR-001: A requirement stating two obligations"));
        assert!(text.contains("| FR-001-AC-1 |"));
        assert!(text.contains("| FR-001-AC-2 |"));
        for criterion in &source.requirements[0].criteria {
            let row = text
                .lines()
                .find(|line| line.starts_with(&format!("| {} |", criterion.id)))
                .unwrap();
            assert!(row.ends_with(&format!("| {} |", criterion.evidence_status.as_str())));
        }
    }
}
