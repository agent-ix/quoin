// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `catalog`: the installed artifact/object type projection (quoin#373,
//! Stage 8).
//!
//! `quoin-catalog` owns every manifest decision. This boundary module owns the
//! command-shaped request, resource bounds, host grants, and semantic-result
//! pairing. It opens no file: a [`CatalogHost`] locates and reads module roots,
//! while [`SemanticHost`] judges their optional semantic blocks in one batch.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use quoin_catalog::{Catalog, CatalogError, SemanticView};
use serde::Deserialize;

use crate::capabilities::{Capabilities, CatalogHost, SemanticHost};
use crate::error::{CoreError, CoreErrorCode};
use crate::ops::{refusal, request_size};
use crate::protocol::Response;

/// Largest catalog request this operation will decide over, in bytes.
pub const MAX_LOAD_BYTES: usize = 1 << 20;
/// Largest explicit root path this operation accepts, in bytes.
pub const MAX_ROOT_BYTES: usize = 4 * 1024;

/// Request accepted by `catalog.load`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LoadRequest {
    /// Explicit module candidates, or absent for the host's default discovery.
    #[serde(default)]
    pub roots: Option<Vec<String>>,
}

/// Answer `catalog.load`.
///
/// The payload is the catalog itself, rather than an extra `{ catalog: ... }`
/// envelope: that is the shape the retained `loadCatalog()` already exposes to
/// every caller and lets its direct successor be a one-line boundary call.
///
/// # Errors
/// Returns a request/refusal/internal [`CoreError`] when a request is malformed
/// or oversized, a required host is missing, a module cannot be read, semantic
/// validation cannot run, or a manifest is invalid YAML.
pub fn load(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let size = request_size(request)?;
    if size > MAX_LOAD_BYTES {
        return Err(refusal("catalog.load", MAX_LOAD_BYTES, size));
    }
    let request: LoadRequest = serde_json::from_value(request.clone()).map_err(|error| {
        CoreError::new(CoreErrorCode::BadRequest, error.to_string())
            .with_context("op", "catalog.load")
    })?;
    if let Some(roots) = &request.roots {
        for root in roots {
            if root.len() > MAX_ROOT_BYTES {
                return Err(CoreError::new(
                    CoreErrorCode::Refused,
                    "catalog root exceeds the accepted size",
                )
                .with_context("op", "catalog.load")
                .with_context("field", "roots")
                .with_context("limit_bytes", MAX_ROOT_BYTES.to_string())
                .with_context("observed_bytes", root.len().to_string()));
            }
        }
    }

    let catalog_host = catalog_host(capabilities)?;
    let roots: Option<Vec<PathBuf>> = request
        .roots
        .as_ref()
        .map(|roots| roots.iter().map(PathBuf::from).collect());
    let documents = catalog_host
        .read_modules(roots.as_deref())
        .map_err(|error| {
            CoreError::new(CoreErrorCode::Io, error.to_string()).with_context("op", "catalog.load")
        })?;

    // The legacy client suppressed duplicate roots and module names before it
    // touched semantic input. Preserve that observable failure boundary: an
    // ignored duplicate must not make an otherwise valid catalog fail.
    let retained = quoin_catalog::build(&documents, &[]).map_err(map_catalog_error)?;
    let retained_roots: BTreeSet<&str> = retained
        .modules
        .iter()
        .map(|module| module.root.as_str())
        .collect();
    let semantic_host = semantic_host(capabilities)?;
    let mut semantics = Vec::with_capacity(documents.len());
    for document in &documents {
        if !retained_roots.contains(document.root.as_str()) {
            continue;
        }
        let result = semantic_host
            .read_module(Path::new(&document.root))
            .map_err(|error| {
                CoreError::new(CoreErrorCode::Refused, error.to_string())
                    .with_context("op", "catalog.load")
                    .with_context("root", document.root.clone())
            })?;
        semantics.push(SemanticView {
            root: document.root.clone(),
            block: result.module.map(|module| module.block),
            diagnostics: result.diagnostics,
        });
    }
    let catalog = quoin_catalog::build(&documents, &semantics).map_err(map_catalog_error)?;
    ok(&catalog)
}

/// Answer `catalog.methods`.
///
/// This is deliberately a separate command-shaped operation rather than a
/// client-side reimplementation of the method merge. The auditor and the
/// command must consume the same first-wins catalog (quire-rs FR-054).
///
/// # Errors
/// Returns a request/refusal/internal [`CoreError`] when the request is
/// malformed or oversized, a root exceeds its byte ceiling, or no catalog host
/// was granted.
pub fn methods(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let size = request_size(request)?;
    if size > MAX_LOAD_BYTES {
        return Err(refusal("catalog.methods", MAX_LOAD_BYTES, size));
    }
    let request: LoadRequest = serde_json::from_value(request.clone()).map_err(|error| {
        CoreError::new(CoreErrorCode::BadRequest, error.to_string())
            .with_context("op", "catalog.methods")
    })?;
    if let Some(roots) = &request.roots {
        for root in roots {
            if root.len() > MAX_ROOT_BYTES {
                return Err(CoreError::new(
                    CoreErrorCode::Refused,
                    "catalog root exceeds the accepted size",
                )
                .with_context("op", "catalog.methods")
                .with_context("field", "roots")
                .with_context("limit_bytes", MAX_ROOT_BYTES.to_string())
                .with_context("observed_bytes", root.len().to_string()));
            }
        }
    }
    let roots: Option<Vec<PathBuf>> = request
        .roots
        .as_ref()
        .map(|roots| roots.iter().map(PathBuf::from).collect());
    let catalog = catalog_host(capabilities)?.load_method_catalog(roots.as_deref());
    serde_json::to_value(catalog)
        .map(Response::ok)
        .map_err(|error| CoreError::new(CoreErrorCode::Io, error.to_string()))
}

fn catalog_host<'a>(capabilities: &'a Capabilities<'a>) -> Result<&'a dyn CatalogHost, CoreError> {
    capabilities.catalog.ok_or_else(|| {
        CoreError::new(
            CoreErrorCode::Io,
            "catalog operation was dispatched without a catalog host",
        )
        .with_context("op", "catalog.load")
    })
}

fn semantic_host<'a>(
    capabilities: &'a Capabilities<'a>,
) -> Result<&'a dyn SemanticHost, CoreError> {
    capabilities.semantic.ok_or_else(|| {
        CoreError::new(
            CoreErrorCode::Io,
            "catalog operation was dispatched without a semantic host",
        )
        .with_context("op", "catalog.load")
    })
}

fn map_catalog_error(error: CatalogError) -> CoreError {
    match error {
        CatalogError::ManifestYaml { root, detail } => {
            CoreError::new(CoreErrorCode::Refused, detail)
                .with_context("op", "catalog.load")
                .with_context("root", root)
        }
    }
}

fn ok(catalog: &Catalog) -> Result<Response, CoreError> {
    serde_json::to_value(catalog)
        .map(Response::ok)
        .map_err(|error| CoreError::new(CoreErrorCode::Io, error.to_string()))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use std::path::{Path, PathBuf};

    use quoin_auditor::MethodCatalog;
    use quoin_catalog::ModuleDocument;
    use quoin_semantic::{SemanticError, SemanticReadResult};
    use serde_json::json;

    use super::{load, methods};
    use crate::capabilities::{Capabilities, CatalogHost, SemanticHost};

    struct CatalogFixture;

    impl CatalogHost for CatalogFixture {
        fn read_modules(&self, roots: Option<&[PathBuf]>) -> std::io::Result<Vec<ModuleDocument>> {
            assert!(roots.is_none());
            Ok(vec![ModuleDocument {
                root: "/modules/example".to_owned(),
                manifest: "name: example\nartifact_types: [{name: FR}]\n".to_owned(),
                skeleton_names: vec!["FR.md".to_owned()],
            }])
        }

        fn load_method_catalog(&self, roots: Option<&[PathBuf]>) -> MethodCatalog {
            assert!(roots.is_none());
            MethodCatalog::default()
        }
    }

    struct SemanticFixture;

    impl SemanticHost for SemanticFixture {
        fn read_module(&self, _module_root: &Path) -> Result<SemanticReadResult, SemanticError> {
            Ok(SemanticReadResult {
                module: None,
                diagnostics: Vec::new(),
            })
        }

        fn sweep(
            &self,
            _roots: &[quoin_semantic::CorpusRoot],
            _identity: &quoin_semantic::SweepIdentity,
            _generated_at: &str,
        ) -> Result<quoin_semantic::SweepReport, SemanticError> {
            unreachable!("catalog does not sweep corpora")
        }
    }

    struct DuplicateCatalogFixture;

    impl CatalogHost for DuplicateCatalogFixture {
        fn read_modules(&self, _roots: Option<&[PathBuf]>) -> std::io::Result<Vec<ModuleDocument>> {
            Ok(vec![
                ModuleDocument {
                    root: "/modules/kept".to_owned(),
                    manifest: "name: shared\nartifact_types: [{name: FR}]\n".to_owned(),
                    skeleton_names: Vec::new(),
                },
                ModuleDocument {
                    root: "/modules/ignored".to_owned(),
                    manifest: "name: shared\nartifact_types: [{name: Invalid}]\n".to_owned(),
                    skeleton_names: Vec::new(),
                },
            ])
        }

        fn load_method_catalog(&self, _roots: Option<&[PathBuf]>) -> MethodCatalog {
            MethodCatalog::default()
        }
    }

    struct RejectIgnoredSemanticFixture;

    struct MethodCatalogFixture;

    impl CatalogHost for MethodCatalogFixture {
        fn read_modules(&self, _roots: Option<&[PathBuf]>) -> std::io::Result<Vec<ModuleDocument>> {
            Ok(Vec::new())
        }

        fn load_method_catalog(&self, roots: Option<&[PathBuf]>) -> MethodCatalog {
            assert!(roots.is_none());
            serde_json::from_value(json!({
                "methods": [{
                    "id": "analysis",
                    "name": "Analysis",
                    "class": "Analysis",
                    "definition": "inspect",
                    "applicability": {},
                    "tooling": [],
                    "moduleName": "fixture"
                }],
                "duplicates": [],
                "unreadable": []
            }))
            .expect("fixture is a method catalog")
        }
    }

    impl SemanticHost for RejectIgnoredSemanticFixture {
        fn read_module(&self, module_root: &Path) -> Result<SemanticReadResult, SemanticError> {
            assert_ne!(module_root, Path::new("/modules/ignored"));
            Ok(SemanticReadResult {
                module: None,
                diagnostics: Vec::new(),
            })
        }

        fn sweep(
            &self,
            _roots: &[quoin_semantic::CorpusRoot],
            _identity: &quoin_semantic::SweepIdentity,
            _generated_at: &str,
        ) -> Result<quoin_semantic::SweepReport, SemanticError> {
            unreachable!("catalog does not sweep corpora")
        }
    }

    /// Trace: FR-101
    #[test]
    fn tc_373_catalog_methods_uses_the_granted_first_wins_projection() {
        let catalog = MethodCatalogFixture;
        let response = methods(&json!({}), &Capabilities::with_catalog(&catalog))
            .expect("method catalog loads");
        assert_eq!(
            response.payload,
            json!({
                "methods": [{
                    "id": "analysis",
                    "name": "Analysis",
                    "class": "Analysis",
                    "definition": "inspect",
                    "applicability": {},
                    "tooling": [],
                    "moduleName": "fixture"
                }],
                "duplicates": [],
                "unreadable": []
            })
        );
    }

    /// Trace: FR-101
    #[test]
    fn tc_373_catalog_load_is_a_single_boundary_projection() {
        let catalog = CatalogFixture;
        let semantic = SemanticFixture;
        let response = load(
            &json!({}),
            &Capabilities {
                modules: None,
                catalog: Some(&catalog),
                semantic: Some(&semantic),
                change_assurance: None,
                evidence: None,
                graph: None,
                quire: None,
            },
        )
        .expect("catalog loads");
        assert_eq!(
            response.payload.pointer("/modules/0/name"),
            Some(&json!("example"))
        );
        assert_eq!(
            response.payload.pointer("/entries/0/skeletonPath"),
            Some(&json!("/modules/example/skeletons/FR.md"))
        );
    }

    /// Trace: FR-101
    #[test]
    fn tc_373_catalog_does_not_semantically_read_suppressed_duplicates() {
        let catalog = DuplicateCatalogFixture;
        let semantic = RejectIgnoredSemanticFixture;
        let response = load(
            &json!({}),
            &Capabilities {
                modules: None,
                catalog: Some(&catalog),
                semantic: Some(&semantic),
                change_assurance: None,
                evidence: None,
                graph: None,
                quire: None,
            },
        )
        .expect("the duplicate module is ignored before semantic reading");
        assert_eq!(
            response.payload.pointer("/modules/0/root"),
            Some(&json!("/modules/kept"))
        );
    }
}
