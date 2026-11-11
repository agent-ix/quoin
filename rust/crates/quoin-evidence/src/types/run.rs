// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Run entries, findings and the two record envelopes built from them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{SuiteId, SymbolId};

/// The metric name `cargo-mutants` scores are recorded under.
///
/// Named where it is recorded (agent-ix/quoin#138): a bare `score` with no
/// metric leaves the auditor guessing from the tool string what the number
/// means.
pub const MUTATION_SCORE_METRIC: &str = "mutation-score";

/// Most surviving mutants one entry keeps. A function with more survivors than
/// this has already failed any floor; the rest add bytes, not information.
pub const MAX_SURVIVORS_PER_ENTRY: usize = 25;

/// Longest description, in characters, a [`Survivor`] keeps.
///
/// cargo-mutants descriptions run `replace <function path> -> <type> with
/// <replacement> in <function>`; the replacement is the part that says what went
/// undetected, so an over-long description is cut in the middle and keeps both
/// ends.
pub const MAX_SURVIVOR_CHARS: usize = 200;

/// Characters kept from the head of an over-long description. The tail keeps the
/// rest of [`MAX_SURVIVOR_CHARS`] after the one-character ellipsis.
const SURVIVOR_HEAD_CHARS: usize = 80;

/// One mutant the tests did not kill: a demonstrated gap in the suite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Survivor {
    /// The line of the mutated code, when the tool reported one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    /// What was changed, in the tool's words (`replace f -> bool with true`).
    pub mutation: String,
}

impl Survivor {
    /// A survivor whose description holds no control character (a newline
    /// included) and is at most [`MAX_SURVIVOR_CHARS`] long.
    #[must_use]
    pub fn new(line: Option<u64>, mutation: &str) -> Self {
        let clean: Vec<char> = mutation
            .trim()
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        let mutation = if clean.len() > MAX_SURVIVOR_CHARS {
            let tail = MAX_SURVIVOR_CHARS - SURVIVOR_HEAD_CHARS - 1;
            let head: String = clean.iter().take(SURVIVOR_HEAD_CHARS).collect();
            let end: String = clean.iter().skip(clean.len() - tail).collect();
            format!("{head}…{end}")
        } else {
            clean.into_iter().collect()
        };
        Self { line, mutation }
    }
}

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
    /// The mutants of this symbol the tests did not kill, bounded by
    /// [`MAX_SURVIVORS_PER_ENTRY`] and [`MAX_SURVIVOR_CHARS`] by
    /// [`RunEntry::bound_survivors`], which every adapter result passes through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub survivors: Option<Vec<Survivor>>,
}

impl RunEntry {
    /// Enforce the survivor bounds on whatever a producer wrote: at most
    /// [`MAX_SURVIVORS_PER_ENTRY`] survivors, each through [`Survivor::new`]. An
    /// empty list becomes absent.
    pub fn bound_survivors(&mut self) {
        let Some(kept) = self.survivors.take() else {
            return;
        };
        let bounded: Vec<Survivor> = kept
            .iter()
            .take(MAX_SURVIVORS_PER_ENTRY)
            .map(|survivor| Survivor::new(survivor.line, &survivor.mutation))
            .collect();
        self.survivors = (!bounded.is_empty()).then_some(bounded);
    }

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
            survivors: None,
        }
    }
}

/// One run of ONE suite.
///
/// The suite is the atomic unit of evidence: aggregation is a view, and a
/// partial run must never be able to masquerade as a full one. Re-runs at the
/// same timestamp are last-write-wins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RunRecord {
    /// Always [`STORE_SCHEMA_VERSION`](super::STORE_SCHEMA_VERSION).
    pub schema_version: u32,
    /// The suite this run covered.
    pub suite: SuiteId,
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
    use super::{Outcome, RunEntry, RunRecord};

    #[test]
    fn run_reader_refuses_unmodeled_record_fields() {
        let text = r#"{"schemaVersion":1,"suite":"SUITE-001","tool":"runner 1","timestamp":"2026-08-17T00:00:00Z","entries":[],"retiredMember":true}"#;
        assert!(serde_json::from_str::<RunRecord>(text).is_err());
    }

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
