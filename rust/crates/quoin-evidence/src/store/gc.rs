// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Retention: keep the latest record per suite.

use serde_json::{Value, json};

use crate::error::EvidenceError;
use crate::paths::{ANALYSIS_REPORTS_DIR, RUNS_DIR, SCANS_DIR, suite_dir};
use crate::source::EvidenceSource;
use crate::store::codec::{canonical_bytes_of, decode};
use crate::store::records::{StoredRecord, list, list_recorded_suites};
use crate::types::{FindingRecord, Outcome, RunRecord, STORE_SCHEMA_VERSION};

/// Where `gc` records what it retired (FR-030-AC-23).
const RETIRED_DIR: &str = "retired";

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
/// "Latest" is the newest by **timestamp** read from the record, whatever the
/// file is called, so a store written under an older naming scheme is safe.
///
/// Scans are collected on the same rule. Walking only `runs/` left every scan
/// record on disk forever, so a store with scan suites grew without bound while
/// `gc` reported it had collected everything (SR-005 FND-004).
///
/// Before anything is removed, `gc` publishes `retired/<first path>.json`
/// naming every record it is about to delete, with its suite, timestamp, tool
/// and outcome counts. Deleting a superseded failure used to leave no trace
/// that it had existed (PLAT-1791; 21 CFR 11.10(e)). The log is written first
/// and is keyed on its first retired path, so a `gc` interrupted mid-delete and
/// repeated reproduces identical bytes and completes. A record that cannot be
/// read is reported as skipped and kept: `gc` does not delete what it cannot
/// characterise.
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
) -> Result<Vec<String>, EvidenceError> {
    let mut deleted = Vec::new();
    let mut retired = Vec::new();
    let mut skipped = Vec::new();
    for suite in list_recorded_suites(source)? {
        let before_runs = deleted.len();
        collect::<S, RunRecord>(source, &suite, &mut skipped, &mut deleted, &mut retired)?;
        let removed_runs = deleted.get(before_runs..).unwrap_or_default().to_vec();
        for run in &removed_runs {
            if let Some(suffix) = run.strip_prefix("runs/") {
                let projection = format!("{ANALYSIS_REPORTS_DIR}/{suffix}");
                if source.read(&projection)?.is_some() {
                    deleted.push(projection);
                }
            }
        }
        collect::<S, FindingRecord>(source, &suite, &mut skipped, &mut deleted, &mut retired)?;
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
    source.publish(&format!("{RETIRED_DIR}/{name}.json"), &bytes)?;
    Ok(())
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
    let mut readable: Vec<(String, String, T)> = Vec::new();
    for name in &names {
        let path = format!("{directory}/{name}");
        let Some(text) = source.read(&path)? else {
            continue;
        };
        match decode::<T>(&path, &text) {
            Ok(record) => readable.push((name.clone(), record.timestamp().to_owned(), record)),
            Err(EvidenceError::StoreRead { .. }) => skipped.push(path),
            Err(other) => return Err(other),
        }
    }
    let mut keep: Option<(&str, &str)> = None;
    for (name, stamp, _) in &readable {
        if keep.is_none_or(|(best, _)| stamp.as_str() >= best) {
            keep = Some((stamp.as_str(), name.as_str()));
        }
    }
    let keep = keep.map(|(_, name)| name.to_owned());
    for (name, _, record) in &readable {
        if keep.as_deref() == Some(name.as_str()) {
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
