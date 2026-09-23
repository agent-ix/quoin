// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The intervention experiment record.
//!
//! A port of `src/measurement/intervention-types.ts:1-80`. Every closed string
//! union there is a `Copy` enum here; every open JSON shape it leaves open
//! (`configuration`, `held_constant[].value`, the changed-variable values) stays
//! `serde_json::Value`, so a record read and written back returns the bytes it
//! arrived as.

use engineering_assurance::claim_strength::ClaimStrength;
use serde::{Deserialize, Serialize};

use crate::common::identity::{ArmId, MetricName, RecordId, WireInstant};
use crate::common::producer::{Producer, Subject};
use crate::common::recorded_evidence::RecordedEvidenceReference;
use crate::common::scalar::{EffectValue, ScalarValue};
use crate::common::wire_enum::wire_enum;

wire_enum! {
    /// How an interaction or confounder was handled.
    ///
    /// `intervention-types.ts:1-2`.
    pub enum InterventionDisposition {
        /// The effect was held constant across arms.
        Controlled => "controlled",
        /// The effect varied across arms and was not held constant.
        Uncontrolled => "uncontrolled",
        /// Whether the effect varied is not known.
        Unknown => "unknown",
        /// The effect cannot apply to this experiment.
        NotApplicable => "not_applicable",
    }
}

wire_enum! {
    /// The shape of the experiment. `intervention-types.ts:34`.
    pub enum InterventionDesignKind {
        /// The same configuration run more than once.
        Repeated => "repeated",
        /// Arms assigned at random.
        Randomized => "randomized",
        /// A cross of more than one varied factor.
        Factorial => "factorial",
    }
}

wire_enum! {
    /// How units were assigned to arms. `intervention-types.ts:37-41`.
    pub enum InterventionAssignmentMethod {
        /// Assignment does not apply to this design.
        NotApplicable => "not_applicable",
        /// Assignment is a function of the unit.
        Deterministic => "deterministic",
        /// Assignment is random.
        Randomized => "randomized",
        /// Assignment is random within blocks.
        BlockedRandomized => "blocked_randomized",
    }
}

wire_enum! {
    /// Whether the experiment ran to a usable end. `intervention-types.ts:69`.
    pub enum InterventionStatus {
        /// The experiment ran and produced its measurements.
        Completed => "completed",
        /// The experiment did not run to completion.
        Failed => "failed",
        /// The experiment ran but settled nothing.
        Inconclusive => "inconclusive",
    }
}

wire_enum! {
    /// What the experiment concluded. `intervention-types.ts:71-74`.
    pub enum InterventionConclusionKind {
        /// A causal effect was established.
        CausalEffectEstablished => "causal_effect_established",
        /// No effect was observed.
        NoEffectObserved => "no_effect_observed",
        /// An effect may exist but its cause was not established.
        CauseNotEstablished => "cause_not_established",
    }
}

wire_enum! {
    /// How much of the effect the conclusion attributes to the intervention.
    ///
    /// `intervention-types.ts:76`.
    pub enum AttributionConfidence {
        /// No attribution is claimed.
        None => "none",
        /// Weak attribution.
        Low => "low",
        /// Qualified attribution.
        Moderate => "moderate",
        /// Strong attribution.
        High => "high",
    }
}

/// One arm of an experiment. `intervention-types.ts:4-9`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterventionArm {
    /// The arm's identity, referred to by `treatment_id`.
    pub id: ArmId,
    /// The population the arm drew from.
    pub population: String,
    /// How many units the arm measured.
    pub sample_size: u64,
    /// The arm's configuration, left open by the schema.
    pub configuration: serde_json::Map<String, serde_json::Value>,
}

/// How units were assigned to arms. `intervention-types.ts:36-43`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterventionAssignment {
    /// The assignment method.
    pub method: InterventionAssignmentMethod,
    /// The seed, when the method used one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<String>,
}

/// The experiment's design. `intervention-types.ts:33-45`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterventionDesign {
    /// The design's shape.
    pub kind: InterventionDesignKind,
    /// How many times the design was repeated.
    pub repetitions: u64,
    /// How units reached arms.
    pub assignment: InterventionAssignment,
    /// The conditions the sample was drawn under.
    pub sampling_conditions: Vec<String>,
}

/// A variable the treatment changed. `intervention-types.ts:48-53`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangedVariable {
    /// The variable's name.
    pub name: String,
    /// The arm the change applies to.
    pub treatment_id: ArmId,
    /// The value in the baseline arm, left open by the schema.
    pub baseline_value: serde_json::Value,
    /// The value in the treatment arm, left open by the schema.
    pub treatment_value: serde_json::Value,
}

/// A variable held constant across arms. `intervention-types.ts:54`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeldConstant {
    /// The variable's name.
    pub name: String,
    /// The value it was held at, left open by the schema.
    pub value: serde_json::Value,
}

/// One metric, measured on one arm against the baseline.
///
/// `intervention-types.ts:55-62`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasuredEffect {
    /// The arm measured.
    pub treatment_id: ArmId,
    /// The metric measured.
    pub metric: MetricName,
    /// The baseline arm's value.
    pub baseline_value: ScalarValue,
    /// The treatment arm's value.
    pub treatment_value: ScalarValue,
    /// The effect, where one could be stated.
    pub effect: EffectValue,
    /// The unit both values and the effect are in.
    pub unit: String,
}

/// An interaction or confounder, with how it was handled.
///
/// `intervention-types.ts:63-68` declares these as two structurally identical
/// lists. They are one type here; which list a value came from is carried by
/// the field it was read from, and by
/// [`CounterevidenceKind`](crate::intervention::report::CounterevidenceKind)
/// once the report has merged them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispositionedEffect {
    /// What the effect is.
    pub description: String,
    /// How it was handled.
    pub disposition: InterventionDisposition,
}

/// What the experiment concluded. `intervention-types.ts:70-77`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterventionConclusion {
    /// The kind of conclusion reached.
    pub kind: InterventionConclusionKind,
    /// The conclusion in words; the report carries this as the record's claim.
    pub statement: String,
    /// How strongly the conclusion attributes the effect.
    pub attribution_confidence: AttributionConfidence,
}

/// A recorded intervention experiment. `intervention-types.ts:19-80`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterventionExperimentRecord {
    /// The record schema's version — always `1` today.
    pub schema_version: u32,
    /// The record family — always `"intervention_experiment"`.
    pub record_type: InterventionRecordType,
    /// The record's identity.
    pub record_id: RecordId,
    /// When the experiment was observed.
    pub observed_at: WireInstant,
    /// What was measured.
    pub subject: Subject,
    /// What produced the record.
    pub producer: Producer,
    /// The kind of support this record carries (FR-022, EA `ClaimStrength`,
    /// PLAT-972). An intervention experiment is a deliberate, controlled
    /// exercise over a selected set of cases, so it is always `tested`; the
    /// wire schema fixes it with a `const`, and the field is required rather
    /// than inferred so a record with no strength is refused, not defaulted.
    pub strength: ClaimStrength,
    /// The experiment's design.
    pub design: InterventionDesign,
    /// The arm the treatments are measured against.
    pub baseline: InterventionArm,
    /// The arms measured against the baseline.
    pub treatments: Vec<InterventionArm>,
    /// What the treatments changed.
    pub changed_variables: Vec<ChangedVariable>,
    /// What was held constant across arms.
    pub held_constant: Vec<HeldConstant>,
    /// What was measured.
    pub measured_effects: Vec<MeasuredEffect>,
    /// Interactions between varied factors.
    pub interactions: Vec<DispositionedEffect>,
    /// Effects that could explain the measurement other than the intervention.
    pub confounders: Vec<DispositionedEffect>,
    /// Whether the experiment ran to a usable end.
    pub status: InterventionStatus,
    /// What it concluded.
    pub conclusion: InterventionConclusion,
    /// What the experiment could not settle.
    pub gaps: Vec<String>,
    /// Who owns the record.
    pub owner: String,
    /// What the record asks for next.
    pub actions: Vec<String>,
    /// The retained files the record rests on.
    pub raw_evidence: Vec<RecordedEvidenceReference>,
}

wire_enum! {
    /// The `record_type` discriminant. `intervention-types.ts:21`.
    pub enum InterventionRecordType {
        /// The only value.
        InterventionExperiment => "intervention_experiment",
    }
}
