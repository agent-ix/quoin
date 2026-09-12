// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Store layout and durable writes.
//!
//! Every write here is **crash-atomic**: the bytes are written to a temporary
//! name in the destination directory, `fsync`ed, renamed into place, and the
//! destination directory is `fsync`ed. A reader therefore sees either the
//! previous file or the complete new one, never a truncated one. Evidence is
//! referenced by digest; a half-written record is not a recoverable state.
//!
//! The oracle's evidence-store writer (`writeCanonical` → `writeFileSync`) is
//! *not* atomic; its change-assurance writer is. Making both atomic here is a
//! strengthening, not a format change: the bytes on disk are identical.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::StoreError;
use crate::json::pretty::canonical_json_bytes;
use crate::json::value::JsonValue;

/// Schema version carried by every machine-written file in the evidence store.
///
/// Frozen. A store file's `schemaVersion` is `1` and the Rust port does not get
/// to renumber it — the number is written into evidence that already exists.
pub const STORE_SCHEMA_VERSION: u32 = 1;

/// The evidence store root for a repository: `<repo>/spec/evidence`.
#[must_use]
pub fn store_root(repo: &Path) -> PathBuf {
    repo.join("spec").join("evidence")
}

/// The change-assurance family root: `<store>/change-assurance`.
#[must_use]
pub fn change_assurance_root(repo: &Path) -> PathBuf {
    store_root(repo).join("change-assurance")
}

/// A sealed change-assurance record's path.
#[must_use]
pub fn record_path(repo: &Path, digest: &crate::digest::CanonicalDigest) -> PathBuf {
    change_assurance_root(repo)
        .join("records")
        .join(format!("{}.json", digest.as_hex()))
}

/// A proof attestation's directory.
#[must_use]
pub fn attestation_path(repo: &Path, digest: &crate::digest::CanonicalDigest) -> PathBuf {
    change_assurance_root(repo)
        .join("attestations")
        .join(digest.as_hex())
}

fn io<'a>(
    operation: &'static str,
    path: &'a Path,
) -> impl FnOnce(std::io::Error) -> StoreError + 'a {
    move |source| StoreError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

/// `fsync` a directory so a rename into it survives a crash.
///
/// # Errors
///
/// Refuses if the directory cannot be opened, or if the `fsync` fails.
pub fn fsync_directory(path: &Path) -> Result<(), StoreError> {
    let handle = File::open(path).map_err(io("open directory", path))?;
    handle.sync_all().map_err(io("fsync directory", path))
}

/// Write bytes to a new file that must not already exist, then `fsync` it.
///
/// Mirrors the oracle's `openSync(path, "wx")`: an existing path is a refusal,
/// never a silent overwrite.
///
/// Deliberately **private**. The file's own bytes are durable when this
/// returns, but its *directory entry* is not — only the `fsync` of the parent
/// makes the name survive a crash, and that fsync belongs to the caller that
/// knows which directory finally holds the name. Every caller here is inside
/// [`write_atomic`] or [`write_content_addressed`], both of which end with
/// [`fsync_directory`]; exporting it would let a caller believe a file is
/// durable when a crash can still lose its name.
fn write_durable_new(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let mut handle = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(io("create", path))?;
    handle.write_all(bytes).map_err(io("write", path))?;
    handle.sync_all().map_err(io("fsync", path))?;
    Ok(())
}

/// A temporary name beside `path`, distinct per process and per call.
fn temporary_beside(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = path.file_name().map_or_else(
        || "store".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!(".{name}.tmp-{}-{sequence}", std::process::id()))
}

/// Replace `path` with `bytes`, atomically and durably.
///
/// Creates parent directories, writes a sibling temporary, `fsync`s it, renames
/// over the destination, then `fsync`s the directory.
///
/// # Errors
///
/// Refuses on any I/O failure creating the parent directory, writing or
/// `fsync`ing the temporary, renaming it into place, or `fsync`ing the
/// destination directory.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(io("create directory", parent))?;
    let temporary = temporary_beside(path);
    // A leftover temporary from a killed process must not block the write.
    let _ = fs::remove_file(&temporary);
    write_durable_new(&temporary, bytes)?;
    match fs::rename(&temporary, path) {
        Ok(()) => {}
        Err(source) => {
            let _ = fs::remove_file(&temporary);
            return Err(StoreError::Io {
                operation: "rename",
                path: path.to_path_buf(),
                source,
            });
        }
    }
    fsync_directory(parent)
}

/// Write a value in the evidence store's canonical form, atomically.
///
/// The bytes are exactly what the oracle's `canonicalJson` produces.
///
/// # Errors
///
/// As [`write_atomic`], plus the refusals of [`canonical_json_bytes`] for a
/// value that has no canonical spelling.
pub fn write_canonical(path: &Path, value: &JsonValue) -> Result<(), StoreError> {
    write_atomic(path, &canonical_json_bytes(value)?)
}

/// Write content-addressed bytes, refusing a collision.
///
/// If the path already exists it must already hold exactly these bytes; a
/// differing body under a digest-named path is corruption, not an overwrite to
/// perform. Returns `true` when the file was newly written.
///
/// The publish step is [`fs::hard_link`], not [`fs::rename`]. Rename replaces
/// whatever is at the destination, so a `exists()` test followed by a rename
/// has a window in which a concurrent writer's *differing* bytes are clobbered
/// by this one — silently turning the collision this function exists to refuse
/// into an overwrite. `hard_link` fails with
/// [`std::io::ErrorKind::AlreadyExists`] instead, which sends the loser of the
/// race down the compare-or-refuse path it would have taken had it arrived
/// second in wall-clock order.
///
/// # Errors
///
/// Refuses with [`StoreError::ContentCollision`] when `path` already holds
/// bytes that differ from `bytes`, and on any I/O failure along the
/// write-temporary, link, compare path.
pub fn write_content_addressed(path: &Path, bytes: &[u8]) -> Result<bool, StoreError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(io("create directory", parent))?;
    let temporary = temporary_beside(path);
    let _ = fs::remove_file(&temporary);
    write_durable_new(&temporary, bytes)?;
    let linked = fs::hard_link(&temporary, path);
    let _ = fs::remove_file(&temporary);
    match linked {
        Ok(()) => {
            fsync_directory(parent)?;
            Ok(true)
        }
        Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = fs::read(path).map_err(io("read", path))?;
            if existing == bytes {
                Ok(false)
            } else {
                Err(StoreError::ContentCollision {
                    path: path.to_path_buf(),
                })
            }
        }
        Err(source) => Err(StoreError::Io {
            operation: "link",
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Remove temporaries a killed write left behind in `directory`.
///
/// A write interrupted between [`write_durable_new`] and the rename leaves a
/// dot-prefixed sibling that no reader ever interprets as a record — the
/// store's readers address files by name, and the temporary's name is not one
/// of them, so an interrupted write can never expose a torn record. It does
/// leave a file, though, and a store that accumulates them is a store nobody
/// can audit by listing.
///
/// This mirrors the retained implementation's `recoverChangeAssuranceStaging`,
/// which is likewise an explicit recovery pass rather than something a write
/// does on another writer's behalf: a temporary that belongs to a write still
/// in flight is indistinguishable from an orphan, so sweeping automatically
/// inside [`write_atomic`] would break exactly the concurrent writers it was
/// meant to protect.
///
/// Returns the number of temporaries removed.
///
/// # Errors
///
/// An unreadable directory. A missing directory is zero, not an error.
pub fn recover_orphaned_temporaries(directory: &Path) -> Result<usize, StoreError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(source) => {
            return Err(StoreError::Io {
                operation: "read directory",
                path: directory.to_path_buf(),
                source,
            });
        }
    };
    let mut removed = 0_usize;
    for entry in entries {
        let entry = entry.map_err(io("read directory entry", directory))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') && name.contains(".tmp-") {
            let path = entry.path();
            fs::remove_file(&path).map_err(io("remove", &path))?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// Read a file's bytes.
///
/// # Errors
///
/// Refuses on any I/O failure reading the file.
pub fn read_bytes(path: &Path) -> Result<Vec<u8>, StoreError> {
    fs::read(path).map_err(io("read", path))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]
    use super::{
        STORE_SCHEMA_VERSION, read_bytes, write_atomic, write_content_addressed, write_durable_new,
    };
    use crate::error::StoreErrorCode;

    fn scratch() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "quoin-store-tc-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&base).expect("scratch");
        base
    }

    /// Trace: FR-100-AC-2
    #[test]
    fn tc_380_schema_version_is_frozen_at_one() {
        assert_eq!(STORE_SCHEMA_VERSION, 1);
    }

    /// Trace: FR-100-AC-9
    #[test]
    fn tc_380_atomic_write_replaces_and_leaves_no_temporary() {
        let dir = scratch().join("atomic");
        let path = dir.join("nested").join("record.json");
        write_atomic(&path, b"first").expect("first write");
        write_atomic(&path, b"second").expect("second write");
        assert_eq!(read_bytes(&path).expect("read"), b"second");
        let leftovers: Vec<_> = std::fs::read_dir(path.parent().expect("parent"))
            .expect("readdir")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty(), "temporary files left behind");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn tc_380_durable_new_refuses_an_existing_path() {
        let dir = scratch().join("exclusive");
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("once.bin");
        write_durable_new(&path, b"a").expect("first");
        let error = write_durable_new(&path, b"b").expect_err("second must refuse");
        assert_eq!(error.code(), StoreErrorCode::Io);
        assert_eq!(read_bytes(&path).expect("read"), b"a");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn tc_380_content_addressed_write_refuses_differing_bytes() {
        let dir = scratch().join("content");
        let path = dir.join("d.json");
        assert!(write_content_addressed(&path, b"body").expect("write"));
        assert!(!write_content_addressed(&path, b"body").expect("idempotent"));
        let error = write_content_addressed(&path, b"other").expect_err("collision");
        assert_eq!(error.code(), StoreErrorCode::ContentCollision);
        std::fs::remove_dir_all(&dir).ok();
    }
}
