// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Contract conformance JSONL — one replayed corpus fixture per line.
//!
//! Produced by `quire-contract-conformance run --manifest <corpus>` in
//! `agent-ix/quire-contract-ir`. The runner replays a pinned corpus and reports
//! whether each fixture's canonicalization matched; it decides nothing about
//! sufficiency, and neither does this.
//!
//! No existing adapter reads it. `entries` expects one JSON object with an
//! `entries` array, `junit` expects XML, and the finding-shaped adapters would
//! write it into `findings/` — where the clean-versus-unrun distinction that
//! FR-034 exists to make would be lost for every fixture in the corpus.

use serde_json::Value;

use super::AdapterResult;
use crate::error::EvidenceError;
use crate::types::{Outcome, RunEntry};

/// The one protocol this adapter reads.
///
/// Named rather than sniffed: a JSONL stream with different semantics under a
/// different protocol would parse just as cleanly and mean something else, and
/// a conformance run is exactly the place where a silently-misread row becomes
/// a passing fixture that was never checked.
pub const PROTOCOL: &str = "quire.contract.conformance-jsonl/v1";

/// Every status the protocol declares. An unknown one is refused, not skipped.
///
/// A slice and not a map keyed by `&str`, for the same reason the retained
/// source reaches for `Map` rather than an object literal: an object literal
/// resolves inherited property names like `valueOf`, so `"status": "valueOf"`
/// was read as a declared status. Rust has no prototype chain, so this is
/// stated rather than defended against — but the counterexample is kept as a
/// test so the property is observable here too.
const STATUS: &[(&str, Outcome)] = &[("match", Outcome::Pass), ("mismatch", Outcome::Fail)];

/// `JSON.stringify(value)` for the two diagnostics that interpolate one.
///
/// An absent member is `undefined` in JavaScript, and `${undefined}` renders as
/// the bare word. `serde_json` has no `undefined`, so absence is passed as
/// `None` and rendered the same way.
fn stringify(value: Option<&Value>) -> String {
    value.map_or_else(
        || "undefined".to_owned(),
        |value| serde_json::to_string(value).unwrap_or_else(|_| "undefined".to_owned()),
    )
}

fn refuse(message: String) -> EvidenceError {
    EvidenceError::adapter("contract-conformance", message)
}

/// Parse a contract-conformance JSONL stream.
///
/// # Errors
///
/// Refuses an empty stream, a line that is not JSON, a line under another
/// protocol, a line missing `corpus_id`, `fixture_id` or `operation`, a line
/// with an unknown status, and a line whose `trace_ids` is not a non-empty
/// array of distinct non-blank strings.
pub fn parse_contract_conformance(raw: &str) -> Result<AdapterResult, EvidenceError> {
    let lines: Vec<&str> = raw
        .split('\n')
        .filter(|line| !line.trim().is_empty())
        .collect();
    if lines.is_empty() {
        // An empty stream is not an empty corpus: a runner that produced no
        // line did not report that every fixture matched, it reported nothing.
        return Err(refuse(
            "input contains no conformance rows; an empty run is not a clean run".to_owned(),
        ));
    }

    let mut entries: Vec<RunEntry> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let number = index + 1;
        let row: Value = serde_json::from_str(line)
            .map_err(|error| refuse(format!("line {number} is not JSON: {error}")))?;
        if row.get("protocol").and_then(Value::as_str) != Some(PROTOCOL) {
            return Err(refuse(format!(
                "line {number} declares protocol {}, expected {PROTOCOL}",
                stringify(row.get("protocol"))
            )));
        }
        let mut named: Vec<&str> = Vec::with_capacity(3);
        for field in ["corpus_id", "fixture_id", "operation"] {
            match row.get(field).and_then(Value::as_str) {
                Some(value) if !value.is_empty() => named.push(value),
                _ => return Err(refuse(format!("line {number} has no {field}"))),
            }
        }
        let status = row.get("status").and_then(Value::as_str);
        let Some(outcome) = status.and_then(|status| {
            STATUS
                .iter()
                .find(|(name, _)| *name == status)
                .map(|(_, outcome)| *outcome)
        }) else {
            return Err(refuse(format!(
                "line {number} has unknown status {}",
                stringify(row.get("status"))
            )));
        };
        let trace_ids = match row.get("trace_ids") {
            None => None,
            Some(value) => Some(read_trace_ids(value, number)?),
        };
        let [corpus_id, fixture_id, operation] = named.as_slice() else {
            // Unreachable: the loop above pushes exactly three or returns.
            return Err(refuse(format!("line {number} has no corpus_id")));
        };
        let mut entry = RunEntry::new(
            // Corpus and operation are part of the identity: the same fixture id
            // is replayed under several operations, and collapsing them would
            // make one result overwrite another.
            format!("{corpus_id}::{operation}::{fixture_id}"),
            outcome,
        );
        entry.trace_ids = trace_ids;
        entries.push(entry);
    }

    Ok(AdapterResult::from_entries(entries))
}

/// Bindings belong to `record_run`: producer values and ordering are preserved.
/// Legacy producers may omit `trace_ids`; that must not invent a binding.
fn read_trace_ids(value: &Value, number: usize) -> Result<Vec<String>, EvidenceError> {
    let bad = || {
        refuse(format!(
            "line {number} trace_ids must be a non-empty array of distinct non-blank strings"
        ))
    };
    let array = value.as_array().ok_or_else(bad)?;
    if array.is_empty() {
        return Err(bad());
    }
    let mut ids: Vec<String> = Vec::with_capacity(array.len());
    for item in array {
        let id = item.as_str().ok_or_else(bad)?;
        if id.trim().is_empty() {
            return Err(bad());
        }
        ids.push(id.to_owned());
    }
    let mut distinct = ids.clone();
    distinct.sort_unstable();
    distinct.dedup();
    if distinct.len() != ids.len() {
        return Err(bad());
    }
    Ok(ids)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{PROTOCOL, parse_contract_conformance};
    use crate::types::Outcome;

    fn row(body: &str) -> String {
        format!(r#"{{"protocol":"{PROTOCOL}",{body}}}"#)
    }

    #[test]
    fn a_symbol_carries_corpus_operation_and_fixture() {
        let input = row(r#""corpus_id":"c","operation":"op","fixture_id":"f","status":"match""#);
        let result = parse_contract_conformance(&input).unwrap();
        assert_eq!(result.entries[0].symbol, "c::op::f");
        assert_eq!(result.entries[0].outcome, Outcome::Pass);
        assert!(result.entries[0].trace_ids.is_none());
    }

    #[test]
    fn a_status_named_for_an_inherited_property_is_refused() {
        // The counterexample CI's property test found against an object
        // literal. There is no prototype chain here, and this pins that.
        let input = row(r#""corpus_id":"c","operation":"o","fixture_id":"f","status":"valueOf""#);
        assert_eq!(
            parse_contract_conformance(&input).unwrap_err().to_string(),
            "contract-conformance: line 1 has unknown status \"valueOf\""
        );
    }

    #[test]
    fn refuses_a_missing_status_naming_it_undefined() {
        let input = row(r#""corpus_id":"c","operation":"o","fixture_id":"f""#);
        assert_eq!(
            parse_contract_conformance(&input).unwrap_err().to_string(),
            "contract-conformance: line 1 has unknown status undefined"
        );
    }

    #[test]
    fn refuses_every_malformed_trace_ids_shape() {
        for shape in [r#""x""#, "[]", r#"[""]"#, r#"[" "]"#, "[1]", r#"["a","a"]"#] {
            let input = row(&format!(
                r#""corpus_id":"c","operation":"o","fixture_id":"f","status":"match","trace_ids":{shape}"#
            ));
            assert_eq!(
                parse_contract_conformance(&input).unwrap_err().to_string(),
                "contract-conformance: line 1 trace_ids must be a non-empty array of distinct non-blank strings",
                "{shape}"
            );
        }
    }

    #[test]
    fn refuses_an_empty_stream_and_a_foreign_protocol() {
        assert_eq!(
            parse_contract_conformance("\n \n").unwrap_err().to_string(),
            "contract-conformance: input contains no conformance rows; an empty run is not a clean run"
        );
        assert_eq!(
            parse_contract_conformance(r#"{"protocol":"other/v1"}"#)
                .unwrap_err()
                .to_string(),
            format!(
                "contract-conformance: line 1 declares protocol \"other/v1\", expected {PROTOCOL}"
            )
        );
    }

    #[test]
    fn the_line_number_is_one_based_and_counts_only_non_blank_lines() {
        let good = row(r#""corpus_id":"c","operation":"o","fixture_id":"f","status":"match""#);
        let input = format!("{good}\n\n{{\"protocol\":\"other\"}}");
        assert!(
            parse_contract_conformance(&input)
                .unwrap_err()
                .to_string()
                .contains("line 2 declares protocol")
        );
    }
}
