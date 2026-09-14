// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `evidence` operations that need no capability at all.
//!
//! `store_facts` serves the constants, and the three `parse_*` operations read
//! one producer, lineage or policy document out of the request. None of them
//! touches a store, so none of them takes a host: the containment audit in
//! `tests/tc_library_containment.rs` is what keeps that true.

use std::path::Path;

use quoin_evidence::MUTATION_SCORE_METRIC;
use quoin_evidence::adapters::{AdapterSelection, select_adapter, select_finding_adapter};
use quoin_evidence::independence::{
    require_known_policy_obligations, validate_evidence_lineage, validate_independence_policy,
};
use quoin_evidence::paths::{baseline_path, bindings_path, inspections_path, suites_path};
use quoin_evidence::store::COLLECTED_FAMILIES;
use quoin_evidence::trust::REQUIRED_TRIGGERS;
use quoin_evidence::types::{EvidenceLineage, IndependencePolicy, STORE_SCHEMA_VERSION};

use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::Response;

use super::taxonomy::map_error;
use super::wire::{
    EmptyRequest, MAX_PARSE_LINEAGE_BYTES, MAX_PARSE_POLICY_BYTES, MAX_PARSE_RESULTS_BYTES,
    MAX_STORE_FACTS_BYTES, ParseLineagePayload, ParseLineageRequest, ParsePolicyPayload,
    ParsePolicyRequest, ParseResultsPayload, ParseResultsRequest, StoreFactsPayload,
    UnrepresentedView,
};
use super::{bound, check_scalar, ok, parse};

// ─────────────────────────── facts, no capability ───────────────────────────

/// Answer an `evidence.store_facts`.
///
/// The constants the command layer needs before it can build any other request:
/// the adapter names an `--adapter` flag offers at flag-definition time, the
/// metric a mutation score is recorded under, the schema version stamped into
/// every envelope, and the store-relative names of the files a reader cites.
///
/// Served rather than restated. `src/core/evidence.ts` holds one copy of each
/// and `tests/core-evidence.test.ts` pins that copy against this operation —
/// the `REGISTRY_KEY_ORDER` precedent in `src/core/modules.ts`. A second
/// declaration nobody compares is how the two halves drift.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin carries any field at all.
pub fn store_facts(request: &serde_json::Value) -> Result<Response, CoreError> {
    let op = "evidence.store_facts";
    bound(request, op, MAX_STORE_FACTS_BYTES)?;
    let _: EmptyRequest = parse(request, op)?;
    ok(&StoreFactsPayload {
        adapter_names: quoin_evidence::adapters::adapter_names()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
        mutation_score_metric: MUTATION_SCORE_METRIC.to_owned(),
        store_schema_version: STORE_SCHEMA_VERSION,
        // Through `store_root` itself rather than spelled out, so the layout
        // has one statement: an empty root makes it answer with the relative
        // part and nothing else.
        store_root_path: quoin_store::store::store_root(Path::new(""))
            .display()
            .to_string(),
        bindings_path: bindings_path(),
        baseline_path: baseline_path(),
        suites_path: suites_path(),
        inspections_path: inspections_path(),
        collected_families: COLLECTED_FAMILIES
            .iter()
            .map(|family| (*family).to_owned())
            .collect(),
        required_triggers: REQUIRED_TRIGGERS.to_vec(),
    })
}

/// Answer an `evidence.parse_results`.
///
/// The adapter contract is pure — `parse(&str) -> Result<_, _>` reads no file,
/// spawns no process, reaches no network — so this operation needs no
/// capability at all. That is not an accident of this port: an adapter that
/// could run the tool would make `quoin evidence record` a test runner, and a
/// transcript nobody can trust (ADR-0011 invariant 1).
///
/// # Errors
///
/// - [`CoreErrorCode::Refused`] when the request exceeds [`MAX_PARSE_RESULTS_BYTES`].
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`ParseResultsRequest`],
///   when `adapter` names nothing, or when the document does not read.
pub fn parse_results(request: &serde_json::Value) -> Result<Response, CoreError> {
    let op = "evidence.parse_results";
    bound(request, op, MAX_PARSE_RESULTS_BYTES)?;
    let request: ParseResultsRequest = parse(request, op)?;
    check_scalar(op, "adapter", request.adapter.as_deref().unwrap_or(""))?;
    check_scalar(op, "tool", request.tool.as_deref().unwrap_or(""))?;
    ok(&parsed_results(
        &request.text,
        request.adapter.as_deref(),
        request.tool.as_deref(),
        op,
    )?)
}

/// Answer an `evidence.parse_lineage`.
///
/// # Errors
///
/// - [`CoreErrorCode::Refused`] when the request exceeds [`MAX_PARSE_LINEAGE_BYTES`].
/// - [`CoreErrorCode::BadRequest`] when the text is not a lineage document, or
///   names no dimension at all.
pub fn parse_lineage(request: &serde_json::Value) -> Result<Response, CoreError> {
    let op = "evidence.parse_lineage";
    bound(request, op, MAX_PARSE_LINEAGE_BYTES)?;
    let request: ParseLineageRequest = parse(request, op)?;
    let lineage: EvidenceLineage = serde_json::from_str(&request.text).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, format!("evidence lineage: {e}"))
            .with_context("op", op)
    })?;
    validate_evidence_lineage(&lineage).map_err(|e| map_error(&e, op))?;
    ok(&ParseLineagePayload { lineage })
}

/// Answer an `evidence.parse_policy`.
///
/// Validation and the known-obligation check are one call, not two: a policy
/// whose obligation was renamed reports every requirement as vacuously
/// assessed, which is the opposite of what an independence check is for, and a
/// caller that could ask the first question without the second would have a way
/// to get that outcome.
///
/// # Errors
///
/// - [`CoreErrorCode::Refused`] when the request exceeds [`MAX_PARSE_POLICY_BYTES`].
/// - [`CoreErrorCode::BadRequest`] when the text is not a policy, fails the
///   FR-094 boundary, or names an obligation the corpus does not derive.
pub fn parse_policy(request: &serde_json::Value) -> Result<Response, CoreError> {
    let op = "evidence.parse_policy";
    bound(request, op, MAX_PARSE_POLICY_BYTES)?;
    let request: ParsePolicyRequest = parse(request, op)?;
    let policy: IndependencePolicy = serde_json::from_str(&request.text).map_err(|e| {
        CoreError::new(
            CoreErrorCode::BadRequest,
            format!("independence policy: {e}"),
        )
        .with_context("op", op)
    })?;
    validate_independence_policy(&policy).map_err(|e| map_error(&e, op))?;
    require_known_policy_obligations(
        &policy,
        request.known_obligations.iter().map(String::as_str),
    )
    .map_err(|e| map_error(&e, op))?;
    ok(&ParsePolicyPayload { policy })
}

/// Read a producer document through the selected adapter.
pub(super) fn parsed_results(
    text: &str,
    adapter: Option<&str>,
    tool: Option<&str>,
    op: &'static str,
) -> Result<ParseResultsPayload, CoreError> {
    let selection = AdapterSelection { adapter, tool };
    if let Some(finding) = select_finding_adapter(selection) {
        let result = (finding.parse)(text).map_err(|e| map_error(&e, op))?;
        return Ok(ParseResultsPayload::Finding {
            findings: result.findings,
            tool: result.tool,
            ruleset: result.ruleset,
            rules_evaluated: result.rules_evaluated,
        });
    }
    let run = select_adapter(selection).map_err(|e| map_error(&e, op))?;
    let result = (run.parse)(text).map_err(|e| map_error(&e, op))?;
    Ok(ParseResultsPayload::Run {
        unrepresented: result.unrepresented.as_deref().map(unrepresented_views),
        entries: result.entries,
        evidence_kind: result.evidence_kind,
    })
}

/// The wire view of the results an adapter could not transcribe.
pub(super) fn unrepresented_views(
    items: &[quoin_evidence::adapters::UnrepresentedResult],
) -> Vec<UnrepresentedView> {
    items
        .iter()
        .map(|item| UnrepresentedView {
            symbol: item.symbol.clone(),
            state: item.state.clone(),
            reason: item.reason.clone(),
        })
        .collect()
}

/// The first whitespace-separated word of a tool identity.
pub(super) fn tool_name(value: &str) -> Option<&str> {
    value.split_whitespace().next()
}
