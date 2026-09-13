// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading a `cli-agent-evals` report as pass rates per scenario.
//!
//! A port of `reportFrom` (`agent-eval-intervention.ts:145-181`).
//!
//! # This is not the adapter
//!
//! [`quoin_evidence::adapters::parse_agent_eval`] is the FR-042
//! refusal boundary for this format and runs first, unchanged and by name
//! (`agent-eval-intervention.ts:21-27`). What is here is the
//! intervention-specific projection the adapter does not compute: one pass rate
//! per scenario, with the report's own `repeats` as the cross-check. The two
//! are kept apart on purpose — a second copy of the adapter's refusals here
//! would be a second place to disagree about what the format is.
//!
//! # Order
//!
//! The retained code builds a `Map` and sorts its keys before use. This holds
//! a [`BTreeMap`], which *is* that sort, so the ordering cannot be skipped at a
//! call site. The two orders differ only for scenario ids outside the BMP
//! (`DIVERGENCE.md` §11.5).

use std::collections::BTreeMap;

use serde_json::Value;

use crate::common::scalar::js_string;
use crate::intervention::intake::{InterventionIntakeError, InterventionRefusalCode};

/// One scenario's outcome across the repeats of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenarioRate {
    /// How many repeats passed.
    pub passed: u64,
    /// How many repeats ran. Equal to the report's `repeats`.
    pub total: u64,
}

impl ScenarioRate {
    /// The pass rate as the record carries it.
    ///
    /// Total by construction: [`parse`](AgentEvalReport::parse) admits a rate
    /// only when its total equals the report's `repeats`, which is at least
    /// one, so this cannot divide by zero.
    #[must_use]
    #[expect(
        clippy::cast_precision_loss,
        reason = "the oracle's `passed / total` is IEEE-754 double division; \
                  reproducing it exactly is the point"
    )]
    pub fn rate(self) -> f64 {
        self.passed as f64 / self.total as f64
    }
}

/// A `cli-agent-evals` report, as an intervention reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentEvalReport {
    /// The report's `generatedAt`, which becomes the record's `observed_at`.
    generated_at: String,
    /// One rate per scenario id, in scenario-id order.
    scenarios: BTreeMap<String, ScenarioRate>,
}

impl AgentEvalReport {
    /// Read a report, naming the arm so a refusal says which file failed.
    ///
    /// # Errors
    ///
    /// [`InterventionRefusalCode::InvalidRecord`], carrying the retained
    /// sentence for whichever of the seven checks failed first.
    pub fn parse(raw: &str, label: &str) -> Result<Self, InterventionIntakeError> {
        // `JSON.parse`, and deliberately not `quoin_store::parse_strict_json`:
        // this is a foreign producer's file, read by the same reader the
        // adapter that already accepted it used. A stricter parse here would
        // refuse a file the FR-042 boundary admitted, which is a refusal this
        // port would have invented.
        let root: Value = serde_json::from_str(raw)
            .map_err(|error| refuse(format!("{label} agent-eval report is not JSON: {error}")))?;
        let generated_at = root
            .get("generatedAt")
            .and_then(Value::as_str)
            .ok_or_else(|| refuse(format!("{label} agent-eval report lacks generatedAt")))?
            .to_owned();
        let repeats = root.get("repeats").and_then(integer).ok_or_else(|| {
            refuse(format!(
                "{label} agent-eval report is structurally incompatible: positive repeats are \
                 required"
            ))
        })?;
        let results = root
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                // The oracle reaches `(root.results as unknown[]).entries()`
                // with no check and throws a bare `TypeError` here. It is
                // unreachable through the producer — `parse_agent_eval` has
                // already refused a report with no `results` array — so this
                // says what happened rather than reproducing a TypeError's
                // wording (`DIVERGENCE.md` §11.6).
                refuse(format!("{label} agent-eval report lacks a results array"))
            })?;

        let mut scenarios = BTreeMap::new();
        for (index, result) in results.iter().enumerate() {
            let id = result
                .as_object()
                .and_then(|object| object.get("id"))
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| refuse(format!("{label} result {index} lacks a scenario id")))?;
            if scenarios.contains_key(id) {
                return Err(refuse(format!("{label} report repeats scenario id {id}")));
            }
            let rate = pass_rate(result.get("passRate"))
                .ok_or_else(|| refuse(format!("{label} scenario {id} has invalid passRate")))?;
            if rate.total < 1 || rate.passed > rate.total || rate.total != repeats {
                return Err(refuse(format!(
                    "{label} scenario {id} has inconsistent sample count"
                )));
            }
            scenarios.insert(id.to_owned(), rate);
        }
        if scenarios.is_empty() {
            return Err(refuse(format!("{label} report contains no scenarios")));
        }
        Ok(Self {
            generated_at,
            scenarios,
        })
    }

    /// When the report was generated.
    #[must_use]
    pub fn generated_at(&self) -> &str {
        &self.generated_at
    }

    /// The scenarios, in scenario-id order.
    #[must_use]
    pub const fn scenarios(&self) -> &BTreeMap<String, ScenarioRate> {
        &self.scenarios
    }

    /// The sample size the arm records: every repeat of every scenario.
    #[must_use]
    pub fn sample_size(&self) -> u64 {
        self.scenarios.values().map(|rate| rate.total).sum()
    }
}

/// `Number.isInteger(x) && Number(x) >= 1`.
fn integer(value: &Value) -> Option<u64> {
    let double = value.as_f64()?;
    // `Number.isInteger` is false for a non-finite double and true for any
    // integral one; `as_u64` is the same predicate for the range that can then
    // be a repeat count.
    (double.fract() == 0.0)
        .then(|| value.as_u64())
        .flatten()
        .filter(|repeats| *repeats >= 1)
}

/// `/^(\d+)\/(\d+)$/` over `String(result.passRate ?? "")`.
///
/// The grammar only; `total < 1` is refused by the caller, in the oracle's
/// order, so that a zero denominator keeps the oracle's sentence.
fn pass_rate(value: Option<&Value>) -> Option<ScenarioRate> {
    // `?? ""` maps only null and undefined; every other type is stringified.
    let rendered = match value {
        None | Some(Value::Null) => String::new(),
        Some(found) => js_string(Some(found)),
    };
    let (passed, total) = rendered.split_once('/')?;
    let digits = |part: &str| {
        (!part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse::<u64>().ok())
            .flatten()
    };
    Some(ScenarioRate {
        passed: digits(passed)?,
        total: digits(total)?,
    })
}

/// Every refusal here is the record the producer was asked to assemble.
fn refuse(finding: String) -> InterventionIntakeError {
    InterventionIntakeError::new(InterventionRefusalCode::InvalidRecord, vec![finding])
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::AgentEvalReport;

    fn report(results: &str, repeats: &str) -> String {
        format!(
            r#"{{"generatedAt":"2026-01-01T00:00:00Z","repeats":{repeats},"results":{results}}}"#
        )
    }

    /// Trace: FR-100-AC-6
    /// Provenance: quoin#471
    #[test]
    fn a_report_reads_as_one_rate_per_scenario_in_id_order() {
        let raw = report(
            r#"[{"id":"zeta","passRate":"1/2"},{"id":"alpha","passRate":"2/2"}]"#,
            "2",
        );
        let parsed = AgentEvalReport::parse(&raw, "baseline").unwrap();
        assert_eq!(parsed.generated_at(), "2026-01-01T00:00:00Z");
        assert_eq!(
            parsed.scenarios().keys().collect::<Vec<_>>(),
            ["alpha", "zeta"]
        );
        assert_eq!(parsed.sample_size(), 4);
        assert!((parsed.scenarios()["zeta"].rate() - 0.5).abs() < f64::EPSILON);
    }

    /// Trace: FR-100-AC-6
    /// Provenance: quoin#471
    ///
    /// Every refusal sentence is the retained one, spelled here rather than
    /// derived, so a reworded message is a test failure and not a silent
    /// contract change.
    #[test]
    fn each_structural_refusal_keeps_its_sentence() {
        for (raw, expected) in [
            (
                r#"{"repeats":1,"results":[]}"#.to_owned(),
                "baseline agent-eval report lacks generatedAt",
            ),
            (
                report(r#"[{"id":"a","passRate":"1/1"}]"#, "0"),
                "baseline agent-eval report is structurally incompatible: positive repeats are \
                 required",
            ),
            (
                report(r#"[{"id":"a","passRate":"1/1"}]"#, "1.5"),
                "baseline agent-eval report is structurally incompatible: positive repeats are \
                 required",
            ),
            (
                report(r#"[{"passRate":"1/1"}]"#, "1"),
                "baseline result 0 lacks a scenario id",
            ),
            (
                report(r#"[{"id":"","passRate":"1/1"}]"#, "1"),
                "baseline result 0 lacks a scenario id",
            ),
            (
                report(
                    r#"[{"id":"a","passRate":"1/1"},{"id":"a","passRate":"1/1"}]"#,
                    "1",
                ),
                "baseline report repeats scenario id a",
            ),
            (
                report(r#"[{"id":"a"}]"#, "1"),
                "baseline scenario a has invalid passRate",
            ),
            (
                report(r#"[{"id":"a","passRate":"1/0"}]"#, "1"),
                "baseline scenario a has inconsistent sample count",
            ),
            (
                report(r#"[{"id":"a","passRate":"2/1"}]"#, "1"),
                "baseline scenario a has inconsistent sample count",
            ),
            (
                report(r#"[{"id":"a","passRate":"1/1"}]"#, "2"),
                "baseline scenario a has inconsistent sample count",
            ),
            (report("[]", "1"), "baseline report contains no scenarios"),
        ] {
            let error = AgentEvalReport::parse(&raw, "baseline").unwrap_err();
            assert_eq!(error.findings(), [expected], "{raw}");
        }
    }
}
