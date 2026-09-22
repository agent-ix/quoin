// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Intake: why a record is refused, and the two ways one crosses the boundary.
//!
//! A port of `intervention-types.ts:104-122`. The refusal set is the API: a
//! caller that learned `raw_evidence_mismatch` keeps it forever, which is the
//! same contract `quoin_core::error::CoreErrorCode` carries and the reason the
//! spellings are generated from one table rather than written at each use.
//!
//! quoin#471 added the intake itself: [`write_intervention_record`], the
//! write-once publication of `intervention.ts:66-82`, and
//! [`read_intervention_records`], the whole-directory read of
//! `intervention.ts:84-110`. Both raise into the vocabulary above rather than
//! into a second error type.
//!
//! # The order the checks run in is the contract
//!
//! `writeInterventionRecord` validates the record, then checks the governing
//! plan, then the retained files, and only then writes. A caller whose record
//! is both schema-invalid and points at a moved file sees `invalid_record`,
//! not `raw_evidence_mismatch`, and that ordering is reproduced exactly: it is
//! what makes the refusal a caller gets deterministic.

use std::path::{Path, PathBuf};

use quoin_store::store::write_content_addressed;
use quoin_store::{JsonValue, canonical_json_bytes, parse_strict_json};

use crate::common::wire_enum::wire_enum;
use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::intervention::ids::InterventionRecordId;
use crate::intervention::record::InterventionExperimentRecord;
use crate::intervention::validate::validate_intervention_record;
use crate::json_bridge::to_serde;
use crate::plans::{PlanLoadOptions, load_measurement_plans};
use crate::raw_evidence::{assert_governing_definition, verify_raw_evidence_references};
use crate::source::{DiskMeasurement, MeasurementSource};
use crate::store::paths::intervention_path;

wire_enum! {
    /// The reason intake refused a record. `intervention-types.ts:104-110`.
    pub enum InterventionRefusalCode {
        /// The record did not validate against the intervention schema.
        InvalidRecord => "invalid_record",
        /// A referenced raw evidence file does not match its recorded digest.
        RawEvidenceMismatch => "raw_evidence_mismatch",
        /// No governing measurement plan admits this record.
        GoverningPlanAbsent => "governing_plan_absent",
        /// The record does not match the producer definition it names.
        DefinitionMismatch => "definition_mismatch",
        /// Another intake holds the store.
        IntakeBusy => "intake_busy",
        /// A record already exists under this identity.
        RecordIdCollision => "record_id_collision",
    }
}

/// An intake refusal, with the findings that justify it.
///
/// `intervention-types.ts:112-122`. The rendered message is the TypeScript
/// one — `` `${code}: ${findings.join("; ")}` `` — because it is what a caller
/// reading stderr today already sees.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}: {}", .code.as_str(), .findings.join("; "))]
pub struct InterventionIntakeError {
    /// The refusal reason. Codes are contractual; messages are not.
    code: InterventionRefusalCode,
    /// What was found, in the order it was found.
    findings: Vec<String>,
}

impl InterventionIntakeError {
    /// Builds a refusal.
    #[must_use]
    pub fn new(code: InterventionRefusalCode, findings: Vec<String>) -> Self {
        Self { code, findings }
    }

    /// The refusal reason.
    #[must_use]
    pub fn code(&self) -> InterventionRefusalCode {
        self.code
    }

    /// What was found.
    #[must_use]
    pub fn findings(&self) -> &[String] {
        &self.findings
    }
}

impl From<MeasurementError> for InterventionIntakeError {
    /// Carry a crate refusal into the intake vocabulary.
    ///
    /// An exhaustive match rather than a catch-all: a code added to
    /// [`MeasurementErrorCode`] must be placed here deliberately, and inside
    /// the defining crate the compiler says so — `#[non_exhaustive]` binds
    /// callers, not this match. The findings travel with it, because they are the
    /// JSON pointers the caller acts on; when a refusal carries none, its
    /// rendered sentence becomes the single finding so nothing is dropped.
    fn from(error: MeasurementError) -> Self {
        let code = match error.code() {
            MeasurementErrorCode::GoverningPlanAbsent => {
                InterventionRefusalCode::GoverningPlanAbsent
            }
            MeasurementErrorCode::DefinitionMismatch => InterventionRefusalCode::DefinitionMismatch,
            MeasurementErrorCode::RawEvidenceMismatch
            | MeasurementErrorCode::RawEvidencePathUnsafe
            | MeasurementErrorCode::RawEvidenceUnavailable => {
                InterventionRefusalCode::RawEvidenceMismatch
            }
            MeasurementErrorCode::CollectionIdCollision => {
                InterventionRefusalCode::RecordIdCollision
            }
            MeasurementErrorCode::CollectionInvalid
            | MeasurementErrorCode::CollectionIdUnsafe
            | MeasurementErrorCode::CollectionUnreadable
            | MeasurementErrorCode::RecordUnreadable
            | MeasurementErrorCode::PlanInvalid
            | MeasurementErrorCode::ProfileInvalid
            | MeasurementErrorCode::DateTimeInvalid
            | MeasurementErrorCode::Yaml
            | MeasurementErrorCode::Io
            | MeasurementErrorCode::Store
            | MeasurementErrorCode::ArtifactNameUnsafe
            | MeasurementErrorCode::ArtifactUnreadable => InterventionRefusalCode::InvalidRecord,
        };
        let findings = if error.findings().is_empty() {
            vec![error.to_string()]
        } else {
            error.findings().to_vec()
        };
        Self::new(code, findings)
    }
}

/// Validate governance and retained bytes, then publish without replacement.
///
/// Ports `writeInterventionRecord` (`intervention.ts:66-82`). Returns the path
/// written, whether or not this call was the writer: republishing identical
/// bytes is idempotent, and republishing different bytes under one identity is
/// [`InterventionRefusalCode::RecordIdCollision`].
///
/// # Errors
///
/// [`InterventionRefusalCode::InvalidRecord`] from the schema and semantic
/// passes, [`InterventionRefusalCode::GoverningPlanAbsent`] and
/// [`InterventionRefusalCode::DefinitionMismatch`] from the plan check,
/// [`InterventionRefusalCode::RawEvidenceMismatch`] when a retained file no
/// longer matches its reference, and
/// [`InterventionRefusalCode::RecordIdCollision`] on a differing republication.
pub fn write_intervention_record(
    repo: &Path,
    candidate: &JsonValue,
) -> Result<PathBuf, InterventionIntakeError> {
    let record = validate_intervention_record(&to_serde(candidate)?)?;
    let source = DiskMeasurement::new(repo);
    let plans = load_measurement_plans(&source, PlanLoadOptions::default())?;
    assert_governing_definition(&plans, &record.producer.definition_version)?;
    verify_raw_evidence_references(&source, &record.raw_evidence)?;
    let record_id = InterventionRecordId::parse(record.record_id.as_str())?;
    let path = intervention_path(repo, &record_id);
    // The **caller's** value is what is written, canonicalised — not a
    // re-serialisation of the typed record. `intervention.ts:78` does the same,
    // and it is what keeps a member this crate does not model from being
    // dropped on the way to disk.
    let bytes = canonical_json_bytes(candidate).map_err(MeasurementError::from)?;
    write_content_addressed(&path, &bytes).map_err(|error| {
        let store: MeasurementError = error.into();
        match store.code() {
            MeasurementErrorCode::CollectionIdCollision => InterventionIntakeError::new(
                InterventionRefusalCode::RecordIdCollision,
                vec![format!(
                    "{}: record id already exists with different canonical bytes",
                    path.display()
                )],
            ),
            _ => store.into(),
        }
    })?;
    Ok(path)
}

/// Read every retained intervention record.
///
/// Ports `readInterventionRecords` (`intervention.ts:84-110`): a source
/// holding no interventions directory reads as an empty list, every `.json`
/// file must validate, and the result is ordered by `observed_at` then
/// `record_id`.
///
/// # Errors
///
/// [`InterventionRefusalCode::InvalidRecord`] naming the first file that could
/// not be read or did not validate.
pub fn read_intervention_records<S: MeasurementSource + ?Sized>(
    source: &S,
) -> Result<Vec<InterventionExperimentRecord>, InterventionIntakeError> {
    let mut records = Vec::new();
    for name in source.intervention_names()? {
        let record = source
            .intervention_bytes(&name)
            .map_err(InterventionIntakeError::from)
            .and_then(|bytes| {
                let stored = parse_strict_json(&bytes).map_err(MeasurementError::from)?;
                validate_intervention_record(&to_serde(&stored)?)
            })
            .map_err(|error| {
                InterventionIntakeError::new(
                    error.code(),
                    vec![format!(
                        "{name}: unreadable intervention record: {}",
                        error.findings().join("; ")
                    )],
                )
            })?;
        records.push(record);
    }
    records.sort_by(|left, right| {
        left.observed_at
            .as_str()
            .cmp(right.observed_at.as_str())
            .then_with(|| left.record_id.as_str().cmp(right.record_id.as_str()))
    });
    Ok(records)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{InterventionIntakeError, InterventionRefusalCode};

    /// The literal the TypeScript constructor writes, not a re-derivation of it.
    #[test]
    fn the_message_is_the_typescript_message() {
        let error = InterventionIntakeError::new(
            InterventionRefusalCode::RawEvidenceMismatch,
            vec![
                "a/one.json differs".to_owned(),
                "b/two.json absent".to_owned(),
            ],
        );
        assert_eq!(
            error.to_string(),
            "raw_evidence_mismatch: a/one.json differs; b/two.json absent"
        );
    }

    #[test]
    fn a_refusal_with_no_findings_still_names_its_code() {
        let error = InterventionIntakeError::new(InterventionRefusalCode::IntakeBusy, Vec::new());
        assert_eq!(error.to_string(), "intake_busy: ");
    }
}
