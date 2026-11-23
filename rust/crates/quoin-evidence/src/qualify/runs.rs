// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! What the retained runs say about one obligation (FR-153).
//!
//! Runs are ordered the way the store orders them (FR-030): by parsed UTC
//! instant, ties broken by the record's file name, whatever order the caller
//! supplied them in. The later run of a suite decides.

use std::collections::BTreeMap;

use super::{Cause, Refusal, RetainedRun, RunLocator};
use crate::ids::SuiteId;
use crate::parse_timestamp;
use crate::types::{Outcome, RunEntry};

/// A passing entry that is the latest word of its suite on one symbol.
pub(super) struct Candidate<'a> {
    pub(super) locator: RunLocator,
    pub(super) run: &'a RetainedRun,
    pub(super) entry: &'a RunEntry,
}

/// The disposition half of qualification, before any premise is read.
pub(super) struct Disposition<'a> {
    pub(super) candidates: Vec<Candidate<'a>>,
    pub(super) refusals: Vec<Refusal>,
}

/// Whether the entry states the obligation, whatever its outcome.
fn states(entry: &RunEntry, obligation: &str) -> bool {
    entry
        .trace_ids
        .as_deref()
        .is_some_and(|ids| ids.iter().any(|id| id == obligation))
}

fn locator(run: &RetainedRun, entry: &RunEntry) -> RunLocator {
    RunLocator {
        record: run.file.clone(),
        run: format!("{}@{}", run.record.suite, run.record.timestamp),
        entry: entry.symbol.to_string(),
    }
}

/// The suite's runs in the store's order (FR-030): parsed UTC instant, then the
/// record's file name, whatever order they were supplied in.
fn ordered(mut runs: Vec<&RetainedRun>) -> Vec<&RetainedRun> {
    runs.sort_by(|a, b| {
        parse_timestamp(&a.record.timestamp)
            .cmp(&parse_timestamp(&b.record.timestamp))
            .then_with(|| a.file.cmp(&b.file))
    });
    runs
}

fn by_suite(runs: &[RetainedRun]) -> BTreeMap<&SuiteId, Vec<&RetainedRun>> {
    let mut suites: BTreeMap<&SuiteId, Vec<&RetainedRun>> = BTreeMap::new();
    for run in runs {
        suites.entry(&run.record.suite).or_default().push(run);
    }
    suites
}

/// Whether more than one supplied observation claims the same canonical key.
fn duplicate_record_path(runs: &[&RetainedRun]) -> bool {
    let mut paths = std::collections::BTreeSet::new();
    runs.iter().any(|run| !paths.insert(run.file.as_str()))
}

fn invalid_timestamp(runs: &[&RetainedRun]) -> bool {
    runs.iter()
        .any(|run| parse_timestamp(&run.record.timestamp).is_none())
}

fn duplicate_instant(runs: &[&RetainedRun]) -> bool {
    let mut keys = std::collections::BTreeMap::new();
    runs.iter().any(|run| {
        let Some(key) = parse_timestamp(&run.record.timestamp) else {
            return false;
        };
        keys.insert(key, run.file.as_str())
            .is_some_and(|first_path| first_path != run.file)
    })
}

fn duplicate_path_refusal(
    suite: &SuiteId,
    runs: &[&RetainedRun],
    obligation: &str,
) -> Option<(bool, Refusal)> {
    duplicate_record_path(runs).then(|| {
        let stated = runs
            .iter()
            .flat_map(|run| &run.record.entries)
            .any(|entry| states(entry, obligation));
        (
            stated,
            Refusal {
                cause: Cause::RecordConflict,
                premise: None,
                locator: None,
                detail: format!(
                    "multiple retained observations name the same canonical record path for suite {suite}"
                ),
            },
        )
    })
}

/// The newest valid, unambiguous run of every suite, by the store's order. The one place the
/// latest run is chosen: the FR-032 audit is given these runs and the
/// supersession rules below read the same ones.
#[must_use]
pub fn latest_per_suite(runs: &[RetainedRun]) -> Vec<&RetainedRun> {
    by_suite(runs)
        .into_values()
        .filter_map(|suite_runs| {
            if invalid_timestamp(&suite_runs) || duplicate_instant(&suite_runs) {
                return None;
            }
            ordered(suite_runs).pop()
        })
        .collect()
}

/// Classify every symbol of every suite that states `obligation`.
pub(super) fn dispositions<'a>(obligation: &str, runs: &'a [RetainedRun]) -> Disposition<'a> {
    let mut candidates = Vec::new();
    let mut refusals = Vec::new();
    let mut stated_anywhere = false;
    for (suite, suite_runs) in by_suite(runs) {
        let suite_runs = ordered(suite_runs);
        if invalid_timestamp(&suite_runs) {
            let stated = suite_runs
                .iter()
                .flat_map(|run| &run.record.entries)
                .any(|entry| states(entry, obligation));
            stated_anywhere |= stated;
            refusals.push(Refusal {
                cause: Cause::RecordConflict,
                premise: None,
                locator: None,
                detail: format!("suite {suite} has a run with an invalid timestamp"),
            });
            continue;
        }
        if duplicate_instant(&suite_runs) {
            let stated = suite_runs
                .iter()
                .flat_map(|run| &run.record.entries)
                .any(|entry| states(entry, obligation));
            stated_anywhere |= stated;
            refusals.push(Refusal {
                cause: Cause::RecordConflict,
                premise: None,
                locator: None,
                detail: format!("suite {suite} has multiple records for the same instant"),
            });
            continue;
        }
        if let Some((stated, refusal)) = duplicate_path_refusal(suite, &suite_runs, obligation) {
            stated_anywhere |= stated;
            refusals.push(refusal);
            continue;
        }
        let Some((latest, earlier)) = suite_runs.split_last() else {
            continue;
        };
        let stated_before = suite_runs
            .iter()
            .flat_map(|run| &run.record.entries)
            .any(|entry| states(entry, obligation));
        stated_anywhere |= stated_before;
        // The latest run decides: the symbols it states are the suite's bound
        // symbols now, so a second report for another request replaces the
        // first (FR-153-AC-7) and a later run that states none leaves the
        // obligation absent (FR-153-AC-5).
        let current_entries: Vec<&RunEntry> = latest
            .record
            .entries
            .iter()
            .filter(|entry| states(entry, obligation))
            .collect();
        if current_entries.is_empty() && stated_before {
            refusals.push(Refusal {
                cause: Cause::Absent,
                premise: None,
                locator: None,
                detail: format!(
                    "the latest run of suite {suite} holds no entry stating {obligation}"
                ),
            });
        }
        for current in current_entries {
            let symbol = &current.symbol;
            let here = locator(latest, current);
            let superseded_pass = earlier.iter().rev().find_map(|run| {
                run.record
                    .entries
                    .iter()
                    .find(|entry| {
                        &entry.symbol == symbol
                            && states(entry, obligation)
                            && entry.outcome == Outcome::Pass
                    })
                    .map(|entry| locator(run, entry))
            });
            let failing = match current.outcome {
                Outcome::Pass => None,
                Outcome::Fail => Some((Cause::Failed, "reported failure")),
                Outcome::Error => Some((Cause::Errored, "reported an error")),
                Outcome::Skip => Some((Cause::Skipped, "was skipped")),
            };
            if let Some((cause, what)) = failing {
                if let Some(earlier_pass) = superseded_pass {
                    refusals.push(Refusal {
                        cause: Cause::Stale,
                        premise: None,
                        locator: Some(earlier_pass),
                        detail: format!(
                            "an earlier passing run is superseded: the later run of suite {suite} {what} on {symbol}"
                        ),
                    });
                }
                refusals.push(Refusal {
                    cause,
                    premise: None,
                    locator: Some(here),
                    detail: format!("{symbol} in suite {suite} {what}"),
                });
            } else {
                candidates.push(Candidate {
                    locator: here,
                    run: latest,
                    entry: current,
                });
            }
        }
    }
    if !stated_anywhere {
        refusals.push(Refusal {
            cause: Cause::Absent,
            premise: None,
            locator: None,
            detail: format!("no retained entry states {obligation}"),
        });
    }
    Disposition {
        candidates,
        refusals,
    }
}
