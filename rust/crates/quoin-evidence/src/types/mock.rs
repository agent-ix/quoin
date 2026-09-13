// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Source-level mock-substitution inspection records.

use serde::{Deserialize, Serialize};

use crate::ids::{Commit, SuiteId, SymbolId};

/// One test symbol observed substituting a stand-in for real behaviour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockInjection {
    /// The suite whose source was inspected.
    pub suite: SuiteId,
    /// The test symbol containing the substitution.
    pub symbol: SymbolId,
    /// Identifiers substituted for real behaviour, sorted and deduplicated.
    pub injects: Vec<String>,
    /// Repo-relative source location, when the inspection can name one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The line the call was seen on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
}

/// One completed source inspection for mock substitutions.
///
/// The record may contain no injections and its existence is still material:
/// an empty completed inspection means "looked and found none", no record at
/// all means "nobody looked" (agent-ix/quoin#204).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockInspectionRecord {
    /// Always [`STORE_SCHEMA_VERSION`](super::STORE_SCHEMA_VERSION).
    pub schema_version: u32,
    /// The suite whose source was inspected.
    pub suite: SuiteId,
    /// Full commit sha whose source was inspected.
    pub commit: Commit,
    /// The inspecting tool and its version.
    pub tool: String,
    /// ISO-8601, supplied by the caller.
    pub timestamp: String,
    /// One entry per substituting symbol, sorted.
    pub injections: Vec<MockInjection>,
}
