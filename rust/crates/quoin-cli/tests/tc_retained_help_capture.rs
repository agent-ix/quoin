// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Frozen retained-shell contracts for Stage 8 (quoin#520).
//!
//! The capture was produced once by `scripts/capture-command-contracts.mjs`
//! before the oclif shell is retired. These tests never execute Node: Rust
//! reads the provenance-bearing record and replays native output against it.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test assertions deliberately panic to identify a broken shell contract"
)]

use std::process::{Command, Output};

use serde::Deserialize;

const CAPTURE: &str = include_str!("fixtures/retained-command-help.json");
const MIN_HELP_ROUTE_COUNT: usize = 60;
const MIN_SHELL_CASE_COUNT: usize = 8;

#[derive(Debug, Deserialize)]
struct Capture {
    schema_version: u64,
    captured_by: String,
    captured_revision: String,
    normalization: String,
    cases: Vec<CaptureCase>,
}

#[derive(Debug, Deserialize)]
struct CaptureCase {
    kind: String,
    argv: Vec<String>,
    exit: i32,
    stdout: String,
    stderr: String,
}

fn capture() -> Capture {
    serde_json::from_str(CAPTURE).expect("the retained command capture is JSON")
}

fn invoke(arguments: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(arguments)
        .output()
        .expect("the native quoin binary runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("the native shell emits UTF-8")
}

/// Preserve the only intentional capture normalizations: release identifiers
/// vary across build hosts, while command text and whitespace do not.
fn normalize_version(text: &str) -> String {
    let normalized_help = text
        .split_inclusive('\n')
        .map(|line| {
            let (body, newline) = line
                .strip_suffix('\n')
                .map_or((line, ""), |body| (body, "\n"));
            let trimmed = body.trim_start();
            if trimmed.starts_with("@agent-ix/quoin/") {
                format!(
                    "{}@agent-ix/quoin/<VERSION>{newline}",
                    &body[..body.len() - trimmed.len()]
                )
            } else {
                line.to_owned()
            }
        })
        .collect::<String>();
    let bare = normalized_help.trim_end_matches('\n');
    if bare.starts_with(|character: char| character.is_ascii_digit())
        && bare
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
        && bare.matches('.').count() >= 2
    {
        normalized_help.replacen(bare, "<VERSION>", 1)
    } else {
        normalized_help
    }
}

/// The fixture is a non-vacuous retained capture, with enough provenance to
/// determine exactly which shell produced it before Node disappears.
///
/// Trace: FR-101, FR-102, TC-1650
#[test]
fn tc_1650_retained_help_capture_is_provenanced_and_covers_every_native_route() {
    let capture = capture();
    assert_eq!(capture.schema_version, 1);
    assert_eq!(capture.captured_by, "scripts/capture-command-contracts.mjs");
    assert_eq!(capture.captured_revision.len(), 40);
    assert!(
        capture
            .captured_revision
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    );
    assert!(capture.normalization.contains("@agent-ix/quoin/<VERSION>"));
    assert!(capture.normalization.contains("<VERSION>"));
    let help_count = capture
        .cases
        .iter()
        .filter(|case| case.kind == "help")
        .count();
    assert!(
        help_count >= MIN_HELP_ROUTE_COUNT,
        "anti-vacuity: expected at least {MIN_HELP_ROUTE_COUNT} retained help routes, got {help_count}",
    );
    let shell_count = capture
        .cases
        .iter()
        .filter(|case| case.kind == "shell")
        .count();
    assert!(
        shell_count >= MIN_SHELL_CASE_COUNT,
        "anti-vacuity: expected at least {MIN_SHELL_CASE_COUNT} retained shell cases, got {shell_count}",
    );
    let mut seen = std::collections::BTreeSet::new();
    for case in &capture.cases {
        assert!(
            seen.insert(&case.argv),
            "duplicate argv in retained capture"
        );
        assert!(matches!(case.kind.as_str(), "help" | "shell"));
        if case.kind == "help" {
            assert_eq!(case.exit, 0, "{:?} must be successful help", case.argv);
            assert!(
                case.stderr.is_empty(),
                "{:?} wrote retained stderr",
                case.argv
            );
            assert!(
                !case.stdout.is_empty(),
                "{:?} has empty retained help",
                case.argv
            );
        }
    }
}

/// Root help is the first whole-byte replay over the frozen retained capture.
/// Route-specific renderers are added from the same fixture without invoking
/// the retained implementation again.
///
/// Trace: FR-003, FR-101, FR-102, TC-1650
#[test]
fn tc_1650_native_root_help_matches_the_retained_capture() {
    let capture = capture();
    let root = capture
        .cases
        .iter()
        .find(|case| case.kind == "help" && case.argv == ["--help"])
        .expect("the retained capture contains root --help");
    let output = invoke(&root.argv);
    assert_eq!(output.status.code(), Some(root.exit));
    assert_eq!(normalize_version(&text(&output.stdout)), root.stdout);
    assert_eq!(text(&output.stderr), root.stderr);
}

/// Every native-owned route replays its retained page from the frozen capture.
/// No Node process participates: this is the deletion-safe half of the Stage 8
/// command-contract evidence.
///
/// Trace: FR-003, FR-101, FR-102, TC-1650
#[test]
fn tc_1650_native_help_replays_every_retained_command_contract() {
    for case in capture()
        .cases
        .into_iter()
        .filter(|case| case.kind == "help")
    {
        let output = invoke(&case.argv);
        assert_eq!(output.status.code(), Some(case.exit), "{:?}", case.argv);
        assert_eq!(
            normalize_version(&text(&output.stdout)),
            case.stdout,
            "stdout for {:?}",
            case.argv
        );
        assert_eq!(
            text(&output.stderr),
            case.stderr,
            "stderr for {:?}",
            case.argv
        );
    }
}

/// The frozen non-help cases prove shell-level stream and status behavior that
/// route-help replay cannot exercise, without invoking the retained CLI.
///
/// Trace: FR-005, FR-101, FR-102, TC-1650
#[test]
fn tc_1650_native_shell_replays_the_retained_offline_contracts() {
    for case in capture()
        .cases
        .into_iter()
        .filter(|case| case.kind == "shell")
    {
        let output = invoke(&case.argv);
        assert_eq!(output.status.code(), Some(case.exit), "{:?}", case.argv);
        assert_eq!(
            normalize_version(&text(&output.stdout)),
            case.stdout,
            "stdout for {:?}",
            case.argv
        );
        assert_eq!(
            normalize_version(&text(&output.stderr)),
            case.stderr,
            "stderr for {:?}",
            case.argv
        );
    }
}
