// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Validating an authored assurance argument (quoin#384, ported from
//! `parseAssuranceArgument` in `src/assurance/argument.ts`).
//!
//! # Why this one is not a `#[derive(Deserialize)]`
//!
//! The two slices before this were transformers, and their Rust types carried
//! the work: [`crate::case::CaseInput`] deserialises and `build_case` runs.
//! This is a **validator**, and a validator's types carry almost none of it.
//!
//! The declared shape is 37 fields and every one is read — a field subset of
//! 100%, which is the number a validator always produces, because reading
//! every field IS the function. What the shape does not carry is **seventeen
//! predicates**: four format rules, three non-emptiness rules, six uniqueness
//! rules, and two graph reachability rules. A derive with
//! `deny_unknown_fields` reproduces none of them.
//!
//! So the port is written against [`serde_json::Value`] rather than against a
//! derived type. That is not a stylistic preference. Three of the behaviours
//! below are invisible to serde and would each have been a silent divergence:
//!
//! - `null` in an optional field is **asymmetric** in the retained source, and
//!   `Option<T>` collapses both halves. See [`Challenge`].
//! - JavaScript's `trim` and Rust's `str::trim` disagree on two code points,
//!   **in opposite directions**. See [`js_trim_is_empty`].
//! - An instant naming a day that does not exist must be refused, which
//!   `Date.parse` did not do until quoin#436 fixed the retained code.
//!
//! For the same reason the payload types are `Serialize` only. A derived
//! `Deserialize` on [`AssuranceArgument`] would be a second, permissive door
//! into the same struct that bypasses all seventeen predicates, and it would
//! read as harmless because deriving both is the habit.
//!
//! # Permissiveness runs both ways
//!
//! Recorded on quoin#384 and worth repeating where the code is: a port that
//! **accepts** input the retained implementation rejects is a divergence,
//! exactly as much as one that refuses what it accepts. Every deviation below
//! is annotated with which direction it would have gone.

mod instant;
mod parse;
mod read;
mod types;

pub use parse::parse_assurance_argument;
pub use types::{
    ArgumentError, ArgumentKind, ArgumentStatus, Assumption, AssumptionStatus, AssuranceArgument,
    Challenge, ChallengeStatus, Participant, Reasoning, Relationship, RelationshipType, TopClaim,
};

pub(crate) use instant::{instant_epoch_millis, js_trim_end, js_trim_is_empty};
pub(crate) use read::{exact_keys, literal, optional_string_at, record, string_array, string_at};
pub(crate) use types::reject;
