// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Judging an `approved` decision's claimed approval (FR-065).
//!
//! Quoin compares the claim with the reviewed record and with the record's
//! narrowing policy, and nothing else. It never verifies a signature, runs no
//! Git and contacts no identity provider; the claim's `trust` is informational
//! and is not read here, and no trust comes from the record. Whoever relies on
//! the approver re-runs Engineering Assurance's `approval-verify`.
//!
//! The rules apply in Engineering Assurance FR-041's order: status, then
//! policy narrowing, then record binding, then meaning. The first that holds
//! decides.

use crate::model::approval::{ApprovalStatus, ClaimedApproval, VerifiedApproval};
use crate::model::outcome::{Outcome, Reason};
use crate::model::record::{ChangeAssuranceRecord, SignedApprovalPolicy};

/// What the approval rules concluded for one `approved` decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalJudgment {
    /// The review check's outcome.
    pub outcome: Outcome,
    /// The premise refused, when one was.
    pub reasons: Vec<Reason>,
    /// The claim the receipt reports, only when every rule held.
    pub claimed: Option<ClaimedApproval>,
}

impl ApprovalJudgment {
    fn refusing(outcome: Outcome, reason: Reason) -> Self {
        Self {
            outcome,
            reasons: vec![reason],
            claimed: None,
        }
    }
}

/// Judge the claim an `approved` event carries, or its absence, against the
/// record and the digest of the reviewed revision's stored bytes.
#[must_use]
pub fn judge_approval(
    record: &ChangeAssuranceRecord,
    record_digest: &str,
    claim: Option<&VerifiedApproval>,
) -> ApprovalJudgment {
    let policy = &record.review_workflow.signed_approval;
    let Some(claim) = claim else {
        return if policy.required {
            ApprovalJudgment::refusing(Outcome::Incomplete, Reason::ApprovalMissing)
        } else {
            ApprovalJudgment {
                outcome: Outcome::Valid,
                reasons: Vec::new(),
                claimed: None,
            }
        };
    };
    match &claim.status {
        ApprovalStatus::SignedBad => {
            return ApprovalJudgment::refusing(Outcome::Invalid, Reason::ApprovalSignatureBad);
        }
        ApprovalStatus::UntrustedSigner => {
            return ApprovalJudgment::refusing(Outcome::Invalid, Reason::ApprovalSignerUntrusted);
        }
        ApprovalStatus::Unsigned | ApprovalStatus::Unverifiable => {
            return ApprovalJudgment::refusing(Outcome::Incomplete, Reason::ApprovalUnverified);
        }
        _ => {}
    }
    if outside_policy(policy, claim) {
        return ApprovalJudgment::refusing(Outcome::Invalid, Reason::ApprovalSignerUntrusted);
    }
    if matches!(claim.status, ApprovalStatus::WrongRecord)
        || claim.record_id.as_str() != record.record_id.as_str()
        || claim.record_digest.as_str() != record_digest
    {
        return ApprovalJudgment::refusing(Outcome::Invalid, Reason::ApprovalRecordMismatch);
    }
    if matches!(claim.status, ApprovalStatus::MeaningMissing) {
        return ApprovalJudgment::refusing(Outcome::Incomplete, Reason::ApprovalMeaningMissing);
    }
    ApprovalJudgment {
        outcome: Outcome::Valid,
        reasons: Vec::new(),
        claimed: ClaimedApproval::from_claim(claim),
    }
}

/// Whether an `approved` claim falls outside a `required` policy's narrowing.
fn outside_policy(policy: &SignedApprovalPolicy, claim: &VerifiedApproval) -> bool {
    let Some(facts) = claim.approved().filter(|_| policy.required) else {
        return false;
    };
    !policy.signature_kinds.contains(&facts.signature_kind)
        || (!policy.approvers.is_empty()
            && !policy
                .approvers
                .iter()
                .any(|approver| approver == facts.approver.as_str()))
}
