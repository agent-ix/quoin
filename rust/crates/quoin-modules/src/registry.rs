// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The install registry at `~/.ix/filament/registry.json` (quoin#381, FR-019).
//!
//! **Wire-compatible with `@agent-ix/ts-plugin-kit`.** During staged
//! coexistence the retained TypeScript reads and writes this same file, so the
//! field names, the omission of absent optionals, and the two-space-plus-newline
//! serialization are contract, not formatting. `tc_381_231` pins the byte-level
//! shape against a golden captured from the TypeScript writer.

use std::fs;
use std::io::Write as _;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::ModulesError;
use crate::ids::{CommitSha, ModuleName};
use crate::semantic::SemanticPin;
use crate::source::Source;

/// Upper bound on the registry file, in bytes.
///
/// One entry is ~300 bytes and the default set is ten. The ceiling stops a
/// corrupt or substituted file being parsed whole.
pub const MAX_REGISTRY_BYTES: u64 = 8 << 20;

/// One installed module's registry record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledModule {
    /// The module's declared name; also its directory name.
    pub name: ModuleName,
    /// Where it came from.
    pub source: Source,
    /// The git ref (tag/branch) that was requested, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<String>,
    /// The resolved commit id — the durable pin used for drift detection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha: Option<CommitSha>,
    /// The cache path the content was materialized from.
    pub resolved_path: String,
    /// The materialized path under the modules directory.
    pub target_path: String,
    /// RFC 3339 timestamp of the install.
    pub installed_at: String,
    /// The semantic contract pin, when the module declares a semantic block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic: Option<SemanticPin>,
}

/// The registry document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleRegistry {
    /// Must be `1`.
    pub schema_version: u32,
    /// Every installed module.
    #[serde(default)]
    pub plugins: Vec<InstalledModule>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self {
            schema_version: 1,
            plugins: Vec::new(),
        }
    }
}

impl ModuleRegistry {
    /// An empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the registry file.
    ///
    /// An absent file, or one whose `plugins` key is not an array, yields an
    /// empty registry — matching the oracle, which treats both as "nothing is
    /// installed" rather than as a failure. Unlike the oracle, **malformed JSON
    /// is an error**: `readRegistry` lets `JSON.parse` throw despite its doc
    /// comment claiming otherwise, and an install that silently forgot every
    /// previous entry because one byte was corrupt would lose the rollback
    /// snapshot `install` depends on.
    ///
    /// # Errors
    /// [`ModulesError::RegistryUnreadable`] on an I/O failure, a file over
    /// [`MAX_REGISTRY_BYTES`], or unparsable JSON.
    pub fn read(path: &Path) -> Result<Self, ModulesError> {
        let meta = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(e) => {
                return Err(ModulesError::RegistryUnreadable {
                    path: path.to_path_buf(),
                    detail: e.to_string(),
                });
            }
        };
        if meta.is_file() && meta.len() > MAX_REGISTRY_BYTES {
            return Err(ModulesError::RegistryUnreadable {
                path: path.to_path_buf(),
                detail: format!(
                    "registry is {} bytes, over the {MAX_REGISTRY_BYTES}-byte limit",
                    meta.len()
                ),
            });
        }
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::new()),
            Err(e) => {
                return Err(ModulesError::RegistryUnreadable {
                    path: path.to_path_buf(),
                    detail: e.to_string(),
                });
            }
        };
        let document: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| ModulesError::RegistryUnreadable {
                path: path.to_path_buf(),
                detail: e.to_string(),
            })?;
        if !document
            .get("plugins")
            .is_some_and(serde_json::Value::is_array)
        {
            return Ok(Self::new());
        }
        serde_json::from_value(document).map_err(|e| ModulesError::RegistryUnreadable {
            path: path.to_path_buf(),
            detail: e.to_string(),
        })
    }

    /// Write the registry atomically: temp sibling, fsync, rename.
    ///
    /// # Errors
    /// [`ModulesError::RegistryUnwritable`] on any I/O failure.
    pub fn write(&self, path: &Path) -> Result<(), ModulesError> {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|source| ModulesError::RegistryUnwritable {
            path: path.to_path_buf(),
            source,
        })?;
        // Two-space indentation plus a trailing newline, matching the bytes the
        // TypeScript writer produces — the file is shared during coexistence.
        let mut body =
            serde_json::to_string_pretty(self).map_err(|e| ModulesError::RegistryUnwritable {
                path: path.to_path_buf(),
                source: std::io::Error::other(e.to_string()),
            })?;
        body.push('\n');

        let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
        let write = || -> std::io::Result<()> {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(body.as_bytes())?;
            file.sync_all()?;
            drop(file);
            fs::rename(&tmp, path)
        };
        write().map_err(|source| {
            let _ = fs::remove_file(&tmp);
            ModulesError::RegistryUnwritable {
                path: path.to_path_buf(),
                source,
            }
        })
    }

    /// Insert or replace `module` by name; the new record goes last, matching
    /// the oracle's `upsertPlugin`.
    pub fn upsert(&mut self, module: InstalledModule) {
        self.plugins.retain(|p| p.name != module.name);
        self.plugins.push(module);
    }

    /// Remove a module by name, reporting whether it was present.
    pub fn remove(&mut self, name: &ModuleName) -> bool {
        let before = self.plugins.len();
        self.plugins.retain(|p| &p.name != name);
        self.plugins.len() != before
    }

    /// Find a module by name.
    #[must_use]
    pub fn find(&self, name: &ModuleName) -> Option<&InstalledModule> {
        self.plugins.iter().find(|p| &p.name == name)
    }

    /// Record a semantic pin under an existing entry, if it is present.
    pub fn pin_semantic(&mut self, name: &ModuleName, pin: SemanticPin) {
        if let Some(entry) = self.plugins.iter_mut().find(|p| &p.name == name) {
            entry.semantic = Some(pin);
        }
    }
}
