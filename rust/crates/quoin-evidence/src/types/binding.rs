// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The binding graph and the ratchet baseline.

use serde::{Deserialize, Serialize};

use crate::ids::{Commit, ObligationId, StatementHash, SuiteId, SymbolId};

/// Separation facts carried by one evidence relationship.
///
/// Every field is optional and the absence of one stays visible: a policy that
/// asks for a dimension nothing records reports the suites that are missing it,
/// rather than inferring a value (FR-094).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceLineage {
    /// Who produced the evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    /// The toolchain the implementation under test was built with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation_toolchain: Option<String>,
    /// The verification technique used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technique: Option<String>,
    /// Where the inputs came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_source: Option<String>,
    /// The review path the result travelled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_path: Option<String>,
}

impl EvidenceLineage {
    /// Whether the lineage names nothing at all.
    ///
    /// The retained zod schema refuses a lineage with no key set, on the
    /// grounds that an empty object is a claim to have recorded separation
    /// facts while recording none.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.actor.is_none()
            && self.implementation_toolchain.is_none()
            && self.technique.is_none()
            && self.data_source.is_none()
            && self.review_path.is_none()
    }
}

/// Someone re-affirming a binding after the statement it was made against
/// changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Affirmation {
    /// Who affirmed.
    pub who: String,
    /// The commit they affirmed at.
    pub commit: Commit,
    /// Why, when they said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One obligation bound to the evidence that discharges it.
///
/// [`Binding::statement_hash_at_binding`] is the entire suspect mechanism:
/// suspect detection is `current != statement_hash_at_binding` against the
/// obligation quire re-derives today. quire computes the hash; quoin only ever
/// compares.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    /// The obligation id the matrix keys on.
    pub obligation: ObligationId,
    /// The statement hash as it stood when the binding was first made.
    pub statement_hash_at_binding: StatementHash,
    /// The suite that discharged it.
    pub suite: SuiteId,
    /// The commit of the run that first discharged it.
    pub commit: Commit,
    /// Symbols within that suite carrying the obligation's trace id.
    pub symbols: Vec<SymbolId>,
    /// Separation facts, for profile-selected independence checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<EvidenceLineage>,
    /// Re-affirmations recorded after a statement changed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affirmations: Option<Vec<Affirmation>>,
}

/// The unified binding graph.
///
/// Unified rather than per-suite because the graph IS cross-suite: one
/// obligation can be discharged by a unit test and a benchmark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingsFile {
    /// Always [`STORE_SCHEMA_VERSION`](super::STORE_SCHEMA_VERSION).
    pub schema_version: u32,
    /// Sorted by obligation, then by suite.
    pub bindings: Vec<Binding>,
}

/// The accepted violation set a ratchet compares against.
///
/// [`BaselineFile::accepted`] holds `<kind>:<obligation>` keys for *every*
/// finding kind. The original shape carried two named buckets, so five other
/// kinds could never appear in a baseline and `--ratchet` reported the whole
/// existing backlog for them — the outcome the mode exists to prevent
/// (agent-ix/quoin#105).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineFile {
    /// Always [`STORE_SCHEMA_VERSION`](super::STORE_SCHEMA_VERSION).
    pub schema_version: u32,
    /// The commit the baseline was accepted at.
    pub commit: Commit,
    /// Accepted findings as `<kind>:<obligation>`, sorted.
    pub accepted: Vec<String>,
}
