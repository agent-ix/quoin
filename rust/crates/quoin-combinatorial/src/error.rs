// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The one error this crate surfaces, and its stable code catalogue.
//!
//! **Codes are the API. Never rename one, never reuse one.**
//!
//! # There is exactly one, and it is a resource bound
//!
//! Every other outcome in this crate is a value: a statement that is not a
//! combinatorial obligation is `None` from
//! [`parse_space`](crate::parse_space), not an error, because that `None` is
//! how a caller tells a combinatorial obligation from every other kind without
//! a second flag to keep in agreement with the first.
//!
//! What *is* an error is the demand set outgrowing memory. The retained
//! implementation builds `demandedTuples` into an unbounded `Set<string>`, so
//! `20-way over` a statement naming forty dimensions is a heap exhaustion with
//! no diagnostic — the process dies and the audit produces nothing. A named
//! refusal at a ceiling is this workspace's rule for every accumulator, and the
//! difference it makes is refusal versus exhaustion rather than two different
//! answers. `DIVERGENCE.md` records it.

use std::fmt;

/// A stable, never-reused code for each refusal this crate can produce.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum CombinatorialErrorCode {
    /// The declared space demands more tuples than [`crate::MAX_DEMANDED_TUPLES`].
    DemandTooLarge,
}

impl CombinatorialErrorCode {
    /// Every code, in declaration order.
    pub const ALL: [Self; 1] = [Self::DemandTooLarge];

    /// The stable wire spelling of this code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DemandTooLarge => "QCB-DEMAND-TOO-LARGE",
        }
    }

    /// The code a spelling denotes, if any denotes it.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == code)
    }
}

impl fmt::Display for CombinatorialErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A refusal from the covering-array algebra.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct CombinatorialError {
    /// Which refusal this is.
    pub code: CombinatorialErrorCode,
    /// What was refused, in one sentence.
    pub message: Box<str>,
}

impl CombinatorialError {
    /// Build a refusal.
    #[must_use]
    pub fn new(code: CombinatorialErrorCode, message: impl Into<Box<str>>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::CombinatorialErrorCode;

    #[test]
    fn every_code_round_trips_through_its_spelling() {
        for code in CombinatorialErrorCode::ALL {
            assert_eq!(CombinatorialErrorCode::from_code(code.as_str()), Some(code));
        }
    }

    #[test]
    fn no_two_codes_share_one_spelling() {
        let mut seen = std::collections::BTreeSet::new();
        for code in CombinatorialErrorCode::ALL {
            assert!(seen.insert(code.as_str()), "{code} is spelled twice");
        }
        assert_eq!(seen.len(), CombinatorialErrorCode::ALL.len());
    }
}
