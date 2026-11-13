// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Counting an analysis result as retained evidence (FR-153; quire-rs FR-078).
//!
//! The bridge asks the Quoin qualifier ([`quoin_evidence::qualify`]) about
//! each requested obligation and hands the answer to the engine's retained
//! evidence reconciliation. The only coverage credit an analysis result earns
//! is the engine's `qualified-record` status, and only from a positive
//! qualifier decision. A binding, a passing entry or a healthy audit is not
//! one (FR-153-AC-8).
//!
//! It reads the supplied observations and nothing else: no solver runs, no
//! report is re-read from outside them, nothing is written (FR-153-CON-2). It
//! adds no field, status or report member to the engine (FR-153-CON-1), and a
//! record claim never becomes a source symbol: the retained entry's own
//! `<analysisKind>::<requestDigest>` identity is a locator, not a symbol
//! (FR-153-AC-9).

use quire_rs::evidence::{
    ActResult, BackingInput, CauseCode, EvidenceTarget, RecordBinder, RecordClaim, RecordLocator,
    TraceQuery,
};
use quire_rs::obligation::inventory::{InventoryOptions, SelectedInventory, inventory};
use quoin_evidence::qualify::{
    self, Cause, Decision, HostObservations, Qualification, Refusal, RetainedRun, RunLocator,
};
use quoin_evidence::types::Binding;

use crate::coverage::{self, Request as ScopeRequest, Walked};
use crate::error::{Error, Result};
use crate::modules::Notice;

/// One counting run.
#[derive(Debug)]
pub struct Request {
    /// The scope and modules whose obligations are counted.
    pub scope: ScopeRequest,
    /// The obligation ids to count, as the caller requests them.
    pub requested: Vec<String>,
    /// Every retained run, in any order.
    pub runs: Vec<RetainedRun>,
    /// The binding graph.
    pub bindings: Vec<Binding>,
    /// The host's admitted observations.
    pub host: HostObservations,
}

/// What a counting run produced.
#[derive(Debug)]
pub struct Outcome {
    /// The engine's report, with `record_binders` and `evidence_results`.
    pub report: quire_rs::CoverageReport,
    /// Advisory notices from the walks.
    pub notices: Vec<Notice>,
    inventory: SelectedInventory,
    accepted: Vec<(usize, Decision)>,
}

/// A non-qualified result held until the engine borrows it.
struct Pending {
    entry: Option<usize>,
    requested: String,
    cause: CauseCode,
    locator: Option<RunLocator>,
    detail: String,
}

impl Pending {
    fn unresolved(requested: &str, cause: CauseCode, detail: &str) -> Self {
        Self {
            entry: None,
            requested: requested.to_owned(),
            cause,
            locator: None,
            detail: detail.to_owned(),
        }
    }

    fn refused(entry: usize, requested: &str, refusal: Refusal) -> Self {
        Self {
            entry: Some(entry),
            requested: requested.to_owned(),
            cause: cause_code(refusal.cause),
            locator: refusal.locator,
            detail: refusal.detail,
        }
    }
}

fn cause_code(cause: Cause) -> CauseCode {
    match cause {
        Cause::Failed => CauseCode::Failed,
        Cause::Errored => CauseCode::Errored,
        Cause::Skipped => CauseCode::Skipped,
        Cause::Absent => CauseCode::Absent,
        Cause::Unevaluated => CauseCode::Unevaluated,
        Cause::WrongRevision => CauseCode::WrongRevision,
        Cause::Stale => CauseCode::Stale,
        Cause::RecordConflict => CauseCode::RecordConflict,
        Cause::QualificationRejected => CauseCode::QualificationRejected,
    }
}

fn locator_of(locator: &RunLocator) -> RecordLocator<'_> {
    RecordLocator {
        record: &locator.record,
        run: &locator.run,
        entry: &locator.entry,
    }
}

/// The inventory entries that state `id`, as their ordinals.
fn entries_stating(inventory: &SelectedInventory, id: &str) -> Vec<usize> {
    inventory
        .entries()
        .enumerate()
        .filter(|(_, entry)| entry.obligation().id == id)
        .map(|(ordinal, _)| ordinal)
        .collect()
}

/// Count the requested obligations.
///
/// # Errors
/// As [`coverage::compute`].
pub fn compute(request: &Request) -> Result<Outcome> {
    let Walked {
        registry,
        spec,
        graph,
        notices,
    } = coverage::walk(&request.scope)?;
    let model = registry
        .traceability()
        .ok_or(Error::TraceabilityModelUndeclared)?;
    let sources: Vec<&str> = model
        .obligations
        .iter()
        .map(|source| source.name.as_str())
        .collect();
    let selected = inventory(
        request.scope.scope.as_path(),
        model,
        &sources,
        InventoryOptions::default(),
    );

    let mut accepted: Vec<(usize, Decision)> = Vec::new();
    let mut pending: Vec<Pending> = Vec::new();
    for id in &request.requested {
        let found = entries_stating(&selected, id);
        let (ordinal, method) = match found.as_slice() {
            [ordinal] => (
                *ordinal,
                selected
                    .entries()
                    .nth(*ordinal)
                    .and_then(|entry| entry.obligation().method.clone()),
            ),
            [] => {
                pending.push(Pending::unresolved(
                    id,
                    CauseCode::InvalidTarget,
                    "no selected obligation has this id",
                ));
                continue;
            }
            _ => {
                pending.push(Pending::unresolved(
                    id,
                    CauseCode::AmbiguousTarget,
                    "more than one selected row has this id",
                ));
                continue;
            }
        };
        let answer = qualify::qualify(&qualify::Request {
            obligation: id,
            declared_method: method.as_deref(),
            runs: &request.runs,
            bindings: &request.bindings,
            host: &request.host,
        });
        match answer {
            Qualification::Qualified(decisions) => {
                accepted.extend(decisions.into_iter().map(|decision| (ordinal, decision)));
            }
            Qualification::NotQualified(refusals) => {
                pending.extend(
                    refusals
                        .into_iter()
                        .map(|refusal| Pending::refused(ordinal, id, refusal)),
                );
            }
        }
    }

    let report = with_backing(&selected, &accepted, &pending, |backing| {
        quire_rs::coverage::compute_with_evidence(
            &spec,
            &registry,
            &graph,
            request.scope.scope.as_path(),
            backing,
        )
        .map_err(coverage::coverage_error)
    })?;
    Ok(Outcome {
        report,
        notices,
        inventory: selected,
        accepted,
    })
}

/// Build the engine's input over borrowed views and run `use_backing` on it.
fn with_backing<T>(
    selected: &SelectedInventory,
    accepted: &[(usize, Decision)],
    pending: &[Pending],
    use_backing: impl FnOnce(&BackingInput<'_>) -> T,
) -> T {
    let target = |ordinal: usize| {
        selected
            .entries()
            .nth(ordinal)
            .map(|entry| EvidenceTarget::from(entry.identity()))
    };
    let provenance: Vec<Vec<&str>> = accepted
        .iter()
        .map(|(_, decision)| decision.provenance.iter().map(String::as_str).collect())
        .collect();
    let claims: Vec<RecordClaim<'_>> = accepted
        .iter()
        .zip(&provenance)
        .filter_map(|((ordinal, decision), provenance)| {
            Some(RecordClaim {
                target: target(*ordinal)?,
                locator: locator_of(&decision.locator),
                method: &decision.method,
                actor: &decision.actor,
                provenance,
            })
        })
        .collect();
    let results: Vec<ActResult<'_>> = pending
        .iter()
        .map(|item| ActResult {
            target: item.entry.and_then(target),
            requested_target: &item.requested,
            locator: item.locator.as_ref().map(locator_of),
            cause: item.cause,
            detail: &item.detail,
        })
        .collect();
    use_backing(&BackingInput {
        inventory: selected,
        claims: &claims,
        results: &results,
    })
}

/// How a record claim is looked up.
#[derive(Debug, Clone, Copy)]
pub enum Query<'a> {
    /// Forward: the obligation id.
    Obligation(&'a str),
    /// Inverse: the original record, run and entry locators.
    Record(&'a RunLocator),
    /// Every entry of one run locator.
    Run(&'a str),
}

impl Outcome {
    /// The record claims the counted obligations carry (FR-153-AC-9), from the
    /// same projection the coverage report used.
    #[must_use]
    pub fn trace(&self, query: Query<'_>) -> Vec<RecordBinder> {
        with_backing(
            &self.inventory,
            &self.accepted,
            &[],
            |backing| match query {
                Query::Obligation(id) => entries_stating(&self.inventory, id)
                    .into_iter()
                    .filter_map(|ordinal| self.inventory.entries().nth(ordinal))
                    .flat_map(|entry| {
                        quire_rs::evidence::trace(
                            backing,
                            TraceQuery::Target(EvidenceTarget::from(entry.identity())),
                        )
                    })
                    .collect(),
                Query::Record(locator) => {
                    quire_rs::evidence::trace(backing, TraceQuery::Record(locator_of(locator)))
                }
                Query::Run(run) => quire_rs::evidence::trace(backing, TraceQuery::Run(run)),
            },
        )
    }
}
