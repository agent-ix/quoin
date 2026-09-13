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
    /// A store file exists and cannot be read as JSON.
    StoreRead,
    /// A store path could not be read, written, listed or removed.
    StoreIo,
    /// An identity does not have the shape its position requires.
    InvalidIdentity,
    /// A machine-written record does not satisfy its schema.
    InvalidRecord,
    /// A policy names an obligation the corpus does not derive.
    PolicyUnknownObligation,
    /// A content-addressed record disagrees with its own identity.
    RecordIntegrity,
    /// A value has no canonical spelling, so it has no stored form.
    Canonicalization,
}

impl EvidenceErrorCode {
    /// Every code, in declaration order. The population a catalogue test walks.
    pub const ALL: &'static [Self] = &[
        Self::AdapterInput,
        Self::AdapterUnknown,
        Self::StoreRead,
        Self::StoreIo,
        Self::InvalidIdentity,
        Self::InvalidRecord,
        Self::PolicyUnknownObligation,
        Self::RecordIntegrity,
        Self::Canonicalization,
    ];

    /// The code's contractual spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdapterInput => "QE-E001",
            Self::AdapterUnknown => "QE-E002",
            Self::StoreRead => "QE-E003",
            Self::StoreIo => "QE-E004",
            Self::InvalidIdentity => "QE-E005",
            Self::InvalidRecord => "QE-E006",
            Self::PolicyUnknownObligation => "QE-E007",
            Self::RecordIntegrity => "QE-E008",
            Self::Canonicalization => "QE-E009",
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
    /// A store file exists and is not readable JSON.
    ///
    /// `bindings.json` and `baseline.json` are **checked into git**, so a merge
    /// conflict leaves `<<<<<<< HEAD` in one of them. The retained
    /// `StoreReadError` exists because every store read used to throw a bare
    /// `SyntaxError` naming no file (agent-ix/quoin#106); the sentence is
    /// reproduced here for the same reason.
    #[error(
        "{path} exists but is not readable JSON: {detail}. A merge conflict in a checked-in store file is the usual cause — resolve it, or delete the file to start from an empty store."
    )]
    StoreRead {
        /// The store-relative path, as the reader named it.
        path: String,
        /// The parser's own words.
        detail: String,
    },
    /// A store path could not be read, written, listed or removed.
    #[error("cannot {operation} {path}: {detail}")]
    StoreIo {
        /// What was being attempted: `read`, `write`, `list`, `remove`.
        operation: &'static str,
        /// The store-relative path.
        path: String,
        /// The underlying refusal, rendered.
        detail: String,
    },
    /// A trust decision id is not `ETD-<digits>`.
    #[error("invalid trust decision id '{id}'")]
    InvalidTrustDecisionId {
        /// The id as given.
        id: String,
    },
    /// A profile id is not `AP-<digits>`.
    #[error("invalid profile id '{id}'")]
    InvalidProfileId {
        /// The id as given.
        id: String,
    },
    /// A machine-written record does not satisfy its schema.
    ///
    /// `label` names the record class and `detail` is the accumulated
    /// field-level complaint, joined with `; ` — the shape the retained zod
    /// boundary rendered, so the operator-facing text is unchanged.
    #[error("invalid {label}: {detail}")]
    InvalidRecord {
        /// The record class, for example `trust decision`.
        label: &'static str,
        /// One or more `field: complaint` clauses, joined with `; `.
        detail: String,
    },
    /// An independence policy names obligations the corpus does not derive.
    ///
    /// Refused rather than silently evaluated over nothing: a policy whose
    /// obligation was renamed would otherwise report every requirement as
    /// vacuously assessed.
    #[error("independence policy names unknown obligation(s): {obligations}")]
    PolicyUnknownObligations {
        /// The unknown obligation ids, sorted, comma separated.
        obligations: String,
    },
    /// A content-addressed record disagrees with its own identity.
    #[error("{what} at {path}{note}")]
    RecordIntegrity {
        /// Which disagreement: the id, the digest, or the stored bytes.
        what: &'static str,
        /// The path holding the record.
        path: String,
        /// What the store did about it, when saying so is load-bearing.
        ///
        /// Empty for most refusals. A collision carries
        /// `"; existing bytes were not overwritten"`, because the first
        /// question an operator asks on seeing it is whether the record they
        /// had is still there.
        note: &'static str,
    },
    /// A value has no canonical spelling, so it has no stored form.
    #[error("cannot canonicalize {what}: {detail}")]
    Canonicalization {
        /// What was being serialized.
        what: &'static str,
        /// `quoin-store`'s own words.
        detail: String,
    },
}

impl EvidenceError {
    /// The stable code for this refusal.
    #[must_use]
    pub const fn code(&self) -> EvidenceErrorCode {
        match self {
            Self::Adapter { .. } => EvidenceErrorCode::AdapterInput,
            Self::UnknownAdapter { .. } => EvidenceErrorCode::AdapterUnknown,
            Self::StoreRead { .. } => EvidenceErrorCode::StoreRead,
            Self::StoreIo { .. } => EvidenceErrorCode::StoreIo,
            Self::InvalidTrustDecisionId { .. } | Self::InvalidProfileId { .. } => {
                EvidenceErrorCode::InvalidIdentity
            }
            Self::InvalidRecord { .. } => EvidenceErrorCode::InvalidRecord,
            Self::PolicyUnknownObligations { .. } => EvidenceErrorCode::PolicyUnknownObligation,
            Self::RecordIntegrity { .. } => EvidenceErrorCode::RecordIntegrity,
            Self::Canonicalization { .. } => EvidenceErrorCode::Canonicalization,
        }
    }

    /// An adapter refusal, built the way every adapter builds one.
    pub(crate) fn adapter(adapter: &str, message: impl Into<String>) -> Self {
        Self::Adapter {
            adapter: adapter.to_owned(),
            message: message.into(),
        }
    }

    /// A schema refusal, with its clauses already joined.
    pub(crate) fn invalid(label: &'static str, detail: impl Into<String>) -> Self {
        Self::InvalidRecord {
            label,
            detail: detail.into(),
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
