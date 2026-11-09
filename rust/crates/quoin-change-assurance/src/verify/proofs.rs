// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One proof obligation's verdict.
//!
//! The port of the body of `verifyChangeAssurance`'s proof loop
//! (`verify.ts:87-200`). Every premise it can refuse is checked, and refusing
//! one never stops the rest: a proof whose attestation is bound to the wrong
//! record *and* whose output is absent records both, because a receipt is a
//! report on the evidence rather than the first thing wrong with it.

use crate::attestations::read_attestation_json;
use crate::ids::{AttestationId, ObligationId, ProofId, RecordId, Revision};
use crate::model::attestation::{ProducerResult, ProofAttestation};
use crate::model::json::{compare_utf16, sort_utf16};
use crate::model::outcome::{Reason, normalize_reasons, outcome_for_reasons};
use crate::model::receipt::{AuditFinding, ProofResult};
use crate::model::record::ProofObligation;
use crate::verify::audit::{AuditState, adapt_audit, map_audit_finding};
use crate::verify::input::{RetainedAttestation, RetainedAudit, Selection};

/// Everything one proof's verdict is drawn from.
pub struct ProofContext<'a> {
    /// The obligation being judged.
    pub obligation: &'a ProofObligation,
    /// The identity of the record under verification.
    pub record_id: &'a RecordId,
    /// The revision of the record under verification.
    pub record_revision: Revision,
    /// The candidate revision the verification is about.
    pub candidate_revision: &'a str,
    /// The selections naming this obligation, in input order.
    pub selected: Vec<&'a Selection>,
    /// The attestations retained under the selected id.
    pub retained: Vec<&'a RetainedAttestation>,
    /// The audits filed against this obligation.
    pub audits: Vec<&'a RetainedAudit>,
}

/// Judge one proof obligation.
#[must_use]
pub fn judge(context: &ProofContext<'_>) -> ProofResult {
    let mut reasons: Vec<Reason> = Vec::new();

    if context.selected.is_empty() {
        reasons.push(Reason::AttestationMissing);
    }
    if context.selected.len() > 1 {
        reasons.push(Reason::AttestationSchemaInvalid);
    }
    // A selection naming an id that two retained attestations are both filed
    // under is an ambiguity, not a choice.
    if context.retained.len() > 1 {
        reasons.push(Reason::AttestationSchemaInvalid);
    }
    if context.selected.len() == 1 && context.retained.is_empty() {
        reasons.push(Reason::AttestationMissing);
    }

    let pair = match context.retained.as_slice() {
        [only] => Some(*only),
        _ => None,
    };
    let mut verified: Option<ProofAttestation> = None;
    if let Some(pair) = pair {
        match read_attestation_json(&pair.attestation) {
            Ok(attestation) => verified = Some(attestation),
            Err(_) => reasons.push(Reason::AttestationSchemaInvalid),
        }
    }
    if let (Some(pair), Some(attestation)) = (pair, verified.as_ref()) {
        judge_attestation(context, pair, attestation, &mut reasons);
    }

    let audit_findings = judge_audits(context, &mut reasons);

    let mut obligation_ids: Vec<String> = context
        .obligation
        .obligation_ids
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect();
    sort_utf16(&mut obligation_ids);

    let ordered = normalize_reasons(&reasons);
    ProofResult {
        proof_id: context.obligation.proof_id.clone(),
        obligation_ids: obligation_ids
            .iter()
            .filter_map(|id| {
                ObligationId::parse(id, crate::error::Subject::Receipt, "obligation_id").ok()
            })
            .collect(),
        // The id cited is the one selected: a receipt that cited nothing for
        // an invalid attestation would lose the only pointer back to the
        // evidence that failed.
        attestation_id: context.selected.first().and_then(|selection| {
            AttestationId::parse(
                &selection.attestation_id,
                crate::error::Subject::Receipt,
                "attestation_id",
            )
            .ok()
        }),
        audit_findings,
        outcome: outcome_for_reasons(&ordered),
        reasons: ordered,
    }
}

/// The nine comparisons a retained attestation must survive.
fn judge_attestation(
    context: &ProofContext<'_>,
    pair: &RetainedAttestation,
    attestation: &ProofAttestation,
    reasons: &mut Vec<Reason>,
) {
    let obligation = context.obligation;
    if let [only] = context.selected.as_slice()
        && attestation.attestation_id.as_str() != only.attestation_id
    {
        reasons.push(Reason::AttestationIdMismatch);
    }
    if attestation.record_id != *context.record_id
        || attestation.record_revision != context.record_revision
    {
        reasons.push(Reason::RecordBindingMismatch);
    }
    if attestation.candidate_revision.as_str() != context.candidate_revision {
        reasons.push(Reason::CandidateRevisionMismatch);
    }
    if attestation.proof_id != obligation.proof_id {
        reasons.push(Reason::ProofIdMismatch);
    }
    if attestation.command.argv != obligation.command.argv
        || attestation.command.working_directory != obligation.command.working_directory
    {
        reasons.push(Reason::CommandMismatch);
    }
    if attestation.tool.identity != obligation.tool_identity {
        reasons.push(Reason::ToolIdentityMismatch);
    }
    if attestation.tool.configuration_ref != obligation.configuration_ref {
        reasons.push(Reason::ConfigurationMismatch);
    }
    match &pair.output {
        None => reasons.push(Reason::OutputMissing),
        Some(bytes) => {
            let observed_size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
            if attestation.retained_output.size_bytes != observed_size {
                reasons.push(Reason::OutputSizeMismatch);
            }
        }
    }
    match attestation.result {
        ProducerResult::Failed => reasons.push(Reason::ResultFailed),
        ProducerResult::Unavailable => reasons.push(Reason::ResultUnavailable),
        ProducerResult::NotComputed => reasons.push(Reason::ResultNotComputed),
        ProducerResult::Passed => {}
    }
}

/// The audit half of a proof's verdict.
fn judge_audits(context: &ProofContext<'_>, reasons: &mut Vec<Reason>) -> Vec<AuditFinding> {
    let owning: Vec<String> = context
        .obligation
        .obligation_ids
        .iter()
        .map(|id| id.as_str().to_owned())
        .collect();
    let mut adapted = None;
    if let [only] = context.audits.as_slice() {
        match adapt_audit(only, &owning) {
            Ok(audit) => adapted = Some(audit),
            Err(_) => reasons.push(Reason::AuditFinding),
        }
    }
    if context.audits.len() > 1 {
        reasons.push(Reason::AuditFinding);
    }
    let Some(audit) = adapted else {
        reasons.push(Reason::AuditNotEvaluated);
        return Vec::new();
    };
    // A report that left an owning obligation unevaluated still names itself,
    // and the receipt still cites it: the proof is undischarged *by this
    // report*, which is a fact about evidence someone can go and read.
    if audit.state == AuditState::NotEvaluated {
        reasons.push(Reason::AuditNotEvaluated);
    } else {
        for obligation in &owning {
            if !audit.healthy_obligation_ids.contains(obligation) {
                reasons.push(Reason::AuditFinding);
            }
        }
        for (_, kind) in &audit.findings {
            reasons.push(map_audit_finding(kind.as_str()));
        }
    }
    let mut findings: Vec<AuditFinding> = audit
        .findings
        .iter()
        .filter_map(|(obligation, kind)| {
            ObligationId::parse(
                obligation,
                crate::error::Subject::Receipt,
                "audit obligation_id",
            )
            .ok()
            .map(|obligation_id| AuditFinding {
                obligation_id,
                kind: kind.clone(),
            })
        })
        .collect();
    findings.sort_by(|left, right| {
        compare_utf16(left.obligation_id.as_str(), right.obligation_id.as_str())
            .then_with(|| compare_utf16(left.kind.as_str(), right.kind.as_str()))
    });
    findings
}

/// The obligations this crate judges, in the order a receipt lists them.
#[must_use]
pub fn ordered_obligations(obligations: &[ProofObligation]) -> Vec<&ProofObligation> {
    let mut ordered: Vec<&ProofObligation> = obligations.iter().collect();
    ordered.sort_by(|left, right| compare_utf16(left.proof_id.as_str(), right.proof_id.as_str()));
    ordered
}

/// Which retained attestations are filed under `attestation_id`.
#[must_use]
pub fn retained_under<'a>(
    attestations: &'a [RetainedAttestation],
    attestation_id: &str,
) -> Vec<&'a RetainedAttestation> {
    attestations
        .iter()
        .filter(|pair| pair.filed_as == attestation_id)
        .collect()
}

/// A proof obligation's identity, for grouping.
#[must_use]
pub fn proof_key(obligation: &ProofObligation) -> &ProofId {
    &obligation.proof_id
}
