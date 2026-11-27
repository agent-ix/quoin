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

fn command_names(help: &str) -> Vec<(String, String)> {
    let mut section = None;
    let mut names = Vec::new();
    for line in help.lines() {
        let heading = line.trim().trim_end_matches(':');
        if matches!(heading, "TOPICS" | "COMMANDS") {
            section = Some(heading.to_owned());
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with("  ") && !line.starts_with("    ") {
            if let (Some(section), Some(name)) = (section.as_ref(), line.split_whitespace().next())
            {
                names.push((section.clone(), name.to_owned()));
            }
        } else if !line.starts_with(' ') {
            section = None;
        }
    }
    names.sort();
    names.dedup();
    names
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

#[trace("FR-102-CON-3")]
#[test]
fn tc_1650_root_command_inventory_matches_the_retained_shell() {
    let capture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/retained-command-help.json"))
            .expect("retained command help is valid JSON");
    let retained = capture["cases"]
        .as_array()
        .expect("retained cases")
        .iter()
        .find(|case| {
            case["argv"]
                .as_array()
                .is_some_and(|argv| argv.len() == 1 && argv[0] == "--help")
        })
        .and_then(|case| case["stdout"].as_str())
        .expect("retained root help case");

    let output = invoke(&["--help"]);
    assert!(output.status.success(), "stderr: {}", text(&output.stderr));
    let current = text(&output.stdout);
    let retained = command_names(retained);
    let current: std::collections::BTreeSet<_> = command_names(&current).into_iter().collect();
    let missing: Vec<_> = retained
        .into_iter()
        .filter(|command| !current.contains(command))
        .collect();
    assert!(
        missing.is_empty(),
        "retained root topics or commands disappeared during the port: {missing:?}"
    );
}

/// Unknown command refusals are not parser-success output: they retain exit 2
/// and send the unresolved name and a usable command catalog to stderr.
#[trace("FR-005", "FR-102", "FR-005-AC-1", "FR-102-AC-5")]
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
