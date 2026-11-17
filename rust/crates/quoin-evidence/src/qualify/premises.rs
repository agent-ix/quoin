// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The FR-116 premises of the analysis act (FR-184), each read from a supplied
//! observation. Nothing here runs a solver, reads a clock or touches a file
//! (FR-184-CON-2).

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

use super::{
    Cause, ExecutionAssociation, HostObservations, POSITIVE_USE_DECISION, Premise, Refusal,
    RetainedRun, RunLocator,
};
use crate::paths::{RUNS_DIR, record_path};
use crate::trust::assess_trust;
use crate::types::TrustStatus;

/// The two engines a `filament.differential-report/v2` agreement names.
const ENGINES: [&str; 2] = ["z3", "cvc5"];

/// What one candidate's premises are evaluated over. Every observation read is
/// the run's own, so none can belong to another record (FR-116).
pub(super) struct Probe<'a> {
    pub(super) obligation: &'a str,
    pub(super) host: &'a HostObservations,
    pub(super) run: &'a RetainedRun,
    pub(super) locator: &'a RunLocator,
}

fn refusal(cause: Cause, premise: Premise, locator: &RunLocator, reason: &str) -> Refusal {
    Refusal {
        cause,
        premise: Some(premise),
        locator: Some(locator.clone()),
        detail: premise.detail(reason),
    }
}

/// Evaluate every premise of one passing candidate.
pub(super) fn evaluate(
    probe: &Probe<'_>,
    provenance: &mut Vec<String>,
    refusals: &mut Vec<Refusal>,
) {
    let producer = report_premises(probe, provenance, refusals);
    reliance(probe, producer, provenance, refusals);
    actor(probe, provenance, refusals);
    revision(probe, provenance, refusals);
    association(probe, provenance, refusals);
}

/// The FR-093 premise: a decision for this exact producer and positive use.
///
/// A decision that states any limitation is `unevaluated`: limitations are free
/// text and cannot be shown not to exclude the target or the act. It stays so
/// until an owner ruling or a structured limitation scope says otherwise.
fn reliance(
    probe: &Probe<'_>,
    producer: Option<(&str, &str)>,
    provenance: &mut Vec<String>,
    refusals: &mut Vec<Refusal>,
) {
    let locator = probe.locator;
    let mut refuse = |cause, reason: &str| {
        refusals.push(refusal(cause, Premise::BoundedReliance, locator, reason));
    };
    let Some(decision) = &probe.host.reliance else {
        refuse(
            Cause::Unevaluated,
            "no FR-093 assessment of filament-analyze and the solvers exists for this positive use",
        );
        return;
    };
    let assessment = match assess_trust(decision) {
        Ok(assessment) => assessment,
        Err(error) => {
            refuse(
                Cause::Unevaluated,
                &format!("the decision is invalid: {error}"),
            );
            return;
        }
    };
    match assessment.status {
        TrustStatus::Invalidated => {
            refuse(
                Cause::Stale,
                "the assessment is invalidated by a changed producer context",
            );
            return;
        }
        TrustStatus::NotAccepted => {
            refuse(
                Cause::QualificationRejected,
                "the producer is not relied upon for this use",
            );
            return;
        }
        TrustStatus::Unobserved => {
            refuse(
                Cause::Unevaluated,
                "the assessment has no observed context to compare",
            );
            return;
        }
        TrustStatus::Accepted | TrustStatus::AcceptedWithLimitations => {}
    }
    if !assessment
        .permitted_decisions
        .iter()
        .any(|permitted| permitted == POSITIVE_USE_DECISION)
    {
        refuse(
            Cause::QualificationRejected,
            &format!("the assessed use does not permit {POSITIVE_USE_DECISION}"),
        );
        return;
    }
    let accepted = &decision.accepted_context;
    match producer {
        None => {
            refuse(
                Cause::Unevaluated,
                "the report names no producer to compare with the assessed context",
            );
            return;
        }
        Some((name, version)) if name != accepted.name || version != accepted.version => {
            refuse(
                Cause::QualificationRejected,
                &format!(
                    "the assessment is for {} {}, not the report's producer {name} {version}",
                    accepted.name, accepted.version
                ),
            );
            return;
        }
        Some(_) => {}
    }
    // Fail closed (FR-116, FR-116-AC-6): limitations are free text, so a
    // limitation cannot be shown not to exclude this target or this act. Any
    // limitation leaves the use unevaluated, with the text kept in the detail.
    if !assessment.limitations.is_empty() {
        refuse(
            Cause::Unevaluated,
            &format!(
                "the decision states limitations that cannot be shown not to exclude this use: {}",
                assessment.limitations.join("; ")
            ),
        );
        return;
    }
    provenance.push(format!(
        "reliance:{}:{}",
        assessment.id,
        assessment.status.as_str()
    ));
}

fn actor(probe: &Probe<'_>, provenance: &mut Vec<String>, refusals: &mut Vec<Refusal>) {
    let locator = probe.locator;
    let mut refuse = |cause, reason: &str| {
        refusals.push(refusal(cause, Premise::ActorAuthority, locator, reason));
    };
    let Some(admission) = &probe.run.observation.actor else {
        if let Some(reason) = &probe.run.observation.actor_rejection {
            refuse(Cause::QualificationRejected, reason);
        } else {
            refuse(Cause::Unevaluated, "no signer admitted for the record");
        }
        return;
    };
    if admission.actor.is_empty() || admission.admission_ref.is_empty() {
        refuse(
            Cause::Unevaluated,
            "the actor has no host authority-admission reference",
        );
    } else if !admission
        .authorized
        .iter()
        .any(|authorized| authorized == probe.obligation)
    {
        refuse(
            Cause::QualificationRejected,
            &format!("the admitted authority does not cover {}", probe.obligation),
        );
    } else {
        provenance.push(format!(
            "actor-authority:{}:{}",
            admission.actor, admission.admission_ref
        ));
    }
}

fn revision(probe: &Probe<'_>, provenance: &mut Vec<String>, refusals: &mut Vec<Refusal>) {
    let locator = probe.locator;
    if probe.host.selected_head_conflict {
        refusals.push(refusal(
            Cause::RecordConflict,
            Premise::PerformedRevision,
            locator,
            "run observations supply conflicting selected source HEAD values",
        ));
        return;
    }
    let performed = probe
        .run
        .observation
        .performed_revision
        .as_deref()
        .filter(|sha| !sha.is_empty());
    let head = probe
        .host
        .selected_head
        .as_deref()
        .filter(|sha| !sha.is_empty());
    match (performed, head) {
        (None, _) => refusals.push(refusal(
            Cause::Unevaluated,
            Premise::PerformedRevision,
            locator,
            "the run-package step has no CI checkout SHA for this record",
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

fn association(probe: &Probe<'_>, provenance: &mut Vec<String>, refusals: &mut Vec<Refusal>) {
    let locator = probe.locator;
    match &probe.run.observation.execution {
        None => refusals.push(refusal(
            Cause::Unevaluated,
            Premise::ExecutionAssociation,
            locator,
            "no execution association supplied for this record",
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
    let executable = text(record, "configured_executable")
        .filter(|path| Path::new(path).is_absolute())
        .ok_or_else(|| format!("{engine} has no absolute configured executable"))?;
    let path = text(record, "identity_path");
    let version = text(record, "identity_version");
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
fn report_premises<'a>(
    probe: &Probe<'a>,
    provenance: &mut Vec<String>,
    refusals: &mut Vec<Refusal>,
) -> Option<(&'a str, &'a str)> {
    let locator = probe.locator;
    let tool = Premise::ToolAndConfiguration;
    let run: &'a RetainedRun = probe.run;
    let Some(report) = run.report.as_ref() else {
        refusals.push(refusal(
            Cause::Unevaluated,
            tool,
            locator,
            "no report observation supplied for the record",
        ));
        return None;
    };
    if run.file != record_path(RUNS_DIR, &run.record.suite, &run.record.timestamp)
        || !valid_projection(report, &run.record)
    {
        refusals.push(refusal(
            Cause::RecordConflict,
            tool,
            locator,
            "the supplied projection does not match the retained run",
        ));
        return None;
    }
    let expected_kind = run.observation.run_kind.as_deref().unwrap_or("analysis");
    let projected_kind = text(report, "run_kind").unwrap_or("analysis");
    if expected_kind == "formal_rule"
        && (projected_kind != "formal_rule" || !formal_rule_selection_is_approved(report))
    {
        refusals.push(refusal(
            Cause::QualificationRejected,
            Premise::ApprovedFormalRules,
            locator,
            "draft_rule_in_judgement: the selected claim set is missing, mismatched or includes a non-approved formal rule",
        ));
        return None;
    }
    if expected_kind == "analysis" && projected_kind != "analysis" {
        refusals.push(refusal(
            Cause::RecordConflict,
            Premise::ToolAndConfiguration,
            locator,
            "the host's expected run_kind differs from the retained projection",
        ));
        return None;
    }
    let agreed = text(report, "disposition") == Some("agreement")
        && text(report, "agreed_status") == Some("satisfied");
    if !agreed {
        refusals.push(Refusal {
            cause: Cause::RecordConflict,
            premise: Some(tool),
            locator: Some(locator.clone()),
            detail: tool.detail("the entry passed but the report is not a satisfied agreement"),
        });
        return None;
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
        return None;
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
    Some((name, version))
}

/// Validate the exact allowlisted projection and its one-to-one run mapping.
fn valid_projection(report: &Value, run: &crate::types::RunRecord) -> bool {
    let Some(root) = report.as_object() else {
        return false;
    };
    if !valid_projection_header(root)
        || text(report, "protocol") != Some("quoin.analysis-report-projection/v1")
        || !matches!(
            text(report, "disposition"),
            Some("agreement" | "inconclusive" | "unavailable" | "disagreement")
        )
    {
        return false;
    }
    if root.contains_key("selection") && !root.get("selection").is_some_and(valid_selection) {
        return false;
    }
    let Some(producer) = root.get("producer").and_then(Value::as_object) else {
        return false;
    };
    if producer.len() != 2
        || text(&Value::Object(producer.clone()), "name").is_none()
        || text(&Value::Object(producer.clone()), "version").is_none()
    {
        return false;
    }
    let disposition = text(report, "disposition");
    let agreed = report.get("agreed_status");
    match (disposition, agreed.and_then(Value::as_str), agreed) {
        (Some("agreement"), Some("satisfied" | "refuted"), _)
        | (Some("inconclusive" | "unavailable" | "disagreement"), None, Some(Value::Null)) => {}
        _ => return false,
    }
    let Some(engines) = report.get("engines").and_then(Value::as_array) else {
        return false;
    };
    if engines.len() != 2 {
        return false;
    }
    for (index, engine) in engines.iter().enumerate() {
        let Some(object) = engine.as_object() else {
            return false;
        };
        let expected_name = if index == 0 { "z3" } else { "cvc5" };
        if text(engine, "engine") != Some(expected_name)
            || text(engine, "configured_executable").is_none()
            || text(engine, "status").is_none()
            || text(engine, "outcome").is_none()
            || object.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "engine"
                        | "configured_executable"
                        | "identity_path"
                        | "identity_version"
                        | "status"
                        | "outcome"
                )
            })
            || ["identity_path", "identity_version"]
                .iter()
                .any(|key| object.get(*key).is_some_and(|value| !value.is_string()))
        {
            return false;
        }
    }
    match disposition {
        Some("disagreement") => run.entries.is_empty(),
        Some("agreement") => run.entries.first().is_some_and(|entry| {
            run.entries.len() == 1
                && entry.outcome
                    == match agreed.and_then(Value::as_str) {
                        Some("satisfied") => crate::types::Outcome::Pass,
                        Some("refuted") => crate::types::Outcome::Fail,
                        _ => return false,
                    }
        }),
        Some("inconclusive" | "unavailable") => run.entries.first().is_some_and(|entry| {
            run.entries.len() == 1 && entry.outcome == crate::types::Outcome::Error
        }),
        _ => false,
    }
}

fn valid_projection_header(root: &serde_json::Map<String, Value>) -> bool {
    if !(5..=8).contains(&root.len())
        || root.keys().any(|key| {
            !matches!(
                key.as_str(),
                "protocol"
                    | "disposition"
                    | "agreed_status"
                    | "selection"
                    | "run_kind"
                    | "formal_rule_selection"
                    | "producer"
                    | "engines"
            )
        })
    {
        return false;
    }
    let run_kind = root.get("run_kind").and_then(Value::as_str);
    if root.contains_key("run_kind") && !matches!(run_kind, Some("analysis" | "formal_rule")) {
        return false;
    }
    let has_formal_selection = root.contains_key("formal_rule_selection");
    !matches!(
        (run_kind, has_formal_selection),
        (Some("analysis") | None, true)
    )
}

/// A selected report has a non-empty, duplicate-free partition of its claims.
fn valid_selection(selection: &Value) -> bool {
    let Some(object) = selection.as_object() else {
        return false;
    };
    if object.len() != 2 {
        return false;
    }
    let Some(selected) = object.get("selected").and_then(Value::as_array) else {
        return false;
    };
    let Some(excluded) = object.get("excluded").and_then(Value::as_array) else {
        return false;
    };
    if selected.is_empty() {
        return false;
    }
    let Some(selected_ids) = selection_ids(selected) else {
        return false;
    };
    let Some(excluded_ids) = selection_ids(excluded) else {
        return false;
    };
    selected_ids.is_disjoint(&excluded_ids)
}

fn selection_ids(items: &[Value]) -> Option<BTreeSet<&str>> {
    let mut values = BTreeSet::new();
    for item in items {
        let value = item.as_str().filter(|value| !value.is_empty())?;
        if !values.insert(value) {
            return None;
        }
    }
    Some(values)
}

fn formal_rule_selection_is_approved(report: &Value) -> bool {
    let Some(mapping) = report.get("formal_rule_selection") else {
        return false;
    };
    let Some(claims) = mapping.get("claims").and_then(Value::as_array) else {
        return false;
    };
    valid_formal_rule_selection(Some(mapping), report.get("selection"))
        && claims
            .iter()
            .all(|claim| claim.get("state").and_then(Value::as_str) == Some("approved"))
}

fn valid_formal_rule_selection(mapping: Option<&Value>, selection: Option<&Value>) -> bool {
    let Some(mapping) = mapping.and_then(Value::as_object) else {
        return false;
    };
    if mapping.len() != 1 {
        return false;
    }
    let Some(claims) = mapping.get("claims").and_then(Value::as_array) else {
        return false;
    };
    let Some(selection) = selection.and_then(Value::as_object) else {
        return false;
    };
    let Some(selected) = selection.get("selected").and_then(Value::as_array) else {
        return false;
    };
    if selected.len() != claims.len() {
        return false;
    }
    let mut symbols = BTreeSet::new();
    for (selected_id, claim) in selected.iter().zip(claims) {
        let Some(selected_id) = selected_id.as_str() else {
            return false;
        };
        let Some(claim) = claim.as_object().filter(|claim| claim.len() == 4) else {
            return false;
        };
        if claim.get("claim_id").and_then(Value::as_str) != Some(selected_id) {
            return false;
        }
        let Some(symbol) = claim.get("symbol").and_then(Value::as_str) else {
            return false;
        };
        if !symbol.starts_with("qsl:") || !symbols.insert(symbol) {
            return false;
        }
        if !matches!(
            claim.get("state").and_then(Value::as_str),
            Some("approved" | "draft")
        ) || !valid_source(claim.get("source"))
        {
            return false;
        }
    }
    true
}

fn valid_source(source: Option<&Value>) -> bool {
    let Some(source) = source.and_then(Value::as_object) else {
        return false;
    };
    if source.len() != 4
        || source.get("document").and_then(Value::as_str).is_none()
        || source.get("revision").and_then(Value::as_u64).is_none()
    {
        return false;
    }
    ["start", "end"].iter().all(|point| {
        let Some(point) = source.get(*point).and_then(Value::as_object) else {
            return false;
        };
        point.len() == 3
            && ["byte", "column", "line"]
                .iter()
                .all(|member| point.get(*member).and_then(Value::as_u64).is_some())
    })
}
