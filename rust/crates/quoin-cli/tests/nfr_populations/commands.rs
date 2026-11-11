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

/// The `about` line of the commands that launch an `ix-flow` workflow.
const WORKFLOW_ABOUT: &str = "Start the Quoin-owned specification workflow";

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

/// How the staged `ix-flow` fails.
const FAILURE_MODES: [&str; 2] = ["ix-flow-absent", "ix-flow-exits-nonzero"];

/// The commands whose own help says they launch an `ix-flow` workflow.
fn workflow_commands(home: &Path) -> BTreeSet<String> {
    catalogue(home)
        .into_iter()
        .filter(|command| {
            let help = quoin(home, &[command, "--help"], |_| {});
            text(&help.stdout).starts_with(WORKFLOW_ABOUT)
        })
        .collect()
}

/// A skill directory the workflow command accepts, so the launch reaches
/// `ix-flow`.
fn skill(root: &Path, command: &str) -> std::path::PathBuf {
    let skill = root.join("skill");
    let workflow = skill.join("workflows").join(command);
    std::fs::create_dir_all(&workflow).expect("the workflow directory is creatable");
    std::fs::write(skill.join("SKILL.md"), "# skill\n").expect("SKILL.md is writable");
    std::fs::write(workflow.join("def.yaml"), "name: x\n").expect("def.yaml is writable");
    skill
}

/// NFR-007-M-1: each workflow command with `ix-flow` absent and with `ix-flow`
/// failing, matched when `quoin` exits non-zero.
#[cfg(unix)]
pub(crate) fn external_tool_failures(home: &Path) -> Probed {
    use std::os::unix::fs::PermissionsExt;

    let mut examined = BTreeSet::new();
    let mut matched = BTreeSet::new();
    for command in workflow_commands(home) {
        for mode in FAILURE_MODES {
            let id = format!("{command}:{mode}");
            let scratch = tempfile::tempdir().expect("scratch is creatable");
            let bin = scratch.path().join("bin");
            std::fs::create_dir_all(&bin).expect("bin is creatable");
            if mode == "ix-flow-exits-nonzero" {
                let stub = bin.join("ix-flow");
                std::fs::write(
                    &stub,
                    "#!/bin/sh\necho 'ix-flow stub failure' >&2\nexit 7\n",
                )
                .expect("the stub is writable");
                std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755))
                    .expect("the stub is executable");
            }
            let skill = skill(scratch.path(), &command);
            let output = quoin(
                home,
                &[
                    command.as_str(),
                    "--path",
                    skill.to_str().expect("UTF-8 path"),
                ],
                |process| {
                    process.env("PATH", &bin);
                },
            );
            if !output.status.success() {
                matched.insert(id.clone());
            }
            examined.insert(id);
        }
    }
    Probed { examined, matched }
}
