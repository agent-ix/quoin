// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The carried anomalies of a scope: selection and authored values only
//! (FR-162 Selection and Row, FR-163 As authored).
//!
//! This module reads the `Anomaly` documents under a scope and nothing else.
//! It derives no git value: `fixingChange` and `rationaleChange` belong to
//! [`crate::history`], which this module never calls.

use std::path::{Path, PathBuf};

use crate::error::AnomalyError;
use crate::record::{MappingErrorCode, Record, parse_record};
use crate::refs::anomaly_documents;

/// The `resolution` of a defect deliberately shipped unfixed.
const WONT_FIX: &str = "wont-fix";
/// The `safety_impact` of an anomaly nobody has assessed.
const UNASSESSED: &str = "unassessed";

/// One carried anomaly and the document it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarriedAnomaly {
    /// The document path, under the scope that was read.
    pub path: PathBuf,
    /// The record as authored.
    pub record: Record,
}

impl CarriedAnomaly {
    /// Whether `safety_impact` is `unassessed` (FR-162's row flag).
    #[must_use]
    pub fn unassessed(&self) -> bool {
        self.record.safety_impact == UNASSESSED
    }
}

/// A document in scope that has a mapping error and so no record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableAnomaly {
    /// The document path, under the scope that was read.
    pub path: PathBuf,
    /// The mapping error's code.
    pub code: MappingErrorCode,
}

/// What a scope holds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CarriedRead {
    /// The carried anomalies, ordered by anomaly id and then path.
    pub carried: Vec<CarriedAnomaly>,
    /// The documents that yield no record, in path order.
    pub unreadable: Vec<UnreadableAnomaly>,
}

/// A parsed anomaly record, whether currently carried or resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredAnomaly {
    /// The document path, under the scope that was read.
    pub path: PathBuf,
    /// The record as authored.
    pub record: Record,
}

/// Every readable anomaly record and every record that could not be mapped.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AnomalyRead {
    /// All parsed anomaly records, ordered by anomaly id and then path.
    pub anomalies: Vec<AuthoredAnomaly>,
    /// The documents that yield no record, in path order.
    pub unreadable: Vec<UnreadableAnomaly>,
}

/// Whether a record is carried: not `closed`, `closed` as `wont-fix`, or
/// `unassessed` whatever its status.
#[must_use]
pub fn is_carried(record: &Record) -> bool {
    record.status_word != "closed"
        || record.resolution.as_deref() == Some(WONT_FIX)
        || record.safety_impact == UNASSESSED
}

/// Read every `Anomaly` document under `scope` and select the carried ones.
///
/// # Errors
///
/// Returns an error when `scope` cannot be read.
pub fn read_carried(scope: &Path) -> Result<CarriedRead, AnomalyError> {
    let read = read_anomalies(scope)?;
    let carried = read
        .anomalies
        .into_iter()
        .filter(|anomaly| is_carried(&anomaly.record))
        .map(|anomaly| CarriedAnomaly {
            path: anomaly.path,
            record: anomaly.record,
        })
        .collect();
    Ok(CarriedRead {
        carried,
        unreadable: read.unreadable,
    })
}

/// Read every `Anomaly` document under `scope`, including resolved records.
///
/// # Errors
///
/// Returns an error when `scope` cannot be read.
pub fn read_anomalies(scope: &Path) -> Result<AnomalyRead, AnomalyError> {
    let mut read = AnomalyRead::default();
    for (path, text) in anomaly_documents(scope)? {
        match parse_record(&text) {
            Ok(record) => read.anomalies.push(AuthoredAnomaly { path, record }),
            Err(error) => read.unreadable.push(UnreadableAnomaly {
                path,
                code: error.code,
            }),
        }
    }
    read.anomalies
        .sort_by(|a, b| (&a.record.id, &a.path).cmp(&(&b.record.id, &b.path)));
    Ok(read)
}
