// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The intake order of the measurement store, obtained from git.
//!
//! `quoin measurement verify` and `quoin report` obtain the order here and
//! nowhere else, so the two name the same pair of collections (FR-132-AC-15).

use ix_trace_rs::implements;

/// The intake order handed to the checker, where it came from, and what the
/// operator should be told about it.
pub(crate) struct IntakeOrder {
    /// Collection ids per first-add commit, earliest first.
    pub(crate) groups: Vec<Vec<String>>,
    /// The wire spelling of the order's source.
    pub(crate) source: &'static str,
    /// What to tell the operator when no order was obtained.
    pub(crate) warning: Option<String>,
}

/// Collection ids grouped by the first-parent git commit that added each file
/// under the measurement store, earliest first (FR-108-AC-2).
///
/// The store records no intake order, and a collection's own `timestamp` is
/// the producer's to choose. The commit that added the file on the
/// first-parent line is the most producer-independent order available:
/// changing it means rewriting published history, and following first
/// parents only means a side branch's own commit dates cannot reorder
/// collections merged together. `--no-renames` makes a moved file count as
/// added where it now lives.
///
/// A directory that is not a git work tree has no order, and says so. A
/// shallow clone's history is truncated, so its first-add commits are not
/// the real ones: the order is dropped and the checker reports
/// `order_unattested`. Any other git failure is an error, not a silent
/// absence of order.
#[implements("FR-108-AC-2")]
pub(crate) fn intake_order(repo: &str) -> Result<IntakeOrder, String> {
    let git = |arguments: &[&str]| {
        std::process::Command::new("git")
            .args(["-C", repo])
            .args(arguments)
            .output()
            .map_err(|error| format!("cannot run git: {error}"))
    };
    let inside = git(&["rev-parse", "--is-inside-work-tree"])?;
    if !inside.status.success() {
        return Ok(IntakeOrder {
            groups: Vec::new(),
            source: "none",
            warning: Some(format!(
                "no intake order: {repo} is not a git work tree ({})",
                String::from_utf8_lossy(&inside.stderr).trim()
            )),
        });
    }
    let shallow = git(&["rev-parse", "--is-shallow-repository"])?;
    if String::from_utf8_lossy(&shallow.stdout).trim() == "true" {
        return Ok(IntakeOrder {
            groups: Vec::new(),
            source: "git-shallow",
            warning: Some(format!(
                "no intake order: {repo} is a shallow clone, so the commit that first added \
                 each collection is not known; fetch full history to attest the order"
            )),
        });
    }
    let log = git(&[
        "log",
        "--first-parent",
        "--reverse",
        "--topo-order",
        "--no-renames",
        "--diff-filter=A",
        "--relative",
        "--format=commit %H",
        "--name-only",
        "--",
        MEASUREMENTS_DIRECTORY,
    ])?;
    if !log.status.success() {
        return Err(format!(
            "git log over {MEASUREMENTS_DIRECTORY} failed: {}",
            String::from_utf8_lossy(&log.stderr).trim()
        ));
    }
    let mut groups: Vec<Vec<String>> = Vec::new();
    for line in String::from_utf8_lossy(&log.stdout).lines() {
        if line.starts_with("commit ") {
            groups.push(Vec::new());
        } else if let Some(id) = line
            .strip_prefix(MEASUREMENTS_DIRECTORY)
            .and_then(|rest| rest.strip_prefix('/'))
            .and_then(|name| name.strip_suffix(".json"))
            .filter(|id| !id.contains('/'))
            && let Some(group) = groups.last_mut()
        {
            group.push(id.to_owned());
        }
    }
    Ok(IntakeOrder {
        groups,
        source: "git-first-parent-add",
        warning: None,
    })
}

/// Where `quoin-measurement` publishes collections, relative to the
/// repository root (`quoin_measurement::store::measurements_root`).
pub(crate) const MEASUREMENTS_DIRECTORY: &str = "spec/evidence/measurements";
