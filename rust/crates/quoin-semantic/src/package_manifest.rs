// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Derived package manifest and registry pins (FR-075, issue #293).
//!
//! Port of `src/semantic/package-manifest.ts`. From a module's `semantic` block
//! Quoin derives the filament-core-data `package-manifest` document (FR-021
//! there) and records one schema digest per exported object type, so dynamic use
//! and a later generated package share one identity graph.
//!
//! The derived document is a **typed struct**, not a hand-built
//! `serde_json::Value`. The TypeScript builds an object literal, so the only
//! contract on its shape is whichever test happened to assert a key; here the
//! field list is the contract and the validator confirms it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use crate::contract::{
    COMMON_SCHEMA_URI, common_schema_path, file_sha256, package_manifest_schema_path,
};
use crate::data_schema::DataSchemaForm;
use crate::diagnostic::{DiagnosticCode, SemanticDiagnostic};
use crate::error::SemanticError;
use crate::ids::{ObjectTypeName, PackageIdentity};
use crate::manifest::{CompatibilityPosture, SemanticModule};
use crate::schema::{SchemaError, SchemaValidator, read_json};

/// The semantic-core package every derived manifest imports.
pub const SEMANTIC_CORE_PACKAGE: &str = "agent-ix/semantic-core";

/// `ix://<org>/<repo>/type/<name>`.
#[must_use]
pub fn type_identity(package: &PackageIdentity, name: &ObjectTypeName) -> String {
    format!("ix://{}/{}/type/{name}", package.org(), package.repo())
}

/// `ix://<org>/<repo>/mapping/<name>`.
#[must_use]
pub fn mapping_identity(package: &PackageIdentity, name: &str) -> String {
    format!("ix://{}/{}/mapping/{name}", package.org(), package.repo())
}

/// One `imports[]` entry of the derived manifest.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DerivedImport {
    /// The imported package's identity.
    #[serde(rename = "packageIdentity")]
    pub package_identity: String,
    /// `=<version>`: the contract admits only exact pins.
    #[serde(rename = "versionConstraint")]
    pub version_constraint: String,
    /// Always empty: the `semantic` block does not name per-import exports.
    pub exports: Vec<String>,
    /// Always empty: the `semantic` block does not name capabilities.
    pub capabilities: Vec<String>,
}

/// One `exports[]` entry of the derived manifest.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DerivedExport {
    /// The object type's name.
    pub name: String,
    /// Its `ix://` identity.
    #[serde(rename = "typeIdentity")]
    pub type_identity: String,
    /// Always `public`: the `semantic` block has no private form.
    pub visibility: &'static str,
}

/// The single derived profile.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DerivedProfile {
    /// Always `default`.
    pub name: &'static str,
    /// The module's version.
    pub version: String,
    /// Exported type names, sorted.
    pub exports: Vec<String>,
    /// Declared targets, in manifest order.
    pub targets: Vec<String>,
    /// Mapping identities, sorted by mapping name.
    pub mappings: Vec<String>,
    /// Always empty.
    pub options: BTreeMap<String, Value>,
    /// The block's posture.
    #[serde(rename = "compatibilityPosture")]
    pub compatibility_posture: CompatibilityPosture,
}

/// The package identity block.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DerivedPackage {
    /// `<org>/<repo>`.
    pub identity: String,
    /// The module's version.
    pub version: String,
}

/// The derived filament-core-data package manifest (FR-075).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DerivedPackageManifest {
    /// Always `1.0.0`.
    #[serde(rename = "contractVersion")]
    pub contract_version: &'static str,
    /// Package identity and version.
    pub package: DerivedPackage,
    /// Always the 2020-12 dialect URI.
    #[serde(rename = "schemaDialect")]
    pub schema_dialect: &'static str,
    /// Always `["schemas/"]`.
    #[serde(rename = "sourceRoots")]
    pub source_roots: Vec<&'static str>,
    /// semantic-core first, then declared imports sorted by identity.
    pub imports: Vec<DerivedImport>,
    /// Exports, sorted by name.
    pub exports: Vec<DerivedExport>,
    /// The single `default` profile.
    pub profiles: Vec<DerivedProfile>,
    /// Declared targets, in manifest order.
    pub targets: Vec<String>,
    /// Mapping identities, sorted by mapping name.
    pub mappings: Vec<String>,
    /// Always empty.
    pub extensions: Vec<Value>,
}

impl DerivedPackageManifest {
    /// The manifest as JSON, for validation and for writing.
    ///
    /// # Errors
    ///
    /// Never in practice: every field is a plain scalar or collection. The
    /// `Result` is `serde_json`'s, not a modelled failure.
    pub fn to_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

/// Derive the filament-core-data package manifest for one semantic module.
#[must_use]
pub fn derive_package_manifest(module: &SemanticModule) -> DerivedPackageManifest {
    let block = &module.block;

    let mut imports = vec![DerivedImport {
        package_identity: SEMANTIC_CORE_PACKAGE.to_owned(),
        version_constraint: format!("={}", block.semantic_core),
        exports: Vec::new(),
        capabilities: Vec::new(),
    }];
    // `block.imports` is a BTreeMap, which is already the TypeScript's
    // `localeCompare` order for the `[a-z0-9._-]/[a-z0-9._-]` identities the
    // schema admits — those are ASCII, where byte order and locale order agree.
    imports.extend(
        block
            .imports
            .iter()
            .map(|(identity, version)| DerivedImport {
                package_identity: identity.as_str().to_owned(),
                version_constraint: format!("={version}"),
                exports: Vec::new(),
                capabilities: Vec::new(),
            }),
    );

    let mut mapping_names: Vec<&str> = block
        .mappings
        .iter()
        .map(crate::ids::MappingName::as_str)
        .collect();
    mapping_names.sort_unstable();
    let mappings: Vec<String> = mapping_names
        .iter()
        .map(|name| mapping_identity(&block.package, name))
        .collect();

    let mut export_names: Vec<&ObjectTypeName> = block.exports.iter().collect();
    export_names.sort_unstable();
    let exports: Vec<DerivedExport> = export_names
        .iter()
        .map(|name| DerivedExport {
            name: name.as_str().to_owned(),
            type_identity: type_identity(&block.package, name),
            visibility: "public",
        })
        .collect();

    let profile = DerivedProfile {
        name: "default",
        version: module.version.as_str().to_owned(),
        exports: exports.iter().map(|e| e.name.clone()).collect(),
        targets: block.targets.clone(),
        mappings: mappings.clone(),
        options: BTreeMap::new(),
        compatibility_posture: block.compatibility_posture,
    };

    DerivedPackageManifest {
        contract_version: "1.0.0",
        package: DerivedPackage {
            identity: block.package.as_str().to_owned(),
            version: module.version.as_str().to_owned(),
        },
        schema_dialect: "https://json-schema.org/draft/2020-12/schema",
        source_roots: vec!["schemas/"],
        imports,
        exports,
        profiles: vec![profile],
        targets: block.targets.clone(),
        mappings,
        extensions: Vec::new(),
    }
}

/// The compiled filament-core-data package-manifest validator.
#[derive(Debug)]
pub struct PackageManifestValidator {
    validator: SchemaValidator,
}

impl PackageManifestValidator {
    /// Compile from the vendored tree rooted at `semantic_root`.
    ///
    /// `common.schema.json` is registered under its own `$id`, which is what the
    /// package-manifest schema's relative `common.schema.json#/$defs/…`
    /// references resolve against — the Rust equivalent of ajv's `addSchema`.
    ///
    /// # Errors
    ///
    /// Any of the `VendoredSchema*` conditions in [`SemanticError`].
    pub fn load(semantic_root: &Path) -> Result<Self, SemanticError> {
        let schema_path = package_manifest_schema_path(semantic_root);
        let schema = read_json(&schema_path)?;
        let common = read_json(&common_schema_path(semantic_root))?;
        Ok(Self {
            validator: SchemaValidator::compile(
                &schema_path,
                &schema,
                &[(COMMON_SCHEMA_URI, common)],
            )?,
        })
    }

    /// Validate a manifest document.
    ///
    /// # Errors
    ///
    /// The schema errors, when the document does not satisfy the schema. This
    /// is a *verdict*, not a failure of the validator.
    pub fn validate(&self, manifest: &Value) -> Result<(), Vec<SchemaError>> {
        if self.validator.is_valid(manifest) {
            Ok(())
        } else {
            Err(self.validator.errors(manifest))
        }
    }

    /// The verdict alone.
    #[must_use]
    pub fn is_valid(&self, manifest: &Value) -> bool {
        self.validator.is_valid(manifest)
    }
}

/// Validate a derived manifest against the vendored filament-core-data schema.
///
/// The free function mirrors the TypeScript's `validatePackageManifest`; a
/// caller validating more than one manifest should hold a
/// [`PackageManifestValidator`] instead of paying for compilation each time.
///
/// # Errors
///
/// Any of the `VendoredSchema*` conditions in [`SemanticError`].
pub fn validate_package_manifest(
    semantic_root: &Path,
    manifest: &Value,
) -> Result<Result<(), Vec<SchemaError>>, SemanticError> {
    Ok(PackageManifestValidator::load(semantic_root)?.validate(manifest))
}

/// One sha256 per exported object type's referenced schema (FR-075-AC-2).
///
/// An export whose `data_schema` is not a resolved reference contributes no
/// entry — the `semantic.export-without-schema` diagnostic has already reported
/// it, and inventing a digest for it would pin nothing.
///
/// # Errors
///
/// [`SemanticError::VendoredSchemaUnreadable`] when a referenced file that
/// resolved cannot be re-read.
pub fn export_digests(module: &SemanticModule) -> Result<BTreeMap<String, String>, SemanticError> {
    let mut names: Vec<&ObjectTypeName> = module.block.exports.iter().collect();
    names.sort_unstable();
    let mut digests = BTreeMap::new();
    for name in names {
        let Some(resolved) = module.data_schemas.get(name) else {
            continue;
        };
        if resolved.kind != DataSchemaForm::Reference {
            continue;
        }
        let Some(file) = resolved.file.as_deref() else {
            continue;
        };
        digests.insert(name.as_str().to_owned(), file_sha256(file)?);
    }
    Ok(digests)
}

/// The registry pin recorded under a plugin entry's `semantic` key.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SemanticRegistryPin {
    /// `<org>/<repo>`.
    pub package: String,
    /// The semantic-core version the module compiles against.
    #[serde(rename = "semanticCore")]
    pub semantic_core: String,
    /// One digest per exported type, by type name.
    pub exports: BTreeMap<String, String>,
}

/// The registry pin for one module.
///
/// # Errors
///
/// [`SemanticError::VendoredSchemaUnreadable`] when a referenced schema file
/// cannot be re-read.
pub fn registry_pin(module: &SemanticModule) -> Result<SemanticRegistryPin, SemanticError> {
    Ok(SemanticRegistryPin {
        package: module.block.package.as_str().to_owned(),
        semantic_core: module.block.semantic_core.as_str().to_owned(),
        exports: export_digests(module)?,
    })
}

/// Write `<module root>/semantic/package-manifest.json`; returns the path.
///
/// # Errors
///
/// [`SemanticError::VendoredSchemaUnreadable`] carrying the write failure — the
/// path is the file that could not be produced.
pub fn write_package_manifest(
    module: &SemanticModule,
    manifest: &DerivedPackageManifest,
) -> Result<std::path::PathBuf, SemanticError> {
    let dir = module.root.join("semantic");
    std::fs::create_dir_all(&dir).map_err(|source| SemanticError::VendoredSchemaUnreadable {
        path: dir.clone(),
        source,
    })?;
    let path = dir.join("package-manifest.json");
    let value = manifest
        .to_value()
        .map_err(|source| SemanticError::VendoredSchemaNotJson {
            path: path.clone(),
            source,
        })?;
    let mut text = serde_json::to_string_pretty(&value).map_err(|source| {
        SemanticError::VendoredSchemaNotJson {
            path: path.clone(),
            source,
        }
    })?;
    text.push('\n');
    std::fs::write(&path, text).map_err(|source| SemanticError::VendoredSchemaUnreadable {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

/// Resolve `semantic.imports` against the installed semantic modules (FR-075):
/// every import must be provided at exactly that version, and the import graph
/// must be acyclic.
#[must_use]
pub fn resolve_imports(
    candidate: &SemanticModule,
    installed: &[SemanticModule],
) -> Vec<SemanticDiagnostic> {
    let mut diagnostics = Vec::new();

    // `new Map([candidate, ...installed].map(...))`: later entries overwrite
    // earlier ones, so an installed module with the candidate's package wins.
    let mut by_package: BTreeMap<&PackageIdentity, &SemanticModule> = BTreeMap::new();
    by_package.insert(&candidate.block.package, candidate);
    for module in installed {
        by_package.insert(&module.block.package, module);
    }

    for (identity, version) in &candidate.block.imports {
        let provided = by_package
            .get(identity)
            .is_some_and(|module| &module.version == version);
        if !provided {
            let installed_versions: Vec<&str> = installed
                .iter()
                .filter(|module| &module.block.package == identity)
                .map(|module| module.version.as_str())
                .collect();
            let shown = if installed_versions.is_empty() {
                "none".to_owned()
            } else {
                installed_versions.join(", ")
            };
            diagnostics.push(SemanticDiagnostic::error(
                DiagnosticCode::ImportUnresolved,
                format!("semantic.imports.{identity}"),
                format!(
                    "import {identity}@{version} is provided by no installed module \
                     (installed: {shown})"
                ),
            ));
        }
    }

    let mut visitor = CycleVisitor {
        by_package: &by_package,
        open: BTreeSet::new(),
        done: BTreeSet::new(),
        trail: Vec::new(),
        diagnostics: Vec::new(),
    };
    visitor.visit(&candidate.block.package);
    diagnostics.extend(visitor.diagnostics);
    diagnostics
}

/// Depth-first cycle detection over the import graph rooted at the candidate.
struct CycleVisitor<'a> {
    by_package: &'a BTreeMap<&'a PackageIdentity, &'a SemanticModule>,
    open: BTreeSet<&'a PackageIdentity>,
    done: BTreeSet<&'a PackageIdentity>,
    trail: Vec<&'a PackageIdentity>,
    diagnostics: Vec<SemanticDiagnostic>,
}

impl<'a> CycleVisitor<'a> {
    fn visit(&mut self, identity: &'a PackageIdentity) {
        let Some((key, module)) = self.by_package.get_key_value(identity) else {
            return;
        };
        let key: &'a PackageIdentity = key;
        if self.open.contains(key) {
            let start = self.trail.iter().position(|p| *p == key).unwrap_or(0);
            let mut cycle: Vec<&str> = self
                .trail
                .get(start..)
                .unwrap_or_default()
                .iter()
                .map(|p| p.as_str())
                .collect();
            cycle.push(key.as_str());
            self.diagnostics.push(SemanticDiagnostic::error(
                DiagnosticCode::ImportCycle,
                "semantic.imports",
                format!("import cycle: {}", cycle.join(" -> ")),
            ));
            return;
        }
        if self.done.contains(key) {
            return;
        }
        self.open.insert(key);
        self.trail.push(key);
        // `Object.keys(module.block.imports)` order. A BTreeMap gives sorted
        // order instead of insertion order; the *set* of cycles found is the
        // same, and the trail a cycle prints can differ where a module has more
        // than one import that both close cycles. Recorded in DIVERGENCE.md §5.
        let dependencies: Vec<&'a PackageIdentity> = module.block.imports.keys().collect();
        for dependency in dependencies {
            self.visit(dependency);
        }
        self.trail.pop();
        self.open.remove(key);
        self.done.insert(key);
    }
}

#[cfg(test)]
// Indexing and `unreachable!` are a test-only convenience: an out-of-range
// index in a test is a failing test, not a downed worker.
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Trace: FR-075
    #[test]
    fn tc_378_070_type_identity_is_ix_scheme() {
        assert_eq!(
            type_identity(
                &PackageIdentity::from("agent-ix/spec-objects"),
                &ObjectTypeName::from("entity")
            ),
            "ix://agent-ix/spec-objects/type/entity"
        );
    }

    /// Trace: FR-075
    #[test]
    fn tc_378_071_mapping_identity_is_ix_scheme() {
        assert_eq!(
            mapping_identity(
                &PackageIdentity::from("agent-ix/spec-objects"),
                "config-version"
            ),
            "ix://agent-ix/spec-objects/mapping/config-version"
        );
    }
}
