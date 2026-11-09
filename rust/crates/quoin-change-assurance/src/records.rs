// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Preparing, reading and chaining change-assurance records (FR-063).
//!
//! A record is addressed by `(record_id, revision)`; nothing here hashes it.
//! Preparing normalizes the input's collections into identity order, validates
//! the result against the v1 schema, and returns the typed record - the bytes
//! written are those of [`record_bytes`], so two runs over the same input write
//! the same bytes.

use quoin_store::{JsonValue, canonical_bytes};

use crate::error::{ChangeAssuranceError, LineageFailure};
use crate::model::decision::{DecisionHistory, ReviewDecision, read_review_event};
use crate::model::outcome::{Outcome, Reason};
use crate::model::receipt::ReceiptDecisionEvent;
use crate::model::record::{ChangeAssuranceRecord, DECISION_EVENT_KIND, normalize_record};

/// Normalize a record body's collections and validate it.
///
/// # Errors
///
/// Every refusal of the v1 schema, including an undeclared member such as a
/// `digest`.
pub fn prepare_change_record(
    input: &JsonValue,
) -> Result<ChangeAssuranceRecord, ChangeAssuranceError> {
    ChangeAssuranceRecord::from_json(&normalize_record(input))
}

/// Validate a stored record.
///
/// # Errors
///
/// Every refusal of the v1 schema.
pub fn read_change_record_json(
    value: &JsonValue,
) -> Result<ChangeAssuranceRecord, ChangeAssuranceError> {
    ChangeAssuranceRecord::from_json(value)
}

/// The canonical bytes a record is retained as.
///
/// # Errors
///
/// As [`quoin_store::canonical_bytes`].
pub fn record_bytes(record: &ChangeAssuranceRecord) -> Result<Vec<u8>, ChangeAssuranceError> {
    Ok(canonical_bytes(&record.to_json()?)?)
}

/// Check a record's parent chain.
///
/// The rule is strict N-1: revision `n` has exactly `n - 1` parents, at
/// revisions 1 through `n - 1`, all carrying the same `record_id`. Revision 1
/// has none.
///
/// # Why the parents arrive as JSON
///
/// A parent that fails to read is a distinct verdict (`parent_invalid`) from
/// one that is absent or mislinked, so each is read where the walk reaches it.
///
/// # Errors
///
/// [`ChangeAssuranceError::Lineage`] for each rule the chain breaks, and
/// [`ChangeAssuranceError::LineageParent`] when a parent is not a valid record.
pub fn verify_lineage(
    record: &ChangeAssuranceRecord,
    parents: &[JsonValue],
) -> Result<(), ChangeAssuranceError> {
    let mut ordered: Vec<&JsonValue> = parents.iter().collect();
    // A parent whose `revision` is not a number has no place in the order. It
    // sorts to the front here; see `DIVERGENCE.md` for why the verdict does
    // not depend on where it lands.
    ordered.sort_by(|left, right| declared_revision(left).total_cmp(&declared_revision(right)));

    if record.revision.is_genesis() {
        if ordered.is_empty() {
            return Ok(());
        }
        return Err(ChangeAssuranceError::Lineage {
            failure: LineageFailure::GenesisHasAParent,
        });
    }
    let expected_length = record.revision.get().saturating_sub(1);
    if u64::try_from(ordered.len()).unwrap_or(u64::MAX) != expected_length {
        return Err(ChangeAssuranceError::Lineage {
            failure: LineageFailure::RevisionGap,
        });
    }
    for (index, raw) in ordered.iter().enumerate() {
        let parent =
            read_change_record_json(raw).map_err(|source| ChangeAssuranceError::LineageParent {
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
    }
    Ok(())
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
/// Takes the history as JSON rather than as a parsed value because
/// verification distinguishes *absent* (`event_chain_missing`, incomplete) from *present
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
    // Events are read in `sequence` order, which must run 1..n with no gap and
    // no repeat.
    if history.in_sequence_order().is_none() {
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
        sequence: review.sequence,
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
