// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Closed read-only model and parser for `quoin.release-snapshot/v1`.
//!
//! The release command and dossier share this model so reading a committed
//! snapshot never introduces a second interpretation of its schema.

use serde::{Deserialize, Serialize};

/// The only supported release snapshot protocol.
pub const RELEASE_SNAPSHOT_PROTOCOL: &str = "quoin.release-snapshot/v1";
/// The executable name that writes release snapshots.
pub const QUOIN_TOOL_NAME: &str = "quoin";

/// A release evidence snapshot as written by `quoin release snapshot`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseSnapshot {
    /// Protocol identifier.
    pub protocol: String,
    /// Quoin executable identity.
    pub tool_identity: ToolIdentity,
    /// State of the clean-tree precondition.
    pub clean_tree_precondition: CleanTreePrecondition,
    /// Evaluation instant supplied to the snapshot command.
    pub as_of: String,
    /// Baseline commit supplied to the snapshot command.
    pub baseline_commit: String,
    /// Previous release, if the source history has one.
    pub previous_release: Option<SnapshotPreviousRelease>,
    /// Requirement coverage in source order.
    pub coverage: Vec<CoverageRequirement>,
    /// Counts for the evidence freshness statuses.
    pub evidence_freshness: Freshness,
    /// Open findings in snapshot order.
    pub open_findings: Vec<OpenFinding>,
    /// SOUP state captured in the snapshot.
    pub soup: quoin_soup::SoupSection,
    /// Carried anomalies in snapshot order.
    pub anomalies: Vec<SnapshotAnomaly>,
    /// Change records in FR-168 source order.
    pub change_records: Vec<SnapshotChangeRecord>,
}

/// Identity of the Quoin executable that wrote a snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolIdentity {
    /// Executable name.
    pub name: String,
    /// Version printed by the executable's `--version` flag.
    pub version: String,
}

/// Whether Quoin checked the caller's clean-tree precondition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanTreePrecondition {
    /// Quoin does not inspect git state and could not verify this condition.
    NotVerified,
}

impl ReleaseSnapshot {
    /// Parse and validate a closed release snapshot.
    ///
    /// # Errors
    ///
    /// Returns a JSON error if the bytes are malformed, the record is not
    /// closed, its protocol identifier is unsupported, `tool_identity` does
    /// not name `quoin` with a non-empty version, or a `soup.not_assessed`
    /// item has an unknown `kind`.
    pub fn parse(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        let snapshot: Self = serde_json::from_slice(bytes)?;
        if snapshot.protocol != RELEASE_SNAPSHOT_PROTOCOL {
            return Err(<serde_json::Error as serde::de::Error>::custom(format!(
                "protocol must be {RELEASE_SNAPSHOT_PROTOCOL}"
            )));
        }
        if snapshot.tool_identity.name != QUOIN_TOOL_NAME
            || snapshot.tool_identity.version.trim().is_empty()
        {
            return Err(<serde_json::Error as serde::de::Error>::custom(format!(
                "tool_identity must name {QUOIN_TOOL_NAME} and carry a non-empty version"
            )));
        }
        if snapshot
            .soup
            .not_assessed
            .iter()
            .any(|item| !NOT_ASSESSED_KINDS.contains(&item.kind.as_str()))
        {
            return Err(<serde_json::Error as serde::de::Error>::custom(
                "soup.not_assessed kind must be one of unmatched_advisory, \
                 unmatched_policy_finding, scan_not_run, scan_vacuous",
            ));
        }
        Ok(snapshot)
    }
}

/// The `soup.not_assessed` kinds a snapshot may carry.
const NOT_ASSESSED_KINDS: [&str; 4] = [
    "unmatched_advisory",
    "unmatched_policy_finding",
    "scan_not_run",
    "scan_vacuous",
];

/// One requirement's criteria.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageRequirement {
    /// Requirement identifier.
    pub id: String,
    /// Its criteria.
    pub criteria: Vec<CoverageCriterion>,
}

/// One criterion's static and evidence status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageCriterion {
    /// Criterion identifier.
    pub id: String,
    /// Static status as captured.
    pub static_status: String,
    /// Evidence status as captured.
    pub evidence_status: quoin_assurance::matrix::EvidenceStatus,
}

/// Counts for the five evidence statuses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Freshness {
    /// Bound evidence count.
    pub bound: usize,
    /// Stale evidence count.
    pub stale: usize,
    /// Suspect evidence count.
    pub suspect: usize,
    /// Undischarged evidence count.
    pub undischarged: usize,
    /// Criteria with no run evidence.
    #[serde(rename = "no run evidence")]
    pub no_run_evidence: usize,
}

impl Freshness {
    /// Add one criterion to the appropriate status count.
    pub fn count(&mut self, status: quoin_assurance::matrix::EvidenceStatus) {
        let slot = match status {
            quoin_assurance::matrix::EvidenceStatus::Bound => &mut self.bound,
            quoin_assurance::matrix::EvidenceStatus::Stale => &mut self.stale,
            quoin_assurance::matrix::EvidenceStatus::Suspect => &mut self.suspect,
            quoin_assurance::matrix::EvidenceStatus::Undischarged => &mut self.undischarged,
            quoin_assurance::matrix::EvidenceStatus::NoRunEvidence => &mut self.no_run_evidence,
        };
        *slot += 1;
    }
}

/// One open auditor finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenFinding {
    /// Obligation identifier.
    pub obligation: String,
    /// Finding kind.
    pub kind: String,
    /// Finding summary.
    pub summary: String,
    /// Repository path, when present.
    pub path: Option<String>,
    /// Source line, when present.
    pub line: Option<u64>,
    /// Source symbol, when present.
    pub symbol: Option<String>,
    /// Typed stale-evidence cause, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<quoin_assurance::matrix::StaleCause>,
}

impl OpenFinding {
    /// The stable FR-167 finding order key.
    #[must_use]
    pub fn order_key(
        &self,
    ) -> (
        &str,
        &str,
        Option<&str>,
        Option<u64>,
        Option<&str>,
        u8,
        &str,
    ) {
        (
            &self.obligation,
            &self.kind,
            self.path.as_deref(),
            self.line,
            self.symbol.as_deref(),
            cause_order(self.cause),
            &self.summary,
        )
    }
}

fn cause_order(cause: Option<quoin_assurance::matrix::StaleCause>) -> u8 {
    match cause {
        None | Some(quoin_assurance::matrix::StaleCause::Invalid) => 0,
        Some(quoin_assurance::matrix::StaleCause::FailedRun) => 1,
        Some(quoin_assurance::matrix::StaleCause::MissingRun) => 2,
    }
}

/// An authored anomaly rationale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RationaleEntry {
    /// Rationale author.
    pub author: String,
    /// Rationale text.
    pub rationale: String,
}

/// One carried anomaly as recorded in the snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotAnomaly {
    /// Anomaly identifier.
    pub id: String,
    /// Authored title, when present.
    pub title: Option<String>,
    /// Authored status.
    pub status: String,
    /// Authored resolution, when present.
    pub resolution: Option<String>,
    /// Authored severity, when present.
    pub severity: Option<String>,
    /// Authored safety-impact assessment.
    pub safety_impact: String,
    /// Whether safety impact is unassessed.
    pub unassessed: bool,
    /// Release rationale entries in source order.
    pub release_rationale: Vec<RationaleEntry>,
}

/// Previous release as captured in snapshot history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotPreviousRelease {
    /// Previous release tag name.
    pub tag: String,
    /// Previous release commit value.
    pub commit: String,
}

/// One change-assurance record in the snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotChangeRecord {
    /// Record identifier.
    pub record_id: String,
    /// Record revision.
    pub revision: u64,
    /// Repository-relative record path.
    pub path: String,
    /// Commit value captured by the source history.
    pub commit: String,
}
