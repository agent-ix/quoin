// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `modules`: installing, listing, removing and reconciling spec
//! modules (quoin#446, Stage 7).
//!
//! Replaces `src/plugins.ts` and `src/modules.ts`. What those two decided —
//! how a CLI source argument maps to a typed source, what the default set is,
//! that a semantic-contract violation rejects an install and rolls the previous
//! version back, that a removal takes the tree and the registry record together
//! — is decided here and carried out by a [`ModuleHost`].
//!
//! # No host capability, and how
//!
//! A module install fetches a git remote, materialises a subtree and rewrites a
//! registry. Those bytes cannot ride on stdin, so this module does not do them:
//! it is **granted** a [`ModuleHost`] by `main.rs`, which is the only file in
//! the crate allowed to hold a host capability. Everything this module decides
//! on its own — parsing the source argument, validating the manifest, mapping a
//! `ModulesError` onto the exit taxonomy, refusing an oversized field — is
//! decided with no disk and no network, and every test below proves it by
//! running against an in-memory host. See [`crate::capabilities`].
//!
//! The `home` field is a path the caller names and this module never opens; it
//! is handed to the host verbatim. `None` means "the home the host resolves",
//! which only the production host can answer.
//!
//! The domain is split the way it reads: [`wire`] holds the request and
//! payload shapes together with the ceilings they are read under, [`taxonomy`]
//! holds the one place a `ModulesError` becomes an exit status, and this file
//! holds the four operations and the helpers they share.

mod taxonomy;
mod wire;

#[cfg(test)]
mod support;
#[cfg(test)]
mod tests;

use std::path::Path;

use serde::{Deserialize, Serialize};

use quoin_modules::{
    InstallOutcome, MarketplaceManifest, ModuleName, ReconcileReport, Source, parse_source_arg,
};

use crate::capabilities::{Capabilities, ModuleHost};
use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::Response;

use self::taxonomy::map_error;
pub use self::wire::{
    EnsureDefaultsPayload, EnsureDefaultsRequest, InstallPayload, InstallRequest, ListPayload,
    ListRequest, MAX_MANIFEST_BYTES, MAX_SCALAR_BYTES, MAX_SOURCE_ARG_BYTES, Mode, RemovePayload,
    RemoveRequest,
};

/// Answer a `modules.list`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`ListRequest`].
/// - [`CoreErrorCode::Refused`] when `home` exceeds [`MAX_SCALAR_BYTES`].
/// - Whatever [`map_error`] makes of the host's failure.
pub fn list(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let request: ListRequest = parse::<ListRequest>(request, "modules.list")?;
    check_bound(
        "modules.list",
        "home",
        request.home.as_deref(),
        MAX_SCALAR_BYTES,
    )?;
    let host = host(capabilities, "modules.list")?;

    let modules = host
        .list(request.home.as_deref().map(Path::new))
        .map_err(|e| map_error(&e, "modules.list"))?;
    ok(&ListPayload { modules })
}

/// Answer a `modules.install`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not an [`InstallRequest`], or
///   when `source` is not a source argument this build understands.
/// - [`CoreErrorCode::Refused`] when a bounded field exceeds its ceiling.
/// - Whatever [`map_error`] makes of the host's failure — including the
///   semantic-contract rejection that rolls the previous version back.
pub fn install(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let request: InstallRequest = parse::<InstallRequest>(request, "modules.install")?;
    check_bound(
        "modules.install",
        "source",
        Some(request.source.as_str()),
        MAX_SOURCE_ARG_BYTES,
    )?;
    check_bound(
        "modules.install",
        "home",
        request.home.as_deref(),
        MAX_SCALAR_BYTES,
    )?;

    // Parsed BEFORE the host is consulted: a source argument the build does not
    // understand is a caller mistake, and answering it needs no capability at
    // all. `parse_source_arg` is the port of `parseSourceArg`, so the `path:`,
    // `github:owner/repo//subdir@ref` and `package:` spellings are the same
    // ones, refused in the same cases.
    let source: Source =
        parse_source_arg(&request.source).map_err(|e| map_error(&e, "modules.install"))?;

    let host = host(capabilities, "modules.install")?;
    let outcome: InstallOutcome = host
        .install(request.home.as_deref().map(Path::new), &source)
        .map_err(|e| map_error(&e, "modules.install"))?;

    ok(&InstallPayload {
        module: outcome.module,
        replaced_previous: outcome.replaced_previous,
    })
}

/// Answer a `modules.remove`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`RemoveRequest`] or the
///   name is not usable as a directory name.
/// - [`CoreErrorCode::Refused`] when a bound is exceeded or the module is not
///   installed.
pub fn remove(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let request: RemoveRequest = parse::<RemoveRequest>(request, "modules.remove")?;
    check_bound(
        "modules.remove",
        "name",
        Some(request.name.as_str()),
        MAX_SCALAR_BYTES,
    )?;
    check_bound(
        "modules.remove",
        "home",
        request.home.as_deref(),
        MAX_SCALAR_BYTES,
    )?;

    // Validated here, not in the host: `..` or `a/b` as a module name is a
    // path escape, and it is refused before anything holding a filesystem
    // capability ever sees it.
    let name =
        ModuleName::new(request.name.clone()).map_err(|e| map_error(&e, "modules.remove"))?;

    let host = host(capabilities, "modules.remove")?;
    host.remove(request.home.as_deref().map(Path::new), &name)
        .map_err(|e| map_error(&e, "modules.remove"))?;

    ok(&RemovePayload {
        removed: name.as_str().to_owned(),
    })
}

/// Answer a `modules.ensure_defaults`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not an
///   [`EnsureDefaultsRequest`] or the manifest is structurally invalid.
/// - [`CoreErrorCode::Refused`] when a bound is exceeded.
pub fn ensure_defaults(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let request: EnsureDefaultsRequest =
        parse::<EnsureDefaultsRequest>(request, "modules.ensure_defaults")?;
    check_bound(
        "modules.ensure_defaults",
        "manifest",
        Some(request.manifest.as_str()),
        MAX_MANIFEST_BYTES,
    )?;
    check_bound(
        "modules.ensure_defaults",
        "home",
        request.home.as_deref(),
        MAX_SCALAR_BYTES,
    )?;

    // Validated before the host is consulted, for the same reason as the source
    // argument: a malformed default set is a caller mistake, answerable with no
    // capability.
    let manifest = MarketplaceManifest::from_yaml(&request.manifest)
        .map_err(|e| map_error(&e, "modules.ensure_defaults"))?;

    let host = host(capabilities, "modules.ensure_defaults")?;
    let report: ReconcileReport = host
        .ensure_defaults(
            request.home.as_deref().map(Path::new),
            &manifest,
            request.mode.into(),
        )
        .map_err(|e| map_error(&e, "modules.ensure_defaults"))?;

    // AFTER the reconcile, and unconditionally: `src/modules.ts` ran
    // `reconcile(...)` and then `validateInstalledSemantics(home)`, because a
    // `lazy` reconcile skips a settled entry without re-reading it, so a module
    // whose manifest stopped satisfying the contract after it was installed
    // would otherwise never be judged again (FR-070-AC-1, NFR-017-AC-1).
    host.validate_installed(request.home.as_deref().map(Path::new))
        .map_err(|e| map_error(&e, "modules.ensure_defaults"))?;

    let names = |list: &[ModuleName]| -> Vec<String> {
        list.iter().map(|n| n.as_str().to_owned()).collect()
    };
    ok(&EnsureDefaultsPayload {
        installed: names(&report.installed),
        unchanged: names(&report.unchanged),
        updated: names(&report.updated),
        skipped: names(&report.skipped),
    })
}

/// Parse a request, naming the operation on the refusal.
fn parse<T: for<'de> Deserialize<'de>>(
    request: &serde_json::Value,
    op: &'static str,
) -> Result<T, CoreError> {
    serde_json::from_value(request.clone()).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
    })
}

/// Serialise a payload into a clean success.
fn ok<T: Serialize>(payload: &T) -> Result<Response, CoreError> {
    let value = serde_json::to_value(payload)
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;
    Ok(Response::ok(value))
}

/// The granted module host, or the internal fault of having none.
fn host<'a>(
    capabilities: &Capabilities<'a>,
    op: &'static str,
) -> Result<&'a dyn ModuleHost, CoreError> {
    capabilities.modules.ok_or_else(|| {
        // Internal (4), not Refused (2): the caller did nothing wrong. This is
        // `main.rs` having failed to grant a capability the operation table
        // says this operation needs, and it must read as a build fault rather
        // than as "quoin declined to install your module".
        CoreError::new(
            CoreErrorCode::Io,
            "this build dispatched a module operation without granting a module host",
        )
        .with_context("op", op)
    })
}

/// Refuse an oversized field before any work is done on it.
fn check_bound(
    op: &'static str,
    field: &str,
    value: Option<&str>,
    limit: usize,
) -> Result<(), CoreError> {
    match value {
        Some(text) if text.len() > limit => Err(CoreError::new(
            CoreErrorCode::Refused,
            "a request field exceeds the accepted size",
        )
        .with_context("op", op)
        .with_context("field", field.to_owned())
        .with_context("limit_bytes", limit.to_string())
        .with_context("observed_bytes", text.len().to_string())),
        _ => Ok(()),
    }
}
