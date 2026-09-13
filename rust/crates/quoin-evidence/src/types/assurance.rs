// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Content-addressed, append-only experiment and operational records (FR-048).
//!
//! These carry their own producer-provenance contract and do not alter the
//! FR-030 run records: the accepted provenance boundary leaves those unchanged
//! and binds richer provenance only where the record's declared schema has it.

use serde::{Deserialize, Serialize};

/// One artifact a producer emitted, with its digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceArtifact {
    /// The artifact's name.
    pub name: String,
    /// `sha256:<64 lowercase hex>`.
    pub digest: String,
}

/// Who produced a record, from what source, with what capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProducerProvenance {
    /// Always `producer-provenance-v1`.
    pub schema_version: String,
    /// The producer's identity.
    pub identity: String,
    /// Its version.
    pub version: String,
    /// The source revision it was built from.
    pub source_revision: String,
    /// Whether that source tree was clean or dirty.
    pub source_state: SourceState,
    /// `sha256:<64 lowercase hex>` over the executable.
    pub executable_digest: String,
    /// `sha256:<64 lowercase hex>` over the configuration.
    pub configuration_digest: String,
    /// The capabilities the producer declared.
    pub capabilities: Vec<String>,
    /// The artifacts it emitted.
    pub artifacts: Vec<ProvenanceArtifact>,
}

/// Whether a producer's source tree carried uncommitted changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceState {
    /// No uncommitted changes.
    Clean,
    /// Uncommitted changes were present.
    Dirty,
}

/// What a record is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceSubject {
    /// The subject's kind.
    pub kind: String,
    /// Its id.
    pub id: String,
    /// The source revision it was observed at.
    pub source_revision: String,
}

/// How an experiment was set up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentDesign {
    /// The time box it ran in.
    pub time_box: String,
    /// The corpora it read, unique and non-empty.
    pub corpus_refs: Vec<String>,
    /// How the comparison was made.
    pub comparison_method: String,
    /// The rule that decided the outcome.
    pub decision_rule: String,
}

/// Whether an experiment supported its hypothesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentStatus {
    /// The hypothesis was supported.
    Supported,
    /// It was not.
    NotSupported,
    /// The experiment could not say.
    Inconclusive,
}

/// What an experiment concluded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentResult {
    /// The verdict.
    pub status: ExperimentStatus,
    /// A sentence saying what was found.
    pub summary: String,
    /// The supporting evidence, unique and non-empty.
    pub evidence_refs: Vec<String>,
}

/// An experiment record as its producer supplied it, before identification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentRecordInput {
    /// Always `experiment-record-v1`.
    pub schema_version: String,
    /// What the experiment was about.
    pub subject: EvidenceSubject,
    /// ISO-8601 instant.
    pub recorded_at: String,
    /// What was being tested.
    pub hypothesis: String,
    /// How.
    pub design: ExperimentDesign,
    /// What came of it.
    pub result: ExperimentResult,
    /// Who produced it.
    pub producer_provenance: ProducerProvenance,
}

/// An identified experiment record, as it is written to disk.
///
/// The id is a digest over the canonical JSON of the *input*, so the record on
/// disk is the input plus one key. `flatten` rather than a nested object for
/// exactly that reason: a nested input would change every stored byte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExperimentRecord {
    /// `sha256:<64 lowercase hex>` over the canonical JSON of the input.
    pub record_id: String,
    /// The producer-supplied fields.
    #[serde(flatten)]
    pub input: ExperimentRecordInput,
}

/// The window an operational observation covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationWindow {
    /// ISO-8601 instant.
    pub started_at: String,
    /// ISO-8601 instant, strictly after [`ObservationWindow::started_at`].
    pub ended_at: String,
}

/// One signal observed in operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    /// What was measured.
    pub signal: String,
    /// The measured value, as a string — the record never guesses a number.
    pub value: String,
    /// The unit, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// What the value means.
    pub interpretation: String,
    /// The supporting evidence, unique and non-empty.
    pub evidence_refs: Vec<String>,
}

/// Whether operation stayed inside its declared bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationalOutcome {
    /// It did.
    WithinBounds,
    /// It did not.
    OutsideBounds,
    /// The observations could not say.
    Inconclusive,
}

/// An operational evidence record as its producer supplied it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationalEvidenceRecordInput {
    /// Always `operational-evidence-record-v1`.
    pub schema_version: String,
    /// What was observed.
    pub subject: EvidenceSubject,
    /// ISO-8601 instant.
    pub recorded_at: String,
    /// The window the observations cover.
    pub window: ObservationWindow,
    /// Where it was running.
    pub environment: String,
    /// One entry per signal, non-empty.
    pub observations: Vec<Observation>,
    /// The verdict.
    pub outcome: OperationalOutcome,
    /// Who produced it.
    pub producer_provenance: ProducerProvenance,
}

/// An identified operational evidence record, as it is written to disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationalEvidenceRecord {
    /// `sha256:<64 lowercase hex>` over the canonical JSON of the input.
    pub record_id: String,
    /// The producer-supplied fields.
    #[serde(flatten)]
    pub input: OperationalEvidenceRecordInput,
}

/// The outcome of publishing one content-addressed record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAssuranceRecord<T> {
    /// The identified record.
    pub record: T,
    /// Where it was published, relative to the store root.
    pub path: String,
    /// Whether this call created the file, or found identical bytes there.
    pub created: bool,
}
