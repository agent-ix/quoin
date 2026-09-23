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
/// let an earlier stale reference hide a later suspect one until the first
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
                Some(EvidenceRefState::Suspect) => "is suspect",
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
mod tests {
    use super::super::fixtures::{
        AS_OF, CLAIM_EVIDENCE_REF, argument, build, decision, json, request, request_with_evidence,
        resolved_evidence, with,
    };
    use super::super::view::{EvidenceIndexEntry, EvidenceRefState, ViewStatus};
    use super::build_authored_argument_view;
    use crate::argument::parse_assurance_argument;

    /// Trace: FR-047-AC-1
    /// Provenance: quoin#445, quoin#447
    #[test]
    fn tc_1131_preserves_the_authored_claim_authority_and_independence() {
        let view = build(argument(), vec![decision()], AS_OF);
        // The expectation is the RETAINED implementation's own output, taken
        // from `dist/` at this revision and written out — not a re-derivation
        // from the code under test.
        let expected: serde_json::Value = serde_json::from_str(
            r#"{"schemaVersion":"authored-assurance-view-v1","argument":{"id":"AA-900","title":"Synthetic widget release decision","status":"active","owner":"release-owner","profile":"ix://example.invalid/widget/AP-900"},"asOf":"2026-08-15T00:00:00.000Z","topClaim":{"id":"CLAIM-900","statement":"The bounded synthetic widget change is acceptable.","subject":"widget revision 0123456789abcdef","status":"supported","reasons":[]},"reasoning":[{"id":"ARG-900","statement":"Argue from the explicitly reviewed clause disposition.","supports":"CLAIM-900","status":"supported","criteria":[{"criterion":"Every binding synthetic clause has a current disposition.","status":"supported","decision":{"reasoningId":"ARG-900","criterion":"Every binding synthetic clause has a current disposition.","state":"satisfied","evidenceRefs":["evidence://discharge/widget-900"],"decidedBy":"reviewer-900","authority":"may accept or reject this synthetic release","decidedAt":"2026-08-01T00:00:00.000Z","expiresAt":"2026-09-01T00:00:00.000Z","sourceRevision":"0123456789abcdef","evidenceDigest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}]}],"assumptions":[{"id":"ASM-900","statement":"The test environment represents the bounded target.","owner":"release-owner","status":"supported","declaredStatus":"accepted","reviewBy":"2026-09-01T00:00:00.000Z"}],"participants":[{"id":"reviewer-900","role":"decision reviewer","authority":"may accept or reject this synthetic release","independence":"did not produce the implementation evidence"}],"challenges":[{"id":"CH-900","target":"CLAIM-900","statement":"A bounded recovery case needed review.","owner":"release-owner","status":"resolved","declaredStatus":"resolved","resolutionRefs":["evidence://experiment/recovery-900"]}],"relationships":[{"target":"ix://example.invalid/widget/AP-900","type":"references"}],"unusedDecisions":[]}"#,
        )
        .expect("the recorded oracle output is JSON");
        assert_eq!(json(&view), expected);

        // The participants are carried through unchanged, authority and
        // independence included — which is the half of AC-1 a `toMatchObject`
        // on the top claim alone would not have seen.
        assert_eq!(json(&view)["participants"], argument()["participants"]);
    }

    /// No score, anywhere, at any depth.
    ///
    /// Trace: FR-047-AC-1
    /// Provenance: quoin#445
    #[test]
    fn tc_1131_emits_no_score_at_any_depth() {
        let view = build(argument(), vec![decision()], AS_OF);
        let text = serde_json::to_string(&view).expect("the view serialises");
        assert!(
            !text.contains("score"),
            "the view must promote no evidence result into a number: {text}"
        );
    }

    /// Trace: FR-047-AC-2
    /// Provenance: quoin#445
    #[test]
    fn tc_1132_leaves_an_undecided_criterion_open_instead_of_inferring_from_evidence() {
        // The argument DOES carry an evidence reference — the challenge's
        // `resolution_refs` — so an implementation that inferred from evidence
        // has something to infer from here.
        let view = build(argument(), vec![], AS_OF);
        assert_eq!(view.reasoning[0].criteria[0].status, ViewStatus::Open);
        assert_eq!(
            view.reasoning[0].criteria[0].reason.as_deref(),
            Some("no sufficiency decision")
        );
        assert_eq!(view.reasoning[0].criteria[0].decision, None);
        assert_eq!(view.top_claim.status, ViewStatus::Open);
        assert_eq!(
            view.top_claim.reasons,
            vec!["one or more sufficiency criteria are open".to_owned()]
        );
    }

    /// Trace: FR-047-AC-3
    /// Provenance: quoin#445
    #[test]
    fn tc_1133_reopens_expired_decisions_and_assumptions_due_for_review() {
        let assumption = with(
            &argument()["assumptions"][0],
            "review_by",
            serde_json::json!("2026-08-10T00:00:00.000Z"),
        );
        let expired = with(
            &decision(),
            "expiresAt",
            serde_json::json!("2026-08-10T00:00:00.000Z"),
        );
        let view = build(
            with(&argument(), "assumptions", serde_json::json!([assumption])),
            vec![expired],
            AS_OF,
        );
        assert_eq!(
            view.reasoning[0].criteria[0].reason.as_deref(),
            Some("decision is expired")
        );
        // The refused decision is still reported, which is how a reader tells
        // an expired decision from an absent one.
        assert!(view.reasoning[0].criteria[0].decision.is_some());
        assert_eq!(
            view.assumptions[0].reason.as_deref(),
            Some("assumption review is due")
        );
        assert_eq!(view.top_claim.status, ViewStatus::Open);
        assert_eq!(
            view.top_claim.reasons,
            vec![
                "one or more sufficiency criteria are open".to_owned(),
                "one or more assumptions are open, invalidated, or due for review".to_owned(),
            ]
        );
    }

    /// Trace: FR-047-AC-5
    /// Provenance: quoin#445
    #[test]
    fn tc_1135_validates_the_closed_authored_contract_and_rejects_duplicate_decisions() {
        // The round trip the retained test spelled `toEqual(argument)`: the
        // parse emits the authored document back, unchanged.
        let parsed = parse_assurance_argument(&argument()).expect("the fixture is valid");
        assert_eq!(
            serde_json::to_value(&parsed).expect("the argument serialises"),
            argument()
        );

        let cases: [(serde_json::Value, Vec<serde_json::Value>, &str); 5] = [
            (
                with(&argument(), "invented_score", serde_json::json!(100)),
                vec![decision()],
                "argument has unknown field invented_score",
            ),
            (
                argument(),
                vec![decision(), decision()],
                "duplicate sufficiency decision for ARG-900: Every binding synthetic clause has a current disposition.",
            ),
            (
                argument(),
                vec![with(
                    &decision(),
                    "authority",
                    serde_json::json!("self-declared authority"),
                )],
                "sufficiency decision authority for reviewer-900 does not match the authored participant",
            ),
            (
                argument(),
                vec![with(&decision(), "inventedScore", serde_json::json!(100))],
                "sufficiency decision has unknown field inventedScore",
            ),
            (
                with(
                    &argument(),
                    "assumptions",
                    serde_json::json!([with(
                        &argument()["assumptions"][0],
                        "id",
                        argument()["top_claim"]["id"].clone()
                    )]),
                ),
                vec![decision()],
                "top claim, reasoning, and assumption ids must be unique",
            ),
        ];
        for (argument, decisions, message) in cases {
            let error = build_authored_argument_view(&request(argument, decisions, AS_OF))
                .expect_err("the case is a rejection");
            assert_eq!(error.0, message);
        }
    }

    /// The instant is read as a NUMBER here, and the number has to be right —
    /// a validator that only answered yes/no would pass every test above and
    /// still order two instants wrongly.
    ///
    /// Trace: FR-047-AC-3, FR-047-AC-7
    /// Provenance: quoin#436, quoin#445
    #[test]
    fn tc_1712_the_offset_and_the_fraction_move_the_comparison() {
        // `2026-08-15T00:00:00.000Z` as an offset instant one minute EARLIER,
        // which must therefore read as due.
        let due = with(
            &argument()["assumptions"][0],
            "review_by",
            serde_json::json!("2026-08-15T05:29:00.000+05:30"),
        );
        let view = build(
            with(&argument(), "assumptions", serde_json::json!([due])),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(
            view.assumptions[0].reason.as_deref(),
            Some("assumption review is due"),
            "05:29+05:30 is 23:59Z the previous day, which is before the asOf"
        );

        // One minute LATER, same offset: not due.
        let current = with(
            &argument()["assumptions"][0],
            "review_by",
            serde_json::json!("2026-08-15T05:31:00.000+05:30"),
        );
        let view = build(
            with(&argument(), "assumptions", serde_json::json!([current])),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(view.assumptions[0].reason, None);

        // One millisecond after the asOf is not due; exactly the asOf is.
        for (review_by, expected) in [
            ("2026-08-15T00:00:00.001Z", None),
            ("2026-08-15T00:00:00.000Z", Some("assumption review is due")),
        ] {
            let assumption = with(
                &argument()["assumptions"][0],
                "review_by",
                serde_json::json!(review_by),
            );
            let view = build(
                with(&argument(), "assumptions", serde_json::json!([assumption])),
                vec![decision()],
                AS_OF,
            );
            assert_eq!(view.assumptions[0].reason.as_deref(), expected);
        }
    }

    /// A decision nobody asked for is reported rather than dropped.
    ///
    /// Trace: FR-047-AC-2
    /// Provenance: quoin#445
    #[test]
    fn tc_1132_a_decision_matching_no_authored_criterion_is_reported_unused() {
        let view = build(
            argument(),
            vec![with(
                &decision(),
                "criterion",
                serde_json::json!("an unmatched criterion"),
            )],
            AS_OF,
        );
        assert_eq!(view.unused_decisions.len(), 1);
        assert_eq!(view.unused_decisions[0].reasoning_id, "ARG-900");
        assert_eq!(view.unused_decisions[0].criterion, "an unmatched criterion");
        // And the criterion it did not decide is still open.
        assert_eq!(view.reasoning[0].criteria[0].status, ViewStatus::Open);
    }

    /// A non-active argument opens the top claim on its own.
    ///
    /// Trace: FR-047-AC-1
    /// Provenance: quoin#445
    #[test]
    fn tc_1131_a_proposed_argument_opens_the_top_claim() {
        let view = build(
            with(&argument(), "status", serde_json::json!("proposed")),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(view.top_claim.status, ViewStatus::Open);
        assert_eq!(
            view.top_claim.reasons,
            vec!["argument status is proposed".to_owned()]
        );
    }

    /// The control case: a claim citing evidence the store resolves cleanly
    /// passes clean, and this is what every OTHER fixture in this file already
    /// relies on — `argument()`'s top claim carries one reference and
    /// [`request`] resolves it by default.
    ///
    /// Trace: FR-047-AC-8
    /// Provenance: PLAT-966
    #[test]
    fn tc_1868_a_claim_backed_by_resolved_evidence_passes_clean() {
        let view = build(argument(), vec![decision()], AS_OF);
        assert_eq!(view.top_claim.status, ViewStatus::Supported);
        assert_eq!(view.top_claim.reasons, Vec::<String>::new());
    }

    /// A claim with no evidence references at all is unbacked.
    ///
    /// Trace: FR-047-AC-8
    /// Provenance: PLAT-966
    #[test]
    fn tc_1868_a_claim_with_no_evidence_references_is_unbacked() {
        let claim_without_refs = with(
            &argument()["top_claim"],
            "evidence_refs",
            serde_json::json!([]),
        );
        let view = build(
            with(&argument(), "top_claim", claim_without_refs),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(view.top_claim.status, ViewStatus::Open);
        assert_eq!(
            view.top_claim.reasons,
            vec!["the top claim has no evidence references".to_owned()]
        );
    }

    /// A claim citing a reference the evidence store has never heard of is
    /// unbacked, distinctly from citing nothing.
    ///
    /// Trace: FR-047-AC-8
    /// Provenance: PLAT-966
    #[test]
    fn tc_1868_a_claim_citing_an_unresolved_reference_is_unbacked() {
        let view = build_authored_argument_view(&request_with_evidence(
            argument(),
            vec![decision()],
            AS_OF,
            Vec::new(),
        ))
        .expect("the fixture builds");
        assert_eq!(view.top_claim.status, ViewStatus::Open);
        assert_eq!(
            view.top_claim.reasons,
            vec![format!(
                "the top claim's evidence reference {CLAIM_EVIDENCE_REF} does not resolve in the evidence store"
            )]
        );
    }

    /// A claim citing evidence the auditor already flagged is unbacked, and
    /// the reason names the auditor's own word for each of the three states.
    ///
    /// Trace: FR-047-AC-8
    /// Provenance: PLAT-966
    #[test]
    fn tc_1868_a_claim_citing_stale_vacuous_or_suspect_evidence_is_unbacked() {
        let cases = [
            (EvidenceRefState::Stale, "is stale"),
            (EvidenceRefState::Vacuous, "is vacuous"),
            (EvidenceRefState::Suspect, "is suspect"),
        ];
        for (state, expected_tail) in cases {
            let view = build_authored_argument_view(&request_with_evidence(
                argument(),
                vec![decision()],
                AS_OF,
                vec![EvidenceIndexEntry {
                    reference: CLAIM_EVIDENCE_REF.to_owned(),
                    state,
                }],
            ))
            .expect("the fixture builds");
            assert_eq!(view.top_claim.status, ViewStatus::Open);
            assert_eq!(
                view.top_claim.reasons,
                vec![format!(
                    "the top claim's evidence reference {CLAIM_EVIDENCE_REF} {expected_tail}"
                )]
            );
        }
    }

    /// Every failing reference is reported, in authored order: an earlier
    /// stale reference does not hide a later suspect one, and a resolved
    /// reference between them contributes nothing.
    ///
    /// Trace: FR-047-AC-8
    /// Provenance: PLAT-966
    #[test]
    fn tc_1868_reports_every_failing_reference_in_authored_order() {
        let claim = with(
            &argument()["top_claim"],
            "evidence_refs",
            serde_json::json!([
                "evidence://widget/stale",
                CLAIM_EVIDENCE_REF,
                "evidence://widget/suspect",
                "evidence://widget/missing"
            ]),
        );
        let mut evidence = resolved_evidence();
        evidence.push(EvidenceIndexEntry {
            reference: "evidence://widget/stale".to_owned(),
            state: EvidenceRefState::Stale,
        });
        evidence.push(EvidenceIndexEntry {
            reference: "evidence://widget/suspect".to_owned(),
            state: EvidenceRefState::Suspect,
        });
        let view = build_authored_argument_view(&request_with_evidence(
            with(&argument(), "top_claim", claim),
            vec![decision()],
            AS_OF,
            evidence,
        ))
        .expect("the fixture builds");
        assert_eq!(view.top_claim.status, ViewStatus::Open);
        assert_eq!(
            view.top_claim.reasons,
            vec![
                "the top claim's evidence reference evidence://widget/stale is stale".to_owned(),
                "the top claim's evidence reference evidence://widget/suspect is suspect"
                    .to_owned(),
                "the top claim's evidence reference evidence://widget/missing does not resolve in the evidence store"
                    .to_owned(),
            ]
        );
    }

    /// An evidence index naming one reference twice is refused rather than
    /// resolved by list order: last-write-wins would let a trailing
    /// `resolved` entry mask a `stale` one.
    ///
    /// Trace: FR-047-AC-8
    /// Provenance: PLAT-966
    #[test]
    fn tc_1868_a_duplicate_evidence_index_entry_is_refused() {
        let mut evidence = vec![EvidenceIndexEntry {
            reference: CLAIM_EVIDENCE_REF.to_owned(),
            state: EvidenceRefState::Stale,
        }];
        evidence.extend(resolved_evidence());
        let error = build_authored_argument_view(&request_with_evidence(
            argument(),
            vec![decision()],
            AS_OF,
            evidence,
        ))
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("duplicate evidence index entry for {CLAIM_EVIDENCE_REF}")
        );
    }
}
