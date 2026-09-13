// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What depends on the requirements the caller named (`analysis.ts:263`).

use std::collections::{BTreeMap, BTreeSet};

use quoin_finding_types::AuditReport;
use quoin_quire::model::{AssuranceExport, AssuranceRelation, AssuranceResolution};

use crate::analysis::draft::{Draft, is_requirement_artifact, obligation_owners};
use crate::analysis::paths::{CorpusEdge, shortest_reverse_paths};
use crate::error::Result;
use crate::ids::{ArtifactId, ObligationId, RelationKind, SuiteId};
use crate::model::audit::canonicalize_report;
use crate::model::report::{
    AuditorVerdict, ChangeImpactAnalysis, ChangeImpactRow, GraphGapKind, GraphView, ImpactBinding,
    ImpactObligation,
};
use crate::model::{Binding, DEFAULT_RELATION_KINDS, GraphAnalysisInput};

/// What depends on the requirements the caller named.
///
/// `selected` is the caller's relationship vocabulary: `None` means the eight
/// [`DEFAULT_RELATION_KINDS`], and an **empty slice** means none at all, which
/// is a walk with no edges rather than a walk with the defaults
/// (`analysis.ts:270`).
///
/// # Errors
///
/// [`crate::GraphError::Canonicalization`] when an audit report entry has no
/// canonical spelling.
pub fn analyze_change_impact(
    input: &GraphAnalysisInput,
    requested: &[ArtifactId],
    selected: Option<&[RelationKind]>,
) -> Result<ChangeImpactAnalysis> {
    let mut draft = Draft::new(input, GraphView::ChangeImpact);
    let relation_kinds: BTreeSet<RelationKind> = selected.map_or_else(
        || {
            DEFAULT_RELATION_KINDS
                .iter()
                .map(|kind| RelationKind::new(*kind))
                .collect()
        },
        |kinds| kinds.iter().cloned().collect(),
    );
    let requested: BTreeSet<ArtifactId> = requested.iter().cloned().collect();
    let head = |draft: Draft| {
        (
            draft,
            requested.iter().cloned().collect::<Vec<_>>(),
            relation_kinds.iter().cloned().collect::<Vec<_>>(),
        )
    };

    let Some(bindings) = input.bindings.available() else {
        let (draft, requested, relation_kinds) = head(draft);
        return Ok(ChangeImpactAnalysis {
            base: draft.not_computed(),
            requested,
            relation_kinds,
            rows: Vec::new(),
        });
    };

    let available: BTreeSet<&str> = input
        .assurance
        .relation_kinds
        .iter()
        .map(|kind| kind.kind.as_str())
        .collect();
    for kind in relation_kinds
        .iter()
        .filter(|kind| !available.contains(kind.as_str()))
    {
        draft.gap(
            GraphGapKind::UnknownRelationKind,
            kind.as_str(),
            format!("relationship kind {kind} is absent from the accepted export vocabulary"),
        );
    }
    if draft.has(GraphGapKind::UnknownRelationKind) {
        let (draft, requested, relation_kinds) = head(draft);
        return Ok(ChangeImpactAnalysis {
            base: draft.not_computed(),
            requested,
            relation_kinds,
            rows: Vec::new(),
        });
    }

    let rows = walk(input, bindings, &requested, &relation_kinds, &mut draft)?;
    let (draft, requested, relation_kinds) = head(draft);
    Ok(ChangeImpactAnalysis {
        base: draft.finish(),
        requested,
        relation_kinds,
        rows,
    })
}

/// The walk itself, once the vocabulary and the seeds are settled.
fn walk(
    input: &GraphAnalysisInput,
    bindings: &[Binding],
    requested: &BTreeSet<ArtifactId>,
    relation_kinds: &BTreeSet<RelationKind>,
    draft: &mut Draft,
) -> Result<Vec<ChangeImpactRow>> {
    let requirement_ids: BTreeSet<ArtifactId> = input
        .assurance
        .artifacts
        .iter()
        .filter(|artifact| is_requirement_artifact(&artifact.artifact_type))
        .map(|artifact| ArtifactId::new(artifact.id.clone()))
        .collect();
    let all_ids: BTreeSet<ArtifactId> = input
        .assurance
        .artifacts
        .iter()
        .map(|artifact| ArtifactId::new(artifact.id.clone()))
        .collect();
    let mut seeds = Vec::new();
    for seed in requested {
        if requirement_ids.contains(seed) {
            seeds.push(seed.clone());
        } else {
            draft.gap(
                GraphGapKind::UnknownRequirement,
                seed.as_str(),
                crate::analysis::draft::absent_from_export(seed.as_str()),
            );
        }
    }

    let edges = selected_corpus_edges(
        &input.assurance,
        relation_kinds,
        &requirement_ids,
        &all_ids,
        draft,
    );
    let paths = shortest_reverse_paths(&seeds, &edges);
    let owners = obligation_owners(&input.assurance, true);
    let mut by_owner: BTreeMap<&ArtifactId, Vec<ObligationId>> = BTreeMap::new();
    for obligation in &input.assurance.obligations {
        let id = ObligationId::new(obligation.id.clone());
        for owner in owners.get(&id).into_iter().flatten() {
            by_owner.entry(owner).or_default().push(id.clone());
        }
    }
    let suites = suites_by_obligation(bindings);
    // `canonicalizeAuditReport` runs inside `verdictFor` (`analysis.ts:499`),
    // once per binding. It is a sort, so it is idempotent and its result does
    // not depend on the obligation being asked about; running it once here is
    // the same report.
    let report = canonicalize_report(input.audit.report.clone())?;

    Ok(paths
        .into_iter()
        .map(|(requirement, path)| {
            let mut obligations: Vec<ObligationId> =
                by_owner.get(&requirement).cloned().unwrap_or_default();
            obligations.sort();
            ChangeImpactRow {
                depth: path.path.edges.len(),
                obligations: obligations
                    .into_iter()
                    .map(|obligation| ImpactObligation {
                        bindings: suites
                            .get(&obligation)
                            .into_iter()
                            .flatten()
                            .map(|suite| ImpactBinding {
                                suite: suite.clone(),
                                auditor_verdict: verdict_for(&report, &obligation, draft),
                            })
                            .collect(),
                        requirements: owners.get(&obligation).cloned().unwrap_or_default(),
                        obligation,
                    })
                    .collect(),
                path: path.path,
                requirement,
            }
        })
        .collect())
}

/// The corpus edges the caller's vocabulary selects (`analysis.ts:523`).
///
/// # "corpus" is the spec corpus
///
/// [`AssuranceRelation::Corpus`] is an edge between two documents in the spec
/// corpus — `FR-001 refines StR-002`. It has nothing to do with
/// evidence-corpus accounting, which belongs to engineering-assurance. The
/// name has already sent one reader to the wrong subsystem (quoin#385).
///
/// # Which unusable edges are reported and which are not
///
/// A selected edge that resolves both ends to artifacts the export has, where
/// at least one of them is not a *requirement*, is skipped in silence: it is a
/// well-formed edge that this view simply does not walk. Anything else — an
/// unresolved edge, or one naming an id the export does not carry — is a
/// `dangling-relation` gap.
fn selected_corpus_edges(
    export: &AssuranceExport,
    selection: &BTreeSet<RelationKind>,
    requirement_ids: &BTreeSet<ArtifactId>,
    all_ids: &BTreeSet<ArtifactId>,
    draft: &mut Draft,
) -> Vec<CorpusEdge> {
    let mut edges = Vec::new();
    for relation in &export.relations {
        let AssuranceRelation::Corpus {
            source,
            target,
            edge_type,
            resolution,
            ..
        } = relation
        else {
            continue;
        };
        let edge = CorpusEdge {
            source: ArtifactId::new(source.clone()),
            target: ArtifactId::new(target.clone()),
            edge_type: RelationKind::new(edge_type.clone()),
        };
        if !selection.contains(&edge.edge_type) {
            continue;
        }
        let resolved = *resolution == AssuranceResolution::Resolved;
        let both_requirements =
            requirement_ids.contains(&edge.source) && requirement_ids.contains(&edge.target);
        if resolved
            && all_ids.contains(&edge.source)
            && all_ids.contains(&edge.target)
            && !both_requirements
        {
            continue;
        }
        if resolved && both_requirements {
            edges.push(edge);
        } else {
            draft.gap(
                GraphGapKind::DanglingRelation,
                format!("{}:{}:{}", edge.source, edge.edge_type, edge.target),
                "selected relationship cannot resolve both accepted artifacts",
            );
        }
    }
    edges.sort_by(CorpusEdge::order);
    edges
}

/// `groupBindings()` (`analysis.ts:477`), reduced to what its caller reads.
///
/// The retained function keeps the first binding seen per `(obligation,
/// suite)` and sorts the survivors by suite — and then the only member its
/// caller touches is `suite` (`analysis.ts:346`). So what it computes is the
/// distinct suites bound to each obligation, in order, and that is what this
/// returns. Keeping the whole binding would be keeping it to throw away.
fn suites_by_obligation(bindings: &[Binding]) -> BTreeMap<&ObligationId, BTreeSet<SuiteId>> {
    let mut groups: BTreeMap<&ObligationId, BTreeSet<SuiteId>> = BTreeMap::new();
    for binding in bindings {
        groups
            .entry(&binding.obligation)
            .or_default()
            .insert(binding.suite.clone());
    }
    groups
}

/// What the auditor said about one obligation (`analysis.ts:494`).
///
/// An obligation with a binding and no verdict of any kind is a gap: FR-032
/// was asked and said nothing, which is not the same as saying it is healthy.
fn verdict_for(
    report: &AuditReport,
    obligation: &ObligationId,
    draft: &mut Draft,
) -> AuditorVerdict {
    let verdict = AuditorVerdict {
        findings: report
            .findings
            .iter()
            .filter(|finding| finding.obligation.as_str() == obligation.as_str())
            .cloned()
            .collect(),
        healthy: report
            .healthy
            .iter()
            .filter(|id| id.as_str() == obligation.as_str())
            .map(|id| ObligationId::new(id.clone()))
            .collect(),
        unevaluated: report
            .unevaluated
            .iter()
            .filter(|check| check.obligation.as_str() == obligation.as_str())
            .cloned()
            .collect(),
    };
    if verdict.findings.is_empty() && verdict.healthy.is_empty() && verdict.unevaluated.is_empty() {
        draft.gap(
            GraphGapKind::MissingAuditorVerdict,
            obligation.as_str(),
            format!("{obligation} has no FR-032 finding, healthy result, or unevaluated check"),
        );
    }
    verdict
}
