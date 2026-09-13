// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading back what the operational store holds.
//!
//! Ports `operational.ts:127-172` (`readOperationalRecords` and
//! `readOperationalEntries`).
//!
//! # A pair file holds two records and is one file
//!
//! The store has two shapes in it: a record file, whose whole content is one
//! record, and a pair file, whose content is a two-record envelope written by
//! `writeOperationalPair`. Both contribute records to the same population, and
//! a pair's two records both report the pair's path as their own — which is
//! what makes an idempotent re-publication of a pair recognisable.
//!
//! # Every refusal names the file
//!
//! `operational.ts:325` prefixes the path onto whatever went wrong, because a
//! store with one bad file in it is otherwise a refusal with no way to find the
//! file. Kept, including for the duplicate-identity refusal, which is about the
//! store rather than about any one file and so names the identity instead.

use std::path::{Path, PathBuf};

use quoin_store::parse_strict_json;

use crate::intervention::intake::{InterventionIntakeError, InterventionRefusalCode};
use crate::json_bridge::to_serde;
use crate::operational::paths::{operational_pairs_root, operational_root};
use crate::operational::validate::{ValidOperationalRecord, validate_operational_record};

/// How many records one pair envelope holds. `operational.ts:152`.
const RECORDS_PER_PAIR: usize = 2;

/// One retained record, and the file it was read from.
///
/// `operational.ts:37-40`'s `RetainedOperationalEntry`.
#[derive(Debug, Clone, PartialEq)]
pub struct RetainedOperationalEntry {
    /// The file the record was read from — a pair file for both of a pair's
    /// records.
    pub path: PathBuf,
    /// The record.
    pub record: ValidOperationalRecord,
}

/// Every retained operational record, in `(observed_at, record_id)` order.
///
/// # Errors
///
/// [`InterventionRefusalCode::InvalidRecord`] naming the file, for a directory
/// that cannot be listed, a file that cannot be read, a pair envelope that does
/// not hold two records, a record that does not validate, or two records
/// sharing one identity.
pub fn read_operational_records(
    repo: &Path,
) -> Result<Vec<ValidOperationalRecord>, InterventionIntakeError> {
    Ok(read_operational_entries(repo)?
        .into_iter()
        .map(|entry| entry.record)
        .collect())
}

/// Every retained operational record with the file it came from.
///
/// # Errors
///
/// As [`read_operational_records`].
pub fn read_operational_entries(
    repo: &Path,
) -> Result<Vec<RetainedOperationalEntry>, InterventionIntakeError> {
    let mut entries = Vec::new();
    for path in json_files(&operational_root(repo))? {
        let record = read_one(&path)?;
        entries.push(RetainedOperationalEntry { path, record });
    }
    for path in json_files(&operational_pairs_root(repo))? {
        for record in read_pair(&path)? {
            entries.push(RetainedOperationalEntry {
                path: path.clone(),
                record,
            });
        }
    }
    refuse_duplicate_identities(&entries)?;
    // Stable, so the file-name order the listing was taken in survives a tie on
    // both keys — which is the only thing that decides it in the retained code
    // either.
    entries.sort_by(|left, right| {
        let left = left.record.record();
        let right = right.record.record();
        left.base()
            .observed_at
            .cmp(&right.base().observed_at)
            .then_with(|| left.base().record_id.cmp(&right.base().record_id))
    });
    Ok(entries)
}

/// The `.json` files directly inside `directory`, sorted by name.
///
/// An absent directory lists as empty, which is `operational.ts:136,145`'s
/// `existsSync(root) ? … : []`.
fn json_files(directory: &Path) -> Result<Vec<PathBuf>, InterventionIntakeError> {
    let listing = match std::fs::read_dir(directory) {
        Ok(listing) => listing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(unreadable(directory, &error.to_string())),
    };
    let mut names = Vec::new();
    for entry in listing {
        let entry = entry.map_err(|error| unreadable(directory, &error.to_string()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.rsplit_once('.').is_some_and(|(_, end)| end == "json") {
            names.push(name);
        }
    }
    names.sort_unstable();
    Ok(names.into_iter().map(|name| directory.join(name)).collect())
}

/// One record file. `operational.ts:320-330`.
fn read_one(path: &Path) -> Result<ValidOperationalRecord, InterventionIntakeError> {
    validate_operational_record(&document(path)?)
        .map_err(|error| unreadable(path, &error.findings().join("; ")))
}

/// One pair envelope. `operational.ts:146-160`.
fn read_pair(path: &Path) -> Result<Vec<ValidOperationalRecord>, InterventionIntakeError> {
    let envelope = document(path)?;
    let records = envelope
        .get("records")
        .and_then(serde_json::Value::as_array)
        .filter(|records| records.len() == RECORDS_PER_PAIR)
        .ok_or_else(|| {
            InterventionIntakeError::new(
                InterventionRefusalCode::InvalidRecord,
                vec![format!(
                    "{}: unreadable operational pair: expected two records",
                    path.display()
                )],
            )
        })?;
    records
        .iter()
        .map(|record| {
            validate_operational_record(record)
                .map_err(|error| unreadable(path, &error.findings().join("; ")))
        })
        .collect()
}

/// One file, read through the crate's one JSON reader.
fn document(path: &Path) -> Result<serde_json::Value, InterventionIntakeError> {
    let bytes = std::fs::read(path).map_err(|error| unreadable(path, &error.to_string()))?;
    let stored = parse_strict_json(&bytes).map_err(|error| unreadable(path, &error.to_string()))?;
    to_serde(&stored).map_err(|error| unreadable(path, &error.to_string()))
}

/// `operational.ts:162-170`: one identity, one record, across the whole store.
fn refuse_duplicate_identities(
    entries: &[RetainedOperationalEntry],
) -> Result<(), InterventionIntakeError> {
    let mut seen: Vec<&str> = Vec::with_capacity(entries.len());
    for entry in entries {
        let identity = entry.record.record_id().as_str();
        if seen.contains(&identity) {
            return Err(InterventionIntakeError::new(
                InterventionRefusalCode::RecordIdCollision,
                vec![format!(
                    "duplicate retained operational record id {identity}"
                )],
            ));
        }
        seen.push(identity);
    }
    Ok(())
}

fn unreadable(path: &Path, detail: &str) -> InterventionIntakeError {
    InterventionIntakeError::new(
        InterventionRefusalCode::InvalidRecord,
        vec![format!(
            "{}: unreadable operational record: {detail}",
            path.display()
        )],
    )
}
