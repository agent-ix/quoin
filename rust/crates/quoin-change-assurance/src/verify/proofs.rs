// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One proof obligation's verdict.
//!
//! The port of the body of `verifyChangeAssurance`'s proof loop
//! (`verify.ts:87-200`). Every premise it can refuse is checked, and refusing
//! one never stops the rest: a proof whose attestation is bound to the wrong
//! record *and* whose output is absent records both, because a receipt is a
//! report on the evidence rather than the first thing wrong with it.

use quoin_store::{CanonicalDigest, JsonValue, RawBytesDigest, digest_raw_bytes};

use crate::attestations::verify_attestation;
use crate::ids::{ArtifactDigest, ObligationId, ProofId};
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
    /// The digest of the record under verification.
    pub record_digest: &'a str,
    /// The candidate revision the verification is about.
    pub candidate_revision: &'a str,
    /// The selections naming this obligation, in input order.
    pub selected: Vec<&'a Selection>,
    /// The attestations retained under the selected digest.
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
    // A selection naming a digest that two retained attestations both claim is
    // an ambiguity, not a choice.
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
        match verify_attestation(&pair.attestation) {
            Ok(attestation) => verified = Some(attestation),
            Err(error) => reasons.push(if error.concerns_a_digest() {
                Reason::AttestationDigestMismatch
            } else {
                Reason::AttestationSchemaInvalid
            }),
        }
    }
    if let (Some(pair), Some(attestation)) = (pair, verified.as_ref()) {
        judge_attestation(context, pair, attestation, &mut reasons);
    }

    let (audit_report_digest, audit_findings) = judge_audits(context, &mut reasons);

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
        // Both digests are read from the attestation **as retained**, not from
        // the verified value: the oracle cites what an invalid attestation
        // declared, and a receipt that cited nothing would lose the only
        // pointer back to the evidence that failed.
        attestation_digest: pair
            .map(|pair| declared_digest(&pair.attestation))
            .filter(|digest| !digest.is_empty())
            .or_else(|| {
                context
                    .selected
                    .first()
                    .map(|selection| selection.attestation_digest.as_str())
            })
            .and_then(|digest| CanonicalDigest::parse_stored(digest).ok()),
        retained_output_digest: pair
            .and_then(|pair| retained_output_digest(&pair.attestation))
            .and_then(|digest| RawBytesDigest::parse_stored(digest).ok()),
        audit_report_digest,
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
    if attestation.record_digest.as_hex() != context.record_digest {
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
    if attestation.tool.configuration_digest != obligation.configuration_digest {
        reasons.push(Reason::ConfigurationMismatch);
    }
    match &pair.output {
        None => reasons.push(Reason::OutputMissing),
        Some(bytes) => {
            let observed_size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
            if attestation.retained_output.size_bytes != observed_size
                || attestation.retained_output.digest != digest_raw_bytes(bytes)
            {
                reasons.push(Reason::OutputDigestMismatch);
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
fn judge_audits(
    context: &ProofContext<'_>,
    reasons: &mut Vec<Reason>,
) -> (Option<ArtifactDigest>, Vec<AuditFinding>) {
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
        return (None, Vec::new());
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
    (Some(audit.report_digest), findings)
}

/// The obligations this crate judges, in the order a receipt lists them.
#[must_use]
pub fn ordered_obligations(obligations: &[ProofObligation]) -> Vec<&ProofObligation> {
    let mut ordered: Vec<&ProofObligation> = obligations.iter().collect();
    ordered.sort_by(|left, right| compare_utf16(left.proof_id.as_str(), right.proof_id.as_str()));
    ordered
}

/// Which retained attestations claim `digest` as their own.
#[must_use]
pub fn retained_under<'a>(
    attestations: &'a [RetainedAttestation],
    digest: &str,
) -> Vec<&'a RetainedAttestation> {
    attestations
        .iter()
        .filter(|pair| declared_digest(&pair.attestation) == digest)
        .collect()
}

/// The `digest` a retained attestation declares, or the empty string.
///
/// The empty string is the oracle's own `?? ""` fallback, and it matters: two
/// attestations that both declare no digest group together, and the pair is
/// then ambiguous rather than absent.
#[must_use]
pub fn declared_digest(attestation: &JsonValue) -> &str {
    attestation
        .as_object()
        .ok()
        .and_then(|fields| fields.get("digest"))
        .and_then(JsonValue::as_str)
        .unwrap_or("")
}

/// The `retained_output.digest` an attestation declares, if it declares one.
#[must_use]
pub fn retained_output_digest(attestation: &JsonValue) -> Option<&str> {
    attestation
        .as_object()
        .ok()
        .and_then(|fields| fields.get("retained_output"))
        .and_then(|output| output.as_object().ok())
        .and_then(|fields| fields.get("digest"))
        .and_then(JsonValue::as_str)
}

/// A proof obligation's identity, for grouping.
#[must_use]
pub fn proof_key(obligation: &ProofObligation) -> &ProofId {
    &obligation.proof_id
}
