// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `graph`: the three read-only assurance-graph projections
//! (FR-062, quoin#500).
//!
//! Replaces `src/graph-analysis/`'s TypeScript — 1,373 lines across five files
//! — which `src/commands/graph/*` reached directly. What it computed is
//! `quoin-graph-analysis`'; what this module does is read a request, ask the
//! granted reader for four files, and hand back ONE rendered document.
//!
//! # Why the payload is a rendered string and not a report object
//!
//! A graph command prints one document and nothing else. Both spellings a
//! caller may ask for are `quoin-graph-analysis`': `render_graph_analysis_json`
//! is `quoin_store::canonical_json` and `render_graph_analysis` is the markdown
//! the retained `render.ts` wrote. Putting the report object on the wire as
//! well would be a second encoding of the same report for the TypeScript side
//! to re-render and disagree with — which is the shape FR-097 forbids.
//!
//! # No host capability of this domain's own, and how
//!
//! Loading reads four files: the export, the premises, the audit envelope and
//! the retained bindings store. Those bytes could ride on stdin, but their
//! PATHS are what the command was given and the absent/unreadable distinction
//! the bindings store turns on is a property of the read itself. So this module
//! does not read them: it is **granted** a [`GraphInputReader`] by `main.rs`,
//! the only file in the crate allowed to hold a host capability.
//!
//! The seam is `quoin-graph-analysis`' own trait rather than a second one
//! restating it, for the reason [`crate::capabilities::EvidenceHost`] gives:
//! the crate already says "where the four inputs come from" once, and a
//! parallel declaration here would be a shape nobody checks against it. Every
//! test below substitutes an in-memory tree and touches no disk at all.

mod taxonomy;
mod wire;

#[cfg(test)]
mod tests;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use quoin_graph_analysis::{
    ArtifactId, GraphAnalysis, GraphAnalysisInput, GraphInputReader, GraphLoadOptions,
    RelationKind, analyze_change_impact, analyze_churn, analyze_fan_out, load_graph_analysis_input,
    render_graph_analysis, render_graph_analysis_json,
};

use crate::capabilities::Capabilities;
use crate::error::{CoreError, CoreErrorCode};
use crate::ops::{refusal, request_size};
use crate::protocol::Response;

use self::taxonomy::map_error;
pub use self::wire::{
    ChangeImpactRequest, MAX_GRAPH_REQUEST_BYTES, MAX_SCALAR_BYTES, RenderedPayload, ViewRequest,
};

/// Answer a `graph.fan_out`: which suites carry which live obligations.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`ViewRequest`].
/// - [`CoreErrorCode::Refused`] when the request or a path exceeds its ceiling,
///   when a declared input cannot be read or accepted, or when the build
///   granted no reader.
pub fn fan_out(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "graph.fan_out";
    let request: ViewRequest = read_view(request, op)?;
    let input = load(&request, capabilities, op)?;
    render(
        &GraphAnalysis::from(analyze_fan_out(&input)),
        request.json,
        op,
    )
}

/// Answer a `graph.churn`: which retained obligations were re-affirmed, by whom.
///
/// # Errors
///
/// As [`fan_out`].
pub fn churn(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "graph.churn";
    let request: ViewRequest = read_view(request, op)?;
    let input = load(&request, capabilities, op)?;
    render(
        &GraphAnalysis::from(analyze_churn(&input)),
        request.json,
        op,
    )
}

/// Answer a `graph.change_impact`: what depends on the named requirements.
///
/// # Errors
///
/// As [`fan_out`], plus [`CoreErrorCode::Io`] when the report has no canonical
/// spelling.
pub fn change_impact(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "graph.change_impact";
    let size = request_size(request)?;
    if size > MAX_GRAPH_REQUEST_BYTES {
        return Err(refusal(op, MAX_GRAPH_REQUEST_BYTES, size));
    }
    let request: ChangeImpactRequest = parse(request, op)?;
    let paths = ViewRequest {
        repo: request.repo.clone(),
        export_path: request.export_path.clone(),
        premises_path: request.premises_path.clone(),
        audit_path: request.audit_path.clone(),
        json: request.json,
    };
    check_paths(&paths, op)?;
    for requirement in &request.requirements {
        check_bound(op, "requirements", requirement)?;
    }
    for kind in request.relations.iter().flatten() {
        check_bound(op, "relations", kind)?;
    }

    let input = load(&paths, capabilities, op)?;
    let requested: Vec<ArtifactId> = request
        .requirements
        .iter()
        .map(|id| ArtifactId::new(id.as_str()))
        .collect();
    // `None` and `Some([])` are different walks — the defaults, and no edges at
    // all — so the option is carried through rather than collapsed.
    let selected: Option<Vec<RelationKind>> = request.relations.as_ref().map(|kinds| {
        kinds
            .iter()
            .map(|kind| RelationKind::new(kind.as_str()))
            .collect()
    });
    let analysis = analyze_change_impact(&input, &requested, selected.as_deref())
        .map_err(|error| map_error(&error, op))?;
    render(&GraphAnalysis::from(analysis), request.json, op)
}

/// Read and bound a two-view request.
fn read_view(request: &serde_json::Value, op: &'static str) -> Result<ViewRequest, CoreError> {
    let size = request_size(request)?;
    if size > MAX_GRAPH_REQUEST_BYTES {
        return Err(refusal(op, MAX_GRAPH_REQUEST_BYTES, size));
    }
    let request: ViewRequest = parse(request, op)?;
    check_paths(&request, op)?;
    Ok(request)
}

/// Bound each of the four paths before any of them is opened.
fn check_paths(request: &ViewRequest, op: &'static str) -> Result<(), CoreError> {
    check_bound(op, "repo", &request.repo)?;
    check_bound(op, "export_path", &request.export_path)?;
    check_bound(op, "premises_path", &request.premises_path)?;
    check_bound(op, "audit_path", &request.audit_path)
}

/// The four inputs, read through the granted reader.
fn load(
    request: &ViewRequest,
    capabilities: &Capabilities<'_>,
    op: &'static str,
) -> Result<GraphAnalysisInput, CoreError> {
    let reader = reader(capabilities, op)?;
    let options = GraphLoadOptions {
        repo: PathBuf::from(&request.repo),
        export_path: PathBuf::from(&request.export_path),
        premises_path: PathBuf::from(&request.premises_path),
        audit_path: PathBuf::from(&request.audit_path),
    };
    load_graph_analysis_input(reader, &options).map_err(|error| map_error(&error, op))
}

/// One report, rendered the way the request asked for it.
fn render(analysis: &GraphAnalysis, json: bool, op: &'static str) -> Result<Response, CoreError> {
    let rendered = if json {
        render_graph_analysis_json(analysis).map_err(|error| map_error(&error, op))?
    } else {
        render_graph_analysis(analysis)
    };
    ok(&RenderedPayload { rendered })
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

/// The granted reader, or the internal fault of having none.
fn reader<'a>(
    capabilities: &Capabilities<'a>,
    op: &'static str,
) -> Result<&'a dyn GraphInputReader, CoreError> {
    capabilities.graph.ok_or_else(|| {
        // Internal (4), not Refused (2): the caller did nothing wrong. This is
        // `main.rs` having failed to grant a capability the operation table
        // says this operation needs, and it must read as a build fault rather
        // than as "quoin declined to read your export".
        CoreError::new(
            CoreErrorCode::Io,
            "this build dispatched a graph operation without granting a graph input reader",
        )
        .with_context("op", op)
    })
}

/// Refuse an oversized field before any work is done on it.
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
