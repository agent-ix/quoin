// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded extraction of npm package tarballs.

use std::fs::{self, OpenOptions};
use std::io::{Cursor, Write};
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use tar::Archive;

use super::{MAX_ARCHIVE_ENTRIES, MAX_EXTRACTED_BYTES, resource_bound};
use crate::error::ModulesError;

pub(super) fn extract_package(
    archive_bytes: &[u8],
    destination: &Path,
) -> Result<(), ModulesError> {
    let decoder = GzDecoder::new(Cursor::new(archive_bytes));
    let mut archive = Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|error| ModulesError::NpmArchive {
            detail: error.to_string(),
        })?;
    let mut count = 0_u64;
    let mut extracted_bytes = 0_u64;
    for entry in entries {
        count = count.checked_add(1).ok_or_else(|| {
            resource_bound("npm archive entries", usize::MAX, MAX_ARCHIVE_ENTRIES)
        })?;
        if count > MAX_ARCHIVE_ENTRIES {
            return Err(resource_bound(
                "npm archive entries",
                usize::try_from(count).unwrap_or(usize::MAX),
                MAX_ARCHIVE_ENTRIES,
            ));
        }
        let mut entry = entry.map_err(|error| ModulesError::NpmArchive {
            detail: error.to_string(),
        })?;
        let entry_path = entry.path().map_err(|error| ModulesError::NpmArchive {
            detail: error.to_string(),
        })?;
        let relative = package_relative_path(&entry_path)?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_dir() {
            if !relative.as_os_str().is_empty() {
                let dir = destination.join(relative);
                fs::create_dir_all(&dir).map_err(|source| ModulesError::MaterializeFailed {
                    target: dir,
                    source,
                })?;
            }
            continue;
        }
        if !entry_type.is_file() || relative.as_os_str().is_empty() {
            return Err(ModulesError::NpmArchive {
                detail: "only regular files and directories under package/ are accepted".to_owned(),
            });
        }
        let declared_size = entry.size();
        extracted_bytes = extracted_bytes.checked_add(declared_size).ok_or_else(|| {
            resource_bound("npm extracted bytes", usize::MAX, MAX_EXTRACTED_BYTES)
        })?;
        if extracted_bytes > MAX_EXTRACTED_BYTES {
            return Err(resource_bound(
                "npm extracted bytes",
                usize::try_from(extracted_bytes).unwrap_or(usize::MAX),
                MAX_EXTRACTED_BYTES,
            ));
        }
        let output = destination.join(relative);
        let parent = output
            .parent()
            .ok_or_else(|| ModulesError::UnsafeTreePath {
                entry: output.display().to_string(),
            })?;
        fs::create_dir_all(parent).map_err(|source| ModulesError::MaterializeFailed {
            target: parent.to_path_buf(),
            source,
        })?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)
            .map_err(|source| ModulesError::MaterializeFailed {
                target: output.clone(),
                source,
            })?;
        let copied = std::io::copy(&mut entry, &mut file).map_err(|source| {
            ModulesError::MaterializeFailed {
                target: output.clone(),
                source,
            }
        })?;
        if copied != declared_size {
            return Err(ModulesError::NpmArchive {
                detail: format!(
                    "archive entry size changed: declared {declared_size}, read {copied}"
                ),
            });
        }
        file.flush()
            .map_err(|source| ModulesError::MaterializeFailed {
                target: output,
                source,
            })?;
    }
    Ok(())
}

pub(super) fn package_relative_path(path: &Path) -> Result<PathBuf, ModulesError> {
    if path.is_absolute() {
        return Err(ModulesError::UnsafeTreePath {
            entry: path.display().to_string(),
        });
    }
    let raw = path.to_str().ok_or_else(|| ModulesError::UnsafeTreePath {
        entry: path.to_string_lossy().into_owned(),
    })?;
    if raw.contains('\\') || raw.contains('\0') {
        return Err(ModulesError::UnsafeTreePath {
            entry: raw.to_owned(),
        });
    }
    let Some(relative) = raw.strip_prefix("package/") else {
        if raw == "package" {
            return Ok(PathBuf::new());
        }
        return Err(ModulesError::UnsafeTreePath {
            entry: raw.to_owned(),
        });
    };
    if relative.is_empty() {
        return Ok(PathBuf::new());
    }
    let relative = relative.strip_suffix('/').unwrap_or(relative);
    for component in relative.split('/') {
        if component.is_empty() || component == "." || component == ".." {
            return Err(ModulesError::UnsafeTreePath {
                entry: raw.to_owned(),
            });
        }
    }
    let relative_path = Path::new(relative);
    if relative_path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ModulesError::UnsafeTreePath {
            entry: raw.to_owned(),
        });
    }
    Ok(relative_path.to_path_buf())
}
