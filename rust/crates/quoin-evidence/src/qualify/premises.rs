// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The FR-116 premises of the analysis act (FR-184), each read from a supplied
//! observation. Nothing here runs a solver, reads a clock or touches a file
//! (FR-184-CON-2).

use std::path::Path;

use serde_json::Value;

use super::{
    Cause, ExecutionAssociation, HostObservations, POSITIVE_USE_DECISION, Premise, Refusal,
    RunLocator,
};
use crate::types::{RunEntry, TrustStatus};

/// The two engines a `quire.differential-report/v2` agreement names.
const ENGINES: [&str; 2] = ["z3", "cvc5"];

fn refusal(cause: Cause, premise: Premise, locator: &RunLocator, reason: &str) -> Refusal {
    Refusal {
        cause,
        premise: Some(premise),
        locator: Some(locator.clone()),
        detail: premise.detail(reason),
    }
}

/// The host-wide premises, each with the reference a positive decision retains.
pub(super) fn host_premises(
    host: &HostObservations,
    locator: &RunLocator,
    provenance: &mut Vec<String>,
    refusals: &mut Vec<Refusal>,
) {
    reliance(host, locator, provenance, refusals);
    actor(host, locator, provenance, refusals);
    revision(host, locator, provenance, refusals);
    association(host, locator, provenance, refusals);
}

fn reliance(
    host: &HostObservations,
    locator: &RunLocator,
    provenance: &mut Vec<String>,
    refusals: &mut Vec<Refusal>,
) {
    match &host.reliance {
        None => refusals.push(refusal(
            Cause::Unevaluated,
            Premise::BoundedReliance,
            locator,
            "no FR-093 assessment of quire-analyze and the solvers exists for this positive use",
        )),
        Some(assessment) => match assessment.status {
            TrustStatus::Accepted | TrustStatus::AcceptedWithLimitations
                if assessment
                    .permitted_decisions
                    .iter()
                    .any(|decision| decision == POSITIVE_USE_DECISION) =>
            {
                provenance.push(format!(
                    "reliance:{}:{}",
                    assessment.id,
                    assessment.status.as_str()
                ));
            }
            TrustStatus::Accepted | TrustStatus::AcceptedWithLimitations => {
                refusals.push(refusal(
                    Cause::QualificationRejected,
                    Premise::BoundedReliance,
                    locator,
                    &format!("the assessed use does not permit {POSITIVE_USE_DECISION}"),
                ));
            }
            TrustStatus::Invalidated => refusals.push(refusal(
                Cause::Stale,
                Premise::BoundedReliance,
                locator,
                "the assessment is invalidated by a changed producer context",
            )),
            TrustStatus::NotAccepted => refusals.push(refusal(
                Cause::QualificationRejected,
                Premise::BoundedReliance,
                locator,
                "the producer is not relied upon for this use",
            )),
            TrustStatus::Unobserved => refusals.push(refusal(
                Cause::Unevaluated,
                Premise::BoundedReliance,
                locator,
                "the assessment has no observed context to compare",
            )),
        },
    }
}

fn actor(
    host: &HostObservations,
    locator: &RunLocator,
    provenance: &mut Vec<String>,
    refusals: &mut Vec<Refusal>,
) {
    match host.actor.as_deref().filter(|actor| !actor.is_empty()) {
        Some(actor) => provenance.push(format!("actor-authority:{actor}")),
        None => refusals.push(refusal(
            Cause::Unevaluated,
            Premise::ActorAuthority,
            locator,
            "no signer admitted for the asserting actor",
        )),
    }
}

fn revision(
    host: &HostObservations,
    locator: &RunLocator,
    provenance: &mut Vec<String>,
    refusals: &mut Vec<Refusal>,
) {
    let performed = host
        .performed_revision
        .as_deref()
        .filter(|sha| !sha.is_empty());
    let head = host.selected_head.as_deref().filter(|sha| !sha.is_empty());
    match (performed, head) {
        (None, _) => refusals.push(refusal(
            Cause::Unevaluated,
            Premise::PerformedRevision,
            locator,
            "the run-package step has no CI checkout SHA",
        )),
        (Some(_), None) => refusals.push(refusal(
            Cause::Unevaluated,
            Premise::PerformedRevision,
            locator,
            "no selected source HEAD was supplied to compare with",
        )),
        (Some(performed), Some(head)) if performed != head => refusals.push(refusal(
            Cause::WrongRevision,
            Premise::PerformedRevision,
            locator,
            &format!("performed revision {performed} is not the selected HEAD {head}"),
        )),
        (Some(performed), Some(head)) => {
            provenance.push(format!("performed-revision:{performed}"));
            provenance.push(format!("selected-head:{head}"));
        }
    }
}

fn association(
    host: &HostObservations,
    locator: &RunLocator,
    provenance: &mut Vec<String>,
    refusals: &mut Vec<Refusal>,
) {
    match &host.execution {
        None => refusals.push(refusal(
            Cause::Unevaluated,
            Premise::ExecutionAssociation,
            locator,
            "no execution association supplied",
        )),
        Some(ExecutionAssociation::Manual { .. }) => refusals.push(refusal(
            Cause::QualificationRejected,
            Premise::ExecutionAssociation,
            locator,
            "the analysis act admits a CI association only; a manual run is not admitted",
        )),
        Some(ExecutionAssociation::Ci(ci)) => {
            let parts = [
                &ci.system,
                &ci.project,
                &ci.run,
                &ci.attempt,
                &ci.job,
                &ci.invocation,
            ];
            if parts.iter().any(|part| part.is_empty()) {
                refusals.push(refusal(
                    Cause::Unevaluated,
                    Premise::ExecutionAssociation,
                    locator,
                    "the CI association omits its system, project, run, attempt, job or invocation",
                ));
            } else {
                provenance.push(format!(
                    "execution:ci:{}/{}/{}/{}/{}/{}",
                    ci.system, ci.project, ci.run, ci.attempt, ci.job, ci.invocation
                ));
            }
        }
    }
}

fn text<'a>(value: &'a Value, member: &str) -> Option<&'a str> {
    value
        .get(member)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}

/// What the report retains of one engine, or why it is not an observation of a
/// solver started through `producer-execute`.
fn engine_fact(report: &Value, engine: &str) -> Result<String, String> {
    let record = report
        .get("engines")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|record| text(record, "engine") == Some(engine))
        .ok_or_else(|| format!("no {engine} engine record"))?;
    let executable = text(record, "configuredExecutable")
        .filter(|path| Path::new(path).is_absolute())
        .ok_or_else(|| format!("{engine} has no absolute configuredExecutable"))?;
    let identity = record.get("identity");
    let path = identity.and_then(|identity| text(identity, "path"));
    let version = identity.and_then(|identity| text(identity, "version"));
    let (Some(_path), Some(version)) = (path, version) else {
        return Err(format!(
            "{engine} has no observed identity path and version"
        ));
    };
    if text(record, "status").is_none() || text(record, "outcome").is_none() {
        return Err(format!("{engine} has no status and outcome"));
    }
    Ok(format!("engine:{engine}:{version}:{executable}"))
}

/// The tool and configuration premise, and the report's own agreement with the
/// passing entry it is supplied for.
pub(super) fn report_premises(
    report: Option<&Value>,
    entry: &RunEntry,
    locator: &RunLocator,
    provenance: &mut Vec<String>,
    refusals: &mut Vec<Refusal>,
) {
    let tool = Premise::ToolAndConfiguration;
    let Some(report) = report else {
        refusals.push(refusal(
            Cause::Unevaluated,
            tool,
            locator,
            "no report observation supplied for the record",
        ));
        return;
    };
    let symbol = match (text(report, "analysisKind"), text(report, "requestDigest")) {
        (Some(kind), Some(digest)) => format!("{kind}::{digest}"),
        _ => String::new(),
    };
    if symbol != entry.symbol.as_str() {
        refusals.push(refusal(
            Cause::Unevaluated,
            tool,
            locator,
            "the supplied report does not correspond to the entry",
        ));
        return;
    }
    let differential = report.get("differential");
    let agreed = differential.and_then(|differential| text(differential, "disposition"))
        == Some("agreement")
        && differential.and_then(|differential| text(differential, "agreedStatus"))
            == Some("satisfied");
    if !agreed {
        refusals.push(Refusal {
            cause: Cause::RecordConflict,
            premise: Some(tool),
            locator: Some(locator.clone()),
            detail: tool.detail("the entry passed but the report is not a satisfied agreement"),
        });
        return;
    }
    let producer = report.get("producer");
    let (Some(name), Some(version)) = (
        producer.and_then(|producer| text(producer, "name")),
        producer.and_then(|producer| text(producer, "version")),
    ) else {
        refusals.push(refusal(
            Cause::Unevaluated,
            tool,
            locator,
            "the report has no producer name and version",
        ));
        return;
    };
    let mut facts = vec![format!("tool:{name}:{version}")];
    for engine in ENGINES {
        match engine_fact(report, engine) {
            Ok(fact) => facts.push(fact),
            Err(reason) => refusals.push(refusal(Cause::Unevaluated, tool, locator, &reason)),
        }
    }
    if facts.len() == ENGINES.len() + 1 {
        provenance.extend(facts);
    }
}
