// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The three reports, and the shared head every one of them carries.
//!
//! Member names are the retained ones, camel case included
//! (`obligationCount`, `unresolvedBindings`, `relationKinds`,
//! `auditorVerdict`, `eventCount`). They are the bytes a consumer already
//! reads, so they are `#[serde(rename)]`d rather than tidied.
//!
//! Member *order* is not declared anywhere here, and does not need to be: the
//! canonical writer orders every member itself. What a field ordering would
//! control is the one thing this crate does not get to choose.

use std::cmp::Ordering;

use crate::ids::compare_text;
use quoin_finding_types::{Finding, UnevaluatedCheck};
use quoin_quire::model::AssuranceExport;
use serde::Serialize;

use crate::ids::{ArtifactId, ObligationId, RelationKind, SuiteId};
use crate::model::audit::AuditEnvelope;
use crate::model::binding::BindingInput;
use crate::model::premises::AcceptedPremises;

/// Everything one analysis reads.
#[derive(Debug, Clone)]
pub struct GraphAnalysisInput {
    /// The accepted assurance export.
    pub assurance: AssuranceExport,
    /// The premises the caller accepted it under.
    pub premises: AcceptedPremises,
    /// The FR-032 audit taken at the same identity.
    pub audit: AuditEnvelope,
    /// The retained bindings, or why there are none.
    pub bindings: BindingInput,
}

/// Which projection a report is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphView {
    /// Which suites carry which live obligations.
    FanOut,
    /// What depends on the requirements the caller named.
    ChangeImpact,
}

impl GraphView {
    /// The wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FanOut => "fan-out",
            Self::ChangeImpact => "change-impact",
        }
    }
}

/// How complete the analysis is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphAnalysisState {
    /// Nothing is missing.
    Complete,
    /// Something is missing, and the gaps say what.
    Incomplete,
    /// The bindings store was not available, so nothing was computed.
    NotComputed,
}

impl GraphAnalysisState {
    /// The wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Incomplete => "incomplete",
            Self::NotComputed => "not_computed",
        }
    }
}

/// What kind of thing is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphGapKind {
    /// The bindings store is not there.
    AbsentBindingsStore,
    /// A selected relationship does not resolve both accepted artifacts.
    DanglingRelation,
    /// The bindings store is there, valid, and empty.
    EmptyBindingsStore,
    /// An obligation has no FR-032 verdict of any kind.
    MissingAuditorVerdict,
    /// A declared required relation is not satisfied.
    MissingRequiredRelation,
    /// A relation's availability could not be determined.
    UnknownRelationAvailability,
    /// A requested relationship kind is not in the export's vocabulary.
    UnknownRelationKind,
    /// A requested requirement is not in the export.
    UnknownRequirement,
    /// The bindings store is there and does not parse.
    UnreadableBindingsStore,
    /// A binding names an obligation the export does not have.
    UnresolvedBinding,
    /// An obligation's document identifies no accepted artifact.
    UnresolvedObligationOwner,
}

impl GraphGapKind {
    /// The wire spelling, which is also what the markdown prints.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AbsentBindingsStore => "absent-bindings-store",
            Self::DanglingRelation => "dangling-relation",
            Self::EmptyBindingsStore => "empty-bindings-store",
            Self::MissingAuditorVerdict => "missing-auditor-verdict",
            Self::MissingRequiredRelation => "missing-required-relation",
            Self::UnknownRelationAvailability => "unknown-relation-availability",
            Self::UnknownRelationKind => "unknown-relation-kind",
            Self::UnknownRequirement => "unknown-requirement",
            Self::UnreadableBindingsStore => "unreadable-bindings-store",
            Self::UnresolvedBinding => "unresolved-binding",
            Self::UnresolvedObligationOwner => "unresolved-obligation-owner",
        }
    }
}

/// One thing the analysis could not account for.
///
/// `Ord` is `uniqueGaps`' ordering (`analysis.ts:705`) — kind, then subject,
/// then reason, all UTF-16 — so a `BTreeSet<GraphGap>` *is* `uniqueGaps`: it
/// dedupes on the same triple the retained map keys on and iterates in the
/// order the retained sort produces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphGap {
    /// What kind of gap.
    pub kind: GraphGapKind,
    /// What it is about.
    pub subject: String,
    /// Why it is a gap.
    pub reason: String,
}

impl Ord for GraphGap {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_text(self.kind.as_str(), other.kind.as_str())
            .then_with(|| compare_text(&self.subject, &other.subject))
            .then_with(|| compare_text(&self.reason, &other.reason))
    }
}

impl PartialOrd for GraphGap {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The export identity a report repeats back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportIdentity {
    /// The format the export declared.
    pub format: String,
    /// The format version the export declared.
    pub format_version: u32,
}

/// The head every report carries.
#[derive(Debug, Clone, Serialize)]
pub struct GraphReportBase {
    /// Which projection this is.
    pub view: GraphView,
    /// The identity the export declared for itself.
    pub export: ExportIdentity,
    /// The premises the caller accepted.
    pub premises: AcceptedPremises,
    /// How complete this analysis is.
    pub state: GraphAnalysisState,
    /// What it could not account for.
    pub gaps: Vec<GraphGap>,
}

/// One obligation and the accepted artifacts that own its document.
#[derive(Debug, Clone, Serialize)]
pub struct OwnedObligation {
    /// The obligation.
    pub obligation: ObligationId,
    /// The artifacts whose locator path is the obligation's document.
    pub requirements: Vec<ArtifactId>,
}

/// One suite's row in the fan-out view.
#[derive(Debug, Clone, Serialize)]
pub struct FanOutRow {
    /// The suite.
    pub suite: SuiteId,
    /// Its live obligations.
    pub obligations: Vec<OwnedObligation>,
    /// How many there are.
    #[serde(rename = "obligationCount")]
    pub obligation_count: usize,
    /// Bindings on this suite that name obligations the export does not have.
    #[serde(rename = "unresolvedBindings")]
    pub unresolved_bindings: Vec<ObligationId>,
}

/// Which suites carry which live obligations.
#[derive(Debug, Clone, Serialize)]
pub struct FanOutAnalysis {
    /// The shared head.
    #[serde(flatten)]
    pub base: GraphReportBase,
    /// One row per suite that has any binding at all.
    pub rows: Vec<FanOutRow>,
}

/// One step along a reverse dependency path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImpactPathEdge {
    /// The dependent.
    pub source: ArtifactId,
    /// What it depends on.
    pub target: ArtifactId,
    /// How.
    pub relationship: RelationKind,
}

/// The shortest route from a seed to one dependent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImpactPath {
    /// The requested requirement the route starts at.
    pub seed: ArtifactId,
    /// The steps, seed-first.
    pub edges: Vec<ImpactPathEdge>,
}

/// What the auditor said about one obligation.
#[derive(Debug, Clone, Serialize)]
pub struct AuditorVerdict {
    /// Its findings, carried verbatim.
    pub findings: Vec<Finding>,
    /// Whether it was reported healthy.
    pub healthy: Vec<ObligationId>,
    /// Its unevaluated checks, carried verbatim.
    pub unevaluated: Vec<UnevaluatedCheck>,
}

/// One suite bound to an obligation, with the auditor's verdict on it.
#[derive(Debug, Clone, Serialize)]
pub struct ImpactBinding {
    /// The suite.
    pub suite: SuiteId,
    /// The verdict.
    #[serde(rename = "auditorVerdict")]
    pub auditor_verdict: AuditorVerdict,
}

/// One obligation reached by the impact walk.
#[derive(Debug, Clone, Serialize)]
pub struct ImpactObligation {
    /// The obligation.
    pub obligation: ObligationId,
    /// The artifacts that own its document.
    pub requirements: Vec<ArtifactId>,
    /// Its bindings.
    pub bindings: Vec<ImpactBinding>,
}

/// One requirement reached from a seed.
#[derive(Debug, Clone, Serialize)]
pub struct ChangeImpactRow {
    /// The requirement.
    pub requirement: ArtifactId,
    /// How many edges away from its seed it is.
    pub depth: usize,
    /// The route.
    pub path: ImpactPath,
    /// The obligations it owns.
    pub obligations: Vec<ImpactObligation>,
}

/// What depends on the requirements the caller named.
#[derive(Debug, Clone, Serialize)]
pub struct ChangeImpactAnalysis {
    /// The shared head.
    #[serde(flatten)]
    pub base: GraphReportBase,
    /// The seeds, deduplicated and ordered.
    pub requested: Vec<ArtifactId>,
    /// The relationship kinds walked, deduplicated and ordered.
    #[serde(rename = "relationKinds")]
    pub relation_kinds: Vec<RelationKind>,
    /// One row per reached requirement.
    pub rows: Vec<ChangeImpactRow>,
}

/// Any one of the two reports.
///
/// `untagged`: the discriminant is already in the payload as `view`, and a
/// second one would be a member the retained format does not have.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum GraphAnalysis {
    /// The fan-out view.
    FanOut(FanOutAnalysis),
    /// The change-impact view.
    ChangeImpact(ChangeImpactAnalysis),
}

impl GraphAnalysis {
    /// The head, whichever view this is.
    #[must_use]
    pub const fn base(&self) -> &GraphReportBase {
        match self {
            Self::FanOut(analysis) => &analysis.base,
            Self::ChangeImpact(analysis) => &analysis.base,
        }
    }
}

impl From<FanOutAnalysis> for GraphAnalysis {
    fn from(analysis: FanOutAnalysis) -> Self {
        Self::FanOut(analysis)
    }
}

impl From<ChangeImpactAnalysis> for GraphAnalysis {
    fn from(analysis: ChangeImpactAnalysis) -> Self {
        Self::ChangeImpact(analysis)
    }
}
