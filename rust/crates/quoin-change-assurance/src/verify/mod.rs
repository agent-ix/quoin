// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Verifying retained change-assurance evidence (FR-065).
//!
//! The port of `verifyChangeAssurance`, `verifyReceipt` and
//! `schemaInvalidReceipt` (`src/change-assurance/verify.ts`).
//!
//! # A verification always produces a receipt
//!
//! Nothing here refuses to answer. A record that does not parse produces a
//! `schema_invalid` receipt rather than an error, because "this evidence
//! cannot be read" is itself the verification result and it has to be
//! retainable, citable and digestible like any other. The one `Err` this
//! module can return is a canonicalization failure, which means the receipt
//! could not be sealed at all.

pub mod apparatus;
pub mod audit;
pub mod input;
pub mod lineage;
pub mod proofs;

use quoin_store::{CanonicalDigest, JsonValue, digest_canonical_value};

use crate::error::ChangeAssuranceError;
use crate::model::json::{compare_utf16, object};
use crate::model::outcome::{Check, Outcome, Reason, normalize_reasons, outcome_for_reasons};
use crate::model::receipt::{ReceiptChecks, ReceiptUnknown, UnsealedReceipt, VerificationReceipt};
use crate::model::record::{ChangeAssuranceRecord, Completeness, Disposition};
use crate::records::{validate_decision, verify_change_record, verify_lineage};
use crate::verify::apparatus::ApparatusContext;
use crate::verify::input::VerificationInput;
use crate::verify::lineage::lineage_reason;
use crate::verify::proofs::{ProofContext, declared_digest, judge, ordered_obligations};

pub use crate::verify::audit::{AdaptedAudit, AuditState, adapt_audit, map_audit_finding};
pub use crate::verify::input::{
    AuditReport, GoverningPlan, ReportFinding, ReportUnevaluated, RetainedAttestation,
    RetainedAudit, Selection, VerificationInput as Input,
};

/// Verify retained evidence and produce a sealed receipt.
///
/// # Errors
///
/// Only when the receipt cannot be canonicalized or fails its own schema —
/// both of which mean this crate disagrees with itself rather than that the
/// evidence is bad.
pub fn verify_change_assurance(
    input: &VerificationInput,
) -> Result<VerificationReceipt, ChangeAssuranceError> {
    let mut record_reasons: Vec<Reason> = Vec::new();
    let record = match verify_change_record(&input.record) {
        Ok(record) => record,
        Err(error) if error.is_digest_mismatch() => {
            record_reasons.push(Reason::RecordDigestMismatch);
            // The shape already passed; only the digest did not. Reading it
            // again cannot fail, but if it somehow did the verification is a
            // schema-invalid one.
            match ChangeAssuranceRecord::read_sealed(&input.record) {
                Ok(record) => record,
                Err(_) => return schema_invalid_receipt(input),
            }
        }
        Err(_) => return schema_invalid_receipt(input),
    };
    let record_check = Check::from_reasons(&record_reasons, Outcome::Invalid);

    let mut lineage_reasons: Vec<Reason> = Vec::new();
    let parent_digests: Vec<CanonicalDigest> = match verify_lineage(&record, &input.parents) {
        Ok(digests) => digests,
        Err(error) => {
            lineage_reasons.push(lineage_reason(&error));
            Vec::new()
        }
    };
    let lineage_check = Check::from_reasons(&lineage_reasons, Outcome::Invalid);

    let decision = validate_decision(&record, input.decision_history.as_ref());
    let review_check = Check {
        outcome: decision.outcome,
        reasons: decision.reasons.clone(),
    };

    let impact_check = impact_check(&record);

    let mut global_reasons: Vec<Reason> = Vec::new();
    if input.selections.iter().any(|selection| {
        !record
            .definition
            .proof_obligations
            .iter()
            .any(|proof| proof.proof_id.as_str() == selection.proof_id)
    }) {
        global_reasons.push(Reason::ProofIdMismatch);
    }

    let proofs = judge_proofs(input, &record);

    let plan = input.governing_plan.as_ref();
    global_reasons.extend(apparatus::judge(&ApparatusContext {
        protected_apparatus: plan.and_then(|plan| plan.protected_apparatus.as_ref()),
        negative_controls: plan.and_then(|plan| plan.negative_controls.as_ref()),
        diff_paths: input.diff_paths.as_deref(),
    }));

    let mut all: Vec<Reason> = global_reasons;
    all.extend(record_check.reasons.iter().copied());
    all.extend(lineage_check.reasons.iter().copied());
    all.extend(review_check.reasons.iter().copied());
    all.extend(impact_check.reasons.iter().copied());
    for proof in &proofs {
        all.extend(proof.reasons.iter().copied());
    }
    let reasons = normalize_reasons(&all);

    let mut unknowns: Vec<ReceiptUnknown> = record
        .definition
        .unknowns
        .iter()
        .map(|unknown| ReceiptUnknown {
            id: unknown.id.clone(),
            disposition: unknown.disposition,
        })
        .collect();
    unknowns.sort_by(|left, right| compare_utf16(left.id.as_str(), right.id.as_str()));

    seal(UnsealedReceipt {
        record_digest: record.digest,
        candidate_revision: crate::ids::NonEmptyText::parse(
            &input.candidate_revision,
            crate::error::Subject::Receipt,
            "candidate revision",
        )?,
        decision_event: decision.event,
        parent_digests,
        checks: ReceiptChecks {
            record: record_check,
            lineage: lineage_check,
            review: review_check,
            impact: impact_check,
        },
        proofs,
        unknowns,
        outcome: outcome_for_reasons(&reasons),
        reasons,
        governing_plan_id: governing_plan_id(input)?,
        diff_paths_supplied: input.diff_paths.is_some(),
    })
}

/// The plan id this receipt was checked against, parsed from
/// [`VerificationInput::governing_plan_id`] (PLAT-1015, FR-111).
///
/// # Errors
///
/// [`ChangeAssuranceError::Shape`] when the caller's plan id is empty — it
/// already passed [`crate::ids::Identity::parse`] at the wire boundary that
/// resolved [`VerificationInput::governing_plan`], so this is not expected to
/// fail in practice, the same way `candidate_revision`'s re-parse above is
/// not.
fn governing_plan_id(
    input: &VerificationInput,
) -> Result<Option<crate::ids::NonEmptyText>, ChangeAssuranceError> {
    input
        .governing_plan_id
        .as_deref()
        .map(|id| {
            crate::ids::NonEmptyText::parse(id, crate::error::Subject::Receipt, "governing plan id")
        })
        .transpose()
}

/// What the impact snapshot and the unknowns say about completeness.
///
/// All three are *incomplete* rather than invalid: an analysis that admits it
/// did not see everything, and a review that admits something is unsettled,
/// are honest evidence of an unfinished job rather than bad evidence.
fn impact_check(record: &ChangeAssuranceRecord) -> Check {
    let mut reasons: Vec<Reason> = Vec::new();
    if record.impact_snapshot.completeness == Completeness::Incomplete {
        reasons.push(Reason::ImpactIncomplete);
    }
    if record.impact_snapshot.truncated {
        reasons.push(Reason::ImpactTruncated);
    }
    if record
        .definition
        .unknowns
        .iter()
        .any(|unknown| unknown.disposition != Disposition::Resolved)
    {
        reasons.push(Reason::UnresolvedUnknown);
    }
    Check::from_reasons(&reasons, Outcome::Incomplete)
}

/// One verdict per proof obligation, in `proof_id` order.
fn judge_proofs(
    input: &VerificationInput,
    record: &ChangeAssuranceRecord,
) -> Vec<crate::model::receipt::ProofResult> {
    let record_digest = record.digest.as_hex().to_owned();
    ordered_obligations(&record.definition.proof_obligations)
        .into_iter()
        .map(|obligation| {
            let selected: Vec<_> = input
                .selections
                .iter()
                .filter(|selection| selection.proof_id == obligation.proof_id.as_str())
                .collect();
            // The retained attestations are only looked up when exactly one
            // selection names this obligation: with two selections there is no
            // single digest to look up, and the ambiguity is already refused.
            let retained: Vec<_> = match selected.as_slice() {
                [only] => input
                    .attestations
                    .iter()
                    .filter(|pair| declared_digest(&pair.attestation) == only.attestation_digest)
                    .collect(),
                _ => Vec::new(),
            };
            let audits: Vec<_> = input
                .audits
                .iter()
                .filter(|audit| audit.proof_id == obligation.proof_id.as_str())
                .collect();
            judge(&ProofContext {
                obligation,
                record_digest: &record_digest,
                candidate_revision: &input.candidate_revision,
                selected,
                retained,
                audits,
            })
        })
        .collect()
}

/// The receipt a record that cannot be read produces.
///
/// It still names the evidence: a record that declares a well-formed digest is
/// cited by that digest, and one that does not is cited by the digest of
/// itself wrapped in `retained_record`, so that two unreadable records are
/// never confused with one another.
///
/// # Errors
///
/// As [`verify_change_assurance`].
pub fn schema_invalid_receipt(
    input: &VerificationInput,
) -> Result<VerificationReceipt, ChangeAssuranceError> {
    // A record that is not a JSON object is wrapped before anything reads a
    // member from it, and the digest is then taken over that wrapping wrapped
    // once more (`verify.ts:284`). The double wrapping is the oracle's, and it
    // is reproduced rather than tidied: the digest is the only name an
    // unreadable record has, and changing it would strand every receipt that
    // already cites one.
    let record_object = if input.record.as_object().is_ok() {
        input.record.clone()
    } else {
        object(vec![("retained_record", input.record.clone())])
    };
    let record_digest = match record_object
        .as_object()
        .ok()
        .and_then(|fields| fields.get("digest"))
        .and_then(JsonValue::as_str)
        .and_then(|declared| CanonicalDigest::parse_stored(declared).ok())
    {
        Some(declared) => declared,
        None => digest_canonical_value(&object(vec![("retained_record", record_object)]))?,
    };
    let candidate_revision = crate::ids::NonEmptyText::parse(
        if input.candidate_revision.is_empty() {
            "invalid"
        } else {
            &input.candidate_revision
        },
        crate::error::Subject::Receipt,
        "candidate revision",
    )?;
    seal(UnsealedReceipt {
        record_digest,
        candidate_revision,
        decision_event: None,
        parent_digests: Vec::new(),
        checks: ReceiptChecks {
            record: Check {
                outcome: Outcome::Invalid,
                reasons: vec![Reason::SchemaInvalid],
            },
            lineage: Check::valid(),
            review: Check::valid(),
            impact: Check::valid(),
        },
        proofs: Vec::new(),
        unknowns: Vec::new(),
        outcome: Outcome::Invalid,
        reasons: vec![Reason::SchemaInvalid],
        governing_plan_id: governing_plan_id(input)?,
        diff_paths_supplied: input.diff_paths.is_some(),
    })
}

/// Seal a receipt and re-read it, so nothing leaves here unvalidated.
fn seal(receipt: UnsealedReceipt) -> Result<VerificationReceipt, ChangeAssuranceError> {
    let digest = digest_canonical_value(&receipt.to_json()?)?;
    verify_receipt(&receipt.seal_with(digest).to_json()?)
}

/// Read a sealed receipt and check its digest.
///
/// # Errors
///
/// Every refusal of the receipt schema — including a reason list that is
/// unordered or repeated, and an outcome that disagrees with the precedence
/// its own reasons imply — and
/// [`ChangeAssuranceError::DigestMismatch`](crate::error::ChangeAssuranceError::DigestMismatch).
pub fn verify_receipt(value: &JsonValue) -> Result<VerificationReceipt, ChangeAssuranceError> {
    let receipt = VerificationReceipt::read_sealed(value)?;
    let mut unsigned = value.as_object()?.clone();
    unsigned.remove("digest");
    let recomputed = digest_canonical_value(&JsonValue::Object(unsigned))?;
    if recomputed == receipt.digest {
        Ok(receipt)
    } else {
        Err(ChangeAssuranceError::DigestMismatch {
            subject: crate::error::Subject::Receipt,
            stored: receipt.digest.as_hex().to_owned(),
            recomputed: recomputed.as_hex().to_owned(),
        })
    }
}
