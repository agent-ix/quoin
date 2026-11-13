// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quire.differential-report/v2` — one analysis request judged by two SMT
//! engines (FR-151).
//!
//! The report contract belongs to `agent-ix/quire-analyze`. This reader does
//! not restate it: `quire_analyze::validate_report_document` decides whether a
//! document is well formed, and this module adds only the mapping of an
//! accepted report to a run entry or an unrepresented result. It derives no
//! digest, decodes no hex and parses no solver output.
//!
//! One report is one comparison, so it is one entry. A `disagreement` is the
//! one disposition no run-entry outcome carries, so it is named, not rounded.

use ix_trace_rs::implements;
use quire_analyze::{MAX_REPORT_BYTES, validate_report_document};
use serde_json::Value;

use super::{AdapterResult, UnrepresentedResult};
use crate::error::EvidenceError;
use crate::types::{Outcome, RunEntry};

/// The schema this reader accepts. Used only to word a refusal; acceptance is
/// the owner's validator's.
const SCHEMA: &str = "quire.differential-report/v2";

fn refuse(message: impl Into<String>) -> EvidenceError {
    EvidenceError::adapter("differential-report", message)
}

/// Why a document the validator refused was refused, naming a wrong `schema`
/// value when that is the cause so a v1, v3 or retired-schema document says so.
fn refusal(raw: &str, validator_error: &str) -> EvidenceError {
    let found = serde_json::from_str::<Value>(raw).ok().and_then(|value| {
        let object = value.as_object()?;
        match object.get("schema") {
            Some(Value::String(schema)) if schema == SCHEMA => None,
            Some(other) => Some(other.to_string()),
            None => Some("absent".to_owned()),
        }
    });
    match found {
        Some(found) => refuse(format!("schema {found} is not {SCHEMA}: {validator_error}")),
        None => refuse(validator_error.to_owned()),
    }
}

/// The text of a member the validator has already proved is a string.
fn text<'a>(value: &'a Value, member: &str) -> Result<&'a str, EvidenceError> {
    value
        .get(member)
        .and_then(Value::as_str)
        .ok_or_else(|| refuse(format!("validated report lacks string member {member}")))
}

/// `<engine> status <status> outcome <outcome>`, each verbatim from the report.
fn describe_engine(engine: &Value) -> Result<String, EvidenceError> {
    Ok(format!(
        "{} status {} outcome {}",
        text(engine, "engine")?,
        text(engine, "status")?,
        text(engine, "outcome")?,
    ))
}

/// Transcribe a `quire.differential-report/v2` document.
///
/// # Errors
///
/// Refuses input larger than [`MAX_REPORT_BYTES`] without parsing it, and any
/// document `quire_analyze::validate_report_document` refuses (wrong or absent
/// schema, a missing, undeclared or mistyped member, a self-contradicting
/// disposition, non-canonical bytes).
#[implements("FR-151")]
pub fn parse_differential_report(raw: &str) -> Result<AdapterResult, EvidenceError> {
    if raw.len() > MAX_REPORT_BYTES {
        return Err(refuse(format!(
            "report is {} bytes, over the {MAX_REPORT_BYTES}-byte limit",
            raw.len()
        )));
    }
    validate_report_document(raw.as_bytes()).map_err(|error| refusal(raw, &error))?;
    let report: Value = serde_json::from_str(raw)
        .map_err(|error| refuse(format!("validated report is not JSON: {error}")))?;

    let symbol = format!(
        "{}::{}",
        text(&report, "analysisKind")?,
        text(&report, "requestDigest")?
    );
    let differential = report
        .get("differential")
        .ok_or_else(|| refuse("validated report lacks differential"))?;
    let agreed = differential.get("agreedStatus").and_then(Value::as_str);
    let outcome = match (text(differential, "disposition")?, agreed) {
        ("agreement", Some("satisfied")) => Outcome::Pass,
        ("agreement", Some("refuted")) => Outcome::Fail,
        ("inconclusive" | "unavailable", None) => Outcome::Error,
        ("disagreement", None) => {
            let engines = report
                .get("engines")
                .and_then(Value::as_array)
                .ok_or_else(|| refuse("validated report lacks engines"))?;
            let described = engines
                .iter()
                .map(describe_engine)
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(AdapterResult {
                entries: Vec::new(),
                unrepresented: Some(vec![UnrepresentedResult {
                    symbol,
                    state: "disagreement".to_owned(),
                    reason: format!(
                        "the two engines both ran and concluded opposite results ({}); that is neither a pass nor a fail nor an error, and no run-entry outcome carries it",
                        described.join("; ")
                    ),
                }]),
                evidence_kind: None,
            });
        }
        (disposition, status) => {
            return Err(refuse(format!(
                "validated report has disposition {disposition} with agreedStatus {status:?}"
            )));
        }
    };

    Ok(AdapterResult {
        entries: vec![RunEntry::new(symbol, outcome)],
        unrepresented: None,
        evidence_kind: None,
    })
}
