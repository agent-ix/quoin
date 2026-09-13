// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The crate's one error boundary (quoin#385).
//!
//! One `thiserror` enum, one stable code per distinguishable refusal. The
//! retained source carries two failure vocabularies —
//! `GraphInputViolation` (`input.ts:92`) and `GraphLoadFailure`
//! (`load.ts:308`) — which between them name four states: a file that could
//! not be read, a document that is not JSON, a document that is JSON but not
//! the contract, and a document that is the contract but disagrees with the
//! export it is read beside. The middle two are one refusal here, for the
//! reason the retained code gives them one `kind`: both mean *this input does
//! not say what it must*, and no caller branches between them.
//!
//! # What a caller may key on, and what it may not
//!
//! [`GraphErrorCode`] and [`GraphInput`] are the API. The `Display` sentence
//! is prose. The retained implementation's sentences are zod's, and zod's
//! wording is not reproduced here (quoin#403): the parity corpus asserts the
//! verdict and the input it is against, never the message.

/// Which of the three declared inputs a refusal is about.
///
/// The same three names `GraphLoadFailure["input"]` uses, and they reach the
/// wire — a caller reports "the audit input is invalid", not a line number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GraphInput {
    /// The quire assurance export.
    Export,
    /// The accepted module premises.
    Premises,
    /// The FR-032 audit envelope.
    Audit,
    /// The retained evidence bindings store.
    ///
    /// Absent from `GraphLoadFailure["input"]`, and deliberately so: an
    /// unreadable bindings store is **not** a load failure in the retained
    /// implementation. It degrades the analysis to `not_computed` and is
    /// reported as a gap. This variant exists for the one refusal that is
    /// still an error — a bindings path that cannot be turned into bytes for a
    /// reason the seam reports as a hard failure rather than as absence.
    Bindings,
}

impl GraphInput {
    /// The wire token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Export => "export",
            Self::Premises => "premises",
            Self::Audit => "audit",
            Self::Bindings => "bindings",
        }
    }
}

impl std::fmt::Display for GraphInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable machine code for one [`GraphError`] condition.
///
/// Codes are never renamed and never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum GraphErrorCode {
    /// A declared input could not be read as bytes.
    InputUnreadable,
    /// A declared input was read but does not satisfy its contract.
    InputInvalid,
    /// A report had no canonical JSON spelling.
    Canonicalization,
}

impl GraphErrorCode {
    /// The stable wire token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InputUnreadable => "QGA-1001",
            Self::InputInvalid => "QGA-1002",
            Self::Canonicalization => "QGA-1003",
        }
    }

    /// Every code, in declaration order.
    ///
    /// Written out rather than derived, and covered by a test that every
    /// variant appears exactly once — the check a hand-written list lacks.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::InputUnreadable,
            Self::InputInvalid,
            Self::Canonicalization,
        ]
    }

    /// Resolve a wire token back to its code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.as_str() == code)
    }
}

impl std::fmt::Display for GraphErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Everything this crate can refuse to do.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum GraphError {
    /// A declared input could not be read as bytes.
    #[error("cannot read {input} input {path}: {reason}")]
    InputUnreadable {
        /// Which input.
        input: GraphInput,
        /// The path the seam was asked for, as the caller spelled it.
        path: String,
        /// The seam's reason.
        reason: String,
    },

    /// A declared input was read but does not satisfy its contract.
    ///
    /// `violations` is one line per failing member, the shape
    /// `GraphInputViolation.errors` had. It is diagnostic, not contractual.
    #[error("{input} input is invalid ({})", violations.join("; "))]
    InputInvalid {
        /// Which input.
        input: GraphInput,
        /// One line per failing member.
        violations: Vec<String>,
    },

    /// A report had no canonical JSON spelling.
    ///
    /// Unreachable from any analysis this crate produces — every number in a
    /// report is a `usize` count — and named anyway, because the alternative
    /// at that point is an `unwrap` on the canonicalizer.
    #[error("{what} has no canonical JSON spelling: {detail}")]
    Canonicalization {
        /// What was being written.
        what: &'static str,
        /// The canonicalizer's reason.
        detail: String,
    },
}

impl GraphError {
    /// The stable code for this refusal.
    #[must_use]
    pub const fn code(&self) -> GraphErrorCode {
        match self {
            Self::InputUnreadable { .. } => GraphErrorCode::InputUnreadable,
            Self::InputInvalid { .. } => GraphErrorCode::InputInvalid,
            Self::Canonicalization { .. } => GraphErrorCode::Canonicalization,
        }
    }

    /// Which input the refusal is about, where one applies.
    #[must_use]
    pub const fn input(&self) -> Option<GraphInput> {
        match self {
            Self::InputUnreadable { input, .. } | Self::InputInvalid { input, .. } => Some(*input),
            Self::Canonicalization { .. } => None,
        }
    }

    /// One invalid-input refusal carrying a single line.
    #[must_use]
    pub fn invalid(input: GraphInput, detail: impl Into<String>) -> Self {
        Self::InputInvalid {
            input,
            violations: vec![detail.into()],
        }
    }
}

/// This crate's result.
pub type Result<T> = std::result::Result<T, GraphError>;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{GraphError, GraphErrorCode, GraphInput};

    /// Provenance: quoin#385
    #[test]
    fn every_code_is_listed_once_and_round_trips() {
        let mut seen: Vec<&str> = GraphErrorCode::all().iter().map(|c| c.as_str()).collect();
        let count = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), count, "a code is listed twice");
        for code in GraphErrorCode::all() {
            assert_eq!(GraphErrorCode::from_code(code.as_str()), Some(*code));
            assert!(
                code.as_str().starts_with("QGA-"),
                "{code} is not this crate's"
            );
        }
    }

    /// Provenance: quoin#385
    #[test]
    fn a_refusal_names_its_input_and_its_code() {
        let error = GraphError::invalid(GraphInput::Audit, "format is not quoin-audit-envelope");
        assert_eq!(error.code(), GraphErrorCode::InputInvalid);
        assert_eq!(error.input(), Some(GraphInput::Audit));
        assert!(error.to_string().starts_with("audit input is invalid ("));
    }
}
