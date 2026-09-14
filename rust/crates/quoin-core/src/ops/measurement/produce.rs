// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The two producers: a definition in, one published record out.
//!
//! Both routes consume RETAINED artefacts only. Neither invokes an agent, an
//! evaluation harness, a workflow, a network client or a process — that is the
//! property `src/commands/measurement/intervention.ts` and
//! `operational-release.ts` state in their own `description`, and moving them
//! behind this boundary does not weaken it: nothing here names a host
//! capability, and the files the definitions point at are read by
//! `quoin-measurement` through its own `MeasurementSource`.

use std::path::Path;

use quoin_measurement::intervention::agent_eval::produce_agent_eval_intervention as produce_intervention;
use quoin_measurement::operational::github_release::produce_github_release_operational as produce_release;
use quoin_measurement::source::SystemClock;

use crate::error::CoreError;
use crate::protocol::Response;

use super::taxonomy::{map_github_release, map_intake};
use super::wire::{
    AgentEvalRequest, GitHubReleaseRequest, MAX_PRODUCER_DEFINITION_BYTES, MAX_RECORD_ID_BYTES,
    MAX_RETAINED_EXPORT_BYTES, MAX_WORKFLOW_YAML_BYTES,
};
use super::{bound, bound_field, parse, path_payload, string_len};
use crate::ops::request_size;

/// Bound a producer request: the whole of it, then its definition, then the
/// identity the definition mints records under.
///
/// One function for both routes rather than two copies differing in a
/// constant: the shape of a producer request is the same shape, and the
/// identity bound in particular is a store rule that cannot hold for one
/// producer and not the other.
fn bound_producer(
    request: &serde_json::Value,
    op: &'static str,
    whole: usize,
    identity: &'static str,
) -> Result<(), CoreError> {
    bound(request, op, whole)?;
    let Some(definition) = request.get("definition") else {
        // Absent: `parse` will name it. Nothing is measured, and nothing is
        // read — the point of this function is that no store is opened first.
        return Ok(());
    };
    bound_field(
        op,
        "definition",
        request_size(definition)?,
        MAX_PRODUCER_DEFINITION_BYTES,
    )?;
    bound_field(
        op,
        identity,
        string_len(definition, identity),
        MAX_RECORD_ID_BYTES,
    )
}

/// Answer a `measurement.produce_agent_eval_intervention`.
///
/// # Errors
///
/// - [`crate::error::CoreErrorCode::BadRequest`] when stdin is not an
///   [`AgentEvalRequest`], or when the definition or the retained reports do
///   not satisfy the producer.
/// - [`crate::error::CoreErrorCode::Refused`] when a ceiling is exceeded, or
///   when the store declines the produced record.
pub fn produce_agent_eval_intervention(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "measurement.produce_agent_eval_intervention";
    bound_producer(request, OP, MAX_RETAINED_EXPORT_BYTES, "record_id")?;

    let request: AgentEvalRequest = parse(request, OP)?;
    let produced = produce_intervention(Path::new(&request.repo), &request.definition)
        .map_err(|e| map_intake(&e, OP))?;
    path_payload(&produced.path, OP)
}

/// Answer a `measurement.produce_github_release_operational`.
///
/// # Errors
///
/// - [`crate::error::CoreErrorCode::BadRequest`] when stdin is not a
///   [`GitHubReleaseRequest`], or when the definition and the three retained
///   exports disagree.
/// - [`crate::error::CoreErrorCode::Refused`] when a ceiling is exceeded, or
///   when the store declines the produced pair.
pub fn produce_github_release_operational(
    request: &serde_json::Value,
) -> Result<Response, CoreError> {
    const OP: &str = "measurement.produce_github_release_operational";
    bound_producer(request, OP, MAX_WORKFLOW_YAML_BYTES, "record_prefix")?;

    let request: GitHubReleaseRequest = parse(request, OP)?;
    let produced = produce_release(Path::new(&request.repo), &SystemClock, &request.definition)
        .map_err(|e| map_github_release(&e, OP))?;
    path_payload(&produced.path, OP)
}
