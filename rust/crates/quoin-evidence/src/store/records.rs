// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Reading and writing the three per-suite record families.
//!
//! Runs, scans and mock inspections are laid out identically — one directory
//! per suite, one file per commit — so they share the generic readers below and
//! differ only in their family directory and their record type. The retained
//! `store.ts` writes the three sets out three times; a port that copied that
//! would have three places for the ordering defect agent-ix/quoin#104 fixed to
//! come back to.

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::EvidenceError;
use crate::ids::{Commit, SuiteId};
use crate::paths::{MOCK_INSPECTIONS_DIR, RUNS_DIR, SCANS_DIR, suite_dir};
use crate::source::EvidenceSource;
use crate::store::codec::{canonical_bytes_of, decode};
use crate::types::{FindingRecord, MockInspectionRecord, RunRecord, STORE_SCHEMA_VERSION};

/// What the store orders records by: the caller's timestamp, then the commit.
///
/// **Not the file name.** A file name is a commit prefix, which is uniformly
/// random hex, so the lexicographically last file is the newest run with
/// probability 1/n. That arbitrary choice drove the auditor's freshness and
/// vacuity checks and `gc`'s retention until agent-ix/quoin#104 fixed it, and
/// no amount of re-recording could clear the resulting finding because the
/// ordering is a property of the hashes.
pub(crate) trait StoredRecord {
    /// The family directory this record lives under.
    const FAMILY: &'static str;
    /// A name for the record class, used in a canonicalization refusal.
    const LABEL: &'static str;

    /// The suite the record covers.
    fn suite(&self) -> &SuiteId;
    /// The commit the record was made at.
    fn commit(&self) -> &Commit;
    /// The caller-supplied ISO-8601 stamp.
    fn timestamp(&self) -> &str;
}

impl StoredRecord for RunRecord {
    const FAMILY: &'static str = RUNS_DIR;
    const LABEL: &'static str = "run record";

    fn suite(&self) -> &SuiteId {
        &self.suite
    }
    fn commit(&self) -> &Commit {
        &self.commit
    }
    fn timestamp(&self) -> &str {
        &self.timestamp
    }
}

impl StoredRecord for FindingRecord {
    const FAMILY: &'static str = SCANS_DIR;
    const LABEL: &'static str = "scan record";

    fn suite(&self) -> &SuiteId {
        &self.suite
    }
    fn commit(&self) -> &Commit {
        &self.commit
    }
    fn timestamp(&self) -> &str {
        &self.timestamp
    }
}

impl StoredRecord for MockInspectionRecord {
    const FAMILY: &'static str = MOCK_INSPECTIONS_DIR;
    const LABEL: &'static str = "mock inspection record";

    fn suite(&self) -> &SuiteId {
        &self.suite
    }
    fn commit(&self) -> &Commit {
        &self.commit
    }
    fn timestamp(&self) -> &str {
        &self.timestamp
    }
}

/// Where one record belongs, from the record itself.
fn path_of<T: StoredRecord>(record: &T) -> String {
    format!(
        "{}/{}/{}.json",
        T::FAMILY,
        record.suite(),
        record.commit().short()
    )
}

/// Where one record belongs, from its coordinates.
fn path_for<T: StoredRecord>(suite: &SuiteId, commit: &Commit) -> String {
    format!("{}/{suite}/{}.json", T::FAMILY, commit.short())
}

/// Write one record, last-write-wins at the same `(suite, commit)`.
///
/// The stored `schemaVersion` is always [`STORE_SCHEMA_VERSION`] regardless of
/// what the caller put in the record, reproducing the retained
/// `{ ...record, schemaVersion: STORE_SCHEMA_VERSION }` spread.
///
/// # Errors
///
/// [`EvidenceError::Canonicalization`] when the record has no canonical
/// spelling, [`EvidenceError::StoreIo`] when the write cannot be completed.
pub(crate) fn write_record<S, T>(source: &mut S, record: &T) -> Result<String, EvidenceError>
where
    S: EvidenceSource + ?Sized,
    T: StoredRecord + Serialize + Clone,
{
    let path = path_of(record);
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
    source.write(&path, &bytes)?;
    Ok(path)
}

/// Read one record, or `None` when that `(suite, commit)` has none.
///
/// # Errors
///
/// [`EvidenceError::StoreRead`] when the file exists and does not parse.
pub(crate) fn read_record<S, T>(
    source: &S,
    suite: &SuiteId,
    commit: &Commit,
) -> Result<Option<T>, EvidenceError>
where
    S: EvidenceSource + ?Sized,
    T: StoredRecord + DeserializeOwned,
{
    let path = path_for::<T>(suite, commit);
    match source.read(&path)? {
        None => Ok(None),
        Some(text) => decode(&path, &text).map(Some),
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

/// Every record recorded for a suite, oldest first by timestamp.
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
        let Some(text) = source.read(&path)? else {
            continue;
        };
        match decode::<T>(&path, &text) {
            Ok(record) => records.push(record),
            Err(EvidenceError::StoreRead { .. }) => skipped.push(path),
            Err(other) => return Err(other),
        }
    }
    records.sort_by(|a, b| {
        a.timestamp()
            .cmp(b.timestamp())
            .then_with(|| a.commit().as_str().cmp(b.commit().as_str()))
    });
    Ok(records)
}

/// The newest record of a suite by timestamp, or `None` when it has none.
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
    Ok(read_all::<S, T>(source, suite, skipped)?.pop())
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
