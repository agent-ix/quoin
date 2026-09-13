// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! An evidence store held in memory.
//!
//! Not a mock: it enforces exactly the same rules as the disk store — a slot
//! that already holds different bytes is a collision, and a pair is published
//! whole or not at all — so a test that passes here is a test about the rules
//! rather than about the filesystem. What it cannot show is durability, which
//! is why the disk store carries its own interrupted-publish test.

use std::collections::BTreeMap;

use quoin_store::CanonicalDigest;

use crate::error::{ChangeAssuranceError, Subject};
use crate::intake::{EvidenceStore, Published, RetainedPair};

/// Evidence kept in memory.
#[derive(Clone, Debug, Default)]
pub struct MemoryEvidenceStore {
    records: BTreeMap<String, Vec<u8>>,
    attestations: BTreeMap<String, RetainedPair>,
    staging: BTreeMap<String, RetainedPair>,
}

impl MemoryEvidenceStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Stage a pair without publishing it, as an interrupted publish would
    /// leave behind.
    ///
    /// The disk store cannot be interrupted on demand without a failure seam;
    /// this is the same situation stated directly, so that
    /// [`EvidenceStore::recover_staging`] can be tested over both.
    pub fn strand(&mut self, name: &str, pair: RetainedPair) {
        self.staging.insert(name.to_owned(), pair);
    }

    /// How many pairs are staged but not published.
    #[must_use]
    pub fn staged(&self) -> usize {
        self.staging.len()
    }
}

impl EvidenceStore for MemoryEvidenceStore {
    fn record(&self, digest: &CanonicalDigest) -> Result<Option<Vec<u8>>, ChangeAssuranceError> {
        Ok(self.records.get(digest.as_hex()).cloned())
    }

    fn publish_record(
        &mut self,
        digest: &CanonicalDigest,
        bytes: &[u8],
    ) -> Result<Published, ChangeAssuranceError> {
        match self.records.get(digest.as_hex()) {
            Some(existing) if existing == bytes => Ok(Published::AlreadyRetained),
            Some(_) => Err(ChangeAssuranceError::ContentCollision {
                what: Subject::Record,
                digest: digest.as_hex().to_owned(),
            }),
            None => {
                self.records
                    .insert(digest.as_hex().to_owned(), bytes.to_vec());
                Ok(Published::Retained)
            }
        }
    }

    fn attestation(
        &self,
        digest: &CanonicalDigest,
    ) -> Result<Option<RetainedPair>, ChangeAssuranceError> {
        Ok(self.attestations.get(digest.as_hex()).cloned())
    }

    fn publish_attestation(
        &mut self,
        digest: &CanonicalDigest,
        pair: &RetainedPair,
    ) -> Result<Published, ChangeAssuranceError> {
        match self.attestations.get(digest.as_hex()) {
            Some(existing) if existing == pair => Ok(Published::AlreadyRetained),
            Some(_) => Err(ChangeAssuranceError::ContentCollision {
                what: Subject::Attestation,
                digest: digest.as_hex().to_owned(),
            }),
            None => {
                self.attestations
                    .insert(digest.as_hex().to_owned(), pair.clone());
                Ok(Published::Retained)
            }
        }
    }

    fn recover_staging(&mut self) -> Result<usize, ChangeAssuranceError> {
        let removed = self.staging.len();
        self.staging.clear();
        Ok(removed)
    }
}
