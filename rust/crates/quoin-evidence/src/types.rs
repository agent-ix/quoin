// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The record vocabulary the evidence store reads and writes.
//!
//! Ported from `src/evidence/types.ts`. The on-disk layout is a frozen
//! compatibility surface (FR-100-AC-2, NFR-025): field names are the
//! TypeScript's `camelCase` and an optional field is **omitted** rather than
//! written as `null`, which is what `...(x === undefined ? {} : {x})` did.
//!
//! Every optional here is optional in the retained source. Where quoin reads a
//! field unconditionally it is required, per the doctrine
//! `quoin_quire_types` states.

use serde::{Deserialize, Serialize};

/// The store schema version written into every record envelope.
///
/// Frozen: FR-100-AC-2 states that reading and re-serializing every store in
/// the ecosystem returns byte-identical records **and** that this value is
/// unchanged. `quoin_store::STORE_SCHEMA_VERSION` is the same number for the
/// change-assurance side; they are restated rather than shared because the two
/// stores version independently and a single constant would couple them.
pub const STORE_SCHEMA_VERSION: u32 = 1;

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
pub struct RunEntry {
    /// The producer's own identity for the result.
    pub symbol: String,
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
    /// Producer-supplied configuration, carried opaquely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

impl RunEntry {
    /// A minimal entry: the two fields every adapter fills.
    #[must_use]
    pub fn new(symbol: impl Into<String>, outcome: Outcome) -> Self {
        Self {
            symbol: symbol.into(),
            outcome,
            score: None,
            metric: None,
            trace_ids: None,
            config: None,
        }
    }
}

/// One scanner result, transcribed.
///
/// `severity` is the scanner's own word, never normalized (FR-034-CON-2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{Finding, Outcome, RunEntry};

    #[test]
    fn an_absent_optional_is_omitted_and_never_written_as_null() {
        // `...(x === undefined ? {} : {x})` in the retained source. A `null`
        // here would change every record byte the store already holds.
        let entry = RunEntry::new("tests::tc001", Outcome::Pass);
        assert_eq!(
            serde_json::to_string(&entry).unwrap(),
            r#"{"symbol":"tests::tc001","outcome":"pass"}"#
        );
        assert_eq!(
            serde_json::to_string(&Finding::new("no-eval")).unwrap(),
            r#"{"ruleId":"no-eval"}"#
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
