// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Ported help routes remain byte-for-byte equal to the retained CLI fixture.

#![allow(
    clippy::expect_used,
    reason = "test fixtures use expect as the failure report"
)]

use ix_trace_rs::trace;
use serde::Deserialize;
use std::process::Command;

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    kind: String,
    argv: Vec<String>,
    exit: i32,
    stdout: String,
    stderr: String,
}

fn normalize(text: &str) -> String {
    let version = env!("CARGO_PKG_VERSION");
    text.replace(
        &format!("@agent-ix/quoin/{version}"),
        "@agent-ix/quoin/<VERSION>",
    )
    .replace(version, "<VERSION>")
}

#[trace("FR-102-CON-3")]
#[test]
fn retained_help_surface_is_unchanged_across_the_native_port() {
    let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/retained-command-help.json");
    let fixture: Fixture = serde_json::from_slice(
        &std::fs::read(fixture_path).expect("retained command fixture is readable"),
    )
    .expect("retained command fixture is valid");
    let cases: Vec<_> = fixture
        .cases
        .into_iter()
        .filter(|case| case.kind == "help")
        .collect();
    assert!(!cases.is_empty(), "fixture has captured help routes");
    for case in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(&case.argv)
            .output()
            .expect("native quoin executes");
        assert_eq!(output.status.code(), Some(case.exit), "{:?}", case.argv);
        let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
        let stderr = String::from_utf8(output.stderr).expect("stderr is UTF-8");
        assert_eq!(
            normalize(&stdout),
            case.stdout,
            "stdout changed for {:?}",
            case.argv
        );
        assert_eq!(
            normalize(&stderr),
            case.stderr,
            "stderr changed for {:?}",
            case.argv
        );
    }
}
