// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Where this crate gets its bytes, and the only place it names a filesystem.
//!
//! # Why one trait
//!
//! Plan loading, profile loading, the collection read-back and the
//! raw-evidence check are the same analysis whether the bytes are files under
//! `<repo>/spec/assurance` and `<repo>/spec/evidence` or a map the caller
//! assembled. Only the source of the bytes differs, so only the source of the
//! bytes is abstracted, exactly as `quoin_evidence`'s `EvidenceSource` and
//! `quoin_validators`' `RepoSource` do. The alternative is two copies of the
//! analysis, and a port that duplicates the thing it ported has not ported it.
//!
//! [`DiskMeasurement`] is what the shipped command runs on, so the retained
//! corpus under `spec/evidence/measurements/` exercises the same code the
//! boundary runs. [`MemoryMeasurement`] is what a caller with no filesystem
//! uses, and is what lets this crate's tests state a repository in three lines.
//!
//! # Two path spaces, said out loud
//!
//! - Assurance documents are **repository relative**, `/`-separated:
//!   `spec/assurance/tier1.md`.
//! - Collection names are bare file names inside the measurements directory:
//!   `tier1-2026.json`. The store root is `quoin-store`'s
//!   ([`quoin_store::store::store_root`]) and is not spelled again here.
//! - A [`RawEvidencePath`] is **store-root relative** and has already passed
//!   the lexical guards of `intervention.ts:179-190` before it reaches a
//!   source.
//!
//! # `raw_evidence_file` is a host capability, not a hash function
//!
//! The retained `rawEvidenceFor` reads the file and hashes the bytes
//! (`intervention.ts:116-124`). Hashing is `quoin-store`'s, and `quoin-store`
//! exposes sha256 only over a **path** ([`quoin_store::digest_file_sha256`]) —
//! there is no public bytes-wise sha256 and this crate will not mint a second
//! one (FR-100-CON-4, Stage 6 plan §13.2). So the digest is taken by the
//! source that owns the file, and [`MemoryMeasurement`] refuses with
//! [`MeasurementErrorCode::RawEvidenceUnavailable`] rather than growing a
//! private `sha2` dependency. Closing that gap is `quoin-store`'s ticket.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quoin_store::{RawFileSha256Digest, digest_file_sha256, store::store_root};

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::raw_evidence::RawEvidencePath;

/// One retained file, as the raw-evidence accounting sees it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawEvidenceFile {
    /// The file's size in bytes.
    pub size_bytes: u64,
    /// Its sha256 digest, in the `sha256:<64 hex>` spelling.
    pub digest: RawFileSha256Digest,
}

/// Where measurement inputs are read from.
pub trait MeasurementSource {
    /// Every `.md` document under the repository's assurance roots, as
    /// repository-relative `/`-separated paths, sorted.
    ///
    /// `plans.ts:29-33` looks under `spec/assurance` and then `assurance`,
    /// skipping either if absent, and sorts the union. An absent root is not an
    /// error.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::Io`] when a root exists and cannot be walked.
    fn assurance_documents(&self) -> Result<Vec<String>, MeasurementError>;

    /// The UTF-8 text of one assurance document.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::Io`] when the document cannot be read.
    fn document_text(&self, path: &str) -> Result<String, MeasurementError>;

    /// The `.json` file names directly inside the measurements directory,
    /// sorted.
    ///
    /// An absent directory lists as empty, which is `store.ts:79`'s
    /// `existsSync(root) ? … : []`.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::Io`] when the directory exists and cannot be
    /// listed.
    fn collection_names(&self) -> Result<Vec<String>, MeasurementError>;

    /// The bytes of one stored collection.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::Io`] when the file cannot be read.
    fn collection_bytes(&self, name: &str) -> Result<Vec<u8>, MeasurementError>;

    /// The size and sha256 digest of one retained raw-evidence file.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::RawEvidencePathUnsafe`] when the path resolves
    /// outside the evidence store or names something that is not a regular
    /// file, and [`MeasurementErrorCode::RawEvidenceUnavailable`] from a source
    /// that holds no files.
    fn raw_evidence_file(
        &self,
        path: &RawEvidencePath,
    ) -> Result<RawEvidenceFile, MeasurementError>;
}

/// A repository on disk.
#[derive(Clone, Debug)]
pub struct DiskMeasurement {
    repo: PathBuf,
}

impl DiskMeasurement {
    /// Read from the repository rooted at `repo`.
    #[must_use]
    pub fn new(repo: impl Into<PathBuf>) -> Self {
        Self { repo: repo.into() }
    }

    /// The repository root.
    #[must_use]
    pub fn repo(&self) -> &Path {
        &self.repo
    }
}

/// Whether `name` ends in `.<extension>`.
///
/// `plans.ts:92` and `store.ts:81` both compare the suffix **case
/// sensitively**, so `A.JSON` is not a collection there and is not one here
/// either. Written on the last dot rather than as `ends_with(".json")` because
/// the two agree on every input and only this spelling says which half is the
/// extension.
fn has_extension(name: &str, extension: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, found)| found == extension)
}

/// Render an I/O failure the way the retained code does: the path, then why.
fn io(path: &Path, error: &std::io::Error) -> MeasurementError {
    MeasurementError::new(
        MeasurementErrorCode::Io,
        format!("{}: {error}", path.display()),
    )
}

/// Every `.md` file under `root`, depth first, as absolute paths.
///
/// `plans.ts:86-96` walks with an explicit stack and sorts the result; the sort
/// is the caller's here because the caller sorts the union anyway.
fn markdown_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), MeasurementError> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        // An absent root is skipped, not refused: `existsSync` filters it out
        // before the walk ever starts.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(io(root, &error)),
    };
    for entry in entries {
        let entry = entry.map_err(|error| io(root, &error))?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|error| io(&path, &error))?;
        if kind.is_dir() {
            markdown_files(&path, out)?;
        } else if kind.is_file() && path.extension().is_some_and(|end| end == "md") {
            out.push(path);
        }
    }
    Ok(())
}

impl MeasurementSource for DiskMeasurement {
    fn assurance_documents(&self) -> Result<Vec<String>, MeasurementError> {
        let mut found = Vec::new();
        markdown_files(&self.repo.join("spec").join("assurance"), &mut found)?;
        markdown_files(&self.repo.join("assurance"), &mut found)?;
        let mut out: Vec<String> = found
            .iter()
            .filter_map(|path| path.strip_prefix(&self.repo).ok())
            .map(|path| {
                path.components()
                    .filter_map(|part| part.as_os_str().to_str())
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .collect();
        out.sort_unstable();
        Ok(out)
    }

    fn document_text(&self, path: &str) -> Result<String, MeasurementError> {
        let full = self.repo.join(path);
        std::fs::read_to_string(&full).map_err(|error| io(&full, &error))
    }

    fn collection_names(&self) -> Result<Vec<String>, MeasurementError> {
        let root = crate::store::measurements_root(&self.repo);
        let entries = match std::fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(io(&root, &error)),
        };
        let mut out = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| io(&root, &error))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if has_extension(&name, "json") {
                out.push(name);
            }
        }
        out.sort_unstable();
        Ok(out)
    }

    fn collection_bytes(&self, name: &str) -> Result<Vec<u8>, MeasurementError> {
        let path = crate::store::measurements_root(&self.repo).join(name);
        std::fs::read(&path).map_err(|error| io(&path, &error))
    }

    fn raw_evidence_file(
        &self,
        path: &RawEvidencePath,
    ) -> Result<RawEvidenceFile, MeasurementError> {
        let root = store_root(&self.repo);
        let target = root.join(path.as_str());
        // `intervention.ts:194-210` re-checks containment after `realpath`,
        // because the lexical guards say nothing about a symlink. The same
        // check, in the same order: resolve both ends, then compare.
        let unsafe_path = || {
            MeasurementError::new(
                MeasurementErrorCode::RawEvidencePathUnsafe,
                format!("path resolves outside evidence store: {path}"),
            )
        };
        let metadata = std::fs::metadata(&target).map_err(|error| {
            MeasurementError::new(
                MeasurementErrorCode::RawEvidencePathUnsafe,
                format!("retained evidence file is absent: {path} ({error})"),
            )
        })?;
        if !metadata.is_file() {
            return Err(MeasurementError::new(
                MeasurementErrorCode::RawEvidencePathUnsafe,
                format!("retained evidence file is absent: {path}"),
            ));
        }
        let real_root = root.canonicalize().map_err(|error| io(&root, &error))?;
        let real_target = target.canonicalize().map_err(|error| io(&target, &error))?;
        if !real_target.starts_with(&real_root) {
            return Err(unsafe_path());
        }
        Ok(RawEvidenceFile {
            size_bytes: metadata.len(),
            digest: digest_file_sha256(&real_target)?,
        })
    }
}

/// A repository stated in memory.
///
/// Holds no files, so [`MeasurementSource::raw_evidence_file`] refuses; see the
/// module header for why that is a refusal rather than a second hasher.
#[derive(Clone, Debug, Default)]
pub struct MemoryMeasurement {
    documents: BTreeMap<String, String>,
    collections: BTreeMap<String, Vec<u8>>,
}

impl MemoryMeasurement {
    /// An empty repository.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an assurance document at a repository-relative path.
    #[must_use]
    pub fn with_document(mut self, path: impl Into<String>, text: impl Into<String>) -> Self {
        self.documents.insert(path.into(), text.into());
        self
    }

    /// Add a stored collection under a bare `<id>.json` file name.
    #[must_use]
    pub fn with_collection(mut self, name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        self.collections.insert(name.into(), bytes.into());
        self
    }
}

impl MeasurementSource for MemoryMeasurement {
    fn assurance_documents(&self) -> Result<Vec<String>, MeasurementError> {
        // A `BTreeMap` is already in the order the disk walk sorts into.
        Ok(self
            .documents
            .keys()
            .filter(|path| {
                has_extension(path, "md")
                    && (path.starts_with("spec/assurance/") || path.starts_with("assurance/"))
            })
            .cloned()
            .collect())
    }

    fn document_text(&self, path: &str) -> Result<String, MeasurementError> {
        self.documents.get(path).cloned().ok_or_else(|| {
            MeasurementError::new(MeasurementErrorCode::Io, format!("{path}: absent"))
        })
    }

    fn collection_names(&self) -> Result<Vec<String>, MeasurementError> {
        Ok(self
            .collections
            .keys()
            .filter(|name| has_extension(name, "json"))
            .cloned()
            .collect())
    }

    fn collection_bytes(&self, name: &str) -> Result<Vec<u8>, MeasurementError> {
        self.collections.get(name).cloned().ok_or_else(|| {
            MeasurementError::new(MeasurementErrorCode::Io, format!("{name}: absent"))
        })
    }

    fn raw_evidence_file(
        &self,
        path: &RawEvidencePath,
    ) -> Result<RawEvidenceFile, MeasurementError> {
        Err(MeasurementError::new(
            MeasurementErrorCode::RawEvidenceUnavailable,
            format!("this source holds no files, so `{path}` cannot be digested"),
        ))
    }
}
