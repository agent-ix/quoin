// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The committed default module set (quoin#381, FR-019).
//!
//! `default-modules.yaml` is the canonical source of truth for which modules
//! quoin ships and which revision each is pinned to. This module parses and
//! validates it, reproducing `@agent-ix/ts-plugin-kit`'s
//! `validateMarketplaceManifest` refusals so a manifest the TypeScript accepted
//! is accepted here and nothing else is.

use serde::{Deserialize, Serialize};

use crate::error::ModulesError;
use crate::ids::ModuleName;
use crate::source::Source;

/// Upper bound on a manifest file, in bytes.
///
/// The shipped default set is under 4 KiB. The ceiling stops a corrupt or
/// substituted file being read whole.
pub const MAX_MANIFEST_BYTES: u64 = 1 << 20;

/// Upper bound on entries in one manifest.
///
/// Each entry costs a network fetch on a cold reconcile, so the count is a
/// resource bound, not a formatting preference.
pub const MAX_MANIFEST_ENTRIES: usize = 256;

/// One entry in a marketplace / default-set manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketplaceEntry {
    /// The module's name. Required in a manifest; an ad-hoc install derives it.
    pub name: ModuleName,
    /// Where the content comes from.
    pub source: Source,
    /// Informational pin label (e.g. the git tag); not used for resolution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// `false` makes reconcile skip the entry. Absent means enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_enabled: Option<bool>,
    /// Optional subdirectory *within the resolved source* holding the module
    /// root. Distinct from [`Source::GitSubdir`]'s `path`, which selects what
    /// is fetched; this selects within what was fetched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl MarketplaceEntry {
    /// Whether reconcile should process this entry.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.default_enabled.unwrap_or(true)
    }
}

/// A validated default-set manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketplaceManifest {
    /// Must be `1`.
    pub schema_version: u32,
    /// A human-facing name for the set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The entries.
    pub entries: Vec<MarketplaceEntry>,
}

impl MarketplaceManifest {
    /// The only schema version this crate understands.
    pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

    /// Parse and validate a manifest from YAML text.
    ///
    /// # Errors
    /// [`ModulesError::InvalidManifest`] for a wrong `schemaVersion`, a missing
    /// or malformed entry, an entry count over [`MAX_MANIFEST_ENTRIES`], or any
    /// structurally invalid [`Source`].
    pub fn from_yaml(text: &str) -> Result<Self, ModulesError> {
        if text.len() as u64 > MAX_MANIFEST_BYTES {
            return Err(ModulesError::InvalidManifest {
                detail: format!(
                    "manifest is {} bytes, over the {MAX_MANIFEST_BYTES}-byte limit",
                    text.len()
                ),
            });
        }
        let manifest: Self =
            serde_yaml_ng::from_str(text).map_err(|e| ModulesError::InvalidManifest {
                detail: e.to_string(),
            })?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate a parsed manifest.
    ///
    /// # Errors
    /// As [`MarketplaceManifest::from_yaml`].
    pub fn validate(&self) -> Result<(), ModulesError> {
        if self.schema_version != Self::SUPPORTED_SCHEMA_VERSION {
            return Err(ModulesError::InvalidManifest {
                detail: format!(
                    "manifest schemaVersion must be {}",
                    Self::SUPPORTED_SCHEMA_VERSION
                ),
            });
        }
        if self.entries.len() > MAX_MANIFEST_ENTRIES {
            return Err(ModulesError::InvalidManifest {
                detail: format!(
                    "manifest declares {} entries, over the {MAX_MANIFEST_ENTRIES} limit",
                    self.entries.len()
                ),
            });
        }
        for (index, entry) in self.entries.iter().enumerate() {
            entry
                .source
                .validate()
                .map_err(|e| ModulesError::InvalidManifest {
                    detail: format!("entry {index} ({}): {e}", entry.name),
                })?;
        }
        Ok(())
    }

    /// Every entry reconcile should process, in declaration order.
    pub fn enabled_entries(&self) -> impl Iterator<Item = &MarketplaceEntry> {
        self.entries.iter().filter(|e| e.is_enabled())
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]

    use super::*;

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_220_schema_version_must_be_one() {
        let err = MarketplaceManifest::from_yaml("schemaVersion: 2\nentries: []\n")
            .expect_err("version 2 is refused");
        assert!(err.to_string().contains("schemaVersion"), "{err}");
    }

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_221_an_entry_requires_a_name() {
        let err = MarketplaceManifest::from_yaml(
            "schemaVersion: 1\nentries:\n  - source: {type: path, path: p}\n",
        )
        .expect_err("a nameless entry is refused");
        assert!(err.to_string().contains("name"), "{err}");
    }

    /// Trace: FR-019-AC-2
    #[test]
    fn tc_381_222_default_enabled_false_is_skipped() {
        let manifest = MarketplaceManifest::from_yaml(
            "schemaVersion: 1\nentries:\n  - name: a\n    defaultEnabled: false\n    source: {type: path, path: p}\n  - name: b\n    source: {type: path, path: q}\n",
        )
        .expect("manifest is valid");
        let enabled: Vec<&str> = manifest
            .enabled_entries()
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(enabled, vec!["b"]);
    }
}
