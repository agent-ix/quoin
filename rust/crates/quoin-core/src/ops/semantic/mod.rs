// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `semantic`: the module `semantic` block, and the legacy-form corpus
//! sweep (quoin#452, Stage 8).
//!
//! Replaces `src/semantic/`'s TypeScript. What it decided — whether a
//! manifest's `semantic` block satisfies the vendored contract and with what
//! diagnostics, how a Markdown artifact's Properties section classifies, and
//! what a corpus sweep tallies — is decided by `quoin-semantic` and reported
//! through here. The vendored schema tree itself does not move: it is DATA,
//! shipped in the npm package, and its path arrives as `QUOIN_SEMANTIC_ROOT`.
//!
//! # No host capability, and how
//!
//! Judging a module reads its `manifest.yaml`, every `data_schema` file it
//! ships, and 35 vendored JSON schemas; a sweep walks whole repositories.
//! Those bytes cannot ride on stdin, so this module does not read them: it is
//! **granted** a [`SemanticHost`] by `main.rs`, the only file in the crate
//! allowed to hold a host capability. Everything this module decides on its
//! own — that a request is well formed, that a path is within its ceiling, how
//! a `SemanticError` maps onto the exit taxonomy, what the payload looks like —
//! is decided with no disk at all, and the tests below prove it by running
//! against an in-memory host. See [`crate::capabilities`].
//!
//! The domain is split the way it reads: [`wire`] holds the request and payload
//! shapes together with the ceilings they are read under, [`taxonomy`] holds the
//! one place a `SemanticError` becomes an exit status, and this file holds the
//! three operations and the helpers they share.

mod taxonomy;
mod wire;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use quoin_semantic::{CorpusRoot, LEGACY_MIGRATION_EXAMPLE, SweepIdentity};

use crate::capabilities::{Capabilities, SemanticHost};
use crate::error::{CoreError, CoreErrorCode};
use crate::ops::{refusal, request_size};
use crate::protocol::Response;

use self::taxonomy::map_error;
use self::wire::EmptyRequest;
pub use self::wire::{
    MAX_READ_BLOCKS_BYTES, MAX_SCALAR_BYTES, MAX_SWEEP_CORPUS_BYTES, MigrationExamplePayload,
    ModuleSemanticView, ReadBlocksPayload, ReadBlocksRequest, SweepCorpusPayload,
    SweepCorpusRequest, SweepRootRequest,
};

/// Answer a `semantic.read_blocks`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`ReadBlocksRequest`].
/// - [`CoreErrorCode::Refused`] when the request or any root exceeds its
///   ceiling, or when the host has no vendored contract to judge against.
/// - Whatever [`map_error`] makes of the host's failure.
pub fn read_blocks(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    // The whole-request bound first, then each path: a caller that sends ten
    // thousand short roots is refused by the first, one that sends a single
    // enormous root by the second, and neither reaches the host.
    let size = request_size(request)?;
    if size > MAX_READ_BLOCKS_BYTES {
        return Err(refusal("semantic.read_blocks", MAX_READ_BLOCKS_BYTES, size));
    }
    let request: ReadBlocksRequest = parse(request, "semantic.read_blocks")?;
    for root in &request.roots {
        check_bound("semantic.read_blocks", "roots", root, MAX_SCALAR_BYTES)?;
    }

    let host = host(capabilities, "semantic.read_blocks")?;
    let mut modules = Vec::with_capacity(request.roots.len());
    for root in &request.roots {
        let result = host
            .read_module(Path::new(root))
            .map_err(|e| map_error(&e, "semantic.read_blocks"))?;
        modules.push(ModuleSemanticView {
            root: root.clone(),
            block: result.module.map(|module| module.block),
            diagnostics: result.diagnostics,
        });
    }
    ok(&ReadBlocksPayload { modules })
}

/// Answer a `semantic.sweep_corpus`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`SweepCorpusRequest`].
/// - [`CoreErrorCode::Refused`] when the request or any scalar exceeds its
///   ceiling, or when a corpus root cannot be walked.
pub fn sweep_corpus(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let size = request_size(request)?;
    if size > MAX_SWEEP_CORPUS_BYTES {
        return Err(refusal(
            "semantic.sweep_corpus",
            MAX_SWEEP_CORPUS_BYTES,
            size,
        ));
    }
    let request: SweepCorpusRequest = parse(request, "semantic.sweep_corpus")?;
    let op = "semantic.sweep_corpus";
    check_bound(op, "package", &request.package, MAX_SCALAR_BYTES)?;
    check_bound(op, "version", &request.version, MAX_SCALAR_BYTES)?;
    check_bound(op, "generated_at", &request.generated_at, MAX_SCALAR_BYTES)?;
    for root in &request.roots {
        check_bound(op, "roots.root", &root.root, MAX_SCALAR_BYTES)?;
        check_bound(op, "roots.repository", &root.repository, MAX_SCALAR_BYTES)?;
        check_bound(op, "roots.revision", &root.revision, MAX_SCALAR_BYTES)?;
    }

    let roots: Vec<CorpusRoot> = request
        .roots
        .iter()
        .map(|root| CorpusRoot {
            root: PathBuf::from(&root.root),
            repository: root.repository.clone(),
            revision: root.revision.clone(),
        })
        .collect();
    let identity = SweepIdentity {
        package: request.package.as_str().into(),
        version: request.version.clone(),
    };

    let host = host(capabilities, op)?;
    let report = host
        .sweep(&roots, &identity, &request.generated_at)
        .map_err(|e| map_error(&e, op))?;
    ok(&SweepCorpusPayload { report })
}

/// Answer a `semantic.migration_example`.
///
/// The FR-074 migration guidance, served as a fact rather than restated on the
/// TypeScript side. `quoin write` prints it once per authoring pack that names
/// a semantic module, and every legacy-form diagnostic a sweep emits carries
/// the same text; one copy, in `quoin_semantic`, is what keeps those two from
/// drifting apart.
///
/// It needs no capability: the answer is a constant, and it is answered with a
/// grant of nothing.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin carries any field at all.
/// - [`CoreErrorCode::Io`] when the payload cannot be serialised.
pub fn migration_example(request: &serde_json::Value) -> Result<Response, CoreError> {
    let _: EmptyRequest = parse(request, "semantic.migration_example")?;
    ok(&MigrationExamplePayload {
        example: LEGACY_MIGRATION_EXAMPLE.to_owned(),
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

/// The granted semantic host, or the internal fault of having none.
fn host<'a>(
    capabilities: &Capabilities<'a>,
    op: &'static str,
) -> Result<&'a dyn SemanticHost, CoreError> {
    capabilities.semantic.ok_or_else(|| {
        // Internal (4), not Refused (2): the caller did nothing wrong. This is
        // `main.rs` having failed to grant a capability the operation table
        // says this operation needs, and it must read as a build fault rather
        // than as "quoin declined to read your module".
        CoreError::new(
            CoreErrorCode::Io,
            "this build dispatched a semantic operation without granting a semantic host",
        )
        .with_context("op", op)
    })
}

/// Refuse an oversized field before any work is done on it.
fn check_bound(op: &'static str, field: &str, value: &str, limit: usize) -> Result<(), CoreError> {
    if value.len() > limit {
        return Err(CoreError::new(
            CoreErrorCode::Refused,
            "a request field exceeds the accepted size",
        )
        .with_context("op", op)
        .with_context("field", field.to_owned())
        .with_context("limit_bytes", limit.to_string())
        .with_context("observed_bytes", value.len().to_string()));
    }
    Ok(())
}
