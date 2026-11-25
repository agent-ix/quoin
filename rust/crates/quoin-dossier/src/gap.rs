// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The gap model (FR-191, "Gaps section").
//!
//! A gap is one missing input, reported rather than dropped or defaulted. The
//! `gap_kind` list is closed and ordered; the position of a kind in it is the
//! last term of the gap sort key.

use std::cmp::Ordering;

use serde::Serialize;

use crate::order::optional_cmp;

/// The closed list of gap kinds, in the order FR-191 fixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GapKind {
    /// The repository holds no requirement.
    NoRequirements,
    /// A requirement has no acceptance criterion.
    NoAcceptanceCriteria,
    /// A requirement has no criticality.
    CriticalityUnassigned,
    /// A requirement records no software safety class.
    SafetyClassUnassigned,
    /// The design relations could not be read.
    DesignRelationsUnavailable,
    /// A requirement is allocated to no design element.
    NoDesignElement,
    /// A design reference names nothing.
    DanglingDesignReference,
    /// A design reference names an element of the wrong kind.
    DesignKindMismatch,
    /// A design element is allocated to nothing.
    NoAllocation,
    /// A criterion has no backing.
    UnbackedCriterion,
    /// A criterion has no evidence.
    NoEvidence,
    /// No clause binding was supplied.
    NoClauseBinding,
    /// A bound clause is still open.
    ClauseOpen,
    /// A criterion's source finding carries the typed `failed-run` cause.
    EvidenceFailed,
    /// A criterion's evidence is stale without a failed run.
    EvidenceStale,
    /// A criterion's evidence is distrusted by the auditor.
    EvidenceSuspect,
    /// A SOUP advisory or policy finding has no current disposition.
    SoupUnresolvedAdvisory,
    /// A SOUP component is past its stated support end date.
    SoupPastEndOfSupport,
    /// A required SOUP field is absent or unknown.
    SoupFieldNotStated,
    /// Anomaly safety impact has not been assessed.
    AnomalyUnassessed,
    /// An impacting carried anomaly has no release rationale.
    AnomalyNoReleaseRationale,
    /// A source risk-summary row has a completeness finding.
    RiskChainIncomplete,
    /// A residual risk has no decision or decider.
    ResidualRiskUndecided,
    /// The corpus holds no overall residual risk evaluation record.
    NoOverallEvaluation,
    /// An overall residual risk evaluation is undecided.
    OverallEvaluationUndecided,
    /// An overall evaluation does not consider a residual risk in scope.
    ResidualRiskNotConsidered,
    /// An overall evaluation is decided over a considered open residual risk.
    EvaluationAcceptedWithOpenRisk,
    /// A decision's typed decider differs from the author of the commit that
    /// set it.
    DecisionSignerMismatch,
    /// The commit that set a decision carries no valid signature.
    DecisionCommitUnsigned,
    /// A control has no verification.
    ControlUnverified,
    /// No acceptability policy is present for a risk estimate.
    NoAcceptabilityPolicy,
    /// No authored risk management plan record exists.
    NoPlanRecord,
    /// No committed release snapshot was supplied.
    NoSnapshot,
    /// No Engineering Assurance release verification was supplied.
    NoReleaseVerifyInput,
    /// Engineering Assurance did not verify a release.
    ReleaseNotVerified,
    /// A covered connection has no reported threat.
    CrossingWithoutThreat,
    /// A covered data sink reached by a classification has no reported threat.
    ClassifiedSinkWithoutThreat,
    /// A considered threat has neither a treatment control nor an accepted risk.
    ThreatWithoutTreatment,
    /// The selected scope has no threat model record.
    NoThreatModel,
    /// The selected scope has no software description record.
    NoSoftwareDescriptionSource,
    /// A software description omits an authored field.
    SoftwareDescriptionFieldUnstated,
    /// No documentation-level decision record was selected.
    NoDocumentationLevelDecision,
    /// A documentation-level decision omits an authored field.
    DocumentationLevelDecisionFieldUnstated,
    /// Selected documentation-level decision records conflict.
    ConflictingDocumentationLevelDecisions,
    /// No software design source was selected.
    NoSoftwareDesignSource,
    /// A software design item has no explicit architecture part list.
    SoftwareDesignPartListUnstated,
    /// A software design item has no explicit architecture decision list.
    SoftwareDesignDecisionListUnstated,
    /// A product source reference does not resolve to its declared kind.
    SourceReferenceUnresolved,
    /// No device description record was selected.
    NoDeviceDescriptionSource,
    /// A device description omits an authored field.
    DeviceFieldUnstated,
    /// The selected scope has no processing activity record.
    NoProcessingActivity,
    /// Required authored model facts were not available in the flow report.
    ModelFactsUnavailable,
    /// A connection or sink has no personal-data classification evidence.
    PersonalDataCategoriesUnstated,
    /// A sink has no recipient category evidence.
    RecipientCategoriesUnstated,
    /// A sink's transfer region cannot be established from the model.
    TransferRegionUnstated,
    /// A sink's transfer safeguards cannot be established from the model.
    TransferSafeguardsUnstated,
    /// A connection or sink has no stated security measure.
    SecurityMeasuresUnstated,
}

impl GapKind {
    /// Every kind, in the closed order.
    #[cfg(test)]
    pub(crate) const ALL: [Self; 57] = [
        Self::NoRequirements,
        Self::NoAcceptanceCriteria,
        Self::CriticalityUnassigned,
        Self::SafetyClassUnassigned,
        Self::DesignRelationsUnavailable,
        Self::NoDesignElement,
        Self::DanglingDesignReference,
        Self::DesignKindMismatch,
        Self::NoAllocation,
        Self::UnbackedCriterion,
        Self::NoEvidence,
        Self::NoClauseBinding,
        Self::ClauseOpen,
        Self::EvidenceFailed,
        Self::EvidenceStale,
        Self::EvidenceSuspect,
        Self::SoupUnresolvedAdvisory,
        Self::SoupPastEndOfSupport,
        Self::SoupFieldNotStated,
        Self::AnomalyUnassessed,
        Self::AnomalyNoReleaseRationale,
        Self::RiskChainIncomplete,
        Self::ResidualRiskUndecided,
        Self::NoOverallEvaluation,
        Self::OverallEvaluationUndecided,
        Self::ResidualRiskNotConsidered,
        Self::EvaluationAcceptedWithOpenRisk,
        Self::DecisionSignerMismatch,
        Self::DecisionCommitUnsigned,
        Self::ControlUnverified,
        Self::NoAcceptabilityPolicy,
        Self::NoPlanRecord,
        Self::NoSnapshot,
        Self::NoReleaseVerifyInput,
        Self::ReleaseNotVerified,
        Self::CrossingWithoutThreat,
        Self::ClassifiedSinkWithoutThreat,
        Self::ThreatWithoutTreatment,
        Self::NoThreatModel,
        Self::NoSoftwareDescriptionSource,
        Self::SoftwareDescriptionFieldUnstated,
        Self::NoDocumentationLevelDecision,
        Self::DocumentationLevelDecisionFieldUnstated,
        Self::ConflictingDocumentationLevelDecisions,
        Self::NoSoftwareDesignSource,
        Self::SoftwareDesignPartListUnstated,
        Self::SoftwareDesignDecisionListUnstated,
        Self::SourceReferenceUnresolved,
        Self::NoDeviceDescriptionSource,
        Self::DeviceFieldUnstated,
        Self::NoProcessingActivity,
        Self::ModelFactsUnavailable,
        Self::PersonalDataCategoriesUnstated,
        Self::RecipientCategoriesUnstated,
        Self::TransferRegionUnstated,
        Self::TransferSafeguardsUnstated,
        Self::SecurityMeasuresUnstated,
    ];

    /// The spelling written in the Markdown and the JSON report.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoRequirements => "no_requirements",
            Self::NoAcceptanceCriteria => "no_acceptance_criteria",
            Self::CriticalityUnassigned => "criticality_unassigned",
            Self::SafetyClassUnassigned => "safety_class_unassigned",
            Self::DesignRelationsUnavailable => "design_relations_unavailable",
            Self::NoDesignElement => "no_design_element",
            Self::DanglingDesignReference => "dangling_design_reference",
            Self::DesignKindMismatch => "design_kind_mismatch",
            Self::NoAllocation => "no_allocation",
            Self::UnbackedCriterion => "unbacked_criterion",
            Self::NoEvidence => "no_evidence",
            Self::NoClauseBinding => "no_clause_binding",
            Self::ClauseOpen => "clause_open",
            Self::EvidenceFailed => "evidence_failed",
            Self::EvidenceStale => "evidence_stale",
            Self::EvidenceSuspect => "evidence_suspect",
            Self::SoupUnresolvedAdvisory => "soup_unresolved_advisory",
            Self::SoupPastEndOfSupport => "soup_past_end_of_support",
            Self::SoupFieldNotStated => "soup_field_not_stated",
            Self::AnomalyUnassessed => "anomaly_unassessed",
            Self::AnomalyNoReleaseRationale => "anomaly_no_release_rationale",
            Self::RiskChainIncomplete => "risk_chain_incomplete",
            Self::ResidualRiskUndecided => "residual_risk_undecided",
            Self::NoOverallEvaluation => "no_overall_evaluation",
            Self::OverallEvaluationUndecided => "overall_evaluation_undecided",
            Self::ResidualRiskNotConsidered => "residual_risk_not_considered",
            Self::EvaluationAcceptedWithOpenRisk => "evaluation_accepted_with_open_risk",
            Self::DecisionSignerMismatch => "decision_signer_mismatch",
            Self::DecisionCommitUnsigned => "decision_commit_unsigned",
            Self::ControlUnverified => "control_unverified",
            Self::NoAcceptabilityPolicy => "no_acceptability_policy",
            Self::NoPlanRecord => "no_plan_record",
            Self::NoSnapshot => "no_snapshot",
            Self::NoReleaseVerifyInput => "no_release_verify_input",
            Self::ReleaseNotVerified => "release_not_verified",
            Self::CrossingWithoutThreat => "crossing_without_threat",
            Self::ClassifiedSinkWithoutThreat => "classified_sink_without_threat",
            Self::ThreatWithoutTreatment => "threat_without_treatment",
            Self::NoThreatModel => "no_threat_model",
            Self::NoSoftwareDescriptionSource => "no_software_description_source",
            Self::SoftwareDescriptionFieldUnstated => "software_description_field_unstated",
            Self::NoDocumentationLevelDecision => "no_documentation_level_decision",
            Self::DocumentationLevelDecisionFieldUnstated => "documentation_level_decision_field_unstated",
            Self::ConflictingDocumentationLevelDecisions => "conflicting_documentation_level_decisions",
            Self::NoSoftwareDesignSource => "no_software_design_source",
            Self::SoftwareDesignPartListUnstated => "software_design_part_list_unstated",
            Self::SoftwareDesignDecisionListUnstated => "software_design_decision_list_unstated",
            Self::SourceReferenceUnresolved => "source_reference_unresolved",
            Self::NoDeviceDescriptionSource => "no_device_description_source",
            Self::DeviceFieldUnstated => "device_field_unstated",
            Self::NoProcessingActivity => "no_processing_activity",
            Self::ModelFactsUnavailable => "model_facts_unavailable",
            Self::PersonalDataCategoriesUnstated => "personal_data_categories_unstated",
            Self::RecipientCategoriesUnstated => "recipient_categories_unstated",
            Self::TransferRegionUnstated => "transfer_region_unstated",
            Self::TransferSafeguardsUnstated => "transfer_safeguards_unstated",
            Self::SecurityMeasuresUnstated => "security_measures_unstated",
        }
    }

    /// The position of this kind in the closed list.
    #[must_use]
    pub(crate) const fn position(self) -> usize {
        self as usize
    }
}

/// The ids a gap names, which order it among its siblings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct GapKey {
    requirement: Option<String>,
    criterion: Option<String>,
    element: Option<String>,
    clause: Option<String>,
}

/// One missing input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Gap {
    /// Which closed-list kind of input is missing.
    pub gap_kind: GapKind,
    /// The one thing the gap is about.
    pub subject: String,
    /// What is missing, for a reader.
    pub detail: String,
    #[serde(skip)]
    key: GapKey,
}

impl Gap {
    /// A gap with no id: it sorts before every gap that names one.
    #[must_use]
    pub fn new(gap_kind: GapKind, subject: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            gap_kind,
            subject: subject.into(),
            detail: detail.into(),
            key: GapKey::default(),
        }
    }

    /// Name the requirement this gap belongs to.
    #[must_use]
    pub fn for_requirement(mut self, id: impl Into<String>) -> Self {
        self.key.requirement = Some(id.into());
        self
    }

    /// Name the criterion this gap belongs to.
    #[must_use]
    pub fn for_criterion(mut self, id: impl Into<String>) -> Self {
        self.key.criterion = Some(id.into());
        self
    }

    /// Name the element (a part) this gap belongs to.
    #[must_use]
    pub fn for_element(mut self, id: impl Into<String>) -> Self {
        self.key.element = Some(id.into());
        self
    }

    /// Name the clause this gap belongs to.
    #[must_use]
    pub fn for_clause(mut self, id: impl Into<String>) -> Self {
        self.key.clause = Some(id.into());
        self
    }

    /// The sort key (requirement, criterion, element, clause, kind position).
    #[must_use]
    pub(crate) fn order(&self, other: &Self) -> Ordering {
        optional_cmp(
            self.key.requirement.as_deref(),
            other.key.requirement.as_deref(),
        )
        .then_with(|| {
            optional_cmp(
                self.key.criterion.as_deref(),
                other.key.criterion.as_deref(),
            )
        })
        .then_with(|| optional_cmp(self.key.element.as_deref(), other.key.element.as_deref()))
        .then_with(|| optional_cmp(self.key.clause.as_deref(), other.key.clause.as_deref()))
        .then_with(|| self.gap_kind.position().cmp(&other.gap_kind.position()))
    }

    /// The bullet this gap is written as: a backticked kind, then
    /// `<subject>: <detail>`.
    #[must_use]
    pub(crate) fn bullet(&self) -> String {
        format!(
            "- `{}` {}: {}",
            self.gap_kind.as_str(),
            one_line(&self.subject),
            one_line(&self.detail)
        )
    }
}

/// A bullet is one line; a line break inside a subject or detail would end it.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Sort gaps into the order FR-191 fixes. The sort is stable, so gaps with an
/// equal key keep the order the generator produced them in.
pub(crate) fn sort_gaps(gaps: &mut [Gap]) {
    gaps.sort_by(Gap::order);
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("FR-191-AC-14")]
    #[test]
    fn the_closed_list_is_in_the_spec_order() {
        let spelled: Vec<_> = GapKind::ALL.iter().map(|kind| kind.as_str()).collect();
        assert_eq!(
            spelled,
            [
                "no_requirements",
                "no_acceptance_criteria",
                "criticality_unassigned",
                "safety_class_unassigned",
                "design_relations_unavailable",
                "no_design_element",
                "dangling_design_reference",
                "design_kind_mismatch",
                "no_allocation",
                "unbacked_criterion",
                "no_evidence",
                "no_clause_binding",
                "clause_open",
                "evidence_failed",
                "evidence_stale",
                "evidence_suspect",
                "soup_unresolved_advisory",
                "soup_past_end_of_support",
                "soup_field_not_stated",
                "anomaly_unassessed",
                "anomaly_no_release_rationale",
                "risk_chain_incomplete",
                "residual_risk_undecided",
                "no_overall_evaluation",
                "overall_evaluation_undecided",
                "residual_risk_not_considered",
                "evaluation_accepted_with_open_risk",
                "decision_signer_mismatch",
                "decision_commit_unsigned",
                "control_unverified",
                "no_acceptability_policy",
                "no_plan_record",
                "no_snapshot",
                "no_release_verify_input",
                "release_not_verified",
                "crossing_without_threat",
                "classified_sink_without_threat",
                "threat_without_treatment",
                "no_threat_model",
                "no_processing_activity",
                "model_facts_unavailable",
                "personal_data_categories_unstated",
                "recipient_categories_unstated",
                "transfer_region_unstated",
                "transfer_safeguards_unstated",
                "security_measures_unstated",
            ]
        );
        for (index, kind) in GapKind::ALL.iter().enumerate() {
            assert_eq!(kind.position(), index);
        }
    }

    #[trace("FR-191-AC-15")]
    #[test]
    fn document_level_gaps_sort_before_gaps_naming_an_id() {
        let mut gaps = vec![
            Gap::new(GapKind::NoEvidence, "FR-100-AC-10", "none")
                .for_requirement("FR-100")
                .for_criterion("FR-100-AC-10"),
            Gap::new(GapKind::NoAcceptanceCriteria, "FR-99", "none").for_requirement("FR-99"),
            Gap::new(GapKind::NoClauseBinding, "document", "none"),
            Gap::new(GapKind::NoEvidence, "FR-100-AC-2", "none")
                .for_requirement("FR-100")
                .for_criterion("FR-100-AC-2"),
        ];
        sort_gaps(&mut gaps);
        let subjects: Vec<_> = gaps.iter().map(|gap| gap.subject.as_str()).collect();
        assert_eq!(
            subjects,
            ["document", "FR-99", "FR-100-AC-2", "FR-100-AC-10"]
        );
    }

    #[trace("FR-191-AC-14")]
    #[test]
    fn kind_position_breaks_a_tie_on_the_ids() {
        let mut gaps = vec![
            Gap::new(GapKind::NoEvidence, "FR-1-AC-1", "b").for_requirement("FR-1"),
            Gap::new(GapKind::NoAcceptanceCriteria, "FR-1", "a").for_requirement("FR-1"),
        ];
        sort_gaps(&mut gaps);
        assert_eq!(gaps[0].gap_kind, GapKind::NoAcceptanceCriteria);
    }

    #[trace("FR-191-AC-14")]
    #[test]
    fn a_bullet_carries_kind_subject_and_detail_on_one_line() {
        let gap = Gap::new(GapKind::ClauseOpen, "CL-1", "no\nevidence");
        assert_eq!(gap.bullet(), "- `clause_open` CL-1: no evidence");
    }

    #[trace("FR-191-AC-15")]
    #[test]
    fn an_element_gap_sorts_by_its_element_id_and_a_requirement_one_by_its_requirement() {
        let mut gaps = vec![
            Gap::new(GapKind::SafetyClassUnassigned, "FR-2", "none").for_requirement("FR-2"),
            Gap::new(GapKind::SafetyClassUnassigned, "PART-B", "none").for_element("PART-B"),
            Gap::new(GapKind::SafetyClassUnassigned, "PART-A", "none").for_element("PART-A"),
            Gap::new(GapKind::EvidenceStale, "FR-2-AC-1", "none")
                .for_requirement("FR-2")
                .for_criterion("FR-2-AC-1"),
        ];
        sort_gaps(&mut gaps);
        let subjects: Vec<_> = gaps.iter().map(|gap| gap.subject.as_str()).collect();
        assert_eq!(subjects, ["PART-A", "PART-B", "FR-2", "FR-2-AC-1"]);
    }
}
