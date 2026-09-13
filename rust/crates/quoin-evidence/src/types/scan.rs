// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Finding-shaped scan records.
//!
//! Not a [`RunRecord`](super::RunRecord): five of the eight formats quoin reads
//! emit findings, not run outcomes, and forced into a run entry a clean semgrep
//! run and a semgrep run that never executed are indistinguishable. The
//! record's existence is the proof the scan executed; `findings: []` on a
//! present record means ran and found nothing.

use serde::{Deserialize, Serialize};

use crate::ids::{Commit, SuiteId};

/// One scanner result, transcribed.
///
/// `severity` is the scanner's own word, never normalized (FR-034-CON-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Finding {
    /// The scanner's rule identity.
    pub rule_id: String,
    /// The scanner's own severity word.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    /// The scanner's message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// The path the finding is about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The line the finding is about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    /// Criterion ids the scanner named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_ids: Option<Vec<String>>,
}

impl Finding {
    /// A finding carrying only its rule identity.
    #[must_use]
    pub fn new(rule_id: impl Into<String>) -> Self {
        Self {
            rule_id: rule_id.into(),
            severity: None,
            message: None,
            path: None,
            line: None,
            trace_ids: None,
        }
    }
}

/// One finding-shaped scan of ONE suite at ONE commit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FindingRecord {
    /// Always [`STORE_SCHEMA_VERSION`](super::STORE_SCHEMA_VERSION).
    pub schema_version: u32,
    /// The suite this scan covered.
    pub suite: SuiteId,
    /// Full commit sha the scan was performed at.
    pub commit: Commit,
    /// Tool and version, as the adapter reported them.
    pub tool: String,
    /// The declared `test_type` this scan produced, when the caller names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_kind: Option<String>,
    /// ISO-8601, supplied by the caller — never read from the clock here.
    pub timestamp: String,
    /// What the scan covered, as the tool reported it.
    ///
    /// Load-bearing for vacuity: a scan that ran with no rules enabled also
    /// reports zero findings and cannot be told apart by the findings alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruleset: Option<String>,
    /// Number of rules the scan actually evaluated, when the tool reports it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules_evaluated: Option<u64>,
    /// One entry per finding the scanner reported.
    pub findings: Vec<Finding>,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::Finding;

    #[test]
    fn a_finding_carrying_only_its_rule_writes_only_that_key() {
        assert_eq!(
            serde_json::to_string(&Finding::new("no-eval")).unwrap(),
            r#"{"ruleId":"no-eval"}"#
        );
    }
}
