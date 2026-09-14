// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The filesystem host: a repository read off disk.
//!
//! Split out of `source.rs` when quoin#471's and quoin#472's waves met on one
//! tree; see that module's header for the contract this implements and for
//! why `raw_evidence_file` digests through `quoin_store` rather than hashing
//! bytes here.

use std::path::{Path, PathBuf};

use quoin_store::{digest_file_sha256, store::store_root};

use crate::error::{MeasurementError, MeasurementErrorCode};
use crate::raw_evidence::RawEvidencePath;

use super::{MeasurementSource, RawEvidenceFile, has_extension};

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

    /// The filesystem half of `resolveRawPath` (`intervention.ts:190-210`).
    ///
    /// The lexical half already happened when the [`RawEvidencePath`] was
    /// parsed. What is left is the part that needs a filesystem: the target
    /// must exist, must be a regular file, and must still be inside the store
    /// once both ends are resolved — the lexical guards say nothing about a
    /// symlink. Written once, because reading a retained file and digesting it
    /// must resolve the same path or they are not talking about the same file.
    fn resolve_raw_evidence(&self, path: &RawEvidencePath) -> Result<PathBuf, MeasurementError> {
        let root = store_root(&self.repo);
        let target = root.join(path.as_str());
        // `existsSync(target) || !statSync(target).isFile()` is one refusal
        // with one sentence in the oracle, so a missing file and a directory
        // read the same way here too.
        let metadata = std::fs::metadata(&target).map_err(|_| absent(path))?;
        if !metadata.is_file() {
            return Err(absent(path));
        }
        let real_root = root.canonicalize().map_err(|error| io(&root, &error))?;
        let real_target = target.canonicalize().map_err(|error| io(&target, &error))?;
        if real_target.starts_with(&real_root) {
            Ok(real_target)
        } else {
            Err(MeasurementError::new(
                MeasurementErrorCode::RawEvidencePathUnsafe,
                format!("path resolves outside evidence store: {path}"),
            ))
        }
    }
}

/// `retained evidence file is absent: <path>` — `intervention.ts:202`.
fn absent(path: &RawEvidencePath) -> MeasurementError {
    MeasurementError::new(
        MeasurementErrorCode::RawEvidencePathUnsafe,
        format!("retained evidence file is absent: {path}"),
    )
}

/// The sorted `.json` file names directly inside `root`, empty if absent.
///
/// `store.ts:79-82` and `intervention.ts:87-90` are the same listing over two
/// directories. Porting them twice would carry a duplicate into new code.
fn json_names(root: &Path) -> Result<Vec<String>, MeasurementError> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(io(root, &error)),
    };
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| io(root, &error))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if has_extension(&name, "json") {
            out.push(name);
        }
    }
    out.sort_unstable();
    Ok(out)
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
        json_names(&crate::store::measurements_root(&self.repo))
    }

    fn collection_location(&self, name: &str) -> String {
        crate::store::measurements_root(&self.repo)
            .join(name)
            .to_string_lossy()
            .into_owned()
    }

    fn collection_bytes(&self, name: &str) -> Result<Vec<u8>, MeasurementError> {
        let path = crate::store::measurements_root(&self.repo).join(name);
        std::fs::read(&path).map_err(|error| io(&path, &error))
    }

    fn intervention_names(&self) -> Result<Vec<String>, MeasurementError> {
        json_names(&crate::store::interventions_root(&self.repo))
    }

    fn intervention_bytes(&self, name: &str) -> Result<Vec<u8>, MeasurementError> {
        let path = crate::store::interventions_root(&self.repo).join(name);
        std::fs::read(&path).map_err(|error| io(&path, &error))
    }

    fn raw_evidence_text(&self, path: &RawEvidencePath) -> Result<String, MeasurementError> {
        let resolved = self.resolve_raw_evidence(path)?;
        std::fs::read_to_string(&resolved).map_err(|error| io(&resolved, &error))
    }

    fn raw_evidence_file(
        &self,
        path: &RawEvidencePath,
    ) -> Result<RawEvidenceFile, MeasurementError> {
        let resolved = self.resolve_raw_evidence(path)?;
        let size_bytes = std::fs::metadata(&resolved)
            .map_err(|error| io(&resolved, &error))?
            .len();
        Ok(RawEvidenceFile {
            size_bytes,
            digest: digest_file_sha256(&resolved)?,
        })
    }

    fn retained_evidence_text(&self, path: &RawEvidencePath) -> Result<String, MeasurementError> {
        let target = store_root(&self.repo).join(path.as_str());
        std::fs::read_to_string(&target).map_err(|error| io(&target, &error))
    }
}
