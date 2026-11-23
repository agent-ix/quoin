// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Retention: keep the latest record per suite.

use serde_json::{Value, json};

use crate::error::EvidenceError;
use crate::paths::{ANALYSIS_REPORTS_DIR, RUNS_DIR, SCANS_DIR, suite_dir};
use crate::source::EvidenceSource;
use crate::store::codec::{canonical_bytes_of, decode};
use crate::store::records::{StoredRecord, list, list_recorded_suites};
use crate::timestamp::parse_timestamp;
use crate::types::{FindingRecord, Outcome, RunRecord, STORE_SCHEMA_VERSION};

/// Where `gc` records what it retired (FR-030-AC-23).
const RETIRED_DIR: &str = "retired";

/// How many same-named retirement logs `gc` will tell apart before refusing.
const MAX_LOG_NAMES: u32 = 1000;

/// What a retired record said, kept after the record itself is gone.
trait Retire: StoredRecord {
    /// The record's own counts, beside its path, suite and timestamp.
    fn facts(&self) -> Value;
}

impl Retire for RunRecord {
    fn facts(&self) -> Value {
        let count = |wanted: Outcome| {
            self.entries
                .iter()
                .filter(|entry| entry.outcome == wanted)
                .count()
        };
        json!({
            "tool": self.tool,
            "passed": count(Outcome::Pass),
            "failed": count(Outcome::Fail),
            "skipped": count(Outcome::Skip),
            "errored": count(Outcome::Error),
        })
    }
}

impl Retire for FindingRecord {
    fn facts(&self) -> Value {
        json!({ "tool": self.tool, "findings": self.findings.len() })
    }
}

/// Delete every run and scan that is not the suite's latest. Returns the deleted store-relative paths, sorted.
///
/// "Latest" is the newest by parsed UTC instant read from the record, whatever
/// the file is called, so a store written under an older naming scheme is safe.
///
/// Scans are collected on the same rule. Walking only `runs/` left every scan
/// record on disk forever, so a store with scan suites grew without bound while
/// `gc` reported it had collected everything (SR-005 FND-004).
///
/// Before anything is removed, `gc` publishes `retired/<first path>.json`
/// naming every record it is about to delete, with its suite, timestamp, tool
/// and outcome counts. Deleting a superseded failure used to leave no trace
/// that it had existed (PLAT-1791; 21 CFR 11.10(e)). The log is written first
/// and is keyed on its first retired path (suffixed when that name already holds a
/// different log), so a `gc` interrupted mid-delete and repeated completes. A record that cannot be
/// read is kept: `gc` does not delete what it cannot
/// characterise, and it names the file in `skipped`. A suite with any record it
/// cannot read has nothing collected from that family: the unreadable file may
/// be the newest, and deleting the readable ones around it could leave no
/// evidence of the suite at all. `gc` holds the store lock while it runs.
///
/// Mock inspections are deliberately **not** collected, matching the retained
/// behaviour: an inspection record is the only evidence anybody looked, and
/// there is no second place that fact is written.
///
/// # Errors
///
/// [`EvidenceError::StoreIo`] when a directory cannot be listed or a file
/// cannot be removed.
pub fn gc<S: EvidenceSource + ?Sized>(
    source: &mut S,
    dry_run: bool,
    skipped: &mut Vec<String>,
) -> Result<Vec<String>, EvidenceError> {
    let _lock = source.lock_store()?;
    let mut deleted = Vec::new();
    let mut retired = Vec::new();
    for suite in list_recorded_suites(source)? {
        let before_runs = deleted.len();
        collect::<S, RunRecord>(source, &suite, skipped, &mut deleted, &mut retired)?;
        let removed_runs = deleted.get(before_runs..).unwrap_or_default().to_vec();
        for run in &removed_runs {
            if let Some(suffix) = run.strip_prefix("runs/") {
                let projection = format!("{ANALYSIS_REPORTS_DIR}/{suffix}");
                if source.read(&projection)?.is_some() {
                    deleted.push(projection);
                }
            }
        }
        collect::<S, FindingRecord>(source, &suite, skipped, &mut deleted, &mut retired)?;
    }
    if !dry_run {
        publish_retirement_log(source, &mut deleted, &retired)?;
        for path in &deleted {
            source.remove(path)?;
        }
    }
    deleted.sort();
    Ok(deleted)
}

fn publish_retirement_log<S: EvidenceSource + ?Sized>(
    source: &mut S,
    deleted: &mut [String],
    retired: &[(String, Value)],
) -> Result<(), EvidenceError> {
    deleted.sort();
    let mut records: Vec<&(String, Value)> = retired.iter().collect();
    records.sort_by(|a, b| a.0.cmp(&b.0));
    let Some((first, _)) = records.first() else {
        return Ok(());
    };
    let projections: Vec<&String> = deleted
        .iter()
        .filter(|path| path.starts_with(ANALYSIS_REPORTS_DIR))
        .collect();
    let log = json!({
        "schemaVersion": STORE_SCHEMA_VERSION,
        "retired": records.iter().map(|(_, facts)| facts).collect::<Vec<_>>(),
        "projections": projections,
    });
    let bytes = canonical_bytes_of("retirement log", &log)?;
    let name = first.trim_end_matches(".json").replace('/', "-");
    // The first retired path names the log. A later `gc` can retire a different
    // set under the same first path (an interrupted gc, or a key recorded again
    // after being collected); that is a new log, never a refusal that would
    // block collection for good. Identical bytes are the interrupted-and-
    // repeated case and publish as a no-op.
    for attempt in 0..MAX_LOG_NAMES {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("-{attempt}")
        };
        match source.publish(&format!("{RETIRED_DIR}/{name}{suffix}.json"), &bytes) {
            Ok(_) => return Ok(()),
            Err(EvidenceError::RecordIntegrity { .. }) => {}
            Err(other) => return Err(other),
        }
    }
    Err(EvidenceError::RecordIntegrity {
        what: "retirement log names exhausted",
        path: format!("{RETIRED_DIR}/{name}.json"),
        note: "",
    })
}

fn collect<S, T>(
    source: &S,
    suite: &crate::ids::SuiteId,
    skipped: &mut Vec<String>,
    deleted: &mut Vec<String>,
    retired: &mut Vec<(String, Value)>,
) -> Result<(), EvidenceError>
where
    S: EvidenceSource + ?Sized,
    T: Retire + serde::de::DeserializeOwned,
{
    let names = list::<S, T>(source, suite)?;
    // The newest record is chosen by the timestamp read from each file, never
    // by its name, so a file under any older naming scheme is still weighed.
    let directory = suite_dir(T::FAMILY, suite);
    let skipped_before = skipped.len();
    let mut readable: Vec<(String, crate::timestamp::TimestampKey, T)> = Vec::new();
    for name in &names {
        let path = format!("{directory}/{name}");
        let Some(text) = source.read(&path)? else {
            skipped.push(path);
            continue;
        };
        match decode::<T>(&path, &text) {
            Ok(record)
                if record.validate().is_ok()
                    && record.suite() == suite
                    && crate::paths::record_path(T::FAMILY, record.suite(), record.timestamp())
                        == path =>
            {
                if let Some(timestamp) = parse_timestamp(record.timestamp()) {
                    readable.push((name.clone(), timestamp, record));
                } else {
                    skipped.push(path);
                }
            }
            Ok(_) => skipped.push(path),
            Err(EvidenceError::StoreRead { .. }) => skipped.push(path),
            Err(other) => return Err(other),
        }
    }
    if skipped.len() > skipped_before {
        return Ok(());
    }
    let keep = readable
        .iter()
        .max_by(|(left_name, left_stamp, _), (right_name, right_stamp, _)| {
            left_stamp
                .cmp(right_stamp)
                .then_with(|| left_name.cmp(right_name))
        })
        .map(|(name, _, _)| name.as_str());
    for (name, _, record) in &readable {
        if keep == Some(name.as_str()) {
            continue;
        }
        let path = format!("{directory}/{name}");
        let mut facts = record.facts();
        if let Some(object) = facts.as_object_mut() {
            object.insert("path".to_owned(), json!(path));
            object.insert("suite".to_owned(), json!(record.suite().as_str()));
            object.insert("timestamp".to_owned(), json!(record.timestamp()));
        }
        retired.push((path.clone(), facts));
        deleted.push(path);
    }
    Ok(())
}

/// The two families `gc` collects, named so a test can assert the population is
/// not empty.
pub const COLLECTED_FAMILIES: [&str; 2] = [RUNS_DIR, SCANS_DIR];
