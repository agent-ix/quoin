// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Reading and writing `trust/<ETD-n>.json`.

use crate::error::EvidenceError;
use crate::ids::TrustDecisionId;
use crate::paths::{TRUST_DIR, trust_decision_path};
use crate::source::EvidenceSource;
use crate::store::codec::{canonical_bytes_of, decode};
use crate::trust::validate_trust_decision;
use crate::types::{STORE_SCHEMA_VERSION, TrustDecision};

/// Write one validated trust decision. Returns the store-relative path.
///
/// The decision is validated **before** it is written: a record that would be
/// refused on the way back in has no business being on disk.
///
/// # Errors
///
/// [`EvidenceError::InvalidRecord`] when the decision fails the FR-093
/// boundary, otherwise [`EvidenceError::Canonicalization`] or
/// [`EvidenceError::StoreIo`].
pub fn write_trust_decision<S: EvidenceSource + ?Sized>(
    source: &mut S,
    decision: &TrustDecision,
) -> Result<String, EvidenceError> {
    validate_trust_decision(decision)?;
    let mut stored = decision.clone();
    stored.schema_version = STORE_SCHEMA_VERSION;
    let path = trust_decision_path(&stored.id);
    let bytes = canonical_bytes_of("trust decision", &stored)?;
    source.write(&path, &bytes)?;
    Ok(path)
}

/// Read one trust decision by id, or `None` when it has never been written.
///
/// # Errors
///
/// [`EvidenceError::StoreRead`] when the file is unparseable and
/// [`EvidenceError::InvalidRecord`] when it parses and does not validate.
pub fn read_trust_decision<S: EvidenceSource + ?Sized>(
    source: &S,
    id: &TrustDecisionId,
) -> Result<Option<TrustDecision>, EvidenceError> {
    let path = trust_decision_path(id);
    let Some(text) = source.read(&path)? else {
        return Ok(None);
    };
    let decision: TrustDecision = decode(&path, &text)?;
    validate_trust_decision(&decision)?;
    Ok(Some(decision))
}

/// Every readable trust decision, in file-name order.
///
/// A decision that does not parse or does not validate is **skipped** and its
/// path pushed onto `skipped`, not fatal: one malformed decision must not hide
/// every other reliance judgement in the store, and the caller is told exactly
/// which file to look at.
///
/// # Errors
///
/// [`EvidenceError::StoreIo`] when the directory exists and cannot be listed.
pub fn read_trust_decisions<S: EvidenceSource + ?Sized>(
    source: &S,
    skipped: &mut Vec<String>,
) -> Result<Vec<TrustDecision>, EvidenceError> {
    let mut decisions = Vec::new();
    for name in source.list_files(TRUST_DIR)? {
        let path = format!("{TRUST_DIR}/{name}");
        let Some(text) = source.read(&path)? else {
            continue;
        };
        match decode::<TrustDecision>(&path, &text)
            .and_then(|decision| validate_trust_decision(&decision).map(|()| decision))
        {
            Ok(decision) => decisions.push(decision),
            Err(_) => skipped.push(path),
        }
    }
    Ok(decisions)
}
