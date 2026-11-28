// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Complete store-population snapshots and byte read-back comparisons.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

use ix_trace_rs::implements;
use serde::{Deserialize, Serialize};

use crate::attachment::sha256_hex;
use crate::bounded_read::read_regular_file_bounded;
use crate::error::StoreError;

/// The checked-in inventory location relative to a repository root.
pub const INVENTORY_PATH: &str = "spec/evidence-store-inventory.json";
const INVENTORY_MAX_BYTES: u64 = 1_048_576;
const SNAPSHOT_MAX_BYTES: u64 = 67_108_864;
const STORE_FILE_MAX_BYTES: u64 = 1_073_741_824;
const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// The versioned list of store roots included in a read-back.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StoreInventory {
    /// Inventory schema version.
    pub schema_version: u32,
    /// Repository-relative roots, in deterministic display order.
    pub roots: Vec<String>,
}

/// One file in the complete snapshot population.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadbackFile {
    /// Repository-relative file path.
    pub path: String,
    /// Exact byte length.
    pub bytes: u64,
    /// Lowercase SHA-256 of the complete file contents.
    pub sha256: String,
}

/// A baseline snapshot captured before a cutover or byte-read-back check.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadbackSnapshot {
    /// Snapshot schema version.
    pub schema_version: u32,
    /// Digest of the exact inventory bytes used to select roots.
    pub inventory_sha256: String,
    /// Every regular file found under every inventoried root.
    pub files: Vec<ReadbackFile>,
}

/// Result of a read-back operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadbackStatus {
    /// A non-empty baseline was captured; no comparison was requested.
    Captured,
    /// Every current path and byte digest matches the non-empty baseline.
    Clean,
    /// One or more paths or byte digests differ from the baseline.
    Changed,
    /// No complete population could be established.
    Inconclusive,
}

/// Human-readable inventory, population, and comparison report.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadbackReport {
    /// Outcome of the capture or comparison.
    pub status: ReadbackStatus,
    /// The inventory roots used, if the inventory was valid.
    pub roots: Vec<String>,
    /// Exact current non-empty file population.
    pub files: Vec<ReadbackFile>,
    /// Paths absent from or byte-different from the baseline.
    pub changed_paths: Vec<String>,
    /// Why the result is inconclusive, if applicable.
    pub inconclusive_reason: Option<String>,
    /// Current snapshot for a caller to persist as the next baseline.
    pub snapshot: Option<ReadbackSnapshot>,
}

/// Enumerate the checked-in store inventory and hash every reachable regular file.
///
/// With no baseline, the report carries a snapshot for the caller to retain.
/// With a baseline, the report names every changed, missing, or added path. An
/// absent inventory and an empty population are reported as inconclusive.
#[implements("NFR-025-AC-1")]
#[implements("NFR-025-AC-2")]
#[implements("NFR-025-AC-5")]
pub fn read_back(
    repo: &Path,
    baseline: Option<&ReadbackSnapshot>,
) -> Result<ReadbackReport, StoreError> {
    let inventory_path = repo.join(INVENTORY_PATH);
    let inventory_bytes = match read_regular_file_bounded(&inventory_path, INVENTORY_MAX_BYTES) {
        Ok(bytes) => bytes,
        Err(StoreError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(inconclusive("store inventory is absent"));
        }
        Err(error) => return Err(error),
    };
    let inventory: StoreInventory = serde_json::from_slice(&inventory_bytes)
        .map_err(|error| inventory_invalid(&inventory_path, &error.to_string()))?;
    if inventory.schema_version != 1 || inventory.roots.is_empty() {
        return Err(inventory_invalid(
            &inventory_path,
            "unsupported version or empty root list",
        ));
    }
    validate_roots(&inventory, &inventory_path)?;

    let files = enumerate_files(repo, &inventory)?;
    if files.is_empty() {
        return Ok(ReadbackReport {
            status: ReadbackStatus::Inconclusive,
            roots: inventory.roots,
            files,
            changed_paths: Vec::new(),
            inconclusive_reason: Some("store population is empty".to_owned()),
            snapshot: None,
        });
    }

    let snapshot = ReadbackSnapshot {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        inventory_sha256: sha256_hex(&inventory_bytes),
        files: files.clone(),
    };
    let changed_paths =
        baseline.map_or_else(Vec::new, |baseline| changed_paths(baseline, &snapshot));
    let status = match baseline {
        None => ReadbackStatus::Captured,
        Some(_) if changed_paths.is_empty() => ReadbackStatus::Clean,
        Some(_) => ReadbackStatus::Changed,
    };
    Ok(ReadbackReport {
        status,
        roots: inventory.roots,
        files,
        changed_paths,
        inconclusive_reason: None,
        snapshot: Some(snapshot),
    })
}

/// Read and strictly validate a stored snapshot file.
pub fn read_snapshot(path: &Path) -> Result<ReadbackSnapshot, StoreError> {
    let bytes = read_regular_file_bounded(path, SNAPSHOT_MAX_BYTES)?;
    let snapshot: ReadbackSnapshot = serde_json::from_slice(&bytes)
        .map_err(|error| snapshot_invalid(path, &error.to_string()))?;
    if snapshot.schema_version != SNAPSHOT_SCHEMA_VERSION {
        return Err(snapshot_invalid(
            path,
            "unsupported snapshot schema version",
        ));
    }
    validate_snapshot_files(&snapshot, path)?;
    Ok(snapshot)
}

fn inconclusive(reason: &str) -> ReadbackReport {
    ReadbackReport {
        status: ReadbackStatus::Inconclusive,
        roots: Vec::new(),
        files: Vec::new(),
        changed_paths: Vec::new(),
        inconclusive_reason: Some(reason.to_owned()),
        snapshot: None,
    }
}

fn inventory_invalid(path: &Path, reason: &str) -> StoreError {
    StoreError::ReadbackInventoryInvalid {
        path: path.to_path_buf(),
        reason: reason.to_owned(),
    }
}

fn snapshot_invalid(path: &Path, reason: &str) -> StoreError {
    StoreError::ReadbackSnapshotInvalid {
        path: path.to_path_buf(),
        reason: reason.to_owned(),
    }
}

fn validate_roots(inventory: &StoreInventory, path: &Path) -> Result<(), StoreError> {
    let mut seen = BTreeSet::new();
    if !inventory.roots.iter().any(|root| root == "spec/evidence") {
        return Err(inventory_invalid(
            path,
            "inventory must include the evidence store root spec/evidence",
        ));
    }
    for root in &inventory.roots {
        let root_path = Path::new(root);
        if root_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
            || root_path.as_os_str().is_empty()
            || !seen.insert(root)
        {
            return Err(inventory_invalid(path, "root path is unsafe or duplicated"));
        }
    }
    Ok(())
}

fn enumerate_files(
    repo: &Path,
    inventory: &StoreInventory,
) -> Result<Vec<ReadbackFile>, StoreError> {
    let mut files = BTreeMap::new();
    for root in &inventory.roots {
        let root_path = repo.join(root);
        let metadata = match fs::symlink_metadata(&root_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => {
                return Err(StoreError::Io {
                    operation: "inspect store root",
                    path: root_path,
                    source,
                });
            }
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(inventory_invalid(
                &root_path,
                "root is not a real directory",
            ));
        }
        visit_directory(repo, &root_path, &mut files)?;
    }
    Ok(files.into_values().collect())
}

fn visit_directory(
    repo: &Path,
    directory: &Path,
    files: &mut BTreeMap<String, ReadbackFile>,
) -> Result<(), StoreError> {
    let mut entries = fs::read_dir(directory)
        .map_err(|source| StoreError::Io {
            operation: "enumerate store directory",
            path: directory.to_path_buf(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| StoreError::Io {
            operation: "read store directory entry",
            path: directory.to_path_buf(),
            source,
        })?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|source| StoreError::Io {
            operation: "inspect store entry",
            path: path.clone(),
            source,
        })?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            return Err(StoreError::ReadSourceIsSymlink { path });
        }
        if file_type.is_dir() {
            visit_directory(repo, &path, files)?;
            continue;
        }
        if !file_type.is_file() {
            return Err(StoreError::ReadSourceNotRegularFile {
                path,
                kind: "special file",
            });
        }
        let bytes = read_regular_file_bounded(&path, STORE_FILE_MAX_BYTES)?;
        let relative = path
            .strip_prefix(repo)
            .map_err(|_| inventory_invalid(&path, "enumerated path escaped repository"))?
            .to_string_lossy()
            .replace('\\', "/");
        let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        files.insert(
            relative.clone(),
            ReadbackFile {
                path: relative,
                bytes: size,
                sha256: sha256_hex(&bytes),
            },
        );
    }
    Ok(())
}

fn changed_paths(baseline: &ReadbackSnapshot, current: &ReadbackSnapshot) -> Vec<String> {
    let old: BTreeMap<&str, (&str, u64)> = baseline
        .files
        .iter()
        .map(|file| (file.path.as_str(), (file.sha256.as_str(), file.bytes)))
        .collect();
    let new: BTreeMap<&str, (&str, u64)> = current
        .files
        .iter()
        .map(|file| (file.path.as_str(), (file.sha256.as_str(), file.bytes)))
        .collect();
    let mut changed = old
        .keys()
        .chain(new.keys())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| old.get(path) != new.get(path))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if baseline.schema_version != SNAPSHOT_SCHEMA_VERSION
        || baseline.inventory_sha256 != current.inventory_sha256
    {
        if changed.is_empty() {
            changed.extend(current.files.iter().map(|file| file.path.clone()));
        }
    }
    changed
}

fn validate_snapshot_files(snapshot: &ReadbackSnapshot, path: &Path) -> Result<(), StoreError> {
    let mut seen = BTreeSet::new();
    for file in &snapshot.files {
        if file.path.is_empty()
            || Path::new(&file.path)
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
            || file.sha256.len() != 64
            || !file
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            || !seen.insert(&file.path)
        {
            return Err(snapshot_invalid(
                path,
                "snapshot has an unsafe or duplicate file entry",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ReadbackStatus, read_back};
    use ix_trace_rs::trace;

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temporary repository");
        std::fs::create_dir_all(dir.path().join("spec/evidence/runs")).unwrap();
        std::fs::write(
            dir.path().join(super::INVENTORY_PATH),
            br#"{"schema_version":1,"roots":["spec/evidence"]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("spec/evidence/runs/one.json"),
            br#"{"schemaVersion":1,"result":"pass"}"#,
        )
        .unwrap();
        dir
    }

    #[trace("NFR-025-AC-1")]
    #[trace("NFR-025-AC-2")]
    #[trace("NFR-025-AC-5")]
    #[test]
    fn readback_enumerates_every_store_file_and_detects_a_single_byte_change() {
        let dir = fixture();
        let baseline_report = read_back(dir.path(), None).unwrap();
        assert_eq!(baseline_report.status, ReadbackStatus::Captured);
        let snapshot = baseline_report.snapshot.expect("non-empty baseline");
        assert_eq!(baseline_report.files.len(), 1);
        assert_eq!(baseline_report.files[0].path, "spec/evidence/runs/one.json");

        let record = dir.path().join("spec/evidence/runs/one.json");
        std::fs::write(&record, br#"{"schemaVersion":1,"result":"fail"}"#).unwrap();
        let changed = read_back(dir.path(), Some(&snapshot)).unwrap();
        assert_eq!(changed.status, ReadbackStatus::Changed);
        assert_eq!(changed.changed_paths, ["spec/evidence/runs/one.json"]);
    }

    #[trace("NFR-025-AC-1")]
    #[trace("NFR-025-AC-5")]
    #[test]
    fn empty_population_and_missing_inventory_are_inconclusive() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("spec/evidence")).unwrap();
        std::fs::write(
            dir.path().join(super::INVENTORY_PATH),
            br#"{"schema_version":1,"roots":["spec/evidence"]}"#,
        )
        .unwrap();
        assert_eq!(
            read_back(dir.path(), None).unwrap().status,
            ReadbackStatus::Inconclusive
        );

        std::fs::remove_file(dir.path().join(super::INVENTORY_PATH)).unwrap();
        let missing = read_back(dir.path(), None).unwrap();
        assert_eq!(missing.status, ReadbackStatus::Inconclusive);
        assert_eq!(
            missing.inconclusive_reason.as_deref(),
            Some("store inventory is absent")
        );
    }

    #[test]
    fn inventory_roots_are_repository_relative() {
        let dir = fixture();
        std::fs::write(
            dir.path().join(super::INVENTORY_PATH),
            br#"{"schema_version":1,"roots":["../outside"]}"#,
        )
        .unwrap();
        let result = read_back(dir.path(), None);
        assert!(result.is_err());
    }
}
