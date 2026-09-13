// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The crate's one error envelope, and the stable codes that are its API.
//!
//! Modelled on `quoin_validators::error` and `quoin_core::error`: a `Copy` code
//! enum whose spellings are contractual, plus a `thiserror` enum carrying typed
//! fields rather than pre-formatted prose.
//!
//! **Codes are the API. Never rename one, never reuse one.** A consumer that
//! learned `QE-E001` keeps it forever.
//!
//! The `Display` text of the adapter variants reproduces the retained
//! TypeScript's `AdapterError` exactly — `` `${adapter}: ${message}` `` — because
//! `quoin evidence record` prints it and the store's users read it.

use std::fmt;

/// Every condition a caller of this crate must be able to distinguish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum EvidenceErrorCode {
    /// An adapter could not read the producer document it was handed.
    AdapterInput,
    /// An explicit `--adapter` named an adapter that does not exist.
    AdapterUnknown,
}

impl EvidenceErrorCode {
    /// Every code, in declaration order. The population a catalogue test walks.
    pub const ALL: &'static [Self] = &[Self::AdapterInput, Self::AdapterUnknown];

    /// The code's contractual spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdapterInput => "QE-E001",
            Self::AdapterUnknown => "QE-E002",
        }
    }

    /// The code with this spelling, or `None`.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|c| c.as_str() == code)
    }
}

impl fmt::Display for EvidenceErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What this crate refuses, and why.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EvidenceError {
    /// An adapter refused its input.
    ///
    /// The rendered text is `<adapter>: <message>`, which is byte-for-byte what
    /// the retained `AdapterError` produced.
    #[error("{adapter}: {message}")]
    Adapter {
        /// The adapter's registry name, or `adapter` for a selection failure.
        adapter: String,
        /// The refusal, without the adapter prefix.
        message: String,
    },
    /// `--adapter` named something the registry does not hold.
    ///
    /// Its own code because the fix is a typo in the invocation, not in the
    /// producer document — falling through to the default adapter would send
    /// the reader to look at their `JUnit` file instead of at their command line.
    #[error("adapter: unknown adapter '{name}'. Available: {available}")]
    UnknownAdapter {
        /// The name as given.
        name: String,
        /// Every registered name, comma separated, in `--help` order.
        available: String,
    },
}

impl EvidenceError {
    /// The stable code for this refusal.
    #[must_use]
    pub const fn code(&self) -> EvidenceErrorCode {
        match self {
            Self::Adapter { .. } => EvidenceErrorCode::AdapterInput,
            Self::UnknownAdapter { .. } => EvidenceErrorCode::AdapterUnknown,
        }
    }

    /// An adapter refusal, built the way every adapter builds one.
    pub(crate) fn adapter(adapter: &str, message: impl Into<String>) -> Self {
        Self::Adapter {
            adapter: adapter.to_owned(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{EvidenceError, EvidenceErrorCode};
    use std::collections::BTreeSet;

    #[test]
    fn every_code_round_trips_through_its_spelling() {
        for code in EvidenceErrorCode::ALL {
            assert_eq!(EvidenceErrorCode::from_code(code.as_str()), Some(*code));
        }
    }

    #[test]
    fn no_two_codes_share_one_spelling() {
        let spellings: BTreeSet<&str> = EvidenceErrorCode::ALL.iter().map(|c| c.as_str()).collect();
        assert_eq!(spellings.len(), EvidenceErrorCode::ALL.len());
        assert!(!EvidenceErrorCode::ALL.is_empty());
    }

    #[test]
    fn an_adapter_refusal_renders_the_typescript_text() {
        let error = EvidenceError::adapter("junit", "no <testcase> elements found");
        assert_eq!(error.to_string(), "junit: no <testcase> elements found");
        assert_eq!(error.code(), EvidenceErrorCode::AdapterInput);
    }
}
