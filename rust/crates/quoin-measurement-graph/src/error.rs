// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The one error this crate surfaces, and its stable code catalogue.
//!
//! Ports `GraphAdapterError` and `GraphAdapterErrorCode`
//! (`src/measurement/graph-adapters.ts:24-40`) and
//! `GraphPortfolioMappingError` / `GraphPortfolioMappingErrorCode`
//! (`src/measurement/graph-portfolio.ts:66-82`).
//!
//! # One enum, two retained classes
//!
//! The retained tree raises two error classes at this boundary — the adapters'
//! eight codes and the governed portfolio's four. They are refusals of one
//! crate, so there is **one** catalogue here rather than two: the variant set
//! is the API, and a caller that wants to know which half refused reads the
//! code, not the class (quoin#475, quoin#476).
//!
//! # Two spellings, and why both are kept
//!
//! The retained class embeds a lowercase code in the message it throws —
//! `` `${code}: ${message}` `` — and callers of the retained module read that
//! spelling. This workspace's own catalogues are `QM-`/`QQ-` screaming codes
//! ([`quoin_measurement::MeasurementErrorCode`] is the model). Renaming either
//! would break a consumer, so both are stated: [`GraphAdapterErrorCode::as_str`]
//! is this crate's code and [`GraphAdapterErrorCode::retained_spelling`] is the
//! retained one, `None` for the codes the port added.
//!
//! The retained spelling is not decoration. `tests/tc_475_parity.rs` asserts
//! it against the code the TypeScript oracle actually threw on each corpus
//! entry, so a refusal here that classifies differently from the retained
//! refusal is a failure rather than a nuance.
//!
//! **Codes are the API. Never rename one, never reuse one.**

use std::fmt;

/// A stable, never-reused code for each refusal this crate can produce.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum GraphAdapterErrorCode {
    /// A name that is not one of [`crate::GRAPH_ADAPTER_NAMES`].
    UnknownAdapter,
    /// An assurance export did not satisfy the contract, or disagreed with the
    /// premises the caller accepted it under.
    InvalidPremise,
    /// A producer observation did not satisfy `graph-quality-observation-v1`.
    InvalidObservation,
    /// An invocation attestation did not satisfy its contract.
    InvalidAttestation,
    /// The scorer attachment's bytes or media type were not supplied.
    AttachmentMissing,
    /// The scorer attachment's digest disagreed with the record's.
    AttachmentDigestMismatch,
    /// No single active plan governs the observed definition version.
    InactivePlan,
    /// Two producer facts map to one partition, so one would overwrite the other.
    DuplicatePartition,
    /// A retained `bytesBase64` attachment is not strict RFC 4648 §4 base64.
    ///
    /// Port-only, and a declared divergence: Node's `Buffer.from(x, "base64")`
    /// silently discards characters outside the alphabet, so it accepts input
    /// this refuses (quoin#465). See `tests/tc_475_parity.rs`.
    AttachmentEncodingInvalid,
    /// The transcribed collection was refused by the measurement validator.
    ///
    /// Port-only: the retained adapter lets `MeasurementValidationError`
    /// propagate out of `validateMeasurementCollection` unchanged, and a
    /// distinct code here keeps that refusal distinguishable from this crate's
    /// own.
    CollectionRefused,
    /// A `<repository>=<value>` mapping was malformed, or named a repository
    /// that is not in the portfolio. `graph-portfolio.ts:66`.
    InvalidRepositoryMapping,
    /// One repository was mapped to two different graph exports.
    DuplicateGraphExport,
    /// One repository was mapped to two different premises documents.
    DuplicateGraphPremises,
    /// One repository was mapped to two different audit documents.
    DuplicateGraphAudit,
    /// A refusal raised by `quoin-measurement` and passed through unchanged.
    ///
    /// Port-only: the governed portfolio renders the ungoverned portfolio
    /// report (`graph-portfolio.ts:335`) and crosses the JSON bridge, and both
    /// belong to that crate.
    Measurement,
    /// A refusal raised by `quoin-store` and passed through unchanged.
    Store,
}

impl GraphAdapterErrorCode {
    /// Every code, in declaration order.
    pub const ALL: [Self; 16] = [
        Self::UnknownAdapter,
        Self::InvalidPremise,
        Self::InvalidObservation,
        Self::InvalidAttestation,
        Self::AttachmentMissing,
        Self::AttachmentDigestMismatch,
        Self::InactivePlan,
        Self::DuplicatePartition,
        Self::AttachmentEncodingInvalid,
        Self::CollectionRefused,
        Self::InvalidRepositoryMapping,
        Self::DuplicateGraphExport,
        Self::DuplicateGraphPremises,
        Self::DuplicateGraphAudit,
        Self::Measurement,
        Self::Store,
    ];

    /// The stable wire spelling of this code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnknownAdapter => "QMG-UNKNOWN-ADAPTER",
            Self::InvalidPremise => "QMG-INVALID-PREMISE",
            Self::InvalidObservation => "QMG-INVALID-OBSERVATION",
            Self::InvalidAttestation => "QMG-INVALID-ATTESTATION",
            Self::AttachmentMissing => "QMG-ATTACHMENT-MISSING",
            Self::AttachmentDigestMismatch => "QMG-ATTACHMENT-DIGEST-MISMATCH",
            Self::InactivePlan => "QMG-INACTIVE-PLAN",
            Self::DuplicatePartition => "QMG-DUPLICATE-PARTITION",
            Self::AttachmentEncodingInvalid => "QMG-ATTACHMENT-ENCODING-INVALID",
            Self::CollectionRefused => "QMG-COLLECTION-REFUSED",
            Self::InvalidRepositoryMapping => "QMG-INVALID-REPOSITORY-MAPPING",
            Self::DuplicateGraphExport => "QMG-DUPLICATE-GRAPH-EXPORT",
            Self::DuplicateGraphPremises => "QMG-DUPLICATE-GRAPH-PREMISES",
            Self::DuplicateGraphAudit => "QMG-DUPLICATE-GRAPH-AUDIT",
            Self::Measurement => "QMG-MEASUREMENT",
            Self::Store => "QMG-STORE",
        }
    }

    /// The spelling the retained `GraphAdapterErrorCode` union uses, when the
    /// retained module can raise this condition at all.
    #[must_use]
    pub const fn retained_spelling(self) -> Option<&'static str> {
        match self {
            Self::UnknownAdapter => Some("unknown_adapter"),
            Self::InvalidPremise => Some("invalid_premise"),
            Self::InvalidObservation => Some("invalid_observation"),
            Self::InvalidAttestation => Some("invalid_attestation"),
            Self::AttachmentMissing => Some("attachment_missing"),
            Self::AttachmentDigestMismatch => Some("attachment_digest_mismatch"),
            Self::InactivePlan => Some("inactive_plan"),
            Self::DuplicatePartition => Some("duplicate_partition"),
            Self::InvalidRepositoryMapping => Some("invalid_repository_mapping"),
            Self::DuplicateGraphExport => Some("duplicate_graph_export"),
            Self::DuplicateGraphPremises => Some("duplicate_graph_premises"),
            Self::DuplicateGraphAudit => Some("duplicate_graph_audit"),
            Self::AttachmentEncodingInvalid
            | Self::CollectionRefused
            | Self::Measurement
            | Self::Store => None,
        }
    }

    /// Recover a code from its stable spelling.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == code)
    }
}

impl fmt::Display for GraphAdapterErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One refusal, carrying the code a caller acts on and the sentence it renders.
///
/// The message is `Box<str>` because it is immutable once built, which is the
/// shape `quoin_core::error` established for this workspace.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct GraphAdapterError {
    code: GraphAdapterErrorCode,
    message: Box<str>,
}

impl GraphAdapterError {
    /// Build a refusal.
    #[must_use]
    pub fn new(code: GraphAdapterErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into().into_boxed_str(),
        }
    }

    /// The stable code.
    #[must_use]
    pub const fn code(&self) -> GraphAdapterErrorCode {
        self.code
    }

    /// The rendered sentence. Never matched on; read by humans.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<quoin_measurement::MeasurementError> for GraphAdapterError {
    fn from(source: quoin_measurement::MeasurementError) -> Self {
        Self::new(GraphAdapterErrorCode::Measurement, source.to_string())
    }
}

impl From<quoin_store::StoreError> for GraphAdapterError {
    fn from(source: quoin_store::StoreError) -> Self {
        Self::new(GraphAdapterErrorCode::Store, source.to_string())
    }
}

/// What this crate returns.
pub type Result<T> = std::result::Result<T, GraphAdapterError>;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::GraphAdapterErrorCode;
    use std::collections::BTreeSet;

    /// Every code round-trips through its spelling, and no two share one.
    #[test]
    fn tc_475_001_codes_are_distinct_and_round_trip() {
        let mut seen = BTreeSet::new();
        for code in GraphAdapterErrorCode::ALL {
            assert!(seen.insert(code.as_str()), "{code} is spelled twice");
            assert_eq!(GraphAdapterErrorCode::from_code(code.as_str()), Some(code));
        }
        assert_eq!(seen.len(), GraphAdapterErrorCode::ALL.len());
    }

    /// The twelve retained spellings are distinct and are the retained union
    /// of both classes this crate replaces.
    #[test]
    fn tc_475_002_the_retained_spellings_are_the_retained_union() {
        let retained: BTreeSet<&str> = GraphAdapterErrorCode::ALL
            .into_iter()
            .filter_map(GraphAdapterErrorCode::retained_spelling)
            .collect();
        assert_eq!(
            retained,
            BTreeSet::from([
                "attachment_digest_mismatch",
                "attachment_missing",
                "duplicate_partition",
                "inactive_plan",
                "invalid_attestation",
                "invalid_observation",
                "invalid_premise",
                "unknown_adapter",
                // graph-portfolio.ts:66-70, the second retained class.
                "duplicate_graph_audit",
                "duplicate_graph_export",
                "duplicate_graph_premises",
                "invalid_repository_mapping",
            ]),
            "graph-adapters.ts:25-33 declares eight and graph-portfolio.ts:66-70 four"
        );
    }
}
