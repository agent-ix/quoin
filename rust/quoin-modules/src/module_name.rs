// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Reading a module's declared name from its `manifest.yaml` (quoin#381,
//! FR-019-AC-2).
//!
//! Two layouts are accepted, matching the oracle's `readModuleName`: the
//! manifest at the root, or at `<root>/<basename of root>/manifest.yaml` — the
//! shape a git subdirectory checkout produces when the subdir and the module
//! share a name.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::ModulesError;
use crate::ids::ModuleName;

/// Upper bound on a module manifest, in bytes.
pub const MAX_MODULE_MANIFEST_BYTES: u64 = 4 << 20;

/// Locate the `manifest.yaml` for a module root.
///
/// # Errors
/// [`ModulesError::ManifestNotFound`] when neither layout has one.
pub fn locate_manifest(module_root: &Path) -> Result<PathBuf, ModulesError> {
    let direct = module_root.join("manifest.yaml");
    if direct.is_file() {
        return Ok(direct);
    }
    let nested = module_root
        .file_name()
        .map(|base| module_root.join(base).join("manifest.yaml"))
        .filter(|p| p.is_file());
    nested.ok_or_else(|| ModulesError::ManifestNotFound {
        root: module_root.to_path_buf(),
    })
}

/// Read a module's declared name from its `manifest.yaml`.
///
/// # Errors
/// [`ModulesError::ManifestNotFound`] when no manifest exists,
/// [`ModulesError::ManifestUnreadable`] when it cannot be read or parsed, and
/// [`ModulesError::ManifestHasNoName`] when `name` is absent, not a string, or
/// empty.
pub fn read_module_name(module_root: &Path) -> Result<ModuleName, ModulesError> {
    let path = locate_manifest(module_root)?;
    let meta = fs::metadata(&path).map_err(|e| ModulesError::ManifestUnreadable {
        path: path.clone(),
        detail: e.to_string(),
    })?;
    if meta.len() > MAX_MODULE_MANIFEST_BYTES {
        return Err(ModulesError::ManifestUnreadable {
            path,
            detail: format!(
                "manifest is {} bytes, over the {MAX_MODULE_MANIFEST_BYTES}-byte limit",
                meta.len()
            ),
        });
    }
    let text = fs::read_to_string(&path).map_err(|e| ModulesError::ManifestUnreadable {
        path: path.clone(),
        detail: e.to_string(),
    })?;
    let document: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&text).map_err(|e| ModulesError::ManifestUnreadable {
            path: path.clone(),
            detail: e.to_string(),
        })?;
    let name = document
        .get("name")
        .and_then(serde_yaml_ng::Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ModulesError::ManifestHasNoName { path: path.clone() })?;
    // A manifest naming `../elsewhere` would otherwise pick the directory a
    // module is materialized into. `ModuleName` is where that is refused.
    ModuleName::new(name).map_err(|_| ModulesError::ManifestHasNoName { path })
}
