// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Where operational evidence lives under the store.
//!
//! Ports `operational.ts:42-48`. The store root is
//! [`quoin_store::store::store_root`]'s and is not respelled.
//!
//! # The file name is a digest, and the digest has a domain
//!
//! `operational.ts:455-462` names a record file by `sha256(record_id)`, which
//! makes the digest an **identifier inside retained evidence**: the record on
//! disk cannot be found by any other name. This crate mints no sha256 of its
//! own (FR-100-CON-4, and `tc_468_boundary` asserts it), so the minting is
//! [`quoin_store::digest_record_file_name`] under its own
//! [`quoin_store::DigestDomain`] — non-substitutable with the canonical-JCS
//! and change-assurance domains beside it, as FR-201 requires.

use std::path::{Path, PathBuf};

use quoin_store::store::store_root;
use quoin_store::{digest_record_file_name, digest_record_pair_file_name};

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
    operational_root(repo).join(format!(
        "{}.json",
        digest_record_file_name(record_id.as_str())
    ))
}

/// Where one linked pair lives.
///
/// `operational.ts:103-106`. The pair's identity is the two record identities
/// joined by a NUL, which no identity can contain — the schema's
/// `^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$` admits no control character — so the
/// join is unambiguous and the order of the two is part of the name.
#[must_use]
pub fn operational_pair_path(
    repo: &Path,
    capability: &SafeRecordId,
    exercise: &SafeRecordId,
) -> PathBuf {
    let name = digest_record_pair_file_name(capability.as_str(), exercise.as_str());
    operational_pairs_root(repo).join(format!("{name}.json"))
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
    fn a_record_lives_under_the_evidence_store_named_by_its_digest() {
        let repo = Path::new("/repo");
        assert_eq!(
            operational_root(repo),
            Path::new("/repo/spec/evidence/operational")
        );
        // `sha256("quoin-271-release-v0.22.5-capability")`, taken with
        // `printf %s | sha256sum` against the retained identity.
        assert_eq!(
            operational_path(repo, &id("quoin-271-release-v0.22.5-capability")),
            Path::new("/repo/spec/evidence/operational").join(format!(
                "{}.json",
                "017fa3c142e2d63deb5ee7f62d2fbd4f61799cb74c5ea5c611d257a98b52d92c"
            ))
        );
    }

    /// The one retained pair in this repository, named by the derivation.
    #[test]
    fn the_retained_pair_is_named_by_the_two_identities_joined_by_a_nul() {
        let path = operational_pair_path(
            Path::new("/repo"),
            &id("quoin-271-release-v0.22.5-capability"),
            &id("quoin-271-release-v0.22.5-exercise"),
        );
        assert_eq!(
            path,
            Path::new("/repo/spec/evidence/operational/pairs")
                .join("c1b30a188d4d03bbe316e0fdb7582eff3fa314c55268a5f71f81f99f8ea2acf8.json")
        );
    }
}
