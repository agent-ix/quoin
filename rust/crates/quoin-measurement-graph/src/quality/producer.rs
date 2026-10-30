// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Who produced a graph-quality observation, and against which plan.
//!
//! Ports the `producer` and `measurement_plan` members of
//! `graphQualityObservationSchema` (`src/measurement/graph-adapters.ts:274-310`).

use std::num::NonZeroU64;

use serde::Deserialize;

use crate::scalars::{RevisionIdentity, ScorerOutputPath};
use crate::wire::{GraphQualityDefinitionVersion, MeasurementPlanRef};

/// Everything that determined what the extractor produced.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    /// The extractor's revision.
    pub extractor_revision: RevisionIdentity,
    /// Which producer contract it emits.
    pub producer_contract_version: NonZeroU64,
    /// The repository-relative path of the configuration file.
    pub configuration_ref: ScorerOutputPath,
    /// The revision of the measured source.
    // BLOCKED-BY QSL-1300 (owner: planner titan:QSL:plan): graph-quality-observation-v1 requires producer.source_revision (additionalProperties false).
    pub source_revision: RevisionIdentity,
    /// The revision of the measured corpus.
    // BLOCKED-BY QSL-1300 (owner: planner titan:QSL:plan): graph-quality-observation-v1 requires producer.corpus_revision (additionalProperties false).
    pub corpus_revision: RevisionIdentity,
    /// The scorer's revision.
    pub scorer_version: RevisionIdentity,
}

/// Which plan, at which definition version, the observation is against.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeasurementPlanReference {
    /// Always `ix://agent-ix/quire-code-rs/MP-001`.
    pub r#ref: MeasurementPlanRef,
    /// Always `quire-code.graph-quality-v1`.
    pub definition_version: GraphQualityDefinitionVersion,
}

impl MeasurementPlanReference {
    /// The plan id the retained code takes from the `ix://` reference.
    ///
    /// `graph-adapters.ts:517` is `ref.split("/").at(-1)`, and the reference is
    /// a literal, so the answer is a constant. It is written as the constant it
    /// is rather than as a split that cannot fail differently.
    pub const PLAN_ID: &'static str = "MP-001";
}

/// Where the scorer's complete output was retained.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawScorerOutput {
    /// The relative, non-drive-qualified path.
    pub path: crate::scalars::ScorerOutputPath,
}
