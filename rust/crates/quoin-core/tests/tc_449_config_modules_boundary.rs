// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The remaining `config` operations reached by their wire names.
//!
//! `tc_447_600_every_operation_is_invoked_by_name_by_some_test` requires every
//! entry in `OPERATIONS` to be handed to the real binary, spelled, by some
//! integration test — because a swapped match arm leaves the drift guard, the
//! unit tests and native fixture replays all green and breaks only for a user.
//! The two module routes moved to the shipped CLI in quoin#521; this file keeps
//! the two config routes until their command-path replacements land.
//!
//! Each test therefore asserts something only its own handler could have
//! written. A payload shape shared with a sibling operation would satisfy the
//! census while leaving the swap undetected.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::io::Write as _;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

/// Invoke the binary with `op` on argv and `request` on stdin.
fn run(op: &str, request: &Value) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin-core"));
    command.arg(op);
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    Run {
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
        status: out.status.code().unwrap(),
    }
}

/// The payload of a run that must have succeeded outright.
fn ok(result: &Run) -> Value {
    assert_eq!(result.status, 0, "{}", result.stderr);
    assert_eq!(result.stderr, "");
    serde_json::from_str(&result.stdout).unwrap()
}

/// Trace: FR-096-AC-1, NFR-024-AC-1
/// Provenance: agent-ix/quoin#449, routing quoin#450's operations
#[test]
fn tc_449_612_config_resolve_org_reports_the_source_that_won() {
    // The flag outranks everything, and `source` names it. Two cases rather
    // than one: a handler returning a constant payload satisfies either alone.
    let by_flag = ok(&run(
        "config.resolve_org",
        &json!({ "flag": "agent-ix", "git_config": "[remote \"origin\"]\n\turl = git@github.com:other-org/quoin.git\n" }),
    ));
    assert_eq!(by_flag["org"], json!("agent-ix"));
    assert_eq!(by_flag["source"], json!("flag"));
    assert_eq!(by_flag["degraded"], json!(false));

    let by_git = ok(&run(
        "config.resolve_org",
        &json!({ "git_config": "[remote \"origin\"]\n\turl = git@github.com:other-org/quoin.git\n" }),
    ));
    assert_eq!(by_git["org"], json!("other-org"));
    assert_eq!(by_git["source"], json!("git"));
}

/// Trace: FR-096-AC-1, NFR-024-AC-1
/// Provenance: agent-ix/quoin#449, routing quoin#450's operations
#[test]
fn tc_449_613_config_unresolved_org_message_is_the_sentence_the_shell_prints() {
    let payload = ok(&run("config.unresolved_org_message", &json!({})));
    let message = payload["message"].as_str().unwrap();
    // The advice half, which is what makes the sentence actionable and what a
    // truncation or a wrong arm would lose.
    assert!(
        message.contains("could not determine the authoring organization"),
        "{message}"
    );
    assert!(message.contains("quoin config set org"), "{message}");
}
