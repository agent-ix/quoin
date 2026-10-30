// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The shipped `quoin validate` command keeps findings advisory until strict.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions report command-contract failures"
)]

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

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
