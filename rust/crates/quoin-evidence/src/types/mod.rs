// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The evidence store's machine-written record vocabulary (FR-030).
//!
//! One module per record family rather than one file per type: the retained
//! `src/evidence/types.ts` is a single 418-line file whose sections are already
//! the modules below, and a reader looking for the binding graph should not
//! have to scroll past the scan records to find it.
//!
//! Every optional field is `Option<T>` with `skip_serializing_if`, never a
//! `null`. The retained source spreads `...(x === undefined ? {} : {x})` at
//! every construction site, so a `null` on the wire would change bytes the
//! store already holds and break NFR-025.

mod assurance;
mod binding;
mod independence;
mod mock;
mod run;
mod scan;
mod trust;

pub use assurance::{
    EvidenceSubject, ExperimentDesign, ExperimentRecord, ExperimentRecordInput, ExperimentResult,
    ExperimentStatus, Observation, ObservationWindow, OperationalEvidenceRecord,
    OperationalEvidenceRecordInput, OperationalOutcome, ProducerProvenance, ProvenanceArtifact,
    SourceState, StoredAssuranceRecord,
};
pub use binding::{Affirmation, BaselineFile, Binding, BindingsFile, EvidenceLineage};
pub use independence::{
    IndependenceAssessment, IndependenceDimension, IndependenceDimensionAssessment,
    IndependencePolicy, IndependenceRequirement, IndependenceStatus,
};
pub use mock::{MockInjection, MockInspectionRecord};
pub use run::{MUTATION_SCORE_METRIC, Outcome, RunEntry, RunRecord};
pub use scan::{Finding, FindingRecord};
pub use trust::{
    ProducerContext, TrustAdapter, TrustAssessment, TrustDecision, TrustDecisionKind,
    TrustEvidenceReference, TrustStatus, TrustTrigger, TrustUse,
};

/// The store schema version written into every record envelope.
///
/// Frozen: FR-100-AC-2 states that reading and re-serializing every store in
/// the ecosystem returns byte-identical records **and** that this value is
/// unchanged. `quoin_store::STORE_SCHEMA_VERSION` is the same number for the
/// change-assurance side; they are restated rather than shared because the two
/// stores version independently and a single constant would couple them.
pub const STORE_SCHEMA_VERSION: u32 = 1;
