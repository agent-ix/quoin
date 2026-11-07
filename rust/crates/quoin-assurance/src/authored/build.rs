// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Computing the view: `build_authored_argument_view` (FR-047).
//!
//! Indexes the sufficiency decisions, walks the authored reasoning and
//! assumptions, and decides what is open and why. The shape it emits is in
//! [`super::view`]; the readers that admit a decision are in
//! [`super::parse`].

use std::collections::BTreeMap;

use crate::argument::{
    ArgumentError, ArgumentStatus, AssumptionStatus, AssuranceArgument, TopClaim,
    parse_assurance_argument, reject,
};
use crate::discharge::DischargeReport;

use super::parse::{
    current_decision, evaluate_challenge, instant_or_reject, parse_sufficiency_decision,
};
use super::view::{
    ArgumentSummary, AssumptionView, AuthoredArgumentView, BuildAuthoredArgumentRequest,
    ChallengeView, ChallengeViewStatus, CriterionView, DecisionState, EvidenceIndexEntry,
    EvidenceRefState, ReasoningView, SufficiencyDecision, TopClaimView, UnusedDecision,
    ViewSchemaVersion, ViewStatus,
};

/// Build the authored view at one stated instant.
///
/// # Errors
///
/// [`ArgumentError`] when the argument fails any predicate in
/// [`crate::argument`], when the `asOf` is not an instant, when a decision is
/// malformed, names an undeclared participant, claims an authority the
/// participant does not hold, or repeats a `(reasoningId, criterion)` pair,
/// or when the evidence index names one reference twice.
pub fn build_authored_argument_view(
    request: &BuildAuthoredArgumentRequest,
) -> Result<AuthoredArgumentView, ArgumentError> {
    let argument = parse_assurance_argument(&request.argument)?;
    let as_of = instant_or_reject("asOf", &request.as_of)?;

    let mut parsed_decisions = Vec::with_capacity(request.decisions.len());
    for value in &request.decisions {
        parsed_decisions.push(parse_sufficiency_decision(value)?);
    }
    let decisions = index_decisions(&argument, &parsed_decisions)?;

    let mut used: Vec<DecisionKey> = Vec::new();
    let reasoning = build_reasoning(&argument, &decisions, as_of, &mut used)?;
    let assumptions = build_assumptions(&argument, as_of)?;
    let mut challenges = Vec::with_capacity(argument.challenges.len());
    for challenge in &argument.challenges {
        challenges.push(evaluate_challenge(challenge, as_of)?);
    }

    let evidence_index = index_evidence(&request.evidence)?;
    let reasons = open_reasons(
        &argument,
        &reasoning,
        &assumptions,
        &challenges,
        request.discharge.as_ref(),
        &evidence_index,
    );
    let top_status = if reasons.is_empty() {
        ViewStatus::Supported
    } else {
        ViewStatus::Open
    };
    let TopClaim {
        id,
        statement,
        subject,
        ..
    } = argument.top_claim.clone();

    let unused_decisions = parsed_decisions
        .iter()
        .filter(|decision| !used.contains(&key_of(decision)))
        .map(|decision| UnusedDecision {
            reasoning_id: decision.reasoning_id.clone(),
            criterion: decision.criterion.clone(),
        })
        .collect();

    Ok(AuthoredArgumentView {
        schema_version: ViewSchemaVersion::V1,
        argument: ArgumentSummary {
            id: argument.id.clone(),
            title: argument.title.clone(),
            status: argument.status,
            owner: argument.owner.clone(),
            profile: argument.profile.clone(),
        },
        // The REQUEST's spelling, not the parse's. A view that re-formatted
        // the instant would make two runs over one authored document differ in
        // their own header.
        as_of: request.as_of.clone(),
        top_claim: TopClaimView {
            id,
            statement,
            subject,
            status: top_status,
            reasons,
        },
        reasoning,
        assumptions,
        participants: argument.participants.clone(),
        challenges,
        relationships: argument.relationships.clone(),
        discharge: request.discharge.clone(),
        unused_decisions,
    })
}

/// `decisionKey(reasoningId, criterion)`.
///
/// The retained source joins the two with a NUL into one string because a
/// JavaScript `Map` keys by identity otherwise. A tuple carries the same pair
/// without the separator, and so without the question of what a criterion
/// containing a NUL would collide with.
type DecisionKey = (String, String);

/// The pair a decision claims.
fn key_of(decision: &SufficiencyDecision) -> DecisionKey {
    (decision.reasoning_id.clone(), decision.criterion.clone())
}

/// Check every decision against the declared participants and index it.
///
/// The map is internal: nothing here is serialised, so its ordering is not
/// observable, and `unusedDecisions` is built from the supplied VECTOR so the
/// caller's order survives.
fn index_decisions(
    argument: &AssuranceArgument,
    parsed: &[SufficiencyDecision],
) -> Result<std::collections::BTreeMap<DecisionKey, SufficiencyDecision>, ArgumentError> {
    let mut decisions = std::collections::BTreeMap::new();
    for decision in parsed {
        let Some(participant) = argument
            .participants
            .iter()
            .find(|candidate| candidate.id == decision.decided_by)
        else {
            return reject(format!(
                "sufficiency decision maker {} is not a declared participant",
                decision.decided_by
            ));
        };
        // Authority is checked against the ARGUMENT's declaration rather than
        // taken from the decision: a decision that names its own authority is
        // the thing this check exists to refuse.
        if participant.authority != decision.authority {
            return reject(format!(
                "sufficiency decision authority for {} does not match the authored participant",
                decision.decided_by
            ));
        }
        let key = key_of(decision);
        if decisions.contains_key(&key) {
            return reject(format!(
                "duplicate sufficiency decision for {}: {}",
                decision.reasoning_id, decision.criterion
            ));
        }
        decisions.insert(key, decision.clone());
    }
    Ok(decisions)
}

/// Every authored criterion, decided or not, in authored order.
fn build_reasoning(
    argument: &AssuranceArgument,
    decisions: &std::collections::BTreeMap<DecisionKey, SufficiencyDecision>,
    as_of: i64,
    used: &mut Vec<DecisionKey>,
) -> Result<Vec<ReasoningView>, ArgumentError> {
    let mut reasoning = Vec::with_capacity(argument.reasoning.len());
    for reason in &argument.reasoning {
        let mut criteria = Vec::with_capacity(reason.sufficiency_criteria.len());
        for criterion in &reason.sufficiency_criteria {
            let key = (reason.id.clone(), criterion.clone());
            let Some(decision) = decisions.get(&key) else {
                criteria.push(CriterionView {
                    criterion: criterion.clone(),
                    status: ViewStatus::Open,
                    reason: Some("no sufficiency decision".to_owned()),
                    decision: None,
                });
                continue;
            };
            used.push(key);
            criteria.push(decide(criterion, decision, as_of)?);
        }
        let status = if criteria
            .iter()
            .all(|criterion| criterion.status == ViewStatus::Supported)
        {
            ViewStatus::Supported
        } else {
            ViewStatus::Open
        };
        reasoning.push(ReasoningView {
            id: reason.id.clone(),
            statement: reason.statement.clone(),
            supports: reason.supports.clone(),
            status,
            criteria,
        });
    }
    Ok(reasoning)
}

/// One criterion against the decision that named it.
///
/// The refused decision is carried into the view either way. A reader who saw
/// only `status` could not tell an expired decision from an absent one, and
/// the difference is exactly the work someone has to do next.
fn decide(
    criterion: &str,
    decision: &SufficiencyDecision,
    as_of: i64,
) -> Result<CriterionView, ArgumentError> {
    let reason = if let Some(current) = current_decision(decision, as_of)? {
        Some(current)
    } else if decision.state == DecisionState::Open {
        // `decision.rationale ?? "decision remains open"`.
        Some(
            decision
                .rationale
                .clone()
                .unwrap_or_else(|| "decision remains open".to_owned()),
        )
    } else {
        None
    };
    Ok(CriterionView {
        criterion: criterion.to_owned(),
        status: if reason.is_some() {
            ViewStatus::Open
        } else {
            ViewStatus::Supported
        },
        reason,
        decision: Some(decision.clone()),
    })
}

/// Every assumption, at the stated instant.
fn build_assumptions(
    argument: &AssuranceArgument,
    as_of: i64,
) -> Result<Vec<AssumptionView>, ArgumentError> {
    let mut assumptions = Vec::with_capacity(argument.assumptions.len());
    for assumption in &argument.assumptions {
        let reason = if assumption.status == AssumptionStatus::Accepted {
            // `<=`, not `<`: an assumption whose review falls exactly on the
            // evaluation instant is DUE, not current.
            if instant_or_reject("review_by", &assumption.review_by)? <= as_of {
                Some("assumption review is due".to_owned())
            } else {
                None
            }
        } else {
            Some(format!(
                "assumption is {}",
                assumption_status_text(assumption.status)
            ))
        };
        assumptions.push(AssumptionView {
            id: assumption.id.clone(),
            statement: assumption.statement.clone(),
            owner: assumption.owner.clone(),
            status: if reason.is_some() {
                ViewStatus::Open
            } else {
                ViewStatus::Supported
            },
            declared_status: assumption.status,
            review_by: assumption.review_by.clone(),
            reason,
        });
    }
    Ok(assumptions)
}

/// Index the caller-supplied evidence store snapshot by reference.
///
/// A repeated reference is refused, as `index_decisions` refuses a repeated
/// decision: with last-write-wins, `[{A, stale}, {A, resolved}]` would read as
/// backed, so the order of a list the caller never promised to order would
/// decide whether the claim stands.
fn index_evidence(
    entries: &[EvidenceIndexEntry],
) -> Result<BTreeMap<&str, EvidenceRefState>, ArgumentError> {
    let mut index = BTreeMap::new();
    for entry in entries {
        if index
            .insert(entry.reference.as_str(), entry.state)
            .is_some()
        {
            return reject(format!(
                "duplicate evidence index entry for {}",
                entry.reference
            ));
        }
    }
    Ok(index)
}

/// Why the top claim's OWN citation does not back it — empty when it does.
///
/// One reason per failing reference, in the claim's authored `evidence_refs`
/// order. Each reference is a separate gap, and stopping at the first would
/// let an earlier stale reference hide a later vacuous one until the first
/// was fixed and the view re-run.
fn claim_backing_reasons(
    top_claim: &TopClaim,
    evidence: &BTreeMap<&str, EvidenceRefState>,
) -> Vec<String> {
    if top_claim.evidence_refs.is_empty() {
        return vec!["the top claim has no evidence references".to_owned()];
    }
    top_claim
        .evidence_refs
        .iter()
        .filter_map(|reference| {
            let reason = match evidence.get(reference.as_str()) {
                None => "does not resolve in the evidence store",
                Some(EvidenceRefState::Stale) => "is stale",
                Some(EvidenceRefState::Vacuous) => "is vacuous",
                Some(EvidenceRefState::Resolved) => return None,
            };
            Some(format!(
                "the top claim's evidence reference {reference} {reason}"
            ))
        })
        .collect()
}

/// Why the top claim does not stand, in the retained order.
///
/// The order is the output. A reader takes the first line as the thing to deal
/// with, so argument status comes before the claim's own evidence backing,
/// evidence before criteria, criteria before assumptions, and the two
/// discharge reasons last.
fn open_reasons(
    argument: &AssuranceArgument,
    reasoning: &[ReasoningView],
    assumptions: &[AssumptionView],
    challenges: &[ChallengeView],
    discharge: Option<&DischargeReport>,
    evidence: &BTreeMap<&str, EvidenceRefState>,
) -> Vec<String> {
    let mut reasons: Vec<String> = Vec::new();
    if argument.status != ArgumentStatus::Active {
        reasons.push(format!(
            "argument status is {}",
            argument_status_text(argument.status)
        ));
    }
    reasons.extend(claim_backing_reasons(&argument.top_claim, evidence));
    if reasoning.iter().any(|item| item.status == ViewStatus::Open) {
        reasons.push("one or more sufficiency criteria are open".to_owned());
    }
    if assumptions
        .iter()
        .any(|item| item.status == ViewStatus::Open)
    {
        reasons.push("one or more assumptions are open, invalidated, or due for review".to_owned());
    }
    if challenges
        .iter()
        .any(|item| item.status == ChallengeViewStatus::Open)
    {
        reasons.push("one or more challenges are open or no longer current".to_owned());
    }
    if let Some(discharge) = discharge {
        if !discharge.binding.open.is_empty() {
            reasons.push("the clause discharge report contains open binding clauses".to_owned());
        }
        if !discharge.unresolved.is_empty() {
            reasons
                .push("the clause discharge report contains unresolved applicability".to_owned());
        }
    }
    reasons
}

/// The declared spellings, which are the JSON's and so are written out.
fn assumption_status_text(status: AssumptionStatus) -> &'static str {
    match status {
        AssumptionStatus::Open => "open",
        AssumptionStatus::Accepted => "accepted",
        AssumptionStatus::Invalidated => "invalidated",
    }
}

/// The same, for the argument's own lifecycle state.
fn argument_status_text(status: ArgumentStatus) -> &'static str {
    match status {
        ArgumentStatus::Proposed => "proposed",
        ArgumentStatus::Active => "active",
        ArgumentStatus::Retired => "retired",
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests;
