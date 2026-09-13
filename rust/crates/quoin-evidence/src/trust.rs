// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Use-specific evidence-producer trust decisions (FR-093).
//!
//! A decision is about **one bounded use**, never a global badge on a tool. The
//! validation below is the retained zod boundary restated: strict objects, a
//! non-empty `revalidateOn` that must name the five required triggers, and
//! `adapter` among them whenever the accepted context relies on one.
//!
//! # One reported divergence
//!
//! The refusal *sentence* is reproduced — `invalid trust decision: ` followed by
//! `path: complaint` clauses joined with `; ` — and every clause the retained
//! `superRefine` adds is reproduced verbatim. The clause text zod generates for
//! a plain type or length failure is zod's own prose and is not byte-copied
//! here; a consumer that matched on it was matching on a dependency's wording.

use crate::error::EvidenceError;
use crate::types::{
    ProducerContext, TrustAssessment, TrustDecision, TrustDecisionKind, TrustStatus, TrustTrigger,
};

/// The triggers every project must revalidate on, whatever else it selects.
///
/// `adapter` is deliberately absent: it is required only when the accepted
/// context names an adapter, because a project that transcribes nothing has no
/// adapter to revalidate against.
pub const REQUIRED_TRIGGERS: [TrustTrigger; 5] = [
    TrustTrigger::ProducerVersion,
    TrustTrigger::Configuration,
    TrustTrigger::ValidationCorpus,
    TrustTrigger::InputContract,
    TrustTrigger::Environment,
];

/// Check one decision against the FR-093 boundary.
///
/// # Errors
///
/// [`EvidenceError::InvalidRecord`] carrying every clause that failed, joined
/// with `; ` — all of them, not the first, because a decision with three
/// problems should take one round trip to fix and not three.
pub fn validate_trust_decision(decision: &TrustDecision) -> Result<(), EvidenceError> {
    let mut clauses: Vec<String> = Vec::new();

    if decision.schema_version != crate::types::STORE_SCHEMA_VERSION {
        clauses.push(format!(
            "schemaVersion: must be {}",
            crate::types::STORE_SCHEMA_VERSION
        ));
    }
    non_empty(&mut clauses, "use.id", &decision.r#use.id);
    non_empty(
        &mut clauses,
        "use.intendedFunction",
        &decision.r#use.intended_function,
    );
    if decision.r#use.permitted_decisions.is_empty() {
        clauses.push("use.permittedDecisions: must not be empty".to_owned());
    }
    for value in &decision.r#use.permitted_decisions {
        non_empty(&mut clauses, "use.permittedDecisions", value);
    }
    if has_duplicates(&decision.r#use.permitted_decisions) {
        clauses.push("use.permittedDecisions: contains duplicate decisions".to_owned());
    }

    context_clauses(&mut clauses, "acceptedContext", &decision.accepted_context);
    if let Some(observed) = &decision.observed_context {
        context_clauses(&mut clauses, "observedContext", observed);
    }

    if decision.revalidate_on.is_empty() {
        clauses.push("revalidateOn: must not be empty".to_owned());
    }
    if has_duplicates(&decision.revalidate_on) {
        clauses.push("revalidateOn: contains duplicate triggers".to_owned());
    }
    for required in REQUIRED_TRIGGERS {
        if !decision.revalidate_on.contains(&required) {
            clauses.push(format!("revalidateOn: must include {required}"));
        }
    }
    if decision.accepted_context.adapter.is_some()
        && !decision.revalidate_on.contains(&TrustTrigger::Adapter)
    {
        clauses
            .push("revalidateOn: must include adapter when an adapter is relied upon".to_owned());
    }

    if decision.validation_evidence.is_empty() {
        clauses.push("validationEvidence: must not be empty".to_owned());
    }
    for (index, evidence) in decision.validation_evidence.iter().enumerate() {
        non_empty(&mut clauses, "validationEvidence.id", &evidence.id);
        if evidence.digest.is_none() && evidence.reference.is_none() {
            clauses.push(format!(
                "validationEvidence.{index}: validation evidence needs digest or reference"
            ));
        }
        if let Some(digest) = &evidence.digest
            && !is_algorithm_digest(digest)
        {
            clauses.push("validationEvidence.digest: must be <algorithm>:<hex>".to_owned());
        }
        if let Some(reference) = &evidence.reference {
            non_empty(&mut clauses, "validationEvidence.reference", reference);
        }
    }

    for limitation in &decision.limitations {
        non_empty(&mut clauses, "limitations", limitation);
    }
    non_empty(&mut clauses, "owner", &decision.owner);
    if !is_instant(&decision.decided_at) {
        clauses.push("decidedAt: must be an ISO-8601 timestamp".to_owned());
    }

    if clauses.is_empty() {
        Ok(())
    } else {
        Err(EvidenceError::invalid("trust decision", clauses.join("; ")))
    }
}

/// Compare a decision's accepted context against what is observed.
///
/// # Errors
///
/// As [`validate_trust_decision`]: an assessment is never produced for a
/// decision the boundary refuses, so an invalid record cannot be rendered as
/// `accepted`.
pub fn assess_trust(decision: &TrustDecision) -> Result<TrustAssessment, EvidenceError> {
    validate_trust_decision(decision)?;
    let mut permitted = decision.r#use.permitted_decisions.clone();
    permitted.sort();
    let mut assessment = TrustAssessment {
        id: decision.id.clone(),
        use_id: decision.r#use.id.clone(),
        producer: decision.accepted_context.name.clone(),
        status: TrustStatus::NotAccepted,
        permitted_decisions: permitted,
        limitations: decision.limitations.clone(),
        triggered_by: Vec::new(),
        owner: decision.owner.clone(),
    };
    if decision.decision == TrustDecisionKind::NotReliedUpon {
        return Ok(assessment);
    }
    let Some(observed) = &decision.observed_context else {
        // Absence is never acceptance: nothing was compared, so nothing is
        // accepted, and the reader is told which of the two it is.
        assessment.status = TrustStatus::Unobserved;
        return Ok(assessment);
    };
    let triggered: Vec<TrustTrigger> = decision
        .revalidate_on
        .iter()
        .copied()
        .filter(|trigger| differs(*trigger, &decision.accepted_context, observed))
        .collect();
    if triggered.is_empty() {
        assessment.status = if decision.limitations.is_empty() {
            TrustStatus::Accepted
        } else {
            TrustStatus::AcceptedWithLimitations
        };
    } else {
        assessment.status = TrustStatus::Invalidated;
        assessment.triggered_by = triggered;
    }
    Ok(assessment)
}

/// Whether one trigger's fact changed between the accepted and observed
/// contexts.
fn differs(trigger: TrustTrigger, accepted: &ProducerContext, observed: &ProducerContext) -> bool {
    match trigger {
        TrustTrigger::ProducerVersion => {
            accepted.name != observed.name || accepted.version != observed.version
        }
        TrustTrigger::Configuration => {
            accepted.configuration_digest != observed.configuration_digest
        }
        // Structural equality, matching the retained `JSON.stringify` compare:
        // an absent adapter on one side and a present one on the other is a
        // difference, which is the case the trigger exists for.
        TrustTrigger::Adapter => accepted.adapter != observed.adapter,
        TrustTrigger::ValidationCorpus => {
            accepted.validation_corpus_digest != observed.validation_corpus_digest
        }
        TrustTrigger::InputContract => accepted.input_contract != observed.input_contract,
        TrustTrigger::Environment => accepted.environment != observed.environment,
    }
}

fn context_clauses(clauses: &mut Vec<String>, path: &str, context: &ProducerContext) {
    non_empty(clauses, &format!("{path}.name"), &context.name);
    non_empty(clauses, &format!("{path}.version"), &context.version);
    non_empty(
        clauses,
        &format!("{path}.configurationDigest"),
        &context.configuration_digest,
    );
    non_empty(
        clauses,
        &format!("{path}.validationCorpusDigest"),
        &context.validation_corpus_digest,
    );
    non_empty(
        clauses,
        &format!("{path}.inputContract"),
        &context.input_contract,
    );
    non_empty(
        clauses,
        &format!("{path}.environment"),
        &context.environment,
    );
    if let Some(adapter) = &context.adapter {
        non_empty(clauses, &format!("{path}.adapter.name"), &adapter.name);
        non_empty(
            clauses,
            &format!("{path}.adapter.version"),
            &adapter.version,
        );
    }
}

fn non_empty(clauses: &mut Vec<String>, path: &str, value: &str) {
    if value.is_empty() {
        clauses.push(format!("{path}: must not be empty"));
    }
}

fn has_duplicates<T: Clone + Ord>(values: &[T]) -> bool {
    let mut sorted = values.to_vec();
    sorted.sort();
    let before = sorted.len();
    sorted.dedup();
    sorted.len() != before
}

/// `<algorithm>:<hex>`, the retained `/^[a-z0-9]+:[a-f0-9]+$/`.
fn is_algorithm_digest(value: &str) -> bool {
    let Some((algorithm, hex)) = value.split_once(':') else {
        return false;
    };
    !algorithm.is_empty()
        && algorithm
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && !hex.is_empty()
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// `Date.parse`-able, restated as the ISO-8601 forms quoin actually writes.
///
/// The retained check is `!Number.isNaN(Date.parse(value))`, which accepts a
/// wide set of legacy spellings no producer emits. Reproducing V8's date parser
/// would be a second implementation of a browser quirk; this accepts the
/// `YYYY-MM-DD` date and the full instant, which is what every record on disk
/// carries. Reported as a divergence rather than silently reproduced.
fn is_instant(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 10 {
        return false;
    }
    let digits_at = |index: usize, count: usize| {
        (index..index + count).all(|position| bytes.get(position).is_some_and(u8::is_ascii_digit))
    };
    let dash_at = |index: usize| bytes.get(index) == Some(&b'-');
    if !(digits_at(0, 4) && dash_at(4) && digits_at(5, 2) && dash_at(7) && digits_at(8, 2)) {
        return false;
    }
    if bytes.len() == 10 {
        return true;
    }
    bytes.get(10) == Some(&b'T')
        && digits_at(11, 2)
        && bytes.get(13) == Some(&b':')
        && digits_at(14, 2)
        && bytes.get(16) == Some(&b':')
        && digits_at(17, 2)
}
