// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The semantic-module contract quoin was written against (FR-070, FR-073,
//! FR-075; issue #293).
//!
//! Port of `src/semantic/contract.ts`. One family of schema is quoin's own
//! (`sweep-report.schema.json`); the rest are `agent-ix/filament-core-data`'s
//! module-manifest, package-manifest, common and semantic-core schemas --
//! real dependency edges, not copies, resolved through the published npm
//! packages `@agent-ix/semantic-schema` and `@agent-ix/semantic-core` in this
//! repository's own `node_modules` (PLAT-887 de-vendoring). `build.rs` embeds
//! the installed packages' bytes into the compiled binary at their own paths,
//! under the internal names the path helpers below expect, so a release built
//! from this crate stays self-contained with no runtime dependency on
//! `node_modules` existing.
//!
//! [`schema_dir`] and its siblings locate the *embedded* tree, materialized
//! at runtime by [`crate::materialize_embedded_contract`] -- not a directory
//! this crate reads off disk directly.

use std::path::{Path, PathBuf};

use crate::ids::ContractVersion;

/// The contract this quoin implements.
#[derive(Debug, Clone, Copy)]
pub struct SemanticContract {
    /// The `semantic.contract_version` this quoin understands (FR-070).
    pub contract_version: &'static str,
    /// The ten admitted `semantic` keys (FR-070).
    pub semantic_keys: &'static [&'static str],
}

impl SemanticContract {
    /// The contract version, as an id.
    #[must_use]
    pub fn contract_version(&self) -> ContractVersion {
        ContractVersion::from(self.contract_version)
    }
}

/// The contract.
pub const SEMANTIC_CONTRACT: SemanticContract = SemanticContract {
    contract_version: "1.0.0",
    semantic_keys: &[
        "contract_version",
        // BLOCKED-BY PLAT-1221 (owner: planner titan:QSL:plan): the upstream module-manifest schema still lists `semantic_core` as a required `semantic` key, so a manifest without it is rejected
        "semantic_core",
        "package",
        "exports",
        "imports",
        "targets",
        "mappings",
        "compatibility_posture",
        "legacy_forms",
        "sweep_report",
    ],
};

/// The base every semantic-core `$id` and `$ref` sits under; the version is
/// the next path segment.
pub(crate) const SEMANTIC_CORE_BASE: &str = "https://schemas.agent-ix.org/semantic-core/";

/// The absolute URI `package-manifest.schema.json` resolves `common.schema.json`
/// against — its `$id`.
pub const COMMON_SCHEMA_URI: &str =
    "https://schemas.agent-ix.org/filament-core-data/v1/common.schema.json";

/// The root of the vendored schema tree: `<semantic root>/schemas`.
#[must_use]
pub fn schema_dir(semantic_root: &Path) -> PathBuf {
    semantic_root.join("schemas")
}

/// The module-manifest schema's path.
#[must_use]
pub fn module_manifest_schema_path(semantic_root: &Path) -> PathBuf {
    schema_dir(semantic_root).join("module-manifest.schema.json")
}

/// The semantic-core bundle directory.
#[must_use]
pub fn semantic_core_dir(semantic_root: &Path) -> PathBuf {
    schema_dir(semantic_root).join("semantic-core")
}

/// The filament-core-data package-manifest schema's path.
#[must_use]
pub fn package_manifest_schema_path(semantic_root: &Path) -> PathBuf {
    schema_dir(semantic_root)
        .join("filament-core-data")
        .join("package-manifest.schema.json")
}

/// The filament-core-data common schema's path.
#[must_use]
pub fn common_schema_path(semantic_root: &Path) -> PathBuf {
    schema_dir(semantic_root)
        .join("filament-core-data")
        .join("common.schema.json")
}

/// The sweep-report schema's path (quoin's own, not vendored).
#[must_use]
pub fn sweep_report_schema_path(semantic_root: &Path) -> PathBuf {
    semantic_root.join("sweep-report.schema.json")
}

#[cfg(test)]
// Indexing and `unreachable!` are a test-only convenience: an out-of-range
// index in a test is a failing test, not a downed worker.
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Trace: FR-070
    #[test]
    fn tc_378_040_contract_admits_exactly_ten_keys() {
        assert_eq!(SEMANTIC_CONTRACT.semantic_keys.len(), 10);
    }
}
