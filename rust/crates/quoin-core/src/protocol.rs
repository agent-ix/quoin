// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The wire contract: what an exit status means, and what is on each stream.
//!
//! # Streams
//!
//! - **stdout** is the payload and nothing else. On a run that carries no
//!   payload, stdout is empty — never a half-written object, never a log line.
//! - **stderr** is diagnostics, as one canonical JSON array. A caller that
//!   wants to report what went wrong parses it; a caller that does not can
//!   print it.
//!
//! # Exit taxonomy
//!
//! The load-bearing distinction is [`Outcome::Partial`]: a non-zero status
//! whose stdout is nevertheless a complete, valid payload. `runQuireAllowFailure`
//! in `src/quire/exec.ts` exists because `quire properties` exits 1 while
//! writing a full result for every document that did resolve, and treating
//! that as total failure silently discarded the whole property axis over two
//! untyped files (agent-ix/quoin#103). The same shape recurs here, so it is in
//! the taxonomy rather than discovered later.

use std::collections::BTreeMap;

/// How the process terminated, and therefore whether stdout is worth reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Outcome {
    /// Complete payload, no diagnostics that matter. Exit 0.
    Ok,
    /// Complete, valid payload AND diagnostics. Exit 1. The caller decides
    /// what a qualified result is worth; this status only stops the decision
    /// being made by an exception.
    Partial,
    /// Understood and refused by a stated rule. No payload. Exit 2.
    Refused,
    /// Not a well-formed request for a known operation. No payload. Exit 3.
    Invalid,
    /// The boundary itself failed. No payload. Exit 4.
    Internal,
}

impl Outcome {
    /// The process exit status.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Ok => 0,
            Self::Partial => 1,
            Self::Refused => 2,
            Self::Invalid => 3,
            Self::Internal => 4,
        }
    }

    /// Whether stdout holds a complete payload.
    ///
    /// This is the predicate `src/core/exec.ts` mirrors. It is a method on the
    /// taxonomy rather than a comparison at each call site, because "non-zero
    /// but valid" is exactly the judgement call sites get wrong.
    #[must_use]
    pub const fn carries_payload(self) -> bool {
        matches!(self, Self::Ok | Self::Partial)
    }

    /// Recover the outcome from an observed exit status.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Ok),
            1 => Some(Self::Partial),
            2 => Some(Self::Refused),
            3 => Some(Self::Invalid),
            4 => Some(Self::Internal),
            _ => None,
        }
    }
}

/// One entry of the stderr array.
///
/// A `Serialize` struct rather than a hand-built `serde_json::Value`: the
/// field list is then reviewable, and the compiler checks that every branch
/// populated it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    /// The stable code, from the catalogued enum — never a literal invented
    /// at the call site.
    pub code: String,
    /// A sentence for an operator.
    pub message: String,
    /// Ordered context. `BTreeMap` for byte-stable serialisation.
    pub context: BTreeMap<String, String>,
}

impl From<&crate::error::CoreError> for Diagnostic {
    fn from(error: &crate::error::CoreError) -> Self {
        Self {
            code: error.code.as_str().to_owned(),
            message: error.message.to_string(),
            context: error.context.clone(),
        }
    }
}

/// What an operation produced: the payload, and anything it wants said about
/// it.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    /// The JSON written to stdout.
    pub payload: serde_json::Value,
    /// The JSON array written to stderr.
    pub diagnostics: Vec<Diagnostic>,
    /// The exit status.
    pub outcome: Outcome,
}

impl Response {
    /// A clean success.
    #[must_use]
    pub fn ok(payload: serde_json::Value) -> Self {
        Self {
            payload,
            diagnostics: Vec::new(),
            outcome: Outcome::Ok,
        }
    }

    /// A complete payload qualified by one diagnostic — exit 1.
    #[must_use]
    pub fn partial(payload: serde_json::Value, diagnostic: &crate::error::CoreError) -> Self {
        Self {
            payload,
            diagnostics: vec![Diagnostic::from(diagnostic)],
            outcome: Outcome::Partial,
        }
    }
}

/// Serialise to canonical JSON: object keys sorted, no insignificant
/// whitespace.
///
/// Canonical on the way OUT, so `quoin-difftest` compares two byte strings
/// rather than two opinions about field order. `serde_json::Map` is a
/// `BTreeMap` in this build (the `preserve_order` feature is deliberately not
/// enabled), so routing a value through [`serde_json::Value`] sorts it.
///
/// # Errors
///
/// Returns [`CoreErrorCode::Io`] if the value cannot be represented as JSON —
/// a non-string map key or a non-finite float.
pub fn canonical_json<T: serde::Serialize>(value: &T) -> Result<String, crate::error::CoreError> {
    let as_value = serde_json::to_value(value).map_err(|e| {
        crate::error::CoreError::new(crate::error::CoreErrorCode::Io, e.to_string())
    })?;
    serde_json::to_string(&as_value)
        .map_err(|e| crate::error::CoreError::new(crate::error::CoreErrorCode::Io, e.to_string()))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    #[test]
    fn exit_statuses_are_the_documented_taxonomy() {
        assert_eq!(
            [
                Outcome::Ok.code(),
                Outcome::Partial.code(),
                Outcome::Refused.code(),
                Outcome::Invalid.code(),
                Outcome::Internal.code(),
            ],
            [0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn every_status_round_trips() {
        for outcome in [
            Outcome::Ok,
            Outcome::Partial,
            Outcome::Refused,
            Outcome::Invalid,
            Outcome::Internal,
        ] {
            assert_eq!(Outcome::from_code(outcome.code()), Some(outcome));
        }
        assert_eq!(Outcome::from_code(5), None);
    }

    #[test]
    fn non_zero_but_valid_is_distinguishable_from_failed() {
        assert!(Outcome::Partial.carries_payload());
        assert_ne!(Outcome::Partial.code(), 0);
        for dead in [Outcome::Refused, Outcome::Invalid, Outcome::Internal] {
            assert!(!dead.carries_payload());
        }
    }

    #[test]
    fn canonical_json_sorts_keys_and_emits_no_whitespace() {
        let value = serde_json::json!({ "z": 1, "a": { "y": 2, "b": 3 } });
        assert_eq!(
            canonical_json(&value).unwrap(),
            r#"{"a":{"b":3,"y":2},"z":1}"#
        );
    }
}
