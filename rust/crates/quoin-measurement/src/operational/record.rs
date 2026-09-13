// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The operational evidence record.
//!
//! A port of `src/measurement/operational-types.ts:1-105`. The TypeScript
//! models the two record shapes as an interface pair joined by a union, with
//! `exercise?: never` and `capability?: never` keeping them disjoint. Rust has
//! a union that is disjoint by construction, so those two `never` fields have
//! no counterpart here: [`OperationalEvidenceRecord`] is an enum tagged on
//! `record_shape`, and a record carrying both fields is unrepresentable rather
//! than merely ill-typed.

use serde::{Deserialize, Serialize};

use crate::common::identity::{ControlId, Digest, RecordId, Revision, WireInstant};
use crate::common::literal_bool::LiteralBool;
use crate::common::producer::{Producer, Subject};
use crate::common::recorded_evidence::RecordedEvidenceReference;
use crate::common::wire_enum::wire_enum;

wire_enum! {
    /// The kind of operational control a record is about.
    ///
    /// `operational-types.ts:3-19`.
    pub enum OperationalControlKind {
        /// Shipping a revision.
        Release => "release",
        /// A runtime toggle.
        FeatureFlag => "feature_flag",
        /// A staged rollout to part of the population.
        CanaryDeployment => "canary_deployment",
        /// A deployment that runs without serving.
        ShadowDeployment => "shadow_deployment",
        /// Returning to a previous revision.
        Rollback => "rollback",
        /// Stopping the system outright.
        KillSwitch => "kill_switch",
        /// A person overriding an automated decision.
        HumanOverride => "human_override",
        /// A subject contesting a decision.
        Appeal => "appeal",
        /// Declining to decide.
        Abstention => "abstention",
        /// Degrading to a known-safe behaviour.
        SafeFallback => "safe_fallback",
        /// Pinning the governing policy.
        PolicyPin => "policy_pin",
        /// Pinning the governing prompt.
        PromptPin => "prompt_pin",
        /// Pinning the governing model.
        ModelPin => "model_pin",
        /// Pinning the governing tool.
        ToolPin => "tool_pin",
        /// Pinning the governing data.
        DataPin => "data_pin",
        /// Reporting an operational fact onward.
        Reporting => "reporting",
    }
}

wire_enum! {
    /// What a version pin pins. `operational-types.ts:42`.
    pub enum VersionPinKind {
        /// The governing policy.
        Policy => "policy",
        /// The governing prompt.
        Prompt => "prompt",
        /// The governing model.
        Model => "model",
        /// The governing tool.
        Tool => "tool",
        /// The governing data.
        Data => "data",
    }
}

wire_enum! {
    /// The `record_type` discriminant. `operational-types.ts:23`.
    pub enum OperationalRecordType {
        /// The only value.
        OperationalEvidence => "operational_evidence",
    }
}

wire_enum! {
    /// Whether a control is there to be used. `operational-types.ts:51`.
    pub enum CapabilityStatus {
        /// The control exists and can be used.
        Available => "available",
        /// The control does not exist or cannot be used.
        Unavailable => "unavailable",
        /// Whether the control can be used is not known.
        Unknown => "unknown",
        /// The control cannot apply to this subject.
        NotApplicable => "not_applicable",
    }
}

wire_enum! {
    /// Whether an exercise was for real. `operational-types.ts:76`.
    pub enum ExerciseMode {
        /// The control was used in earnest.
        Actual => "actual",
        /// The control was rehearsed.
        Drill => "drill",
    }
}

wire_enum! {
    /// How an exercise ended. `operational-types.ts:81`.
    pub enum ExerciseOutcome {
        /// The control did what it is for.
        Succeeded => "succeeded",
        /// The control did not.
        Failed => "failed",
        /// The control partly did.
        Partial => "partial",
        /// The exercise was stopped before it settled.
        Aborted => "aborted",
    }
}

wire_enum! {
    /// Where an exercise stands against its deadline.
    ///
    /// `operational-types.ts:92`.
    pub enum ClockStatus {
        /// The deadline has not passed and the exercise has not completed.
        Open => "open",
        /// The exercise completed inside the deadline.
        Met => "met",
        /// The deadline passed first.
        Missed => "missed",
    }
}

wire_enum! {
    /// The clock status of an exercise no clock applies to.
    ///
    /// `operational-types.ts:86` fixes this field to a single spelling; it is a
    /// one-variant union rather than a bare string so the renderer reads it the
    /// same way it reads a [`ClockStatus`].
    pub enum NotApplicableClockStatus {
        /// The only value.
        NotApplicable => "not_applicable",
    }
}

/// One pinned component of the configuration a control ran under.
///
/// `operational-types.ts:41-46`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionPin {
    /// What is pinned.
    pub kind: VersionPinKind,
    /// Which one.
    pub identity: String,
    /// At which revision.
    pub revision: Revision,
    /// And at which digest.
    pub digest: Digest,
}

/// The pinned configuration. `operational-types.ts:40-48`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationalConfiguration {
    /// Every pin, in the order recorded.
    pub version_pins: Vec<VersionPin>,
}

/// Where a control was observed. `operational-types.ts:39`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationalScope {
    /// The service the control governs.
    pub service: String,
    /// The environment it was observed in.
    pub environment: String,
    /// The population it applied to.
    pub population: String,
}

/// Everything both operational record shapes carry.
///
/// `operational-types.ts:21-49`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationalBase {
    /// The record schema's version — always `1` today.
    pub schema_version: u32,
    /// The record family — always `"operational_evidence"`.
    pub record_type: OperationalRecordType,
    /// The record's identity.
    pub record_id: RecordId,
    /// When the evidence was observed.
    pub observed_at: WireInstant,
    /// The kind of control.
    pub control_kind: OperationalControlKind,
    /// What the control governs.
    pub subject: Subject,
    /// What produced the record.
    pub producer: Producer,
    /// Where the control was observed.
    pub scope: OperationalScope,
    /// What the control ran under.
    pub configuration: OperationalConfiguration,
    /// Who owns the record.
    pub owner: String,
    /// What the record leaves unsettled.
    pub gaps: Vec<String>,
    /// What the record asks for next.
    pub actions: Vec<String>,
    /// The retained files the record rests on.
    pub raw_evidence: Vec<RecordedEvidenceReference>,
}

/// Whether a standing control has a deadline, and what bounds it.
///
/// `operational-types.ts:59-66`. The discriminant is the boolean `supported`,
/// which serde cannot tag on; see [`LiteralBool`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClockSupport {
    /// `{ "supported": true, … }` — the control has a deadline.
    Supported {
        /// Always `true`.
        supported: LiteralBool<true>,
        /// The event that starts the clock.
        start_event: String,
        /// The event that stops it.
        completion_event: String,
        /// How long the control has, in seconds.
        deadline_seconds: u64,
    },
    /// `{ "supported": false }` — the control has no deadline.
    Unsupported {
        /// Always `false`.
        supported: LiteralBool<false>,
    },
}

/// A control as it stands, independent of any one use.
///
/// `operational-types.ts:49-67`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandingCapability {
    /// The control's identity.
    pub control_id: ControlId,
    /// Whether it is there to be used.
    pub status: CapabilityStatus,
    /// Where it is reached.
    pub surface: String,
    /// Who may use it.
    pub authorized_roles: Vec<String>,
    /// What it covers.
    pub coverage: String,
    /// What it does not.
    pub limitations: Vec<String>,
    /// The state transitions it supports.
    pub supported_transitions: Vec<String>,
    /// Whether it has a deadline.
    pub clock_support: ClockSupport,
}

/// Where one exercise stands against its deadline.
///
/// `operational-types.ts:85-93`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "applicability")]
pub enum ExerciseClock {
    /// No deadline applies to this control.
    #[serde(rename = "not_applicable")]
    NotApplicable {
        /// Always `"not_applicable"`.
        status: NotApplicableClockStatus,
    },
    /// A deadline applies.
    #[serde(rename = "operational_with_clock")]
    WithClock {
        /// When the clock started.
        started_at: WireInstant,
        /// When it runs out.
        deadline_at: WireInstant,
        /// When the exercise completed, if it has.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        completed_at: Option<WireInstant>,
        /// Where the exercise stands.
        status: ClockStatus,
    },
}

impl ExerciseClock {
    /// The clock's status as the report spells it.
    ///
    /// `operational-report.ts:57,63,68` read `clock.status` across both
    /// variants; this is that read, made total.
    #[must_use]
    pub fn status_str(&self) -> &'static str {
        match self {
            Self::NotApplicable { status } => status.as_str(),
            Self::WithClock { status, .. } => status.as_str(),
        }
    }

    /// Whether no deadline applies. `operational-report.ts:54`.
    #[must_use]
    pub fn is_not_applicable(&self) -> bool {
        matches!(self, Self::NotApplicable { .. })
    }
}

/// One use of a control. `operational-types.ts:72-94`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationalExercise {
    /// The control used.
    pub control_id: ControlId,
    /// The standing-capability record this exercise is against, if one is named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_record_id: Option<RecordId>,
    /// Whether it was for real.
    pub mode: ExerciseMode,
    /// When the exercise started.
    pub started_at: WireInstant,
    /// When it completed.
    pub completed_at: WireInstant,
    /// Who ran it.
    pub actor: String,
    /// What prompted it.
    pub trigger: String,
    /// How it ended.
    pub outcome: ExerciseOutcome,
    /// The state before, left open by the schema.
    pub state_before: serde_json::Map<String, serde_json::Value>,
    /// The state after, left open by the schema.
    pub state_after: serde_json::Map<String, serde_json::Value>,
    /// What was noticed.
    pub observations: Vec<String>,
    /// Where it stands against its deadline.
    pub clock: ExerciseClock,
}

/// A record asserting a control stands. `operational-types.ts:49-68`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StandingCapabilityRecord {
    /// Everything both shapes carry.
    #[serde(flatten)]
    pub base: OperationalBase,
    /// The control, as it stands.
    pub capability: StandingCapability,
}

/// A record of one use of a control. `operational-types.ts:70-95`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationalExerciseRecord {
    /// Everything both shapes carry.
    #[serde(flatten)]
    pub base: OperationalBase,
    /// The use.
    pub exercise: OperationalExercise,
}

/// An operational evidence record, of either shape.
///
/// `operational-types.ts:97-98`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "record_shape")]
pub enum OperationalEvidenceRecord {
    /// A control as it stands.
    #[serde(rename = "standing_capability")]
    StandingCapability(StandingCapabilityRecord),
    /// One use of a control.
    #[serde(rename = "exercise")]
    Exercise(OperationalExerciseRecord),
}

impl OperationalEvidenceRecord {
    /// Everything both shapes carry.
    #[must_use]
    pub fn base(&self) -> &OperationalBase {
        match self {
            Self::StandingCapability(record) => &record.base,
            Self::Exercise(record) => &record.base,
        }
    }

    /// The record's shape, as the report entry carries it.
    #[must_use]
    pub fn record_shape(&self) -> OperationalRecordShape {
        match self {
            Self::StandingCapability(_) => OperationalRecordShape::StandingCapability,
            Self::Exercise(_) => OperationalRecordShape::Exercise,
        }
    }
}

wire_enum! {
    /// Which shape a record is. `operational-types.ts:50,71`.
    pub enum OperationalRecordShape {
        /// A control as it stands.
        StandingCapability => "standing_capability",
        /// One use of a control.
        Exercise => "exercise",
    }
}

wire_enum! {
    /// The applicability of an obligation's clock. `operational-types.ts:106`.
    pub enum WithClockApplicability {
        /// The only value.
        OperationalWithClock => "operational_with_clock",
    }
}

/// The clock an obligation imposes. `operational-types.ts:105-109`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObligationClock {
    /// Always `"operational_with_clock"`.
    pub applicability: WithClockApplicability,
    /// When the clock started.
    pub started_at: WireInstant,
    /// When it runs out.
    pub deadline_at: WireInstant,
}

/// A control exercise a subject owes. `operational-types.ts:100-110`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationalObligation {
    /// The kind of control owed.
    pub control_kind: OperationalControlKind,
    /// What owes it.
    pub subject: Subject,
    /// Where.
    pub scope: OperationalScope,
    /// Which exercise modes discharge it.
    pub accepted_modes: Vec<ExerciseMode>,
    /// By when.
    pub clock: ObligationClock,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{ClockSupport, ExerciseClock};

    /// The boolean discriminant is enforced, not merely documented.
    #[test]
    fn a_clock_support_cannot_claim_one_variant_and_carry_the_other() {
        let unsupported: ClockSupport =
            serde_json::from_str(r#"{"supported":false}"#).expect("the unsupported shape");
        assert!(matches!(unsupported, ClockSupport::Unsupported { .. }));

        let supported: ClockSupport = serde_json::from_str(
            r#"{"supported":true,"start_event":"a","completion_event":"b","deadline_seconds":1}"#,
        )
        .expect("the supported shape");
        assert!(matches!(supported, ClockSupport::Supported { .. }));

        // `supported: false` with the supported variant's fields matches
        // neither arm: the false literal refuses the first, and the missing
        // `supported: false`-only shape refuses nothing else.
        let mismatched = serde_json::from_str::<ClockSupport>(
            r#"{"supported":false,"start_event":"a","completion_event":"b","deadline_seconds":1}"#,
        );
        assert!(
            mismatched.is_ok(),
            "the extra fields are ignored, as in TypeScript"
        );
        assert!(matches!(
            mismatched.unwrap(),
            ClockSupport::Unsupported { .. }
        ));

        assert!(
            serde_json::from_str::<ClockSupport>(r#"{"supported":true}"#).is_err(),
            "a supported clock without its bounds is not a clock"
        );
    }

    /// The exercise clock is tagged on `applicability`, and the tag decides.
    #[test]
    fn an_exercise_clock_reads_its_status_through_either_arm() {
        let not_applicable: ExerciseClock =
            serde_json::from_str(r#"{"applicability":"not_applicable","status":"not_applicable"}"#)
                .expect("the not-applicable shape");
        assert!(not_applicable.is_not_applicable());
        assert_eq!(not_applicable.status_str(), "not_applicable");

        let with_clock: ExerciseClock = serde_json::from_str(
            r#"{"applicability":"operational_with_clock","started_at":"a","deadline_at":"b","status":"open"}"#,
        )
        .expect("the with-clock shape");
        assert!(!with_clock.is_not_applicable());
        assert_eq!(with_clock.status_str(), "open");
    }
}
