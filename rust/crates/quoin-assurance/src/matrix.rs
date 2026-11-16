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

use quoin_evidence::types::BindingsFile;
pub use quoin_finding_types::StaleCause;
use quoin_finding_types::{AuditReport, Finding, FindingKind};
use quoin_quire_types::{CoverageMatrixBinder, CoverageMatrixUnbackedClaim, TargetLocus};
use serde::{Deserialize, Serialize};

mod render;
pub use self::render::{design_cell, render_markdown};

/// The `matrix.build` request: the three inputs, supplied whole.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixInput {
    /// quire's `coverage_matrix`, exactly as `quire coverage` emits it: a bare
    /// array, absent when the module declares no `obligations:` source.
    #[serde(default)]
    pub coverage: Option<Vec<MatrixCoverageRequirement>>,
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

/// One projected Quire requirement accepted by the strict `matrix.build`
/// boundary.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixCoverageRequirement {
    /// Scope-relative source document.
    pub document: String,
    /// Quire's optional FR-082 document criticality, preserved as authored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criticality: Option<serde_json::Value>,
    /// Values declared under carried names, absent when none are stated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carried: Option<serde_json::Map<String, serde_json::Value>>,
    /// Projected criteria.
    pub criteria: Vec<MatrixCoverageCriterion>,
}

/// One projected Quire criterion accepted by the strict `matrix.build`
/// boundary.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixCoverageCriterion {
    /// The criterion identifier.
    pub id: String,
    /// Quire's source statement, retained in the reader shape but not rendered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement: Option<String>,
    /// The declared verification method, omitted when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// Quire's binding test symbols.
    pub binders: Vec<CoverageMatrixBinder>,
    /// The exact native selected identity when the upstream payload carries it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<TargetLocus>,
    /// Quire's separate retained record binders, which the evidence axis
    /// receives through `analysis_evidence`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_binders: Option<Vec<serde_json::Value>>,
    /// Diagnostic-only source claims that name this criterion without
    /// providing matrix backing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unbacked_claims: Vec<CoverageMatrixUnbackedClaim>,
    /// The engine's computed static status.
    pub status: String,
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
    /// Quire's diagnostic-only source claims with no matrix backing, verbatim.
    /// Omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unbacked_claims: Vec<CoverageMatrixUnbackedClaim>,
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
    pub criteria: Vec<MatrixDesignCoverageCriterion>,
    /// Design blocks emitted for this document.
    #[serde(default)]
    pub design: Option<Vec<serde_json::Value>>,
}

/// A coverage criterion with its optional design-element references.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MatrixDesignCoverageCriterion {
    /// Coverage criterion fields emitted by quire-rs.
    #[serde(flatten)]
    pub criterion: MatrixCoverageCriterion,
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
    /// The typed cause of a `stale-evidence` finding: `missing-run` or `failed-run`.
    ///
    /// Present only on `stale-evidence` findings; omitted (never `null`) on every
    /// other kind. Copied verbatim from the auditor (FR-115-AC-17..18).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<StaleCause>,
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

/// A finding carries an invalid `cause`: outside the enum or on a non-stale kind.
///
/// The auditor sets `cause` only on `stale-evidence` findings, and only to
/// `missing-run` or `failed-run` (FR-032-AC-18..21). A `cause` on another kind,
/// or a `cause` value outside the enum, is a malformed audit report
/// (FR-115-AC-22..23).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidStaleCause {
    /// The obligation whose finding carries the invalid cause.
    pub obligation: String,
}

impl std::fmt::Display for InvalidStaleCause {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "a finding for {} carries an invalid stale-evidence cause (outside the enum or on a non-stale kind)",
            self.obligation
        )
    }
}

impl std::error::Error for InvalidStaleCause {}

/// The `matrix.build` refusal: contradictory audit or invalid stale cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatrixRefusal {
    /// The audit named one obligation both healthy and not.
    ContradictoryAudit(ContradictoryAudit),
    /// A finding carries an invalid stale-evidence cause.
    InvalidStaleCause(InvalidStaleCause),
}

impl std::fmt::Display for MatrixRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContradictoryAudit(error) => write!(formatter, "{error}"),
            Self::InvalidStaleCause(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for MatrixRefusal {}

impl From<ContradictoryAudit> for MatrixRefusal {
    fn from(error: ContradictoryAudit) -> Self {
        Self::ContradictoryAudit(error)
    }
}

impl From<InvalidStaleCause> for MatrixRefusal {
    fn from(error: InvalidStaleCause) -> Self {
        Self::InvalidStaleCause(error)
    }
}

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
/// [`MatrixRefusal::ContradictoryAudit`] when `audit` names one id in `healthy`
/// and in a finding or unevaluated check. [`MatrixRefusal::InvalidStaleCause`]
/// when a finding carries a `cause` outside the enum or on a non-stale kind.
/// No partial matrix is produced.
#[allow(
    clippy::too_many_lines,
    reason = "one pure projection joins the audit and evidence fields with the optional design axis"
)]
pub fn build(input: &MatrixInput) -> Result<MatrixOutput, MatrixRefusal> {
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
        }
        .into());
    }

    // Validate cause: only on stale-evidence, and only valid values (FR-115-AC-22..23).
    for finding in &input.audit.findings {
        // A stale-evidence finding MAY have a cause (it is optional), but if
        // present it must be one of the two valid spellings; any other kind
        // must not carry one.
        let invalid = if finding.kind == FindingKind::STALE_EVIDENCE {
            finding.cause.is_some_and(|cause| !cause.is_valid())
        } else {
            finding.cause.is_some()
        };
        if invalid {
            return Err(InvalidStaleCause {
                obligation: finding.obligation.clone(),
            }
            .into());
        }
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
    coverage: &[MatrixCoverageRequirement],
) -> Option<Vec<serde_json::Value>> {
    evidence.map(|evidence| analysis_items(identity, evidence, coverage))
}

fn analysis_items(
    identity: Option<&quoin_quire_types::TargetLocus>,
    evidence: &AnalysisEvidenceInput,
    coverage: &[MatrixCoverageRequirement],
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
    criterion: &MatrixCoverageCriterion,
    design_elements: Option<Vec<serde_json::Value>>,
    healthy: &BTreeSet<&str>,
    store_is_empty: bool,
    findings: &BTreeMap<&str, Vec<&Finding>>,
    bindings: &BTreeMap<&str, Vec<BindingDetail>>,
    unevaluated: &BTreeMap<&str, Vec<String>>,
    classes: Option<&serde_json::Value>,
    analysis_evidence: Option<&AnalysisEvidenceInput>,
    coverage: &[MatrixCoverageRequirement],
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
            unbacked_claims: criterion.unbacked_claims.clone(),
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
            cause: finding.cause,
        })
        .collect();
    // Multiple stale-evidence findings are ordered by cause (failed-run before
    // missing-run, a finding with no cause after both), then path, line, symbol and summary
    // (FR-115-AC-24). Kind decides within a bucket; the remaining fields only
    // break a tie between two findings of one kind, so the order never depends
    // on the order the report happened to list them in.
    findings.sort_by(|left, right| {
        bucket(&left.kind).cmp(&bucket(&right.kind)).then_with(|| {
            // Within stale-evidence, order by cause, path, line, symbol, summary
            if left.kind == FindingKind::STALE_EVIDENCE {
                cause_order(left.cause)
                    .cmp(&cause_order(right.cause))
                    .then_with(|| left.path.cmp(&right.path))
                    .then_with(|| left.line.cmp(&right.line))
                    .then_with(|| left.symbol.cmp(&right.symbol))
                    .then_with(|| left.summary.cmp(&right.summary))
            } else {
                left.cmp(right)
            }
        })
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

/// Cause ordering for matrix stale findings: failed-run (0), missing-run (1),
/// absent cause last (2) (FR-115-AC-24).
fn cause_order(cause: Option<StaleCause>) -> u8 {
    match cause {
        Some(StaleCause::FailedRun) => 0,
        Some(StaleCause::MissingRun) => 1,
        Some(StaleCause::Invalid) | None => 2, // absent or invalid: last
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
