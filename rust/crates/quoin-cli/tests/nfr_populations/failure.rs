// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One failing invocation, judged against NFR-003: it exits non-zero and its
//! diagnostic names the cause and points to a next step (NFR-003-M-1), and it
//! is not silent or partial (NFR-003-M-2).

use std::collections::BTreeSet;
use std::process::Output;

use ix_trace_rs::trace;

use super::{Probed, text};

/// Phrases by which a message points a caller to a next step: usage text, the
/// help, or the names that were available.
const NEXT_STEPS: [&str; 6] = [
    "usage",
    "--help",
    "available",
    "commands:",
    "subcommands:",
    "try ",
];

/// The verdicts on one probe.
pub(crate) struct Failure {
    pub(crate) id: String,
    /// M-1: non-zero exit and an actionable diagnostic.
    pub(crate) actionable: bool,
    /// M-2: exit zero, no diagnostic, or output on stdout beside it.
    pub(crate) silent: bool,
    /// The exit code and diagnostic text, to show why a probe did not match.
    pub(crate) detail: String,
}

/// Judge `output`: `actionable` decides whether its stderr, where a failing
/// command puts its diagnostic, names the cause and a next step.
pub(crate) fn judge(id: String, output: &Output, actionable: impl FnOnce(&str) -> bool) -> Failure {
    let (stdout, message) = (text(&output.stdout), text(&output.stderr));
    let exited_zero = output.status.success();
    let detail = format!("exit {:?}: {}", output.status.code(), message.trim());
    Failure {
        id,
        actionable: !exited_zero && actionable(&message),
        silent: exited_zero || message.trim().is_empty() || !stdout.is_empty(),
        detail,
    }
}

/// Whether `message` mentions `token` as a whole word, ignoring case: the
/// characters beside a match are not word characters where the token's own
/// edges are, so `available` is not found in `unavailable` nor `line` in
/// `deadline`.
pub(crate) fn names(message: &str, token: &str) -> bool {
    let (message, token) = (message.to_lowercase(), token.to_lowercase());
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let (first, last) = (token.chars().next(), token.chars().next_back());
    message.match_indices(&token).any(|(start, _)| {
        let before = message.get(..start).and_then(|s| s.chars().next_back());
        let after = message
            .get(start + token.len()..)
            .and_then(|s| s.chars().next());
        let apart = |beside: Option<char>, edge: Option<char>| {
            !(beside.is_some_and(word) && edge.is_some_and(word))
        };
        apart(before, first) && apart(after, last)
    })
}

/// Whether `message` points to usage, help or the available names.
pub(crate) fn points_forward(message: &str) -> bool {
    NEXT_STEPS.iter().any(|phrase| names(message, phrase))
}

/// The ids examined, those whose diagnostic was actionable (NFR-003-M-1), and
/// those that failed silently or with partial output (NFR-003-M-2).
pub(crate) fn probed(failures: &[Failure]) -> (Probed, Probed) {
    let examined: BTreeSet<String> = failures.iter().map(|f| f.id.clone()).collect();
    let select = |keep: fn(&Failure) -> bool| -> BTreeSet<String> {
        failures
            .iter()
            .filter(|f| keep(f))
            .map(|f| f.id.clone())
            .collect()
    };
    (
        Probed {
            examined: examined.clone(),
            matched: select(|f| f.actionable),
        },
        Probed {
            examined,
            matched: select(|f| f.silent),
        },
    )
}

#[trace("FR-131-AC-9")]
#[test]
fn tc_3707_next_steps_and_tokens_match_whole_words_only() {
    for vacuous in [
        "the module is unavailable",
        "registry lookup failed",
        "pipeline stalled",
        "past the deadline",
        "inline value",
    ] {
        assert!(!points_forward(vacuous), "{vacuous}");
        assert!(!names(vacuous, "line"), "{vacuous}");
    }
    for forward in ["Usage: quoin x", "run with --help", "Available types: A"] {
        assert!(points_forward(forward), "{forward}");
    }
    assert!(names("expected value at line 1 column 2", "line"));
}
