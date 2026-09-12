// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The one error envelope crossing the quoin boundary (FR-096).
//!
//! A caller on the other side of a pipe cannot match on a Rust type. What it
//! can match on is a stable string code and an exit status, so those are the
//! API and the message is not. Codes are never renamed and never reused: a
//! consumer that learned `CORE_UNKNOWN_OP` keeps it forever.

use crate::protocol::Outcome;

/// A stable, machine-matchable reason the boundary did not return a plain
/// success.
///
/// Every variant maps to exactly one [`Outcome`], so the exit status a caller
/// observes is derived from the code rather than chosen at the throw site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum CoreErrorCode {
    /// The command line was not `quoin-core <domain>.<op>`.
    BadUsage,
    /// `<domain>.<op>` named an operation this build does not implement.
    UnknownOp,
    /// stdin did not hold a JSON document.
    BadJson,
    /// stdin held JSON that is not a valid request for this operation.
    BadRequest,
    /// The request was understood and refused by a stated rule of the
    /// operation — a bound exceeded, a precondition unmet.
    Refused,
    /// The caller's protocol expectation disagrees with this build's. The
    /// payload is still complete and still worth reading; see [`Outcome::Partial`].
    ProtocolSkew,
    /// An I/O failure on stdin or stdout. Nothing the caller sent caused it.
    Io,
}

impl CoreErrorCode {
    /// The wire spelling. This string is the contract; do not change one.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BadUsage => "CORE_BAD_USAGE",
            Self::UnknownOp => "CORE_UNKNOWN_OP",
            Self::BadJson => "CORE_BAD_JSON",
            Self::BadRequest => "CORE_BAD_REQUEST",
            Self::Refused => "CORE_REFUSED",
            Self::ProtocolSkew => "CORE_PROTOCOL_SKEW",
            Self::Io => "CORE_IO",
        }
    }

    /// Every code this build can emit, for a caller enumerating the catalog.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::BadUsage,
            Self::UnknownOp,
            Self::BadJson,
            Self::BadRequest,
            Self::Refused,
            Self::ProtocolSkew,
            Self::Io,
        ]
    }

    /// Parse a wire spelling back to a code, for the TypeScript-side tests and
    /// for `quoin-difftest`'s normalised diagnostic comparison.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.as_str() == code)
    }

    /// The exit status this code terminates with.
    ///
    /// The mapping is total and lives in ONE place: an operation chooses a
    /// code, never an exit number, so a new operation cannot invent a new
    /// meaning for status 2.
    #[must_use]
    pub const fn outcome(self) -> Outcome {
        match self {
            Self::ProtocolSkew => Outcome::Partial,
            Self::Refused => Outcome::Refused,
            Self::BadUsage | Self::UnknownOp | Self::BadJson | Self::BadRequest => Outcome::Invalid,
            Self::Io => Outcome::Internal,
        }
    }
}

impl std::fmt::Display for CoreErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A code, a human sentence, and ordered context.
///
/// `BTreeMap` rather than `HashMap` so the serialised context is byte-stable —
/// `quoin-difftest` compares canonical JSON, and a map that reorders per run
/// makes that comparison report noise as a difference.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct CoreError {
    /// The stable code.
    pub code: CoreErrorCode,
    /// A sentence for an operator. Never parsed by a caller.
    pub message: Box<str>,
    /// Ordered key/value context.
    pub context: std::collections::BTreeMap<String, String>,
}

impl CoreError {
    /// Build an envelope from a code and a message.
    #[must_use]
    pub fn new(code: CoreErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into().into_boxed_str(),
            context: std::collections::BTreeMap::new(),
        }
    }

    /// Add one context entry. Consuming, so an envelope is built in one
    /// expression and is immutable afterwards.
    #[must_use]
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// The exit status this envelope terminates with.
    #[must_use]
    pub const fn outcome(&self) -> Outcome {
        self.code.outcome()
    }
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
    fn every_code_round_trips_through_its_wire_spelling() {
        for code in CoreErrorCode::all() {
            assert_eq!(CoreErrorCode::from_code(code.as_str()), Some(*code));
        }
    }

    #[test]
    fn wire_spellings_are_unique() {
        let mut seen: Vec<&str> = CoreErrorCode::all().iter().map(|c| c.as_str()).collect();
        seen.sort_unstable();
        let count = seen.len();
        seen.dedup();
        assert_eq!(seen.len(), count, "two codes share a wire spelling");
    }

    #[test]
    fn an_io_failure_is_internal_not_the_callers_fault() {
        assert_eq!(CoreErrorCode::Io.outcome(), Outcome::Internal);
        assert_eq!(CoreErrorCode::Io.outcome().code(), 4);
    }

    #[test]
    fn protocol_skew_is_the_one_code_that_still_carries_a_payload() {
        let carrying: Vec<_> = CoreErrorCode::all()
            .iter()
            .filter(|c| c.outcome().carries_payload())
            .collect();
        assert_eq!(carrying, vec![&CoreErrorCode::ProtocolSkew]);
    }
}
