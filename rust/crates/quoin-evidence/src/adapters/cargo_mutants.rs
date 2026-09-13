// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `cargo-mutants` `outcomes.json` → one entry per mutated `FUNCTION`.
//!
//! The reason this format is a reference adapter: it is the only one of the
//! three original formats with a native numeric result, so it is the one that
//! proves the contract carries more than pass/fail (agent-ix/quoin#114). A
//! contract designed without exercising `score` would be designed blind.
//!
//! **What `symbol` means here is not what it means for `JUnit`.** cargo-mutants
//! exercises PRODUCTION functions, not test symbols, so an entry names
//! `src/path.rs::function` — the thing mutated. Binding that back to a
//! requirement needs a requirement→production-code relation, which does not
//! exist yet (agent-ix/quire-rs#171).

use std::collections::BTreeMap;

use serde::Deserialize;

use super::AdapterResult;
use crate::error::EvidenceError;
use crate::types::{MUTATION_SCORE_METRIC, Outcome, RunEntry};

#[derive(Deserialize)]
struct MutantScenario {
    file: Option<String>,
    function: Option<MutantFunction>,
}

#[derive(Deserialize)]
struct MutantFunction {
    function_name: Option<String>,
}

#[derive(Deserialize)]
struct Scenario {
    #[serde(rename = "Mutant")]
    mutant: Option<MutantScenario>,
}

#[derive(Deserialize)]
struct MutantOutcome {
    /// `"Baseline"` (a string) or `{ "Mutant": … }` (an object). A string is
    /// neither deserialized nor refused — the retained reader skips anything
    /// that is not an object, so this is `Option` and `None` means skip.
    scenario: Option<Scenario>,
    summary: Option<String>,
}

#[derive(Deserialize)]
struct Report {
    outcomes: Option<Vec<serde_json::Value>>,
}

fn symbol_of(mutant: &MutantScenario) -> Option<String> {
    let file = mutant.file.as_deref().unwrap_or("").trim();
    let function = mutant
        .function
        .as_ref()
        .and_then(|f| f.function_name.as_deref())
        .unwrap_or("")
        .trim();
    if file.is_empty() || function.is_empty() {
        return None;
    }
    Some(format!("{file}::{function}"))
}

/// Parse `mutants.out/outcomes.json`.
///
/// # Errors
///
/// Refuses input that is not JSON, input with no `outcomes` array, and a report
/// in which no viable mutant was run.
pub fn parse_cargo_mutants(raw: &str) -> Result<AdapterResult, EvidenceError> {
    let report: Report = serde_json::from_str(raw).map_err(|error| {
        EvidenceError::adapter("cargo-mutants", format!("input is not JSON: {error}"))
    })?;
    let outcomes = report.outcomes.ok_or_else(|| {
        EvidenceError::adapter(
            "cargo-mutants",
            "no `outcomes` array — expected the contents of mutants.out/outcomes.json",
        )
    })?;

    // caught / missed per function. `Unviable` mutants are counted in NEITHER:
    // they do not compile, so they exercised no test and belong in no
    // denominator. Including them would depress every score by an amount that
    // says nothing about the suite.
    let mut caught: BTreeMap<String, u64> = BTreeMap::new();
    let mut missed: BTreeMap<String, u64> = BTreeMap::new();
    for value in outcomes {
        let Ok(outcome) = serde_json::from_value::<MutantOutcome>(value) else {
            continue;
        };
        let Some(mutant) = outcome.scenario.and_then(|s| s.mutant) else {
            continue;
        };
        let Some(symbol) = symbol_of(&mutant) else {
            continue;
        };
        match outcome.summary.as_deref() {
            Some("CaughtMutant") => *caught.entry(symbol).or_default() += 1,
            // A `Timeout` is a mutant the suite did not kill. Counting it as
            // caught would report a gap as coverage.
            Some("MissedMutant" | "Timeout") => *missed.entry(symbol).or_default() += 1,
            _ => {}
        }
    }

    // `BTreeMap` keys are already in byte order, which is the sort the retained
    // `[...new Set(...)].sort()` produces for the ASCII paths cargo-mutants
    // emits.
    let mut symbols: Vec<&String> = caught.keys().chain(missed.keys()).collect();
    symbols.sort_unstable();
    symbols.dedup();
    if symbols.is_empty() {
        return Err(EvidenceError::adapter(
            "cargo-mutants",
            "no viable mutants in the report — nothing was measured",
        ));
    }

    let entries: Vec<RunEntry> = symbols
        .into_iter()
        .map(|symbol| {
            let hit = caught.get(symbol).copied().unwrap_or(0);
            let survived = missed.get(symbol).copied().unwrap_or(0);
            let viable = hit + survived;
            let mut entry = RunEntry::new(
                symbol.clone(),
                // The TOOL's own classification, not a threshold. A surviving
                // mutant is a demonstrated gap in the suite; calling it "fail"
                // reports what cargo-mutants found. Deciding whether a score is
                // ACCEPTABLE is the auditor's job, never this adapter's.
                if survived == 0 { Outcome::Pass } else { Outcome::Fail },
            );
            // `viable` is never 0: `symbols` is the union of the caught and
            // missed keys, so every symbol here has at least one mutant on one
            // side. A zero-guard would be a branch no input can reach.
            #[allow(
                clippy::cast_precision_loss,
                reason = "a mutant count large enough to lose f64 precision (2^53) is not a report cargo-mutants can produce; the retained TypeScript divides two doubles here and the store holds the same double"
            )]
            let score = hit as f64 / viable as f64;
            entry.score = Some(score);
            // The measurement is named where it is recorded (quoin#138).
            entry.metric = Some(MUTATION_SCORE_METRIC.to_owned());
            entry
        })
        .collect();

    Ok(AdapterResult::from_entries(entries))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::parse_cargo_mutants;
    use crate::types::{MUTATION_SCORE_METRIC, Outcome};

    const REPORT: &str = r#"{"outcomes":[
      {"scenario":"Baseline","summary":"Success"},
      {"scenario":{"Mutant":{"file":"src/a.rs","function":{"function_name":"f"}}},"summary":"CaughtMutant"},
      {"scenario":{"Mutant":{"file":"src/a.rs","function":{"function_name":"f"}}},"summary":"MissedMutant"},
      {"scenario":{"Mutant":{"file":"src/a.rs","function":{"function_name":"f"}}},"summary":"Unviable"},
      {"scenario":{"Mutant":{"file":"src/b.rs","function":{"function_name":"g"}}},"summary":"CaughtMutant"},
      {"scenario":{"Mutant":{"file":"src/c.rs","function":{"function_name":"h"}}},"summary":"Timeout"}
    ]}"#;

    #[test]
    fn scores_each_function_over_its_viable_mutants_only() {
        let result = parse_cargo_mutants(REPORT).unwrap();
        let symbols: Vec<&str> = result.entries.iter().map(|e| e.symbol.as_str()).collect();
        assert_eq!(symbols, ["src/a.rs::f", "src/b.rs::g", "src/c.rs::h"]);
        // Unviable is in neither numerator nor denominator: 1 of 2, not 1 of 3.
        assert!((result.entries[0].score.unwrap() - 0.5).abs() < f64::EPSILON);
        assert_eq!(result.entries[0].outcome, Outcome::Fail);
        assert!((result.entries[1].score.unwrap() - 1.0).abs() < f64::EPSILON);
        assert_eq!(result.entries[1].outcome, Outcome::Pass);
        // A timeout is a missed mutant.
        assert!(result.entries[2].score.unwrap().abs() < f64::EPSILON);
        assert_eq!(result.entries[2].outcome, Outcome::Fail);
        assert_eq!(
            result.entries[0].metric.as_deref(),
            Some(MUTATION_SCORE_METRIC)
        );
    }

    #[test]
    fn refuses_a_report_with_no_viable_mutant() {
        let error = parse_cargo_mutants(r#"{"outcomes":[{"scenario":"Baseline"}]}"#).unwrap_err();
        assert_eq!(
            error.to_string(),
            "cargo-mutants: no viable mutants in the report — nothing was measured"
        );
    }

    #[test]
    fn refuses_a_report_with_no_outcomes_array() {
        let error = parse_cargo_mutants("{}").unwrap_err();
        assert_eq!(
            error.to_string(),
            "cargo-mutants: no `outcomes` array — expected the contents of mutants.out/outcomes.json"
        );
    }
}
