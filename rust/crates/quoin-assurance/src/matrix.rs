// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The evidence-backed test matrix (FR-115).
//!
//! quire's `coverage_matrix` says, per criterion, whether a test is **tagged**
//! for it. It cannot say whether that test ran and passed, whether the run is
//! current, or whether the binding behind it still holds. The binding graph
//! and the auditor's report already answer those three questions per
//! obligation, so this module is a join over them and nothing more: it maps
//! the auditor's existing verdicts onto one evidence status per criterion and
//! derives no finding of its own (FR-115-CON-2).
//!
//! Like [`crate::build_case`] it is pure: every input arrives whole and
//! nothing here reads a disk, a clock or a repository (FR-115-CON-1).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use quoin_evidence::types::BindingsFile;
use quoin_finding_types::{AuditReport, Finding, FindingKind};
use quoin_quire_types::{CoverageMatrixBinder, CoverageMatrixCriterion, CoverageMatrixRequirement};
use serde::{Deserialize, Serialize};

/// The `matrix.build` request: the three inputs, supplied whole.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixInput {
    /// quire's `coverage_matrix`, exactly as `quire coverage` emits it: a bare
    /// array, absent when the module declares no `obligations:` source.
    #[serde(default)]
    pub coverage: Option<Vec<CoverageMatrixRequirement>>,
    /// quire-rs `design_matrix` entries, when supplied by `quoin matrix`.
    #[serde(default)]
    pub design_coverage: Option<Vec<MatrixCoverageEntry>>,
    /// The evidence store's binding graph, exactly as persisted.
    pub bindings: BindingsFile,
    /// The auditor's report over the same bindings and obligations.
    pub audit: AuditReport,
    /// The exact FR-078 claims and causes emitted for this selected inventory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analysis_evidence: Option<AnalysisEvidenceInput>,
}

/// FR-078 values supplied by the analysis bridge, preserved without reduction.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisEvidenceInput {
    /// Qualified record claims, each with its exact target locus.
    #[serde(default)]
    pub record_binders: Vec<serde_json::Value>,
    /// Every non-qualified result, including its exact cause and detail.
    #[serde(default)]
    pub evidence_results: Vec<serde_json::Value>,
}

/// The evidence-backed matrix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixOutput {
    /// One entry per requirement id, in ASCII byte order of that id.
    pub requirements: Vec<MatrixRequirement>,
    /// Present exactly when there are no criteria at all: why the matrix is
    /// empty, so a consumer can tell "nothing to show" from "all clear".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// How many criteria state no software safety class (FR-160). Present
    /// exactly when the audit ran under a class policy, which is also when each
    /// criterion carries its class column; absent otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unclassified: Option<usize>,
}

/// One requirement's criteria.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixRequirement {
    /// The requirement id, as [`crate::requirement_of`] derives it.
    pub id: String,
    /// The source documents' design blocks, in repository and document order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design: Option<Vec<serde_json::Value>>,
    /// The requirement's criteria, in `coverage_matrix`'s own relative order.
    pub criteria: Vec<MatrixCriterion>,
}

/// One criterion's row: the static axis verbatim beside the evidence axis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixCriterion {
    /// The obligation id.
    pub id: String,
    /// The engine's design-element references for this criterion.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design_elements: Option<Vec<serde_json::Value>>,
    /// The declared verification method; omitted, never `null`, when the
    /// upstream criterion states none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// quire's static status, verbatim.
    pub static_status: String,
    /// quire's binders, verbatim.
    pub binders: Vec<CoverageMatrixBinder>,
    /// The software safety class the criterion's requirement states, as
    /// written (FR-160). Omitted when it states none, and when no class policy
    /// was given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    /// What the evidence says.
    pub evidence_status: EvidenceStatus,
    /// Everything the evidence status was read from. Total: present on every
    /// criterion, each list empty rather than absent when nothing applies.
    pub evidence_detail: EvidenceDetail,
    /// FR-078 analysis items matching this criterion's exact native identity.
    /// Present on every criterion when the analysis bridge input is supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub analysis_evidence: Option<Vec<serde_json::Value>>,
}

/// One quire-rs `design_matrix` entry at the `matrix.build` boundary.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MatrixCoverageEntry {
    /// Scope-relative source document.
    pub document: String,
    /// Member identity, absent for an unlabelled single member.
    #[serde(default)]
    pub repo: Option<String>,
    /// Document identity, present for design-obliged entries.
    #[serde(default)]
    pub id: Option<String>,
    /// Criteria with their original coverage fields and design references.
    pub criteria: Vec<MatrixCoverageCriterion>,
    /// Design blocks emitted for this document.
    #[serde(default)]
    pub design: Option<Vec<serde_json::Value>>,
}

/// A coverage criterion with its optional design-element references.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MatrixCoverageCriterion {
    /// Coverage criterion fields emitted by quire-rs.
    #[serde(flatten)]
    pub criterion: quoin_quire_types::CoverageMatrixCriterion,
    /// Element references attached by quire-rs when the entry has design.
    #[serde(default)]
    pub design_elements: Option<Vec<serde_json::Value>>,
}

/// The closed, total evidence-status vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EvidenceStatus {
    /// A passing run backs the criterion and the auditor found nothing wrong.
    #[serde(rename = "bound")]
    Bound,
    /// The auditor raised `stale-evidence`.
    #[serde(rename = "stale")]
    Stale,
    /// The auditor distrusts the binding.
    #[serde(rename = "suspect")]
    Suspect,
    /// No passing, trusted evidence for this criterion in a populated store.
    #[serde(rename = "undischarged")]
    Undischarged,
    /// The binding graph is empty: nothing has ever been discharged.
    #[serde(rename = "no run evidence")]
    NoRunEvidence,
}

impl EvidenceStatus {
    /// The wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bound => "bound",
            Self::Stale => "stale",
            Self::Suspect => "suspect",
            Self::Undischarged => "undischarged",
            Self::NoRunEvidence => "no run evidence",
        }
    }
}

/// The evidence a criterion's status was read from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceDetail {
    /// Every audit finding against this id, suspect-class first, then
    /// `stale-evidence`, then the rest, by kind within each group.
    pub findings: Vec<FindingDetail>,
    /// Every binding for this id, ordered by suite.
    pub bindings: Vec<BindingDetail>,
    /// Every check the auditor could not evaluate for this id, lexically.
    pub unevaluated: Vec<String>,
}

/// One audit finding, reduced to what the matrix shows.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FindingDetail {
    /// The finding kind, verbatim.
    pub kind: String,
    /// The auditor's one-line summary, verbatim.
    pub summary: String,
    /// The finding's source path, when it names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The line within `path`, when it names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    /// The symbol the finding is about, when it names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

/// One binding, reduced to what the matrix shows.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingDetail {
    /// The suite that discharged the obligation.
    pub suite: String,
}

/// The audit named one obligation both healthy and not.
///
/// The auditor builds `healthy` disjoint from `findings` and `unevaluated`, so
/// only a hand-edited or merged report can say both. Picking a winner by
/// precedence would render a verdict nobody reached; the request is refused
/// instead (FR-115-CON-4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContradictoryAudit {
    /// The first contradictory obligation id, in byte order.
    pub obligation: String,
}

impl std::fmt::Display for ContradictoryAudit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "the audit report names {} both healthy and in a finding or unevaluated check",
            self.obligation
        )
    }
}

impl std::error::Error for ContradictoryAudit {}

/// What the human class column shows for a criterion whose requirement states
/// no class (FR-160-AC-6). JSON omits the key instead.
pub const UNASSIGNED: &str = "unassigned";

/// The reason an empty population renders.
pub const EMPTY_REASON: &str = "no coverage_matrix criteria: the module declares no `obligations:` source, so there is no criterion to report evidence for";

/// The three kinds the auditor raises when it distrusts a binding.
const SUSPECT_KINDS: [&str; 3] = [
    FindingKind::INSUFFICIENT_INDEPENDENCE,
    FindingKind::MOCKED_CONFIRMATION,
    FindingKind::VACUOUS_EVIDENCE,
];

type DesignSource = (Option<String>, String, Vec<serde_json::Value>);
type DesignSources = BTreeMap<String, Vec<DesignSource>>;

/// A finding kind's precedence bucket: 0 suspect, 1 stale, 2 everything else.
///
/// Everything else includes a kind the auditor adds later: `Finding::kind` is
/// deliberately an open string, and an unrecognised kind is not evidence that
/// the criterion is sound.
pub(crate) fn bucket(kind: &str) -> u8 {
    if SUSPECT_KINDS.contains(&kind) {
        0
    } else if kind == FindingKind::STALE_EVIDENCE {
        1
    } else {
        2
    }
}

/// Build the matrix.
///
/// # Errors
///
/// [`ContradictoryAudit`] when `audit` names one id in `healthy` and in a
/// finding or unevaluated check. No partial matrix is produced.
#[allow(
    clippy::too_many_lines,
    reason = "one pure projection joins the audit and evidence fields with the optional design axis"
)]
pub fn build(input: &MatrixInput) -> Result<MatrixOutput, ContradictoryAudit> {
    let healthy: BTreeSet<&str> = input.audit.healthy.iter().map(String::as_str).collect();
    if let Some(obligation) = input
        .audit
        .findings
        .iter()
        .map(|finding| finding.obligation.as_str())
        .chain(
            input
                .audit
                .unevaluated
                .iter()
                .map(|check| check.obligation.as_str()),
        )
        .filter(|id| healthy.contains(id))
        .min()
    {
        return Err(ContradictoryAudit {
            obligation: obligation.to_owned(),
        });
    }

    let mut findings: BTreeMap<&str, Vec<&Finding>> = BTreeMap::new();
    for finding in &input.audit.findings {
        findings
            .entry(finding.obligation.as_str())
            .or_default()
            .push(finding);
    }
    let mut unevaluated: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for check in &input.audit.unevaluated {
        unevaluated
            .entry(check.obligation.as_str())
            .or_default()
            .push(check.check.clone());
    }
    let mut bindings: BTreeMap<&str, Vec<BindingDetail>> = BTreeMap::new();
    for binding in &input.bindings.bindings {
        bindings
            .entry(binding.obligation.as_str())
            .or_default()
            .push(BindingDetail {
                suite: binding.suite.as_str().to_owned(),
            });
    }
    let store_is_empty = input.bindings.bindings.is_empty();
    // `classReport` is written by the auditor only under a class policy, so its
    // presence is the whole switch for the class column (FR-160-AC-1).
    let class_report = input.audit.other.get("classReport");
    let classes = class_report.and_then(|report| report.get("classes"));

    let mut grouped: BTreeMap<String, Vec<MatrixCriterion>> = BTreeMap::new();
    let mut designs: DesignSources = BTreeMap::new();
    if input.design_coverage.is_none() {
        for criterion in input
            .coverage
            .iter()
            .flatten()
            .flat_map(|requirement| &requirement.criteria)
        {
            add_criterion(
                &mut grouped,
                criterion,
                None,
                &healthy,
                store_is_empty,
                &findings,
                &bindings,
                &unevaluated,
                classes,
                input.analysis_evidence.as_ref(),
                input.coverage.as_deref().unwrap_or_default(),
            );
        }
    }
    if let Some(entries) = &input.design_coverage {
        for entry in entries {
            let requirement_ids: BTreeSet<String> = if entry.criteria.is_empty() {
                entry
                    .id
                    .iter()
                    .map(|id| crate::requirement_of(id).to_owned())
                    .collect()
            } else {
                entry
                    .criteria
                    .iter()
                    .map(|criterion| crate::requirement_of(&criterion.criterion.id).to_owned())
                    .collect()
            };
            if let Some(blocks) = &entry.design {
                for requirement_id in &requirement_ids {
                    designs.entry(requirement_id.clone()).or_default().push((
                        entry.repo.clone(),
                        entry.document.clone(),
                        blocks.clone(),
                    ));
                }
            }
            if entry.criteria.is_empty()
                && let Some(id) = &entry.id
            {
                grouped
                    .entry(crate::requirement_of(id).to_owned())
                    .or_default();
            }
            for criterion in &entry.criteria {
                add_criterion(
                    &mut grouped,
                    &criterion.criterion,
                    criterion.design_elements.clone(),
                    &healthy,
                    store_is_empty,
                    &findings,
                    &bindings,
                    &unevaluated,
                    classes,
                    input.analysis_evidence.as_ref(),
                    input.coverage.as_deref().unwrap_or_default(),
                );
            }
        }
    }

    let has_criteria = grouped.values().any(|criteria| !criteria.is_empty());
    let reason = (!has_criteria).then(|| EMPTY_REASON.to_owned());
    let unclassified = class_report.map(|_| {
        grouped
            .values()
            .flatten()
            .filter(|criterion| criterion.class.is_none())
            .count()
    });
    let requirements = grouped
        .into_iter()
        .map(|(id, criteria)| {
            let design = designs.get_mut(&id).and_then(|documents| {
                documents.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));
                let blocks: Vec<serde_json::Value> = documents
                    .iter()
                    .flat_map(|(_, _, blocks)| blocks.iter().cloned())
                    .collect();
                (!documents.is_empty()).then_some(blocks)
            });
            MatrixRequirement {
                id,
                design,
                criteria,
            }
        })
        .collect();
    Ok(MatrixOutput {
        requirements,
        reason,
        unclassified,
    })
}

fn analysis_evidence_for(
    identity: Option<&quoin_quire_types::TargetLocus>,
    evidence: Option<&AnalysisEvidenceInput>,
    coverage: &[CoverageMatrixRequirement],
) -> Option<Vec<serde_json::Value>> {
    evidence.map(|evidence| analysis_items(identity, evidence, coverage))
}

fn analysis_items(
    identity: Option<&quoin_quire_types::TargetLocus>,
    evidence: &AnalysisEvidenceInput,
    coverage: &[CoverageMatrixRequirement],
) -> Vec<serde_json::Value> {
    let Some(identity) = identity else {
        return Vec::new();
    };
    let identity_count = coverage
        .iter()
        .flat_map(|requirement| &requirement.criteria)
        .filter(|criterion| criterion.identity.as_ref() == Some(identity))
        .count();
    if identity_count != 1 {
        return Vec::new();
    }
    let mut items = Vec::new();
    for binder in &evidence.record_binders {
        if binder.get("target").and_then(|target| {
            serde_json::from_value::<quoin_quire_types::TargetLocus>(target.clone()).ok()
        }) == Some(identity.clone())
        {
            items.push(serde_json::json!({
                "status": "qualified-record",
                "record_binder": binder,
            }));
        }
    }
    for result in &evidence.evidence_results {
        if result.get("target").and_then(|target| {
            serde_json::from_value::<quoin_quire_types::TargetLocus>(target.clone()).ok()
        }) == Some(identity.clone())
            && let Some(status) = result.get("cause")
        {
            items.push(serde_json::json!({ "status": status, "result": result }));
        }
    }
    items.sort_by_key(analysis_item_key);
    items
}

fn analysis_item_key(item: &serde_json::Value) -> (String, String, String, String, String) {
    let locator = item
        .get("record_binder")
        .or_else(|| item.get("result"))
        .and_then(|value| value.get("locator"));
    (
        item.get("status")
            .map_or_else(String::new, serde_json::Value::to_string),
        locator
            .and_then(|value| value.get("record"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        locator
            .and_then(|value| value.get("run"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        locator
            .and_then(|value| value.get("entry"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        item.get("result")
            .and_then(|value| value.get("detail"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "one criterion's evidence and optional design fields are projected together"
)]
fn add_criterion(
    grouped: &mut BTreeMap<String, Vec<MatrixCriterion>>,
    criterion: &CoverageMatrixCriterion,
    design_elements: Option<Vec<serde_json::Value>>,
    healthy: &BTreeSet<&str>,
    store_is_empty: bool,
    findings: &BTreeMap<&str, Vec<&Finding>>,
    bindings: &BTreeMap<&str, Vec<BindingDetail>>,
    unevaluated: &BTreeMap<&str, Vec<String>>,
    classes: Option<&serde_json::Value>,
    analysis_evidence: Option<&AnalysisEvidenceInput>,
    coverage: &[CoverageMatrixRequirement],
) {
    let id = criterion.id.as_str();
    let detail = detail(
        findings.get(id).map_or(&[][..], Vec::as_slice),
        bindings.get(id).map_or(&[][..], Vec::as_slice),
        unevaluated.get(id).map_or(&[][..], Vec::as_slice),
    );
    let evidence_status = if store_is_empty {
        EvidenceStatus::NoRunEvidence
    } else {
        status(healthy.contains(id), &detail)
    };
    grouped
        .entry(crate::requirement_of(id).to_owned())
        .or_default()
        .push(MatrixCriterion {
            id: criterion.id.clone(),
            design_elements,
            method: criterion.method.clone(),
            static_status: criterion.status.clone(),
            binders: criterion.binders.clone(),
            class: classes
                .and_then(|classes| classes.get(id))
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned),
            evidence_status,
            evidence_detail: detail,
            analysis_evidence: analysis_evidence_for(
                criterion.identity.as_ref(),
                analysis_evidence,
                coverage,
            ),
        });
}

/// One criterion's total evidence detail, in the orders FR-115-AC-9 states.
fn detail(findings: &[&Finding], bindings: &[BindingDetail], checks: &[String]) -> EvidenceDetail {
    let mut findings: Vec<FindingDetail> = findings
        .iter()
        .map(|finding| FindingDetail {
            kind: finding.kind.clone(),
            summary: finding.summary.clone(),
            path: finding.path.clone(),
            line: finding.line,
            symbol: finding.symbol.clone(),
        })
        .collect();
    // Kind decides within a bucket; the remaining fields only break a tie
    // between two findings of one kind, so the order never depends on the
    // order the report happened to list them in.
    findings.sort_by(|left, right| {
        bucket(&left.kind)
            .cmp(&bucket(&right.kind))
            .then_with(|| left.cmp(right))
    });
    let mut bindings = bindings.to_vec();
    bindings.sort();
    let mut unevaluated = checks.to_vec();
    unevaluated.sort();
    EvidenceDetail {
        findings,
        bindings,
        unevaluated,
    }
}

/// The status of a criterion in a populated store: bound, then suspect, then
/// stale, then undischarged.
fn status(healthy: bool, detail: &EvidenceDetail) -> EvidenceStatus {
    // `detail.findings` is already sorted by bucket, so its first finding is
    // the one precedence selects.
    status_of(
        healthy,
        detail.findings.first().map(|finding| bucket(&finding.kind)),
    )
}

/// [`status`] from the most serious finding's [`bucket`], for callers that
/// hold findings rather than a built detail.
pub(crate) fn status_of(healthy: bool, worst_bucket: Option<u8>) -> EvidenceStatus {
    if healthy {
        return EvidenceStatus::Bound;
    }
    match worst_bucket {
        Some(0) => EvidenceStatus::Suspect,
        Some(1) => EvidenceStatus::Stale,
        _ => EvidenceStatus::Undischarged,
    }
}

/// Render the matrix as markdown.
///
/// One `##` section per requirement, in the payload's order, each holding one
/// table. An empty matrix renders its reason, never an empty table.
#[must_use]
pub fn render_markdown(matrix: &MatrixOutput) -> String {
    let mut out = String::from("# Test Matrix\n");
    if let Some(reason) = &matrix.reason {
        let _ = write!(out, "\n{reason}\n");
        if matrix.requirements.is_empty() {
            return out;
        }
    }
    let classed = matrix.unclassified.is_some();
    let designed = matrix
        .requirements
        .iter()
        .any(|requirement| requirement.design.is_some());
    for requirement in &matrix.requirements {
        let _ = write!(out, "\n## {}\n", cell(&requirement.id));
        for block in blocks_with_state(requirement.design.as_ref(), "uncovered") {
            if let Some(check) = block.get("check").and_then(serde_json::Value::as_str) {
                let _ = writeln!(out, "\nDesign gap: no design element ({check})");
            }
        }
        if requirement.criteria.is_empty() {
            continue;
        }
        let _ = write!(
            out,
            "\n| Requirement | Criterion | {}Method | Static Status | {}Evidence Status | Detail |\n|---|---|{}---|---|{}---|---|\n",
            if designed { "Design | " } else { "" },
            if classed { "Class | " } else { "" },
            if designed { "---|" } else { "" },
            if classed { "---|" } else { "" },
        );
        for criterion in &requirement.criteria {
            let class = if classed {
                format!(
                    "{} | ",
                    cell(criterion.class.as_deref().unwrap_or(UNASSIGNED))
                )
            } else {
                String::new()
            };
            let detail = detail_text(&criterion.evidence_detail);
            let analysis = criterion
                .analysis_evidence
                .as_ref()
                .into_iter()
                .flatten()
                .map(|item| format!("analysis {item}"))
                .collect::<Vec<_>>()
                .join("; ");
            let detail = if analysis.is_empty() {
                detail
            } else {
                format!("{detail}; {analysis}")
            };
            let design = if designed {
                format!("{} | ", design_cell(requirement, criterion))
            } else {
                String::new()
            };
            let _ = writeln!(
                out,
                "| {} | {} | {}{} | {} | {}{} | {} |",
                cell(&requirement.id),
                cell(&criterion.id),
                design,
                cell(criterion.method.as_deref().unwrap_or("—")),
                cell(&criterion.static_status),
                class,
                criterion.evidence_status.as_str(),
                cell(&detail),
            );
        }
    }
    if let Some(count) = matrix.unclassified {
        let _ = write!(
            out,
            "\n{count} criterion(s) state no software safety class.\n"
        );
    }
    out
}

fn blocks_with_state<'a>(
    design: Option<&'a Vec<serde_json::Value>>,
    state: &str,
) -> impl Iterator<Item = &'a serde_json::Value> {
    design
        .into_iter()
        .flatten()
        .filter(move |block| block.get("state").and_then(serde_json::Value::as_str) == Some(state))
}

fn design_cell(requirement: &MatrixRequirement, criterion: &MatrixCriterion) -> String {
    let Some(blocks) = &requirement.design else {
        return "n/a".to_owned();
    };
    let mut parts = Vec::new();
    for reference in criterion.design_elements.iter().flatten() {
        let id = reference
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let repo = reference.get("repo").and_then(serde_json::Value::as_str);
        let element = blocks
            .iter()
            .flat_map(|block| {
                block
                    .get("elements")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
            })
            .find(|element| {
                element.get("id").and_then(serde_json::Value::as_str) == Some(id)
                    && element.get("repo").and_then(serde_json::Value::as_str) == repo
            });
        let kind = element
            .and_then(|element| element.get("kind"))
            .and_then(serde_json::Value::as_str);
        let qualified_id = repo.map_or_else(|| id.to_owned(), |repo| format!("{repo}/{id}"));
        let label = kind.map_or_else(
            || qualified_id.clone(),
            |kind| format!("{qualified_id} ({kind})"),
        );
        parts.push(label);
    }
    for block in blocks
        .iter()
        .filter(|block| block.get("state").and_then(serde_json::Value::as_str) == Some("uncovered"))
    {
        if let Some(check) = block.get("check").and_then(serde_json::Value::as_str) {
            parts.push(format!("no design element ({check})"));
        }
    }
    for block in blocks.iter().filter(|block| {
        block.get("state").and_then(serde_json::Value::as_str) == Some("covered")
            && block
                .get("elements")
                .and_then(serde_json::Value::as_array)
                .is_some_and(Vec::is_empty)
    }) {
        for dangling in block
            .get("dangling")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(reference) = dangling
                .get("reference")
                .and_then(serde_json::Value::as_str)
            {
                parts.push(format!("-> {reference} (dangling)"));
            }
        }
    }
    for block in blocks {
        for mismatch in block
            .get("kind_mismatch")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let id = mismatch
                .get("id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let repo = mismatch.get("repo").and_then(serde_json::Value::as_str);
            let id = repo.map_or_else(|| id.to_owned(), |repo| format!("{repo}/{id}"));
            let archetype = mismatch
                .get("archetype")
                .and_then(serde_json::Value::as_str);
            parts.push(archetype.map_or_else(
                || format!("kind mismatch: {id}"),
                |kind| format!("kind mismatch: {id} ({kind})"),
            ));
        }
    }
    if parts.is_empty() {
        "(none)".to_owned()
    } else {
        parts.join(", ")
    }
}

/// The detail column: findings, then bindings, then unevaluated checks.
fn detail_text(detail: &EvidenceDetail) -> String {
    let mut parts: Vec<String> = detail
        .findings
        .iter()
        .map(|finding| format!("{}: {}", finding.kind, finding.summary))
        .collect();
    parts.extend(
        detail
            .bindings
            .iter()
            .map(|binding| format!("bound by {}", binding.suite)),
    );
    parts.extend(
        detail
            .unevaluated
            .iter()
            .map(|check| format!("not evaluated: {check}")),
    );
    if parts.is_empty() {
        "—".to_owned()
    } else {
        parts.join("; ")
    }
}

/// One table cell: a `|` would end the cell and a newline the row.
fn cell(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace(['\r', '\n'], " ")
}
