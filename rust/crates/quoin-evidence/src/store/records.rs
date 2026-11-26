// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Reading and writing the three per-suite record families.
//!
//! Runs, scans and mock inspections are laid out identically — one directory
//! per suite, one file per record, named by its timestamp — so they share the generic readers below and
//! differ only in their family directory and their record type. The retained
//! `store.ts` writes the three sets out three times; a port that copied that
//! would have three places for the ordering defect agent-ix/quoin#104 fixed to
//! come back to.

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::EvidenceError;
use crate::ids::SuiteId;
use crate::paths::{
    ANALYSIS_REPORTS_DIR, MOCK_INSPECTIONS_DIR, RUNS_DIR, SCANS_DIR, record_path, suite_dir,
};
use crate::source::EvidenceSource;
use crate::store::codec::{canonical_bytes_of, decode};
use crate::timestamp::{TimestampKey, parse_timestamp};
use crate::types::{FindingRecord, MockInspectionRecord, RunRecord, STORE_SCHEMA_VERSION};

/// What the store orders records by: the caller's timestamp, then the file name.
///
/// The timestamp is read from the record, not from the file name, which is only
/// a sanitised spelling of it.
pub(crate) trait StoredRecord {
    /// The family directory this record lives under.
    const FAMILY: &'static str;
    /// A name for the record class, used in a canonicalization refusal.
    const LABEL: &'static str;

    /// The suite the record covers.
    fn suite(&self) -> &SuiteId;
    /// The caller-supplied ISO-8601 stamp.
    fn timestamp(&self) -> &str;

    /// Refuse malformed fields before a key or bytes are published.
    fn validate(&self) -> Result<(), EvidenceError>;
}

/// Validate a run value at a reader boundary that must retain its source bytes.
///
/// The analysis qualifier reads exact bytes for provenance, so it cannot route
/// through the generic store decoder. It uses this same semantic intake check
/// before it admits the typed record.
///
/// # Errors
///
/// Returns an error when any run-record field violates the retained-record
/// validation rules.
pub fn validate_run_record(record: &RunRecord) -> Result<(), EvidenceError> {
    record.validate()
}

impl StoredRecord for RunRecord {
    const FAMILY: &'static str = RUNS_DIR;
    const LABEL: &'static str = "run record";

    fn suite(&self) -> &SuiteId {
        &self.suite
    }
    fn timestamp(&self) -> &str {
        &self.timestamp
    }

    fn validate(&self) -> Result<(), EvidenceError> {
        validate_envelope(
            Self::LABEL,
            self.schema_version,
            &self.suite,
            &self.tool,
            &self.timestamp,
        )?;
        optional_text(Self::LABEL, "evidenceKind", self.evidence_kind.as_deref())?;
        for (index, entry) in self.entries.iter().enumerate() {
            identity(
                Self::LABEL,
                &format!("entries[{index}].symbol"),
                entry.symbol.as_str(),
            )?;
            if let Some(ids) = &entry.trace_ids {
                for (id_index, id) in ids.iter().enumerate() {
                    identity(
                        Self::LABEL,
                        &format!("entries[{index}].traceIds[{id_index}]"),
                        id,
                    )?;
                }
            }
            if entry.score.is_some_and(|score| !score.is_finite()) {
                return Err(invalid(
                    Self::LABEL,
                    format!("entries[{index}].score: must be finite"),
                ));
            }
            optional_text(
                Self::LABEL,
                &format!("entries[{index}].metric"),
                entry.metric.as_deref(),
            )?;
            if let Some(config) = &entry.config {
                for (key, value) in config {
                    identity(Self::LABEL, &format!("entries[{index}].config key"), key)?;
                    text(
                        Self::LABEL,
                        &format!("entries[{index}].config[{key}]"),
                        value,
                    )?;
                }
            }
            if let Some(survivors) = &entry.survivors {
                for (survivor_index, survivor) in survivors.iter().enumerate() {
                    if survivor.line == Some(0) {
                        return Err(invalid(
                            Self::LABEL,
                            format!(
                                "entries[{index}].survivors[{survivor_index}].line: must be greater than zero"
                            ),
                        ));
                    }
                    text(
                        Self::LABEL,
                        &format!("entries[{index}].survivors[{survivor_index}].mutation"),
                        &survivor.mutation,
                    )?;
                }
            }
        }
        Ok(())
    }
}

impl StoredRecord for FindingRecord {
    const FAMILY: &'static str = SCANS_DIR;
    const LABEL: &'static str = "scan record";

    fn suite(&self) -> &SuiteId {
        &self.suite
    }
    fn timestamp(&self) -> &str {
        &self.timestamp
    }

    fn validate(&self) -> Result<(), EvidenceError> {
        validate_envelope(
            Self::LABEL,
            self.schema_version,
            &self.suite,
            &self.tool,
            &self.timestamp,
        )?;
        optional_text(Self::LABEL, "evidenceKind", self.evidence_kind.as_deref())?;
        optional_text(Self::LABEL, "ruleset", self.ruleset.as_deref())?;
        for (index, finding) in self.findings.iter().enumerate() {
            identity(
                Self::LABEL,
                &format!("findings[{index}].ruleId"),
                &finding.rule_id,
            )?;
            optional_text(
                Self::LABEL,
                &format!("findings[{index}].severity"),
                finding.severity.as_deref(),
            )?;
            optional_text(
                Self::LABEL,
                &format!("findings[{index}].message"),
                finding.message.as_deref(),
            )?;
            optional_text(
                Self::LABEL,
                &format!("findings[{index}].path"),
                finding.path.as_deref(),
            )?;
            if let Some(ids) = &finding.trace_ids {
                for (id_index, id) in ids.iter().enumerate() {
                    identity(
                        Self::LABEL,
                        &format!("findings[{index}].traceIds[{id_index}]"),
                        id,
                    )?;
                }
            }
            if finding.line == Some(0) {
                return Err(invalid(
                    Self::LABEL,
                    format!("findings[{index}].line: must be greater than zero"),
                ));
            }
        }
        if let Some(evaluated) = &self.evaluated {
            for (index, symbol) in evaluated.iter().enumerate() {
                identity(Self::LABEL, &format!("evaluated[{index}]"), symbol)?;
            }
        }
        Ok(())
    }
}

impl StoredRecord for MockInspectionRecord {
    const FAMILY: &'static str = MOCK_INSPECTIONS_DIR;
    const LABEL: &'static str = "mock inspection record";

    fn suite(&self) -> &SuiteId {
        &self.suite
    }
    fn timestamp(&self) -> &str {
        &self.timestamp
    }

    fn validate(&self) -> Result<(), EvidenceError> {
        validate_envelope(
            Self::LABEL,
            self.schema_version,
            &self.suite,
            &self.tool,
            &self.timestamp,
        )?;
        for (index, injection) in self.injections.iter().enumerate() {
            suite_id(
                Self::LABEL,
                &format!("injections[{index}].suite"),
                &injection.suite,
            )?;
            if injection.suite != self.suite {
                return Err(invalid(
                    Self::LABEL,
                    format!("injections[{index}].suite: must match the record suite"),
                ));
            }
            identity(
                Self::LABEL,
                &format!("injections[{index}].symbol"),
                injection.symbol.as_str(),
            )?;
            if injection.injects.is_empty() {
                return Err(invalid(
                    Self::LABEL,
                    format!("injections[{index}].injects: must contain an identifier"),
                ));
            }
            for (inject_index, injected) in injection.injects.iter().enumerate() {
                identity(
                    Self::LABEL,
                    &format!("injections[{index}].injects[{inject_index}]"),
                    injected,
                )?;
            }
            if injection.line == Some(0) {
                return Err(invalid(
                    Self::LABEL,
                    format!("injections[{index}].line: must be greater than zero"),
                ));
            }
            if let Some(path) = injection.path.as_deref() {
                text(Self::LABEL, &format!("injections[{index}].path"), path)?;
                if path.starts_with('/')
                    || path.contains('\\')
                    || path.split('/').any(|segment| segment == "..")
                {
                    return Err(invalid(
                        Self::LABEL,
                        format!("injections[{index}].path: must be a repository-relative path"),
                    ));
                }
            }
        }
        Ok(())
    }
}

fn invalid(label: &'static str, detail: impl Into<String>) -> EvidenceError {
    EvidenceError::invalid(label, detail)
}

fn text(label: &'static str, field: &str, value: &str) -> Result<(), EvidenceError> {
    if value.trim().is_empty() || value != value.trim() || value.chars().any(char::is_control) {
        return Err(invalid(
            label,
            format!("{field}: must be non-empty, trimmed text without control characters"),
        ));
    }
    Ok(())
}

fn optional_text(
    label: &'static str,
    field: &str,
    value: Option<&str>,
) -> Result<(), EvidenceError> {
    if let Some(value) = value {
        text(label, field, value)?;
    }
    Ok(())
}

fn identity(label: &'static str, field: &str, value: &str) -> Result<(), EvidenceError> {
    text(label, field, value)
}

fn suite_id(label: &'static str, field: &str, value: &SuiteId) -> Result<(), EvidenceError> {
    let value = value.as_str();
    identity(label, field, value)?;
    if value == "." || value == ".." || value.contains('/') || value.contains('\\') {
        return Err(invalid(
            label,
            format!("{field}: must be one safe path segment"),
        ));
    }
    Ok(())
}

fn validate_envelope(
    label: &'static str,
    schema_version: u32,
    suite: &SuiteId,
    tool: &str,
    timestamp: &str,
) -> Result<TimestampKey, EvidenceError> {
    if schema_version != STORE_SCHEMA_VERSION {
        return Err(invalid(
            label,
            format!("schemaVersion: expected {STORE_SCHEMA_VERSION}"),
        ));
    }
    suite_id(label, "suite", suite)?;
    text(label, "tool", tool)?;
    parse_timestamp(timestamp).ok_or_else(|| {
        invalid(
            label,
            "timestamp: expected canonical UTC YYYY-MM-DDTHH:MM:SS[.mmm]Z".to_owned(),
        )
    })
}

/// Where one record belongs, from the record itself.
fn path_of<T: StoredRecord>(record: &T) -> String {
    record_path(T::FAMILY, record.suite(), record.timestamp())
}

/// Write one record at its `(suite, timestamp)` key.
///
/// Append-only: replaying the identical record is a no-op, and a different
/// record at an occupied key is refused with the original untouched. Replacing
/// it would silently turn a recorded failure into a pass (PLAT-1791,
/// FR-030-AC-3, 21 CFR 11.10(e)).
///
/// The stored `schemaVersion` is always [`STORE_SCHEMA_VERSION`] regardless of
/// what the caller put in the record, reproducing the retained
/// `{ ...record, schemaVersion: STORE_SCHEMA_VERSION }` spread.
///
/// # Errors
///
/// [`EvidenceError::Canonicalization`] when the record has no canonical
/// spelling, [`EvidenceError::RecordIntegrity`] when the key holds different
/// bytes, [`EvidenceError::StoreIo`] when the write cannot be completed.
pub(crate) fn write_record<S, T>(source: &mut S, record: &T) -> Result<String, EvidenceError>
where
    S: EvidenceSource + ?Sized,
    T: StoredRecord + Serialize + Clone + DeserializeOwned,
{
    record.validate()?;
    let path = path_of(record);
    refuse_timestamp_alias::<S, T>(source, record, &path)?;
    let mut value =
        serde_json::to_value(record).map_err(|error| EvidenceError::Canonicalization {
            what: T::LABEL,
            detail: error.to_string(),
        })?;
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "schemaVersion".to_owned(),
            serde_json::Value::from(STORE_SCHEMA_VERSION),
        );
    }
    let bytes = canonical_bytes_of(T::LABEL, &value)?;
    source.publish(&path, &bytes)?;
    Ok(path)
}

/// Atomically retain a differential-report run with its immutable projection.
/// Existing half-pairs and changed replays are refused without modifying either file.
pub(crate) fn write_analysis_pair<S: EvidenceSource + ?Sized>(
    source: &mut S,
    record: &RunRecord,
    projection: &serde_json::Value,
) -> Result<String, EvidenceError> {
    record.validate()?;
    let run_path = path_of(record);
    refuse_timestamp_alias::<S, RunRecord>(source, record, &run_path)?;
    let projection_path = record_path(ANALYSIS_REPORTS_DIR, &record.suite, &record.timestamp);
    let mut run_value =
        serde_json::to_value(record).map_err(|error| EvidenceError::Canonicalization {
            what: "run record",
            detail: error.to_string(),
        })?;
    if let Some(object) = run_value.as_object_mut() {
        object.insert(
            "schemaVersion".to_owned(),
            serde_json::Value::from(STORE_SCHEMA_VERSION),
        );
    }
    let run_bytes = canonical_bytes_of("run record", &run_value)?;
    let projection_bytes = canonical_bytes_of("analysis report projection", projection)?;
    let existing_run = source.read(&run_path)?;
    let existing_projection = source.read(&projection_path)?;
    match (existing_run.as_deref(), existing_projection.as_deref()) {
        (Some(run), Some(projected))
            if run.as_bytes() == run_bytes && projected.as_bytes() == projection_bytes =>
        {
            return Ok(run_path);
        }
        // The process died after the run was published and before the
        // projection was: the identical retry completes the pair (FR-030-AC-21).
        (Some(run), None) if run.as_bytes() == run_bytes => {
            return if source.publish(&projection_path, &projection_bytes)? {
                Ok(run_path)
            } else {
                Err(pair_conflict(&run_path))
            };
        }
        (Some(_) | None, Some(_)) | (Some(_), None) => {
            return Err(pair_conflict(&run_path));
        }
        (None, None) => {}
    }
    let published_run = source.publish(&run_path, &run_bytes)?;
    if !published_run {
        return Err(pair_conflict(&run_path));
    }
    match source.publish(&projection_path, &projection_bytes) {
        Ok(true) => Ok(run_path),
        Ok(false) => {
            source.remove(&run_path)?;
            Err(pair_conflict(&run_path))
        }
        Err(error) => {
            source.remove(&run_path)?;
            Err(error)
        }
    }
}

/// A second spelling for an instant would create a second path for one suite
/// event. Refuse it, and refuse to append when an existing record is missing or
/// malformed because its timestamp cannot be trusted for collision checks.
fn refuse_timestamp_alias<S, T>(
    source: &S,
    record: &T,
    expected_path: &str,
) -> Result<(), EvidenceError>
where
    S: EvidenceSource + ?Sized,
    T: StoredRecord + DeserializeOwned,
{
    let expected = parse_timestamp(record.timestamp())
        .ok_or_else(|| invalid(T::LABEL, "timestamp: invalid canonical UTC timestamp"))?;
    let directory = suite_dir(T::FAMILY, record.suite());
    for name in source.list_files(&directory)? {
        let path = format!("{directory}/{name}");
        let Some(text) = source.read(&path)? else {
            return Err(existing_record_problem(&path));
        };
        let existing = decode::<T>(&path, &text).map_err(|_| existing_record_problem(&path))?;
        existing
            .validate()
            .map_err(|_| existing_record_problem(&path))?;
        if existing.suite() != record.suite() || path_of(&existing) != path {
            return Err(existing_record_problem(&path));
        }
        let existing_key =
            parse_timestamp(existing.timestamp()).ok_or_else(|| existing_record_problem(&path))?;
        if path != expected_path && existing_key == expected {
            return Err(EvidenceError::RecordIntegrity {
                what: T::LABEL,
                path,
                note: "; another timestamp spelling already names this instant, and existing bytes were not changed",
            });
        }
    }
    Ok(())
}

fn existing_record_problem(path: &str) -> EvidenceError {
    EvidenceError::RecordIntegrity {
        what: "unreadable existing evidence record",
        path: path.to_owned(),
        note: "; no bytes were changed",
    }
}

fn pair_conflict(path: &str) -> EvidenceError {
    EvidenceError::RecordIntegrity {
        what: "analysis report pair",
        path: path.to_owned(),
        note: "; existing bytes were not overwritten",
    }
}

/// Every record file recorded for a suite, in **file-name** order.
///
/// File-name order is not time order; this exists for enumeration, and
/// [`latest`] is what a caller who means "the newest" wants.
///
/// # Errors
///
/// [`EvidenceError::StoreIo`] when the directory exists and cannot be listed.
pub(crate) fn list<S, T>(source: &S, suite: &SuiteId) -> Result<Vec<String>, EvidenceError>
where
    S: EvidenceSource + ?Sized,
    T: StoredRecord,
{
    source.list_files(&suite_dir(T::FAMILY, suite))
}

/// Every valid record recorded for a suite, oldest first by parsed UTC instant.
///
/// An unreadable file is skipped and its path pushed onto `skipped`: one
/// corrupt record must not hide every finding in the report
/// (agent-ix/quoin#106). An I/O failure is not skipped — the file is there and
/// the host refused it, which is not the same fact.
///
/// # Errors
///
/// [`EvidenceError::StoreIo`] when the directory or a present file cannot be
/// read.
pub(crate) fn read_all<S, T>(
    source: &S,
    suite: &SuiteId,
    skipped: &mut Vec<String>,
) -> Result<Vec<T>, EvidenceError>
where
    S: EvidenceSource + ?Sized,
    T: StoredRecord + DeserializeOwned,
{
    let directory = suite_dir(T::FAMILY, suite);
    let mut records: Vec<T> = Vec::new();
    for name in source.list_files(&directory)? {
        let path = format!("{directory}/{name}");
        let text = match source.read(&path) {
            Ok(Some(text)) => text,
            Ok(None) | Err(EvidenceError::StoreRead { .. }) => {
                skipped.push(path);
                continue;
            }
            Err(other) => return Err(other),
        };
        match decode::<T>(&path, &text) {
            Ok(record)
                if record.validate().is_ok()
                    && record.suite() == suite
                    && path_of(&record) == path =>
            {
                records.push(record);
            }
            Ok(_) | Err(EvidenceError::StoreRead { .. }) => skipped.push(path),
            Err(other) => return Err(other),
        }
    }
    records.sort_by(|a, b| {
        parse_timestamp(a.timestamp())
            .cmp(&parse_timestamp(b.timestamp()))
            .then_with(|| path_of(a).cmp(&path_of(b)))
    });
    Ok(records)
}

/// The newest record of a suite by parsed UTC instant, or `None` when it has none
/// or any listed record is missing or unreadable.
///
/// # Errors
///
/// As [`read_all`].
pub(crate) fn latest<S, T>(
    source: &S,
    suite: &SuiteId,
    skipped: &mut Vec<String>,
) -> Result<Option<T>, EvidenceError>
where
    S: EvidenceSource + ?Sized,
    T: StoredRecord + DeserializeOwned,
{
    let skipped_before = skipped.len();
    let mut records = read_all::<S, T>(source, suite, skipped)?;
    if skipped.len() != skipped_before {
        return Ok(None);
    }
    Ok(records.pop())
}

/// The newest record of every recorded suite that has one.
///
/// # Errors
///
/// As [`read_all`].
pub(crate) fn latest_each<S, T>(
    source: &S,
    skipped: &mut Vec<String>,
) -> Result<Vec<T>, EvidenceError>
where
    S: EvidenceSource + ?Sized,
    T: StoredRecord + DeserializeOwned,
{
    let mut records = Vec::new();
    for suite in list_recorded_suites(source)? {
        if let Some(record) = latest::<S, T>(source, &suite, skipped)? {
            records.push(record);
        }
    }
    Ok(records)
}

/// Every suite with at least one recorded run, scan **or** inspection.
///
/// All three directories. A suite that recorded only scans has evidence, and
/// reading `runs/` alone made it invisible to every caller that enumerates —
/// the auditor included, so its obligations read as undischarged while the
/// evidence sat on disk (SR-005 FND-003).
///
/// # Errors
///
/// [`EvidenceError::StoreIo`] when a present directory cannot be listed.
pub fn list_recorded_suites<S>(source: &S) -> Result<Vec<SuiteId>, EvidenceError>
where
    S: EvidenceSource + ?Sized,
{
    let mut suites = std::collections::BTreeSet::new();
    for family in [RUNS_DIR, SCANS_DIR, MOCK_INSPECTIONS_DIR] {
        for name in source.list_directories(family)? {
            suites.insert(name);
        }
    }
    Ok(suites.into_iter().map(SuiteId::new).collect())
}

/// Whether a scan proves nothing — the finding-shaped meaning of vacuity.
///
/// **Not "found nothing".** A scan that ran every rule and reported no finding
/// is a clean result and is exactly the evidence this record type exists to
/// preserve. What proves nothing is a scan that evaluated no rules: it also
/// reports zero findings, and from the findings list alone the two are
/// identical.
///
/// `None` when the tool did not say how many rules it evaluated. The question
/// cannot be asked, so the auditor says nothing rather than something wrong.
#[must_use]
pub fn scan_is_vacuous(record: &FindingRecord) -> Option<bool> {
    record.rules_evaluated.map(|count| count == 0)
}
