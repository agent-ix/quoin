// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Publishing a measurement collection, write-once.
//!
//! Ports `writeMeasurementCollection` (`store.ts:31-53`).
//!
//! # What the retained code does, and what this does instead
//!
//! `store.ts` writes a `wx` temporary beside the destination and then
//! `renameSync`s it into place after an `existsSync` test. Rename **replaces**,
//! so a concurrent writer's differing bytes can be clobbered in the window
//! between the test and the rename — the exact collision the function exists to
//! refuse, turned into a silent overwrite. `atomic-file.ts`, the newer of the
//! two copies, already knows this and uses `linkSync`.
//!
//! Both unify on [`quoin_store::store::write_content_addressed`], which links
//! rather than renames, `fsync`s the file and the directory, and refuses with
//! `ContentCollision`. Two declared divergences follow, both strengthenings:
//! `store.ts`'s rename becomes a link, and durability is added.

use std::path::{Path, PathBuf};

use quoin_store::{JsonValue, canonical_json_bytes, store::write_content_addressed};

use crate::error::MeasurementError;
use crate::plans::{PlanLoadOptions, load_measurement_plans};
use crate::source::DiskMeasurement;
use crate::store::paths::measurement_path;
use crate::types::ids::CollectionId;
use crate::validate;

/// Publish a complete collection, refusing to replace a retained one.
///
/// Returns the path written, whether or not this call was the writer: an
/// identical republication is idempotent, as `store.ts:38` is.
///
/// # Errors
///
/// [`crate::error::MeasurementErrorCode::CollectionInvalid`] when the candidate
/// is not admissible, [`crate::error::MeasurementErrorCode::CollectionIdUnsafe`]
/// when its id does not name a file,
/// [`crate::error::MeasurementErrorCode::CollectionIdCollision`] when the id is
/// already retained holding different bytes, and
/// [`crate::error::MeasurementErrorCode::PlanInvalid`] from the plan load the
/// admission check needs.
pub fn write_measurement_collection(
    repo: &Path,
    candidate: &JsonValue,
) -> Result<PathBuf, MeasurementError> {
    let source = DiskMeasurement::new(repo);
    let plans = load_measurement_plans(&source, PlanLoadOptions::default())?;
    let collection = validate::measurement_collection(candidate, &plans)?;
    let id = CollectionId::parse(collection.collection_id.as_str())?;
    let path = measurement_path(repo, &id);
    // The **caller's** value is what is written, canonicalised — not a
    // re-serialisation of the parsed collection. `store.ts:40` does the same,
    // and it is what keeps members this crate does not model from being
    // dropped on the way to disk.
    let bytes = canonical_json_bytes(candidate)?;
    write_content_addressed(&path, &bytes)?;
    Ok(path)
}
