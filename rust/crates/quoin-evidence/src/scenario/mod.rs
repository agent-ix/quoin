// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Intake of a Quire Protocol scenario-evidence record (FR-130).
//!
//! Quire Protocol composes one record per integration scenario and passes it
//! here; Quoin owns everything after that. This module validates the record
//! against the typed shape below, refuses it with a named kind when a
//! component is missing, duplicated or cross-wired, and files it append-only
//! under `scenarios/<integration>/<scenario>.json`. Sufficiency, scoring and
//! disposition are not decided here, and the producer never reads the record
//! back.
//!
//! The document is read as a typed value. Nothing is recovered from a
//! transcript and no owner, contract or revision is inferred from a path or a
//! display name: every attribution is copied byte-exactly from the document.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::EvidenceError;
use crate::paths::scenario_path;
use crate::source::EvidenceSource;
use crate::store::codec::canonical_bytes_of;

/// The party that owns this intake contract.
pub const INTAKE_OWNER: &str = "quoin";

/// The one contract version this intake accepts.
pub const INTAKE_CONTRACT: &str = "quoin.protocol-scenario-intake/v1";

mod check;

const LABEL: &str = "scenario evidence";

/// The integration a scenario belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Integration {
    /// `IT-001`.
    #[serde(rename = "IT-001")]
    It001,
    /// `IT-002`.
    #[serde(rename = "IT-002")]
    It002,
    /// `IT-003`.
    #[serde(rename = "IT-003")]
    It003,
}

impl Integration {
    /// The spelling used in the document and in the stored path.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::It001 => "IT-001",
            Self::It002 => "IT-002",
            Self::It003 => "IT-003",
        }
    }
}

/// One owner/contract/revision attribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PinnedEntry {
    /// The external party that publishes the contract.
    pub owner: String,
    /// The published contract version of that owner.
    pub contract_version: String,
    /// The exact immutable revision the owner is pinned at.
    pub revision: String,
}

/// The entry that caused a refusal during resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CausingEntry {
    /// The entry's owner.
    pub owner: String,
    /// The entry's contract version.
    pub contract_version: String,
    /// Why resolution refused at this entry.
    pub reason: String,
}

/// The resolution state a scenario ran against, in exactly one admitted shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ResolutionState {
    /// Resolution completed: the full resolved manifest.
    Full {
        /// Every resolved entry.
        entries: Vec<PinnedEntry>,
    },
    /// The scenario refused during resolution: the partial state reached plus
    /// the entry that caused the refusal.
    Partial {
        /// The entries resolved before the refusal.
        reached: Vec<PinnedEntry>,
        /// The entry that caused it.
        causing_entry: CausingEntry,
    },
    /// A frozen legacy facade with no resolver: the exact selected attribution.
    LegacyCompatibility {
        /// The selection.
        selection: PinnedEntry,
    },
}

/// The owner of one consumed producer value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ValueOwner {
    /// The consumed producer value's own identity.
    pub value: String,
    /// The owner it resolved through.
    pub owner: String,
    /// The contract version it resolved through.
    pub contract_version: String,
    /// The revision it resolved through.
    pub revision: String,
}

/// One claim-level outcome of a completed assessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ClaimOutcome {
    /// A canonical result row.
    Row {
        /// The row's identity.
        identity: String,
    },
    /// A claim-local typed refusal.
    Refusal {
        /// The refusal's identity.
        identity: String,
    },
    /// A typed pre-row resource-incomplete outcome.
    ResourceIncomplete {
        /// The outcome's identity.
        identity: String,
    },
}

impl ClaimOutcome {
    const fn identity(&self) -> &String {
        match self {
            Self::Row { identity }
            | Self::Refusal { identity }
            | Self::ResourceIncomplete { identity } => identity,
        }
    }
}

/// The scenario's typed outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TypedOutcome {
    /// The exact shared operation outcome identity.
    pub operation: String,
    /// Ordered claim outcomes, where an assessment completed.
    pub claims: Vec<ClaimOutcome>,
}

/// A resource-budget dimension the caller grants (Protocol FR-016).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[allow(
    missing_docs,
    reason = "the members are the producer's closed dimension vocabulary, named by their spelling"
)]
pub enum Dimension {
    InputByte,
    Definition,
    Node,
    Edge,
    Role,
    Message,
    Branch,
    Iteration,
    ActiveInstance,
    Observation,
    Lineage,
    ResultItem,
    DerivedArtifact,
    OutputByte,
    Operation,
    AccountedAllocation,
    ElapsedTime,
    ResolutionEntryCount,
    ResolutionBytesDigested,
    ResolutionDeadline,
}

/// Granted and consumed figures on one dimension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BudgetFigure {
    /// The dimension.
    pub dimension: Dimension,
    /// The grant; absent is not a grant of zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granted: Option<u64>,
    /// What was consumed.
    pub consumed: u64,
}

/// The trace attribute binding a scenario to its acceptance criterion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TraceBinding {
    /// The acceptance criterion id the scenario is bound to.
    pub criterion: String,
}

/// One scenario-evidence record, as Quire Protocol hands it over.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ScenarioEvidence {
    /// The intake contract version the producer composed against.
    pub contract: String,
    /// The integration the scenario belongs to.
    pub integration: Integration,
    /// The scenario id, such as `IT-002-SC-07a`.
    pub scenario: String,
    /// The resolution state it ran against.
    pub resolution: ResolutionState,
    /// The owner of every consumed producer value.
    pub value_owners: Vec<ValueOwner>,
    /// The typed outcome.
    pub outcome: TypedOutcome,
    /// Granted and consumed budget per declared dimension.
    pub budget: Vec<BudgetFigure>,
    /// The trace binding.
    pub trace_binding: TraceBinding,
}

/// Why the intake refused a document. Spellings are contractual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScenarioRefusal {
    /// The document names a contract version this intake does not accept.
    WrongContract,
    /// A required component is absent or blank.
    MissingComponent,
    /// A key that must be unique appears twice.
    DuplicateComponent,
    /// A component attributes to something the record does not contain.
    CrossWired,
}

impl ScenarioRefusal {
    /// Every kind, in declaration order.
    pub const ALL: &'static [Self] = &[
        Self::WrongContract,
        Self::MissingComponent,
        Self::DuplicateComponent,
        Self::CrossWired,
    ];

    /// The kind's contractual spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WrongContract => "wrong-contract",
            Self::MissingComponent => "missing-component",
            Self::DuplicateComponent => "duplicate-component",
            Self::CrossWired => "cross-wired",
        }
    }
}

/// The outcome of one intake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredScenario {
    /// The accepted record.
    pub record: ScenarioEvidence,
    /// Where it was filed, relative to the store root.
    pub path: String,
    /// Whether this call created the file, or found identical bytes there.
    pub created: bool,
}

/// Validate and file one scenario-evidence document, append-only.
///
/// A second, different record for the same (integration, scenario) is refused;
/// an identical one is idempotent.
///
/// # Errors
///
/// [`EvidenceError::ScenarioRefused`] for a wrong contract version or a
/// missing, duplicated or cross-wired component,
/// [`EvidenceError::InvalidRecord`] when the document does not have the typed
/// shape, [`EvidenceError::RecordIntegrity`] when the key already holds
/// different bytes, otherwise [`EvidenceError::Canonicalization`] or
/// [`EvidenceError::StoreIo`].
pub fn write_scenario_evidence<S: EvidenceSource + ?Sized>(
    source: &mut S,
    document: &Value,
) -> Result<StoredScenario, EvidenceError> {
    let record = check::admit(document)?;
    let path = scenario_path(record.integration.as_str(), &record.scenario);
    let bytes = canonical_bytes_of(LABEL, &record)?;
    let created = source.publish(&path, &bytes)?;
    Ok(StoredScenario {
        record,
        path,
        created,
    })
}

#[cfg(test)]
mod tests;
