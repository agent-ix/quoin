// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Where the store gets its bytes, and the only place this crate names a
//! filesystem.
//!
//! # Why one trait
//!
//! The store analysis — which run is latest, what `gc` retains, which binding a
//! discharge updates — is the same whether the records are files under
//! `<repo>/spec/evidence` or a map the caller assembled. Only the source of the
//! bytes differs, so only the source of the bytes is abstracted, exactly as
//! `quoin_validators`' `RepoSource` / `DiskRepo` / `MemoryRepo` does.
//!
//! The alternative is two copies of the analysis, and a port that duplicates
//! the thing it ported has not ported it. [`DiskEvidence`] is what the shipped
//! command runs on, so the golden corpus exercises the same code the boundary
//! runs; [`MemoryEvidence`] is what a library caller with no filesystem uses
//! and is what makes this crate's tests independent of a temporary directory.
//!
//! # Two path spaces, said out loud
//!
//! Store paths ([`EvidenceSource::read`] and friends) are **store-root
//! relative**, `/`-separated: `runs/SUITE-1/0123456789ab.json`. The root is
//! `quoin-store`'s ([`quoin_store::store::store_root`]) and never appears here.
//!
//! Repository paths ([`EvidenceSource::source_files`],
//! [`EvidenceSource::source_text`]) are **repository relative** and exist for
//! one caller: the mock-injection source inspection, which reads the code under
//! test rather than the store. They are on the same trait because they are the
//! same host capability, and splitting them would let one implementation supply
//! a filesystem while the other pretended not to.
//!
//! # Writes go through `quoin-store`
//!
//! [`DiskEvidence`] writes with [`quoin_store::store::write_atomic`] for a
//! mutable named record and [`quoin_store::store::write_content_addressed`] for
//! a retained one (the #394 durability decision). Neither this module nor any
//! other in this crate opens a file for writing itself.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::EvidenceError;

/// Where the evidence store reads and writes.
///
/// Every method takes a store-root-relative, `/`-separated path except the two
/// `source_*` members, which take repository-relative ones.
pub trait EvidenceSource {
    /// The text at a store path, or `None` when nothing is there.
    ///
    /// Absence is not an error: an unrecorded suite, an unwritten baseline and
    /// a store that has never been used all read as `None`, and the retained
    /// `readJson` returns `null` for each.
    ///
    /// # Errors
    ///
    /// [`EvidenceError::StoreIo`] when the path exists and cannot be read.
    fn read(&self, path: &str) -> Result<Option<String>, EvidenceError>;

    /// The `.json` file names directly inside a store directory, sorted.
    ///
    /// An absent directory lists as empty, which is what every retained
    /// `existsSync(dir) ? … : []` guard produces.
    ///
    /// # Errors
    ///
    /// [`EvidenceError::StoreIo`] when the directory exists and cannot be
    /// listed.
    fn list_files(&self, directory: &str) -> Result<Vec<String>, EvidenceError>;

    /// The sub-directory names directly inside a store directory, sorted.
    ///
    /// # Errors
    ///
    /// As [`EvidenceSource::list_files`].
    fn list_directories(&self, directory: &str) -> Result<Vec<String>, EvidenceError>;

    /// Replace a mutable named record, atomically.
    ///
    /// # Errors
    ///
    /// [`EvidenceError::StoreIo`] when the write cannot be completed.
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), EvidenceError>;

    /// Publish a content-addressed record, refusing a differing body.
    ///
    /// Returns `true` when the record was newly written and `false` when the
    /// identical bytes were already there — the append-only publish of FR-048.
    ///
    /// # Errors
    ///
    /// [`EvidenceError::RecordIntegrity`] when the path already holds different
    /// bytes, and [`EvidenceError::StoreIo`] on any other failure.
    fn publish(&mut self, path: &str, bytes: &[u8]) -> Result<bool, EvidenceError>;

    /// Remove one store file. Removing an absent file is not an error.
    ///
    /// # Errors
    ///
    /// [`EvidenceError::StoreIo`] when the file exists and cannot be removed.
    fn remove(&mut self, path: &str) -> Result<(), EvidenceError>;

    /// Every inspectable source file in the repository, repository-relative and
    /// sorted.
    ///
    /// # Errors
    ///
    /// [`EvidenceError::StoreIo`] when the walk cannot complete.
    fn source_files(&self) -> Result<Vec<String>, EvidenceError>;

    /// The text of one repository source file.
    ///
    /// # Errors
    ///
    /// [`EvidenceError::StoreIo`] when the file cannot be read.
    fn source_text(&self, path: &str) -> Result<String, EvidenceError>;
}

/// A store on disk, rooted at `<repo>/spec/evidence`.
#[derive(Debug, Clone)]
pub struct DiskEvidence {
    repo: PathBuf,
    root: PathBuf,
}

/// Whether a store file name is one of the store's JSON records.
///
/// Extension-cased exactly: the store writes `.json` and only `.json`, and a
/// case-insensitive match would pick up a `.JSON` file no quoin ever wrote.
fn is_json(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .is_some_and(|extension| extension == "json")
}

/// Source extensions the mock inspection reads.
const SOURCE_EXTENSIONS: &[&str] = &["rs", "py", "ts", "tsx", "js", "jsx"];

/// Directories the mock-injection walk never descends into.
const EXCLUDED_DIRECTORIES: &[&str] = &[".git", "dist", "node_modules", "spec", "target", "vendor"];

impl DiskEvidence {
    /// Open the store of a repository.
    #[must_use]
    pub fn new(repo: impl Into<PathBuf>) -> Self {
        let repo = repo.into();
        let root = quoin_store::store::store_root(&repo);
        Self { repo, root }
    }

    /// The repository this store belongs to.
    #[must_use]
    pub fn repo(&self) -> &Path {
        &self.repo
    }

    /// The store root, `<repo>/spec/evidence`.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn resolve(&self, path: &str) -> PathBuf {
        let mut resolved = self.root.clone();
        for segment in path.split('/').filter(|segment| !segment.is_empty()) {
            resolved.push(segment);
        }
        resolved
    }

    fn entries(&self, directory: &str, want_directory: bool) -> Result<Vec<String>, EvidenceError> {
        let resolved = self.resolve(directory);
        let Ok(read) = std::fs::read_dir(&resolved) else {
            // An absent directory is an empty one. A directory that exists but
            // cannot be listed is indistinguishable here from an absent one,
            // exactly as `existsSync(dir) ? readdirSync(dir) : []` was: the
            // retained behaviour is reproduced rather than tightened, because
            // tightening it would make a store with a permission quirk refuse
            // where it used to report.
            return Ok(Vec::new());
        };
        let mut names = Vec::new();
        for entry in read {
            let entry = entry.map_err(io("list", directory))?;
            let is_directory = entry.file_type().map_err(io("list", directory))?.is_dir();
            if is_directory != want_directory {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if want_directory || is_json(&name) {
                names.push(name);
            }
        }
        names.sort();
        Ok(names)
    }

    fn walk_sources(&self, directory: &Path, out: &mut Vec<String>) -> Result<(), EvidenceError> {
        let Ok(read) = std::fs::read_dir(directory) else {
            return Ok(());
        };
        for entry in read {
            let entry = entry.map_err(io("list", &directory.to_string_lossy()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let file_type = entry
                .file_type()
                .map_err(io("list", &directory.to_string_lossy()))?;
            let path = entry.path();
            if file_type.is_dir() {
                if EXCLUDED_DIRECTORIES.contains(&name.as_str()) {
                    continue;
                }
                self.walk_sources(&path, out)?;
            } else if file_type.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| SOURCE_EXTENSIONS.contains(&extension))
                && let Ok(relative) = path.strip_prefix(&self.repo)
            {
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
        Ok(())
    }
}

fn io<'a>(
    operation: &'static str,
    path: &'a str,
) -> impl Fn(std::io::Error) -> EvidenceError + use<'a> {
    move |source| EvidenceError::StoreIo {
        operation,
        path: path.to_owned(),
        detail: source.to_string(),
    }
}

fn store_io<'a>(
    operation: &'static str,
    path: &'a str,
) -> impl Fn(quoin_store::StoreError) -> EvidenceError + use<'a> {
    move |source| EvidenceError::StoreIo {
        operation,
        path: path.to_owned(),
        detail: source.to_string(),
    }
}

impl EvidenceSource for DiskEvidence {
    fn read(&self, path: &str) -> Result<Option<String>, EvidenceError> {
        let resolved = self.resolve(path);
        match std::fs::read(&resolved) {
            Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(io("read", path)(source)),
        }
    }

    fn list_files(&self, directory: &str) -> Result<Vec<String>, EvidenceError> {
        self.entries(directory, false)
    }

    fn list_directories(&self, directory: &str) -> Result<Vec<String>, EvidenceError> {
        self.entries(directory, true)
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), EvidenceError> {
        quoin_store::store::write_atomic(&self.resolve(path), bytes)
            .map_err(store_io("write", path))
    }

    fn publish(&mut self, path: &str, bytes: &[u8]) -> Result<bool, EvidenceError> {
        quoin_store::store::write_content_addressed(&self.resolve(path), bytes).map_err(|source| {
            if matches!(source, quoin_store::StoreError::ContentCollision { .. }) {
                EvidenceError::RecordIntegrity {
                    what: "content-address collision or corrupt immutable record",
                    path: path.to_owned(),
                    note: "; existing bytes were not overwritten",
                }
            } else {
                store_io("write", path)(source)
            }
        })
    }

    fn remove(&mut self, path: &str) -> Result<(), EvidenceError> {
        match std::fs::remove_file(self.resolve(path)) {
            Ok(()) => Ok(()),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(io("remove", path)(source)),
        }
    }

    fn source_files(&self) -> Result<Vec<String>, EvidenceError> {
        let mut out = Vec::new();
        let repo = self.repo.clone();
        self.walk_sources(&repo, &mut out)?;
        out.sort();
        Ok(out)
    }

    fn source_text(&self, path: &str) -> Result<String, EvidenceError> {
        let mut resolved = self.repo.clone();
        for segment in path.split('/').filter(|segment| !segment.is_empty()) {
            resolved.push(segment);
        }
        std::fs::read(&resolved)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .map_err(io("read", path))
    }
}

/// A store the caller already holds, in memory.
///
/// Performs no I/O and names no host capability. Two maps rather than one
/// because the store half and the repository-source half are genuinely
/// different path spaces, and a single map would let a test satisfy a store
/// read with a source file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryEvidence {
    store: BTreeMap<String, String>,
    sources: BTreeMap<String, String>,
}

impl MemoryEvidence {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed one store file.
    #[must_use]
    pub fn with_file(mut self, path: impl Into<String>, text: impl Into<String>) -> Self {
        self.store.insert(path.into(), text.into());
        self
    }

    /// Seed one repository source file.
    #[must_use]
    pub fn with_source(mut self, path: impl Into<String>, text: impl Into<String>) -> Self {
        self.sources.insert(path.into(), text.into());
        self
    }

    /// Every store path currently held, sorted.
    #[must_use]
    pub fn paths(&self) -> Vec<&str> {
        self.store.keys().map(String::as_str).collect()
    }

    fn children(&self, directory: &str, want_directory: bool) -> Vec<String> {
        let prefix = if directory.is_empty() {
            String::new()
        } else {
            format!("{directory}/")
        };
        let mut names: Vec<String> = self
            .store
            .keys()
            .filter_map(|path| path.strip_prefix(&prefix))
            .filter_map(|rest| {
                let mut parts = rest.splitn(2, '/');
                let head = parts.next()?;
                let has_more = parts.next().is_some();
                if has_more == want_directory && (want_directory || is_json(head)) {
                    Some(head.to_owned())
                } else {
                    None
                }
            })
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

impl EvidenceSource for MemoryEvidence {
    fn read(&self, path: &str) -> Result<Option<String>, EvidenceError> {
        Ok(self.store.get(path).cloned())
    }

    fn list_files(&self, directory: &str) -> Result<Vec<String>, EvidenceError> {
        Ok(self.children(directory, false))
    }

    fn list_directories(&self, directory: &str) -> Result<Vec<String>, EvidenceError> {
        Ok(self.children(directory, true))
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), EvidenceError> {
        self.store
            .insert(path.to_owned(), String::from_utf8_lossy(bytes).into_owned());
        Ok(())
    }

    fn publish(&mut self, path: &str, bytes: &[u8]) -> Result<bool, EvidenceError> {
        let incoming = String::from_utf8_lossy(bytes).into_owned();
        match self.store.get(path) {
            Some(existing) if *existing == incoming => Ok(false),
            Some(_) => Err(EvidenceError::RecordIntegrity {
                what: "content-address collision or corrupt immutable record",
                path: path.to_owned(),
                note: "; existing bytes were not overwritten",
            }),
            None => {
                self.store.insert(path.to_owned(), incoming);
                Ok(true)
            }
        }
    }

    fn remove(&mut self, path: &str) -> Result<(), EvidenceError> {
        self.store.remove(path);
        Ok(())
    }

    fn source_files(&self) -> Result<Vec<String>, EvidenceError> {
        Ok(self.sources.keys().cloned().collect())
    }

    fn source_text(&self, path: &str) -> Result<String, EvidenceError> {
        self.sources
            .get(path)
            .cloned()
            .ok_or_else(|| EvidenceError::StoreIo {
                operation: "read",
                path: path.to_owned(),
                detail: "the caller's snapshot does not hold this path".to_owned(),
            })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{DiskEvidence, EvidenceSource, MemoryEvidence};

    #[test]
    fn an_absent_store_path_reads_as_none_not_as_an_error() {
        let source = MemoryEvidence::new();
        assert_eq!(source.read("bindings.json").unwrap(), None);
        assert!(source.list_files("runs/SUITE-1").unwrap().is_empty());
        assert!(source.list_directories("runs").unwrap().is_empty());
    }

    #[test]
    fn listing_separates_files_from_sub_directories() {
        let source = MemoryEvidence::new()
            .with_file("runs/SUITE-1/aaaaaaaaaaaa.json", "{}")
            .with_file("runs/SUITE-2/bbbbbbbbbbbb.json", "{}")
            .with_file("bindings.json", "{}")
            .with_file("runs/SUITE-1/notes.txt", "");
        assert_eq!(
            source.list_directories("runs").unwrap(),
            ["SUITE-1", "SUITE-2"]
        );
        assert_eq!(
            source.list_files("runs/SUITE-1").unwrap(),
            ["aaaaaaaaaaaa.json"]
        );
        assert_eq!(source.list_files("").unwrap(), ["bindings.json"]);
    }

    #[test]
    fn publishing_identical_bytes_twice_creates_once_and_differing_bytes_refuse() {
        let mut source = MemoryEvidence::new();
        assert!(source.publish("experiments/a.json", b"{}").unwrap());
        assert!(!source.publish("experiments/a.json", b"{}").unwrap());
        assert!(source.publish("experiments/a.json", b"{ }").is_err());
    }

    #[test]
    fn the_disk_store_is_rooted_under_spec_evidence() {
        let source = DiskEvidence::new("/repo");
        assert_eq!(source.root(), std::path::Path::new("/repo/spec/evidence"));
        assert_eq!(source.repo(), std::path::Path::new("/repo"));
    }
}
