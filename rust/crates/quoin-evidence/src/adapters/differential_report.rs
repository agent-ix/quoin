// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain differential reports — a domain engine compared against an external
//! reference implementation, case by case.
//!
//! Produced by `agent-ix/tl-mltl` against R2U2/C2PO, and shaped for any domain
//! that runs the same comparison. The report states agreement, mismatch,
//! unsupported and tool-error counts; it reaches no verdict about whether the
//! comparison was sufficient, and neither does this.
//!
//! No existing adapter reads it. `junit` has no state for "the external tool
//! cannot express this case", `cargo-mutants` is mutation-shaped, `agent-eval`
//! is measurement-shaped, and the finding-shaped adapters would record a
//! disagreement as a rule violation — which is a different claim from "two
//! implementations disagree", and one the report does not make.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

use super::{AdapterResult, UnrepresentedResult};
use crate::error::EvidenceError;
use crate::types::{Outcome, RunEntry};

/// The declared schema family this adapter reads.
///
/// A domain declares its own prefix — `tl-mltl.differential-summary/v1` — so
/// two domains can use the same shape without either claiming the other's
/// identity. The version is pinned: a v2 with different semantics must not be
/// read by a v1 reader that happens to find the fields it knows.
#[allow(
    clippy::expect_used,
    reason = "the pattern is a module literal covered by this module's tests; a failure here is a typo caught by the suite, not a runtime condition"
)]
static SCHEMA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-z0-9][a-z0-9\-]*\.differential-summary/v1$")
        .expect("adapter pattern literal must compile")
});

/// Case states this adapter transcribes into run entries.
///
/// `unsupported` is deliberately absent — see [`UnrepresentedResult`]. A slice
/// and not an object literal for the reason the retained source gives: an
/// object literal resolves through the prototype chain, so `"valueOf"` was
/// accepted as a declared status and transcribed with a function as its
/// outcome. CI's property test found it with exactly that counterexample.
const STATUS: &[(&str, Outcome)] = &[
    ("agreement", Outcome::Pass),
    ("mismatch", Outcome::Fail),
    ("tool-error", Outcome::Error),
];

/// Why `unsupported` carries no run-entry outcome. Stated once; the store holds
/// these bytes.
const UNSUPPORTED_REASON: &str = "the external reference does not support this case; that is neither a skip nor an error, and no run-entry outcome carries it";

fn refuse(message: impl Into<String>) -> EvidenceError {
    EvidenceError::adapter("differential-report", message)
}

/// `JSON.stringify(value)`, with an absent member rendering as `undefined`.
fn stringify(value: Option<&Value>) -> String {
    value.map_or_else(
        || "undefined".to_owned(),
        |value| serde_json::to_string(value).unwrap_or_else(|_| "undefined".to_owned()),
    )
}

/// Parse a `<domain>.differential-summary/v1` report.
///
/// # Errors
///
/// Refuses input that is not JSON, a report whose `schemaVersion` is not the
/// declared family, a report with no `cases` array, an empty `cases` array, a
/// case with no id or no status, and a case with an unknown status.
pub fn parse_differential_report(raw: &str) -> Result<AdapterResult, EvidenceError> {
    let report: Value =
        serde_json::from_str(raw).map_err(|error| refuse(format!("input is not JSON: {error}")))?;
    let schema_version = report.get("schemaVersion").and_then(Value::as_str);
    if !schema_version.is_some_and(|version| SCHEMA.is_match(version)) {
        return Err(refuse(format!(
            "unknown schemaVersion {}; expected <domain>.differential-summary/v1",
            stringify(report.get("schemaVersion"))
        )));
    }
    let cases = report
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| refuse("report has no cases array"))?;
    if cases.is_empty() {
        // A comparison that examined nothing agrees with nothing. Recording it
        // as a clean run is how a vacuous differential becomes evidence.
        return Err(refuse(
            "report contains no cases; a comparison that examined nothing is not an agreement",
        ));
    }

    let mut entries: Vec<RunEntry> = Vec::new();
    let mut unrepresented: Vec<UnrepresentedResult> = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let id = case
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| refuse(format!("case {index} has no id")))?;
        let status = case
            .get("status")
            .and_then(Value::as_str)
            .filter(|status| !status.is_empty())
            .ok_or_else(|| refuse(format!("case {id} has no status")))?;
        if status == "unsupported" {
            unrepresented.push(UnrepresentedResult {
                symbol: id.to_owned(),
                state: "unsupported".to_owned(),
                reason: UNSUPPORTED_REASON.to_owned(),
            });
            continue;
        }
        let Some(outcome) = STATUS
            .iter()
            .find(|(name, _)| *name == status)
            .map(|(_, outcome)| *outcome)
        else {
            return Err(refuse(format!(
                "case {id} has unknown status {}",
                stringify(case.get("status"))
            )));
        };
        entries.push(RunEntry::new(id, outcome));
    }

    Ok(AdapterResult {
        entries,
        // Never `Some(vec![])`: an adapter with nothing to report omits the
        // field rather than asserting an empty list.
        unrepresented: (!unrepresented.is_empty()).then_some(unrepresented),
        evidence_kind: None,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::parse_differential_report;
    use crate::types::Outcome;

    const REPORT: &str = r#"{"schemaVersion":"tl-mltl.differential-summary/v1","cases":[
        {"id":"c1","status":"agreement"},
        {"id":"c2","status":"mismatch"},
        {"id":"c3","status":"tool-error"},
        {"id":"c4","status":"unsupported"}
    ]}"#;

    #[test]
    fn unsupported_is_named_rather_than_mapped_onto_the_nearest_outcome() {
        let result = parse_differential_report(REPORT).unwrap();
        let outcomes: Vec<Outcome> = result.entries.iter().map(|e| e.outcome).collect();
        assert_eq!(outcomes, [Outcome::Pass, Outcome::Fail, Outcome::Error]);
        let unrepresented = result.unrepresented.unwrap();
        assert_eq!(unrepresented.len(), 1);
        assert_eq!(unrepresented[0].symbol, "c4");
        assert_eq!(unrepresented[0].state, "unsupported");
    }

    #[test]
    fn a_report_with_nothing_unrepresented_omits_the_field() {
        let result = parse_differential_report(
            r#"{"schemaVersion":"a.differential-summary/v1","cases":[{"id":"c","status":"agreement"}]}"#,
        )
        .unwrap();
        assert!(result.unrepresented.is_none());
    }

    #[test]
    fn refuses_an_inherited_property_name_as_a_status() {
        let error = parse_differential_report(
            r#"{"schemaVersion":"a.differential-summary/v1","cases":[{"id":"c","status":"valueOf"}]}"#,
        )
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "differential-report: case c has unknown status \"valueOf\""
        );
    }

    #[test]
    fn refuses_a_schema_version_outside_the_declared_family() {
        for version in [
            r#""tl-mltl.differential-summary/v2""#,
            r#""TL.differential-summary/v1""#,
            r#""-x.differential-summary/v1""#,
            "1",
        ] {
            let input = format!(
                r#"{{"schemaVersion":{version},"cases":[{{"id":"c","status":"agreement"}}]}}"#
            );
            assert!(
                parse_differential_report(&input)
                    .unwrap_err()
                    .to_string()
                    .starts_with("differential-report: unknown schemaVersion "),
                "{version}"
            );
        }
    }

    #[test]
    fn refuses_an_empty_case_list_and_a_case_with_no_id() {
        assert_eq!(
            parse_differential_report(
                r#"{"schemaVersion":"a.differential-summary/v1","cases":[]}"#
            )
            .unwrap_err()
            .to_string(),
            "differential-report: report contains no cases; a comparison that examined nothing is not an agreement"
        );
        assert_eq!(
            parse_differential_report(
                r#"{"schemaVersion":"a.differential-summary/v1","cases":[{"status":"agreement"}]}"#
            )
            .unwrap_err()
            .to_string(),
            "differential-report: case 0 has no id"
        );
    }
}
