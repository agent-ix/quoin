// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Sealing, verifying and chaining change-assurance records (FR-063).
//!
//! The port of `sealChangeRecord`, `verifyChangeRecord`, `recordBytes`,
//! `verifyLineage` and `validateDecision`.
//!
//! # Sealing digests the normalized value, not the typed one
//!
//! [`seal_change_record`] normalizes the input JSON, validates it, and then
//! takes the digest over **that JSON**, never over a value rebuilt from the
//! typed [`ChangeAssuranceRecord`]. The two agree for every record this crate
//! accepts, and a test asserts it — but "they agree" is a fact to check rather
//! than one to rely on, because a digest that disagrees names evidence that
//! can no longer be found.

use quoin_store::{CanonicalDigest, JsonValue, canonical_bytes, digest_canonical_value};

use crate::error::{ChangeAssuranceError, LineageFailure, Subject};
use crate::model::decision::{DecisionHistory, ReviewDecision, read_review_event};
use crate::model::outcome::{Outcome, Reason};
use crate::model::receipt::ReceiptDecisionEvent;
use crate::model::record::{ChangeAssuranceRecord, DECISION_EVENT_KIND, normalize_record};

/// Seal an unsealed record: normalize its collections, validate it, attach its
/// digest, and verify the result.
///
/// # Errors
///
/// Every refusal of the v1 schema, and [`ChangeAssuranceError::DigestMismatch`]
/// if the attached digest does not verify — which can only happen if the
/// canonicalizer disagrees with itself.
pub fn seal_change_record(
    input: &JsonValue,
) -> Result<ChangeAssuranceRecord, ChangeAssuranceError> {
    let normalized = normalize_record(input);
    crate::model::record::UnsealedRecord::from_json(&normalized)?;
    let digest = digest_canonical_value(&normalized)?;
    let mut sealed = normalized.as_object()?.clone();
    sealed.set("digest", JsonValue::string(digest.as_hex()));
    verify_change_record(&JsonValue::Object(sealed))
}

/// Validate a sealed record and check its digest against its own bytes.
///
/// # Errors
///
/// Every refusal of the v1 schema, then
/// [`ChangeAssuranceError::DigestMismatch`] when the stored digest is not the
/// digest of the record's canonical bytes with the `digest` member removed.
pub fn verify_change_record(
    value: &JsonValue,
) -> Result<ChangeAssuranceRecord, ChangeAssuranceError> {
    let record = ChangeAssuranceRecord::read_sealed(value)?;
    let mut unsigned = value.as_object()?.clone();
    unsigned.remove("digest");
    let recomputed = digest_canonical_value(&JsonValue::Object(unsigned))?;
    if recomputed == record.digest {
        Ok(record)
    } else {
        Err(ChangeAssuranceError::DigestMismatch {
            subject: Subject::Record,
            stored: record.digest.as_hex().to_owned(),
            recomputed: recomputed.as_hex().to_owned(),
        })
    }
}

/// The canonical bytes a record is retained as.
///
/// # Errors
///
/// As [`quoin_store::canonical_bytes`].
pub fn record_bytes(record: &ChangeAssuranceRecord) -> Result<Vec<u8>, ChangeAssuranceError> {
    Ok(canonical_bytes(&record.to_json()?)?)
}

/// Check a record's parent chain, returning the parent digests in revision
/// order.
///
/// The rule is strict N-1: revision `n` has exactly `n - 1` parents, at
/// revisions 1 through `n - 1`, each linking to the one before, all carrying
/// the same `record_id`, and the last being the record's own `parent_digest`.
/// Revision 1 has none.
///
/// # Why the parents arrive as JSON
///
/// The oracle verifies each parent *inside* the walk, after the length check
/// and in revision order, and a parent that fails to verify is a distinct
/// verdict (`parent_invalid`) from one that is absent or mislinked. Taking
/// already-verified parents would erase that distinction, so this takes them
/// as retained and verifies them where the oracle does.
///
/// # Errors
///
/// [`ChangeAssuranceError::Lineage`] for each rule the chain breaks, and
/// [`ChangeAssuranceError::LineageParent`] when a parent is not a valid record.
pub fn verify_lineage(
    record: &ChangeAssuranceRecord,
    parents: &[JsonValue],
) -> Result<Vec<CanonicalDigest>, ChangeAssuranceError> {
    // `verifyLineage`'s first statement re-verifies the subject record
    // (`records.ts:47`). It is not redundant: a record whose digest does not
    // verify has already been admitted by the caller with a
    // `record_digest_mismatch`, and the chain it heads is unusable evidence
    // even so, which the oracle records as a *second*, lineage refusal.
    verify_change_record(&record.to_json()?)?;
    let mut ordered: Vec<&JsonValue> = parents.iter().collect();
    // The oracle sorts on `a.revision - b.revision` before anything is
    // validated, so a parent whose `revision` is not a number has no place in
    // the order. It sorts to the front here; see `DIVERGENCE.md` §3 for why
    // the verdict does not depend on where it lands.
    ordered.sort_by(|left, right| declared_revision(left).total_cmp(&declared_revision(right)));

    if record.revision.is_genesis() {
        if record.parent_digest.is_some() || !ordered.is_empty() {
            return Err(ChangeAssuranceError::Lineage {
                failure: LineageFailure::GenesisHasAParent,
            });
        }
        return Ok(Vec::new());
    }
    let expected_length = record.revision.get().saturating_sub(1);
    if u64::try_from(ordered.len()).unwrap_or(u64::MAX) != expected_length {
        return Err(ChangeAssuranceError::Lineage {
            failure: LineageFailure::RevisionGap,
        });
    }
    let mut digests: Vec<CanonicalDigest> = Vec::with_capacity(ordered.len());
    let mut expected_parent: Option<CanonicalDigest> = None;
    for (index, raw) in ordered.iter().enumerate() {
        let parent =
            verify_change_record(raw).map_err(|source| ChangeAssuranceError::LineageParent {
                index,
                source: Box::new(source),
            })?;
        if parent.record_id != record.record_id {
            return Err(ChangeAssuranceError::Lineage {
                failure: LineageFailure::CrossRecordParent,
            });
        }
        if u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1) != parent.revision.get() {
            return Err(ChangeAssuranceError::Lineage {
                failure: LineageFailure::RevisionGap,
            });
        }
        if parent.parent_digest != expected_parent {
            return Err(ChangeAssuranceError::Lineage {
                failure: LineageFailure::ParentDigestMismatch,
            });
        }
        expected_parent = Some(parent.digest.clone());
        digests.push(parent.digest);
    }
    if record.parent_digest != expected_parent {
        return Err(ChangeAssuranceError::Lineage {
            failure: LineageFailure::ImmediateParentMismatch,
        });
    }
    Ok(digests)
}

/// The `revision` a retained parent declares, or zero when it declares none.
fn declared_revision(parent: &JsonValue) -> f64 {
    parent
        .as_object()
        .ok()
        .and_then(|fields| fields.get("revision"))
        .and_then(JsonValue::as_f64)
        .unwrap_or(0.0)
}

/// The verdict a decision history produces, and the event it cites.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionCheck {
    /// The verdict.
    pub outcome: Outcome,
    /// The premises it refused.
    pub reasons: Vec<Reason>,
    /// The event the receipt cites, when exactly one qualified.
    pub event: Option<ReceiptDecisionEvent>,
}

impl DecisionCheck {
    fn refusing(outcome: Outcome, reason: Reason) -> Self {
        Self {
            outcome,
            reasons: vec![reason],
            event: None,
        }
    }
}

/// Check the retained decision history against a record.
///
/// Takes the history as JSON rather than as a parsed value because the oracle
/// distinguishes *absent* (`event_chain_missing`, incomplete) from *present
/// and unreadable* (`event_chain_invalid`, invalid), and a signature taking a
/// parsed history could not express the second.
///
/// Never fails: every refusal is a reason on the verdict, because a decision
/// that cannot be read is a verification result rather than an error.
#[must_use]
pub fn validate_decision(
    record: &ChangeAssuranceRecord,
    history: Option<&JsonValue>,
) -> DecisionCheck {
    let Some(history) = history else {
        return DecisionCheck::refusing(Outcome::Incomplete, Reason::EventChainMissing);
    };
    let Ok(history) = DecisionHistory::from_json(history) else {
        return DecisionCheck::refusing(Outcome::Invalid, Reason::EventChainInvalid);
    };
    if !history.chains() {
        return DecisionCheck::refusing(Outcome::Invalid, Reason::EventChainInvalid);
    }
    if history.run_id != record.review_workflow.run_id {
        return DecisionCheck::refusing(Outcome::Invalid, Reason::DecisionMismatch);
    }
    if history.events.is_empty() {
        return DecisionCheck::refusing(Outcome::Incomplete, Reason::DecisionMissing);
    }
    let candidates: Vec<_> = history
        .events
        .iter()
        .filter(|event| event.kind() == Some(DECISION_EVENT_KIND))
        .collect();
    if candidates.is_empty() {
        return DecisionCheck::refusing(Outcome::Incomplete, Reason::DecisionMissing);
    }
    let matching: Vec<_> = candidates
        .iter()
        .filter_map(|event| read_review_event(event))
        .filter(|review| {
            review.payload.record_id.as_str() == record.record_id.as_str()
                && review.payload.revision == record.revision.get()
                && review.payload.record_digest == record.digest.as_hex()
        })
        .collect();
    // Exactly one candidate, and every candidate qualifying. Two review events
    // on one run is an ambiguity rather than a decision, and the oracle refuses
    // it in both directions.
    let [review] = matching.as_slice() else {
        return DecisionCheck::refusing(Outcome::Invalid, Reason::DecisionMismatch);
    };
    if matching.len() != candidates.len() {
        return DecisionCheck::refusing(Outcome::Invalid, Reason::DecisionMismatch);
    }
    let event = ReceiptDecisionEvent {
        run_id: history.run_id.clone(),
        event_id: review.event_id.clone(),
        event_hash: review.event_hash.clone(),
        chain_tail_hash: history
            .chain_tail_hash()
            .unwrap_or_else(|| review.event_hash.clone()),
        recorded_actor: review.recorded_actor.clone(),
        decision: review.payload.decision,
    };
    match review.payload.decision {
        ReviewDecision::Rejected => DecisionCheck {
            outcome: Outcome::Invalid,
            reasons: vec![Reason::ReviewRejected],
            event: Some(event),
        },
        ReviewDecision::Revise => DecisionCheck {
            outcome: Outcome::Invalid,
            reasons: vec![Reason::ReviewRevisionRequested],
            event: Some(event),
        },
        ReviewDecision::Approved => DecisionCheck {
            outcome: Outcome::Valid,
            reasons: Vec::new(),
            event: Some(event),
        },
    }
}
