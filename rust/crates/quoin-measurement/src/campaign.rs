// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Pure aggregation of independently checked campaign members (FR-114).
//!
//! Engineering Assurance owns the authored definition and the run inventory.
//! This module owns only Quoin's verdict vocabulary and the `all-required`
//! decision over member evidence. Intake projects EA's generated types into
//! these borrowed views after validating identities and retained bytes.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

/// The version of Quoin's independent campaign-verdict document.
pub const CAMPAIGN_VERDICT_SCHEMA: &str = "quoin.campaign-verdict/v1";

/// The three outcomes that can be granted to a campaign or member.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignOutcome {
    /// All required evidence is present, checked, and accepted.
    Accept,
    /// Evidence contradicts the definition or a required obligation fails.
    Reject,
    /// Evidence is missing or cannot be checked.
    Inconclusive,
}

/// Why a campaign or member cannot be accepted.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignReason {
    /// Two definition members share a name.
    DuplicateMember,
    /// A dependency names no member.
    UnknownDependency,
    /// The member graph contains a cycle.
    DependencyCycle,
    /// A run has an attempt for no declared member.
    UnknownAttemptMember,
    /// A member repeats an attempt index.
    DuplicateAttempt,
    /// A required member has no recorded attempt.
    MissingAttempt,
    /// The producer did not complete a usable attempt.
    ExecutionIncomplete,
    /// The required retained collection or checker receipt is absent.
    EvidenceMissing,
    /// Retained evidence contradicts its claimed identity or member.
    EvidenceContradiction,
    /// An independent checker rejected the member.
    CheckerRejected,
    /// A dependency was rejected.
    DependencyRejected,
    /// A dependency has no conclusive accepted result.
    DependencyInconclusive,
}

/// One member projected from an EA `CampaignDefinition`.
#[derive(Clone, Copy, Debug)]
pub struct Member<'a> {
    /// Unique member name.
    pub name: &'a str,
    /// Optional generic report grouping, with no built-in domain meaning.
    pub group: Option<&'a str>,
    /// Whether `all-required` grants credit only when this member accepts.
    pub required: bool,
    /// Other declared member names that must accept first.
    pub depends_on: &'a [String],
}

/// The checker's assessment of one member attempt's retained evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttemptEvidence {
    /// Collection, plan verdict and domain verdict are identity-bound accepts.
    Accept,
    /// Retained evidence or a checker contradicts the claim.
    Reject,
    /// A required receipt or independently decidable fact is missing.
    Inconclusive,
}

/// One attempt projected from an EA `CampaignRun` after evidence checks.
#[derive(Clone, Copy, Debug)]
pub struct Attempt<'a> {
    /// Declared member name.
    pub member: &'a str,
    /// Positive invocation index, stable across resume.
    pub index: u64,
    /// Whether EA completed a typed producer response.
    pub completed: bool,
    /// The independent checks over retained evidence.
    pub evidence: AttemptEvidence,
}

/// One member's independent outcome.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberOutcome {
    /// Member name.
    pub name: String,
    /// Optional generic report grouping.
    pub group: Option<String>,
    /// Whether the definition requires it.
    pub required: bool,
    /// Independent outcome.
    pub verdict: CampaignOutcome,
    /// Every distinct reason, in declaration order.
    pub reasons: Vec<CampaignReason>,
    /// Number of attempts in the retained run.
    pub attempts: usize,
}

/// Typed Quoin campaign decision before the wire adds exact digest bindings.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignDecision {
    /// Versioned Quoin verdict schema.
    pub schema: &'static str,
    /// Independent campaign outcome.
    pub verdict: CampaignOutcome,
    /// Global structural findings.
    pub reasons: Vec<CampaignReason>,
    /// Every definition member, including optional ones.
    pub members: Vec<MemberOutcome>,
}

/// Apply the `all-required` rule to a complete checked attempt inventory.
///
/// The caller verifies definition, source, collection and receipt bytes before
/// supplying `AttemptEvidence`. A failed invocation never becomes an accept
/// merely because a later retry passed: every attempt remains visible.
#[must_use]
pub fn all_required(members: &[Member<'_>], attempts: &[Attempt<'_>]) -> CampaignDecision {
    let by_name: BTreeMap<&str, Member<'_>> = members
        .iter()
        .map(|member| (member.name, *member))
        .collect();
    let mut reasons = Vec::new();
    if by_name.len() != members.len() {
        reasons.push(CampaignReason::DuplicateMember);
    }
    let mut states: BTreeMap<&str, MemberOutcome> = members
        .iter()
        .map(|member| (member.name, member_outcome(member, attempts, &by_name)))
        .collect();
    if attempts
        .iter()
        .any(|attempt| !by_name.contains_key(attempt.member))
    {
        reasons.push(CampaignReason::UnknownAttemptMember);
    }
    if has_dependency_cycle(members, &by_name) {
        reasons.push(CampaignReason::DependencyCycle);
    }
    propagate_dependencies(members, &mut states);
    reasons.sort_unstable();
    reasons.dedup();
    let outcomes: Vec<MemberOutcome> = members
        .iter()
        .filter_map(|member| states.get(member.name).cloned())
        .collect();
    let verdict = if reasons.iter().any(|reason| is_reject(*reason))
        || outcomes
            .iter()
            .any(|member| member.required && member.verdict == CampaignOutcome::Reject)
    {
        CampaignOutcome::Reject
    } else if outcomes
        .iter()
        .any(|member| member.required && member.verdict != CampaignOutcome::Accept)
    {
        CampaignOutcome::Inconclusive
    } else {
        CampaignOutcome::Accept
    };
    CampaignDecision {
        schema: CAMPAIGN_VERDICT_SCHEMA,
        verdict,
        reasons,
        members: outcomes,
    }
}

fn member_outcome(
    member: &Member<'_>,
    attempts: &[Attempt<'_>],
    by_name: &BTreeMap<&str, Member<'_>>,
) -> MemberOutcome {
    let own: Vec<_> = attempts
        .iter()
        .filter(|attempt| attempt.member == member.name)
        .collect();
    let mut finding = MemberOutcome {
        name: member.name.to_owned(),
        group: member.group.map(str::to_owned),
        required: member.required,
        verdict: CampaignOutcome::Accept,
        reasons: Vec::new(),
        attempts: own.len(),
    };
    if own.is_empty() {
        finding.reasons.push(CampaignReason::MissingAttempt);
    }
    let mut indices = BTreeSet::new();
    for attempt in own {
        if !indices.insert(attempt.index) || attempt.index == 0 {
            finding.reasons.push(CampaignReason::DuplicateAttempt);
        }
        if attempt.completed {
            match attempt.evidence {
                AttemptEvidence::Accept => {}
                AttemptEvidence::Reject => finding.reasons.push(CampaignReason::CheckerRejected),
                AttemptEvidence::Inconclusive => {
                    finding.reasons.push(CampaignReason::EvidenceMissing);
                }
            }
        } else {
            finding.reasons.push(CampaignReason::ExecutionIncomplete);
        }
    }
    for dependency in member.depends_on {
        if !by_name.contains_key(dependency.as_str()) {
            finding.reasons.push(CampaignReason::UnknownDependency);
        }
    }
    finding.reasons.sort_unstable();
    finding.reasons.dedup();
    finding.verdict = outcome_for(&finding.reasons);
    finding
}

fn has_dependency_cycle(members: &[Member<'_>], by_name: &BTreeMap<&str, Member<'_>>) -> bool {
    let mut resolved = BTreeSet::new();
    for _ in members {
        let before = resolved.len();
        for member in members {
            if member.depends_on.iter().all(|dependency| {
                !by_name.contains_key(dependency.as_str()) || resolved.contains(dependency.as_str())
            }) {
                resolved.insert(member.name);
            }
        }
        if resolved.len() == members.len() {
            return false;
        }
        if resolved.len() == before {
            return true;
        }
    }
    resolved.len() != members.len()
}

fn propagate_dependencies<'a>(
    members: &[Member<'a>],
    states: &mut BTreeMap<&'a str, MemberOutcome>,
) {
    // A bounded fixed point is enough: each pass can only move an accepted
    // member down once, and there are at most `members.len()` members.
    for _ in members {
        let previous = states.clone();
        for member in members {
            let Some(outcome) = states.get_mut(member.name) else {
                continue;
            };
            for dependency in member.depends_on {
                match previous.get(dependency.as_str()).map(|value| value.verdict) {
                    Some(CampaignOutcome::Reject) => {
                        outcome.reasons.push(CampaignReason::DependencyRejected);
                    }
                    Some(CampaignOutcome::Inconclusive) => {
                        outcome.reasons.push(CampaignReason::DependencyInconclusive);
                    }
                    _ => {}
                }
            }
            outcome.reasons.sort_unstable();
            outcome.reasons.dedup();
            outcome.verdict = outcome_for(&outcome.reasons);
        }
    }
}

fn outcome_for(reasons: &[CampaignReason]) -> CampaignOutcome {
    if reasons.iter().any(|reason| is_reject(*reason)) {
        CampaignOutcome::Reject
    } else if reasons.is_empty() {
        CampaignOutcome::Accept
    } else {
        CampaignOutcome::Inconclusive
    }
}

fn is_reject(reason: CampaignReason) -> bool {
    matches!(
        reason,
        CampaignReason::DuplicateMember
            | CampaignReason::UnknownDependency
            | CampaignReason::DependencyCycle
            | CampaignReason::UnknownAttemptMember
            | CampaignReason::DuplicateAttempt
            | CampaignReason::EvidenceContradiction
            | CampaignReason::CheckerRejected
            | CampaignReason::DependencyRejected
    )
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    reason = "test assertions index fixed fixture members and panic when the result is incomplete"
)]
mod tests {
    use super::{Attempt, AttemptEvidence, CampaignOutcome, CampaignReason, Member, all_required};

    fn member<'a>(
        name: &'a str,
        group: Option<&'a str>,
        required: bool,
        depends_on: &'a [String],
    ) -> Member<'a> {
        Member {
            name,
            group,
            required,
            depends_on,
        }
    }

    fn accepted(name: &str, index: u64) -> Attempt<'_> {
        Attempt {
            member: name,
            index,
            completed: true,
            evidence: AttemptEvidence::Accept,
        }
    }

    /// Trace: FR-114-AC-4
    /// Provenance: PLAT-1043
    #[test]
    fn tc_1945_required_members_decide_and_optional_groups_remain_visible() {
        let members = [
            member("native-test", Some("baseline"), true, &[]),
            member("advisory", Some("check"), false, &[]),
        ];
        let result = all_required(&members, &[accepted("native-test", 1)]);
        assert_eq!(result.verdict, CampaignOutcome::Accept);
        assert_eq!(result.members[1].group.as_deref(), Some("check"));
        assert_eq!(result.members[1].verdict, CampaignOutcome::Inconclusive);

        let missing = all_required(&members, &[]);
        assert_eq!(missing.verdict, CampaignOutcome::Inconclusive);
        assert_eq!(missing.members[0].reasons, [CampaignReason::MissingAttempt]);
    }

    /// Trace: FR-114-AC-4
    /// Provenance: PLAT-1043
    #[test]
    fn tc_1945_failed_or_rejected_attempt_cannot_be_hidden_by_a_later_pass() {
        let members = [member("native-test", None, true, &[])];
        let failed = Attempt {
            member: "native-test",
            index: 1,
            completed: false,
            evidence: AttemptEvidence::Inconclusive,
        };
        let incomplete = all_required(&members, &[failed, accepted("native-test", 2)]);
        assert_eq!(incomplete.verdict, CampaignOutcome::Inconclusive);
        assert_eq!(
            incomplete.members[0].reasons,
            [CampaignReason::ExecutionIncomplete]
        );

        let rejected = Attempt {
            member: "native-test",
            index: 1,
            completed: true,
            evidence: AttemptEvidence::Reject,
        };
        let contradicted = all_required(&members, &[rejected, accepted("native-test", 2)]);
        assert_eq!(contradicted.verdict, CampaignOutcome::Reject);
        assert_eq!(
            contradicted.members[0].reasons,
            [CampaignReason::CheckerRejected]
        );
    }

    /// Trace: FR-114-AC-1, FR-114-AC-4
    /// Provenance: PLAT-1043
    #[test]
    fn tc_1940_unknown_cycle_and_duplicate_attempts_reject() {
        let cycle_a = ["b".to_owned()];
        let cycle_b = ["a".to_owned()];
        let members = [
            member("a", None, true, &cycle_a),
            member("b", None, true, &cycle_b),
        ];
        let cycle = all_required(&members, &[accepted("a", 1), accepted("b", 1)]);
        assert_eq!(cycle.verdict, CampaignOutcome::Reject);
        assert_eq!(cycle.reasons, [CampaignReason::DependencyCycle]);

        let unknown = all_required(&[member("a", None, true, &[])], &[accepted("outsider", 1)]);
        assert_eq!(unknown.verdict, CampaignOutcome::Reject);
        assert_eq!(unknown.reasons, [CampaignReason::UnknownAttemptMember]);

        let duplicate = all_required(
            &[member("a", None, true, &[])],
            &[accepted("a", 1), accepted("a", 1)],
        );
        assert_eq!(duplicate.verdict, CampaignOutcome::Reject);
        assert_eq!(
            duplicate.members[0].reasons,
            [CampaignReason::DuplicateAttempt]
        );
    }

    /// Trace: FR-114-AC-4
    /// Provenance: PLAT-1043
    #[test]
    fn tc_1945_dependency_outcome_propagates_without_domain_specific_names() {
        let dependencies = ["compile".to_owned()];
        let members = [
            member("compile", None, true, &[]),
            member("test", None, true, &dependencies),
        ];
        let compile = Attempt {
            member: "compile",
            index: 1,
            completed: true,
            evidence: AttemptEvidence::Reject,
        };
        let result = all_required(&members, &[compile, accepted("test", 1)]);
        assert_eq!(result.verdict, CampaignOutcome::Reject);
        assert_eq!(
            result.members[1].reasons,
            [CampaignReason::DependencyRejected]
        );
    }
}
