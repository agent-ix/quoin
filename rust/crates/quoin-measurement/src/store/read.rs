// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading retained collections back, one at a time.
//!
//! Ports `readMeasurementCollectionResults`, `readMeasurementCollections` and
//! `collectionOrder` (`store.ts:55-103`).
//!
//! # Failure isolation is the point
//!
//! `readMeasurementCollectionResults` catches per file so one unreadable
//! collection cannot blank a portfolio view, and
//! `readMeasurementCollections` is the strict caller that raises on the first
//! one. Both are kept, and both are written over
//! [`MeasurementSource`] so the analysis runs unchanged on a repository and on
//! a map stated in a test.

use std::cmp::Ordering;

use quoin_store::parse_strict_json;

use crate::date_time::Rfc3339DateTime;
use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::source::MeasurementSource;
use crate::types::collection::MeasurementCollection;
use crate::validate::stored_measurement_collection;

/// One attempt to read one retained collection.
#[derive(Clone, Debug)]
pub struct MeasurementCollectionReadResult {
    /// The file name that was read, as [`MeasurementSource`] names it.
    pub path: String,
    /// The collection, or why it could not be read.
    pub collection: Result<MeasurementCollection, MeasurementError>,
}

/// Read every retained collection independently.
///
/// A source holding no measurements reads as an empty list, not a refusal.
///
/// # Errors
///
/// [`MeasurementErrorCode::Io`] when the collection directory exists and
/// cannot be listed. A failure to read or parse one collection is reported in
/// that collection's own result, not raised.
pub fn read_measurement_collection_results<S: MeasurementSource + ?Sized>(
    source: &S,
) -> Result<Vec<MeasurementCollectionReadResult>, MeasurementError> {
    let mut out = Vec::new();
    for path in source.collection_names()? {
        let collection = source
            .collection_bytes(&path)
            .and_then(|bytes| Ok(parse_strict_json(&bytes)?))
            .and_then(|value| stored_measurement_collection(&value));
        out.push(MeasurementCollectionReadResult { path, collection });
    }
    Ok(out)
}

/// Read every retained collection, refusing the whole read if any is
/// unreadable.
///
/// The result is in [`collection_order`].
///
/// # Errors
///
/// [`MeasurementErrorCode::CollectionUnreadable`] naming the first collection
/// that could not be read, carrying the underlying refusal as a finding.
pub fn read_measurement_collections<S: MeasurementSource + ?Sized>(
    source: &S,
) -> Result<Vec<MeasurementCollection>, MeasurementError> {
    let mut collections = Vec::new();
    for result in read_measurement_collection_results(source)? {
        match result.collection {
            Ok(collection) => collections.push(collection),
            Err(error) => {
                return Err(MeasurementError::with_findings(
                    MeasurementErrorCode::CollectionUnreadable,
                    format!("{}: unreadable measurement collection", result.path),
                    vec![error.to_string()],
                ));
            }
        }
    }
    collections.sort_by(collection_order);
    Ok(collections)
}

/// The order `store.ts:93-102` sorts collections into: instant, then the
/// timestamp text, then the collection id.
///
/// # The `NaN` branch, stated
///
/// The retained code subtracts two `Date.parse` results. An unparsable
/// timestamp yields `NaN`, `NaN - x` is `NaN`, and `NaN` is falsy, so the
/// comparison **falls through** to the text comparison rather than ordering
/// anything. That is reproduced exactly: a timestamp this crate's grammar
/// refuses drops the pair to the text comparison.
#[must_use]
pub fn collection_order(a: &MeasurementCollection, b: &MeasurementCollection) -> Ordering {
    let instants = Rfc3339DateTime::parse(a.timestamp.as_str())
        .ok()
        .zip(Rfc3339DateTime::parse(b.timestamp.as_str()).ok());
    let by_instant = instants.map_or(Ordering::Equal, |(left, right)| {
        left.epoch_millis().cmp(&right.epoch_millis())
    });
    by_instant
        .then_with(|| a.timestamp.as_str().cmp(b.timestamp.as_str()))
        .then_with(|| a.collection_id.as_str().cmp(b.collection_id.as_str()))
}
