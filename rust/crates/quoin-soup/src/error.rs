// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Typed errors at the shared SOUP library boundary (FR-144, FR-197).

use quoin_core::protocol::Diagnostic;
use std::collections::BTreeMap;

/// Stable failure categories returned by the shared SOUP library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoupErrorCode {
    /// The supplied matches select no supported SOUP command.
    InvalidCommand,
    /// A source could not be read or the host operation failed.
    Operational,
    /// A source returned a coded refusal; its diagnostics remain available.
    SourceRefused,
}

impl SoupErrorCode {
    /// The stable spelling of this failure category.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidCommand => "SOUP_INVALID_COMMAND",
            Self::Operational => "SOUP_OPERATIONAL",
            Self::SourceRefused => "SOUP_SOURCE_REFUSED",
        }
    }

    /// Every category this library emits.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[Self::InvalidCommand, Self::Operational, Self::SourceRefused]
    }

    /// Resolve a stable code without inspecting a human message.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|item| item.as_str() == code)
    }
}

/// A shared-library failure with its original source diagnostics.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{code}: {message}", code = .code.as_str())]
pub struct SoupError {
    code: SoupErrorCode,
    message: Box<str>,
    context: BTreeMap<String, String>,
    source_diagnostics: Vec<Diagnostic>,
}

impl SoupError {
    pub(crate) fn new(code: SoupErrorCode, message: impl Into<Box<str>>) -> Self {
        Self {
            code,
            message: message.into(),
            context: BTreeMap::new(),
            source_diagnostics: Vec::new(),
        }
    }

    pub(crate) fn with_context(mut self, key: &str, value: impl Into<String>) -> Self {
        self.context.insert(key.to_owned(), value.into());
        self
    }

    pub(crate) fn refused(diagnostics: Vec<Diagnostic>) -> Self {
        let message = diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .collect::<Vec<_>>()
            .join("; ");
        let mut error = Self::new(SoupErrorCode::SourceRefused, message);
        if let Some(first) = diagnostics.first() {
            error = error.with_context("source_code", &first.code);
        }
        error.source_diagnostics = diagnostics;
        error
    }

    /// The stable category, independent of the displayed message.
    #[must_use]
    pub const fn code(&self) -> SoupErrorCode {
        self.code
    }

    /// The human-readable source message, preserving the CLI's wording.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Deterministic context attached to this failure.
    #[must_use]
    pub const fn context(&self) -> &BTreeMap<String, String> {
        &self.context
    }

    /// Original coded source diagnostics, including their individual contexts.
    #[must_use]
    pub fn source_diagnostics(&self) -> &[Diagnostic] {
        &self.source_diagnostics
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("FR-197")]
    #[test]
    fn stable_codes_round_trip_and_source_refusals_keep_their_context() {
        let spellings: Vec<_> = SoupErrorCode::all()
            .iter()
            .map(|code| code.as_str())
            .collect();
        assert_eq!(
            spellings,
            [
                "SOUP_INVALID_COMMAND",
                "SOUP_OPERATIONAL",
                "SOUP_SOURCE_REFUSED"
            ]
        );
        for code in SoupErrorCode::all() {
            assert_eq!(SoupErrorCode::from_code(code.as_str()), Some(*code));
        }
        assert_eq!(SoupErrorCode::from_code("unknown"), None);
        let error = SoupError::refused(vec![Diagnostic {
            code: "soup.record_field_invalid".to_owned(),
            message: "invalid version".to_owned(),
            context: [("field".to_owned(), "version".to_owned())].into(),
        }]);
        assert_eq!(error.code(), SoupErrorCode::SourceRefused);
        assert_eq!(
            error.message(),
            "soup.record_field_invalid: invalid version"
        );
        assert_eq!(error.context()["source_code"], "soup.record_field_invalid");
        assert_eq!(error.source_diagnostics()[0].context["field"], "version");
    }
}
