// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Append-only experiment and operational records (FR-048).
//!
//! # Identity is an opaque UUID, and the UUID is the file name
//!
//! A record's id is an opaque UUID that is not derived from its content
//! (FR-098-AC-10), and its path is `<recordId>.json`. A new record is given a
//! fresh UUID; a record published again with the `recordId` it was given is
//! idempotent when the bytes are the same and a named refusal when they are
//! not, and nothing is ever overwritten. Identifiers written before this
//! change (`sha256:<hex>`, file `sha256-<hex>.json`) are read as opaque strings
//! and left unchanged. Tampering is visible in git history; nothing here
//! recomputes an identity from content.
//!
//! # Validation is hand-written, and that is the point
//!
//! These records arrive from producers outside quoin. Every field is checked
//! against an exact key set — unknown keys refused, missing keys named — so a
//! producer that renames a field is told, rather than having the value silently
//! read as absent.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::error::EvidenceError;
use crate::instant::{Instant, InstantGrammar};
use crate::paths::{EXPERIMENTS_DIR, OPERATIONAL_EVIDENCE_DIR, assurance_record_path};
use crate::source::EvidenceSource;
use crate::store::codec::{canonical_bytes_of, decode};
use crate::types::{
    ExperimentRecord, ExperimentRecordInput, OperationalEvidenceRecord,
    OperationalEvidenceRecordInput, StoredAssuranceRecord,
};

/// The language every timestamp in an assurance record is written in.
///
/// The retained check was an anchored regular expression with a mandatory
/// zone, so a bare date is refused and nothing may follow the offset. See
/// [`crate::instant`] for why the two grammars in this crate are not one.
const RECORDED_AT_GRAMMAR: InstantGrammar = InstantGrammar::ZonedInstant;

/// Publish one experiment record, append-only.
///
/// `input` may carry a `recordId` assigned by an earlier publish, which makes
/// publishing it again idempotent; without one a fresh UUID is minted.
///
/// # Errors
///
/// [`EvidenceError::InvalidRecord`] when the input does not validate,
/// [`EvidenceError::RecordIntegrity`] when the path already holds different
/// bytes, otherwise [`EvidenceError::Canonicalization`] or
/// [`EvidenceError::StoreIo`].
pub fn write_experiment_record<S: EvidenceSource + ?Sized>(
    source: &mut S,
    input: &Value,
) -> Result<StoredAssuranceRecord<ExperimentRecord>, EvidenceError> {
    let (assigned, input) = split_record_id(input)?;
    let parsed: ExperimentRecordInput = validated(&input, "experiment record", experiment_shape)?;
    publish(
        source,
        EXPERIMENTS_DIR,
        "experiment record",
        assigned,
        parsed,
        |id, input| ExperimentRecord {
            record_id: id,
            input,
        },
    )
}

/// Publish one operational evidence record, append-only.
///
/// # Errors
///
/// As [`write_experiment_record`].
pub fn write_operational_evidence_record<S: EvidenceSource + ?Sized>(
    source: &mut S,
    input: &Value,
) -> Result<StoredAssuranceRecord<OperationalEvidenceRecord>, EvidenceError> {
    let (assigned, input) = split_record_id(input)?;
    let parsed: OperationalEvidenceRecordInput =
        validated(&input, "operational evidence record", operational_shape)?;
    publish(
        source,
        OPERATIONAL_EVIDENCE_DIR,
        "operational evidence record",
        assigned,
        parsed,
        |id, input| OperationalEvidenceRecord {
            record_id: id,
            input,
        },
    )
}

/// Read one experiment record back, checking it against its own identity.
///
/// # Errors
///
/// [`EvidenceError::RecordIntegrity`] when the stored id disagrees with the
/// path it was read from.
pub fn read_experiment_record<S: EvidenceSource + ?Sized>(
    source: &S,
    record_id: &str,
) -> Result<Option<ExperimentRecord>, EvidenceError> {
    read_checked(
        source,
        EXPERIMENTS_DIR,
        record_id,
        |record: &ExperimentRecord| &record.record_id,
    )
}

/// Read one operational evidence record back, checking its identity.
///
/// # Errors
///
/// As [`read_experiment_record`].
pub fn read_operational_evidence_record<S: EvidenceSource + ?Sized>(
    source: &S,
    record_id: &str,
) -> Result<Option<OperationalEvidenceRecord>, EvidenceError> {
    read_checked(
        source,
        OPERATIONAL_EVIDENCE_DIR,
        record_id,
        |record: &OperationalEvidenceRecord| &record.record_id,
    )
}

/// Every published experiment record id, sorted.
///
/// # Errors
///
/// [`EvidenceError::StoreIo`] when the directory exists and cannot be listed.
pub fn list_experiment_records<S: EvidenceSource + ?Sized>(
    source: &S,
) -> Result<Vec<String>, EvidenceError> {
    list_records(source, EXPERIMENTS_DIR)
}

/// Every published operational evidence record id, sorted.
///
/// # Errors
///
/// As [`list_experiment_records`].
pub fn list_operational_evidence_records<S: EvidenceSource + ?Sized>(
    source: &S,
) -> Result<Vec<String>, EvidenceError> {
    list_records(source, OPERATIONAL_EVIDENCE_DIR)
}

/// Whether `value` is a record id this store addresses: a lowercase
/// hyphenated UUID, or a legacy `sha256:<64 lowercase hex>` identifier, which
/// is an opaque string here.
#[must_use]
pub fn is_record_id(value: &str) -> bool {
    is_uuid(value) || legacy_hex(value).is_some()
}

fn is_lower_hex(text: &str) -> bool {
    text.bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The hex of a legacy `sha256:<64 lowercase hex>` identifier.
fn legacy_hex(value: &str) -> Option<&str> {
    value
        .strip_prefix("sha256:")
        .filter(|hex| hex.len() == 64 && is_lower_hex(hex))
}

/// A lowercase hyphenated UUID: 8-4-4-4-12 hexadecimal characters.
fn is_uuid(value: &str) -> bool {
    let groups: Vec<&str> = value.split('-').collect();
    groups.len() == 5
        && groups
            .iter()
            .zip([8_usize, 4, 4, 4, 12])
            .all(|(group, length)| group.len() == length && is_lower_hex(group))
}

/// The file stem a record id is stored under: the id itself for a UUID, and
/// the id without its `sha256:` prefix, behind `sha256-`, for a legacy one.
///
/// # Errors
///
/// [`EvidenceError::InvalidRecord`] when `record_id` is neither shape.
pub fn record_file_stem(record_id: &str) -> Result<String, EvidenceError> {
    if is_uuid(record_id) {
        return Ok(record_id.to_owned());
    }
    legacy_hex(record_id)
        .map(|hex| format!("sha256-{hex}"))
        .ok_or_else(|| {
            EvidenceError::invalid(
                "assurance record",
                "recordId: must be a UUID or a legacy sha256:<64 lowercase hex> identifier",
            )
        })
}

fn record_path(family: &str, record_id: &str) -> Result<String, EvidenceError> {
    record_file_stem(record_id).map(|stem| assurance_record_path(family, &stem))
}

/// The record id an input carries, when it carries one, and the input without
/// it. The shape check below then sees only the producer's own members.
fn split_record_id(input: &Value) -> Result<(Option<String>, Value), EvidenceError> {
    let Some(object) = input.as_object() else {
        return Ok((None, input.clone()));
    };
    let mut rest = object.clone();
    let Some(assigned) = rest.remove("recordId") else {
        return Ok((None, input.clone()));
    };
    match assigned.as_str() {
        Some(text) if is_record_id(text) => Ok((Some(text.to_owned()), Value::Object(rest))),
        _ => Err(EvidenceError::invalid(
            "assurance record",
            "recordId: must be a UUID or a legacy sha256:<64 lowercase hex> identifier",
        )),
    }
}

fn publish<S, I, R, F>(
    source: &mut S,
    family: &str,
    label: &'static str,
    assigned: Option<String>,
    input: I,
    build: F,
) -> Result<StoredAssuranceRecord<R>, EvidenceError>
where
    S: EvidenceSource + ?Sized,
    R: Serialize,
    F: FnOnce(String, I) -> R,
{
    let identity = assigned.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let record = build(identity.clone(), input);
    let path = record_path(family, &identity)?;
    let bytes = canonical_bytes_of(label, &record)?;
    let created = source.publish(&path, &bytes)?;
    Ok(StoredAssuranceRecord {
        record,
        path,
        created,
    })
}

fn read_checked<S, R, F>(
    source: &S,
    family: &str,
    record_id: &str,
    stored: F,
) -> Result<Option<R>, EvidenceError>
where
    S: EvidenceSource + ?Sized,
    R: DeserializeOwned,
    F: Fn(&R) -> &String,
{
    let path = record_path(family, record_id)?;
    let Some(text) = source.read(&path)? else {
        return Ok(None);
    };
    let record: R = decode(&path, &text)?;
    if stored(&record) != record_id {
        return Err(EvidenceError::RecordIntegrity {
            what: "record id does not match path",
            path,
            note: "",
        });
    }
    Ok(Some(record))
}

fn list_records<S: EvidenceSource + ?Sized>(
    source: &S,
    family: &str,
) -> Result<Vec<String>, EvidenceError> {
    let mut ids = Vec::new();
    for name in source.list_files(family)? {
        let Some(stem) = name.strip_suffix(".json") else {
            continue;
        };
        if is_uuid(stem) {
            ids.push(stem.to_owned());
        } else if let Some(hex) = stem.strip_prefix("sha256-")
            && hex.len() == 64
            && is_lower_hex(hex)
        {
            ids.push(format!("sha256:{hex}"));
        }
    }
    ids.sort();
    Ok(ids)
}

/// Validate a raw document against a shape, then deserialize it.
fn validated<T: DeserializeOwned>(
    value: &Value,
    label: &'static str,
    shape: fn(&Value, &mut Vec<String>),
) -> Result<T, EvidenceError> {
    let mut clauses = Vec::new();
    shape(value, &mut clauses);
    if !clauses.is_empty() {
        return Err(EvidenceError::invalid(label, clauses.join("; ")));
    }
    serde_json::from_value(value.clone())
        .map_err(|error| EvidenceError::invalid(label, error.to_string()))
}

fn experiment_shape(value: &Value, clauses: &mut Vec<String>) {
    let Some(root) = object_at("experiment record", value, clauses) else {
        return;
    };
    exact(
        root,
        "experiment record",
        &[
            "schemaVersion",
            "subject",
            "recordedAt",
            "hypothesis",
            "design",
            "result",
            "producerProvenance",
        ],
        &[],
        clauses,
    );
    literal(
        root.get("schemaVersion"),
        "schemaVersion",
        &["experiment-record-v1"],
        clauses,
    );
    subject_shape(root.get("subject"), clauses);
    instant("recordedAt", root.get("recordedAt"), clauses);
    string_value("hypothesis", root.get("hypothesis"), clauses);
    if let Some(design) = object_at(
        "design",
        root.get("design").unwrap_or(&Value::Null),
        clauses,
    ) {
        exact(
            design,
            "design",
            &["timeBox", "corpusRefs", "comparisonMethod", "decisionRule"],
            &[],
            clauses,
        );
        string_value("timeBox", design.get("timeBox"), clauses);
        strings("corpusRefs", design.get("corpusRefs"), true, clauses);
        string_value("comparisonMethod", design.get("comparisonMethod"), clauses);
        string_value("decisionRule", design.get("decisionRule"), clauses);
    }
    if let Some(result) = object_at(
        "result",
        root.get("result").unwrap_or(&Value::Null),
        clauses,
    ) {
        exact(
            result,
            "result",
            &["status", "summary", "evidenceRefs"],
            &[],
            clauses,
        );
        literal(
            result.get("status"),
            "result.status",
            &["supported", "not_supported", "inconclusive"],
            clauses,
        );
        string_value("result.summary", result.get("summary"), clauses);
        strings(
            "result.evidenceRefs",
            result.get("evidenceRefs"),
            true,
            clauses,
        );
    }
    provenance_shape(root.get("producerProvenance"), clauses);
}

fn operational_shape(value: &Value, clauses: &mut Vec<String>) {
    let Some(root) = object_at("operational evidence record", value, clauses) else {
        return;
    };
    exact(
        root,
        "operational evidence record",
        &[
            "schemaVersion",
            "subject",
            "recordedAt",
            "window",
            "environment",
            "observations",
            "outcome",
            "producerProvenance",
        ],
        &[],
        clauses,
    );
    literal(
        root.get("schemaVersion"),
        "schemaVersion",
        &["operational-evidence-record-v1"],
        clauses,
    );
    subject_shape(root.get("subject"), clauses);
    instant("recordedAt", root.get("recordedAt"), clauses);
    if let Some(window) = object_at(
        "window",
        root.get("window").unwrap_or(&Value::Null),
        clauses,
    ) {
        exact(window, "window", &["startedAt", "endedAt"], &[], clauses);
        let started = instant("window.startedAt", window.get("startedAt"), clauses);
        let ended = instant("window.endedAt", window.get("endedAt"), clauses);
        // Compared as text: both are validated ISO-8601 instants, and for the
        // fixed-width form that check admits, lexical order is chronological.
        if let (Some(started), Some(ended)) = (started, ended)
            && ended <= started
        {
            clauses.push("window.endedAt: must be after window.startedAt".to_owned());
        }
    }
    string_value("environment", root.get("environment"), clauses);
    match root.get("observations") {
        Some(Value::Array(items)) if !items.is_empty() => {
            for (index, item) in items.iter().enumerate() {
                let label = format!("observations[{index}]");
                let Some(observation) = object_at(&label, item, clauses) else {
                    continue;
                };
                exact(
                    observation,
                    &label,
                    &["signal", "value", "unit", "interpretation", "evidenceRefs"],
                    &["unit"],
                    clauses,
                );
                string_value("signal", observation.get("signal"), clauses);
                string_value("value", observation.get("value"), clauses);
                if observation.get("unit").is_some() {
                    string_value("unit", observation.get("unit"), clauses);
                }
                string_value("interpretation", observation.get("interpretation"), clauses);
                strings(
                    "evidenceRefs",
                    observation.get("evidenceRefs"),
                    true,
                    clauses,
                );
            }
        }
        _ => clauses.push("observations: must be a non-empty array".to_owned()),
    }
    literal(
        root.get("outcome"),
        "outcome",
        &["within_bounds", "outside_bounds", "inconclusive"],
        clauses,
    );
    provenance_shape(root.get("producerProvenance"), clauses);
}

fn subject_shape(value: Option<&Value>, clauses: &mut Vec<String>) {
    let Some(subject) = object_at("subject", value.unwrap_or(&Value::Null), clauses) else {
        return;
    };
    exact(
        subject,
        "subject",
        &["kind", "id", "sourceRevision"],
        &[],
        clauses,
    );
    string_value("subject.kind", subject.get("kind"), clauses);
    string_value("subject.id", subject.get("id"), clauses);
    string_value(
        "subject.sourceRevision",
        subject.get("sourceRevision"),
        clauses,
    );
}

fn provenance_shape(value: Option<&Value>, clauses: &mut Vec<String>) {
    let Some(root) = object_at("producerProvenance", value.unwrap_or(&Value::Null), clauses) else {
        return;
    };
    exact(
        root,
        "producerProvenance",
        &[
            "schemaVersion",
            "identity",
            "version",
            "sourceRevision",
            "sourceState",
            "executableDigest",
            "configurationDigest",
            "capabilities",
            "artifacts",
        ],
        &[],
        clauses,
    );
    literal(
        root.get("schemaVersion"),
        "producerProvenance.schemaVersion",
        &["producer-provenance-v1"],
        clauses,
    );
    string_value("producerProvenance.identity", root.get("identity"), clauses);
    string_value("producerProvenance.version", root.get("version"), clauses);
    string_value(
        "producerProvenance.sourceRevision",
        root.get("sourceRevision"),
        clauses,
    );
    literal(
        root.get("sourceState"),
        "producerProvenance.sourceState",
        &["clean", "dirty"],
        clauses,
    );
    sha256_value(
        "producerProvenance.executableDigest",
        root.get("executableDigest"),
        clauses,
    );
    sha256_value(
        "producerProvenance.configurationDigest",
        root.get("configurationDigest"),
        clauses,
    );
    strings(
        "producerProvenance.capabilities",
        root.get("capabilities"),
        false,
        clauses,
    );
    match root.get("artifacts") {
        Some(Value::Array(items)) => {
            for (index, item) in items.iter().enumerate() {
                let label = format!("artifacts[{index}]");
                let Some(artifact) = object_at(&label, item, clauses) else {
                    continue;
                };
                exact(artifact, &label, &["name", "digest"], &[], clauses);
                string_value("artifact.name", artifact.get("name"), clauses);
                sha256_value("artifact.digest", artifact.get("digest"), clauses);
            }
        }
        _ => clauses.push("producerProvenance.artifacts: must be an array".to_owned()),
    }
}

fn object_at<'a>(
    name: &str,
    value: &'a Value,
    clauses: &mut Vec<String>,
) -> Option<&'a Map<String, Value>> {
    if let Some(object) = value.as_object() {
        return Some(object);
    }
    clauses.push(format!("{name}: must be an object"));
    None
}

fn exact(
    value: &Map<String, Value>,
    name: &str,
    allowed: &[&str],
    optional: &[&str],
    clauses: &mut Vec<String>,
) {
    for key in value.keys() {
        if !allowed.contains(&key.as_str()) {
            clauses.push(format!("{name}: has unknown field {key}"));
        }
    }
    for key in allowed {
        if !optional.contains(key) && !value.contains_key(*key) {
            clauses.push(format!("{name}: is missing {key}"));
        }
    }
}

fn string_value(name: &str, value: Option<&Value>, clauses: &mut Vec<String>) -> Option<String> {
    match value.and_then(Value::as_str) {
        Some(text) if !text.trim().is_empty() => Some(text.to_owned()),
        _ => {
            clauses.push(format!("{name}: must be a non-empty string"));
            None
        }
    }
}

fn strings(name: &str, value: Option<&Value>, non_empty: bool, clauses: &mut Vec<String>) {
    let Some(Value::Array(items)) = value else {
        clauses.push(format!(
            "{name}: must be {} array",
            if non_empty { "a non-empty" } else { "an" }
        ));
        return;
    };
    if non_empty && items.is_empty() {
        clauses.push(format!("{name}: must be a non-empty array"));
        return;
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut duplicated = false;
    for item in items {
        if let Some(text) = string_value(name, Some(item), clauses)
            && !seen.insert(text)
        {
            duplicated = true;
        }
    }
    if duplicated {
        clauses.push(format!("{name}: must contain unique values"));
    }
}

fn literal(value: Option<&Value>, name: &str, allowed: &[&str], clauses: &mut Vec<String>) {
    if !value
        .and_then(Value::as_str)
        .is_some_and(|text| allowed.contains(&text))
    {
        clauses.push(format!("{name}: must be one of {}", allowed.join(", ")));
    }
}

/// ISO-8601 with a mandatory zone, as the retained regex spells it.
///
/// Returns the text when it reads, so the caller stores what was validated
/// rather than re-reaching for the raw value. The grammar is named because the
/// crate has two and they are not interchangeable; see [`crate::instant`].
fn instant(name: &str, value: Option<&Value>, clauses: &mut Vec<String>) -> Option<String> {
    let text = value.and_then(Value::as_str).unwrap_or("");
    let Some(read) = Instant::parse(text, RECORDED_AT_GRAMMAR) else {
        clauses.push(format!("{name}: {}", RECORDED_AT_GRAMMAR.expectation()));
        return None;
    };
    Some(read.into())
}

// BLOCKED-BY QSL-1275 (owner: planner titan:QSL:plan): the producer-provenance
// `executableDigest`, `configurationDigest` and artifact `digest` members are a
// digest-valued data contract outside record identity; removing them changes
// the record schema and its producers.
fn sha256_value(name: &str, value: Option<&Value>, clauses: &mut Vec<String>) {
    if !value
        .and_then(Value::as_str)
        .is_some_and(|text| legacy_hex(text).is_some())
    {
        clauses.push(format!("{name}: must be sha256:<64 lowercase hex>"));
    }
}
