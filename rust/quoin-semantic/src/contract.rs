// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Agent-IX

//! The semantic-module contract quoin was written against (FR-070, FR-073,
//! FR-075; issue #293).
//!
//! Port of `src/semantic/contract.ts`. Three families of schema are vendored
//! under `src/semantic/schemas/`, each with recorded provenance, because there
//! is no dependency edge along which the files could travel and quoin performs
//! no network read on a command path.
//!
//! The vendored tree is **not duplicated into this crate**. It stays where the
//! TypeScript keeps it, and [`schema_dir`] locates it, so the two
//! implementations cannot drift onto different bytes during the coexistence
//! window NFR-024 bounds.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::SemanticError;
use crate::ids::{ContractVersion, SemanticCoreVersion};

/// A vendored file's origin: repository, exact commit, path there, and bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendoredSource {
    /// Owning repository, `<org>/<repo>`.
    pub repository: &'static str,
    /// The exact commit the bytes were taken from.
    pub source_revision: &'static str,
    /// The path within that repository.
    pub source_path: &'static str,
    /// `sha256:<hex>` over the raw bytes.
    pub sha256: &'static str,
}

/// The semantic-core bundle's origin and recorded digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendoredBundle {
    /// Owning repository, `<org>/<repo>`.
    pub repository: &'static str,
    /// The exact commit the bytes were taken from.
    pub source_revision: &'static str,
    /// The path within that repository.
    pub source_path: &'static str,
    /// The bundle version.
    pub version: &'static str,
    /// `sha256:<hex>` over name + newline + bytes, in sorted file order.
    pub bundle_digest: &'static str,
}

/// The contract this quoin implements.
#[derive(Debug, Clone, Copy)]
pub struct SemanticContract {
    /// The `semantic.contract_version` this quoin understands (FR-070).
    pub contract_version: &'static str,
    /// The `semantic.semantic_core` versions this quoin ships a bundle for.
    pub semantic_core_versions: &'static [&'static str],
    /// The ten admitted `semantic` keys (FR-070).
    pub semantic_keys: &'static [&'static str],
    /// The module-manifest schema, owned by filament-core-service.
    pub module_manifest_schema: VendoredSource,
    /// The semantic-core JSON Schema bundle.
    pub semantic_core: VendoredBundle,
    /// The filament-core-data package-manifest schema.
    pub package_manifest_schema: VendoredSource,
    /// The filament-core-data common schema.
    pub common_schema: VendoredSource,
}

impl SemanticContract {
    /// The contract version, as an id.
    #[must_use]
    pub fn contract_version(&self) -> ContractVersion {
        ContractVersion::from(self.contract_version)
    }

    /// True when `version` is one this quoin ships a semantic-core bundle for.
    #[must_use]
    pub fn ships_semantic_core(&self, version: &SemanticCoreVersion) -> bool {
        self.semantic_core_versions.contains(&version.as_str())
    }
}

/// The pinned contract. Every hash is asserted by the crate's tests.
pub const SEMANTIC_CONTRACT: SemanticContract = SemanticContract {
    contract_version: "1.0.0",
    semantic_core_versions: &["0.1.0"],
    semantic_keys: &[
        "contract_version",
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
    module_manifest_schema: VendoredSource {
        repository: "agent-ix/filament-core-service",
        source_revision: "a77f31efc757f3578ad80d8c7e619897aa3b2513",
        source_path: "filament_core_service/schemas/module-manifest.schema.json",
        sha256: "sha256:69cf9738600e7d8daa45ed5cd7231b17ca8dc58d068bd36af9b0d2c9b69dcbbc",
    },
    semantic_core: VendoredBundle {
        repository: "agent-ix/filament-core-data",
        source_revision: "d48b8da7ae5e40b8b3d465d45b2bd3e24b994dbb",
        source_path: "packages/semantic-core/generated/json-schema",
        version: "0.1.0",
        bundle_digest: "sha256:dd33c886f70e908b14507c35e078d163b76308c3d170d2b54ddf933d1a4ebb52",
    },
    package_manifest_schema: VendoredSource {
        repository: "agent-ix/filament-core-data",
        source_revision: "d48b8da7ae5e40b8b3d465d45b2bd3e24b994dbb",
        source_path: "schema/semantic/v1/package-manifest.schema.json",
        sha256: "sha256:d6e696577f58abd59c36588803c019ad3a43f9a7078c873ad41a0aec41031ffd",
    },
    common_schema: VendoredSource {
        repository: "agent-ix/filament-core-data",
        source_revision: "d48b8da7ae5e40b8b3d465d45b2bd3e24b994dbb",
        source_path: "schema/semantic/v1/common.schema.json",
        sha256: "sha256:1de370f344b099b511960c32ddc98d512218183c13b03350201627bdcba7710a",
    },
};

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

/// `sha256:<hex>` over a file's raw bytes.
///
/// # Errors
///
/// [`SemanticError::VendoredSchemaUnreadable`] when the file cannot be read.
pub fn file_sha256(path: &Path) -> Result<String, SemanticError> {
    let bytes = std::fs::read(path).map_err(|source| SemanticError::VendoredSchemaUnreadable {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(sha256_hex(&bytes))
}

/// `sha256:<hex>` over arbitrary bytes.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

/// The semantic-core bundle digest, computed exactly as filament-core-data does:
/// `name + "\n" + bytes`, in sorted file order, over every `*.json` except
/// `toolchain.json`.
///
/// # Errors
///
/// [`SemanticError::VendoredSchemaUnreadable`] when the directory or one of its
/// files cannot be read.
pub fn semantic_core_bundle_digest(dir: &Path) -> Result<String, SemanticError> {
    let mut names: Vec<String> = Vec::new();
    let entries =
        std::fs::read_dir(dir).map_err(|source| SemanticError::VendoredSchemaUnreadable {
            path: dir.to_path_buf(),
            source,
        })?;
    for entry in entries {
        let entry = entry.map_err(|source| SemanticError::VendoredSchemaUnreadable {
            path: dir.to_path_buf(),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // Case-sensitive on purpose. filament-core-data computes the bundle
        // digest over `name.endsWith(".json")`, and a case-insensitive match
        // here would admit a `FieldDecl.JSON` the upstream digest excluded —
        // i.e. a different digest for the same bytes.
        #[allow(clippy::case_sensitive_file_extension_comparisons)]
        let is_bundle_member = name.ends_with(".json") && name != "toolchain.json";
        if is_bundle_member {
            names.push(name);
        }
    }
    // `readdirSync(...).sort()` in the TypeScript: JavaScript's default sort is
    // by UTF-16 code unit, which agrees with Rust's byte order for the ASCII
    // file names this bundle uses.
    names.sort();

    let mut hasher = Sha256::new();
    for name in &names {
        let path = dir.join(name);
        let bytes =
            std::fs::read(&path).map_err(|source| SemanticError::VendoredSchemaUnreadable {
                path: path.clone(),
                source,
            })?;
        hasher.update(name.as_bytes());
        hasher.update(b"\n");
        hasher.update(&bytes);
    }
    Ok(format!("sha256:{}", hex::encode(hasher.finalize())))
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

    /// Trace: FR-070
    #[test]
    fn tc_378_041_ships_semantic_core_is_exact() {
        assert!(SEMANTIC_CONTRACT.ships_semantic_core(&SemanticCoreVersion::from("0.1.0")));
        assert!(!SEMANTIC_CONTRACT.ships_semantic_core(&SemanticCoreVersion::from("0.1.1")));
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_042_sha256_matches_the_typescript_form() {
        assert_eq!(
            sha256_hex(b""),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
