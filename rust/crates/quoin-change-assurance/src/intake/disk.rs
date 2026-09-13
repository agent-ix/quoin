// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! An evidence store on a filesystem.
//!
//! This is the only module in the crate that names a host capability, and it
//! names four: `std::fs`, `std::path`, the process id and the clock. The last
//! two appear solely in a staging directory's name, where they make one
//! writer's staging unlikely to collide with another's; nothing reads them
//! back and no decision depends on either.
//!
//! Paths come from `quoin-store` — [`record_path`] and [`attestation_path`] —
//! so the layout is stated in one place for every family that shares the
//! store.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use quoin_store::CanonicalDigest;
use quoin_store::store::{
    attestation_path, change_assurance_root, fsync_directory, record_path, write_content_addressed,
};

use crate::error::{ChangeAssuranceError, Subject};
use crate::intake::{EvidenceStore, Published, RetainedPair};

/// The prefix every staging directory carries.
///
/// A leading dot keeps it out of the store's own listings, and a reader that
/// addresses attestations by digest never resolves a name beginning with one:
/// staging is invisible by construction rather than by convention.
pub const STAGING_PREFIX: &str = ".tmp-attestation-";

/// The name of the attestation inside a published pair.
const ATTESTATION_FILE: &str = "attestation.json";
/// The name of the output inside a published pair.
const OUTPUT_FILE: &str = "output.bin";

/// Evidence kept under a repository's store root.
pub struct DiskEvidenceStore {
    repo: PathBuf,
    interrupt: Option<Box<dyn Fn() -> Result<(), ChangeAssuranceError> + Send + Sync>>,
}

impl std::fmt::Debug for DiskEvidenceStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DiskEvidenceStore")
            .field("repo", &self.repo)
            .field("interrupt", &self.interrupt.is_some())
            .finish()
    }
}

impl DiskEvidenceStore {
    /// A store under `repo`'s store root.
    #[must_use]
    pub fn new(repo: &Path) -> Self {
        Self {
            repo: repo.to_path_buf(),
            interrupt: None,
        }
    }

    /// Install a hook that runs immediately before the single rename that
    /// makes a staged pair visible.
    ///
    /// The one seam durability needs: "the process died between staging and
    /// publishing" is the failure that a pair-granular publish exists to
    /// survive, and it cannot be observed without being caused. A hook that
    /// returns an error stops the publish exactly where a kill would.
    #[must_use]
    pub fn interrupting(
        mut self,
        hook: impl Fn() -> Result<(), ChangeAssuranceError> + Send + Sync + 'static,
    ) -> Self {
        self.interrupt = Some(Box::new(hook));
        self
    }

    /// The directory published attestations live in.
    #[must_use]
    pub fn attestations_directory(&self) -> PathBuf {
        change_assurance_root(&self.repo).join("attestations")
    }

    fn stage(parent: &Path, pair: &RetainedPair) -> Result<PathBuf, ChangeAssuranceError> {
        fs::create_dir_all(parent).map_err(io("create directory", parent))?;
        let staging = parent.join(staging_name());
        fs::create_dir(&staging).map_err(io("create staging directory", &staging))?;
        write_durable(&staging.join(ATTESTATION_FILE), &pair.attestation)?;
        write_durable(&staging.join(OUTPUT_FILE), &pair.output)?;
        fsync_directory(&staging)?;
        Ok(staging)
    }
}

/// A staging directory name no two writers on one host share.
fn staging_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    format!("{STAGING_PREFIX}{}-{nanos}", std::process::id())
}

fn io(operation: &'static str, path: &Path) -> impl FnOnce(std::io::Error) -> ChangeAssuranceError {
    let path = path.to_path_buf();
    move |source| ChangeAssuranceError::Io {
        operation,
        path,
        source,
    }
}

/// Write a file that must not already exist, and fsync it before returning.
fn write_durable(path: &Path, bytes: &[u8]) -> Result<(), ChangeAssuranceError> {
    use std::io::Write as _;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(io("create", path))?;
    file.write_all(bytes).map_err(io("write", path))?;
    file.sync_all().map_err(io("fsync", path))?;
    Ok(())
}

impl EvidenceStore for DiskEvidenceStore {
    fn record(&self, digest: &CanonicalDigest) -> Result<Option<Vec<u8>>, ChangeAssuranceError> {
        let path = record_path(&self.repo, digest);
        match fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(ChangeAssuranceError::Io {
                operation: "read",
                path,
                source,
            }),
        }
    }

    fn publish_record(
        &mut self,
        digest: &CanonicalDigest,
        bytes: &[u8],
    ) -> Result<Published, ChangeAssuranceError> {
        // One file named by its own digest: `quoin-store` already refuses the
        // collision and survives the race, and there is nothing to add.
        match write_content_addressed(&record_path(&self.repo, digest), bytes) {
            Ok(true) => Ok(Published::Retained),
            Ok(false) => Ok(Published::AlreadyRetained),
            Err(quoin_store::StoreError::ContentCollision { .. }) => {
                Err(ChangeAssuranceError::ContentCollision {
                    what: Subject::Record,
                    digest: digest.as_hex().to_owned(),
                })
            }
            Err(source) => Err(source.into()),
        }
    }

    fn attestation(
        &self,
        digest: &CanonicalDigest,
    ) -> Result<Option<RetainedPair>, ChangeAssuranceError> {
        let directory = attestation_path(&self.repo, digest);
        let attestation = match fs::read(directory.join(ATTESTATION_FILE)) {
            Ok(bytes) => bytes,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(ChangeAssuranceError::Io {
                    operation: "read",
                    path: directory.join(ATTESTATION_FILE),
                    source,
                });
            }
        };
        // A published directory always holds both halves, so a missing output
        // beside a present attestation is damage rather than absence, and it
        // is reported as such instead of as "no such attestation".
        let output = fs::read(directory.join(OUTPUT_FILE))
            .map_err(io("read", &directory.join(OUTPUT_FILE)))?;
        Ok(Some(RetainedPair {
            attestation,
            output,
        }))
    }

    fn publish_attestation(
        &mut self,
        digest: &CanonicalDigest,
        pair: &RetainedPair,
    ) -> Result<Published, ChangeAssuranceError> {
        let final_directory = attestation_path(&self.repo, digest);
        let parent = final_directory
            .parent()
            .map_or_else(|| self.attestations_directory(), Path::to_path_buf);
        if let Some(existing) = self.attestation(digest)? {
            return if &existing == pair {
                Ok(Published::AlreadyRetained)
            } else {
                Err(ChangeAssuranceError::ContentCollision {
                    what: Subject::Attestation,
                    digest: digest.as_hex().to_owned(),
                })
            };
        }
        let staging = Self::stage(&parent, pair)?;
        if let Some(interrupt) = self.interrupt.as_ref() {
            // Staging is deliberately left where it is: it is invisible, it
            // can never expose half a pair, and removing it here would hide
            // the very thing `recover_staging` is asked to find.
            interrupt()?;
        }
        match fs::rename(&staging, &final_directory) {
            Ok(()) => {}
            Err(source) => {
                return Err(ChangeAssuranceError::Io {
                    operation: "publish attestation",
                    path: final_directory,
                    source,
                });
            }
        }
        fsync_directory(&parent)?;
        Ok(Published::Retained)
    }

    fn recover_staging(&mut self) -> Result<usize, ChangeAssuranceError> {
        let parent = self.attestations_directory();
        let entries = match fs::read_dir(&parent) {
            Ok(entries) => entries,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(source) => {
                return Err(ChangeAssuranceError::Io {
                    operation: "read directory",
                    path: parent,
                    source,
                });
            }
        };
        let mut removed = 0_usize;
        for entry in entries {
            let entry = entry.map_err(io("read directory entry", &parent))?;
            if entry
                .file_name()
                .to_string_lossy()
                .starts_with(STAGING_PREFIX)
            {
                let path = entry.path();
                fs::remove_dir_all(&path).map_err(io("remove staging directory", &path))?;
                removed = removed.saturating_add(1);
            }
        }
        Ok(removed)
    }
}
