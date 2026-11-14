// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Shared stale-run predicates consumed by evidence exports and the auditor.

use crate::types::{Binding, Outcome, RunEntry, RunRecord};

/// The selected run entries where a binding names a symbol that failed or errored.
///
/// This is the failed-run condition behind `STALE_EVIDENCE` in the auditor.
/// Callers decide how a missing run is treated for their own evidence contract.
#[must_use]
pub fn failed_bound_symbols<'a>(binding: &Binding, run: &'a RunRecord) -> Vec<&'a RunEntry> {
    binding
        .symbols
        .iter()
        .filter_map(|symbol| {
            run.entries
                .iter()
                .find(|entry| entry.symbol.as_str() == symbol.as_str())
        })
        .filter(|entry| matches!(entry.outcome, Outcome::Fail | Outcome::Error))
        .collect()
}
