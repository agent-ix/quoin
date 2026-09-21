// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The semantic-module contract quoin was written against (FR-070, FR-073,
//! FR-075; issue #293).
//!
//! Port of `src/semantic/contract.ts`. One family of schema is quoin's own
//! (`sweep-report.schema.json`); the other two are `agent-ix/filament-core-data`'s
//! module-manifest, package-manifest, common and semantic-core schemas --
//! real dependency edges, not copies, resolved through the published npm
//! packages `@agent-ix/filament-core-data` and `@agent-ix/semantic-core` in
//! this repository's own `node_modules` (PLAT-887 de-vendoring; see
//! `build.rs` for why module-manifest/package-manifest/common are still
//! blocked on that package publishing `schema/semantic/v1/`, and why that is
//! reported rather than bridged). `build.rs` embeds the installed packages'
//! bytes into the compiled binary at their own paths, under the internal
//! names the path helpers below expect, so a release built from this crate
//! stays self-contained with no runtime dependency on `node_modules` existing.
//!
//! [`schema_dir`] and its siblings locate the *embedded* tree, materialized
//! at runtime by [`crate::materialize_embedded_contract`] -- not a directory
//! this crate reads off disk directly. `SEMANTIC_CONTRACT`'s `sha256` and
//! `bundle_digest` fields are compiled assertions: every test in this crate
//! that reads a schema re-derives its digest from the live embedded bytes and
//! compares it here -- that is the actual gate. `source_revision` is not
//! re-derived the same way -- it names the published package version the
//! bytes were embedded from, for humans reading a diagnostic, and the tests
//! only check its shape. For the three still blocked on publish it is a
//! placeholder (see below), because there is no real version to name yet.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::SemanticError;
use crate::ids::{ContractVersion, SemanticCoreVersion};

/// A vendored file's origin: repository, identifying revision, path there,
/// and bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendoredSource {
    /// Owning repository, `<org>/<repo>`.
    pub repository: &'static str,
    /// Informational only -- the compiled assertion is `sha256`. A 40-hex
    /// identifier for the source the bytes came from: the published npm
    /// tarball's SHA-1 shasum where one exists, or 40 zeros as an explicit
    /// placeholder while the source is not yet published (see `build.rs`).
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
    /// Informational only -- the compiled assertion is `bundle_digest`. The
    /// published npm tarball's SHA-1 shasum.
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
    // BLOCKED (PLAT-887): filament-core-data has not yet published
    // `schema/semantic/v1/module-manifest.schema.json` -- see `build.rs`.
    // `source_revision` and `sha256` are explicit 40/64-zero placeholders,
    // not a guess: `SemanticValidators::load` refuses at runtime with
    // `SemanticError::VendoredSchemaUnreadable` until the real file is
    // embedded, and every test that needs its real bytes is `#[ignore]`d
    // with this same reason. Fill in the real values once published.
    module_manifest_schema: VendoredSource {
        repository: "agent-ix/filament-core-data",
        source_revision: "0000000000000000000000000000000000000000",
        source_path: "schema/semantic/v1/module-manifest.schema.json",
        sha256: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    },
    semantic_core: VendoredBundle {
        repository: "agent-ix/filament-core-data",
        source_revision: "03ddad89553e449b645fafc7ce47acaba4504590",
        source_path: "packages/semantic-core/generated/json-schema",
        version: "0.1.0",
        bundle_digest: "sha256:dd33c886f70e908b14507c35e078d163b76308c3d170d2b54ddf933d1a4ebb52",
    },
    // BLOCKED (PLAT-887): see module_manifest_schema above.
    package_manifest_schema: VendoredSource {
        repository: "agent-ix/filament-core-data",
        source_revision: "0000000000000000000000000000000000000000",
        source_path: "schema/semantic/v1/package-manifest.schema.json",
        sha256: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    },
    // BLOCKED (PLAT-887): see module_manifest_schema above.
    common_schema: VendoredSource {
        repository: "agent-ix/filament-core-data",
        source_revision: "0000000000000000000000000000000000000000",
        source_path: "schema/semantic/v1/common.schema.json",
        sha256: "sha256:0000000000000000000000000000000000000000000000000000000000000000",
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
