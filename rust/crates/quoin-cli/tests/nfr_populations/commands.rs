// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! NFR-003 (a bad argument fails out loud) and NFR-007 (an `ix-flow` failure
//! reaches the caller), probed against every command the binary lists.

#![allow(
    clippy::expect_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeSet;
use std::path::Path;

use super::{Probed, quoin, text};

/// Every command in the COMMANDS block of `quoin --help`, found by the binary
/// rather than named here.
pub(crate) fn catalogue(home: &Path) -> BTreeSet<String> {
    let help = quoin(home, &["--help"], |_| {});
    let help = text(&help.stdout);
    let block = help
        .split("\nCOMMANDS\n")
        .nth(1)
        .expect("root help has a COMMANDS block");
    block
        .lines()
        .take_while(|line| !line.is_empty())
        .filter(|line| line.starts_with("  ") && !line.starts_with("   "))
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

/// One command's answer to an argument it does not have.
struct Refusal {
    exited_zero: bool,
    stderr_empty: bool,
    stdout_empty: bool,
}

fn refusal(home: &Path, command: &str) -> Refusal {
    let output = quoin(home, &[command, "--no-such-flag-for-nfr003"], |_| {});
    Refusal {
        exited_zero: output.status.success(),
        stderr_empty: output.stderr.is_empty(),
        stdout_empty: output.stdout.is_empty(),
    }
}

/// NFR-003-M-1 and NFR-003-M-2 over the same probes: the commands that fail
/// out loud, and the commands that fail silently or with output.
pub(crate) fn bad_arguments(home: &Path) -> (Probed, Probed) {
    let examined = catalogue(home);
    let mut actionable = BTreeSet::new();
    let mut silent = BTreeSet::new();
    for command in &examined {
        let answer = refusal(home, command);
        if !answer.exited_zero && !answer.stderr_empty {
            actionable.insert(command.clone());
        }
        if answer.exited_zero || answer.stderr_empty || !answer.stdout_empty {
            silent.insert(command.clone());
        }
    }
    (
        Probed {
            examined: examined.clone(),
            matched: actionable,
        },
        Probed {
            examined,
            matched: silent,
        },
    )
}

/// The `about` line of the commands that launch an `ix-flow` workflow.
#[cfg(unix)]
const WORKFLOW_ABOUT: &str = "Start the Quoin-owned specification workflow";

/// How the staged `ix-flow` fails.
#[cfg(unix)]
const FAILURE_MODES: [&str; 2] = ["ix-flow-absent", "ix-flow-fails-the-run"];

/// A stub `ix-flow` that logs each operation it is asked for. It answers the
/// version handshake and, per `NFR_STUB_MODE`, either fails the `run` with a
/// marker on stderr or completes the launch.
#[cfg(unix)]
const STUB: &str = r#"#!/bin/sh
echo "$1" >> "$NFR_STUB_LOG"
case "$1" in
  --version) echo 0.3.1 ;;
  run)
    if [ "$NFR_STUB_MODE" = fail ]; then
      echo 'nfr007-stub-run-failure' >&2
      exit 7
    fi
    echo '{"instance_id":"nfr007-run"}' ;;
  *) echo '{}' ;;
esac
"#;

/// The commands whose own help says they launch an `ix-flow` workflow.
#[cfg(unix)]
fn workflow_commands(home: &Path) -> BTreeSet<String> {
    catalogue(home)
        .into_iter()
        .filter(|command| {
            let help = quoin(home, &[command, "--help"], |_| {});
            text(&help.stdout).starts_with(WORKFLOW_ABOUT)
        })
        .collect()
}

/// One launch of `command` with `ix-flow` absent (`None`) or the stub in
/// `mode`, returning the process output and the operations the stub logged.
#[cfg(unix)]
fn launch(
    home: &Path,
    command: &str,
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
    }
    let skill = scratch.path().join("skill");
    let workflow = skill.join("workflows").join(command);
    std::fs::create_dir_all(&workflow).expect("the workflow directory is creatable");
    std::fs::write(skill.join("SKILL.md"), "# skill\n").expect("SKILL.md is writable");
    std::fs::write(workflow.join("def.yaml"), "name: x\n").expect("def.yaml is writable");
    let output = quoin(
        home,
        &[command, "--path", skill.to_str().expect("UTF-8 path")],
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

/// NFR-007-M-1: each workflow command with `ix-flow` absent and with `ix-flow`
/// failing the run.
///
/// A pair matches only when `quoin` exits non-zero and shows the failure was
/// the staged one: for the failing stub, the stub was asked to `run` (it got
/// past the version handshake) and its marker reaches stderr; for the absent
/// tool, `quoin` says it could not start `ix-flow`. A control launch against a
/// stub that succeeds must exit zero, so a non-zero exit cannot come from the
/// staging itself.
#[cfg(unix)]
pub(crate) fn external_tool_failures(home: &Path) -> Probed {
    let mut examined = BTreeSet::new();
    let mut matched = BTreeSet::new();
    for command in workflow_commands(home) {
        let (control, operations) = launch(home, &command, Some("succeed"));
        assert!(
            control.status.success() && operations.iter().any(|operation| operation == "run"),
            "{command}: the control launch with a working ix-flow failed: {}",
            text(&control.stderr)
        );
        for mode in FAILURE_MODES {
            let id = format!("{command}:{mode}");
            let (output, operations, surfaced) = if mode == "ix-flow-absent" {
                let (output, operations) = launch(home, &command, None);
                let surfaced = text(&output.stderr).contains("could not start ix-flow");
                (output, operations, surfaced)
            } else {
                let (output, operations) = launch(home, &command, Some("fail"));
                let surfaced = operations.iter().any(|operation| operation == "run")
                    && text(&output.stderr).contains("nfr007-stub-run-failure");
                (output, operations, surfaced)
            };
            let reached = mode == "ix-flow-absent" || !operations.is_empty();
            if !output.status.success() && surfaced && reached {
                matched.insert(id.clone());
            }
            examined.insert(id);
        }
    }
    Probed { examined, matched }
}
