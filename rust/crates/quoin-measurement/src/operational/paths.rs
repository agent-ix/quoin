// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Where operational evidence lives under the store.
//!
//! Ports `operational.ts:42-48`. The store root is
//! [`quoin_store::store::store_root`]'s and is not respelled.
//!
//! # File names
//!
//! A record file is named by its record identity, with `/` (which the identity
//! grammar admits and a file name cannot hold) written `_` (which it does not
//! admit), so the mapping is injective. A pair file joins the two names with
//! `+`, which no identity contains.

use std::path::{Path, PathBuf};

use quoin_store::store::store_root;

use crate::types::ids::SafeRecordId;

/// The directory every operational record is published into.
#[must_use]
pub fn operational_root(repo: &Path) -> PathBuf {
    store_root(repo).join("operational")
}

/// The directory every linked capability/exercise pair is published into.
#[must_use]
pub fn operational_pairs_root(repo: &Path) -> PathBuf {
    operational_root(repo).join("pairs")
}

/// Where one record lives.
///
/// `operational.ts:46` runs the identity through `recordFileId`'s guard on
/// every call; here the refusal happened when the [`SafeRecordId`] was parsed,
/// so this cannot be called with an identity that does not name a file.
#[must_use]
pub fn operational_path(repo: &Path, record_id: &SafeRecordId) -> PathBuf {
    operational_root(repo).join(format!("{}.json", file_stem(record_id)))
}

/// A record identity as a file-name stem.
fn file_stem(record_id: &SafeRecordId) -> String {
    record_id.as_str().replace('/', "_")
}

/// Where one linked pair lives.
///
/// The two record identities joined by `+`, which the schema's
/// `^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$` does not admit, so the join is
/// unambiguous and the order of the two is part of the name.
#[must_use]
pub fn operational_pair_path(
    repo: &Path,
    capability: &SafeRecordId,
    exercise: &SafeRecordId,
) -> PathBuf {
    operational_pairs_root(repo).join(format!(
        "{}+{}.json",
        file_stem(capability),
        file_stem(exercise)
    ))
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{operational_pair_path, operational_path, operational_root};
    use crate::error::MeasurementErrorCode;
    use crate::types::ids::SafeRecordId;

    fn id(value: &str) -> SafeRecordId {
        SafeRecordId::parse(value, MeasurementErrorCode::CollectionInvalid).unwrap()
    }

    #[test]
    fn a_record_and_a_pair_are_named_by_their_identities() {
        let repo = Path::new("/repo");
        assert_eq!(
            operational_root(repo),
            Path::new("/repo/spec/evidence/operational")
        );
        assert_eq!(
            operational_path(repo, &id("quoin-271/release")),
            Path::new("/repo/spec/evidence/operational/quoin-271_release.json")
        );
        assert_eq!(
            operational_pair_path(repo, &id("a-capability"), &id("a-exercise")),
            Path::new("/repo/spec/evidence/operational/pairs/a-capability+a-exercise.json")
        );
    }
}
