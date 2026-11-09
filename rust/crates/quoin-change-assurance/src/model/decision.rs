// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The retained ix-flow decision history (FR-065's use of it).
//!
//! # Events are ordered by `sequence`
//!
//! An ix-flow event carries a positive integer `sequence` within its run. The
//! history is read in `sequence` order, and it must run 1..n with no gap and no
//! repeat; no chain between events is read or checked. An event is held as its own JSON
//! rather than as a struct, so an actor carrying a member this crate does not
//! know is still the actor that was retained.

use quoin_store::{JsonObject, JsonValue};

use crate::error::{ChangeAssuranceError, Subject};
use crate::ids::{EventId, Identity, NonEmptyText, RunId};
use crate::model::json::{Fields, number};

const SUBJECT: Subject = Subject::DecisionHistory;

/// The six names an ix-flow event carries, and no others.
pub const EVENT_MEMBERS: [&str; 6] = ["id", "ts", "actor", "kind", "payload", "sequence"];

/// Who recorded an event.
///
/// Attribution only. `kind` says what sort of party was recorded; it is not an
/// authentication of one.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ActorKind {
    /// An automated agent.
    Agent,
    /// A person.
    Human,
    /// A service.
    Service,
}

impl ActorKind {
    /// Every kind, in the oracle's declaration order.
    pub const ALL: [Self; 3] = [Self::Agent, Self::Human, Self::Service];

    /// The stored spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Human => "human",
            Self::Service => "service",
        }
    }

    /// Read a stored spelling.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// What a reviewer decided.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ReviewDecision {
    /// The change may proceed.
    Approved,
    /// The change may not proceed.
    Rejected,
    /// The change must be revised first.
    Revise,
}

impl ReviewDecision {
    /// Every decision, in the oracle's declaration order.
    pub const ALL: [Self; 3] = [Self::Approved, Self::Rejected, Self::Revise];

    /// The stored spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Revise => "revise",
        }
    }

    /// Read a stored spelling.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == value)
    }
}

/// One retained ix-flow event, held as the object it was read from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedDecisionEvent {
    members: JsonObject,
}

impl RetainedDecisionEvent {
    /// Read an event, refusing anything but an object carrying exactly
    /// [`EVENT_MEMBERS`].
    ///
    /// Nothing beyond the member set is checked here; `sequence` is read by
    /// [`DecisionHistory::in_sequence_order`].
    ///
    /// # Errors
    ///
    /// [`ChangeAssuranceError::Shape`] for a non-object, an absent member and
    /// an undeclared one.
    pub fn from_json(value: &JsonValue) -> Result<Self, ChangeAssuranceError> {
        let fields = Fields::read(value, SUBJECT, "ix-flow event")?;
        fields.exact(&EVENT_MEMBERS)?;
        Ok(Self {
            members: fields.object().clone(),
        })
    }

    /// The object this event was read from.
    #[must_use]
    pub const fn members(&self) -> &JsonObject {
        &self.members
    }

    /// The JSON form: the object it was read from, unchanged.
    #[must_use]
    pub fn to_json(&self) -> JsonValue {
        JsonValue::Object(self.members.clone())
    }

    /// The event's `sequence`, when it is a positive whole number.
    #[must_use]
    pub fn sequence(&self) -> Option<u64> {
        match self.members.get("sequence") {
            Some(JsonValue::Number(number)) => {
                let value = number.get();
                if value >= 1.0 && value.fract() == 0.0 && value <= 9_007_199_254_740_991.0 {
                    #[allow(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "checked above: a positive whole number within 2^53"
                    )]
                    Some(value as u64)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// The event's `kind`, when it is a string.
    #[must_use]
    pub fn kind(&self) -> Option<&str> {
        self.text("kind")
    }

    /// The event's `payload`.
    #[must_use]
    pub fn payload(&self) -> Option<&JsonValue> {
        self.members.get("payload")
    }

    /// The event's `actor`.
    #[must_use]
    pub fn actor(&self) -> Option<&JsonValue> {
        self.members.get("actor")
    }

    fn text(&self, name: &str) -> Option<&str> {
        self.members.get(name).and_then(JsonValue::as_str)
    }
}

/// The retained decision history for one ix-flow run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionHistory {
    /// The run the events belong to.
    pub run_id: RunId,
    /// The events, in retained order.
    pub events: Vec<RetainedDecisionEvent>,
}

impl DecisionHistory {
    /// Read a decision history.
    ///
    /// # Errors
    ///
    /// [`ChangeAssuranceError::Shape`] for anything `validateDecision`'s own
    /// `object`/`exact`/`identity`/`array` pass refuses, and for an event that
    /// does not carry exactly [`EVENT_MEMBERS`].
    pub fn from_json(value: &JsonValue) -> Result<Self, ChangeAssuranceError> {
        let fields = Fields::read(value, SUBJECT, "decision history")?;
        fields.exact(&["run_id", "events"])?;
        let run_id = RunId::parse(fields.text("run_id")?, SUBJECT, "decision history run_id")?;
        let mut events = Vec::new();
        for entry in fields.array("events", false)? {
            events.push(RetainedDecisionEvent::from_json(entry)?);
        }
        Ok(Self { run_id, events })
    }

    /// The events in `sequence` order, or `None` when the sequences are not
    /// exactly 1..n: a missing, non-integer, repeated or skipped `sequence`.
    ///
    /// An empty history is in order trivially, which is why "no events" is
    /// `decision_missing` rather than `event_chain_invalid`.
    #[must_use]
    pub fn in_sequence_order(&self) -> Option<Vec<&RetainedDecisionEvent>> {
        let mut ordered: Vec<(u64, &RetainedDecisionEvent)> = Vec::with_capacity(self.events.len());
        for event in &self.events {
            ordered.push((event.sequence()?, event));
        }
        ordered.sort_by_key(|(sequence, _)| *sequence);
        for (index, (sequence, _)) in ordered.iter().enumerate() {
            if u64::try_from(index).ok()?.checked_add(1)? != *sequence {
                return None;
            }
        }
        Some(ordered.into_iter().map(|(_, event)| event).collect())
    }
}

/// The decision payload a review event carries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionPayload {
    /// The record the decision is about.
    pub record_id: Identity,
    /// The revision it is about.
    pub revision: u64,
    /// What was decided.
    pub decision: ReviewDecision,
    /// An optional note.
    pub note: Option<NonEmptyText>,
}

/// A review event that passed the full candidate check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewEvent {
    /// The event's identity.
    pub event_id: EventId,
    /// Its position in the run's history.
    pub sequence: u64,
    /// The actor recorded against it. Attribution, not authentication.
    pub recorded_actor: NonEmptyText,
    /// The decision it carries.
    pub payload: DecisionPayload,
}

/// Read the candidate shape `validateDecision`'s filter requires.
///
/// Returns `None` — never an error — because the oracle's filter swallows
/// every refusal into `false`. The count of survivors is what the verdict is
/// made of, so a refused candidate is data rather than a failure.
#[must_use]
pub fn read_review_event(event: &RetainedDecisionEvent) -> Option<ReviewEvent> {
    let event_id = EventId::parse(event.text("id")?, SUBJECT, "decision event id").ok()?;
    NonEmptyText::parse(event.text("ts")?, SUBJECT, "decision event timestamp").ok()?;
    let sequence = event.sequence()?;

    let actor = Fields::read(event.actor()?, SUBJECT, "decision actor").ok()?;
    let carries_token = actor.optional("ackToken").is_some();
    if carries_token {
        actor.exact(&["kind", "id", "ackToken"]).ok()?;
        NonEmptyText::parse(
            actor.text("ackToken").ok()?,
            SUBJECT,
            "decision actor ackToken",
        )
        .ok()?;
    } else {
        actor.exact(&["kind", "id"]).ok()?;
    }
    let kind = ActorKind::parse(actor.text("kind").ok()?)?;
    let recorded_actor =
        NonEmptyText::parse(actor.text("id").ok()?, SUBJECT, "decision actor id").ok()?;

    let payload = Fields::read(event.payload()?, SUBJECT, "decision payload").ok()?;
    let carries_note = payload.optional("note").is_some();
    let allowed: &[&str] = if carries_note {
        &[
            "schema_version",
            "record_id",
            "revision",
            "decision",
            "note",
        ]
    } else {
        &["schema_version", "record_id", "revision", "decision"]
    };
    payload.exact(allowed).ok()?;
    let note = if carries_note {
        Some(NonEmptyText::parse(payload.text("note").ok()?, SUBJECT, "decision note").ok()?)
    } else {
        None
    };

    // The five equalities the oracle's filter returns, all of which must hold
    // for the candidate to count. `schema_version` must be exactly 1, the
    // actor must be a person, and the decision must be one of the three.
    // Compared as a JSON value rather than as a double: `JsonNumber`'s
    // equality is equality of the canonical serialization, which is the
    // comparison the oracle's `=== 1` actually performs on a parsed number.
    if payload.value("schema_version").ok()? != &number(1).ok()? || kind != ActorKind::Human {
        return None;
    }
    let record_id = Identity::parse(payload.text("record_id").ok()?, SUBJECT, "record_id").ok()?;
    let revision = payload.whole_number("revision").ok()?;
    let decision = ReviewDecision::parse(payload.text("decision").ok()?)?;

    Some(ReviewEvent {
        event_id,
        sequence,
        recorded_actor,
        payload: DecisionPayload {
            record_id,
            revision,
            decision,
            note,
        },
    })
}
