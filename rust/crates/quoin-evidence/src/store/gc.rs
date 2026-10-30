// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Retention: keep the latest record per suite.

use crate::error::EvidenceError;
use crate::paths::{RUNS_DIR, SCANS_DIR, suite_dir};
use crate::source::EvidenceSource;
use crate::store::codec::decode;
use crate::store::records::{StoredRecord, list, list_recorded_suites};
use crate::types::{FindingRecord, RunRecord};

/// Delete every run and scan that is not the suite's latest. Returns the deleted store-relative paths, sorted.
///
/// "Latest" is the newest by **timestamp** read from the record, whatever the
/// file is called, so a store written under an older naming scheme is safe.
///
/// Scans are collected on the same rule. Walking only `runs/` left every scan
/// record on disk forever, so a store with scan suites grew without bound while
/// `gc` reported it had collected everything (SR-005 FND-004).
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
    let mut skipped = Vec::new();
    for suite in list_recorded_suites(source)? {
        collect::<S, RunRecord>(source, &suite, &mut skipped, &mut deleted)?;
        collect::<S, FindingRecord>(source, &suite, &mut skipped, &mut deleted)?;
    }
    if !dry_run {
        for path in &deleted {
            source.remove(path)?;
        }
    }
    deleted.sort();
    Ok(deleted)
}

fn collect<S, T>(
    source: &S,
    suite: &crate::ids::SuiteId,
    skipped: &mut Vec<String>,
    deleted: &mut Vec<String>,
) -> Result<(), EvidenceError>
where
    S: EvidenceSource + ?Sized,
    T: StoredRecord + serde::de::DeserializeOwned,
{
    let names = list::<S, T>(source, suite)?;
    // The newest record is chosen by the timestamp read from each file, never
    // by its name, so a file under any older naming scheme is still weighed.
    let directory = suite_dir(T::FAMILY, suite);
    let mut keep: Option<(String, String)> = None;
    for name in &names {
        let path = format!("{directory}/{name}");
        let Some(text) = source.read(&path)? else {
            continue;
        };
        match decode::<T>(&path, &text) {
            Ok(record) => {
                let stamp = record.timestamp().to_owned();
                if keep.as_ref().is_none_or(|(best, _)| stamp >= *best) {
                    keep = Some((stamp, name.clone()));
                }
            }
            Err(EvidenceError::StoreRead { .. }) => skipped.push(path),
            Err(other) => return Err(other),
        }
    }
    let keep = keep.map(|(_, name)| name);
    for name in names {
        if keep.as_deref() == Some(name.as_str()) {
            continue;
        }
        deleted.push(format!("{}/{name}", suite_dir(T::FAMILY, suite)));
    }
    Ok(())
}

/// The two families `gc` collects, named so a test can assert the population is
/// not empty.
pub const COLLECTED_FAMILIES: [&str; 2] = [RUNS_DIR, SCANS_DIR];
