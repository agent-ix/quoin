// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The owner qualifier for the analysis act (FR-116 with the producer
//! definition of FR-184), and the counting rules of FR-153.
//!
//! [`qualify`] answers one question for one obligation: does the retained
//! evidence supply a positive decision for this exact target and its declared
//! method? It is a pure reader of what the host supplies (FR-116-CON-1): the
//! retained runs, the binding graph, the reports the runs transcribe and the
//! host's admitted observations. It runs no solver, reads no clock and writes
//! nothing (FR-153-CON-2, FR-184-CON-2).
//!
//! A positive decision is rare by design. The credit is consistency evidence
//! only, and while no FR-093 assessment exists or the CI checkout SHA is
//! missing, every analysis-backed obligation is `unevaluated`.

mod premises;
mod runs;

pub use runs::latest_per_suite;

use ix_trace_rs::implements;
use serde_json::Value;

use crate::types::{RunRecord, TrustDecision};

/// The declared method an analysis act can discharge.
pub const ANALYSIS_METHOD: &str = "Analysis";

/// The identity of the producer definition the owner approved (FR-184).
pub const PRODUCER_DEFINITION: &str = "analysis-act/1";

/// The decision an FR-093 assessment must permit for the positive use.
pub const POSITIVE_USE_DECISION: &str = "qualify-analysis-act";

/// The stable non-qualified categories of the FR-116 output table that this
/// qualifier can reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Cause {
    /// A required act reported failure.
    Failed,
    /// A required act or its producer reported an error.
    Errored,
    /// A required act was skipped; not a waiver.
    Skipped,
    /// A required retained record or act result is absent.
    Absent,
    /// A required premise was not supplied or evaluated.
    Unevaluated,
    /// The performed revision is not the selected HEAD.
    WrongRevision,
    /// Newer evidence or an invalidated reliance supersedes the pass.
    Stale,
    /// Observations contradict each other.
    RecordConflict,
    /// An evaluated premise refuses the act.
    QualificationRejected,
}

/// The FR-116 premise a refusal names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Premise {
    /// The FR-032 audit of the latest run (FR-153 credit step 2).
    Audit,
    /// The declared method and the act kind pair (FR-116-AC-11).
    MethodActPairing,
    /// The FR-093 assessment for the positive use.
    BoundedReliance,
    /// The signer admitted as the asserting actor.
    ActorAuthority,
    /// The CI checkout SHA against the selected HEAD.
    PerformedRevision,
    /// The CI run, attempt, job and invocation.
    ExecutionAssociation,
    /// The producer, solver identities and `producer-execute` observation.
    ToolAndConfiguration,
    /// One report states one obligation (FR-184 owner answer 8).
    ActScope,
}

impl Premise {
    /// The premise's stable name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Audit => "FR-032 audit",
            Self::MethodActPairing => "method and act pairing",
            Self::BoundedReliance => "bounded reliance",
            Self::ActorAuthority => "actor authority",
            Self::PerformedRevision => "performed revision",
            Self::ExecutionAssociation => "execution association",
            Self::ToolAndConfiguration => "tool and configuration",
            Self::ActScope => "act scope",
        }
    }

    /// A refusal's detail text naming this premise.
    pub(crate) fn detail(self, reason: &str) -> String {
        format!("{} premise: {reason}", self.as_str())
    }
}

/// Where a retained entry lives.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RunLocator {
    /// The run record's store-relative file.
    pub record: String,
    /// The run: its suite and timestamp.
    pub run: String,
    /// The entry within the run: its symbol.
    pub entry: String,
}

/// One retained run, as the host read it, with the report it transcribes.
#[derive(Debug, Clone)]
pub struct RetainedRun {
    /// The record's store-relative file name; the tie-breaker of the run order.
    pub file: String,
    /// The record as the store holds it.
    pub record: RunRecord,
    /// The `quire.differential-report/v2` document the record was transcribed
    /// from. Absent when the host did not supply it.
    pub report: Option<Value>,
    /// What the host observed of the act that produced this record. It belongs
    /// to this record alone (FR-116 exact correlation): another record's
    /// checkout SHA, CI invocation or actor never qualifies it.
    pub observation: RunObservation,
}

/// The host's admitted observations of one retained run (FR-116 Inputs).
#[derive(Debug, Clone, Default)]
pub struct RunObservation {
    /// The CI checkout SHA of the `run-package` step that produced the record.
    pub performed_revision: Option<String>,
    /// The execution association of that step.
    pub execution: Option<ExecutionAssociation>,
    /// The signer admitted as the asserting actor for the record.
    pub actor: Option<ActorAdmission>,
}

/// An actor and the host's admission of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorAdmission {
    /// The signer mapped to a person through the signed-records feature.
    pub actor: String,
    /// The host's authority-admission reference.
    pub admission_ref: String,
    /// The obligation ids the actor is authorized to assert.
    pub authorized: Vec<String>,
}

/// The CI invocation an act ran in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiAssociation {
    /// The CI system.
    pub system: String,
    /// The project.
    pub project: String,
    /// The run.
    pub run: String,
    /// The attempt.
    pub attempt: String,
    /// The job.
    pub job: String,
    /// The `run-package` step's invocation.
    pub invocation: String,
}

/// How the act was executed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionAssociation {
    /// A CI invocation, the only association the analysis act admits.
    Ci(CiAssociation),
    /// A manual run, which the analysis act does not admit.
    Manual {
        /// The accountable attester.
        attester: String,
        /// Why CI is not applicable.
        reason: String,
    },
}

/// The observations that are not tied to one record (FR-116 Inputs).
#[derive(Debug, Clone, Default)]
pub struct HostObservations {
    /// The FR-093 decision for `quire-analyze` and the solvers for the positive
    /// use, when one exists. The qualifier assesses it and compares its
    /// accepted context with the report's producer.
    pub reliance: Option<TrustDecision>,
    /// The source HEAD selected for this qualification.
    pub selected_head: Option<String>,
}

/// One finding of the FR-032 audit about the obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditFinding {
    /// The finding kind.
    pub kind: String,
    /// The audit's own summary.
    pub summary: String,
}

/// One obligation's qualification question.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// The obligation id.
    pub obligation: &'a str,
    /// The obligation's declared method.
    pub declared_method: Option<&'a str>,
    /// Every retained run, in any order.
    pub runs: &'a [RetainedRun],
    /// The findings of the shared FR-032 audit about this obligation, run by
    /// the bridge over the same latest runs; absent when no audit ran.
    pub audit: Option<&'a [AuditFinding]>,
    /// The host's admitted observations.
    pub host: &'a HostObservations,
}

/// A positive decision for one retained entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// The retained entry.
    pub locator: RunLocator,
    /// The declared method, preserved.
    pub method: String,
    /// The asserting actor.
    pub actor: String,
    /// The evaluated premises and references the decision relies on.
    pub provenance: Vec<String>,
}

/// A non-qualified result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The category.
    pub cause: Cause,
    /// The premise, when one is the reason.
    pub premise: Option<Premise>,
    /// The retained entry, absent when none exists.
    pub locator: Option<RunLocator>,
    /// The cause's detail, naming the premise.
    pub detail: String,
}

/// The answer for one obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Qualification {
    /// Every premise holds for every bound entry.
    Qualified(Vec<Decision>),
    /// Nothing is counted; every applicable cause is retained.
    NotQualified(Vec<Refusal>),
}

/// Qualify one obligation.
///
/// All or nothing across the obligation's bound entries: one failing,
/// erroring, superseded or unqualified entry leaves the obligation uncounted
/// (FR-032, FR-153-AC-6).
#[must_use]
#[implements("FR-116")]
pub fn qualify(request: &Request<'_>) -> Qualification {
    let runs::Disposition {
        candidates,
        mut refusals,
    } = runs::dispositions(request.obligation, request.runs);
    if !candidates.is_empty() {
        match request.audit {
            None => refusals.push(Refusal {
                cause: Cause::Unevaluated,
                premise: Some(Premise::Audit),
                locator: None,
                detail: Premise::Audit.detail("no audit of the latest runs was supplied"),
            }),
            Some(findings) => refusals.extend(findings.iter().map(|finding| Refusal {
                cause: Cause::Unevaluated,
                premise: Some(Premise::Audit),
                locator: None,
                detail: Premise::Audit.detail(&format!("{}: {}", finding.kind, finding.summary)),
            })),
        }
    }
    let mut decisions = Vec::new();
    for candidate in &candidates {
        let mut provenance = vec![
            format!("producer-definition:{PRODUCER_DEFINITION}"),
            "credit:consistency-evidence-only".to_owned(),
            "basis:conditional-on-host-admitted-observations".to_owned(),
        ];
        if request.declared_method != Some(ANALYSIS_METHOD) {
            refusals.push(Refusal {
                cause: Cause::QualificationRejected,
                premise: Some(Premise::MethodActPairing),
                locator: Some(candidate.locator.clone()),
                detail: Premise::MethodActPairing.detail(&format!(
                    "the analysis act discharges {ANALYSIS_METHOD}; the obligation declares {}",
                    request.declared_method.unwrap_or("no method")
                )),
            });
        }
        if candidate
            .entry
            .trace_ids
            .as_deref()
            .is_some_and(|ids| ids.len() > 1)
        {
            refusals.push(Refusal {
                cause: Cause::Unevaluated,
                premise: Some(Premise::ActScope),
                locator: Some(candidate.locator.clone()),
                detail: Premise::ActScope
                    .detail("one report states more than one obligation, so it qualifies none"),
            });
        }
        premises::evaluate(
            &premises::Probe {
                obligation: request.obligation,
                host: request.host,
                run: candidate.run,
                entry: candidate.entry,
                locator: &candidate.locator,
            },
            &mut provenance,
            &mut refusals,
        );
        decisions.push(Decision {
            locator: candidate.locator.clone(),
            method: ANALYSIS_METHOD.to_owned(),
            actor: candidate
                .run
                .observation
                .actor
                .as_ref()
                .map(|admission| admission.actor.clone())
                .unwrap_or_default(),
            provenance,
        });
    }
    if refusals.is_empty() && !decisions.is_empty() {
        Qualification::Qualified(decisions)
    } else {
        // FR-116: target (one per call), then the record/entry locus, then the
        // cause category, with the premise and detail as stable tie-breakers.
        refusals.sort_by(|a, b| {
            (&a.locator, a.cause, a.premise, &a.detail)
                .cmp(&(&b.locator, b.cause, b.premise, &b.detail))
        });
        refusals.dedup();
        Qualification::NotQualified(refusals)
    }
}
