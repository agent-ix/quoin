// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Where measurement collections live under the evidence store.
//!
//! Ports `store.ts:21-28`. The store root itself is
//! [`quoin_store::store::store_root`]'s and is not respelled here.

use std::path::{Path, PathBuf};

use quoin_store::store::store_root;

use crate::types::ids::CollectionId;

/// The directory every measurement collection is published into.
#[must_use]
pub fn measurements_root(repo: &Path) -> PathBuf {
    store_root(repo).join("measurements")
}

/// Where one collection lives.
///
/// `store.ts:26` runs the id through `safeId` on every call; here the refusal
/// happened when the [`CollectionId`] was parsed, so this cannot be called with
/// an id that does not name a file.
#[must_use]
pub fn measurement_path(repo: &Path, collection_id: &CollectionId) -> PathBuf {
    measurements_root(repo).join(format!("{collection_id}.json"))
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

    use super::{measurement_path, measurements_root};
    use crate::types::ids::CollectionId;

    #[test]
    fn a_collection_lives_under_the_evidence_store() {
        let repo = Path::new("/repo");
        assert_eq!(
            measurements_root(repo),
            Path::new("/repo/spec/evidence/measurements")
        );
        let id = CollectionId::parse("tier1-2026").unwrap();
        assert_eq!(
            measurement_path(repo, &id),
            Path::new("/repo/spec/evidence/measurements/tier1-2026.json")
        );
    }
}
