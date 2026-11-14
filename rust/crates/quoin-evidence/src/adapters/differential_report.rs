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
use quoin_store::{canonical_bytes, parse_strict_json_str};
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

/// Refuse a report whose bytes are not the canonical compact form, whatever
/// the validator's own check accepted.
///
/// The validator compares the input with a re-serialisation of the parsed
/// document, which preserves member order whenever any crate in the build
/// enables `serde_json/preserve_order`; a report with reordered members then
/// round-trips and is accepted. The re-encoding here sorts members through the
/// store's canonical JSON, so the answer does not depend on feature unification.
fn require_canonical(raw: &str) -> Result<(), EvidenceError> {
    let canonical = parse_strict_json_str(raw)
        .and_then(|value| canonical_bytes(&value))
        .map_err(|error| refuse(format!("report is not canonical JSON: {error}")))?;
    if canonical == raw.as_bytes() {
        Ok(())
    } else {
        Err(refuse("report JSON is not in canonical compact form"))
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
    require_canonical(raw)?;
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
    let projection = analysis_projection(&report, differential)?;
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
                analysis_projection: Some(projection),
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
        analysis_projection: Some(projection),
    })
}

/// Copy only the FR-151 projection allowlist from an already validated report.
fn analysis_projection(report: &Value, differential: &Value) -> Result<Value, EvidenceError> {
    let producer = report
        .get("producer")
        .and_then(Value::as_object)
        .ok_or_else(|| refuse("validated report lacks producer"))?;
    let mut engines = Vec::with_capacity(2);
    for engine in report
        .get("engines")
        .and_then(Value::as_array)
        .ok_or_else(|| refuse("validated report lacks engines"))?
    {
        let source = engine
            .as_object()
            .ok_or_else(|| refuse("validated engine is not an object"))?;
        let identity = source.get("identity").and_then(Value::as_object);
        let mut projected = serde_json::Map::new();
        for (from, to) in [
            ("engine", "engine"),
            ("configuredExecutable", "configured_executable"),
            ("status", "status"),
            ("outcome", "outcome"),
        ] {
            if let Some(value) = source.get(from) {
                projected.insert(to.to_owned(), value.clone());
            }
        }
        if let Some(identity) = identity {
            if let Some(value) = identity.get("path") {
                projected.insert("identity_path".to_owned(), value.clone());
            }
            if let Some(value) = identity.get("version") {
                projected.insert("identity_version".to_owned(), value.clone());
            }
        }
        engines.push(Value::Object(projected));
    }
    let agreed_status = differential
        .get("agreedStatus")
        .cloned()
        .unwrap_or(Value::Null);
    Ok(serde_json::json!({
        "protocol": "quoin.analysis-report-projection/v1",
        "disposition": differential.get("disposition").cloned().unwrap_or(Value::Null),
        "agreed_status": agreed_status,
        "producer": {
            "name": producer.get("name").cloned().unwrap_or(Value::Null),
            "version": producer.get("version").cloned().unwrap_or(Value::Null),
        },
        "engines": engines,
    }))
}
