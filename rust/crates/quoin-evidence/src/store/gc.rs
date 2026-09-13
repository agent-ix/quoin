// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Retention: keep the latest record per suite plus anything a binding names.

use std::collections::BTreeSet;

use crate::error::EvidenceError;
use crate::paths::{RUNS_DIR, SCANS_DIR, suite_dir};
use crate::source::EvidenceSource;
use crate::store::records::{StoredRecord, latest, list, list_recorded_suites};
use crate::types::{FindingRecord, RunRecord};

/// Delete every run and scan that is neither the suite's latest nor referenced
/// by a binding. Returns the deleted store-relative paths, sorted.
///
/// "Latest" is the newest by **timestamp**, not the last file name. A file name
/// is a commit prefix — uniformly random hex — so retaining the last entry
/// deleted the actual newest run whenever its prefix sorted low and no binding
/// named it (agent-ix/quoin#104).
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
/// cannot be removed, and [`EvidenceError::StoreRead`] when `bindings.json` is
/// present and unparseable.
pub fn gc<S: EvidenceSource + ?Sized>(
    source: &mut S,
    dry_run: bool,
) -> Result<Vec<String>, EvidenceError> {
    let referenced: BTreeSet<String> = crate::store::graph::read_bindings(source)?
        .bindings
        .iter()
        .map(|binding| format!("{}/{}.json", binding.suite, binding.commit.short()))
        .collect();

    let mut deleted = Vec::new();
    let mut skipped = Vec::new();
    for suite in list_recorded_suites(source)? {
        collect::<S, RunRecord>(source, &suite, &referenced, &mut skipped, &mut deleted)?;
        collect::<S, FindingRecord>(source, &suite, &referenced, &mut skipped, &mut deleted)?;
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
    referenced: &BTreeSet<String>,
    skipped: &mut Vec<String>,
    deleted: &mut Vec<String>,
) -> Result<(), EvidenceError>
where
    S: EvidenceSource + ?Sized,
    T: StoredRecord + serde::de::DeserializeOwned,
{
    let names = list::<S, T>(source, suite)?;
    let keep = latest::<S, T>(source, suite, skipped)?
        .map(|record| format!("{}.json", record.commit().short()));
    for name in names {
        if keep.as_deref() == Some(name.as_str()) {
            continue;
        }
        if referenced.contains(&format!("{suite}/{name}")) {
            continue;
        }
        deleted.push(format!("{}/{name}", suite_dir(T::FAMILY, suite)));
    }
    Ok(())
}

/// The two families `gc` collects, named so a test can assert the population is
/// not empty.
pub const COLLECTED_FAMILIES: [&str; 2] = [RUNS_DIR, SCANS_DIR];
