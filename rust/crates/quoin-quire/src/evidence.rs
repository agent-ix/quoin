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
//! record claim never becomes a source symbol: the retained entry's
//! `analysisKind` identity is a locator, not a symbol
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
    /// Exact FR-078 identity for each requested ID when supplied by the host.
    /// Kept parallel to `requested`; absent identities retain ID-only behavior.
    pub requested_targets: Vec<Option<quoin_quire_types::TargetLocus>>,
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
        Cause::SelectionMismatch => CauseCode::SelectionMismatch,
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

fn target_locus(
    identity: quire_rs::obligation::inventory::CriterionIdentity<'_>,
) -> quoin_quire_types::TargetLocus {
    let (id, origin, row) = match identity.origin() {
        quire_rs::obligation::inventory::CriterionOrigin::AuthoredRow { authored_id } => {
            (authored_id, "authored-row", None)
        }
        quire_rs::obligation::inventory::CriterionOrigin::RenderedRow { rendered_id, row } => {
            (rendered_id, "rendered-row", Some(row))
        }
        quire_rs::obligation::inventory::CriterionOrigin::CombinatorialTable { rendered_id } => {
            (rendered_id, "combinatorial-table", None)
        }
    };
    quoin_quire_types::TargetLocus {
        source: identity.source_name().to_owned(),
        document: identity.document().to_owned(),
        id: id.to_owned(),
        origin: origin.to_owned(),
        row,
        line: None,
    }
}

fn same_target_identity(
    left: &quoin_quire_types::TargetLocus,
    right: &quoin_quire_types::TargetLocus,
) -> bool {
    left.source == right.source
        && left.document == right.document
        && left.id == right.id
        && left.origin == right.origin
        && left.row == right.row
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
                        identity: None,
                        document: None,
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
    compute_with(request, CoveragePurpose::Strict, |_, _, _| Ok(())).map(|(outcome, ())| outcome)
}

/// Count the requested obligations and derive design rows from the same coverage.
///
/// # Errors
/// As [`compute`], plus errors in the declared design relations.
#[implements("FR-165-AC-21")]
pub fn compute_for_matrix(
    request: &Request,
) -> Result<(Outcome, crate::design_matrix::DesignMatrix)> {
    compute_with(
        request,
        CoveragePurpose::DesignMatrix,
        |spec, registry, report| {
            crate::design_matrix::for_coverage(
                request.scope.scope.as_path(),
                spec,
                registry,
                report,
            )
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoveragePurpose {
    Strict,
    DesignMatrix,
}

#[implements("FR-165-AC-21")]
fn compute_with<T>(
    request: &Request,
    purpose: CoveragePurpose,
    finish: impl FnOnce(&quire_rs::Spec, &quire_rs::Registry, &quire_rs::CoverageReport) -> Result<T>,
) -> Result<(Outcome, T)> {
    let Walked {
        registry,
        spec,
        graph,
        notices,
    } = match purpose {
        CoveragePurpose::Strict => coverage::walk(&request.scope)?,
        CoveragePurpose::DesignMatrix => coverage::walk_for_design_matrix(&request.scope)?,
    };
    // The shared walk checked this registry already; the model is read again
    // here, so ask again before reading it (FR-029).
    match purpose {
        CoveragePurpose::Strict => crate::modules::require_complete_model(&registry)?,
        CoveragePurpose::DesignMatrix => {
            crate::modules::require_design_matrix_compatible_model(&registry)?;
        }
    }
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
        InventoryOptions {
            model_policy: match purpose {
                CoveragePurpose::Strict => {
                    quire_rs::obligation::inventory::InventoryModelPolicy::Strict
                }
                CoveragePurpose::DesignMatrix => {
                    quire_rs::obligation::inventory::InventoryModelPolicy::DesignMatrix
                }
            },
            ..InventoryOptions::default()
        },
    );
    let entries: Vec<CriterionEntry<'_>> = selected.entries().collect();
    let audit = audit_report(request, &entries)?;

    let (accepted, pending) = resolve_requests(request, &entries, &audit);

    let report = with_backing(&selected, &entries, &accepted, &pending, |backing| {
        quire_rs::coverage::compute_with_evidence(
            &spec,
            &registry,
            &graph,
            request.scope.scope.as_path(),
            backing,
            match purpose {
                CoveragePurpose::Strict => quire_rs::coverage::EvidenceCoverageMode::Strict,
                CoveragePurpose::DesignMatrix => {
                    quire_rs::coverage::EvidenceCoverageMode::DesignMatrix
                }
            },
        )
        .map_err(coverage::coverage_error)
    })?;
    drop(entries);
    let extra = finish(&spec, &registry, &report)?;
    Ok((
        Outcome {
            report,
            notices,
            inventory: selected,
            accepted,
        },
        extra,
    ))
}

#[implements("FR-165-AC-21")]
fn resolve_requests(
    request: &Request,
    entries: &[CriterionEntry<'_>],
    audit: &quoin_finding_types::AuditReport,
) -> (Vec<(usize, Decision)>, Vec<Pending>) {
    let mut accepted = Vec::new();
    let mut pending = Vec::new();
    for (index, id) in request.requested.iter().enumerate() {
        let (decisions, refusals) = resolve_request(request, entries, audit, index, id);
        accepted.extend(decisions);
        pending.extend(refusals);
    }
    (accepted, pending)
}

#[implements("FR-165-AC-21")]
fn resolve_request(
    request: &Request,
    entries: &[CriterionEntry<'_>],
    audit: &quoin_finding_types::AuditReport,
    requested_index: usize,
    id: &str,
) -> (Vec<(usize, Decision)>, Vec<Pending>) {
    let identity = request
        .requested_targets
        .get(requested_index)
        .and_then(Option::as_ref);
    let (ordinal, method) = match select_requested_entry(entries, id, identity) {
        Ok(selected) => selected,
        Err(refusal) => return (Vec::new(), vec![*refusal]),
    };
    if let Some(identity) = identity
        && let Some(refusal) = conflicting_selection(request, ordinal, id, identity)
    {
        return (Vec::new(), vec![refusal]);
    }
    let about = audit_outcome(audit, id);
    let answer = qualify::qualify(&qualify::Request {
        obligation: id,
        declared_method: method.as_deref(),
        runs: &request.runs,
        audit: Some(&about),
        host: &request.host,
    });
    match answer {
        Qualification::Qualified(decisions) => (
            decisions
                .into_iter()
                .map(|decision| (ordinal, decision))
                .collect(),
            Vec::new(),
        ),
        Qualification::NotQualified(refusals) => (
            Vec::new(),
            refusals
                .into_iter()
                .map(|refusal| Pending::refused(ordinal, id, refusal))
                .collect(),
        ),
    }
}

#[implements("FR-165-AC-21")]
fn select_requested_entry(
    entries: &[CriterionEntry<'_>],
    id: &str,
    identity: Option<&quoin_quire_types::TargetLocus>,
) -> std::result::Result<(usize, Option<String>), Box<Pending>> {
    let found = ordinals_stating(entries, id);
    let matched = identity.and_then(|identity| {
        found.iter().copied().find(|ordinal| {
            entries.get(*ordinal).is_some_and(|entry| {
                let actual = target_locus(entry.identity());
                same_target_identity(&actual, identity)
            })
        })
    });
    let ordinal = match (identity, matched, found.as_slice()) {
        (Some(_), Some(ordinal), _) => ordinal,
        (None, _, [ordinal]) => *ordinal,
        (Some(identity), None, [ordinal]) => {
            return Err(Box::new(Pending {
                entry: Some(*ordinal),
                requested: id.to_owned(),
                cause: CauseCode::SelectionMismatch,
                locator: None,
                detail: format!(
                    "requested target {}:{}:{}:{} is not the selected FR-078 target",
                    identity.source, identity.document, identity.id, identity.origin
                ),
            }));
        }
        (Some(_), None, []) | (None, _, []) => {
            return Err(Box::new(Pending::unresolved(
                id,
                CauseCode::InvalidTarget,
                "no selected obligation has this id",
            )));
        }
        (Some(_), None, _) => {
            return Err(Box::new(Pending::unresolved(
                id,
                CauseCode::AmbiguousTarget,
                "more than one selected row has this id and no exact target identity matched",
            )));
        }
        (None, _, _) => {
            return Err(Box::new(Pending::unresolved(
                id,
                CauseCode::AmbiguousTarget,
                "more than one selected row has this id",
            )));
        }
    };
    let method = entries
        .get(ordinal)
        .and_then(|entry| entry.obligation().method.clone());
    Ok((ordinal, method))
}

#[implements("FR-165-AC-21")]
fn conflicting_selection(
    request: &Request,
    ordinal: usize,
    id: &str,
    identity: &quoin_quire_types::TargetLocus,
) -> Option<Pending> {
    let run = request.runs.iter().find(|run| {
        run.record.entries.iter().any(|entry| {
            entry
                .trace_ids
                .iter()
                .flatten()
                .any(|trace_id| trace_id == id)
        }) && run
            .observation
            .selected_target
            .as_ref()
            .is_some_and(|target| target.id == id && !same_target_identity(target, identity))
    })?;
    let admitted = run.observation.selected_target.as_ref();
    Some(Pending {
        entry: Some(ordinal),
        requested: id.to_owned(),
        cause: CauseCode::SelectionMismatch,
        locator: Some(RunLocator {
            record: run.file.clone(),
            run: format!("{}/{}", run.record.suite, run.record.timestamp),
            entry: run
                .record
                .entries
                .first()
                .map_or_else(String::new, |entry| entry.symbol.to_string()),
        }),
        detail: format!(
            "requested target {}:{}:{}:{} conflicts with the admitted target {}:{}:{}:{}",
            identity.source,
            identity.document,
            identity.id,
            identity.origin,
            admitted.map_or("", |target| target.source.as_str()),
            admitted.map_or("", |target| target.document.as_str()),
            admitted.map_or("", |target| target.id.as_str()),
            admitted.map_or("", |target| target.origin.as_str())
        ),
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
