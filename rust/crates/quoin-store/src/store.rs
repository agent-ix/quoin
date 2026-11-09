// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Store layout and durable writes.
//!
//! Every write goes through [`publish`] and is **crash-atomic**: the bytes are
//! written to a temporary name in the destination directory, `fsync`ed,
//! published under the destination name, and the destination directory is
//! `fsync`ed. A reader therefore sees either the previous file or the complete
//! new one, never a truncated one. A half-written record is not a recoverable
//! state. Durability is not a caller's choice; there is no fast variant.
//!
//! The destination type ([`StorePath`]) decides replace-versus-refuse, and the
//! content type ([`Content`]) decides canonicalize-versus-keep-verbatim.
//!
//! The oracle's evidence-store writer (`writeCanonical` → `writeFileSync`) is
//! *not* atomic; its change-assurance writer is. Making both atomic here is a
//! strengthening, not a format change: the bytes on disk are identical.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use engineering_assurance::atomic_publish::{
    self as ea, PublishError, PublishStep, write_file_atomic_no_replace,
};

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

/// A change-assurance record's path: `records/<record_id>/<revision>.json`.
///
/// A record is addressed by the pair `(record_id, revision)`; nothing in the
/// path is derived from content.
#[must_use]
pub fn record_path(repo: &Path, record_id: &str, revision: u64) -> PathBuf {
    change_assurance_root(repo)
        .join("records")
        .join(record_id)
        .join(format!("{revision}.json"))
}

/// A proof attestation's directory: `attestations/<attestation_id>`.
#[must_use]
pub fn attestation_path(repo: &Path, attestation_id: &str) -> PathBuf {
    change_assurance_root(repo)
        .join("attestations")
        .join(attestation_id)
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
/// knows which directory finally holds the name. The only caller is
/// [`publish`], which always ends with [`fsync_directory`]; exporting this
/// would let a caller believe a file is durable when a crash can still lose its
/// name.
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

/// A destination in the store, carrying whether it may ever be replaced.
///
/// The path decides replace-versus-refuse; the caller never chooses. A new
/// caller therefore has no rule to remember and cannot pick wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorePath {
    /// A path that names its own bytes, or an identity that is written once.
    /// A second write of identical bytes is [`Published::AlreadyPresent`]; a
    /// second write of *different* bytes is [`StoreError::ContentCollision`].
    /// Nothing is ever replaced.
    ContentAddressed(PathBuf),
    /// A mutable named record. A write replaces whatever is there, atomically.
    Named(PathBuf),
}

impl StorePath {
    /// The filesystem path this destination names.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::ContentAddressed(path) | Self::Named(path) => path,
        }
    }
}

/// What a store file holds: a record the system authored, or an artifact it
/// received.
///
/// The two are different kinds of thing. A [`Content::Record`] is canonicalized
/// on the way in, so it is canonical by construction. A [`Content::Artifact`]
/// is opaque: its evidentiary value is that it is exactly what was received (a
/// raw API payload, a SARIF file), so its bytes are never re-serialized. An
/// artifact cannot be pushed through the canonical writer.
///
/// Bytes a caller has already encoded (for example the RFC 8785 form the
/// change-assurance family retains) are also [`Content::Artifact`] here: the
/// store writes them as given, so a caller that holds encoded bytes for a
/// record chooses `Artifact` and owns that encoding.
#[derive(Debug, Clone, Copy)]
pub enum Content<'a> {
    /// An authored record, written in the evidence store's canonical form.
    Record(&'a JsonValue),
    /// Opaque bytes, written exactly as given.
    Artifact(&'a [u8]),
}

/// The outcome of a successful [`publish`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Published {
    /// The file was newly written (or, for a named path, replaced).
    Written,
    /// A content-addressed path already held exactly these bytes.
    AlreadyPresent,
}

/// Publish `content` at `destination`, durably.
///
/// This is the store's one write entry point. Durability is not a choice: the
/// bytes are written to a sibling temporary and `fsync`ed, published under
/// the destination name, and the destination directory is `fsync`ed. Parent
/// directories are created.
///
/// How the name is published follows the path:
///
/// * [`StorePath::Named`] renames over the destination, so a reader sees the
///   previous file or the complete new one, never a truncated one.
/// * [`StorePath::ContentAddressed`] uses [`fs::hard_link`], not
///   [`fs::rename`]. Rename replaces whatever is at the destination, so an
///   `exists()` test followed by a rename has a window in which a concurrent
///   writer's *differing* bytes are clobbered, silently turning the collision
///   this path refuses into an overwrite. `hard_link` fails with
///   [`std::io::ErrorKind::AlreadyExists`] instead, which sends the loser of
///   the race down the compare-or-refuse path it would have taken had it
///   arrived second in wall-clock order.
///
/// # Errors
///
/// Refuses with [`StoreError::ContentCollision`] when a content-addressed path
/// already holds different bytes; with the refusals of
/// [`canonical_json_bytes`] for a record that has no canonical spelling; and on
/// any I/O failure creating the parent directory, writing or `fsync`ing the
/// temporary, publishing the name, or `fsync`ing the destination directory.
pub fn publish(destination: &StorePath, content: Content<'_>) -> Result<Published, StoreError> {
    let encoded;
    let bytes = match content {
        Content::Record(value) => {
            encoded = canonical_json_bytes(value)?;
            encoded.as_slice()
        }
        Content::Artifact(bytes) => bytes,
    };
    let path = destination.path();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(io("create directory", parent))?;
    let published = match destination {
        StorePath::Named(_) => {
            let temporary = temporary_beside(path);
            // A leftover temporary from a killed process must not block the write.
            let _ = fs::remove_file(&temporary);
            write_durable_new(&temporary, bytes)?;
            let outcome = replace(&temporary, path);
            let _ = fs::remove_file(&temporary);
            outcome?
        }
        StorePath::ContentAddressed(_) => publish_once(path, bytes)?,
    };
    // Also on `AlreadyPresent`: the writer that created the name may have
    // crashed before its own directory fsync, and success here must mean the
    // name is durable.
    fsync_directory(parent)?;
    Ok(published)
}

/// Rename `temporary` over `path`.
fn replace(temporary: &Path, path: &Path) -> Result<Published, StoreError> {
    fs::rename(temporary, path).map_err(io("rename", path))?;
    Ok(Published::Written)
}

/// Publish `bytes` at `path` without ever replacing it, through Engineering
/// Assurance's durable no-replace publisher. Identical bytes already at the
/// name are [`Published::AlreadyPresent`]; differing bytes are a
/// [`StoreError::ContentCollision`].
fn publish_once(path: &Path, bytes: &[u8]) -> Result<Published, StoreError> {
    match write_file_atomic_no_replace(path, bytes, ()) {
        Ok(ea::Published::Written) => Ok(Published::Written),
        Ok(ea::Published::AlreadyPresent) => Ok(Published::AlreadyPresent),
        Err(PublishError::Collision { path, .. }) => Err(StoreError::ContentCollision { path }),
        Err(PublishError::NoFileName { path }) => Err(StoreError::Io {
            operation: "publish",
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "destination has no file name",
            ),
            path,
        }),
        Err(PublishError::Io { step, path, source }) => Err(StoreError::Io {
            operation: publish_step_operation(step),
            path,
            source,
        }),
    }
}

fn publish_step_operation(step: PublishStep) -> &'static str {
    match step {
        PublishStep::CreateSidecar => "create",
        PublishStep::WriteSidecar => "write",
        PublishStep::SyncSidecar => "fsync",
        PublishStep::Link => "link",
        PublishStep::ReadDestination => "read",
        PublishStep::RemoveSidecar => "remove",
        PublishStep::SyncDirectory => "fsync directory",
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
/// inside [`publish`] would break exactly the concurrent writers it was
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
        Content, Published, STORE_SCHEMA_VERSION, StorePath, publish, read_bytes, write_durable_new,
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
        let named = StorePath::Named(path.clone());
        publish(&named, Content::Artifact(b"first")).expect("first write");
        publish(&named, Content::Artifact(b"second")).expect("second write");
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
        let addressed = StorePath::ContentAddressed(path);
        let body = Content::Artifact(b"body");
        assert_eq!(
            publish(&addressed, body).expect("write"),
            Published::Written
        );
        assert_eq!(
            publish(&addressed, body).expect("idempotent"),
            Published::AlreadyPresent
        );
        let error = publish(&addressed, Content::Artifact(b"other")).expect_err("collision");
        assert_eq!(error.code(), StoreErrorCode::ContentCollision);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A record is canonicalized on the way in; an artifact is kept verbatim.
    /// A compact third-party payload published as an artifact is not
    /// reserialized, and the same value published as a record is.
    ///
    /// Trace: FR-100-AC-9
    #[test]
    fn tc_130_record_is_canonicalized_and_artifact_is_verbatim() {
        let dir = scratch().join("kinds");
        let raw: &[u8] = br#"{"b":0.0,"a":1}"#;
        let artifact = dir.join("payload.json");
        publish(
            &StorePath::ContentAddressed(artifact.clone()),
            Content::Artifact(raw),
        )
        .expect("artifact");
        assert_eq!(read_bytes(&artifact).expect("read"), raw);

        let value = crate::parse_strict_json(raw).expect("parse");
        let record = dir.join("record.json");
        publish(&StorePath::Named(record.clone()), Content::Record(&value)).expect("record");
        assert_eq!(
            read_bytes(&record).expect("read"),
            crate::canonical_json_bytes(&value).expect("canonical")
        );
        assert_ne!(read_bytes(&record).expect("read"), raw);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A named path is replaced and reports `Written`; a content-addressed path
    /// is never replaced, whatever the content kind.
    ///
    /// Trace: FR-100-AC-9
    #[test]
    fn tc_137_the_path_decides_replace_versus_refuse() {
        let dir = scratch().join("decides");
        let named = StorePath::Named(dir.join("named.json"));
        assert_eq!(
            publish(&named, Content::Artifact(b"1")).expect("first"),
            Published::Written
        );
        assert_eq!(
            publish(&named, Content::Artifact(b"2")).expect("replace"),
            Published::Written
        );
        let addressed = StorePath::ContentAddressed(dir.join("addressed.json"));
        publish(&addressed, Content::Artifact(b"1")).expect("first");
        let value = crate::parse_strict_json(b"{}").expect("parse");
        let error = publish(&addressed, Content::Record(&value)).expect_err("collision");
        assert_eq!(error.code(), StoreErrorCode::ContentCollision);
        std::fs::remove_dir_all(&dir).ok();
    }
}
