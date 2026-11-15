// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Stale evidence: a missing run or a failed or errored run (FR-032 line 28).
//!
//! Two of the ladder's rungs answer one question — is the evidence this
//! binding claims still there and still good — so they live together here,
//! the same way `method.rs`, `mocks.rs` and `scores.rs` already split the
//! conformance, mock and score rungs out of `ladder.rs` (PLAT-1086, PR #647
//! review FND-007).

use quoin_combinatorial::js;
use quoin_evidence::stale::failed_bound_symbols;
use quoin_evidence::types::{Binding, Outcome};
use quoin_finding_types::{Finding, FindingKind, Severity, StaleCause};
use quoin_quire_types::Obligation;

use super::Indexed;

/// One stale suite and its cause, for deterministic selection (FR-032-AC-20).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct StaleSuite<'a> {
    /// Suite name, for bytewise ordering.
    suite: &'a str,
    /// The cause this suite exhibits.
    cause: StaleCause,
    /// First failing symbol in bytewise order, when cause is `failed-run`.
    symbol: Option<&'a str>,
    /// The outcome (Fail or Error) for failed-run stale suites.
    outcome: Option<Outcome>,
}

/// ── 2. Stale evidence (failed or missing run) ──
///
/// Collects both failed runs and missing runs, returning the first stale suite
/// in bytewise order with its typed cause (FR-032-AC-18..20). This ensures
/// deterministic selection across both causes: if suite "a" failed and suite "z"
/// is missing, "a" is selected with `failed-run` cause.
pub(super) fn stale_evidence(
    indexed: &Indexed<'_>,
    obligation: &Obligation,
    bindings: &[&Binding],
) -> Option<Finding> {
    let mut stale_suites: Vec<StaleSuite<'_>> = Vec::new();

    // Collect failed runs
    for binding in bindings {
        let Some(run) = indexed.runs_by_suite.get(binding.suite.as_str()) else {
            continue;
        };
        let failed_entries = failed_bound_symbols(binding, run);
        if !failed_entries.is_empty() {
            let mut sorted_entries = failed_entries;
            sorted_entries.sort_by(|left, right| {
                js::compare(left.symbol.as_str(), right.symbol.as_str())
            });
            let Some(first_entry) = sorted_entries.first() else {
                continue;
            };
            stale_suites.push(StaleSuite {
                suite: binding.suite.as_str(),
                cause: StaleCause::FailedRun,
                symbol: Some(first_entry.symbol.as_str()),
                outcome: Some(first_entry.outcome),
            });
        }
    }

    // Collect missing runs
    for binding in bindings {
        if !indexed.runs_by_suite.contains_key(binding.suite.as_str())
            && !indexed.scans_by_suite.contains_key(binding.suite.as_str())
        {
            stale_suites.push(StaleSuite {
                suite: binding.suite.as_str(),
                cause: StaleCause::MissingRun,
                symbol: None,
                outcome: None,
            });
        }
    }

    if stale_suites.is_empty() {
        return None;
    }

    // Sort by suite name to get bytewise-first stale suite regardless of cause
    stale_suites.sort_by(|left, right| js::compare(left.suite, right.suite));
    let first = stale_suites.first()?;

    let finding = match first.cause {
        StaleCause::FailedRun => {
            let symbol = first.symbol?;
            let outcome = first.outcome?;
            let outcome_text = match outcome {
                Outcome::Fail => "failed",
                Outcome::Error => "error",
                _ => "failed",
            };
            Finding::new(
                FindingKind::STALE_EVIDENCE,
                &obligation.id,
                Severity::high(),
                format!(
                    "{id} is bound to {suite}:{symbol}, which reported {outcome} in the recorded run. The binding \
                     claims evidence that does not hold.",
                    id = obligation.id,
                    suite = first.suite,
                    symbol = symbol,
                    outcome = outcome_text
                ),
            )
            .with_cause(StaleCause::FailedRun)
        }
        StaleCause::MissingRun => {
            Finding::new(
                FindingKind::STALE_EVIDENCE,
                &obligation.id,
                Severity::high(),
                format!(
                    "{id} is bound to {suite}, which has no recorded run. The binding claims \
                     evidence that is not in the store.",
                    id = obligation.id,
                    suite = first.suite
                ),
            )
            .with_cause(StaleCause::MissingRun)
        }
        StaleCause::Invalid => {
            // This should never occur: the auditor only constructs FailedRun
            // or MissingRun causes. Invalid is a deserialization sentinel.
            return None;
        }
    };
    Some(finding)
}
