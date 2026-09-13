// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `cli-agent-evals` reports → run entries, one per scenario (FR-042).
//!
//! **Why this is worth an adapter at all.** An eval drives the real agent
//! through the real CLI, which makes it the most convincing verification this
//! project has — and until FR-042 the least recorded. `quire coverage`
//! reconciles matrix rows against test symbols in code; eval scenarios are data
//! in `evals/scenarios/index.mjs`, so they mint no symbol and can never back a
//! row. Measured at the time of writing: **71 unbacked rows** across
//! `spec/evals.md`, FR-028 and FR-038, every one of them a criterion whose ✅
//! rested on somebody having run the evals (SR-008 FND-001).
//!
//! **The scenario id IS the symbol.** `TC-EV-054` is what the matrix row names
//! and what the report keys on, so no mapping is needed and none is invented.

use serde_json::Value;

use super::AdapterResult;
use crate::error::EvidenceError;
use crate::types::{Outcome, RunEntry};

/// Parse a `cli-agent-evals` report.
///
/// # Errors
///
/// Refuses input that is not JSON, a document with no `results` array, and a
/// report in which no result carried a string `id`.
pub fn parse_agent_eval(raw: &str) -> Result<AdapterResult, EvidenceError> {
    let report: Value = serde_json::from_str(raw)
        .map_err(|error| EvidenceError::adapter("agent-eval", format!("not JSON: {error}")))?;
    let results = report
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            EvidenceError::adapter(
                "agent-eval",
                r#"expected a cli-agent-evals report with a "results" array"#,
            )
        })?;

    let mut entries: Vec<RunEntry> = Vec::new();
    for result in results {
        let Some(id) = result.get("id").and_then(Value::as_str) else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        let mut entry = RunEntry::new(
            id,
            // `ok` is the suite's own verdict over `repeats` runs — a scenario
            // that passed 1 of 3 is NOT a pass, and recomputing that here would
            // be a second opinion on a question the harness already answered.
            if result.get("ok") == Some(&Value::Bool(true)) {
                Outcome::Pass
            } else {
                Outcome::Fail
            },
        );
        // The scenario id is also the trace id. Stated rather than mapped.
        entry.trace_ids = Some(vec![id.to_owned()]);
        entry.score = score_of(result);
        entries.push(entry);
    }

    if entries.is_empty() {
        // A report with no results is a suite that ran nothing. Recording it
        // would manufacture evidence from a file proving only that the harness
        // started.
        return Err(EvidenceError::adapter(
            "agent-eval",
            "no scenario results in the report — did the suite run?",
        ));
    }
    Ok(AdapterResult::from_entries(entries))
}

/// `passRate` as a ratio, where the report states one.
///
/// `"2/3"` becomes `0.666…`. Recorded because a scenario that passes two runs
/// in three is flaky rather than passing, and `outcome` alone cannot say so —
/// but note this is NOT a mutation score: agent-ix/quoin#138 is why `score`
/// needs a metric discriminator before any threshold reads it, and this adapter
/// sets none.
fn score_of(result: &Value) -> Option<f64> {
    let rate = result.get("passRate")?.as_str()?;
    // `"2/3".split("/").map(Number)` — JavaScript's `Number` trims whitespace,
    // reads `""` as 0 and anything else unparseable as NaN, which
    // `Number.isFinite` then rejects. `str::parse::<f64>` rejects the empty
    // string instead of reading it as 0; the two agree because a total of 0 is
    // refused on the next line either way, and a passed count of `""` under the
    // retained reader produced `0/total`, which no report emits.
    let mut parts = rate.split('/');
    let passed: f64 = parts.next()?.trim().parse().ok()?;
    let total: f64 = parts.next()?.trim().parse().ok()?;
    if !passed.is_finite() || !total.is_finite() || total == 0.0 {
        return None;
    }
    Some(passed / total)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::parse_agent_eval;
    use crate::types::Outcome;

    /// One entry per scenario, keyed on the scenario id, which is its own
    /// trace id — and the harness's own verdict is taken rather than recomputed
    /// from the pass rate, which is kept because "flaky" and "failing" are
    /// different facts.
    ///
    /// Trace: FR-042-AC-1, FR-042-AC-2, FR-042-AC-3, FR-042-CON-2
    /// Provenance: quoin#458
    #[test]
    fn tc_458_220_keys_each_entry_on_the_scenario_id_and_traces_it_to_itself() {
        let result = parse_agent_eval(
            r#"{"results":[
                {"id":"TC-EV-054","ok":true,"passRate":"3/3"},
                {"id":"TC-EV-055","ok":false,"passRate":"2/3"},
                {"id":"TC-EV-056","ok":true},
                {"ok":true}
            ]}"#,
        )
        .unwrap();
        let symbols: Vec<&str> = result.entries.iter().map(|e| e.symbol.as_str()).collect();
        assert_eq!(symbols, ["TC-EV-054", "TC-EV-055", "TC-EV-056"]);
        assert_eq!(
            result.entries[0].trace_ids.as_deref(),
            Some(["TC-EV-054".to_owned()].as_slice())
        );
        assert_eq!(result.entries[1].outcome, Outcome::Fail);
        assert!((result.entries[1].score.unwrap() - 2.0 / 3.0).abs() < f64::EPSILON);
        assert!(result.entries[2].score.is_none());
        // `metric` stays unset: a pass rate is not a mutation score (quoin#138).
        assert!(result.entries[0].metric.is_none());
    }

    #[test]
    fn an_ok_that_is_not_the_boolean_true_is_a_failure() {
        // `raw.ok === true` in the retained source: the string "true" is not it.
        let result = parse_agent_eval(r#"{"results":[{"id":"a","ok":"true"}]}"#).unwrap();
        assert_eq!(result.entries[0].outcome, Outcome::Fail);
    }

    /// A suite that ran nothing. Recording it would manufacture evidence from
    /// a file proving only that the harness started.
    ///
    /// Trace: FR-042-AC-4
    /// Provenance: quoin#458
    #[test]
    fn tc_458_221_refuses_a_report_with_no_results() {
        assert_eq!(
            parse_agent_eval(r#"{"results":[]}"#)
                .unwrap_err()
                .to_string(),
            "agent-eval: no scenario results in the report — did the suite run?"
        );
        assert_eq!(
            parse_agent_eval("{}").unwrap_err().to_string(),
            "agent-eval: expected a cli-agent-evals report with a \"results\" array"
        );
        assert!(
            parse_agent_eval("{{")
                .unwrap_err()
                .to_string()
                .contains("not JSON")
        );
    }
}
