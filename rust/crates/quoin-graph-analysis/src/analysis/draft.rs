// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The head all three views share, and the gap set they all accumulate into.
//!
//! `base()` (`analysis.ts:359`) and `finish()` (`:437`) are one type here. The
//! retained pair passes a half-built report around and calls `uniqueGaps` at
//! each end; [`Draft`] holds the gaps in a [`BTreeSet`] instead, which *is*
//! `uniqueGaps`: the set dedupes on the same `(kind, subject, reason)` triple
//! the retained map keys on, and [`crate::model::GraphGap`]'s `Ord` is the
//! same three-way UTF-16 comparison the retained sort ends with.

use std::collections::{BTreeMap, BTreeSet};

use quoin_quire::model::{AssuranceExport, AssuranceSource, RelationAvailability};

use crate::ids::{ArtifactId, ObligationId};
use crate::model::report::{
    ExportIdentity, GraphAnalysisState, GraphGap, GraphGapKind, GraphReportBase, GraphView,
};
use crate::model::{
    AcceptedPremises, BindingInput, GraphAnalysisInput, REQUIREMENT_ARTIFACT_TYPES,
};

/// Which obligation is owned by which accepted artifacts.
pub(crate) type Owners = BTreeMap<ObligationId, Vec<ArtifactId>>;

/// A report head under construction.
pub(crate) struct Draft {
    view: GraphView,
    source: AssuranceSource,
    export: ExportIdentity,
    premises: AcceptedPremises,
    gaps: BTreeSet<GraphGap>,
}

impl Draft {
    /// `base()` (`analysis.ts:359`).
    pub(crate) fn new(input: &GraphAnalysisInput, view: GraphView) -> Self {
        let mut draft = Self {
            view,
            source: input.assurance.source.clone(),
            export: ExportIdentity {
                format: input.assurance.format.clone(),
                format_version: input.assurance.format_version,
            },
            premises: input.premises.canonicalized(),
            gaps: BTreeSet::new(),
        };
        draft.store_availability(&input.bindings);
        draft.unresolved_bindings(input);
        draft.relation_observations(&input.assurance);
        draft.unowned_obligations(&input.assurance);
        draft
    }

    /// Record one gap. Re-recording an identical one is the retained
    /// `Map.set` on the same key: it changes nothing.
    pub(crate) fn gap(
        &mut self,
        kind: GraphGapKind,
        subject: impl Into<String>,
        reason: impl Into<String>,
    ) {
        self.gaps.insert(GraphGap {
            kind,
            subject: subject.into(),
            reason: reason.into(),
        });
    }

    /// Whether any gap of this kind has been recorded (`analysis.ts:293`).
    pub(crate) fn has(&self, kind: GraphGapKind) -> bool {
        self.gaps.iter().any(|gap| gap.kind == kind)
    }

    /// `finish()` (`analysis.ts:437`) for a view that ran.
    pub(crate) fn finish(self) -> GraphReportBase {
        let state = if self.gaps.is_empty() {
            GraphAnalysisState::Complete
        } else {
            GraphAnalysisState::Incomplete
        };
        self.seal(state)
    }

    /// `finish()` for a view that could not run: the state is fixed and the
    /// gaps say why (`analysis.ts:157`, `:200`, `:279`, `:295`).
    pub(crate) fn not_computed(self) -> GraphReportBase {
        self.seal(GraphAnalysisState::NotComputed)
    }

    fn seal(self, state: GraphAnalysisState) -> GraphReportBase {
        GraphReportBase {
            view: self.view,
            source: self.source,
            export: self.export,
            premises: self.premises,
            state,
            gaps: self.gaps.into_iter().collect(),
        }
    }

    fn store_availability(&mut self, bindings: &BindingInput) {
        const SUBJECT: &str = "bindings.json";
        match bindings {
            BindingInput::Absent { reason } => {
                self.gap(GraphGapKind::AbsentBindingsStore, SUBJECT, reason.clone());
            }
            BindingInput::Unreadable { reason } => {
                self.gap(
                    GraphGapKind::UnreadableBindingsStore,
                    SUBJECT,
                    reason.clone(),
                );
            }
            BindingInput::Available(bindings) if bindings.is_empty() => self.gap(
                GraphGapKind::EmptyBindingsStore,
                SUBJECT,
                "the bindings store is present and valid but contains no bindings",
            ),
            BindingInput::Available(_) => {}
        }
    }

    fn unresolved_bindings(&mut self, input: &GraphAnalysisInput) {
        let Some(bindings) = input.bindings.available() else {
            return;
        };
        let live = live_obligations(&input.assurance);
        for binding in bindings {
            if !live.contains(&binding.obligation) {
                self.gap(
                    GraphGapKind::UnresolvedBinding,
                    format!("{}:{}", binding.suite, binding.obligation),
                    absent_from_export(binding.obligation.as_str()),
                );
            }
        }
    }

    fn relation_observations(&mut self, export: &AssuranceExport) {
        for observation in &export.relation_observations {
            let (kind, fallback) = match observation.availability {
                RelationAvailability::Unknown => (
                    GraphGapKind::UnknownRelationAvailability,
                    format!("availability of {} is unknown", observation.declaration),
                ),
                RelationAvailability::Missing => (
                    GraphGapKind::MissingRequiredRelation,
                    format!("{} is not satisfied", observation.declaration),
                ),
                RelationAvailability::Available | RelationAvailability::NotApplicable => continue,
            };
            let subject = observation
                .subject
                .clone()
                .unwrap_or_else(|| observation.declaration.clone());
            let reason = observation.reason.clone().unwrap_or(fallback);
            self.gap(kind, subject, reason);
        }
    }

    fn unowned_obligations(&mut self, export: &AssuranceExport) {
        let owners = obligation_owners(export, false);
        for obligation in &export.obligations {
            let id = ObligationId::new(obligation.id.clone());
            if owners.get(&id).is_none_or(Vec::is_empty) {
                self.gap(
                    GraphGapKind::UnresolvedObligationOwner,
                    obligation.id.clone(),
                    format!(
                        "{} does not identify an accepted artifact",
                        obligation.document
                    ),
                );
            }
        }
    }
}

/// The one sentence two views both use for a binding the export does not know
/// (`analysis.ts:176`, `:391`).
pub(crate) fn absent_from_export(obligation: &str) -> String {
    format!("{obligation} is absent from the accepted assurance export")
}

/// The obligations the export declares.
pub(crate) fn live_obligations(export: &AssuranceExport) -> BTreeSet<ObligationId> {
    export
        .obligations
        .iter()
        .map(|obligation| ObligationId::new(obligation.id.clone()))
        .collect()
}

/// `obligationOwners()` (`analysis.ts:451`).
///
/// The key set is every obligation the export declares, so `owners.keys()` is
/// the retained `live` set (`analysis.ts:160`). A duplicate obligation id
/// overwrites, which is what `new Map(...)` does with one.
pub(crate) fn obligation_owners(export: &AssuranceExport, requirements_only: bool) -> Owners {
    let mut by_path: BTreeMap<&str, Vec<ArtifactId>> = BTreeMap::new();
    for artifact in &export.artifacts {
        if requirements_only && !is_requirement_artifact(&artifact.artifact_type) {
            continue;
        }
        by_path
            .entry(artifact.locator.path.as_str())
            .or_default()
            .push(ArtifactId::new(artifact.id.clone()));
    }
    for group in by_path.values_mut() {
        group.sort();
    }
    export
        .obligations
        .iter()
        .map(|obligation| {
            (
                ObligationId::new(obligation.id.clone()),
                by_path
                    .get(obligation.document.as_str())
                    .cloned()
                    .unwrap_or_default(),
            )
        })
        .collect()
}

/// `isRequirementArtifact()` (`analysis.ts:471`).
pub(crate) fn is_requirement_artifact(artifact_type: &str) -> bool {
    REQUIREMENT_ARTIFACT_TYPES.contains(&artifact_type)
}
