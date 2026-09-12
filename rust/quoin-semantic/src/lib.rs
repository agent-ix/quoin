// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Agent-IX

//! The semantic-module contract (quoin FR-070, FR-073, FR-074, FR-075).
//!
//! This crate is the Rust port of `src/semantic/` (issue #378, EPIC #373
//! Stage 3). It owns four things and nothing else:
//!
//! - [`contract`] — the vendored schema bundle and its recorded provenance.
//! - [`manifest`] — reading and refusing a module manifest's `semantic` block.
//! - [`data_schema`] — resolving an object type's `data_schema` reference.
//! - [`package_manifest`] — the derived filament-core-data package manifest,
//!   its registry pin, and import resolution.
//! - [`sweep`] — the legacy Properties-form classifier and corpus sweep.
//!
//! # Parity with the TypeScript it replaces
//!
//! The TypeScript in `src/semantic/` is the oracle, and it was consulted
//! **once**: `scripts/capture-semantic-goldens.mjs` recorded every verdict and
//! every diagnostic into `tests/goldens/`, with the quoin revision it ran at.
//! The tests in `tests/` read those files. Nothing here invokes Node.
//!
//! Schema validation moves from `ajv` to the Rust `jsonschema` crate. Verdicts
//! are contractual and measured at exact parity over the golden corpus; error
//! *text* is not, and the two libraries' diagnostic shape, coverage and
//! ordering differ in named, measured ways recorded in `DIVERGENCE.md`.
//! Read that document before changing anything in [`schema`].

#![forbid(unsafe_code)]

pub mod contract;
pub mod data_schema;
pub mod diagnostic;
pub mod error;
pub mod ids;
pub mod manifest;
pub mod package_manifest;
pub mod schema;
pub mod sweep;

pub use contract::{SemanticContract, VendoredSource, SEMANTIC_CONTRACT};
pub use data_schema::{
    classify_data_schema, resolve_data_schema, DataSchemaForm, ResolveContext, ResolvedDataSchema,
};
pub use diagnostic::{DiagnosticCode, SemanticDiagnostic, Severity};
pub use error::{SemanticError, SemanticErrorCode};
pub use ids::{
    ContractVersion, MappingName, ModuleName, ModuleVersion, ObjectTypeName, PackageIdentity,
    SemanticCoreVersion,
};
pub use manifest::{
    duplicate_package_diagnostic, format_diagnostics, has_errors, read_module_semantic,
    read_semantic_block, CompatibilityPosture, LegacyForms, SemanticBlock, SemanticModule,
    SemanticReadResult,
};
pub use package_manifest::{
    derive_package_manifest, export_digests, mapping_identity, registry_pin, resolve_imports,
    type_identity, validate_package_manifest, SemanticRegistryPin, SEMANTIC_CORE_PACKAGE,
};
pub use sweep::{
    classify_artifact, classify_properties, sweep_corpus, CorpusRoot, FormFinding,
    LegacyFormDiagnostic, PropertiesForm, SweepIdentity, SweepReport, LEGACY_MIGRATION_EXAMPLE,
    TYPED_HEADER,
};
