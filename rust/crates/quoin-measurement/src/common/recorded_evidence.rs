// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A pointer from a record to a retained evidence file, as the record wrote
//! it.
//!
//! Distinct from [`crate::raw_evidence::RawEvidenceReference`], which quoin#468
//! ported: that one is the reference `rawEvidenceFor` *mints*, built from
//! `RawEvidencePath`, `NonEmptyText` and `RawFileSha256Digest`, so holding one
//! is proof the path is well-formed and the digest is a sha256. This one is the
//! field as it appears in a stored record — read by serde, unvalidated.
//! Checking a record's claim against the file it names is intake's job, and
//! [`crate::raw_evidence::verify_raw_evidence_references`] takes this type
//! directly: quoin#472 deleted quoin#468's third spelling, `RawEvidenceClaim`,
//! so a retained evidence file has exactly two representations in this crate
//! and `tests/tc_472_evidence_representations.rs` fails if a third returns.

use serde::{Deserialize, Serialize};

use crate::common::identity::{Digest, EvidencePath, MediaType};

/// A retained file a record claims to rest on, as the record spells it.
///
/// `intervention-types.ts:12-17`. `operational-types.ts:1` imports this type
/// rather than redeclaring it, and so does this crate.
///
/// Verifying that the file on disk still digests to [`digest`](Self::digest) is
/// not this type's job and not this wave's — it is
/// [`crate::raw_evidence::verify_raw_evidence_references`], ported by
/// quoin#468 and called at intake by quoin#471/#472.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedEvidenceReference {
    /// Where the file is, relative to the store.
    pub path: EvidencePath,
    /// The file's media type as recorded.
    pub media_type: MediaType,
    /// The file's size in bytes as recorded.
    pub size_bytes: u64,
    /// The file's digest as recorded.
    pub digest: Digest,
}
