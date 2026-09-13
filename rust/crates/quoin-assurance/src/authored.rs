// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The authored assurance-argument view (FR-047, ported from
//! `buildAuthoredArgumentView` and `renderAuthoredArgument` in
//! `src/assurance/argument.ts`).
//!
//! # The one rule the whole module exists to keep
//!
//! **The renderer never promotes an evidence result into a claim.** A criterion
//! is supported only when a named participant, holding the authority the
//! argument itself declares for them, decided it — and the decision has to be
//! current at the stated `asOf`. Everything else reads `open`, with the reason
//! kept next to it. There is no score anywhere in the output and FR-047-AC-1
//! asserts its absence, because a number is precisely the thing a reader would
//! substitute for the decision.
//!
//! # What this module does NOT re-implement
//!
//! [`crate::argument`] already owns the instant grammar, the JavaScript-trim
//! set, the object/string/array readers and [`ArgumentError`]. Every one of
//! them is reached through that module rather than transcribed again: a second
//! instant parser is the exact shape of quoin#436, where two readers of the
//! same timestamp disagreed and a rolled date silently decided a reported
//! status.
//!
//! The comparison the view performs needs a NUMBER, not a yes/no, so
//! `argument::instant_epoch_millis` is the shared reader and
//! `argument::is_instant` is a thin caller of it.

use serde::{Deserialize, Serialize};

use crate::argument::{
    ArgumentError, ArgumentStatus, AssumptionStatus, AssuranceArgument, Challenge, ChallengeStatus,
    Participant, Relationship, TopClaim, exact_keys, instant_epoch_millis, js_trim_end, literal,
    optional_string_at, parse_assurance_argument, record, reject, string_array, string_at,
};
use crate::discharge::DischargeReport;

/// What a participant decided about one sufficiency criterion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SufficiencyDecision {
    /// The reasoning step whose criterion this decides.
    pub reasoning_id: String,
    /// The criterion, matched against the authored text verbatim.
    pub criterion: String,
    /// Whether the decider called it satisfied.
    pub state: DecisionState,
    /// Evidence the decider relied on. Non-empty when the state is satisfied.
    pub evidence_refs: Vec<String>,
    /// The participant id. Must be one the argument declares.
    pub decided_by: String,
    /// The authority claimed, checked against the participant's own.
    pub authority: String,
    /// When it was decided. An instant.
    pub decided_at: String,
    /// When it stops being current. An instant, and after `decided_at`.
    pub expires_at: String,
    /// The revision the decision was made over.
    pub source_revision: String,
    /// `sha256:<64 lowercase hex>`.
    pub evidence_digest: String,
    /// Why, when the decider gave a reason. Omitted when absent, never null.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

/// A decision's own state, which is not the same as the criterion's status.
///
/// `Satisfied` is a claim by the decider; the criterion is supported only if
/// the decision is also current. Keeping the two spellings apart is what stops
/// `state` reading as a verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum DecisionState {
    /// The decider called the criterion met.
    Satisfied,
    /// The decider explicitly did not.
    Open,
}

/// The two states everything in this view reduces to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ViewStatus {
    /// Decided, current, and nothing outstanding.
    Supported,
    /// Anything else. The `reason` beside it says which.
    Open,
}

/// A challenge answers "resolved", never "supported".
///
/// A separate enum rather than a reuse of [`ViewStatus`]: the retained view
/// spells these two words and the renderer's tick mark keys off both, so
/// collapsing them would have changed the JSON to make the Rust tidier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ChallengeViewStatus {
    /// Answered, with a reference, and still current.
    Resolved,
    /// Anything else.
    Open,
}

/// One authored criterion, as decided or not decided.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CriterionView {
    /// The authored criterion text.
    pub criterion: String,
    /// Supported only with a current decision that says satisfied.
    pub status: ViewStatus,
    /// Why it is open. Omitted when it is not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The decision consulted, when one was found — including when it was
    /// found and refused. A reader who sees only `status` cannot tell an
    /// expired decision from an absent one, and the difference is the work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<SufficiencyDecision>,
}

/// One reasoning step and its criteria.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ReasoningView {
    /// The step's id.
    pub id: String,
    /// The reasoning.
    pub statement: String,
    /// What it argues toward.
    pub supports: String,
    /// Supported only when EVERY criterion is.
    pub status: ViewStatus,
    /// In authored order.
    pub criteria: Vec<CriterionView>,
}

/// One assumption, as it reads at the stated instant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AssumptionView {
    /// The assumption's id.
    pub id: String,
    /// What is assumed.
    pub statement: String,
    /// Who owns it.
    pub owner: String,
    /// Open when it is not accepted, or when its review has come due.
    pub status: ViewStatus,
    /// What the author declared, kept beside the derived status so a reader
    /// can see that "accepted" and "supported" are different facts.
    pub declared_status: AssumptionStatus,
    /// The authored review instant.
    pub review_by: String,
    /// Why it is open. Omitted when it is not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One challenge, as it reads at the stated instant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ChallengeView {
    /// The challenge's id.
    pub id: String,
    /// The node it targets.
    pub target: String,
    /// The objection.
    pub statement: String,
    /// Who owns it.
    pub owner: String,
    /// Resolved only with a reference, and — for accepted risk — a current
    /// expiry.
    pub status: ChallengeViewStatus,
    /// What the author declared.
    pub declared_status: ChallengeStatus,
    /// Always present, and empty when none were authored. The retained view
    /// spreads `[...refs]` unconditionally, so this is NOT the optional field
    /// the definition carries.
    pub resolution_refs: Vec<String>,
    /// The authored expiry, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// Why it is open. Omitted when it is not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The argument's identity, without its body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ArgumentSummary {
    /// `AA-<digits>`.
    pub id: String,
    /// Its title.
    pub title: String,
    /// Its lifecycle state. Anything but `active` opens the top claim.
    pub status: ArgumentStatus,
    /// Who owns it.
    pub owner: String,
    /// The governing profile.
    pub profile: String,
}

/// The top claim, with the reasons it is not supported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct TopClaimView {
    /// The claim's id.
    pub id: String,
    /// What is claimed.
    pub statement: String,
    /// What it is about.
    pub subject: String,
    /// Supported only when `reasons` is empty.
    pub status: ViewStatus,
    /// Every reason, in the retained order: argument status, criteria,
    /// assumptions, challenges, then the two discharge reasons.
    pub reasons: Vec<String>,
}

/// A decision that matched no authored criterion.
///
/// Reported rather than dropped: a decision nobody asked for usually means the
/// criterion text was edited after the decision was recorded, and a view that
/// silently discarded it would read as a clean argument over a population the
/// decider did not think they were deciding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct UnusedDecision {
    /// The reasoning id the decision named.
    pub reasoning_id: String,
    /// The criterion it named.
    pub criterion: String,
}

/// The one value `schemaVersion` takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ViewSchemaVersion {
    /// `authored-assurance-view-v1`.
    #[serde(rename = "authored-assurance-view-v1")]
    V1,
}

/// The complete authored view, as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AuthoredArgumentView {
    /// Always [`ViewSchemaVersion::V1`].
    pub schema_version: ViewSchemaVersion,
    /// The argument's identity.
    pub argument: ArgumentSummary,
    /// The instant the view was evaluated at, echoed verbatim from the
    /// request — the authored spelling, not a re-formatting of the parse.
    pub as_of: String,
    /// The claim and why it does or does not stand.
    pub top_claim: TopClaimView,
    /// In authored order.
    pub reasoning: Vec<ReasoningView>,
    /// In authored order.
    pub assumptions: Vec<AssumptionView>,
    /// Carried through unchanged, so authority and independence stay visible.
    pub participants: Vec<Participant>,
    /// In authored order.
    pub challenges: Vec<ChallengeView>,
    /// Carried through unchanged.
    pub relationships: Vec<Relationship>,
    /// The clause discharge report, when one was supplied. Omitted when not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discharge: Option<DischargeReport>,
    /// In the order the decisions were supplied.
    pub unused_decisions: Vec<UnusedDecision>,
}

/// Everything the view is built from.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildAuthoredArgumentRequest {
    /// The authored argument, unvalidated. `unknown` in the retained source,
    /// and a [`serde_json::Value`] here for the same reason: the seventeen
    /// predicates in [`crate::argument`] are the contract, and a typed field
    /// would be a second door past them.
    pub argument: serde_json::Value,
    /// The sufficiency decisions, also unvalidated. The retained signature
    /// types these and the retained BODY re-parses every one, which is what
    /// FR-047-AC-5 exercises by casting an object with an extra key through
    /// it; a `Vec<SufficiencyDecision>` here would have made that test
    /// unwritable.
    pub decisions: Vec<serde_json::Value>,
    /// The evaluation instant. This layer never reads the wall clock.
    pub as_of: String,
    /// The clause discharge report, when the caller has one. Absent and `null`
    /// mean the same thing — no report — because the retained signature makes
    /// the parameter optional and a caller that omits it is not in error.
    #[serde(default)]
    pub discharge: Option<DischargeReport>,
}

/// Build the authored view at one stated instant.
///
/// # Errors
///
/// [`ArgumentError`] when the argument fails any predicate in
/// [`crate::argument`], when the `asOf` is not an instant, when a decision is
/// malformed, names an undeclared participant, claims an authority the
/// participant does not hold, or repeats a `(reasoningId, criterion)` pair.
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

    let reasons = open_reasons(
        &argument,
        &reasoning,
        &assumptions,
        &challenges,
        request.discharge.as_ref(),
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

/// Why the top claim does not stand, in the retained order.
///
/// The order is the output. A reader takes the first line as the thing to deal
/// with, so argument status comes before criteria, criteria before
/// assumptions, and the two discharge reasons last.
fn open_reasons(
    argument: &AssuranceArgument,
    reasoning: &[ReasoningView],
    assumptions: &[AssumptionView],
    challenges: &[ChallengeView],
    discharge: Option<&DischargeReport>,
) -> Vec<String> {
    let mut reasons: Vec<String> = Vec::new();
    if argument.status != ArgumentStatus::Active {
        reasons.push(format!(
            "argument status is {}",
            argument_status_text(argument.status)
        ));
    }
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

/// Render the view as deterministic markdown.
///
/// Byte-for-byte with the retained `renderAuthoredArgument`, including the
/// closing `trimEnd()` over JavaScript's whitespace set and the single newline
/// that follows it.
#[must_use]
pub fn render_authored_argument(view: &AuthoredArgumentView) -> String {
    let mut lines: Vec<String> = vec![
        format!("# {}: {}", view.argument.id, view.argument.title),
        String::new(),
        format!(
            "**{} {}** — {}",
            mark(view.top_claim.status == ViewStatus::Supported),
            view_status_upper(view.top_claim.status),
            view.top_claim.statement
        ),
        String::new(),
        format!("Subject: {}", view.top_claim.subject),
        format!("Owner: {}", view.argument.owner),
        format!("Evaluated as of: {}", view.as_of),
        String::new(),
    ];

    if !view.top_claim.reasons.is_empty() {
        lines.push("## Open reasons".to_owned());
        lines.push(String::new());
        for reason in &view.top_claim.reasons {
            lines.push(format!("- {reason}"));
        }
        lines.push(String::new());
    }

    lines.push("## Reasoning and sufficiency".to_owned());
    lines.push(String::new());
    for reasoning in &view.reasoning {
        lines.push(format!(
            "### {} {}",
            mark(reasoning.status == ViewStatus::Supported),
            reasoning.id
        ));
        lines.push(String::new());
        lines.push(reasoning.statement.clone());
        lines.push(String::new());
        for criterion in &reasoning.criteria {
            lines.push(format!(
                "- {} {}{}",
                mark(criterion.status == ViewStatus::Supported),
                criterion.criterion,
                suffix(criterion.reason.as_deref())
            ));
        }
        lines.push(String::new());
    }

    lines.push("## Assumptions".to_owned());
    lines.push(String::new());
    if view.assumptions.is_empty() {
        lines.push("_None._".to_owned());
        lines.push(String::new());
    }
    for assumption in &view.assumptions {
        lines.push(format!(
            "- {} `{}` ({}): {}{}",
            mark(assumption.status == ViewStatus::Supported),
            assumption.id,
            assumption.owner,
            assumption.statement,
            suffix(assumption.reason.as_deref())
        ));
    }
    if !view.assumptions.is_empty() {
        lines.push(String::new());
    }

    lines.push("## Challenges".to_owned());
    lines.push(String::new());
    if view.challenges.is_empty() {
        lines.push("_None._".to_owned());
        lines.push(String::new());
    }
    for challenge in &view.challenges {
        lines.push(format!(
            "- {} `{}` ({}): {}{}",
            mark(challenge.status == ChallengeViewStatus::Resolved),
            challenge.id,
            challenge.owner,
            challenge.statement,
            suffix(challenge.reason.as_deref())
        ));
    }
    if !view.challenges.is_empty() {
        lines.push(String::new());
    }

    lines.push("## Participants and authority".to_owned());
    lines.push(String::new());
    for participant in &view.participants {
        lines.push(format!(
            "- `{}` — {}; authority: {}; independence: {}",
            participant.id, participant.role, participant.authority, participant.independence
        ));
    }

    let body = lines.join("\n");
    format!("{}\n", js_trim_end(&body))
}

/// `state === "supported" || state === "resolved" ? "✓" : "◇"`.
///
/// Not `crate::render`'s mark, which is `✅`. Two renderers, two glyphs, and
/// the difference is in the retained source rather than an inconsistency to
/// tidy away.
fn mark(supported: bool) -> &'static str {
    if supported { "✓" } else { "◇" }
}

/// `status.toUpperCase()` over the two values the field can hold.
fn view_status_upper(status: ViewStatus) -> &'static str {
    match status {
        ViewStatus::Supported => "SUPPORTED",
        ViewStatus::Open => "OPEN",
    }
}

/// `reason ? ` — ${reason}` : ""`.
fn suffix(reason: Option<&str>) -> String {
    match reason {
        Some(text) => format!(" — {text}"),
        None => String::new(),
    }
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

/// `instant(name, value)`: the shared grammar, with the retained message.
fn instant_or_reject(name: &str, value: &str) -> Result<i64, ArgumentError> {
    match instant_epoch_millis(value) {
        Some(millis) => Ok(millis),
        None => reject(format!("{name} must be an ISO-8601 instant")),
    }
}

/// `evaluateChallenge(challenge, asOf)`.
///
/// The three tests are an `else if` CHAIN in the retained source, so an open
/// challenge never reaches the reference check and a resolved one never
/// reaches the expiry check. Flattening them into independent `if`s would have
/// changed which reason a reader is given.
fn evaluate_challenge(challenge: &Challenge, as_of: i64) -> Result<ChallengeView, ArgumentError> {
    let refs = challenge.resolution_refs.clone().unwrap_or_default();
    let reason = if challenge.status == ChallengeStatus::Open {
        Some("challenge is open".to_owned())
    } else if refs.is_empty() {
        Some("challenge has no resolution reference".to_owned())
    } else if challenge.status == ChallengeStatus::AcceptedRisk {
        match &challenge.expires_at {
            None => Some("accepted risk has no expiry".to_owned()),
            Some(expires_at) if instant_or_reject("expires_at", expires_at)? <= as_of => {
                Some("accepted risk is expired".to_owned())
            }
            Some(_) => None,
        }
    } else {
        None
    };
    Ok(ChallengeView {
        id: challenge.id.clone(),
        target: challenge.target.clone(),
        statement: challenge.statement.clone(),
        owner: challenge.owner.clone(),
        status: if reason.is_some() {
            ChallengeViewStatus::Open
        } else {
            ChallengeViewStatus::Resolved
        },
        declared_status: challenge.status,
        resolution_refs: refs,
        expires_at: challenge.expires_at.clone(),
        reason,
    })
}

/// The ten required keys of a sufficiency decision, plus the one optional.
const DECISION_KEYS: [&str; 11] = [
    "reasoningId",
    "criterion",
    "state",
    "evidenceRefs",
    "decidedBy",
    "authority",
    "decidedAt",
    "expiresAt",
    "sourceRevision",
    "evidenceDigest",
    "rationale",
];

/// `parseSufficiencyDecision(value)`.
///
/// Reads in the retained order, because the ORDER decides which rejection a
/// malformed decision reports and FR-047-AC-5 asserts specific messages.
fn parse_sufficiency_decision(
    value: &serde_json::Value,
) -> Result<SufficiencyDecision, ArgumentError> {
    let decision = record("sufficiency decision", Some(value))?;
    // `exactShape(required, optional)` and `exactKeys(keys, optional)` are the
    // same predicate over `required ∪ optional`, so the retained pair maps
    // onto the one helper rather than a second copy of it.
    exact_keys(
        decision,
        "sufficiency decision",
        &DECISION_KEYS,
        &["rationale"],
    )?;
    let state = literal(
        decision.get("state"),
        "decision state",
        &[
            ("satisfied", DecisionState::Satisfied),
            ("open", DecisionState::Open),
        ],
    )?;
    // A decision that says "satisfied" must cite something; one that says
    // "open" may cite nothing, and an empty list is then admissible.
    let evidence_refs = string_array(
        "evidenceRefs",
        decision.get("evidenceRefs"),
        state == DecisionState::Satisfied,
    )?;
    let decided_at = string_at(decision, "decidedAt")?;
    let expires_at = string_at(decision, "expiresAt")?;
    instant_or_reject("decidedAt", &decided_at)?;
    instant_or_reject("expiresAt", &expires_at)?;
    let evidence_digest = string_at(decision, "evidenceDigest")?;
    if !is_sha256_digest(&evidence_digest) {
        return reject("decision evidenceDigest must be sha256:<64 lowercase hex>");
    }
    Ok(SufficiencyDecision {
        reasoning_id: string_at(decision, "reasoningId")?,
        criterion: string_at(decision, "criterion")?,
        state,
        evidence_refs,
        decided_by: string_at(decision, "decidedBy")?,
        authority: string_at(decision, "authority")?,
        decided_at,
        expires_at,
        source_revision: string_at(decision, "sourceRevision")?,
        evidence_digest,
        rationale: optional_string_at(decision, "rationale")?,
    })
}

/// `/^sha256:[0-9a-f]{64}$/`, without a regex engine.
///
/// Lowercase only, and exactly 64: an uppercase digest is a different string
/// to every consumer that compares digests as text, and accepting both here
/// would put the normalisation decision in the wrong place.
fn is_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// `currentDecision(decision, asOf)`: the reason it is not current, or none.
///
/// All three comparisons are against the SAME `asOf` number the assumptions
/// and challenges use, which is why the instant reader is shared rather than
/// re-derived per call site.
fn current_decision(
    decision: &SufficiencyDecision,
    as_of: i64,
) -> Result<Option<String>, ArgumentError> {
    let decided_at = instant_or_reject("decidedAt", &decision.decided_at)?;
    let expires_at = instant_or_reject("expiresAt", &decision.expires_at)?;
    if expires_at <= decided_at {
        return Ok(Some("decision expiry is not after decision".to_owned()));
    }
    if decided_at > as_of {
        return Ok(Some("decision is in the future".to_owned()));
    }
    if expires_at <= as_of {
        return Ok(Some("decision is expired".to_owned()));
    }
    Ok(None)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{
        AuthoredArgumentView, BuildAuthoredArgumentRequest, ChallengeViewStatus, ViewStatus,
        build_authored_argument_view, render_authored_argument,
    };
    use crate::argument::parse_assurance_argument;

    const DIGEST: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const CRITERION: &str = "Every binding synthetic clause has a current disposition.";
    const AS_OF: &str = "2026-08-15T00:00:00.000Z";

    /// The fixture `tests/authored-argument.test.ts` carried, field for field.
    fn argument() -> serde_json::Value {
        serde_json::json!({
            "id": "AA-900",
            "title": "Synthetic widget release decision",
            "type": "AssuranceArgument",
            "status": "active",
            "owner": "release-owner",
            "profile": "ix://example.invalid/widget/AP-900",
            "top_claim": {
                "id": "CLAIM-900",
                "statement": "The bounded synthetic widget change is acceptable.",
                "subject": "widget revision 0123456789abcdef"
            },
            "reasoning": [{
                "id": "ARG-900",
                "statement": "Argue from the explicitly reviewed clause disposition.",
                "supports": "CLAIM-900",
                "sufficiency_criteria": [CRITERION]
            }],
            "assumptions": [{
                "id": "ASM-900",
                "statement": "The test environment represents the bounded target.",
                "owner": "release-owner",
                "status": "accepted",
                "review_by": "2026-09-01T00:00:00.000Z"
            }],
            "participants": [{
                "id": "reviewer-900",
                "role": "decision reviewer",
                "authority": "may accept or reject this synthetic release",
                "independence": "did not produce the implementation evidence"
            }],
            "challenges": [{
                "id": "CH-900",
                "target": "CLAIM-900",
                "statement": "A bounded recovery case needed review.",
                "status": "resolved",
                "owner": "release-owner",
                "resolution_refs": ["evidence://experiment/recovery-900"]
            }],
            "relationships": [{
                "target": "ix://example.invalid/widget/AP-900",
                "type": "references"
            }]
        })
    }

    fn decision() -> serde_json::Value {
        serde_json::json!({
            "reasoningId": "ARG-900",
            "criterion": CRITERION,
            "state": "satisfied",
            "evidenceRefs": ["evidence://discharge/widget-900"],
            "decidedBy": "reviewer-900",
            "authority": "may accept or reject this synthetic release",
            "decidedAt": "2026-08-01T00:00:00.000Z",
            "expiresAt": "2026-09-01T00:00:00.000Z",
            "sourceRevision": "0123456789abcdef",
            "evidenceDigest": DIGEST
        })
    }

    /// `{ ...base, [key]: value }`.
    fn with(base: &serde_json::Value, key: &str, value: serde_json::Value) -> serde_json::Value {
        let mut copy = base.clone();
        copy[key] = value;
        copy
    }

    fn request(
        argument: serde_json::Value,
        decisions: Vec<serde_json::Value>,
        as_of: &str,
    ) -> BuildAuthoredArgumentRequest {
        BuildAuthoredArgumentRequest {
            argument,
            decisions,
            as_of: as_of.to_owned(),
            discharge: None,
        }
    }

    fn build(
        argument: serde_json::Value,
        decisions: Vec<serde_json::Value>,
        as_of: &str,
    ) -> AuthoredArgumentView {
        build_authored_argument_view(&request(argument, decisions, as_of))
            .expect("the fixture builds")
    }

    fn json(view: &AuthoredArgumentView) -> serde_json::Value {
        serde_json::to_value(view).expect("the view serialises")
    }

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

    /// The other two ways a decision stops being current, which the retained
    /// suite left to the `expired` case alone.
    ///
    /// Trace: FR-047-AC-3
    /// Provenance: quoin#445
    #[test]
    fn tc_1133_a_future_decision_and_an_inverted_expiry_are_also_not_current() {
        let future = build(
            argument(),
            vec![with(
                &with(
                    &decision(),
                    "decidedAt",
                    serde_json::json!("2026-08-20T00:00:00.000Z"),
                ),
                "expiresAt",
                serde_json::json!("2026-09-20T00:00:00.000Z"),
            )],
            AS_OF,
        );
        assert_eq!(
            future.reasoning[0].criteria[0].reason.as_deref(),
            Some("decision is in the future")
        );

        let inverted = build(
            argument(),
            vec![with(
                &with(
                    &decision(),
                    "decidedAt",
                    serde_json::json!("2026-09-01T00:00:00.000Z"),
                ),
                "expiresAt",
                serde_json::json!("2026-09-01T00:00:00.000Z"),
            )],
            AS_OF,
        );
        assert_eq!(
            inverted.reasoning[0].criteria[0].reason.as_deref(),
            Some("decision expiry is not after decision")
        );
    }

    /// Trace: FR-047-AC-4
    /// Provenance: quoin#445
    #[test]
    fn tc_1134_requires_resolution_evidence_and_a_current_expiry_for_accepted_risk() {
        let risk = with(
            &with(
                &argument()["challenges"][0],
                "status",
                serde_json::json!("accepted-risk"),
            ),
            "resolution_refs",
            serde_json::json!(["decision://risk/900"]),
        );
        let view = build(
            with(&argument(), "challenges", serde_json::json!([risk.clone()])),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(view.challenges[0].status, ChallengeViewStatus::Open);
        assert_eq!(
            view.challenges[0].reason.as_deref(),
            Some("accepted risk has no expiry")
        );
        assert_eq!(
            view.top_claim.reasons,
            vec!["one or more challenges are open or no longer current".to_owned()]
        );

        // An expiry that has passed is the same verdict for a different
        // reason; one still ahead of `asOf` resolves it.
        let expired = with(
            &risk,
            "expires_at",
            serde_json::json!("2026-08-10T00:00:00.000Z"),
        );
        let view = build(
            with(&argument(), "challenges", serde_json::json!([expired])),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(
            view.challenges[0].reason.as_deref(),
            Some("accepted risk is expired")
        );

        let current = with(
            &risk,
            "expires_at",
            serde_json::json!("2026-12-01T00:00:00.000Z"),
        );
        let view = build(
            with(&argument(), "challenges", serde_json::json!([current])),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(view.challenges[0].status, ChallengeViewStatus::Resolved);
        assert_eq!(view.challenges[0].reason, None);

        // A resolved challenge with no reference is open, and says so.
        let mut bare = argument()["challenges"][0].clone();
        bare.as_object_mut().unwrap().remove("resolution_refs");
        let view = build(
            with(&argument(), "challenges", serde_json::json!([bare])),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(
            view.challenges[0].reason.as_deref(),
            Some("challenge has no resolution reference")
        );
        assert_eq!(view.challenges[0].resolution_refs, Vec::<String>::new());

        // An open challenge never reaches the reference check.
        let open = with(
            &argument()["challenges"][0],
            "status",
            serde_json::json!("open"),
        );
        let view = build(
            with(&argument(), "challenges", serde_json::json!([open])),
            vec![decision()],
            AS_OF,
        );
        assert_eq!(
            view.challenges[0].reason.as_deref(),
            Some("challenge is open")
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

    /// The decision-shape half of AC-5 the retained suite reached only through
    /// the unknown-field case.
    ///
    /// Trace: FR-047-AC-5
    /// Provenance: quoin#445
    #[test]
    fn tc_1135_validates_the_decision_maker_timestamps_and_digest() {
        let cases: [(serde_json::Value, &str); 5] = [
            (
                with(&decision(), "decidedBy", serde_json::json!("nobody-900")),
                "sufficiency decision maker nobody-900 is not a declared participant",
            ),
            (
                with(
                    &decision(),
                    "evidenceDigest",
                    serde_json::json!("sha256:zz"),
                ),
                "decision evidenceDigest must be sha256:<64 lowercase hex>",
            ),
            (
                // Uppercase hex is a DIFFERENT string to every consumer that
                // compares digests as text.
                with(
                    &decision(),
                    "evidenceDigest",
                    serde_json::json!(format!("sha256:{}", "B".repeat(64))),
                ),
                "decision evidenceDigest must be sha256:<64 lowercase hex>",
            ),
            (
                with(&decision(), "decidedAt", serde_json::json!("2026-08-01")),
                "decidedAt must be an ISO-8601 instant",
            ),
            (
                // `satisfied` must cite something.
                with(&decision(), "evidenceRefs", serde_json::json!([])),
                "evidenceRefs must be a non-empty array",
            ),
        ];
        for (decision, message) in cases {
            let error = build_authored_argument_view(&request(argument(), vec![decision], AS_OF))
                .expect_err("the case is a rejection");
            assert_eq!(error.0, message);
        }

        // A decision that says "open" may cite nothing, and falls back to the
        // standing sentence when it gives no rationale.
        let view = build(
            argument(),
            vec![with(
                &with(&decision(), "state", serde_json::json!("open")),
                "evidenceRefs",
                serde_json::json!([]),
            )],
            AS_OF,
        );
        assert_eq!(
            view.reasoning[0].criteria[0].reason.as_deref(),
            Some("decision remains open")
        );
    }

    /// Trace: FR-047-AC-6
    /// Provenance: quoin#445
    #[test]
    fn tc_1136_renders_open_reasons_and_explicit_decision_state_deterministically() {
        let view = build(argument(), vec![], AS_OF);
        let first = render_authored_argument(&view);
        assert_eq!(first, render_authored_argument(&view));
        // The retained implementation's own bytes, recorded from `dist/` at
        // this revision. A `contains` assertion would have passed over a
        // renderer that lost a heading or a blank line.
        assert_eq!(
            first,
            "# AA-900: Synthetic widget release decision\n\
             \n\
             **◇ OPEN** — The bounded synthetic widget change is acceptable.\n\
             \n\
             Subject: widget revision 0123456789abcdef\n\
             Owner: release-owner\n\
             Evaluated as of: 2026-08-15T00:00:00.000Z\n\
             \n\
             ## Open reasons\n\
             \n\
             - one or more sufficiency criteria are open\n\
             \n\
             ## Reasoning and sufficiency\n\
             \n\
             ### ◇ ARG-900\n\
             \n\
             Argue from the explicitly reviewed clause disposition.\n\
             \n\
             - ◇ Every binding synthetic clause has a current disposition. — no sufficiency decision\n\
             \n\
             ## Assumptions\n\
             \n\
             - ✓ `ASM-900` (release-owner): The test environment represents the bounded target.\n\
             \n\
             ## Challenges\n\
             \n\
             - ✓ `CH-900` (release-owner): A bounded recovery case needed review.\n\
             \n\
             ## Participants and authority\n\
             \n\
             - `reviewer-900` — decision reviewer; authority: may accept or reject this synthetic release; independence: did not produce the implementation evidence\n"
        );
    }

    /// The supported render, which drops the whole "Open reasons" section.
    ///
    /// Trace: FR-047-AC-6
    /// Provenance: quoin#445
    #[test]
    fn tc_1136_the_supported_render_carries_no_open_reasons_section() {
        let rendered = render_authored_argument(&build(argument(), vec![decision()], AS_OF));
        assert_eq!(
            rendered,
            "# AA-900: Synthetic widget release decision\n\
             \n\
             **✓ SUPPORTED** — The bounded synthetic widget change is acceptable.\n\
             \n\
             Subject: widget revision 0123456789abcdef\n\
             Owner: release-owner\n\
             Evaluated as of: 2026-08-15T00:00:00.000Z\n\
             \n\
             ## Reasoning and sufficiency\n\
             \n\
             ### ✓ ARG-900\n\
             \n\
             Argue from the explicitly reviewed clause disposition.\n\
             \n\
             - ✓ Every binding synthetic clause has a current disposition.\n\
             \n\
             ## Assumptions\n\
             \n\
             - ✓ `ASM-900` (release-owner): The test environment represents the bounded target.\n\
             \n\
             ## Challenges\n\
             \n\
             - ✓ `CH-900` (release-owner): A bounded recovery case needed review.\n\
             \n\
             ## Participants and authority\n\
             \n\
             - `reviewer-900` — decision reviewer; authority: may accept or reject this synthetic release; independence: did not produce the implementation evidence\n"
        );
    }

    /// The empty-section text, which no fixture in the retained suite reached.
    ///
    /// Trace: FR-047-AC-6
    /// Provenance: quoin#445
    #[test]
    fn tc_1136_empty_assumptions_and_challenges_render_the_none_placeholder() {
        let bare = with(
            &with(&argument(), "assumptions", serde_json::json!([])),
            "challenges",
            serde_json::json!([]),
        );
        let rendered = render_authored_argument(&build(bare, vec![decision()], AS_OF));
        assert_eq!(
            rendered,
            "# AA-900: Synthetic widget release decision\n\
             \n\
             **✓ SUPPORTED** — The bounded synthetic widget change is acceptable.\n\
             \n\
             Subject: widget revision 0123456789abcdef\n\
             Owner: release-owner\n\
             Evaluated as of: 2026-08-15T00:00:00.000Z\n\
             \n\
             ## Reasoning and sufficiency\n\
             \n\
             ### ✓ ARG-900\n\
             \n\
             Argue from the explicitly reviewed clause disposition.\n\
             \n\
             - ✓ Every binding synthetic clause has a current disposition.\n\
             \n\
             ## Assumptions\n\
             \n\
             _None._\n\
             \n\
             ## Challenges\n\
             \n\
             _None._\n\
             \n\
             ## Participants and authority\n\
             \n\
             - `reviewer-900` — decision reviewer; authority: may accept or reject this synthetic release; independence: did not produce the implementation evidence\n"
        );
    }

    /// Trace: FR-047-AC-7
    /// Provenance: quoin#436, quoin#445
    #[test]
    fn tc_1712_refuses_an_instant_naming_a_day_or_hour_that_does_not_exist() {
        // `Date.parse` ROLLED each of these rather than rejecting it.
        for review_by in [
            "2026-02-30T00:00:00.000Z",
            "2026-06-31T00:00:00.000Z",
            "2025-02-29T00:00:00.000Z",
            "2026-08-15T24:00:00.000Z",
        ] {
            let assumption = with(
                &argument()["assumptions"][0],
                "review_by",
                serde_json::json!(review_by),
            );
            let error = build_authored_argument_view(&request(
                with(&argument(), "assumptions", serde_json::json!([assumption])),
                vec![decision()],
                AS_OF,
            ))
            .expect_err("an impossible instant is a rejection");
            assert_eq!(error.0, "review_by must be an ISO-8601 instant");
        }

        // Why acceptance is not the whole question. February 30 rolls to March
        // 2, which is AFTER this `asOf` — so the assumption used to read as
        // not yet due and the argument reported a status derived from a date
        // nobody can have authored.
        let assumption = with(
            &argument()["assumptions"][0],
            "review_by",
            serde_json::json!("2026-02-30T00:00:00.000Z"),
        );
        let error = build_authored_argument_view(&request(
            with(&argument(), "assumptions", serde_json::json!([assumption])),
            vec![decision()],
            "2026-03-01T00:00:00.000Z",
        ))
        .expect_err("the rollover case is a rejection");
        assert_eq!(error.0, "review_by must be an ISO-8601 instant");

        // The fix tightens impossible dates, not unusual ones.
        let leap = with(
            &argument()["assumptions"][0],
            "review_by",
            serde_json::json!("2028-02-29T00:00:00.000Z"),
        );
        assert!(
            parse_assurance_argument(&with(&argument(), "assumptions", serde_json::json!([leap])))
                .is_ok(),
            "a real leap day stays accepted"
        );
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
}
