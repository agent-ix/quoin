// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! What the auditor was given to read.
//!
//! Every field is data the caller assembled. There is no handle, no path to
//! open and no command to run in this struct, and [`crate::inert`] is what
//! makes that a fact rustc checks rather than a claim this comment makes.

use std::collections::BTreeMap;

use quoin_evidence::types::{
    Binding, FindingRecord, IndependenceAssessment, IndependencePolicy, MockInjection, RunRecord,
};
use quoin_quire_types::{CoverageMatrixRequirement, Obligation};
use serde::Deserialize;

use super::class::ClassPolicy;
use crate::catalog::MethodCatalog;

/// Everything one audit reads.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct AuditInput {
    /// Obligations as quire derives them today.
    #[serde(default)]
    pub obligations: Vec<Obligation>,
    /// The binding graph from the store.
    #[serde(default)]
    pub bindings: Vec<Binding>,
    /// Every run record the store holds, newest per suite.
    #[serde(default)]
    pub runs: Vec<RunRecord>,
    /// What each suite's tests INJECT in place of real behaviour (#204).
    ///
    /// Supplied by the caller because the auditor reads the store, not source.
    /// An absent list means "nobody looked", never "nothing was mocked": the
    /// result is recorded under `unevaluated`, not counted as healthy.
    #[serde(default)]
    pub injections: Option<Vec<MockInjection>>,
    /// Suites with a current completed inspection, including clean inspections.
    #[serde(default)]
    pub mock_inspection_suites: Option<Vec<String>>,
    /// Suites whose newest finding-shaped scan evaluated no rules (FR-034).
    ///
    /// Answered by the store rather than recomputed here, and answered as a
    /// closed list: a tool that reported no rule count is NOT in it. An absent
    /// list means nothing was asked, never that nothing was vacuous.
    #[serde(default)]
    pub vacuous_scan_suites: Option<Vec<String>>,
    /// One assessment per policy requirement, already made against the
    /// bindings.
    ///
    /// Supplied rather than computed for the same reason as `injections`: the
    /// assessment is the store's judgement over the binding graph, and the
    /// auditor reads.
    #[serde(default)]
    pub independence: Option<Vec<IndependenceAssessment>>,
    /// Every finding-shaped scan record the store holds, newest per suite.
    ///
    /// Separate from `runs` because the two answer different questions. A scan
    /// has no symbols and no pass/fail, so every check that reasons over
    /// `entries` is meaningless against it.
    #[serde(default)]
    pub scans: Option<Vec<FindingRecord>>,
    /// The merged verification-method catalog, for method conformance.
    #[serde(default)]
    pub catalog: Option<MethodCatalog>,
    /// Criticality values that demand two independent methods.
    #[serde(default)]
    pub multiplicity_requires: Option<Vec<String>>,
    /// Criticality value → the mutation score its obligations must reach.
    ///
    /// **Unset by default**, for the reason CR-008 removed the hardcoded
    /// `multiplicityRequires: ["P0"]`: a built-in floor is a rule that fires on
    /// everything the moment a criticality column appears, and nobody chose it.
    #[serde(default)]
    pub mutation_floor: Option<BTreeMap<String, f64>>,
    /// Exact obligations and separation axes selected by an `AssuranceProfile`.
    #[serde(default)]
    pub independence_policy: Option<IndependencePolicy>,
    /// What each software safety class demands (FR-160), a person's data.
    ///
    /// **Unset by default**, and no class has a built-in floor: with no policy
    /// the audit reads no class and adds nothing to its report.
    #[serde(default)]
    pub class_policy: Option<ClassPolicy>,
    /// The ordered legal values of each carried field, from the coverage
    /// report. A class policy is refused without the entry for its class field.
    #[serde(default)]
    pub carried_vocabulary: Option<BTreeMap<String, Vec<String>>>,
    /// quire's `coverage_matrix`: where each requirement's stated class is read
    /// from (`carried`), joined to its obligations by criterion id.
    #[serde(default)]
    pub coverage_matrix: Option<Vec<CoverageMatrixRequirement>>,
}
