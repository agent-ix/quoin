// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `config` and `modules` operations, reached by their wire names.
//!
//! `tc_447_600_every_operation_is_invoked_by_name_by_some_test` requires every
//! entry in `OPERATIONS` to be handed to the real binary, spelled, by some
//! integration test — because a swapped match arm leaves the drift guard, the
//! unit tests and `quoin-difftest` all green and breaks only for a user. Four
//! operations arrived on `main` (quoin#450) with unit coverage of their
//! handlers and no coverage of their ROUTE: `config.resolve_org`,
//! `config.unresolved_org_message`, `modules.list` and `modules.remove`. This
//! file is that route coverage.
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

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

/// Invoke the binary with `op` on argv and `request` on stdin.
///
/// `home` is passed as `IX_HOME` so a `modules` run touches a temporary tree
/// and never the developer's own `~/.ix`.
fn run(op: &str, request: &Value, home: Option<&Path>) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin-core"));
    command.arg(op);
    if let Some(home) = home {
        command.env("IX_HOME", home);
        // The vendored contract every `modules` operation is judged against;
        // without it an install refuses rather than installing unjudged.
        command.env("QUOIN_SEMANTIC_ROOT", semantic_root());
    }
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

/// The repository root: `rust/crates/quoin-core` up three.
fn repo_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..3 {
        root.pop();
    }
    root
}

/// The vendored semantic contract the npm package ships, and the tree
/// `src/core/modules.ts` publishes as `QUOIN_SEMANTIC_ROOT`.
fn semantic_root() -> PathBuf {
    repo_root().join("src").join("semantic")
}

/// The `module-ok` fixture, the same one `tc_446` installs.
fn fixture() -> PathBuf {
    repo_root()
        .join("tests")
        .join("fixtures")
        .join("semantic-module")
        .join("module-ok")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// The module name `module-ok`'s manifest declares.
const INSTALLED: &str = "spec-objects-fixture";

/// A temporary home with `module-ok` installed through the boundary.
struct Home {
    _root: tempfile::TempDir,
    home: PathBuf,
}

fn installed_home() -> Home {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    copy_tree(&fixture(), &source);
    let home = root.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let result = run(
        "modules.install",
        &json!({ "source": format!("path:{}", source.display()), "home": home.display().to_string() }),
        Some(&home),
    );
    assert_eq!(result.status, 0, "install failed: {}", result.stderr);
    Home { _root: root, home }
}

/// Trace: FR-096-AC-1, NFR-024-AC-1
/// Provenance: agent-ix/quoin#449, routing quoin#450's operations
#[test]
fn tc_449_610_modules_list_returns_the_registry_the_install_wrote() {
    let fixture = installed_home();
    let payload = ok(&run(
        "modules.list",
        &json!({ "home": fixture.home.display().to_string() }),
        Some(&fixture.home),
    ));
    let modules = payload["modules"].as_array().unwrap();
    // Not `is_empty()`: an empty registry is what a run against the WRONG home
    // also returns, so the assertion names the module the install wrote.
    let names: Vec<&str> = modules
        .iter()
        .map(|module| module["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec![INSTALLED], "payload: {payload}");
}

/// Trace: FR-096-AC-1, NFR-024-AC-1
/// Provenance: agent-ix/quoin#449, routing quoin#450's operations
#[test]
fn tc_449_611_modules_remove_takes_the_module_out_of_the_registry() {
    let fixture = installed_home();
    let payload = ok(&run(
        "modules.remove",
        &json!({ "name": INSTALLED, "home": fixture.home.display().to_string() }),
        Some(&fixture.home),
    ));
    assert_eq!(payload["removed"], json!(INSTALLED));

    // The claim is checked against the registry rather than taken from the
    // payload that made it: a handler that answered `removed` without removing
    // anything would satisfy the line above.
    let after = ok(&run(
        "modules.list",
        &json!({ "home": fixture.home.display().to_string() }),
        Some(&fixture.home),
    ));
    assert_eq!(after["modules"], json!([]), "payload: {after}");
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
        None,
    ));
    assert_eq!(by_flag["org"], json!("agent-ix"));
    assert_eq!(by_flag["source"], json!("flag"));
    assert_eq!(by_flag["degraded"], json!(false));

    let by_git = ok(&run(
        "config.resolve_org",
        &json!({ "git_config": "[remote \"origin\"]\n\turl = git@github.com:other-org/quoin.git\n" }),
        None,
    ));
    assert_eq!(by_git["org"], json!("other-org"));
    assert_eq!(by_git["source"], json!("git"));
}

/// Trace: FR-096-AC-1, NFR-024-AC-1
/// Provenance: agent-ix/quoin#449, routing quoin#450's operations
#[test]
fn tc_449_613_config_unresolved_org_message_is_the_sentence_the_shell_prints() {
    let payload = ok(&run("config.unresolved_org_message", &json!({}), None));
    let message = payload["message"].as_str().unwrap();
    // The advice half, which is what makes the sentence actionable and what a
    // truncation or a wrong arm would lose.
    assert!(
        message.contains("could not determine the authoring organization"),
        "{message}"
    );
    assert!(message.contains("quoin config set org"), "{message}");
}
