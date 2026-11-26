// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Observable native-shell contracts that a parser unit test cannot prove.

#![allow(
    clippy::expect_used,
    reason = "test assertions deliberately panic to report a failed command contract"
)]

use std::process::{Command, Output};

use ix_trace_rs::trace;

fn invoke(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(arguments)
        .output()
        .expect("the native quoin binary runs")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("the native shell emits UTF-8")
}

/// The root help response keeps the retained stdout-only shell contract and
/// advertises the native command inventory, not Clap's synthetic help route.
#[trace("FR-003", "FR-102")]
#[test]
fn tc_1650_root_help_is_a_successful_stdout_catalogue() {
    let output = invoke(&["--help"]);
    assert!(output.status.success(), "stderr: {}", text(&output.stderr));
    assert_eq!(output.stderr.len(), 0);
    let stdout = text(&output.stdout);
    assert!(stdout.starts_with("Spec-driven development for Claude Code"));
    assert!(stdout.contains("VERSION\n  @agent-ix/quoin/"));
    assert!(stdout.contains("USAGE\n  $ quoin [COMMAND]"));
    assert!(stdout.contains("TOPICS\n  assurance"));
    assert!(stdout.contains("COMMANDS\n  advise"));
    assert!(!stdout.contains("help            "));
}

/// Unknown command refusals are not parser-success output: they retain exit 2
/// and send the unresolved name and a usable command catalog to stderr.
#[trace("FR-005", "FR-102", "FR-005-AC-1")]
#[test]
fn tc_1650_unknown_command_is_a_stderr_refusal_with_the_root_catalogue() {
    let output = invoke(&["not-a-command"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout.len(), 0);
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("command not-a-command not found"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Usage: quoin <command> [options]"),
        "{stderr}"
    );
    assert!(
        stderr.contains("validation") && stderr.contains("write"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Run `quoin <command> --help` for details."),
        "{stderr}"
    );
}

/// Oclif reports the entire unresolved command path with colon separators,
/// while excluding flags from that identity.
#[trace("FR-005", "FR-102", "FR-005-AC-2")]
#[test]
fn tc_1650_nested_unknown_command_keeps_the_oclif_path_identity() {
    let output = invoke(&["catalog", "not-a-command", "--format", "json"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout.len(), 0);
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("command catalog:not-a-command not found"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Usage: quoin <command> [options]"),
        "{stderr}"
    );
    assert!(
        stderr.contains("validation") && stderr.contains("write"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Run `quoin <command> --help` for details."),
        "{stderr}"
    );
}
