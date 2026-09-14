// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The one error type this crate's boundary returns.

use std::fmt;

use quoin_combinatorial::CombinatorialError;

/// Stable codes for everything this crate can refuse.
///
/// Two, and both are refusals the retained TypeScript does not make: it either
/// exhausts the heap ([`Self::DemandTooLarge`]) or reports the failure as data
/// ([`Self::ManifestUnreadable`]). See `DIVERGENCE.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum AuditorErrorCode {
    /// A declared configuration space demands more tuples than can be counted.
    DemandTooLarge,
    /// A module root resolved and its `manifest.yaml` could not be read.
    ManifestUnreadable,
    /// The independence assessments could not be written into the report.
    ///
    /// Unreachable by construction: the only values this crate serialises this
    /// way are `quoin_evidence`'s derived `IndependenceAssessment`s, which
    /// hold strings, unit enums and sequences — none of `serde_json`'s three
    /// failure modes (a `Serialize` that errors, a non-string map key, a
    /// non-finite float) can arise. It is a code rather than a panic because
    /// this crate forbids both, and `report_is_serialisable` in
    /// `audit/mod.rs` holds the reachability question open with a fully
    /// populated assessment.
    ReportNotSerialisable,
}

impl AuditorErrorCode {
    /// Every code, in declaration order.
    pub const ALL: [Self; 3] = [
        Self::DemandTooLarge,
        Self::ManifestUnreadable,
        Self::ReportNotSerialisable,
    ];

    /// The stable spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DemandTooLarge => "QAU-DEMAND-TOO-LARGE",
            Self::ManifestUnreadable => "QAU-MANIFEST-UNREADABLE",
            Self::ReportNotSerialisable => "QAU-REPORT-NOT-SERIALISABLE",
        }
    }

    /// The code a spelling names, if any.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == code)
    }
}

impl fmt::Display for AuditorErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One refusal, with the code a caller branches on and a sentence for a human.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct AuditorError {
    /// What was refused.
    pub code: AuditorErrorCode,
    /// Why, naming the input.
    pub message: Box<str>,
}

impl AuditorError {
    /// Build one.
    pub fn new(code: AuditorErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into().into_boxed_str(),
        }
    }

    /// A module root whose manifest could not be read.
    ///
    /// The message becomes the `reason` in
    /// [`UnreadableModule`](crate::catalog::UnreadableModule): the retained
    /// `loadMethodCatalog` catches and reports rather than throwing, because
    /// the command an operator runs *to diagnose* a module must not be the one
    /// the module takes down (agent-ix/quoin#106).
    pub fn manifest_unreadable(message: impl Into<String>) -> Self {
        Self::new(AuditorErrorCode::ManifestUnreadable, message)
    }

    /// The report's independence assessments could not be serialised.
    ///
    /// See [`AuditorErrorCode::ReportNotSerialisable`] for why no input
    /// reaches this.
    #[must_use]
    pub fn report_not_serialisable(error: &serde_json::Error) -> Self {
        Self::new(AuditorErrorCode::ReportNotSerialisable, error.to_string())
    }
}

impl From<CombinatorialError> for AuditorError {
    fn from(error: CombinatorialError) -> Self {
        Self::new(AuditorErrorCode::DemandTooLarge, error.message.to_string())
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
    use std::collections::BTreeSet;

    use super::{AuditorError, AuditorErrorCode};

    #[test]
    fn every_code_round_trips_through_its_spelling() {
        for code in AuditorErrorCode::ALL {
            assert_eq!(AuditorErrorCode::from_code(code.as_str()), Some(code));
        }
        assert_eq!(AuditorErrorCode::from_code("QAU-NOT-A-CODE"), None);
    }

    #[test]
    fn the_spellings_are_distinct() {
        let distinct: BTreeSet<&str> = AuditorErrorCode::ALL
            .into_iter()
            .map(AuditorErrorCode::as_str)
            .collect();
        assert_eq!(distinct.len(), AuditorErrorCode::ALL.len());
    }

    #[test]
    fn an_error_prints_its_code_first() {
        let error = AuditorError::manifest_unreadable("no such file");
        assert_eq!(error.to_string(), "QAU-MANIFEST-UNREADABLE: no such file");
    }
}
