// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The assurance-case view (quoin#384, Stage 5 of the burn-down).
//!
//! Ported from `src/assurance/`, which its own header describes as "strictly a
//! view": it collects nothing, computes no coverage and writes nothing to the
//! store. That property is why this is the first real capability across the
//! boundary — there is no IO to model, so a difference between the two
//! implementations is a difference in the logic and nowhere else.
//!
//! # Why this crate has no dependencies yet
//!
//! The retained module's five cross-directory imports are all `import type`,
//! which is what makes it cycle-free. It does **not** make it dependency-free
//! here: `build_case` needs `Obligation`, `BundleDocument`, `Finding`,
//! `TrustAssessment` and `IndependenceAssessment` to exist as Rust types
//! before it can be written, and four of those five belong to crates that do
//! not exist yet.
//!
//! So the port started with the part of the retained surface that names no
//! imported type at all. That was not a shortcut: it put a real retained
//! capability through the whole pattern — request shape, exit taxonomy,
//! differential comparison against the retained implementation — on a surface
//! where a failure is legible.
//!
//! # What measuring the field subset changed
//!
//! Sizing from the import list said four missing crates. Measuring the FIELD
//! SUBSET `build_case` can observe said two: twelve fields across five types,
//! and two of the five never read at all. [`case`] states the budget in full
//! and gives the reason for each type that did not get one. The general form
//! is that an import list sizes a crate from the module boundary, while a
//! field subset sizes it from what the code can observe, and those differ by a
//! factor of five here (quoin#425).

pub mod argument;
pub mod authored;
pub mod case;
pub mod discharge;
pub mod matrix;
pub mod render;

pub use argument::{
    ArgumentError, ArgumentKind, ArgumentStatus, Assumption, AssumptionStatus, AssuranceArgument,
    Challenge, ChallengeStatus, Participant, Reasoning, Relationship, RelationshipType, TopClaim,
    parse_assurance_argument,
};
pub use authored::{
    ArgumentSummary, AssumptionView, AuthoredArgumentView, BuildAuthoredArgumentRequest,
    ChallengeView, ChallengeViewStatus, CriterionView, DecisionState, EvidenceIndexEntry,
    EvidenceRefState, ReasoningView, SufficiencyDecision, TopClaimView, UnusedDecision,
    ViewSchemaVersion, ViewStatus, build_authored_argument_view, render_authored_argument,
};
pub use case::{AssuranceCase, CaseInput, CaseNode, NodeKind, NodeStatus, Unreadable, build_case};
pub use discharge::{
    BuildDischargeRequest, ClauseDischarge, CompletenessSchemaVersion, DISCHARGE_DIR,
    DirectDischargeFact, DischargeAttestation, DischargeBinding, DischargeError, DischargeFact,
    DischargeReport, DischargeSchemaVersion, DischargeState, DischargeStoreError,
    DispositionCounts, DispositionDecision, DispositionFact, FactKind, InwardCompleteness,
    MAX_ID_BYTES, MAX_STORED_FACT_BYTES, MAX_STORED_FACTS, OpenClause, PackIndexEntry,
    PackRequirementsMapping, PackRequirementsMappingSchemaVersion, RecordDischargeRequest,
    RecordedFact, RequirementsMapping, RequirementsMappingSchemaVersion, RmmApplicability,
    RmmCompliance, RmmRow, StoreRefusal, StoredDischargeFact, StoredFactSchemaVersion,
    UnusedDischargeFact, UnusedFactReason, build_discharge_report, build_pack_requirements_mapping,
    build_requirements_mapping, discharge_report_from_store, inward_completeness_from_store,
    read_discharge_facts, record_discharge_facts, render_discharge_report,
    render_inward_completeness, render_pack_requirements_mapping, render_requirements_mapping,
};
pub use render::{RenderableCase, render_case};

pub use quoin_quire_types::requirement_of;

#[cfg(test)]
mod tests {
    use super::requirement_of;
    use ix_trace_rs::trace;

    /// Provenance: agent-ix/quoin#447, agent-ix/quoin#384
    #[trace("FR-040-AC-7")]
    #[test]
    fn tc_447_420_an_obligation_id_names_the_requirement_it_belongs_to() {
        assert_eq!(requirement_of("FR-001-AC-3"), "FR-001");
        assert_eq!(requirement_of("NFR-010-M-2"), "NFR-010");
        assert_eq!(requirement_of("StR-009-VC-1"), "StR-009");
    }

    /// Provenance: agent-ix/quoin#447, agent-ix/quoin#384
    #[trace("FR-040-AC-7")]
    #[test]
    fn tc_447_421_the_suffix_is_read_by_shape_and_never_by_an_allow_list() {
        // The defect a suffix allow-list produces, asserted directly.
        assert_eq!(requirement_of("NFR-013-M-1"), "NFR-013");
        assert_eq!(requirement_of("FR-042-QQQ-9"), "FR-042");
        assert_eq!(requirement_of("US-024-EX-5"), "US-024");
    }

    /// Provenance: agent-ix/quoin#447, agent-ix/quoin#384
    #[trace("FR-040-AC-7")]
    #[test]
    fn tc_447_422_an_id_that_is_not_an_obligation_comes_back_unchanged() {
        assert_eq!(requirement_of("not-an-id"), "not-an-id");
        assert_eq!(requirement_of(""), "");
        assert_eq!(requirement_of("123-456"), "123-456");
        assert_eq!(requirement_of("FR"), "FR");
        assert_eq!(requirement_of("FR-"), "FR-");
    }

    /// Provenance: agent-ix/quoin#447, agent-ix/quoin#384
    #[trace("FR-040-AC-7")]
    #[test]
    fn tc_447_423_a_bare_requirement_id_is_its_own_requirement() {
        assert_eq!(requirement_of("FR-001"), "FR-001");
    }

    /// Provenance: agent-ix/quoin#447, agent-ix/quoin#384
    #[trace("FR-040-AC-7")]
    #[test]
    fn tc_447_424_a_multi_byte_character_falls_to_unchanged_rather_than_panicking() {
        // The slice above is only safe because every consumed byte is ASCII.
        // A multi-byte character before the pattern must therefore fall to the
        // unchanged path rather than panic.
        assert_eq!(requirement_of("é-001-AC-1"), "é-001-AC-1");
    }
}
