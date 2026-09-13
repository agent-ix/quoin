// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The boundary as a caller actually meets it: a real subprocess, real pipes,
//! real exit statuses.
//!
//! The unit tests in the library prove the decisions. These prove the SHELL —
//! that a payload lands on stdout and nowhere else, that diagnostics land on
//! stderr and nowhere else, and that the exit status a caller observes is the
//! one the taxonomy promises. Those are exactly the properties a library test
//! cannot see, and exactly the ones `src/core/exec.ts` depends on.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::io::Write as _;
use std::process::{Command, Stdio};

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

fn run(args: &[&str], stdin: &str) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_quoin-core"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    Run {
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
        status: out.status.code().unwrap(),
    }
}

/// Trace: FR-096
/// Provenance: quoin#375
#[test]
fn tc_375_a_ping_round_trips_payload_on_stdout_and_nothing_on_stderr() {
    let result = run(&["core.ping"], r#"{"echo":"corr-7"}"#);
    assert_eq!(result.status, 0);
    assert_eq!(result.stderr, "");
    let payload: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(payload["echo"], "corr-7");
    assert_eq!(payload["protocol_version"], 1);
    assert_eq!(payload["core_version"], env!("CARGO_PKG_VERSION"));
}

/// Trace: FR-096
/// Provenance: quoin#375
#[test]
fn tc_375_stdout_is_canonical_json_one_line() {
    let result = run(&["core.ping"], r#"{"echo":"a"}"#);
    assert_eq!(result.stdout.lines().count(), 1);
    // Keys sorted, no insignificant whitespace: the property `quoin-difftest`
    // compares on. Written out literally rather than re-derived, so a change
    // to the canonicaliser fails here instead of agreeing with itself.
    assert_eq!(
        result.stdout.trim_end(),
        format!(
            r#"{{"core_version":"{}","echo":"a","protocol_version":1}}"#,
            env!("CARGO_PKG_VERSION")
        )
    );
}

/// Trace: FR-096
/// Provenance: quoin#375, agent-ix/quoin#103
#[test]
fn tc_375_exit_1_still_carries_a_complete_payload() {
    // The `runQuireAllowFailure` lesson, in the Rust half: non-zero and valid
    // are independent facts, and the caller must be able to tell them apart.
    let result = run(&["core.ping"], r#"{"echo":"a","expect_protocol":99}"#);
    assert_eq!(result.status, 1);
    let payload: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(payload["echo"], "a");
    assert_eq!(payload["protocol_version"], 1);
    let diagnostics: Vec<serde_json::Value> = serde_json::from_str(&result.stderr).unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["code"], "CORE_PROTOCOL_SKEW");
    assert_eq!(diagnostics[0]["context"]["actual"], "1");
}

/// Trace: FR-096
/// Provenance: quoin#375
#[test]
fn tc_375_a_refusal_writes_no_payload_at_all() {
    let echo = "x".repeat(4 * 1024 + 1);
    let result = run(&["core.ping"], &format!(r#"{{"echo":"{echo}"}}"#));
    assert_eq!(result.status, 2);
    assert_eq!(
        result.stdout, "",
        "a failed run must not leave a partial payload on stdout"
    );
    let diagnostics: Vec<serde_json::Value> = serde_json::from_str(&result.stderr).unwrap();
    assert_eq!(diagnostics[0]["code"], "CORE_REFUSED");
}

/// Trace: FR-096
/// Provenance: quoin#375
#[test]
fn tc_375_every_invalid_shape_exits_3_with_its_own_code() {
    for (args, stdin, code) in [
        (vec![], "", "CORE_BAD_USAGE"),
        (vec!["core.ping", "extra"], "", "CORE_BAD_USAGE"),
        (vec!["notdotted"], "", "CORE_BAD_USAGE"),
        (vec!["evidence.record"], "{}", "CORE_UNKNOWN_OP"),
        (vec!["core.ping"], "{oops", "CORE_BAD_JSON"),
        (vec!["core.ping"], "[1,2]", "CORE_BAD_JSON"),
        (vec!["core.ping"], r#"{"eco":1}"#, "CORE_BAD_REQUEST"),
    ] {
        let result = run(&args, stdin);
        assert_eq!(result.status, 3, "args={args:?} stdin={stdin:?}");
        assert_eq!(result.stdout, "", "args={args:?}");
        let diagnostics: Vec<serde_json::Value> = serde_json::from_str(&result.stderr).unwrap();
        assert_eq!(
            diagnostics[0]["code"], code,
            "args={args:?} stdin={stdin:?}"
        );
    }
}

/// Trace: FR-096
/// Provenance: quoin#375
#[test]
fn tc_375_no_stdin_at_all_is_the_empty_request() {
    let result = run(&["core.ping"], "");
    assert_eq!(result.status, 0);
    let payload: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(payload["echo"], serde_json::Value::Null);
}

/// Trace: FR-096-AC-9
/// Provenance: quoin#412, review #448 FND-003
///
/// The transport ceiling, through the shell a caller actually meets.
///
/// The library tests drive `read_request` directly, which proves the decision
/// but not the process: a ceiling that is only enforced in a function nothing
/// in `main` calls is a ceiling nobody has. This one pipes a real oversize
/// stream into a real subprocess and reads the status off the exit code.
///
/// It also pins the part a unit test cannot see — that the child stops reading
/// and exits while the parent still has bytes to send. The writer below runs on
/// its own thread and tolerates a broken pipe **because a broken pipe is the
/// pass**: a binary that drained 64 MiB before deciding would let `write_all`
/// finish, and the ceiling would be a remark about an allocation that had
/// already happened.
#[test]
fn tc_412_an_oversize_stream_is_refused_by_the_process_not_merely_by_the_library() {
    let ceiling = quoin_core::protocol::MAX_REQUEST_BYTES;
    let envelope = r#"{"echo":""}"#.len();
    let oversize = format!(r#"{{"echo":"{}"}}"#, "x".repeat(ceiling - envelope + 1));
    assert_eq!(oversize.len(), ceiling + 1);

    let mut child = Command::new(env!("CARGO_BIN_EXE_quoin-core"))
        .arg("core.ping")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut sink = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        // `write_all`'s error is discarded, not unwrapped: EPIPE here means the
        // child refused early, which is the behaviour under test.
        let _ = sink.write_all(oversize.as_bytes());
    });
    let out = child.wait_with_output().unwrap();
    writer.join().unwrap();

    assert_eq!(out.status.code().unwrap(), 2);
    assert!(
        out.stdout.is_empty(),
        "a refusal must not leave a partial payload on stdout"
    );
    let diagnostics: Vec<serde_json::Value> =
        serde_json::from_str(&String::from_utf8(out.stderr).unwrap()).unwrap();
    assert_eq!(diagnostics[0]["code"], "CORE_REFUSED");
    assert_eq!(
        diagnostics[0]["context"]["limit_bytes"],
        ceiling.to_string()
    );
    // Read, not sent: the number names what the process consumed, and it is one
    // byte past the ceiling however much the caller offered.
    assert_eq!(
        diagnostics[0]["context"]["read_bytes"],
        (ceiling + 1).to_string()
    );
}
