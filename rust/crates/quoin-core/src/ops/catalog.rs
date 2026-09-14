// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `catalog`: the installed artifact/object type projection (quoin#373,
//! Stage 8).
//!
//! `quoin-catalog` owns every manifest decision. This boundary module owns the
//! command-shaped request, resource bounds, host grants, and semantic-result
//! pairing. It opens no file: a [`CatalogHost`] locates and reads module roots,
//! while [`SemanticHost`] judges their optional semantic blocks in one batch.

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

    let semantic_host = semantic_host(capabilities)?;
    let mut semantics = Vec::with_capacity(documents.len());
    for document in &documents {
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

    use quoin_catalog::ModuleDocument;
    use quoin_semantic::{SemanticError, SemanticReadResult};
    use serde_json::json;

    use super::load;
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
        assert_eq!(response.payload["modules"][0]["name"], "example");
        assert_eq!(
            response.payload["entries"][0]["skeletonPath"],
            "/modules/example/skeletons/FR.md"
        );
    }
}
