// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The release snapshot document (FR-167): its closed types, and the sections
//! built from the evidence matrix and the anomaly documents.
//!
//! Every object admits only the members FR-167 names (`deny_unknown_fields`),
//! so reading a snapshot back through these types is the check that it is
//! closed. No type here holds a digest, hash, score or verdict.

use std::path::{Path, PathBuf};

use super::{Code, Refusal, Stop};
use crate::evidence::audit::AuditOptions;
use crate::matrix::{Built, build};
use quoin_anomaly::{CarriedAnomaly, CarriedRead, GateMode, gate_carried, read_carried};
use quoin_assurance::matrix::MatrixOutput;
#[cfg(test)]
pub(super) use quoin_dossier::release_snapshot::ReleaseSnapshot as Snapshot;
use quoin_dossier::release_snapshot::{
    CoverageCriterion, CoverageRequirement, Freshness, OpenFinding, RationaleEntry,
    SnapshotAnomaly as Anomaly,
};
use quoin_quire::{ModuleRoot, ModuleSelection};
use quoin_risk_advice::{AdviceError, ErrorCode, summarize};

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
    let response = match build(options, repo_text, &[], None)? {
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
                            cause: finding.cause,
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

/// The risk-completeness findings of `repo`, built by the code `quoin
/// risk-summary` uses, one item per distinct finding (FR-170).
///
/// A failure `quoin risk-summary` would refuse with is a refusal here, and so
/// is a file below `repo` that cannot be read or whose frontmatter is not valid
/// YAML, because a record in it would go unreported.
pub(super) fn risk_findings(repo: &Path, modules: &[String]) -> Result<Vec<OpenFinding>, Stop> {
    let mut roots = Vec::new();
    for dir in modules {
        roots.push(ModuleRoot::open(dir).map_err(|error| module_invalid(dir, &error.to_string()))?);
    }
    let resolved: Vec<PathBuf> = roots
        .iter()
        .map(|root| root.as_path().to_path_buf())
        .collect();
    let selection = if roots.is_empty() {
        ModuleSelection::Ambient
    } else {
        ModuleSelection::Closed(roots)
    };
    let summary =
        summarize(repo, &selection).map_err(|error| advice_refusal(modules, &resolved, &error))?;
    if let Some(path) = summary.unusable.first() {
        // The reader works on the canonical scope, so its paths are made
        // relative to that, never to the spelling of `--repo`.
        let root = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
        let shown = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .display()
            .to_string();
        return Err(Refusal::new(
            Code::InputUnreadable,
            format!(
                "--repo holds {shown}, which cannot be read or whose frontmatter is not a YAML mapping"
            ),
            &[("input", "--repo"), ("path", &shown)],
        )
        .into());
    }
    Ok(summary
        .distinct_findings()
        .into_iter()
        .map(|found| OpenFinding {
            obligation: found.names.first().cloned().unwrap_or_default(),
            kind: found.finding.as_str().to_owned(),
            summary: found.names.join(", "),
            path: None,
            line: None,
            symbol: None,
            cause: None,
        })
        .collect())
}

fn module_invalid(dir: &str, message: &str) -> Refusal {
    Refusal::new(
        Code::InputInvalid,
        format!("--module {dir} is invalid: {message}"),
        &[("input", "--module"), ("module", dir)],
    )
}

/// The refusal for a failure `quoin risk-summary` reports with exit status 2.
///
/// A module root that failed to load names its own `--module DIR`. A set of
/// roots that declares no safety record type names every `--module DIR`, and
/// the installed modules when none was given. Anything else names `--repo` and
/// carries the record's own message.
fn advice_refusal(modules: &[String], resolved: &[PathBuf], error: &AdviceError) -> Stop {
    match error.code {
        ErrorCode::Modules | ErrorCode::TypeUndeclared => {
            let failed = error
                .module
                .as_ref()
                .and_then(|path| resolved.iter().position(|root| root == path))
                .and_then(|index| modules.get(index));
            let named = match (failed, modules) {
                (Some(dir), _) => dir.clone(),
                (None, []) => "installed modules".to_owned(),
                (None, all) => all.join(", "),
            };
            module_invalid(&named, &error.message).into()
        }
        ErrorCode::Render => Stop::Failed(error.message.clone()),
        ErrorCode::Scope | ErrorCode::Record | ErrorCode::Unparseable | ErrorCode::Git => {
            Refusal::invalid("--repo", error.code.as_str(), &error.message).into()
        }
    }
}

/// The carried anomalies of `repo`, with the anomaly-gate findings (FR-164)
/// over them: selection and authored values only, no git.
///
/// A document with a mapping error is a refusal, because the snapshot has no
/// member for an unreadable anomaly and one must not vanish from it.
pub(super) fn anomalies(repo: &Path) -> Result<(Vec<Anomaly>, Vec<OpenFinding>), Stop> {
    let read = read_carried(repo).map_err(|error| Refusal::unreadable("--repo", error))?;
    let findings = gate_findings(repo, &read.carried);
    Ok((carried(repo, read)?, findings))
}

/// One item for each failure of either gate mode.
fn gate_findings(repo: &Path, carried: &[CarriedAnomaly]) -> Vec<OpenFinding> {
    gate_carried(repo, carried, &GateMode::ALL)
        .failures
        .into_iter()
        .map(|failure| OpenFinding {
            obligation: failure.anomaly,
            kind: format!("anomaly-{}", failure.mode.as_str()),
            summary: failure.mode.as_str().to_owned(),
            path: Some(failure.path),
            line: None,
            symbol: None,
            cause: None,
        })
        .collect()
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
