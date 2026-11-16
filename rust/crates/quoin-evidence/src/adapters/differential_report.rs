// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `filament.differential-report/v2` — one analysis request judged by two SMT
//! engines (FR-151).
//!
//! The report contract belongs to `agent-ix/filament-analyze`. This reader does
//! not restate it: `filament_analyze::validate_report_document` decides whether a
//! document is well formed, and this module adds only the mapping of an
//! accepted report to a run entry or an unrepresented result. It derives no
//! report field, decodes no hex and parses no solver output.
//!
//! One report is one comparison, so it is one entry. A `disagreement` is the
//! one disposition no run-entry outcome carries, so it is named, not rounded.

use filament_analyze::{MAX_REPORT_BYTES, validate_report_document};
use ix_trace_rs::implements;
use quoin_store::{canonical_bytes, parse_strict_json_str};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use super::{AdapterResult, UnrepresentedResult};
use crate::error::EvidenceError;
use crate::types::{Outcome, RunEntry};

/// The schema this reader accepts. Used only to word a refusal; acceptance is
/// the owner's validator's.
const SCHEMA: &str = "filament.differential-report/v2";

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

/// Transcribe a `filament.differential-report/v2` document.
///
/// # Errors
///
/// Refuses input larger than [`MAX_REPORT_BYTES`] without parsing it, and any
/// document `filament_analyze::validate_report_document` refuses (wrong or absent
/// schema, a missing, undeclared or mistyped member, a self-contradicting
/// disposition, non-canonical bytes).
#[implements("FR-151")]
pub fn parse_differential_report(raw: &str) -> Result<AdapterResult, EvidenceError> {
    parse_differential_report_inner(raw, None)
}

/// Transcribe a report for a newly recorded run with an explicit purpose.
///
/// `formal_rule_selection` is required for `formal_rule` and forbidden for
/// `analysis`; it is checked against the validated report before projection.
pub fn parse_differential_report_for_run(
    raw: &str,
    run_kind: &str,
    formal_rule_selection: Option<&Value>,
) -> Result<AdapterResult, EvidenceError> {
    if !matches!(run_kind, "analysis" | "formal_rule") {
        return Err(refuse(format!("unsupported run_kind {run_kind:?}")));
    }
    match (run_kind, formal_rule_selection) {
        ("analysis", Some(_)) => {
            return Err(refuse(
                "formal_rule_selection must be absent for run_kind analysis",
            ));
        }
        ("formal_rule", None) => {
            return Err(refuse(
                "formal_rule_selection is required for run_kind formal_rule",
            ));
        }
        _ => {}
    }
    parse_differential_report_inner(raw, Some((run_kind, formal_rule_selection)))
}

/// Build the host-supplied mapping used to record a formal-rule judgement.
/// `claim_symbols` comes from the package selection prepared for this run;
/// status is accepted only from the matching FR-188 status report and only
/// when every selected symbol is approved.
pub fn build_approved_formal_rule_selection(
    report: &Value,
    claim_symbols: &BTreeMap<String, String>,
    status: &Value,
) -> Result<Value, String> {
    if status.get("protocol").and_then(Value::as_str) != Some("quoin.formal-rule-status/v1") {
        return Err("formal-rule status report has an unsupported protocol".to_owned());
    }
    let selected = report
        .get("selection")
        .and_then(|value| value.get("selected"))
        .and_then(Value::as_array)
        .ok_or_else(|| "formal-rule report has no selected claim list".to_owned())?;
    let assertions = report
        .get("assertions")
        .and_then(Value::as_array)
        .ok_or_else(|| "formal-rule report has no assertion list".to_owned())?;
    let rules = status
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| "formal-rule status report has no rules list".to_owned())?;
    let selected_ids = selected
        .iter()
        .map(|claim| {
            claim
                .as_str()
                .ok_or_else(|| "selected claim id is not a string".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let unique = selected_ids.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != selected_ids.len() || claim_symbols.len() != selected_ids.len() {
        return Err("formal-rule claim mapping is duplicate or incomplete".to_owned());
    }
    let mut claims = Vec::with_capacity(selected_ids.len());
    for claim_id in selected_ids {
        let symbol = claim_symbols
            .get(claim_id)
            .ok_or_else(|| format!("formal-rule symbol is missing for {claim_id}"))?;
        let assertion = assertions
            .iter()
            .find(|item| {
                item.get("role").and_then(Value::as_str) == Some("selected")
                    && item
                        .get("clause")
                        .and_then(Value::as_str)
                        .is_some_and(|clause| clause.ends_with(&format!(":{claim_id}")))
            })
            .ok_or_else(|| format!("selected report claim {claim_id} has no assertion"))?;
        let matching = rules
            .iter()
            .filter(|rule| rule.get("symbol").and_then(Value::as_str) == Some(symbol))
            .collect::<Vec<_>>();
        if matching.len() != 1
            || matching[0].get("state").and_then(Value::as_str) != Some("approved")
        {
            return Err(format!(
                "formal-rule symbol {symbol} is missing or not approved"
            ));
        }
        let source = assertion
            .get("source")
            .ok_or_else(|| format!("selected report claim {claim_id} has no source span"))?;
        claims.push(serde_json::json!({
            "claim_id": claim_id,
            "source": source,
            "symbol": symbol,
            "state": "approved"
        }));
    }
    Ok(serde_json::json!({"claims": claims}))
}

/// Build the `filament-analyze.claim-selection/v1` input for a formal-rule
/// judgement from an FR-188 status report. Draft and unrecognized states are
/// never emitted.
pub fn build_approved_claim_selection(status: &Value) -> Result<Value, String> {
    if status.get("protocol").and_then(Value::as_str) != Some("quoin.formal-rule-status/v1") {
        return Err("formal-rule status report has an unsupported protocol".to_owned());
    }
    let rules = status
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| "formal-rule status report has no rules list".to_owned())?;
    let mut spans = Vec::new();
    for rule in rules {
        let state = rule
            .get("state")
            .and_then(Value::as_str)
            .ok_or_else(|| "formal-rule status entry has no state".to_owned())?;
        if state == "draft" {
            continue;
        }
        if state != "approved" {
            return Err(format!(
                "formal-rule status contains unknown state {state:?}"
            ));
        }
        let identity = rule
            .get("path")
            .and_then(Value::as_str)
            .filter(|path| {
                !path.is_empty()
                    && !path.starts_with('/')
                    && !path
                        .split('/')
                        .any(|part| part.is_empty() || part == "." || part == "..")
                    && !path.contains('\\')
                    && !path.chars().any(char::is_control)
            })
            .ok_or_else(|| "approved formal-rule status entry has an invalid path".to_owned())?;
        let range = rule
            .get("declaration_range")
            .and_then(Value::as_object)
            .filter(|range| range.len() == 2)
            .ok_or_else(|| {
                "approved formal-rule status entry has an invalid declaration range".to_owned()
            })?;
        let start = range
            .get("start")
            .and_then(Value::as_u64)
            .ok_or_else(|| "approved formal-rule declaration start is invalid".to_owned())?;
        let end = range
            .get("end")
            .and_then(Value::as_u64)
            .filter(|end| start < *end)
            .ok_or_else(|| "approved formal-rule declaration end is invalid".to_owned())?;
        spans.push((identity, start, end));
    }
    spans.sort_by(|left, right| {
        left.0
            .as_bytes()
            .cmp(right.0.as_bytes())
            .then(left.1.cmp(&right.1))
    });
    let mut seen_spans = BTreeSet::new();
    if spans
        .iter()
        .any(|(identity, start, _)| !seen_spans.insert((identity.to_owned(), *start)))
    {
        return Err("formal-rule status contains duplicate approved declaration spans".to_owned());
    }
    let spans = spans
        .into_iter()
        .map(|(identity, start, end)| {
            serde_json::json!({
                "authority": "quoin",
                "identity": identity,
                "start": start,
                "end": end
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "protocol": "filament-analyze.claim-selection/v1",
        "spans": spans
    }))
}

fn parse_differential_report_inner(
    raw: &str,
    run_context: Option<(&str, Option<&Value>)>,
) -> Result<AdapterResult, EvidenceError> {
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

    let symbol = text(&report, "analysisKind")?.to_owned();
    let differential = report
        .get("differential")
        .ok_or_else(|| refuse("validated report lacks differential"))?;
    let agreed = differential.get("agreedStatus").and_then(Value::as_str);
    let projection = analysis_projection(&report, differential, run_context)?;
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
fn analysis_projection(
    report: &Value,
    differential: &Value,
    run_context: Option<(&str, Option<&Value>)>,
) -> Result<Value, EvidenceError> {
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
    let mut projection = serde_json::json!({
        "protocol": "quoin.analysis-report-projection/v1",
        "disposition": differential.get("disposition").cloned().unwrap_or(Value::Null),
        "agreed_status": agreed_status,
        "producer": {
            "name": producer.get("name").cloned().unwrap_or(Value::Null),
            "version": producer.get("version").cloned().unwrap_or(Value::Null),
        },
        "engines": engines,
    });
    if let Some(selection) = report.get("selection") {
        projection["selection"] = selection.clone();
    }
    if let Some((run_kind, formal_rule_selection)) = run_context {
        if run_kind == "formal_rule" {
            validate_formal_rule_selection(report, formal_rule_selection)?;
            if let Some(formal_rule_selection) = formal_rule_selection {
                projection["formal_rule_selection"] = formal_rule_selection.clone();
            }
        }
        projection["run_kind"] = Value::String(run_kind.to_owned());
    }
    Ok(projection)
}

fn validate_formal_rule_selection(
    report: &Value,
    mapping: Option<&Value>,
) -> Result<(), EvidenceError> {
    let Some(mapping) = mapping else {
        return Err(refuse("formal_rule_selection is missing"));
    };
    let Some(mapping_object) = mapping.as_object().filter(|object| object.len() == 1) else {
        return Err(refuse("formal_rule_selection must contain only claims"));
    };
    let Some(claims) = mapping_object.get("claims").and_then(Value::as_array) else {
        return Err(refuse("formal_rule_selection.claims must be an array"));
    };
    let Some(selected) = report
        .get("selection")
        .and_then(|selection| selection.get("selected"))
        .and_then(Value::as_array)
    else {
        return Err(refuse("formal-rule report has no selection.selected array"));
    };
    if selected.len() != claims.len() {
        return Err(refuse(
            "formal_rule_selection.claims must map every selected claim exactly once",
        ));
    }
    let assertions = report
        .get("assertions")
        .and_then(Value::as_array)
        .ok_or_else(|| refuse("validated report lacks assertions"))?;
    for (selected_id, claim) in selected.iter().zip(claims) {
        let Some(selected_id) = selected_id.as_str() else {
            return Err(refuse("selection.selected contains a non-string claim id"));
        };
        let Some(object) = claim.as_object().filter(|object| object.len() == 4) else {
            return Err(refuse(
                "formal_rule_selection claim must contain exactly claim_id, source, symbol and state",
            ));
        };
        let Some(claim_id) = object.get("claim_id").and_then(Value::as_str) else {
            return Err(refuse("formal_rule_selection.claim_id must be a string"));
        };
        if claim_id != selected_id {
            return Err(refuse(
                "formal_rule_selection.claim_id values differ from selection.selected",
            ));
        }
        let Some(symbol) = object.get("symbol").and_then(Value::as_str) else {
            return Err(refuse("formal_rule_selection.symbol must be a string"));
        };
        if symbol.is_empty() || !symbol.starts_with("qsl:") {
            return Err(refuse(
                "formal_rule_selection.symbol must be a QSL symbol id",
            ));
        }
        if !matches!(
            object.get("state").and_then(Value::as_str),
            Some("approved" | "draft")
        ) {
            return Err(refuse(
                "formal_rule_selection.state must be approved or draft",
            ));
        }
        let mut matching = assertions.iter().filter(|assertion| {
            assertion
                .get("clause")
                .and_then(Value::as_str)
                .is_some_and(|clause| clause.ends_with(&format!(":{claim_id}")))
        });
        let Some(assertion) = matching.next() else {
            return Err(refuse(format!(
                "formal_rule_selection has no report source for {claim_id}"
            )));
        };
        if matching.next().is_some()
            || assertion.get("source") != object.get("source")
            || !valid_source(object.get("source"))
        {
            return Err(refuse(format!(
                "formal_rule_selection source does not match report claim {claim_id}"
            )));
        }
    }
    Ok(())
}

fn valid_source(source: Option<&Value>) -> bool {
    let Some(source) = source.and_then(Value::as_object) else {
        return false;
    };
    if source.len() != 4
        || source.get("document").and_then(Value::as_str).is_none()
        || source.get("revision").and_then(Value::as_u64).is_none()
    {
        return false;
    }
    ["start", "end"].iter().all(|point| {
        let Some(point) = source.get(*point).and_then(Value::as_object) else {
            return false;
        };
        point.len() == 3
            && ["byte", "column", "line"]
                .iter()
                .all(|member| point.get(*member).and_then(Value::as_u64).is_some())
    })
}
