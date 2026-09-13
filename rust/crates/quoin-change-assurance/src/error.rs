// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The one error type this crate surfaces, and its stable code catalogue.
//!
//! # Why a validation failure carries a field name
//!
//! The retained TypeScript throws bare `Error`s and then **classifies them by
//! matching their message text**: `src/change-assurance/verify.ts:32` tests
//! `/digest mismatch/i` to decide between `record_digest_mismatch` and
//! `schema_invalid`, `:44-49` tests `/missing|no parent/i`, `/revision gap/i`
//! and `/mismatch|cross-record/i` to pick a lineage reason, and `:111` tests
//! `/digest/i` to pick between `attestation_digest_mismatch` and
//! `attestation_schema_invalid`. Those verdicts are contractual; the prose is
//! not.
//!
//! So this crate does **not** reproduce the prose and re-run the regexes over
//! it. [`FieldFailure`] keeps the one piece of the message the classification
//! actually reads — which field failed, and whether it was absent, undeclared
//! or malformed — as structured data, and [`ChangeAssuranceError`] answers the
//! three classification questions with methods over that structure. The
//! message text is then free to be whatever reads best, because nothing
//! matches on it. `tests/tc_455_classification.rs` pins the mapping against
//! the oracle's own verdicts.

use std::fmt;

use quoin_store::StoreError;

/// A stable, never-reused code for each refusal this crate can produce.
///
/// Codes are the API: a consumer that learned one keeps it forever. Add
/// members; never rename or repurpose one.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum ChangeAssuranceErrorCode {
    /// A change-assurance record did not satisfy the v1 schema.
    RecordInvalid,
    /// A proof attestation did not satisfy the v1 schema.
    AttestationInvalid,
    /// A verification receipt did not satisfy the v1 schema.
    ReceiptInvalid,
    /// A decision history or ix-flow event did not satisfy its shape.
    DecisionHistoryInvalid,
    /// A `digest` member was not 64 lowercase hexadecimal characters.
    DigestMalformed,
    /// A sealed value's stored digest disagreed with the recomputed one.
    DigestMismatch,
    /// A parent chain did not satisfy strict N-1 lineage.
    LineageInvalid,
    /// Retained output bytes did not reproduce the declared digest or size.
    OutputIntegrityMismatch,
    /// A digest-named path already held different content.
    ContentCollision,
    /// A record or attestation read back under a path whose digest it did not
    /// carry.
    PathDigestMismatch,
    /// A filesystem operation failed.
    Io,
    /// A refusal raised by `quoin-store` and passed through unchanged.
    Store,
}

impl ChangeAssuranceErrorCode {
    /// Every code, in declaration order.
    pub const ALL: [Self; 12] = [
        Self::RecordInvalid,
        Self::AttestationInvalid,
        Self::ReceiptInvalid,
        Self::DecisionHistoryInvalid,
        Self::DigestMalformed,
        Self::DigestMismatch,
        Self::LineageInvalid,
        Self::OutputIntegrityMismatch,
        Self::ContentCollision,
        Self::PathDigestMismatch,
        Self::Io,
        Self::Store,
    ];

    /// The stable wire spelling of this code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RecordInvalid => "QCA-RECORD-INVALID",
            Self::AttestationInvalid => "QCA-ATTESTATION-INVALID",
            Self::ReceiptInvalid => "QCA-RECEIPT-INVALID",
            Self::DecisionHistoryInvalid => "QCA-DECISION-HISTORY-INVALID",
            Self::DigestMalformed => "QCA-DIGEST-MALFORMED",
            Self::DigestMismatch => "QCA-DIGEST-MISMATCH",
            Self::LineageInvalid => "QCA-LINEAGE-INVALID",
            Self::OutputIntegrityMismatch => "QCA-OUTPUT-INTEGRITY-MISMATCH",
            Self::ContentCollision => "QCA-CONTENT-COLLISION",
            Self::PathDigestMismatch => "QCA-PATH-DIGEST-MISMATCH",
            Self::Io => "QCA-IO",
            Self::Store => "QCA-STORE",
        }
    }

    /// Recover a code from its stable spelling.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == code)
    }
}

impl fmt::Display for ChangeAssuranceErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Which of the three things went wrong with one field.
///
/// The oracle expresses all three as sentences beginning `missing field `,
/// `extra field ` and the field's own name; the distinction is what its
/// classification regexes actually read, so it is a variant here.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum FieldFailure {
    /// A required member was absent. The oracle's `missing field <name>`.
    Missing {
        /// The absent member's name.
        field: String,
    },
    /// An undeclared member was present. The oracle's `extra field <name>`.
    Extra {
        /// The undeclared member's name.
        field: String,
    },
    /// A member was present and did not satisfy its contract. The oracle
    /// names the field, or the constraint, in its message.
    Malformed {
        /// The field path, or the constraint, that failed.
        field: String,
    },
}

impl FieldFailure {
    /// A required member was absent.
    pub fn missing(field: impl Into<String>) -> Self {
        Self::Missing {
            field: field.into(),
        }
    }

    /// An undeclared member was present.
    pub fn extra(field: impl Into<String>) -> Self {
        Self::Extra {
            field: field.into(),
        }
    }

    /// A member was present and unacceptable.
    pub fn malformed(field: impl Into<String>) -> Self {
        Self::Malformed {
            field: field.into(),
        }
    }

    /// The field path this failure names.
    #[must_use]
    pub fn field(&self) -> &str {
        match self {
            Self::Missing { field } | Self::Extra { field } | Self::Malformed { field } => field,
        }
    }

    /// Whether the failure concerns a digest-bearing member.
    ///
    /// Reproduces `verify.ts:111`'s `/digest/i` over the oracle's message.
    /// Every one of those messages carries the failing field's name and
    /// nothing else that could contain `digest`, so the name is the whole
    /// test.
    #[must_use]
    pub fn concerns_a_digest(&self) -> bool {
        self.field().to_ascii_lowercase().contains("digest")
    }

    /// Whether the failure reads as an absence.
    ///
    /// Reproduces the `missing` half of `verify.ts:44`'s
    /// `/missing|no parent/i`. Only the oracle's `missing field <name>` shape
    /// matches; no field name in the schema contains `missing`.
    #[must_use]
    pub const fn reads_as_absence(&self) -> bool {
        matches!(self, Self::Missing { .. })
    }
}

impl fmt::Display for FieldFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { field } => write!(formatter, "missing field {field}"),
            Self::Extra { field } => write!(formatter, "extra field {field}"),
            Self::Malformed { field } => write!(formatter, "{field}"),
        }
    }
}

/// Why a parent chain was refused.
///
/// One variant per message `verifyLineage` (`records.ts:43-71`) can throw,
/// so the lineage reason is a `match` rather than a regex.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum LineageFailure {
    /// Revision 1 carried a parent digest or a non-empty parent chain.
    GenesisHasAParent,
    /// The parent chain was not exactly `revision - 1` long, or a parent sat
    /// at the wrong revision.
    RevisionGap,
    /// A parent carried a different `record_id`.
    CrossRecordParent,
    /// A parent's own `parent_digest` was not its predecessor's digest.
    ParentDigestMismatch,
    /// The record's `parent_digest` was not the chain tail's digest.
    ImmediateParentMismatch,
}

impl LineageFailure {
    /// The stable description of this failure.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GenesisHasAParent => "revision 1 must have a null parent and no parent chain",
            Self::RevisionGap => "revision gap in parent chain",
            Self::CrossRecordParent => "cross-record parent",
            Self::ParentDigestMismatch => "parent digest mismatch",
            Self::ImmediateParentMismatch => "immediate parent mismatch",
        }
    }
}

impl fmt::Display for LineageFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Which sealed shape a validation failure belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Subject {
    /// A change-assurance record.
    Record,
    /// A proof attestation.
    Attestation,
    /// A verification receipt.
    Receipt,
    /// A retained decision history or one of its ix-flow events.
    DecisionHistory,
}

impl Subject {
    /// The stable name of this shape.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Record => "change assurance record",
            Self::Attestation => "proof attestation",
            Self::Receipt => "verification receipt",
            Self::DecisionHistory => "decision history",
        }
    }
}

impl fmt::Display for Subject {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Everything this crate can refuse, as one enum.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ChangeAssuranceError {
    /// A sealed shape did not satisfy its v1 schema.
    #[error("invalid {subject}: {failure}")]
    Shape {
        /// Which shape was being read.
        subject: Subject,
        /// The field, and how it failed.
        failure: FieldFailure,
    },

    /// A `digest` member was not 64 lowercase hexadecimal characters.
    #[error("digest must be exactly 64 lowercase hexadecimal characters")]
    DigestMalformed {
        /// Which shape carried it.
        subject: Subject,
    },

    /// A sealed value's stored digest disagreed with the recomputed one.
    #[error("digest mismatch: stored {stored}, recomputed {recomputed}")]
    DigestMismatch {
        /// Which shape carried it.
        subject: Subject,
        /// The digest the value declared.
        stored: String,
        /// The digest its bytes actually produce.
        recomputed: String,
    },

    /// A parent chain did not satisfy strict N-1 lineage.
    #[error("invalid lineage: {failure}")]
    Lineage {
        /// Which rule the chain broke.
        failure: LineageFailure,
    },

    /// A parent in the chain was itself unreadable as a record.
    #[error("invalid lineage: parent {index} is not a valid record: {source}")]
    LineageParent {
        /// Position of the offending parent in revision order, from zero.
        index: usize,
        /// Why the parent was refused.
        source: Box<ChangeAssuranceError>,
    },

    /// Retained output bytes did not reproduce the declared digest or size.
    #[error(
        "retained output integrity mismatch: declared {declared_size} bytes / {declared_digest}, \
         observed {observed_size} bytes / {observed_digest}"
    )]
    OutputIntegrity {
        /// The size the attestation declared.
        declared_size: u64,
        /// The digest the attestation declared.
        declared_digest: String,
        /// The size of the bytes supplied.
        observed_size: u64,
        /// The digest of the bytes supplied.
        observed_digest: String,
    },

    /// A digest-named path already held different content.
    #[error("{what} digest collision: {digest} already holds different bytes")]
    ContentCollision {
        /// Which artifact collided.
        what: Subject,
        /// The digest naming the path.
        digest: String,
    },

    /// A record or attestation read back under a path whose digest it did not
    /// carry.
    #[error("{what} path digest mismatch: path names {expected}, content declares {found}")]
    PathDigestMismatch {
        /// Which artifact was read.
        what: Subject,
        /// The digest the path names.
        expected: String,
        /// The digest the content declares.
        found: String,
    },

    /// A filesystem operation failed.
    #[error("{operation} failed for {path}")]
    Io {
        /// What was being attempted.
        operation: &'static str,
        /// The path involved.
        path: std::path::PathBuf,
        /// The underlying failure.
        #[source]
        source: std::io::Error,
    },

    /// A refusal raised by `quoin-store` and passed through unchanged.
    #[error(transparent)]
    Store {
        /// The underlying refusal.
        #[from]
        source: StoreError,
    },
}

impl ChangeAssuranceError {
    /// The stable code for this refusal.
    #[must_use]
    pub const fn code(&self) -> ChangeAssuranceErrorCode {
        match self {
            Self::Shape { subject, .. } => match subject {
                Subject::Record => ChangeAssuranceErrorCode::RecordInvalid,
                Subject::Attestation => ChangeAssuranceErrorCode::AttestationInvalid,
                Subject::Receipt => ChangeAssuranceErrorCode::ReceiptInvalid,
                Subject::DecisionHistory => ChangeAssuranceErrorCode::DecisionHistoryInvalid,
            },
            Self::DigestMalformed { .. } => ChangeAssuranceErrorCode::DigestMalformed,
            Self::DigestMismatch { .. } => ChangeAssuranceErrorCode::DigestMismatch,
            Self::Lineage { .. } | Self::LineageParent { .. } => {
                ChangeAssuranceErrorCode::LineageInvalid
            }
            Self::OutputIntegrity { .. } => ChangeAssuranceErrorCode::OutputIntegrityMismatch,
            Self::ContentCollision { .. } => ChangeAssuranceErrorCode::ContentCollision,
            Self::PathDigestMismatch { .. } => ChangeAssuranceErrorCode::PathDigestMismatch,
            Self::Io { .. } => ChangeAssuranceErrorCode::Io,
            Self::Store { .. } => ChangeAssuranceErrorCode::Store,
        }
    }

    /// Build a shape refusal.
    pub(crate) fn shape(subject: Subject, failure: FieldFailure) -> Self {
        Self::Shape { subject, failure }
    }

    /// Whether this refusal is the one `verify.ts:32`'s `/digest mismatch/i`
    /// selects — the record's own digest disagreeing with its bytes, and
    /// nothing else.
    ///
    /// A malformed digest is deliberately **not** this: the oracle's
    /// `digest must be exactly 64 lowercase hexadecimal characters` does not
    /// contain the words `digest mismatch`, so it falls through to the
    /// schema-invalid receipt.
    #[must_use]
    pub const fn is_digest_mismatch(&self) -> bool {
        matches!(self, Self::DigestMismatch { .. })
    }

    /// Whether this refusal is one `verify.ts:111`'s `/digest/i` selects,
    /// choosing `attestation_digest_mismatch` over
    /// `attestation_schema_invalid`.
    #[must_use]
    pub fn concerns_a_digest(&self) -> bool {
        match self {
            Self::Shape { failure, .. } => failure.concerns_a_digest(),
            Self::DigestMalformed { .. } | Self::DigestMismatch { .. } => true,
            _ => false,
        }
    }
}
