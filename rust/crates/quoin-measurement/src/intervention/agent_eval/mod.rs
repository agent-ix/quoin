// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The agent-eval producer: its definition, and the record it produces.
//!
//! | retained TypeScript | here |
//! | --- | --- |
//! | `intervention-types.ts:82-102` | this module's declarations |
//! | `agent-eval-intervention.ts:11-12,183-215` | [`version`] |
//! | `agent-eval-intervention.ts:134-181` | [`report`] |
//! | `agent-eval-intervention.ts:14-132` | [`produce`] |
//!
//! # What the producer refuses with
//!
//! The retained producer throws bare `Error`s. Every refusal here is an
//! [`InterventionIntakeError`](crate::intervention::intake::InterventionIntakeError)
//! instead, because a caller that already handles intake refusals should not
//! need a second error family to call the producer that feeds it. The mapping
//! is deliberate and is the whole of it: a definition the producer will not
//! accept is `definition_mismatch`; a retained agent-eval report it cannot read
//! is `invalid_record`, since what failed is the record the producer was asked
//! to assemble. The refusal *sentences* are the retained ones.

pub mod produce;
pub mod report;
pub mod version;

pub use produce::{ProducedIntervention, produce_agent_eval_intervention};
pub use report::{AgentEvalReport, ScenarioRate};
pub use version::{ImmutableVersion, ImmutableVersionKind};

use serde::{Deserialize, Serialize};

use crate::common::identity::{ArmId, EvidencePath, RecordId};
use crate::common::producer::{Producer, Subject};
use crate::intervention::record::{
    ChangedVariable, DispositionedEffect, HeldConstant, InterventionDesign,
};

/// An arm as a definition states it, before a sample size is known.
///
/// `intervention-types.ts:89-90` — `Omit<InterventionArm, "sample_size">`. Rust
/// has no structural `Omit`, so the two fields the definition does carry are
/// written out; the record type stays the one with the sample size.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterventionArmDefinition {
    /// The arm's identity.
    pub id: ArmId,
    /// The population the arm draws from.
    pub population: String,
    /// The arm's configuration, left open by the schema.
    pub configuration: serde_json::Map<String, serde_json::Value>,
}

/// What an agent-eval run needs in order to become an intervention record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentEvalInterventionDefinition {
    /// The version of the agent-eval report schema this definition reads.
    pub report_schema_version: String,
    /// The version of `cli-agent-evals` that produced the reports.
    pub cli_agent_evals_version: String,
    /// The identity the produced record will carry.
    pub record_id: RecordId,
    /// What is measured.
    pub subject: Subject,
    /// What produces the record.
    pub producer: Producer,
    /// The experiment's design.
    pub design: InterventionDesign,
    /// The baseline arm.
    pub baseline: InterventionArmDefinition,
    /// The treatment arm.
    pub treatment: InterventionArmDefinition,
    /// What the treatment changes.
    pub changed_variables: Vec<ChangedVariable>,
    /// What is held constant.
    pub held_constant: Vec<HeldConstant>,
    /// Declared interactions.
    pub interactions: Vec<DispositionedEffect>,
    /// Declared confounders.
    pub confounders: Vec<DispositionedEffect>,
    /// How attribution is argued, when the definition says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution_method: Option<String>,
    /// Who owns the produced record.
    pub owner: String,
    /// Declared gaps.
    pub gaps: Vec<String>,
    /// Declared actions.
    pub actions: Vec<String>,
    /// Where the baseline arm's evidence is.
    pub baseline_evidence_path: EvidencePath,
    /// Where the treatment arm's evidence is.
    pub treatment_evidence_path: EvidencePath,
}
