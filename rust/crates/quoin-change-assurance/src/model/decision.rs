// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The retained ix-flow decision history (FR-013's event shape, FR-065's use
//! of it).
//!
//! # Why an event is held as its own JSON and not as a struct
//!
//! `hashIxFlowEvent` (`records.ts:467`) hashes the event **as retained**: it
//! copies `actor` and `payload` across whole, and `verifyIxFlowChain` closes
//! only the seven top-level names. An actor carrying an eighth member is
//! therefore inside the hash and outside the schema, and a port that parsed
//! the actor into a struct and rebuilt it would compute a different hash for
//! an event that already exists on disk.
//!
//! So [`RetainedDecisionEvent`] keeps the object it was read from. The typed
//! accessors read out of it; the hash is taken over it.
//!
//! # The third serializer
//!
//! The oracle spells a third JSON serializer at `records.ts:500` —
//! `canonicalIxFlowJson`, keys sorted with the default `Array.prototype.sort`
//! and scalars through `JSON.stringify`. This crate does not reproduce it. It
//! calls `quoin_store::canonicalize_jcs`, and `tests/tc_455_ix_flow.rs` pins
//! that choice against bytes captured from the oracle itself. See the PR body
//! and `DIVERGENCE.md` for what the comparison found.

use quoin_store::{JsonObject, JsonValue, canonical_bytes};
use sha2::{Digest as _, Sha256};

use crate::error::{ChangeAssuranceError, Subject};
use crate::ids::{EventHash, EventId, Identity, NonEmptyText, RunId};
use crate::model::json::{Fields, number};

const SUBJECT: Subject = Subject::DecisionHistory;

/// The seven names an ix-flow event carries, and no others.
pub const EVENT_MEMBERS: [&str; 7] = ["id", "ts", "actor", "kind", "payload", "prevHash", "hash"];

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
    /// Nothing beyond the member set is checked here, because nothing beyond
    /// it is checked by `verifyIxFlowChain`: a `ts` that is a number still
    /// hashes, and still fails the chain when the hash disagrees.
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

    /// The event's `hash`, when it is a stored hash at all.
    #[must_use]
    pub fn stored_hash(&self) -> Option<EventHash> {
        self.text("hash")
            .and_then(|value| EventHash::parse(value, SUBJECT, "hash").ok())
    }

    /// The event's `prevHash`, when it is a stored hash at all.
    #[must_use]
    pub fn stored_prev_hash(&self) -> Option<EventHash> {
        self.text("prevHash")
            .and_then(|value| EventHash::parse(value, SUBJECT, "prevHash").ok())
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

    /// Recompute the event's hash from everything but the `hash` member.
    ///
    /// # Errors
    ///
    /// Only [`quoin_store::StoreError::JsonNestingTooDeep`], propagated from
    /// the canonicalizer: an event that cannot be serialized has no hash.
    pub fn recompute_hash(&self) -> Result<EventHash, ChangeAssuranceError> {
        let mut unsigned = self.members.clone();
        unsigned.remove("hash");
        let bytes = canonical_bytes(&JsonValue::Object(unsigned))?;
        let hashed = Sha256::digest(&bytes);
        let mut hex = String::with_capacity(64);
        for byte in hashed {
            use std::fmt::Write as _;
            let _ = write!(hex, "{byte:02x}");
        }
        EventHash::parse(&hex, SUBJECT, "hash")
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

    /// Whether the events form an unbroken hash chain from the genesis hash.
    ///
    /// The port of `verifyIxFlowChain`. An empty history chains trivially,
    /// which is the oracle's behaviour and the reason "no events" is
    /// `decision_missing` rather than `event_chain_invalid`.
    #[must_use]
    pub fn chains(&self) -> bool {
        let mut previous = EventHash::genesis();
        for event in &self.events {
            let (Some(prev), Some(stored)) = (event.stored_prev_hash(), event.stored_hash()) else {
                return false;
            };
            if prev != previous {
                return false;
            }
            match event.recompute_hash() {
                Ok(recomputed) if recomputed == stored => {}
                _ => return false,
            }
            previous = stored;
        }
        true
    }

    /// The hash of the last retained event.
    #[must_use]
    pub fn chain_tail_hash(&self) -> Option<EventHash> {
        self.events
            .last()
            .and_then(RetainedDecisionEvent::stored_hash)
    }
}

/// The decision payload a review event carries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionPayload {
    /// The record the decision is about.
    pub record_id: Identity,
    /// The revision it is about.
    pub revision: u64,
    /// The record digest it was taken against.
    pub record_digest: String,
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
    /// Its own hash.
    pub event_hash: EventHash,
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
    EventHash::parse(event.text("prevHash")?, SUBJECT, "decision previous hash").ok()?;
    let event_hash = EventHash::parse(event.text("hash")?, SUBJECT, "decision event hash").ok()?;

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
            "record_digest",
            "decision",
            "note",
        ]
    } else {
        &[
            "schema_version",
            "record_id",
            "revision",
            "record_digest",
            "decision",
        ]
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
    let record_digest = payload.text("record_digest").ok()?.to_owned();
    let decision = ReviewDecision::parse(payload.text("decision").ok()?)?;

    Some(ReviewEvent {
        event_id,
        event_hash,
        recorded_actor,
        payload: DecisionPayload {
            record_id,
            revision,
            record_digest,
            decision,
            note,
        },
    })
}
