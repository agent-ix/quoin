// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Routing `<domain>.<op>` to an operation, and nothing else.
//!
//! This module holds no domain logic and must not acquire any. It is the one
//! place that knows which operations exist, so "which operations exist" is one
//! fact in one place — an exhaustive `match` the compiler checks, not a
//! registry assembled at run time.

use std::io::Read;

use crate::capabilities::Capabilities;
use crate::error::{CoreError, CoreErrorCode};
use crate::ops::evidence;
use crate::protocol::{MAX_REQUEST_BYTES, Response};

/// Every operation this build answers, in the wire spelling.
///
/// Exposed so a caller — and the native fixture suite — can enumerate the surface
/// rather than discover it by trying names.
pub const OPERATIONS: &[&str] = &[
    "assurance.build_authored_argument",
    "assurance.build_case",
    "assurance.build_discharge",
    "assurance.parse_argument",
    "assurance.render_authored_argument",
    "assurance.render_case",
    "assurance.render_discharge",
    "assurance.requirement_of",
    "auditor.advise",
    "auditor.audit",
    "auditor.vocabulary",
    "catalog.load",
    "catalog.methods",
    "change_assurance.intake",
    "change_assurance.prepare_attestation",
    "change_assurance.receipt",
    "change_assurance.record_digest",
    "change_assurance.recover",
    "change_assurance.schema",
    "change_assurance.verify_receipt",
    "change_assurance.write_record",
    "completeness.assess_bundle",
    "completeness.read_frontmatter",
    "completeness.schema_refs",
    "config.resolve_org",
    "config.unresolved_org_message",
    "core.ping",
    "evidence.audit_inputs",
    "evidence.gc",
    "evidence.inspect_mocks",
    "evidence.parse_lineage",
    "evidence.parse_policy",
    "evidence.parse_results",
    "evidence.record",
    "evidence.discharge_report",
    "evidence.inward_completeness",
    "evidence.record_discharge",
    "evidence.record_experiment",
    "evidence.record_operational",
    "evidence.record_scenario",
    "evidence.pack_requirements_matrix",
    "evidence.requirements_matrix",
    "evidence.store_facts",
    "evidence.trust_assessments",
    "evidence.trust_decision",
    "graph.change_impact",
    "graph.fan_out",
    "matrix.build",
    "measurement.attach",
    "measurement.build_graph_portfolio",
    "measurement.build_portfolio",
    "measurement.build_report",
    "measurement.build_series",
    "measurement.produce_agent_eval_intervention",
    "measurement.produce_github_release_operational",
    "measurement.record",
    "measurement.render_graph_portfolio",
    "measurement.render_portfolio",
    "measurement.render_report",
    "measurement.verify",
    "modules.ensure_defaults",
    "modules.install",
    "modules.list",
    "modules.remove",
    "quire.coverage",
    "quire.design_join",
    "quire.properties",
    "semantic.migration_example",
    "semantic.read_blocks",
    "semantic.sweep_corpus",
    "validators.run",
];

/// Route one request.
///
/// `op` is the sole command-line argument; `request` is the parsed stdin
/// document; `capabilities` is what `main.rs` granted this invocation.
///
/// `capabilities` is threaded through every arm rather than only the arms that
/// need one, so the grant is visible at the routing table instead of being
/// reached for inside an operation. An operation that needs a capability it was
/// not granted says so as an internal fault (see `ops::modules`), never by
/// acquiring one itself — `tests/tc_library_containment.rs` is what makes that
/// a rule rather than a habit.
///
/// # Errors
///
/// [`CoreErrorCode::UnknownOp`] when `op` names nothing this build implements,
/// or whatever the operation itself returns.
#[allow(
    clippy::too_many_lines,
    reason = "the routing table: one arm per operation and no logic, which the drift guard reads"
)]
pub fn dispatch(
    op: &str,
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    match op {
        "assurance.requirement_of" => crate::ops::assurance::requirement_of(request),
        "assurance.build_case" => crate::ops::assurance::build_case(request),
        "matrix.build" => crate::ops::matrix::build(request),
        "assurance.render_case" => crate::ops::assurance::render_case(request),
        "assurance.parse_argument" => crate::ops::assurance::parse_argument(request),
        "config.resolve_org" => crate::ops::config::resolve_org(request),
        "config.unresolved_org_message" => crate::ops::config::unresolved_org_message(request),
        "assurance.build_authored_argument" => {
            crate::ops::assurance::build_authored_argument(request)
        }
        "assurance.render_authored_argument" => {
            crate::ops::assurance::render_authored_argument(request)
        }
        "assurance.build_discharge" => crate::ops::assurance::build_discharge(request),
        "assurance.render_discharge" => crate::ops::assurance::render_discharge(request),
        "change_assurance.intake" => crate::ops::change_assurance::intake(request, capabilities),
        "change_assurance.prepare_attestation" => {
            crate::ops::change_assurance::prepare_attestation(request)
        }
        "change_assurance.receipt" => crate::ops::change_assurance::receipt(request, capabilities),
        "change_assurance.record_digest" => {
            crate::ops::change_assurance::record_digest(request, capabilities)
        }
        "change_assurance.recover" => crate::ops::change_assurance::recover(request, capabilities),
        "change_assurance.schema" => crate::ops::change_assurance::schema(request),
        "change_assurance.verify_receipt" => {
            crate::ops::change_assurance::verify_receipt(request, capabilities)
        }
        "change_assurance.write_record" => {
            crate::ops::change_assurance::write_record(request, capabilities)
        }
        "completeness.assess_bundle" => crate::ops::completeness::assess_bundle(request),
        "completeness.read_frontmatter" => crate::ops::completeness::read_frontmatter(request),
        "completeness.schema_refs" => crate::ops::completeness::schema_refs(request),
        "core.ping" => crate::ops::core::ping(request),
        "quire.coverage" => crate::ops::quire::coverage(request, capabilities),
        "quire.design_join" => crate::ops::quire::design_join(request, capabilities),
        "quire.properties" => crate::ops::quire::properties(request, capabilities),
        "evidence.store_facts" => crate::ops::evidence::store_facts(request),
        "evidence.parse_results" => crate::ops::evidence::parse_results(request),
        "evidence.parse_lineage" => crate::ops::evidence::parse_lineage(request),
        "evidence.parse_policy" => crate::ops::evidence::parse_policy(request),
        "evidence.gc" => crate::ops::evidence::gc(request, capabilities),
        "evidence.record" => crate::ops::evidence::record(request, capabilities),
        "evidence.trust_decision" => crate::ops::evidence::trust_decision(request, capabilities),
        "evidence.trust_assessments" => {
            crate::ops::evidence::trust_assessments(request, capabilities)
        }
        "evidence.inspect_mocks" => crate::ops::evidence::inspect_mocks(request, capabilities),
        "evidence.record_experiment" => {
            crate::ops::evidence::record_experiment(request, capabilities)
        }
        "evidence.record_discharge" => evidence::record_discharge(request, capabilities),
        "evidence.discharge_report" => evidence::discharge_report(request, capabilities),
        "evidence.inward_completeness" => evidence::inward_completeness(request, capabilities),
        "evidence.pack_requirements_matrix" => evidence::pack_matrix(request, capabilities),
        "evidence.requirements_matrix" => evidence::requirements_matrix(request, capabilities),
        "evidence.record_scenario" => crate::ops::evidence::record_scenario(request, capabilities),
        "evidence.record_operational" => {
            crate::ops::evidence::record_operational(request, capabilities)
        }
        "evidence.audit_inputs" => crate::ops::evidence::audit_inputs(request, capabilities),
        "measurement.attach" => crate::ops::measurement::attach(request),
        "measurement.record" => crate::ops::measurement::record(request),
        "measurement.verify" => crate::ops::measurement::verify(request),
        "measurement.produce_agent_eval_intervention" => {
            crate::ops::measurement::produce_agent_eval_intervention(request)
        }
        "measurement.produce_github_release_operational" => {
            crate::ops::measurement::produce_github_release_operational(request)
        }
        "measurement.build_report" => crate::ops::measurement::build_report(request),
        "measurement.render_report" => crate::ops::measurement::render_report(request),
        "measurement.build_series" => crate::ops::measurement::build_series(request),
        "measurement.build_portfolio" => crate::ops::measurement::build_portfolio(request),
        "measurement.render_portfolio" => crate::ops::measurement::render_portfolio(request),
        "auditor.audit" => crate::ops::auditor::audit(request),
        "auditor.advise" => crate::ops::auditor::advise(request),
        "auditor.vocabulary" => crate::ops::auditor::vocabulary(request),
        "catalog.load" => crate::ops::catalog::load(request, capabilities),
        "catalog.methods" => crate::ops::catalog::methods(request, capabilities),
        "graph.fan_out" => crate::ops::graph::fan_out(request, capabilities),
        "graph.change_impact" => crate::ops::graph::change_impact(request, capabilities),
        "measurement.build_graph_portfolio" => {
            crate::ops::measurement::build_graph_portfolio(request)
        }
        "measurement.render_graph_portfolio" => {
            crate::ops::measurement::render_graph_portfolio(request)
        }
        "modules.ensure_defaults" => crate::ops::modules::ensure_defaults(request, capabilities),
        "modules.install" => crate::ops::modules::install(request, capabilities),
        "modules.list" => crate::ops::modules::list(request, capabilities),
        "modules.remove" => crate::ops::modules::remove(request, capabilities),
        "semantic.read_blocks" => crate::ops::semantic::read_blocks(request, capabilities),
        "semantic.sweep_corpus" => crate::ops::semantic::sweep_corpus(request, capabilities),
        "semantic.migration_example" => crate::ops::semantic::migration_example(request),
        "validators.run" => crate::ops::validators::run(request),
        _ => Err(
            CoreError::new(CoreErrorCode::UnknownOp, "no such operation in this build")
                .with_context("op", op)
                .with_context("known", OPERATIONS.join(",")),
        ),
    }
}

/// Parse a stdin document.
///
/// An empty or whitespace-only stdin means the empty request `{}`. A request
/// that is not a JSON **object** is refused: the unit of IPC is a named
/// operation with named arguments, so a bare array or scalar is a caller
/// mistake worth naming rather than coercing.
///
/// # Errors
///
/// [`CoreErrorCode::BadJson`] when stdin is neither empty nor a JSON object.
pub fn parse_request(stdin: &str) -> Result<serde_json::Value, CoreError> {
    if stdin.trim().is_empty() {
        return Ok(serde_json::Value::Object(serde_json::Map::new()));
    }
    let value: serde_json::Value = serde_json::from_str(stdin)
        .map_err(|e| CoreError::new(CoreErrorCode::BadJson, e.to_string()))?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(
            CoreError::new(CoreErrorCode::BadJson, "a request must be a JSON object")
                .with_context("observed_type", json_type_name(&value).to_owned()),
        )
    }
}

/// Read one request off an untrusted stream, bounded.
///
/// The ceiling is [`MAX_REQUEST_BYTES`] and it is applied to the READ, not to
/// what the read produced: the reader is capped at one byte past the ceiling,
/// so a stream of any size beyond it costs `MAX_REQUEST_BYTES + 1` bytes and is
/// refused without a parse, without a re-serialisation and without a clone.
///
/// This is the whole difference between a bound and a remark. Before quoin#448's
/// review, `main.rs` did an uncapped `read_to_string`, `parse_request` built the
/// entire `serde_json::Value`, and only then did an operation measure the
/// request by re-serialising it — a 2 GiB stdin was read, parsed and copied
/// before anything said "too large". `core.ping` never carried more than a
/// token so it never showed; `validators.run` carries a repository.
///
/// A stream that is not UTF-8 is [`CoreErrorCode::BadJson`], the same as any
/// other undecodable request: the boundary speaks JSON text, and a caller
/// cannot act differently on "bad bytes" than on "bad syntax".
///
/// # Errors
///
/// [`CoreErrorCode::Refused`] when the stream exceeds [`MAX_REQUEST_BYTES`],
/// [`CoreErrorCode::BadJson`] when what arrived is not a JSON object, and
/// [`CoreErrorCode::Io`] when the stream itself fails.
pub fn read_request(reader: impl Read) -> Result<serde_json::Value, CoreError> {
    // `+ 1`: reading exactly the ceiling cannot distinguish "exactly at the
    // limit" from "the first `MAX_REQUEST_BYTES` of something larger".
    let ceiling = u64::try_from(MAX_REQUEST_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    // Read BYTES, compare BYTES, decode LAST.
    //
    // `take` cuts at a byte offset, and that offset can fall inside a
    // multi-byte character. Decoding during the read therefore made the
    // classification of an oversized request depend on the CALLER'S ALPHABET:
    // an all-ASCII request one byte over the ceiling was `CORE_REFUSED` (exit
    // 2), while the same request from a caller whose text happened to put a
    // `€` across the cut was `CORE_BAD_JSON` (exit 3) — reported as malformed
    // when it was merely too large, and the message was false besides, because
    // the stream did contain valid UTF-8. `src/core/exec.ts` branches on those
    // statuses, so this is a behaviour difference and not a wording one.
    //
    // Deciding the size first removes the dependency: the length test cannot
    // see an encoding, and the decode below only ever runs on bytes already
    // known to be within the bound (agent-ix/quoin#447; tc_445_104, tc_445_105).
    let mut bytes = Vec::new();
    let read = reader.take(ceiling).read_to_end(&mut bytes).map_err(|e| {
        CoreError::new(CoreErrorCode::Io, e.to_string()).with_context("stream", "stdin")
    })?;

    if read > MAX_REQUEST_BYTES {
        return Err(
            CoreError::new(CoreErrorCode::Refused, "request exceeds the accepted size")
                .with_context("limit_bytes", MAX_REQUEST_BYTES.to_string())
                // What was READ, not what was sent: the stream was abandoned at
                // the ceiling, so the sender's total is a number nobody here
                // observed and naming it would be inventing a fact.
                .with_context("read_bytes", read.to_string()),
        );
    }

    // Within the bound and undecodable is a GENUINE encoding fault, and keeps
    // the malformed-request classification the size test must not borrow.
    let text = String::from_utf8(bytes).map_err(|e| {
        CoreError::new(CoreErrorCode::BadJson, e.to_string()).with_context("stream", "stdin")
    })?;

    parse_request(&text)
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Read the operation name out of the argument vector.
///
/// # Errors
///
/// [`CoreErrorCode::BadUsage`] for anything but exactly one argument shaped
/// `<domain>.<op>`.
pub fn parse_operation(args: &[String]) -> Result<&str, CoreError> {
    let [op] = args else {
        return Err(CoreError::new(
            CoreErrorCode::BadUsage,
            "usage: quoin-core <domain>.<op>  (JSON request on stdin)",
        )
        .with_context("argument_count", args.len().to_string()));
    };
    if op.split('.').filter(|part| !part.is_empty()).count() == 2 && op.matches('.').count() == 1 {
        Ok(op)
    } else {
        Err(CoreError::new(
            CoreErrorCode::BadUsage,
            "an operation is spelled <domain>.<op>",
        )
        .with_context("argument", op.clone()))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests;
