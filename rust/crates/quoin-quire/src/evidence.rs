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

use ix_trace_rs::implements;
use quire_rs::evidence::{
    ActResult, BackingInput, CauseCode, EvidenceTarget, RecordBinder, RecordClaim, RecordLocator,
    TraceQuery,
};
use quire_rs::obligation::inventory::{
    CriterionEntry, InventoryOptions, SelectedInventory, inventory,
};
use quoin_evidence::qualify::{
    self, AuditFinding, AuditOutcome, Cause, Decision, HostObservations, Qualification, Refusal,
    RetainedRun, RunLocator,
};
use quoin_evidence::types::{Binding, MockInjection};

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
    /// Every retained run, in any order, with its own host observations.
    pub runs: Vec<RetainedRun>,
    /// The binding graph, read by the FR-032 audit.
    pub bindings: Vec<Binding>,
    /// What each suite's tests inject in place of real behaviour, as the host
    /// inspected it. Absent means nobody looked, so the audit cannot call an
    /// obligation healthy (FR-032, #204).
    pub injections: Option<Vec<MockInjection>>,
    /// The suites with a current completed mock inspection.
    pub mock_inspection_suites: Option<Vec<String>>,
    /// The host's observations that belong to no one record.
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
fn ordinals_stating(entries: &[CriterionEntry<'_>], id: &str) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.obligation().id == id)
        .map(|(ordinal, _)| ordinal)
        .collect()
}

/// The shared FR-032 audit, run over the newest run of every suite the
/// qualifier also reads, for the requested obligations.
fn audit_report(
    request: &Request,
    entries: &[CriterionEntry<'_>],
) -> Result<quoin_finding_types::AuditReport> {
    let obligations = request
        .requested
        .iter()
        .filter_map(|id| {
            let found = ordinals_stating(entries, id);
            // An unresolved or ambiguous id is refused before any audit.
            match found.as_slice() {
                [ordinal] => entries.get(*ordinal).map(|entry| {
                    let source = entry.obligation();
                    quoin_quire_types::Obligation {
                        id: source.id.clone(),
                        statement: source.statement.clone(),
                        target_ids: None,
                        method: source.method.clone(),
                        criticality: source.criticality.clone(),
                        parameters: (!source.parameters.is_empty())
                            .then(|| source.parameters.clone()),
                    }
                }),
                _ => None,
            }
        })
        .collect();
    quoin_auditor::audit(&quoin_auditor::AuditInput {
        obligations,
        bindings: request.bindings.clone(),
        runs: qualify::latest_per_suite(&request.runs)
            .into_iter()
            .map(|run| run.record.clone())
            .collect(),
        injections: request.injections.clone(),
        mock_inspection_suites: request.mock_inspection_suites.clone(),
        ..quoin_auditor::AuditInput::default()
    })
    .map_err(|error| Error::AuditFailed {
        code: error.code,
        reason: error.message.into_string(),
    })
}

/// What the audit says about one obligation: healthy or not, with every
/// finding and every check it could not evaluate.
fn audit_outcome(report: &quoin_finding_types::AuditReport, id: &str) -> AuditOutcome {
    let findings = report
        .findings
        .iter()
        .filter(|finding| finding.obligation == id)
        .map(|finding| AuditFinding {
            kind: finding.kind.clone(),
            summary: finding.summary.clone(),
        });
    let unevaluated = report
        .unevaluated
        .iter()
        .filter(|check| check.obligation == id)
        .map(|check| AuditFinding {
            kind: check.check.clone(),
            summary: check.reason.clone(),
        });
    AuditOutcome {
        healthy: report.healthy.iter().any(|healthy| healthy == id),
        details: findings.chain(unevaluated).collect(),
    }
}

/// Count the requested obligations.
///
/// # Errors
/// As [`coverage::compute`], and [`Error::AuditFailed`] when the FR-032 audit
/// of the retained runs cannot be built.
#[implements("FR-153")]
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
    let entries: Vec<CriterionEntry<'_>> = selected.entries().collect();
    let audit = audit_report(request, &entries)?;

    let mut accepted: Vec<(usize, Decision)> = Vec::new();
    let mut pending: Vec<Pending> = Vec::new();
    for id in &request.requested {
        let found = ordinals_stating(&entries, id);
        let (ordinal, method) = match found.as_slice() {
            [ordinal] => (
                *ordinal,
                entries
                    .get(*ordinal)
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
        let about = audit_outcome(&audit, id);
        let answer = qualify::qualify(&qualify::Request {
            obligation: id,
            declared_method: method.as_deref(),
            runs: &request.runs,
            audit: Some(&about),
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

    let report = with_backing(&selected, &entries, &accepted, &pending, |backing| {
        quire_rs::coverage::compute_with_evidence(
            &spec,
            &registry,
            &graph,
            request.scope.scope.as_path(),
            backing,
        )
        .map_err(coverage::coverage_error)
    })?;
    drop(entries);
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
    entries: &[CriterionEntry<'_>],
    accepted: &[(usize, Decision)],
    pending: &[Pending],
    use_backing: impl FnOnce(&BackingInput<'_>) -> T,
) -> T {
    let target = |ordinal: usize| {
        entries
            .get(ordinal)
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
        let entries: Vec<CriterionEntry<'_>> = self.inventory.entries().collect();
        with_backing(
            &self.inventory,
            &entries,
            &self.accepted,
            &[],
            |backing| match query {
                Query::Obligation(id) => ordinals_stating(&entries, id)
                    .into_iter()
                    .filter_map(|ordinal| entries.get(ordinal))
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
