// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The one error type this crate surfaces, and its stable code catalogue.
//!
//! # Why a validation failure carries a field name
//!
//! [`FieldFailure`] keeps the one piece of a refusal that classification reads,
//! which field failed and whether it was absent, undeclared or malformed, as
//! structured data, and [`ChangeAssuranceError`] answers the classification
//! questions with methods over that structure. The message text is free to be
//! whatever reads best, because nothing matches on it.

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
    /// A parent chain did not satisfy strict N-1 lineage.
    LineageInvalid,
    /// Retained output bytes did not have the declared size.
    OutputIntegrityMismatch,
    /// An identity-named path already held different content.
    ContentCollision,
    /// A stored document sat under a path naming another identity.
    PathIdentityMismatch,
    /// A stored receipt was not the receipt re-derived from its inputs.
    ReceiptMismatch,
    /// A filesystem operation failed.
    Io,
    /// A refusal raised by `quoin-store` and passed through unchanged.
    Store,
}

impl ChangeAssuranceErrorCode {
    /// Every code, in declaration order.
    pub const ALL: [Self; 11] = [
        Self::RecordInvalid,
        Self::AttestationInvalid,
        Self::ReceiptInvalid,
        Self::DecisionHistoryInvalid,
        Self::LineageInvalid,
        Self::OutputIntegrityMismatch,
        Self::ContentCollision,
        Self::PathIdentityMismatch,
        Self::ReceiptMismatch,
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
            Self::LineageInvalid => "QCA-LINEAGE-INVALID",
            Self::OutputIntegrityMismatch => "QCA-OUTPUT-INTEGRITY-MISMATCH",
            Self::ContentCollision => "QCA-CONTENT-COLLISION",
            Self::PathIdentityMismatch => "QCA-PATH-IDENTITY-MISMATCH",
            Self::ReceiptMismatch => "QCA-RECEIPT-MISMATCH",
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
/// One variant per rule the strict N-1 chain can break, so the lineage reason
/// is a `match` rather than a message comparison.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum LineageFailure {
    /// Revision 1 carried a non-empty parent chain.
    GenesisHasAParent,
    /// The parent chain was not exactly `revision - 1` long, or a parent sat
    /// at the wrong revision.
    RevisionGap,
    /// A parent carried a different `record_id`.
    CrossRecordParent,
}

impl LineageFailure {
    /// The stable description of this failure.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GenesisHasAParent => "revision 1 must have no parent chain",
            Self::RevisionGap => "revision gap in parent chain",
            Self::CrossRecordParent => "cross-record parent",
        }
    }
}

impl fmt::Display for LineageFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Which shape a validation failure belongs to.
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
    /// A shape did not satisfy its v1 schema.
    #[error("invalid {subject}: {failure}")]
    Shape {
        /// Which shape was being read.
        subject: Subject,
        /// The field, and how it failed.
        failure: FieldFailure,
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

    /// Retained output bytes did not have the declared size.
    #[error(
        "retained output size mismatch: declared {declared_size} bytes, observed {observed_size}"
    )]
    OutputIntegrity {
        /// The size the attestation declared.
        declared_size: u64,
        /// The size of the bytes supplied.
        observed_size: u64,
    },

    /// An identity-named path already held different content.
    #[error("{what} collision: {id} already holds different bytes")]
    ContentCollision {
        /// Which artifact collided.
        what: Subject,
        /// The identity naming the path.
        id: String,
    },

    /// A stored document sat under a path naming another identity.
    #[error("{what} path identity mismatch: path names {expected}, content declares {found}")]
    PathIdentityMismatch {
        /// Which artifact was read.
        what: Subject,
        /// The identity the path names.
        expected: String,
        /// The identity the content declares.
        found: String,
    },

    /// A stored receipt was not the receipt re-derived from its inputs.
    #[error("stored receipt differs from the receipt re-derived from the retained inputs")]
    ReceiptMismatch,

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
            Self::Lineage { .. } | Self::LineageParent { .. } => {
                ChangeAssuranceErrorCode::LineageInvalid
            }
            Self::OutputIntegrity { .. } => ChangeAssuranceErrorCode::OutputIntegrityMismatch,
            Self::ContentCollision { .. } => ChangeAssuranceErrorCode::ContentCollision,
            Self::PathIdentityMismatch { .. } => ChangeAssuranceErrorCode::PathIdentityMismatch,
            Self::ReceiptMismatch => ChangeAssuranceErrorCode::ReceiptMismatch,
            Self::Io { .. } => ChangeAssuranceErrorCode::Io,
            Self::Store { .. } => ChangeAssuranceErrorCode::Store,
        }
    }

    /// Build a shape refusal.
    pub(crate) fn shape(subject: Subject, failure: FieldFailure) -> Self {
        Self::Shape { subject, failure }
    }
}
