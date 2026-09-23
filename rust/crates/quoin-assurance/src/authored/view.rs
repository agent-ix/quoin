// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The authored-argument view's declared shape (FR-047).
//!
//! Types and their wire spellings only, every one closed with
//! `deny_unknown_fields`. The view is computed in [`super::build`], rendered
//! in [`super::render`], and the decisions it carries are admitted in
//! [`super::parse`].

use serde::{Deserialize, Serialize};

use crate::argument::{
    ArgumentStatus, AssumptionStatus, ChallengeStatus, Participant, Relationship,
};
use crate::discharge::DischargeReport;

/// What a participant decided about one sufficiency criterion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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
    /// What the evidence store found for every reference a claim cites
    /// (PLAT-966). New surface, not a port: FR-047's retained oracle predates
    /// per-claim evidence entirely, so there is nothing to stay in parity
    /// with here. Absent means "nothing was resolved", which is why an
    /// argument authored with `evidence_refs` and no matching entry here
    /// reads as unresolved rather than silently passing.
    #[serde(default)]
    pub evidence: Vec<EvidenceIndexEntry>,
}

/// What the evidence store reports for one `ix://`/`evidence://` reference a
/// claim cites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EvidenceIndexEntry {
    /// The reference, exactly as a claim's `top_claim.evidence_refs` cites it.
    pub reference: String,
    /// What the store found there.
    pub state: EvidenceRefState,
}

/// Where one evidence reference stands.
///
/// **Not a new vocabulary.** `quoin_finding_types::FindingKind::STALE_EVIDENCE`,
/// `VACUOUS_EVIDENCE` and `SUSPECT_LINK` are the auditor's own spellings for
/// exactly these conditions, and this enum's wire form reuses them verbatim —
/// `tc_1915_reuses_the_auditors_own_evidence_vocabulary` holds the property. A
/// claim citing evidence the auditor has already flagged reports the SAME
/// word a `quoin audit` run would, rather than a second opinion spelled
/// differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceRefState {
    /// Resolves in the evidence store, and the auditor flags nothing on it.
    Resolved,
    /// `quoin_finding_types::FindingKind::STALE_EVIDENCE`.
    #[serde(rename = "stale-evidence")]
    Stale,
    /// `quoin_finding_types::FindingKind::VACUOUS_EVIDENCE`.
    #[serde(rename = "vacuous-evidence")]
    Vacuous,
    /// `quoin_finding_types::FindingKind::SUSPECT_LINK`.
    #[serde(rename = "suspect-link")]
    Suspect,
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
    use quoin_finding_types::FindingKind;

    use super::super::fixtures::{AS_OF, argument, build, decision, json};
    use super::{AuthoredArgumentView, EvidenceRefState};

    /// `EvidenceRefState`'s wire spellings are the auditor's own, not a
    /// second vocabulary that happens to look similar.
    ///
    /// Trace: FR-047-AC-8
    /// Provenance: PLAT-966
    #[test]
    fn tc_1915_reuses_the_auditors_own_evidence_vocabulary() {
        let cases = [
            (EvidenceRefState::Stale, FindingKind::STALE_EVIDENCE),
            (EvidenceRefState::Vacuous, FindingKind::VACUOUS_EVIDENCE),
            (EvidenceRefState::Suspect, FindingKind::SUSPECT_LINK),
        ];
        for (state, expected) in cases {
            let wire = serde_json::to_value(state).expect("the state serialises");
            assert_eq!(wire, serde_json::json!(expected));
        }
    }

    /// Every reader in the view refuses a field it does not know, at every
    /// depth — a field the boundary silently drops is a field the caller
    /// believes it sent, and `assurance.render_authored_argument` reads this
    /// whole document off untrusted stdin.
    ///
    /// The paths are walked rather than each type being read standalone: what
    /// matters is that the unknown key is refused where it is REACHABLE, not
    /// merely that an isolated struct would have refused it.
    ///
    /// Trace: FR-047-AC-5
    /// Provenance: agent-ix/quoin#447
    #[test]
    fn tc_447_441_every_reachable_view_type_refuses_an_unknown_field() {
        let mut unused = decision();
        unused["criterion"] = serde_json::json!("Nobody authored this criterion.");
        let view = build(argument(), vec![decision(), unused], AS_OF);
        let document = json(&view);
        serde_json::from_value::<AuthoredArgumentView>(document.clone())
            .expect("the computed view reads back");

        for pointer in [
            "",
            "/argument",
            "/topClaim",
            "/reasoning/0",
            "/reasoning/0/criteria/0",
            "/reasoning/0/criteria/0/decision",
            "/assumptions/0",
            "/participants/0",
            "/challenges/0",
            "/relationships/0",
            "/unusedDecisions/0",
        ] {
            let mut forged = document.clone();
            let target = if pointer.is_empty() {
                &mut forged
            } else {
                forged
                    .pointer_mut(pointer)
                    .unwrap_or_else(|| panic!("{pointer} is a path this view has"))
            };
            target
                .as_object_mut()
                .unwrap_or_else(|| panic!("{pointer} names an object"))
                .insert("inventedField".to_owned(), serde_json::json!(1));

            let error = serde_json::from_value::<AuthoredArgumentView>(forged).err();
            let error =
                error.unwrap_or_else(|| panic!("an unknown field at {pointer} must be refused"));
            assert!(
                error.to_string().contains("unknown field"),
                "{pointer} was refused for the wrong reason: {error}"
            );
        }
    }
}
