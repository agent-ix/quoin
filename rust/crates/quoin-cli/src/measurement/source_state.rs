// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Source-state attestation for `quoin measurement record` (PLAT-1049).
//!
//! A collection's `verificationStack.sources.<name>` carries a `revision` and
//! a `sourceState` the producer wrote about itself. The measurement crate
//! treats `sources` as opaque, and only this binary has git, so the claim is
//! checked here, before the collection is published:
//!
//! - inside a git work tree, a source whose `revision` resolves to a commit
//!   there is checkable: a `clean` claim must match a work tree with no
//!   uncommitted change outside the measurement store, or the record is
//!   refused;
//! - a source whose `revision` does not resolve in `--repo` (it may belong to
//!   another repository), and every source outside a git work tree, cannot be
//!   checked, so its `sourceState` is rewritten to `unverified`. A claim of
//!   `clean` is never stored when it could not be checked.
//!
//! `revision` and `sourceState` come from the collection file and are
//! attacker-controlled: a revision reaches git only after `--end-of-options`.

use std::process::Output;

use serde_json::Value;

use super::MEASUREMENTS_DIRECTORY;

/// The `sourceState` stored when the claim could not be checked.
pub(super) const UNVERIFIED: &str = "unverified";

fn git(repo: &str, arguments: &[&str]) -> Option<Output> {
    std::process::Command::new("git")
        .args(["-C", repo])
        .args(arguments)
        .output()
        .ok()
}

fn in_work_tree(repo: &str) -> bool {
    git(repo, &["rev-parse", "--is-inside-work-tree"])
        .is_some_and(|output| output.status.success() && output.stdout.starts_with(b"true"))
}

fn revision_exists(repo: &str, revision: &str) -> bool {
    git(
        repo,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &format!("{revision}^{{commit}}"),
        ],
    )
    .is_some_and(|output| output.status.success())
}

/// Whether the work tree has an uncommitted change outside the measurement
/// store (recording a collection must not make the next one look dirty).
fn is_dirty(repo: &str) -> Result<bool, String> {
    let exclude = format!(":(exclude){MEASUREMENTS_DIRECTORY}");
    let output = git(repo, &["status", "--porcelain", "--", ".", &exclude])
        .ok_or_else(|| "cannot run git to check the source state".to_owned())?;
    if !output.status.success() {
        return Err(format!(
            "git status failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(!output.stdout.is_empty())
}

/// Check or downgrade the sources `record` attests, in place.
///
/// # Errors
///
/// A `clean` claim, on a source whose revision resolves in the repository,
/// while the work tree is not clean.
pub(super) fn attest(repo: &str, record: &mut Value) -> Result<(), String> {
    let Some(sources) = record
        .get_mut("verificationStack")
        .and_then(|stack| stack.get_mut("sources"))
        .and_then(Value::as_object_mut)
    else {
        return Ok(());
    };
    let checkable = in_work_tree(repo);
    let dirty = if checkable { is_dirty(repo)? } else { false };
    for (name, source) in sources {
        let Some(source) = source.as_object_mut() else {
            continue;
        };
        let resolves = checkable
            && source
                .get("revision")
                .and_then(Value::as_str)
                .is_some_and(|revision| revision_exists(repo, revision));
        if !resolves {
            source.insert(
                "sourceState".to_owned(),
                Value::String(UNVERIFIED.to_owned()),
            );
            continue;
        }
        if dirty && source.get("sourceState").and_then(Value::as_str) == Some("clean") {
            return Err(format!(
                "verificationStack.sources.{name}.sourceState: claims clean but {repo} has \
                 uncommitted changes"
            ));
        }
    }
    Ok(())
}
