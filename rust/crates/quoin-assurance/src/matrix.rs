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
use quoin_quire_types::{CoverageMatrixBinder, CoverageMatrixRequirement};
use serde::{Deserialize, Serialize};

/// The `matrix.build` request: the three inputs, supplied whole.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixInput {
    /// quire's `coverage_matrix`, exactly as `quire coverage` emits it: a bare
    /// array, absent when the module declares no `obligations:` source.
    #[serde(default)]
    pub coverage: Option<Vec<CoverageMatrixRequirement>>,
    /// The evidence store's binding graph, exactly as persisted.
    pub bindings: BindingsFile,
    /// The auditor's report over the same bindings and obligations.
    pub audit: AuditReport,
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
}

/// One requirement's criteria.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixRequirement {
    /// The requirement id, as [`crate::requirement_of`] derives it.
    pub id: String,
    /// The requirement's criteria, in `coverage_matrix`'s own relative order.
    pub criteria: Vec<MatrixCriterion>,
}

/// One criterion's row: the static axis verbatim beside the evidence axis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixCriterion {
    /// The obligation id.
    pub id: String,
    /// The declared verification method; omitted, never `null`, when the
    /// upstream criterion states none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// quire's static status, verbatim.
    pub static_status: String,
    /// quire's binders, verbatim.
    pub binders: Vec<CoverageMatrixBinder>,
    /// What the evidence says.
    pub evidence_status: EvidenceStatus,
    /// Everything the evidence status was read from. Total: present on every
    /// criterion, each list empty rather than absent when nothing applies.
    pub evidence_detail: EvidenceDetail,
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
    /// Every binding for this id, ordered by `(suite, commit)`.
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
///
/// `commit` is the binding's own field: the run that FIRST discharged the
/// obligation. It is not the run that made the evidence stale, and this view
/// does not claim it is.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindingDetail {
    /// The suite that discharged the obligation.
    pub suite: String,
    /// The commit of the run that first discharged it.
    pub commit: String,
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

/// The reason an empty population renders.
pub const EMPTY_REASON: &str = "no coverage_matrix criteria: the module declares no `obligations:` source, so there is no criterion to report evidence for";

/// The four kinds the auditor raises when it distrusts a binding.
const SUSPECT_KINDS: [&str; 4] = [
    FindingKind::INSUFFICIENT_INDEPENDENCE,
    FindingKind::MOCKED_CONFIRMATION,
    FindingKind::SUSPECT_LINK,
    FindingKind::VACUOUS_EVIDENCE,
];

/// A finding kind's precedence bucket: 0 suspect, 1 stale, 2 everything else.
///
/// Everything else includes a kind the auditor adds later: `Finding::kind` is
/// deliberately an open string, and an unrecognised kind is not evidence that
/// the criterion is sound.
fn bucket(kind: &str) -> u8 {
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
                commit: binding.commit.as_str().to_owned(),
            });
    }
    let store_is_empty = input.bindings.bindings.is_empty();

    let mut grouped: BTreeMap<String, Vec<MatrixCriterion>> = BTreeMap::new();
    for criterion in input
        .coverage
        .iter()
        .flatten()
        .flat_map(|requirement| &requirement.criteria)
    {
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
                method: criterion.method.clone(),
                static_status: criterion.status.clone(),
                binders: criterion.binders.clone(),
                evidence_status,
                evidence_detail: detail,
            });
    }

    let reason = grouped.is_empty().then(|| EMPTY_REASON.to_owned());
    Ok(MatrixOutput {
        requirements: grouped
            .into_iter()
            .map(|(id, criteria)| MatrixRequirement { id, criteria })
            .collect(),
        reason,
    })
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
    if healthy {
        return EvidenceStatus::Bound;
    }
    // `detail.findings` is already sorted by bucket, so its first finding is
    // the one precedence selects.
    match detail.findings.first().map(|finding| bucket(&finding.kind)) {
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
        return out;
    }
    for requirement in &matrix.requirements {
        let _ = write!(
            out,
            "\n## {}\n\n| Requirement | Criterion | Method | Static Status | Evidence Status | Detail |\n|---|---|---|---|---|---|\n",
            cell(&requirement.id)
        );
        for criterion in &requirement.criteria {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} |",
                cell(&requirement.id),
                cell(&criterion.id),
                cell(criterion.method.as_deref().unwrap_or("—")),
                cell(&criterion.static_status),
                criterion.evidence_status.as_str(),
                cell(&detail_text(&criterion.evidence_detail)),
            );
        }
    }
    out
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
            .map(|binding| format!("bound by {}@{}", binding.suite, binding.commit)),
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
