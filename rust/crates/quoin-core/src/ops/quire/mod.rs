// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `quire`: obligations and criterion shapes, from the linked engine
//! (quoin#502, Stage 7).
//!
//! Replaces `src/quire/` — 1,342 lines of TypeScript and five vendored JSON
//! schemas — with two operations. The ratio is not an accident: roughly half of
//! that directory existed only because the boundary was a **subprocess with no
//! shared types**, and a linked engine has no path to resolve, no bytes to pin
//! at run time, no pipe to overflow and no exit status to interpret.
//!
//! # What disappears rather than moving
//!
//! | `src/quire/` | here |
//! |---|---|
//! | `exec.ts` — executable resolution, `QUOIN_EXPECTED_QUIRE_SHA256`, a 64 MiB `maxBuffer`, a three-way termination taxonomy | nothing. There is no child process |
//! | `contract.ts` — a `minimumCli` floor, five schema digests, `checkVersionPremise` | nothing. The premise is the `rev` pin in `quoin-quire`'s manifest, checked by Cargo at build time rather than by quoin at run time |
//! | `validate.ts` — ajv compilation, `ContractViolation` | serde. The type a payload is read into is the engine's own |
//! | `types.ts` — hand-written mirrors of quire's payloads | [`wire`], which restates nothing: both payloads are built from types that already cross the boundary |
//!
//! **`checkVersionPremise` is the one worth naming**, because it was a
//! user-visible error and it can no longer fire. Six commands called it, and it
//! answered "is the `quire` on this machine new enough to emit the payload
//! shape I am about to parse?". After the cutover there is no `quire` on the
//! machine in that sense: quoin links one engine at one revision, so the
//! question is answered by construction. That is strictly stronger than the
//! check it replaces — a version floor compares numbers, a Cargo `rev` compares
//! the object.
//!
//! # No host capability, and how
//!
//! A coverage run walks a repository's whole `spec/` tree, its trace tags and
//! every installed module; those bytes cannot ride on stdin, and
//! `ScopeRoot::open` canonicalizes a path, which is itself a filesystem call.
//! So this module reads nothing: it is **granted** a
//! [`QuireHost`](crate::capabilities::QuireHost) by `main.rs`, the only file in
//! the crate allowed to hold one. What it decides on its own — that a request
//! is well formed, that a path is within its ceiling, how a `quoin_quire::
//! Error` maps onto the exit taxonomy, and which fields of an unbounded report
//! the boundary actually carries — is decided with no disk at all, and the
//! tests below prove it against an in-memory host.
//!
//! # One report field the boundary deliberately drops
//!
//! `quoin_quire` returns advisory [`Notice`](quoin_quire::Notice)s from the
//! module, corpus and symbol walks. They are **not** on either payload.
//! `runQuire` piped quire's stderr and discarded it on a successful run, so no
//! quoin command has ever shown a notice to anyone; surfacing them now would be
//! a user-visible change to six commands' output, which FR-101 says a cutover
//! does not get to make. Reported as quoin#513 rather than absorbed.

mod taxonomy;
mod wire;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quoin_auditor::advise::PropertyShape;
use quoin_quire_types::{CoverageDiagnostic, Obligation};
use serde::{Deserialize, Serialize};

use crate::capabilities::{Capabilities, QuireHost};
use crate::error::{CoreError, CoreErrorCode};
use crate::ops::{refusal, request_size};
use crate::protocol::Response;

use self::taxonomy::map_error;
pub use self::wire::{
    CoveragePayload, CoverageRequest, MAX_REQUEST_BYTES, MAX_SCALAR_BYTES, PropertiesPayload,
    PropertiesRequest,
};

/// Answer a `quire.coverage`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`CoverageRequest`].
/// - [`CoreErrorCode::Refused`] when the request or a path exceeds its ceiling,
///   or when this build was dispatched without a quire host.
/// - Whatever [`map_error`] makes of the engine's failure.
pub fn coverage(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    const OP: &str = "quire.coverage";
    let request: CoverageRequest = read(request, OP)?;
    check_bound(OP, "scope", &request.scope)?;
    for module in &request.modules {
        check_bound(OP, "modules", module)?;
    }

    let outcome = host(capabilities, OP)?
        .coverage(Path::new(&request.scope), &paths(&request.modules))
        .map_err(|e| map_error(&e, OP))?;

    // The projection, and the only decision this operation makes. `report` is
    // an unbounded structure — rows, symbols, totals, the status census — of
    // which the retained TypeScript read exactly two fields across six
    // commands. Carrying the rest would put quire's whole rollup vocabulary on
    // quoin's boundary for nobody.
    ok(&CoveragePayload {
        obligations: outcome.report.obligations.iter().map(obligation).collect(),
        diagnostics: outcome.report.diagnostics.iter().map(diagnostic).collect(),
    })
}

/// Answer a `quire.properties`.
///
/// # Errors
///
/// The same three as [`coverage`].
pub fn properties(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    const OP: &str = "quire.properties";
    let request: PropertiesRequest = read(request, OP)?;
    check_bound(OP, "scope", &request.scope)?;
    for module in &request.modules {
        check_bound(OP, "modules", module)?;
    }
    for document in &request.documents {
        check_bound(OP, "documents", document)?;
    }

    let outcome = host(capabilities, OP)?
        .properties(
            Path::new(&request.scope),
            &paths(&request.modules),
            &request.documents,
        )
        .map_err(|e| map_error(&e, OP))?;

    // Keyed by `row_id`, and a criterion without one is skipped rather than
    // keyed on something else: `row_id` IS the obligation id the advisor joins
    // on, so a criterion the classifier could not tie to a row has no shape to
    // offer anybody.
    let mut shapes = BTreeMap::new();
    for document in &outcome.report.documents {
        for criterion in &document.criteria {
            if let Some(row_id) = criterion.row_id.clone() {
                shapes.insert(
                    row_id,
                    PropertyShape {
                        property: criterion.property.as_str().to_owned(),
                        archetype: document.archetype.clone(),
                    },
                );
            }
        }
    }
    ok(&PropertiesPayload {
        shapes,
        unresolved: outcome
            .unresolved
            .iter()
            .map(|entry| entry.document.clone())
            .collect(),
    })
}

/// Project one engine obligation onto the boundary's reader.
///
/// The five fields quoin reads, and an empty collection back to the absence it
/// was on the wire: quire emits `parameters` and `target_ids` with
/// `skip_serializing_if`, so the retained TypeScript saw a missing key where
/// the engine holds an empty map. `Some(empty)` would be a third state neither
/// side has ever had.
fn obligation(source: &quoin_quire::model::Obligation) -> Obligation {
    Obligation {
        id: source.id.clone(),
        statement: source.statement.clone(),
        statement_hash: source.statement_hash.clone(),
        target_ids: some_unless_empty(source.target_ids.clone(), Vec::is_empty),
        method: source.method.clone(),
        criticality: source.criticality.clone(),
        parameters: some_unless_empty(source.parameters.clone(), BTreeMap::is_empty),
    }
}

/// Project one engine diagnostic onto the two fields the advisor joins on.
fn diagnostic(source: &quoin_quire::model::CoverageDiagnostic) -> CoverageDiagnostic {
    CoverageDiagnostic {
        reason: source.reason.clone(),
        value: source.value.clone(),
    }
}

/// `None` for an empty collection, so an absence stays an absence.
fn some_unless_empty<T>(value: T, is_empty: impl FnOnce(&T) -> bool) -> Option<T> {
    if is_empty(&value) { None } else { Some(value) }
}

/// The module roots a request named, as paths.
fn paths(modules: &[String]) -> Vec<PathBuf> {
    modules.iter().map(PathBuf::from).collect()
}

/// Read a request under the domain's whole-request ceiling.
fn read<T: for<'de> Deserialize<'de>>(
    request: &serde_json::Value,
    op: &'static str,
) -> Result<T, CoreError> {
    let size = request_size(request)?;
    if size > MAX_REQUEST_BYTES {
        return Err(refusal(op, MAX_REQUEST_BYTES, size));
    }
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

/// The granted quire host, or the internal fault of having none.
fn host<'a>(
    capabilities: &Capabilities<'a>,
    op: &'static str,
) -> Result<&'a dyn QuireHost, CoreError> {
    capabilities.quire.ok_or_else(|| {
        // Internal (4), not Refused (2): the caller did nothing wrong. This is
        // `main.rs` having failed to grant a capability the operation table
        // says this operation needs, and it must read as a build fault rather
        // than as "quoin declined to read your repository".
        CoreError::new(
            CoreErrorCode::Io,
            "this build dispatched a quire operation without granting a quire host",
        )
        .with_context("op", op)
    })
}

/// Refuse an oversized path before any work is done on it.
fn check_bound(op: &'static str, field: &str, value: &str) -> Result<(), CoreError> {
    if value.len() > MAX_SCALAR_BYTES {
        return Err(CoreError::new(
            CoreErrorCode::Refused,
            "a request field exceeds the accepted size",
        )
        .with_context("op", op)
        .with_context("field", field.to_owned())
        .with_context("limit_bytes", MAX_SCALAR_BYTES.to_string())
        .with_context("observed_bytes", value.len().to_string()));
    }
    Ok(())
}
