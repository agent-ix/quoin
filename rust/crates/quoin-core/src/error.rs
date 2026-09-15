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
    /// An input the operation could not use in full, answered anyway from what
    /// remained. The payload is the honest answer over a reduced input, not a
    /// complete one; the diagnostic names what was dropped. Distinct from
    /// [`Self::ProtocolSkew`], which is about the caller's protocol version
    /// rather than about the content it sent.
    Degraded,
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
            Self::Degraded => "CORE_DEGRADED",
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
            Self::Degraded,
            Self::Io,
        ]
    }

    /// Parse a wire spelling back to a code, for the TypeScript-side tests and
    /// for the native fixture suite's normalised diagnostic comparison.
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
            Self::ProtocolSkew | Self::Degraded => Outcome::Partial,
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
/// The native fixture suite compares canonical JSON, and a map that reorders per run
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

    /// `all()` is hand-maintained, so nothing but this makes a variant added
    /// to the enum and forgotten there a failure.
    ///
    /// The count is written out and every code is listed by NAME. A loop over
    /// `all()` would only re-run `all()` and agree with itself — the quoin#443
    /// failure mode, and the reason `ModulesErrorCode` is pinned the same way.
    /// A new code is a deliberate act: add it to `all()`, add it here, raise
    /// the number.
    #[test]
    fn the_catalogue_is_every_variant_of_the_enum() {
        assert_eq!(
            CoreErrorCode::all().len(),
            8,
            "a code was added to the enum; add it to `all()` too"
        );
        assert_eq!(
            CoreErrorCode::all(),
            [
                CoreErrorCode::BadUsage,
                CoreErrorCode::UnknownOp,
                CoreErrorCode::BadJson,
                CoreErrorCode::BadRequest,
                CoreErrorCode::Refused,
                CoreErrorCode::ProtocolSkew,
                CoreErrorCode::Degraded,
                CoreErrorCode::Io,
            ]
        );
    }

    #[test]
    fn an_io_failure_is_internal_not_the_callers_fault() {
        assert_eq!(CoreErrorCode::Io.outcome(), Outcome::Internal);
        assert_eq!(CoreErrorCode::Io.outcome().code(), 4);
    }

    #[test]
    fn the_payload_carrying_codes_are_exactly_the_two_partial_ones() {
        // Deliberately extended, not widened: `Degraded` joins `ProtocolSkew`
        // because a partial answer over a reduced input is still an answer the
        // caller must read. Every other code means "there is nothing to read",
        // and a third entry appearing here is a design change, not a detail.
        let carrying: Vec<_> = CoreErrorCode::all()
            .iter()
            .filter(|c| c.outcome().carries_payload())
            .collect();
        assert_eq!(
            carrying,
            vec![&CoreErrorCode::ProtocolSkew, &CoreErrorCode::Degraded]
        );
    }
}
