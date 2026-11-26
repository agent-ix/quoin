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
use ix_trace_rs::implements;
use quoin_anomaly::{
    AnomalyRead, CarriedAnomaly, GateMode, Record as AnomalyRecord, gate_carried, is_carried,
    read_anomalies,
};
use quoin_assurance::matrix::MatrixOutput;
#[cfg(test)]
pub(super) use quoin_dossier::release_snapshot::ReleaseSnapshot as Snapshot;
use quoin_dossier::release_snapshot::{
    CoverageCriterion, CoverageRequirement, Freshness, OpenFinding, RationaleEntry,
    SnapshotAnomaly as Anomaly, SnapshotVulnerability,
};
use quoin_quire::records::Contract;
use quoin_quire::{ModuleRoot, ModuleSelection};
use quoin_risk_advice::{AdviceError, ErrorCode, summarize};
use serde_json::Value;

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

/// Authored vulnerability records whose lifecycle status is still open
/// (FR-032, FR-167). No advisory, VEX or anomaly data is consulted.
#[implements("FR-167-AC-20")]
pub(super) fn vulnerabilities(
    repo: &Path,
    modules: &[String],
) -> Result<Vec<SnapshotVulnerability>, Stop> {
    let mut roots = Vec::new();
    for dir in modules {
        roots.push(ModuleRoot::open(dir).map_err(|error| module_invalid(dir, &error.to_string()))?);
    }
    let selection = if roots.is_empty() {
        ModuleSelection::ScopeOrAmbient
    } else {
        ModuleSelection::Closed(roots.clone())
    };
    let corpus = quoin_quire::records::read(repo, &selection, &["vulnerability"])
        .map_err(|error| vulnerability_records_refusal(modules, &roots, &error))?;
    if let Some(unreadable) = corpus.unreadable.first() {
        let root = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
        let path = unreadable
            .path
            .strip_prefix(&root)
            .unwrap_or(&unreadable.path)
            .display()
            .to_string();
        return Err(Refusal::new(
            Code::InputUnreadable,
            format!("--repo holds {path}, which cannot be read"),
            &[("input", "--repo"), ("path", &path)],
        )
        .into());
    }
    let root = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    if let Some(path) = corpus
        .malformed
        .iter()
        .find(|path| is_vulnerability_record_path(&root, path))
    {
        let shown = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .display()
            .to_string();
        return Err(Refusal::invalid(
            "--repo",
            "vulnerability-record-malformed",
            &format!("{shown} has malformed frontmatter or identity"),
        )
        .into());
    }

    render_vulnerabilities(&corpus.documents)
}

/// Malformed frontmatter is a vulnerability-record failure only in the
/// authored vulnerability-record directory. Other malformed repository files
/// belong to the snapshot's risk-summary refusal, which can name their path.
#[implements("FR-167-AC-24")]
fn is_vulnerability_record_path(repo: &Path, path: &Path) -> bool {
    path.strip_prefix(repo)
        .is_ok_and(|relative| relative.starts_with(Path::new("spec/vulnerabilities")))
}

/// Preserve the module root when reading vulnerability records fails during
/// module loading, just as the risk-summary refusal does.
#[implements("FR-167-AC-24")]
fn vulnerability_records_refusal(
    modules: &[String],
    roots: &[ModuleRoot],
    error: &quoin_quire::Error,
) -> Stop {
    let failed = match error {
        quoin_quire::Error::ModuleLoad { path, .. } => roots
            .iter()
            .position(|root| path.starts_with(root.as_path())),
        quoin_quire::Error::ModuleSetEmpty { .. } if roots.len() == 1 => Some(0),
        _ => None,
    };
    if let Some(index) = failed.and_then(|index| modules.get(index)) {
        return module_invalid(index, &error.to_string()).into();
    }
    if !modules.is_empty()
        && matches!(
            error,
            quoin_quire::Error::ModuleLoad { .. } | quoin_quire::Error::ModuleSetEmpty { .. }
        )
    {
        return module_invalid(&modules.join(", "), &error.to_string()).into();
    }
    Refusal::invalid("--repo", "vulnerability-records", &error.to_string()).into()
}

/// Select and copy authored lifecycle facts from the parsed vulnerability
/// corpus. Kept separate from loading so status filtering and field mapping
/// are testable without a module installation.
fn render_vulnerabilities(
    documents: &[quoin_quire::records::Document],
) -> Result<Vec<SnapshotVulnerability>, Stop> {
    let mut selected = Vec::new();
    for document in documents
        .iter()
        .filter(|document| document.kind == "vulnerability")
    {
        if let Contract::Violated(reason) = &document.contract {
            return Err(Refusal::invalid(
                "--repo",
                "vulnerability-record-invalid",
                &format!("{}: {reason}", document.path.display()),
            )
            .into());
        }
        let status = optional_text(document, "status")?;
        let Some(status) = status else {
            continue;
        };
        match status.as_str() {
            "open" | "in_remediation" => {}
            "remediated" | "closed" => continue,
            other => {
                return Err(Refusal::invalid(
                    "--repo",
                    "vulnerability-record-invalid",
                    &format!(
                        "{} has unsupported status {other:?}",
                        document.path.display()
                    ),
                )
                .into());
            }
        }
        let optional = |key| optional_text(document, key);
        let mut affects = Vec::new();
        let mut references = Vec::new();
        for (target, relation) in &document.edges {
            match relation.as_str() {
                "affects" => affects.push(target.clone()),
                "references" => references.push(target.clone()),
                _ => {}
            }
        }
        selected.push(SnapshotVulnerability {
            id: document.id.clone(),
            cve_id: optional("cve_id")?,
            severity: optional("severity")?,
            affects,
            references,
            status,
            disposition: optional("disposition")?,
            disposition_by: optional("disposition_by")?,
            discovered_at: optional("discovered_at")?,
            disclosed_at: optional("disclosed_at")?,
            fixed_at: optional("fixed_at")?,
        });
    }
    selected.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(selected)
}

/// An optional text field in a contract-checked vulnerability record.
fn optional_text(
    document: &quoin_quire::records::Document,
    field: &str,
) -> Result<Option<String>, Stop> {
    match document.frontmatter.get(field) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(Refusal::invalid(
            "--repo",
            "vulnerability-record-invalid",
            &format!("{} has a non-text {field}", document.path.display()),
        )
        .into()),
    }
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
    let read = read_anomalies(repo).map_err(|error| Refusal::unreadable("--repo", error))?;
    let carried: Vec<CarriedAnomaly> = read
        .anomalies
        .iter()
        .filter(|anomaly| is_carried(&anomaly.record))
        .map(|anomaly| CarriedAnomaly {
            path: anomaly.path.clone(),
            record: anomaly.record.clone(),
        })
        .collect();
    let findings = gate_findings(repo, &carried);
    Ok((snapshot_anomalies(repo, read)?, findings))
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

fn snapshot_anomalies(repo: &Path, read: AnomalyRead) -> Result<Vec<Anomaly>, Stop> {
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
        .anomalies
        .into_iter()
        .filter(|anomaly| is_carried(&anomaly.record) || is_closed_fixed(&anomaly.record))
        .map(|anomaly| {
            let unassessed = anomaly.record.safety_impact == "unassessed";
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

fn is_closed_fixed(anomaly: &AnomalyRecord) -> bool {
    anomaly.status_word == "closed" && anomaly.resolution.as_deref() == Some("fixed")
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic is the failure report"
)]
mod vulnerability_tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use ix_trace_rs::trace;
    use quoin_quire::records::{Contract, Document};
    use serde_json::{Map, json};

    use super::render_vulnerabilities;

    fn document(id: &str, status: &str) -> Document {
        let mut frontmatter = Map::new();
        frontmatter.insert("status".to_owned(), json!(status));
        frontmatter.insert("cve_id".to_owned(), json!(format!("CVE-{id}")));
        frontmatter.insert("severity".to_owned(), json!("high"));
        frontmatter.insert("disposition".to_owned(), json!("maintained component"));
        frontmatter.insert("disposition_by".to_owned(), json!("Ada Example"));
        frontmatter.insert("discovered_at".to_owned(), json!("2026-02-03"));
        frontmatter.insert("disclosed_at".to_owned(), json!("2026-02-04"));
        frontmatter.insert("fixed_at".to_owned(), json!("2026-02-05"));
        Document {
            path: PathBuf::from(format!("spec/vulnerabilities/{id}.md")),
            id: id.to_owned(),
            kind: "vulnerability".to_owned(),
            frontmatter,
            edges: vec![
                ("AN-001".to_owned(), "references".to_owned()),
                ("SOUP-001".to_owned(), "affects".to_owned()),
            ],
            tables: BTreeMap::new(),
            sections: BTreeMap::new(),
            raw: String::new(),
            contract: Contract::Satisfied,
        }
    }

    #[trace("FR-167-AC-20", "FR-167-AC-21")]
    #[test]
    fn snapshot_copies_only_open_lifecycles_and_sorts_by_id() {
        let rendered = render_vulnerabilities(&[
            document("VULN-4", "closed"),
            document("VULN-3", "remediated"),
            document("VULN-2", "in_remediation"),
            document("VULN-1", "open"),
        ])
        .unwrap_or_else(|_| panic!("valid vulnerability fixtures should render"));

        let ids: Vec<&str> = rendered.iter().map(|item| item.id.as_str()).collect();
        assert_eq!(ids, ["VULN-1", "VULN-2"]);
        let first = &rendered[0];
        assert_eq!(first.cve_id.as_deref(), Some("CVE-VULN-1"));
        assert_eq!(first.severity.as_deref(), Some("high"));
        assert_eq!(first.affects, ["SOUP-001"]);
        assert_eq!(first.references, ["AN-001"]);
        assert_eq!(first.status, "open");
        assert_eq!(first.disposition.as_deref(), Some("maintained component"));
        assert_eq!(first.disposition_by.as_deref(), Some("Ada Example"));
        assert_eq!(first.discovered_at.as_deref(), Some("2026-02-03"));
        assert_eq!(first.disclosed_at.as_deref(), Some("2026-02-04"));
        assert_eq!(first.fixed_at.as_deref(), Some("2026-02-05"));
    }

    #[trace("FR-167-AC-20")]
    #[test]
    fn no_open_lifecycle_records_produces_an_empty_section() {
        assert!(
            render_vulnerabilities(&[
                document("VULN-1", "remediated"),
                document("VULN-2", "closed"),
            ])
            .unwrap_or_else(|_| panic!("terminal vulnerability fixtures should render"))
            .is_empty()
        );
    }
}
