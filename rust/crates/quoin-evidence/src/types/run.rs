// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Run entries, findings and the two record envelopes built from them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{Commit, SuiteId, SymbolId};

/// The metric name `cargo-mutants` scores are recorded under.
///
/// Named where it is recorded (agent-ix/quoin#138): a bare `score` with no
/// metric leaves the auditor guessing from the tool string what the number
/// means.
pub const MUTATION_SCORE_METRIC: &str = "mutation-score";

/// What a producer said about one symbol.
///
/// A closed enum here and not a `String`, unlike
/// [`quoin_finding_types::Finding::kind`]: the retained TypeScript's union is
/// enforced at every construction site inside `src/evidence/`, every adapter
/// picks from these four, and the store's readers branch on all four.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Outcome {
    /// The producer reported success.
    Pass,
    /// The producer reported a failure.
    Fail,
    /// The producer chose not to run this symbol.
    Skip,
    /// The producer could not run this symbol.
    Error,
}

impl Outcome {
    /// The spelling written to disk.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Skip => "skip",
            Self::Error => "error",
        }
    }
}

/// One producer result, transcribed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunEntry {
    /// The producer's own identity for the result.
    pub symbol: SymbolId,
    /// What the producer said.
    pub outcome: Outcome,
    /// A native numeric result, where the format has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// What [`RunEntry::score`] measures. See [`MUTATION_SCORE_METRIC`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
    /// Obligation or criterion ids the producer named for this result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_ids: Option<Vec<String>>,
    /// The configuration dimension values this entry was executed under.
    ///
    /// `Record<string, string>` in the retained source, and a `BTreeMap` here
    /// rather than a `Value`: the auditor reads the dimension names to say
    /// which t-way combinations a run reached, and an opaque `Value` would put
    /// that cast at every read site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<BTreeMap<String, String>>,
}

impl RunEntry {
    /// A minimal entry: the two fields every adapter fills.
    #[must_use]
    pub fn new(symbol: impl Into<String>, outcome: Outcome) -> Self {
        Self {
            symbol: SymbolId::new(symbol),
            outcome,
            score: None,
            metric: None,
            trace_ids: None,
            config: None,
        }
    }
}

/// One run of ONE suite at ONE commit.
///
/// The suite is the atomic unit of evidence: aggregation is a view, and a
/// partial run must never be able to masquerade as a full one. Re-runs at the
/// same commit are last-write-wins, latest only — the file name carries the
/// short commit and nothing distinguishing one attempt from the next.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunRecord {
    /// Always [`STORE_SCHEMA_VERSION`](super::STORE_SCHEMA_VERSION).
    pub schema_version: u32,
    /// The suite this run covered.
    pub suite: SuiteId,
    /// Full commit sha the run was performed at.
    pub commit: Commit,
    /// Tool and version, as the adapter reported them.
    pub tool: String,
    /// The declared `test_type` this run produced, when the caller names one.
    ///
    /// Its absence is not an invitation to guess: method conformance once
    /// inferred "this was a test run" from a non-empty entry list, which is
    /// true of a transcribed inspection too (agent-ix/quoin#105).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_kind: Option<String>,
    /// ISO-8601, supplied by the caller — never read from the clock here.
    pub timestamp: String,
    /// One entry per symbol the producer reported.
    pub entries: Vec<RunEntry>,
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{Outcome, RunEntry};

    #[test]
    fn an_absent_optional_is_omitted_and_never_written_as_null() {
        // `...(x === undefined ? {} : {x})` in the retained source. A `null`
        // here would change every record byte the store already holds.
        let entry = RunEntry::new("tests::tc001", Outcome::Pass);
        assert_eq!(
            serde_json::to_string(&entry).unwrap(),
            r#"{"symbol":"tests::tc001","outcome":"pass"}"#
        );
    }

    #[test]
    fn outcomes_spell_themselves_the_way_the_store_holds_them() {
        for (outcome, text) in [
            (Outcome::Pass, "pass"),
            (Outcome::Fail, "fail"),
            (Outcome::Skip, "skip"),
            (Outcome::Error, "error"),
        ] {
            assert_eq!(outcome.as_str(), text);
            assert_eq!(
                serde_json::to_string(&outcome).unwrap(),
                format!("\"{text}\"")
            );
        }
    }
}
