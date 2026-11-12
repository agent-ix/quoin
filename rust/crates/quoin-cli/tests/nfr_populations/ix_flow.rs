// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! NFR-007-M-1: every workflow command, launched against each way `ix-flow`
//! can fail, exits non-zero and says what failed.
//!
//! The only external tool `quoin` runs is `ix-flow`, found by name on `PATH`.
//! Each probe gives the command a skill directory that satisfies it and a
//! `PATH` holding either nothing or a stub `ix-flow` that follows the launch
//! protocol up to the step the mode breaks. A control launch against a stub
//! that succeeds must exit zero first, so a non-zero exit cannot come from the
//! staging itself.

#![cfg(unix)]
#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeSet;

use super::context::Context;
use super::help_tree::CommandHelp;
use super::{Probed, text};

/// The `about` line of the commands that launch an `ix-flow` workflow.
const WORKFLOW_ABOUT: &str = "Start the Quoin-owned specification workflow";

/// One way `ix-flow` fails. `operation` is the stub call the failure belongs
/// to: the probe reaches it, or the failure was not the staged one. `evidence`
/// is the fragment of the message that failure alone produces, from the stub's
/// marker where the message is the tool's own, otherwise from `quoin`'s wording.
struct Mode {
    id: &'static str,
    operation: Option<&'static str>,
    evidence: &'static str,
}

const MODES: [Mode; 12] = [
    Mode {
        id: "ix-flow-absent",
        operation: None,
        evidence: "could not start ix-flow",
    },
    Mode {
        id: "ix-flow-reports-an-old-version",
        operation: Some("--version"),
        evidence: "ix-flow >=0.3.1 is required",
    },
    Mode {
        id: "ix-flow-reports-no-version",
        operation: Some("--version"),
        evidence: "supported semantic version",
    },
    Mode {
        id: "ix-flow-hangs-the-handshake",
        operation: Some("--version"),
        evidence: "launch deadline",
    },
    Mode {
        id: "ix-flow-fails-the-run",
        operation: Some("run"),
        evidence: "nfr007-fails-the-run",
    },
    Mode {
        id: "ix-flow-hangs-the-run",
        operation: Some("run"),
        evidence: "launch deadline",
    },
    Mode {
        id: "ix-flow-prints-garbage",
        operation: Some("run"),
        evidence: "ix-flow returned invalid JSON",
    },
    Mode {
        id: "ix-flow-prints-nothing",
        operation: Some("run"),
        evidence: "ix-flow returned invalid JSON",
    },
    Mode {
        id: "ix-flow-floods-its-output",
        operation: Some("run"),
        evidence: "1 MiB output bound",
    },
    Mode {
        id: "ix-flow-reports-an-error-payload",
        operation: Some("run"),
        evidence: "nfr007-error-payload",
    },
    Mode {
        id: "ix-flow-omits-the-instance-id",
        operation: Some("run"),
        evidence: "ix-flow returned no instance_id",
    },
    Mode {
        id: "ix-flow-fails-the-context",
        operation: Some("add-item"),
        evidence: "nfr007-fails-the-context",
    },
];

/// A stub `ix-flow` that logs each operation it is asked for and breaks the
/// step `NFR_STUB_MODE` names. It uses only shell builtins and `sleep`.
const STUB: &str = r#"#!/bin/sh
echo "$1" >> "$NFR_STUB_LOG"
case "$1" in
  --version)
    case "$NFR_STUB_MODE" in
      reports-an-old-version) echo 0.1.0 ;;
      reports-no-version) echo banana ;;
      hangs-the-handshake) exec sleep 3600 ;;
      *) echo 0.3.1 ;;
    esac ;;
  run)
    case "$NFR_STUB_MODE" in
      fails-the-run) echo 'nfr007-fails-the-run' >&2; exit 7 ;;
      hangs-the-run) exec sleep 3600 ;;
      prints-garbage) echo 'this is not json' ;;
      prints-nothing) ;;
      floods-its-output)
        i=0
        while [ "$i" -lt 2000 ]; do printf '%01000d' 0; i=$((i + 1)); done ;;
      reports-an-error-payload)
        echo '{"ok":false,"error":{"code":"x","message":"nfr007-error-payload"}}' ;;
      omits-the-instance-id) echo '{"ok":true}' ;;
      *) echo '{"instance_id":"nfr007-run"}' ;;
    esac ;;
  add-item)
    case "$NFR_STUB_MODE" in
      fails-the-context) echo 'nfr007-fails-the-context' >&2; exit 9 ;;
      *) echo '{}' ;;
    esac ;;
  *) echo '{}' ;;
esac
"#;

/// The commands whose own help says they launch an `ix-flow` workflow.
fn workflow_commands(cx: &Context) -> Vec<&'static CommandHelp> {
    cx.commands()
        .iter()
        .filter(|command| !command.path.is_empty())
        .filter(|command| command.summary.starts_with(WORKFLOW_ABOUT))
        .collect()
}

/// `sleep`, found on the ambient `PATH`, linked into `bin`: the stub's `PATH`
/// holds nothing else, so an `ix-flow` elsewhere cannot be found.
fn link_sleep(bin: &std::path::Path) {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let sleep = std::env::split_paths(&path)
        .map(|directory| directory.join("sleep"))
        .find(|candidate| candidate.is_file())
        .expect("sleep is on PATH");
    std::os::unix::fs::symlink(sleep, bin.join("sleep")).expect("sleep links");
}

/// One launch of `command` with `ix-flow` absent (`None`) or the stub in
/// `stub_mode`, returning the process output and the operations the stub
/// logged.
fn launch(
    cx: &Context,
    command: &CommandHelp,
    stub_mode: Option<&str>,
) -> (std::process::Output, Vec<String>) {
    use std::os::unix::fs::PermissionsExt;

    let scratch = tempfile::tempdir().expect("scratch is creatable");
    let bin = scratch.path().join("bin");
    std::fs::create_dir_all(&bin).expect("bin is creatable");
    let log = scratch.path().join("ix-flow.log");
    if stub_mode.is_some() {
        let stub = bin.join("ix-flow");
        std::fs::write(&stub, STUB).expect("the stub is writable");
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755))
            .expect("the stub is executable");
        link_sleep(&bin);
    }
    let skill = scratch.path().join("skill");
    let workflow = skill.join("workflows").join(command.id());
    std::fs::create_dir_all(&workflow).expect("the workflow directory is creatable");
    std::fs::write(skill.join("SKILL.md"), "# skill\n").expect("SKILL.md is writable");
    std::fs::write(workflow.join("def.yaml"), "name: x\n").expect("def.yaml is writable");
    let output = cx.run_with(
        command,
        &["--path", skill.to_str().expect("UTF-8 path")],
        |process| {
            process
                .env("PATH", &bin)
                .env("NFR_STUB_LOG", &log)
                .env("NFR_STUB_MODE", stub_mode.unwrap_or("absent"));
        },
    );
    let logged = std::fs::read_to_string(&log)
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default();
    (output, logged)
}

/// Whether `command` met `mode` and failed as the mode says it should: a
/// non-zero exit, the stub reached the step the mode belongs to, and the
/// evidence of the staged failure reached stderr.
fn surfaced(cx: &Context, command: &CommandHelp, mode: &Mode) -> bool {
    let stub_mode = (mode.operation.is_some()).then(|| mode.id.trim_start_matches("ix-flow-"));
    let (output, operations) = launch(cx, command, stub_mode);
    let stderr = text(&output.stderr);
    let evidence = stderr.contains(mode.evidence);
    let reached = mode
        .operation
        .is_none_or(|wanted| operations.iter().any(|operation| operation == wanted));
    !output.status.success() && evidence && reached
}

/// NFR-007-M-1: each workflow command with each way `ix-flow` fails, as
/// `command:mode`. Every pair is probed on its own thread, so the modes that
/// run to `quoin`'s launch deadline cost one deadline, not one each.
pub(crate) fn external_tool_failures(cx: &Context) -> Probed {
    let commands = workflow_commands(cx);
    for command in &commands {
        let (control, operations) = launch(cx, command, Some("succeed"));
        assert!(
            control.status.success() && operations.iter().any(|operation| operation == "run"),
            "{}: the control launch with a working ix-flow failed: {}",
            command.id(),
            text(&control.stderr)
        );
    }
    let pairs: Vec<(&CommandHelp, &Mode)> = commands
        .iter()
        .flat_map(|command| MODES.iter().map(move |mode| (*command, mode)))
        .collect();
    let results: Vec<(String, bool)> = std::thread::scope(|scope| {
        let handles: Vec<_> = pairs
            .iter()
            .map(|(command, mode)| {
                scope.spawn(move || {
                    (
                        format!("{}:{}", command.id(), mode.id),
                        surfaced(cx, command, mode),
                    )
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a probe thread finishes"))
            .collect()
    });
    let examined: BTreeSet<String> = results.iter().map(|(id, _)| id.clone()).collect();
    let matched = results
        .into_iter()
        .filter_map(|(id, surfaced)| surfaced.then_some(id))
        .collect();
    Probed { examined, matched }
}
