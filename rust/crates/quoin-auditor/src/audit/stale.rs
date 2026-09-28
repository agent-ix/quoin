// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Stale evidence: a missing run, a failed or errored run, or a run behind
//! HEAD (FR-032 line 28).
//!
//! Three of the ladder's rungs answer one question — is the evidence this
//! binding claims still there and still good — so they live together here,
//! the same way `method.rs`, `mocks.rs` and `scores.rs` already split the
//! conformance, mock and score rungs out of `ladder.rs` (PLAT-1086, PR #647
//! review FND-007).

use quoin_combinatorial::js;
use quoin_evidence::types::{Binding, Outcome};
use quoin_finding_types::{Finding, FindingKind, Severity};
use quoin_quire_types::Obligation;

use super::Indexed;
use super::ladder::{Paired, join_suites, slice_utf16};

/// ── 2. Stale evidence ──
///
/// A suite that recorded a SCAN has evidence; it simply is not run-shaped.
pub(super) fn unrecorded_evidence(
    indexed: &Indexed<'_>,
    obligation: &Obligation,
    bindings: &[&Binding],
) -> Option<Finding> {
    let unrecorded: Vec<&&Binding> = bindings
        .iter()
        .filter(|binding| {
            !indexed.runs_by_suite.contains_key(binding.suite.as_str())
                && !indexed.scans_by_suite.contains_key(binding.suite.as_str())
        })
        .collect();
    if unrecorded.is_empty() {
        return None;
    }
    Some(Finding::new(
        FindingKind::STALE_EVIDENCE,
        &obligation.id,
        Severity::high(),
        format!(
            "{id} is bound to {suites}, which {verb} no recorded run. The binding claims \
             evidence that is not in the store.",
            id = obligation.id,
            suites = join_suites(&unrecorded),
            verb = if unrecorded.len() == 1 { "has" } else { "have" }
        ),
    ))
}

/// ── 2. Stale evidence (failed run) ──
///
/// A run that recorded a `fail` or `error` outcome for a bound symbol is not
/// freshness rot, it is a claim that does not hold: the binding says this
/// test discharges the obligation, and the newest run of that test says it
/// did not pass. The vacuity rung below only asks `entry.outcome ==
/// Outcome::Skip`; a `Fail`/`Error` outcome passed through it unflagged,
/// which is what let a failing run at HEAD read as healthy (FR-032 line 28,
/// PLAT-1086).
pub(super) fn failed_run(
    indexed: &Indexed<'_>,
    obligation: &Obligation,
    bindings: &[&Binding],
) -> Option<Finding> {
    let mut failed: Vec<String> = Vec::new();
    for binding in bindings {
        let Some(run) = indexed.runs_by_suite.get(binding.suite.as_str()) else {
            continue;
        };
        for symbol in &binding.symbols {
            let Some(entry) = run
                .entries
                .iter()
                .find(|entry| entry.symbol.as_str() == symbol.as_str())
            else {
                continue;
            };
            if !matches!(entry.outcome, Outcome::Fail | Outcome::Error) {
                continue;
            }
            failed.push(format!(
                "{suite}:{symbol} {outcome} in a run at {commit}",
                suite = binding.suite,
                symbol = symbol.as_str(),
                outcome = entry.outcome.as_str(),
                commit = slice_utf16(run.commit.as_str(), 12)
            ));
        }
    }
    if failed.is_empty() {
        return None;
    }
    failed.sort_by(|left, right| js::compare(left, right));
    Some(Finding::new(
        FindingKind::STALE_EVIDENCE,
        &obligation.id,
        Severity::high(),
        format!(
            "{id} is bound to a failing run: {list}. The binding claims evidence that does \
             not hold.",
            id = obligation.id,
            list = failed.join("; ")
        ),
    ))
}

/// Runs recorded at a commit other than the one being audited.
///
/// # Divergence
///
/// The retained code filters `bindings` while indexing a `runs` array built
/// from `runBindings`. Whenever any binding is scan-backed the filter walks
/// past `runs.length`, `runs[i]` is `undefined`, and the retained code throws
/// a `TypeError`. This pairs each binding with its own run, which is exactly
/// what the adjacent vacuity block's own comment says must be done — the same
/// defect was found and fixed there and not here. `DIVERGENCE.md` §1.
pub(super) fn behind_head(
    obligation: &Obligation,
    paired: &[Paired<'_>],
    head: &str,
) -> Option<Finding> {
    let behind: Vec<&Paired<'_>> = paired
        .iter()
        .filter(|(_, run)| run.commit.as_str() != head)
        .collect();
    if behind.is_empty() {
        return None;
    }
    Some(Finding::new(
        FindingKind::STALE_EVIDENCE,
        &obligation.id,
        // Medium: an older run is normal between releases. It becomes
        // actionable at a gate, which is the consuming workflow's policy.
        Severity::medium(),
        format!(
            "{id} rests on {runs}, not at HEAD ({head}).",
            id = obligation.id,
            runs = behind
                .iter()
                .map(|(binding, run)| format!(
                    "a {suite} run at {commit}",
                    suite = binding.suite,
                    commit = slice_utf16(run.commit.as_str(), 12)
                ))
                .collect::<Vec<_>>()
                .join(" and "),
            head = slice_utf16(head, 12)
        ),
    ))
}
