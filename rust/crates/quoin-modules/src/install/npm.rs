// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! npm-specific source resolution and rollback validation for installation.

use std::path::{Path, PathBuf};

use super::support::ResolvedSource;
use crate::error::ModulesError;
pub(super) use crate::npm::{HttpNpmTransport, NpmTransport};
use crate::registry::InstalledModule;
use crate::source::Source;

pub(super) static HTTP_NPM_TRANSPORT: HttpNpmTransport = HttpNpmTransport;

pub(super) fn resolve_source(
    cache_root: &Path,
    source: &Source,
    transport: &dyn NpmTransport,
) -> Result<ResolvedSource, ModulesError> {
    let Source::Npm { package, .. } = source else {
        return Err(ModulesError::UnsupportedSource {
            source_type: source.type_name(),
        });
    };
    Ok(ResolvedSource {
        dir: crate::npm::resolve_package(cache_root, source, transport)?,
        requested_ref: None,
        source: Source::Npm {
            package: package.clone(),
            version: None,
            registry: None,
        },
    })
}

pub(super) fn cached_source(
    cache_root: &Path,
    previous: &InstalledModule,
) -> Result<PathBuf, ModulesError> {
    let cache_root =
        cache_root
            .join("npm")
            .canonicalize()
            .map_err(|error| ModulesError::NpmMetadata {
                detail: format!("previous package cache is unavailable: {error}"),
            })?;
    let resolved = PathBuf::from(&previous.resolved_path)
        .canonicalize()
        .map_err(|error| ModulesError::NpmMetadata {
            detail: format!("previous package cache is unavailable: {error}"),
        })?;
    if !resolved.starts_with(&cache_root) {
        return Err(ModulesError::NpmMetadata {
            detail: "previous package cache path is outside the npm cache".to_owned(),
        });
    }
    if crate::module_name::read_module_name(&resolved)? != previous.name {
        return Err(ModulesError::NpmMetadata {
            detail: "previous package cache no longer names the installed module".to_owned(),
        });
    }
    Ok(resolved)
}
