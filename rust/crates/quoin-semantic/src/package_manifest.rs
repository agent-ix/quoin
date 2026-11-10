// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Derived package manifest (FR-075, issue #293).
//!
//! Port of `src/semantic/package-manifest.ts`. From a module's `semantic` block
//! Quoin derives the filament-core-data `package-manifest` document (FR-021
//! there) so dynamic use and a later generated package share one identity graph.
//!
//! The derived document is a **typed struct**, not a hand-built
//! `serde_json::Value`. The TypeScript builds an object literal, so the only
//! contract on its shape is whichever test happened to assert a key; here the
//! field list is the contract and the validator confirms it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use crate::contract::{COMMON_SCHEMA_URI, common_schema_path, package_manifest_schema_path};
use crate::data_schema::{ref_type_name, schema_base};
use crate::diagnostic::{DiagnosticCode, SemanticDiagnostic};
use crate::error::SemanticError;
use crate::ids::{ObjectTypeName, PackageIdentity};
use crate::manifest::{CompatibilityPosture, SemanticModule};
use crate::schema::{SchemaError, SchemaValidator, SchemaValidatorExt, read_json};

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
    /// The declared version, as written: the module-manifest schema requires
    /// `semantic.imports` values to be exact semantic versions.
    #[serde(rename = "versionConstraint")]
    pub version_constraint: String,
    /// The import's declared `semantic.import_types` list, `[]` when none.
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
    /// Declared imports sorted by identity.
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

    let mut imports: Vec<DerivedImport> = Vec::new();
    // `block.imports` is a BTreeMap, which is already the TypeScript's
    // `localeCompare` order for the `[a-z0-9._-]/[a-z0-9._-]` identities the
    // schema admits — those are ASCII, where byte order and locale order agree.
    imports.extend(block.imports.iter().map(|(identity, version)| {
        DerivedImport {
            package_identity: identity.as_str().to_owned(),
            version_constraint: version.to_string(),
            exports: block
                .import_types
                .get(identity)
                .cloned()
                .unwrap_or_default(),
            capabilities: Vec::new(),
        }
    }));

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
    // `new Map([candidate, ...installed].map(...))`: later entries overwrite
    // earlier ones, so an installed module with the candidate's package wins.
    let mut by_package: BTreeMap<&PackageIdentity, &SemanticModule> = BTreeMap::new();
    by_package.insert(&candidate.block.package, candidate);
    for module in installed {
        by_package.insert(&module.block.package, module);
    }
    let population: Vec<&SemanticModule> = installed.iter().collect();
    resolve_imports_in(candidate, &by_package, &population)
}

/// [`resolve_imports`] against a provider index the caller built once, so that
/// judging every module of a population is one pass over it rather than a copy
/// of it per module (FR-124).
///
/// `by_package` maps a package identity to the module providing it; `population`
/// is every module the unresolved-import message may name versions from.
#[must_use]
pub fn resolve_imports_in<'a>(
    candidate: &'a SemanticModule,
    by_package: &'a BTreeMap<&'a PackageIdentity, &'a SemanticModule>,
    population: &[&SemanticModule],
) -> Vec<SemanticDiagnostic> {
    let mut diagnostics = Vec::new();

    for (identity, version) in &candidate.block.imports {
        let provided = by_package
            .get(identity)
            .is_some_and(|module| &module.version == version);
        if !provided {
            let installed_versions: Vec<&str> = population
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

    diagnostics.extend(import_type_diagnostics(candidate, by_package));

    let mut visitor = CycleVisitor {
        by_package,
        open: BTreeSet::new(),
        done: BTreeSet::new(),
        trail: Vec::new(),
        diagnostics: Vec::new(),
    };
    visitor.visit(&candidate.block.package);
    diagnostics.extend(visitor.diagnostics);
    diagnostics
}

/// `semantic.unknown-import-type` and `semantic.unused-import` for one
/// candidate (FR-070).
///
/// A provider that is not installed at the declared version is the
/// unresolved-import error's business; no unknown-type finding is raised for it.
fn import_type_diagnostics(
    candidate: &SemanticModule,
    by_package: &BTreeMap<&PackageIdentity, &SemanticModule>,
) -> Vec<SemanticDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut reported = BTreeSet::new();
    for (identity, version) in &candidate.block.imports {
        let base = schema_base(identity);
        let provider = by_package
            .get(identity)
            .filter(|module| &module.version == version);
        let urls: Vec<(&str, &str)> = candidate
            .data_schemas
            .values()
            .flat_map(|resolved| {
                resolved
                    .file_refs
                    .iter()
                    .flat_map(|file| file.urls.iter())
                    .filter(|url| url.starts_with(&base))
                    .map(|url| (resolved.locus.as_str(), url.as_str()))
            })
            .collect();

        // A reference-form schema that failed to resolve carries no refs; its
        // own error is the finding, not an unused import.
        let unresolved = candidate
            .data_schemas
            .values()
            .any(|resolved| resolved.schema.is_none());
        if urls.is_empty() && !unresolved {
            diagnostics.push(SemanticDiagnostic::warning(
                DiagnosticCode::UnusedImport,
                format!("semantic.imports.{identity}"),
                format!("import {identity} is referenced by no $ref in the module's data schemas"),
            ));
        }
        let Some(provider) = provider else {
            continue;
        };
        let exported = |name: &str| provider.block.exports.iter().any(|e| e.as_str() == name);

        for name in candidate
            .block
            .import_types
            .get(identity)
            .into_iter()
            .flatten()
        {
            if !exported(name) {
                diagnostics.push(SemanticDiagnostic::error(
                    DiagnosticCode::UnknownImportType,
                    format!("semantic.import_types.{identity}"),
                    format!("{identity}@{version} does not export type {name}"),
                ));
            }
        }
        for (locus, url) in urls {
            let name = ref_type_name(url);
            if !exported(name) && reported.insert((locus, identity, name)) {
                diagnostics.push(SemanticDiagnostic::error(
                    DiagnosticCode::UnknownImportType,
                    format!("{locus}.schema"),
                    format!(
                        "$ref {url} names type {name}, which {identity}@{version} does not export"
                    ),
                ));
            }
        }
    }
    diagnostics
}

/// The type-level import graph over `modules` (FR-075): nodes
/// `<org>/<repo>#<Type>`, one edge per `$ref` of a module's own file into an
/// imported package's type file.
fn type_graph(
    by_package: &BTreeMap<&PackageIdentity, &SemanticModule>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for module in by_package.values() {
        let package = &module.block.package;
        let bases: Vec<(&PackageIdentity, String)> = module
            .block
            .imports
            .keys()
            .map(|import| (import, schema_base(import)))
            .collect();
        for file in module
            .data_schemas
            .values()
            .flat_map(|resolved| resolved.file_refs.iter())
        {
            for url in &file.urls {
                if let Some((import, _)) = bases.iter().find(|(_, base)| url.starts_with(base)) {
                    graph
                        .entry(format!("{package}#{}", file.type_name))
                        .or_default()
                        .insert(format!("{import}#{}", ref_type_name(url)));
                }
            }
        }
    }
    graph
}

/// A cycle among the type-level edges that run along the package `cycle` (a
/// closed path of package identities), as a closed node path, when there is one.
fn type_cycle(
    by_package: &BTreeMap<&PackageIdentity, &SemanticModule>,
    cycle: &[&str],
) -> Option<Vec<String>> {
    let package_of = |node: &str| node.split('#').next().unwrap_or_default().to_owned();
    let along: BTreeSet<(&str, &str)> = cycle
        .iter()
        .copied()
        .zip(cycle.iter().copied().skip(1))
        .collect();
    let on_cycle = |from: &str, to: &str| {
        let (from, to) = (package_of(from), package_of(to));
        along.contains(&(from.as_str(), to.as_str()))
    };
    let graph: BTreeMap<String, Vec<String>> = type_graph(by_package)
        .into_iter()
        .map(|(node, targets)| {
            let kept = targets
                .into_iter()
                .filter(|t| on_cycle(&node, t))
                .collect::<Vec<_>>();
            (node, kept)
        })
        .collect();

    let mut done: BTreeSet<&str> = BTreeSet::new();
    for start in graph.keys() {
        if done.contains(start.as_str()) {
            continue;
        }
        // Iterative depth-first search: `path` is the open chain, each frame
        // holding the next child index to try.
        let mut path: Vec<(&str, usize)> = vec![(start.as_str(), 0)];
        while let Some(&(node, next)) = path.last() {
            let child = graph.get(node).and_then(|targets| targets.get(next));
            if let Some(frame) = path.last_mut() {
                frame.1 += 1;
            }
            match child {
                None => {
                    done.insert(node);
                    path.pop();
                }
                Some(child) => {
                    if let Some(at) = path.iter().position(|(open, _)| *open == child.as_str()) {
                        let mut cycle: Vec<String> =
                            path.iter().skip(at).map(|(n, _)| (*n).to_owned()).collect();
                        cycle.push(child.clone());
                        return Some(cycle);
                    }
                    if !done.contains(child.as_str()) {
                        path.push((child.as_str(), 0));
                    }
                }
            }
        }
    }
    None
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
            // The cycle fails either way; the type-level graph only decides how
            // it is named (FR-075).
            let named = type_cycle(self.by_package, &cycle)
                .map_or_else(|| cycle.join(" -> "), |nodes| nodes.join(" -> "));
            self.diagnostics.push(SemanticDiagnostic::error(
                DiagnosticCode::ImportCycle,
                "semantic.imports",
                format!("import cycle: {named}"),
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
    use ix_trace_rs::trace;

    #[trace("FR-075")]
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

    #[trace("FR-075")]
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
