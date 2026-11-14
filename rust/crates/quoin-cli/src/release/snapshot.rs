// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The release snapshot document (FR-167): its closed types, and the sections
//! built from the evidence matrix and the anomaly documents.
//!
//! Every object admits only the members FR-167 names (`deny_unknown_fields`),
//! so reading a snapshot back through these types is the check that it is
//! closed. No type here holds a digest, hash, score or verdict.

use std::path::Path;

use quoin_anomaly::{CarriedRead, read_carried};
use quoin_assurance::matrix::{EvidenceStatus, MatrixOutput};
use serde::{Deserialize, Serialize};

use super::history::{ChangeRecord, PreviousRelease};
use super::{Code, Refusal, Stop};
use crate::evidence::audit::AuditOptions;
use crate::matrix::{Built, build};
use crate::soup::SoupSection;

/// The protocol of the snapshot.
pub(super) const PROTOCOL: &str = "quoin.release-snapshot/v1";

/// The release evidence snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Snapshot {
    pub(super) protocol: String,
    pub(super) as_of: String,
    pub(super) baseline_commit: String,
    pub(super) previous_release: Option<PreviousRelease>,
    pub(super) coverage: Vec<CoverageRequirement>,
    pub(super) evidence_freshness: Freshness,
    pub(super) open_findings: Vec<OpenFinding>,
    pub(super) soup: SoupSection,
    pub(super) anomalies: Vec<Anomaly>,
    pub(super) change_records: Vec<ChangeRecord>,
}

/// One requirement's criteria.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CoverageRequirement {
    pub(super) id: String,
    pub(super) criteria: Vec<CoverageCriterion>,
}

/// One criterion's row: the static axis beside the evidence axis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CoverageCriterion {
    pub(super) id: String,
    pub(super) static_status: String,
    pub(super) evidence_status: EvidenceStatus,
}

/// The count of criteria at each of the five evidence statuses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Freshness {
    pub(super) bound: usize,
    pub(super) stale: usize,
    pub(super) suspect: usize,
    pub(super) undischarged: usize,
    #[serde(rename = "no run evidence")]
    pub(super) no_run_evidence: usize,
}

impl Freshness {
    fn count(&mut self, status: EvidenceStatus) {
        let slot = match status {
            EvidenceStatus::Bound => &mut self.bound,
            EvidenceStatus::Stale => &mut self.stale,
            EvidenceStatus::Suspect => &mut self.suspect,
            EvidenceStatus::Undischarged => &mut self.undischarged,
            EvidenceStatus::NoRunEvidence => &mut self.no_run_evidence,
        };
        *slot += 1;
    }
}

/// One auditor finding of a criterion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OpenFinding {
    pub(super) obligation: String,
    pub(super) kind: String,
    pub(super) summary: String,
    pub(super) path: Option<String>,
    pub(super) line: Option<u64>,
    pub(super) symbol: Option<String>,
}

impl OpenFinding {
    /// FR-167's order: `obligation`, `kind`, `path`, `line`, `symbol`, then
    /// `summary`. Strings compare bytewise, `line` numerically, and an absent
    /// value sorts before a present one.
    fn order_key(&self) -> (&str, &str, Option<&str>, Option<u64>, Option<&str>, &str) {
        (
            &self.obligation,
            &self.kind,
            self.path.as_deref(),
            self.line,
            self.symbol.as_deref(),
            &self.summary,
        )
    }
}

/// One `{ author, rationale }` entry, as authored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RationaleEntry {
    pub(super) author: String,
    pub(super) rationale: String,
}

/// One carried anomaly, with the values its record states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Anomaly {
    pub(super) id: String,
    pub(super) title: Option<String>,
    pub(super) status: String,
    pub(super) resolution: Option<String>,
    pub(super) severity: Option<String>,
    pub(super) safety_impact: String,
    pub(super) unassessed: bool,
    pub(super) release_rationale: Vec<RationaleEntry>,
}

/// The sections that come from the evidence matrix.
pub(super) struct MatrixSections {
    pub(super) coverage: Vec<CoverageRequirement>,
    pub(super) freshness: Freshness,
    pub(super) findings: Vec<OpenFinding>,
}

/// Build the coverage, freshness and open findings through the code path
/// `quoin matrix` uses (FR-115).
pub(super) fn matrix_sections(options: &AuditOptions, repo: &Path) -> Result<MatrixSections, Stop> {
    let repo_text = repo
        .to_str()
        .ok_or_else(|| Stop::Failed("--repo is not valid UTF-8".to_owned()))?;
    let response = match build(options, repo_text, &[])? {
        Built::Matrix(built) if built.response.outcome.carries_payload() => built.response,
        Built::Matrix(built) => return Err(declined("--repo", &built.response)),
        Built::Declined(response) => return Err(declined("--repo", &response)),
    };
    let matrix: MatrixOutput = serde_json::from_value(response.payload)
        .map_err(|error| Stop::Failed(format!("matrix.build returned {error}")))?;
    Ok(sections(matrix))
}

fn declined(input: &str, response: &quoin_core::protocol::Response) -> Stop {
    let (code, message) = response
        .diagnostics
        .first()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.message.as_str()))
        .unwrap_or_default();
    Refusal::invalid(input, code, message).into()
}

pub(super) fn sections(matrix: MatrixOutput) -> MatrixSections {
    let mut freshness = Freshness::default();
    let mut findings = Vec::new();
    let coverage = matrix
        .requirements
        .into_iter()
        .map(|requirement| CoverageRequirement {
            id: requirement.id,
            criteria: requirement
                .criteria
                .into_iter()
                .map(|criterion| {
                    freshness.count(criterion.evidence_status);
                    for finding in criterion.evidence_detail.findings {
                        findings.push(OpenFinding {
                            obligation: criterion.id.clone(),
                            kind: finding.kind,
                            summary: finding.summary,
                            path: finding.path,
                            line: finding.line,
                            symbol: finding.symbol,
                        });
                    }
                    CoverageCriterion {
                        id: criterion.id,
                        static_status: criterion.static_status,
                        evidence_status: criterion.evidence_status,
                    }
                })
                .collect(),
        })
        .collect();
    findings.sort_by(|a, b| a.order_key().cmp(&b.order_key()));
    MatrixSections {
        coverage,
        freshness,
        findings,
    }
}

/// Read the carried anomalies of `repo`: selection and authored values only.
///
/// A document with a mapping error is a refusal, because the snapshot has no
/// member for an unreadable anomaly and one must not vanish from it.
pub(super) fn anomalies(repo: &Path) -> Result<Vec<Anomaly>, Stop> {
    let read = read_carried(repo).map_err(|error| Refusal::unreadable("--repo", error))?;
    carried(repo, read)
}

fn carried(repo: &Path, read: CarriedRead) -> Result<Vec<Anomaly>, Stop> {
    if let Some(unreadable) = read.unreadable.first() {
        let path = unreadable
            .path
            .strip_prefix(repo)
            .unwrap_or(&unreadable.path)
            .display()
            .to_string();
        let code = unreadable.code.as_str();
        return Err(Refusal::new(
            Code::AnomalyUnreadable,
            format!("the Anomaly document {path} has a mapping error ({code}) and so no record"),
            &[("path", &path), ("code", code)],
        )
        .into());
    }
    Ok(read
        .carried
        .into_iter()
        .map(|anomaly| {
            let unassessed = anomaly.unassessed();
            let record = anomaly.record;
            Anomaly {
                id: record.id,
                title: record.title,
                status: record.status_word,
                resolution: record.resolution,
                severity: record.severity,
                safety_impact: record.safety_impact,
                unassessed,
                release_rationale: record
                    .release_rationale
                    .unwrap_or_default()
                    .into_iter()
                    .map(|entry| RationaleEntry {
                        author: entry.author,
                        rationale: entry.rationale,
                    })
                    .collect(),
            }
        })
        .collect())
}
