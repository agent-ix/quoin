// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The one error type this crate surfaces, and its stable code catalogue.
//!
//! # Why the findings are a list and not a sentence
//!
//! The retained TypeScript raises two different shapes for the same domain.
//! `src/measurement/validate.ts:7` throws a `MeasurementValidationError`
//! carrying one prose sentence, while `src/measurement/intervention-types.ts`'s
//! `InterventionIntakeError` carries a **code plus a list of JSON-pointer
//! findings** and builds its sentence from them. The code and the pointers are
//! what a caller acts on; the sentence is a rendering of them.
//!
//! So this crate keeps the structured half and renders the sentence from it.
//! Every refusal carries a [`MeasurementErrorCode`], and the refusals that name
//! specific instance locations carry those locations as
//! [`MeasurementError::findings`]. Nothing in this crate matches on message
//! text, and no consumer should.
//!
//! Codes are the API: a consumer that learned one keeps it forever. Add
//! members; never rename or repurpose one.

use std::fmt;

use quoin_store::StoreError;

/// A stable, never-reused code for each refusal this crate can produce.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum MeasurementErrorCode {
    /// A measurement collection did not satisfy the stored envelope contract.
    CollectionInvalid,
    /// A collection id was not `[A-Za-z0-9._-]+`, so it does not name a file.
    CollectionIdUnsafe,
    /// A collection id already exists on disk holding different bytes.
    CollectionIdCollision,
    /// A retained collection could not be read back as a collection.
    CollectionUnreadable,
    /// A retained intervention or operational record could not be read.
    ///
    /// The read paths raise [`crate::InterventionIntakeError`], whose codes are
    /// the *intake* vocabulary. A report that merely reads the store carries
    /// that refusal across under this code, with the intake code named in the
    /// subject and its findings kept.
    RecordUnreadable,
    /// A `MeasurementPlan` frontmatter block was present and unacceptable.
    PlanInvalid,
    /// An `AssuranceProfile` frontmatter block was present and unacceptable.
    ProfileInvalid,
    /// No active `MeasurementPlan` exists to govern a definition version.
    GoverningPlanAbsent,
    /// An observed definition version matches no active plan.
    DefinitionMismatch,
    /// A raw-evidence reference's path is not a safe store-relative path.
    RawEvidencePathUnsafe,
    /// A raw-evidence reference's size or digest disagreed with the file.
    RawEvidenceMismatch,
    /// This source has no bytes for the named retained file.
    ///
    /// Raised by [`crate::source::MemoryMeasurement`] for a path it was never
    /// given. It is deliberately *not* raised merely because the host is
    /// in-memory: since quoin#484 `quoin-store` exposes a bytes-wise sha256,
    /// so an in-memory host digests what it holds instead of declining to.
    RawEvidenceUnavailable,
    /// A value was not a date-time this crate's RFC 3339 grammar accepts.
    DateTimeInvalid,
    /// A frontmatter block was not readable YAML.
    Yaml,
    /// A filesystem operation failed.
    Io,
    /// A refusal raised by `quoin-store` and passed through unchanged.
    Store,
    /// A `verificationStack.artifacts` name is not a safe repository-relative
    /// path, so intake cannot tell whether it names a local file (PLAT-969).
    ArtifactNameUnsafe,
    /// A `verificationStack.artifacts` name resolves to an entry under the
    /// repository that cannot be digested — a directory, a symlink, an
    /// unreadable or oversized file — so its submitted digest cannot be
    /// checked (PLAT-969).
    ArtifactUnreadable,
}

impl MeasurementErrorCode {
    /// Every code, in declaration order.
    pub const ALL: [Self; 18] = [
        Self::CollectionInvalid,
        Self::CollectionIdUnsafe,
        Self::CollectionIdCollision,
        Self::CollectionUnreadable,
        Self::RecordUnreadable,
        Self::PlanInvalid,
        Self::ProfileInvalid,
        Self::GoverningPlanAbsent,
        Self::DefinitionMismatch,
        Self::RawEvidencePathUnsafe,
        Self::RawEvidenceMismatch,
        Self::RawEvidenceUnavailable,
        Self::DateTimeInvalid,
        Self::Yaml,
        Self::Io,
        Self::Store,
        Self::ArtifactNameUnsafe,
        Self::ArtifactUnreadable,
    ];

    /// The stable wire spelling of this code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CollectionInvalid => "QM-COLLECTION-INVALID",
            Self::CollectionIdUnsafe => "QM-COLLECTION-ID-UNSAFE",
            Self::CollectionIdCollision => "QM-COLLECTION-ID-COLLISION",
            Self::CollectionUnreadable => "QM-COLLECTION-UNREADABLE",
            Self::RecordUnreadable => "QM-RECORD-UNREADABLE",
            Self::PlanInvalid => "QM-PLAN-INVALID",
            Self::ProfileInvalid => "QM-PROFILE-INVALID",
            Self::GoverningPlanAbsent => "QM-GOVERNING-PLAN-ABSENT",
            Self::DefinitionMismatch => "QM-DEFINITION-MISMATCH",
            Self::RawEvidencePathUnsafe => "QM-RAW-EVIDENCE-PATH-UNSAFE",
            Self::RawEvidenceMismatch => "QM-RAW-EVIDENCE-MISMATCH",
            Self::RawEvidenceUnavailable => "QM-RAW-EVIDENCE-UNAVAILABLE",
            Self::DateTimeInvalid => "QM-DATE-TIME-INVALID",
            Self::Yaml => "QM-YAML",
            Self::Io => "QM-IO",
            Self::Store => "QM-STORE",
            Self::ArtifactNameUnsafe => "QM-ARTIFACT-NAME-UNSAFE",
            Self::ArtifactUnreadable => "QM-ARTIFACT-UNREADABLE",
        }
    }

    /// Recover a code from its stable spelling.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|known| known.as_str() == code)
    }
}

impl fmt::Display for MeasurementErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One refusal from this crate.
///
/// `subject` names what was being read — a store path, a repository-relative
/// document, a collection id — and `findings` holds the instance locations, in
/// the JSON-pointer spelling `intervention.ts:162-176` already emits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeasurementError {
    code: MeasurementErrorCode,
    subject: Box<str>,
    findings: Vec<String>,
}

impl MeasurementError {
    /// Build a refusal with no per-location findings.
    pub fn new(code: MeasurementErrorCode, subject: impl Into<String>) -> Self {
        Self {
            code,
            subject: subject.into().into_boxed_str(),
            findings: Vec::new(),
        }
    }

    /// Build a refusal carrying instance locations.
    pub fn with_findings(
        code: MeasurementErrorCode,
        subject: impl Into<String>,
        findings: Vec<String>,
    ) -> Self {
        Self {
            code,
            subject: subject.into().into_boxed_str(),
            findings,
        }
    }

    /// This refusal's stable code.
    #[must_use]
    pub const fn code(&self) -> MeasurementErrorCode {
        self.code
    }

    /// What was being read.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The instance locations this refusal names, if any.
    #[must_use]
    pub fn findings(&self) -> &[String] {
        &self.findings
    }
}

impl fmt::Display for MeasurementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.subject)?;
        if !self.findings.is_empty() {
            write!(formatter, ": {}", self.findings.join("; "))?;
        }
        Ok(())
    }
}

impl std::error::Error for MeasurementError {}

impl From<StoreError> for MeasurementError {
    fn from(source: StoreError) -> Self {
        // `ContentCollision` is the one store refusal this domain renames: it
        // is the `record_id_collision` of `store.ts:41-44`, and a caller that
        // retries on it would loop forever. Everything else passes through
        // under one code, because the store's own code is the detail.
        let code = match source {
            StoreError::ContentCollision { .. } => MeasurementErrorCode::CollectionIdCollision,
            _ => MeasurementErrorCode::Store,
        };
        Self::new(code, source.to_string())
    }
}

impl From<crate::intervention::intake::InterventionIntakeError> for MeasurementError {
    /// Carry an intake refusal into this crate's envelope.
    ///
    /// The inverse of `From<MeasurementError> for InterventionIntakeError`, and
    /// deliberately lossy in one direction only: the intake code is named in
    /// the subject rather than mapped onto a measurement code, because the six
    /// intake codes answer "why was this write refused" and there is no read
    /// path that acts on them.
    fn from(source: crate::intervention::intake::InterventionIntakeError) -> Self {
        Self::with_findings(
            MeasurementErrorCode::RecordUnreadable,
            source.code().as_str(),
            source.findings().to_vec(),
        )
    }
}

impl From<quoin_yaml::YamlError> for MeasurementError {
    fn from(source: quoin_yaml::YamlError) -> Self {
        Self::new(MeasurementErrorCode::Yaml, source.to_string())
    }
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use super::{MeasurementError, MeasurementErrorCode};

    #[test]
    fn every_code_round_trips_through_its_spelling() {
        for code in MeasurementErrorCode::ALL {
            assert_eq!(MeasurementErrorCode::from_code(code.as_str()), Some(code));
        }
    }

    #[test]
    fn no_two_codes_share_one_spelling() {
        let mut spellings: Vec<&str> = MeasurementErrorCode::ALL
            .iter()
            .map(|code| code.as_str())
            .collect();
        spellings.sort_unstable();
        let count = spellings.len();
        spellings.dedup();
        assert_eq!(spellings.len(), count);
    }

    #[test]
    fn findings_are_rendered_after_the_subject() {
        let error = MeasurementError::with_findings(
            MeasurementErrorCode::RawEvidenceMismatch,
            "records",
            vec!["/raw_evidence/0/digest: expected a, observed b".to_owned()],
        );
        assert_eq!(
            error.to_string(),
            "QM-RAW-EVIDENCE-MISMATCH: records: /raw_evidence/0/digest: expected a, observed b"
        );
    }
}
