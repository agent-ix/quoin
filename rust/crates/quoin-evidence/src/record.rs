// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Transcribing a suite run into the store (FR-030).
//!
//! quoin **transcribes**; the consumer's CI executes (ADR-0011 invariant 1).
//! Nothing here runs a test, and nothing here decides whether a result is
//! acceptable — recording a failing run is as legitimate as recording a passing
//! one, and more useful, because a suite that stopped passing is exactly what a
//! freshness check needs to see.

use std::collections::{BTreeMap, BTreeSet};

use quoin_quire_types::Obligation;

use crate::error::EvidenceError;
use crate::ids::{Commit, ObligationId, StatementHash, SuiteId, SymbolId};
use crate::source::EvidenceSource;
use crate::store::{bind, read_bindings, write_bindings, write_run};
use crate::types::{Binding, EvidenceLineage, Outcome, RunEntry, RunRecord, STORE_SCHEMA_VERSION};

/// What a caller supplies to record one run.
#[derive(Debug, Clone)]
pub struct RecordRequest {
    /// The suite that ran.
    pub suite: SuiteId,
    /// The commit it ran at.
    pub commit: Commit,
    /// Tool and version, as the adapter reported them.
    pub tool: String,
    /// The declared `test_type`, when the caller names one.
    pub evidence_kind: Option<String>,
    /// Separation facts for each binding this run creates.
    pub lineage: Option<EvidenceLineage>,
    /// ISO-8601. Passed in rather than read from the clock, so a record is
    /// reproducible.
    pub timestamp: String,
    /// The transcribed producer results.
    pub entries: Vec<RunEntry>,
}

/// What recording a run changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordOutcome {
    /// Where the run record was written, store-relative.
    pub run_path: String,
    /// Obligations bound for the first time by this run, sorted.
    pub bound: Vec<ObligationId>,
    /// Obligations already bound whose statement has changed since, sorted.
    ///
    /// Reported, never auto-cleared: re-running a test does not re-affirm a
    /// reworded requirement.
    pub suspect: Vec<ObligationId>,
    /// Trace ids in the run that match no derived obligation, sorted.
    pub unmatched: Vec<String>,
}

/// Record one suite run and update the binding graph.
///
/// The join is: run entry → trace ids it carries → obligation of the same id.
/// A trace id matching no obligation is reported rather than dropped, because
/// that is the quire-rs#72 class arriving from the other direction — a test
/// claiming to verify something the spec does not state.
///
/// `obligations` are the obligations as quire derives them **today**. Read live
/// rather than stored: an obligation is always re-derivable from spec and code
/// at HEAD, so a stored copy could only ever disagree with the requirement it
/// describes.
///
/// # Errors
///
/// [`EvidenceError::StoreRead`] when `bindings.json` is present and
/// unparseable, otherwise [`EvidenceError::Canonicalization`] or
/// [`EvidenceError::StoreIo`].
pub fn record_run<S: EvidenceSource + ?Sized>(
    source: &mut S,
    request: &RecordRequest,
    obligations: &[Obligation],
) -> Result<RecordOutcome, EvidenceError> {
    let run_path = write_run(
        source,
        &RunRecord {
            schema_version: STORE_SCHEMA_VERSION,
            suite: request.suite.clone(),
            commit: request.commit.clone(),
            tool: request.tool.clone(),
            evidence_kind: request.evidence_kind.clone(),
            timestamp: request.timestamp.clone(),
            entries: request.entries.clone(),
        },
    )?;

    let by_id: BTreeMap<&str, &Obligation> = obligations
        .iter()
        .map(|obligation| (obligation.id.as_str(), obligation))
        .collect();

    // Test-case id → the obligations whose method cell names it.
    //
    // A tool reports the id it knows: a unit test carries the criterion's own
    // id because the tag is written in the test; an agent-eval report and any
    // other Test-Matrix-keyed tool carries the test case id. Both are stated by
    // the same criteria row, and quire-rs FR-053-AC-11 carries the join on the
    // obligation, so this resolves it rather than re-parsing the table
    // (agent-ix/quoin#144).
    let mut by_target: BTreeMap<&str, Vec<&Obligation>> = BTreeMap::new();
    for obligation in obligations {
        for target in obligation.target_ids.iter().flatten() {
            // An id that is ALSO an obligation id stays an obligation id.
            // Otherwise a criterion naming a sibling criterion would silently
            // bind through the indirect route and report a discharge nobody
            // stated directly.
            if by_id.contains_key(target.as_str()) {
                continue;
            }
            by_target
                .entry(target.as_str())
                .or_default()
                .push(obligation);
        }
    }

    let mut discharged: BTreeMap<&str, Vec<SymbolId>> = BTreeMap::new();
    let mut unmatched: BTreeSet<String> = BTreeSet::new();

    for entry in &request.entries {
        for id in entry.trace_ids.iter().flatten() {
            let matched: Vec<&Obligation> = by_id.get(id.as_str()).map_or_else(
                || by_target.get(id.as_str()).cloned().unwrap_or_default(),
                |obligation| vec![*obligation],
            );
            if matched.is_empty() {
                unmatched.insert(id.clone());
                continue;
            }
            // Only a passing symbol discharges. A failing or skipped test is
            // evidence that the suite ran, which the run record already holds —
            // it is not evidence that the obligation holds, and binding on it
            // would make the store agree with a red build.
            if entry.outcome != Outcome::Pass {
                continue;
            }
            for obligation in matched {
                discharged
                    .entry(obligation.id.as_str())
                    .or_default()
                    .push(entry.symbol.clone());
            }
        }
    }

    let mut bindings = read_bindings(source)?.bindings;
    let mut bound = Vec::new();
    let mut suspect = Vec::new();

    // `BTreeMap` iteration is already the plain byte order the retained source
    // sorts into, and deliberately not a locale collation: the store is written
    // from this order and two machines with different ICU data would otherwise
    // produce a diff nobody made (agent-ix/quoin#106).
    for (id, symbols) in discharged {
        let Some(obligation) = by_id.get(id) else {
            continue;
        };
        let mut sorted = symbols;
        sorted.sort();
        let outcome = bind(
            &bindings,
            &Binding {
                obligation: ObligationId::new(id),
                statement_hash_at_binding: StatementHash::new(obligation.statement_hash.clone()),
                suite: request.suite.clone(),
                commit: request.commit.clone(),
                symbols: sorted,
                lineage: request.lineage.clone(),
                affirmations: None,
            },
        );
        bindings = outcome.bindings;
        if outcome.created {
            bound.push(ObligationId::new(id));
        }
        if outcome.suspect {
            suspect.push(ObligationId::new(id));
        }
    }

    write_bindings(source, &bindings)?;

    bound.sort();
    suspect.sort();
    Ok(RecordOutcome {
        run_path,
        bound,
        suspect,
        unmatched: unmatched.into_iter().collect(),
    })
}
