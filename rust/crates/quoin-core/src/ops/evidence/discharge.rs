// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Discharge facts in the evidence store (FR-133).
//!
//! Two operations: `evidence.record_discharge` persists the facts a person
//! attested, and `evidence.discharge_report` re-derives the partition from a
//! live clause-binding report plus the stored facts. The store never holds the
//! clause list.

use std::path::Path;

use quoin_assurance::{
    DischargeStoreError, RecordDischargeRequest, discharge_report_from_store,
    inward_completeness_from_store, record_discharge_facts, render_inward_completeness,
};
use quoin_quire_types::ClauseBindingReport;
use serde::{Deserialize, Serialize};

use super::taxonomy::map_error;
use super::{absolute, bound, check_scalar, host, parse, value};
use crate::capabilities::Capabilities;
use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::Response;

/// The largest `evidence.record_discharge` or `evidence.discharge_report`
/// request, in bytes.
pub const MAX_DISCHARGE_BYTES: usize = 8 * 1024 * 1024;

/// The request accepted by `evidence.discharge_report`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DischargeReportRequest {
    /// The repository root.
    pub repo: String,
    /// The live `clause-binding-v1` report.
    pub binding: ClauseBindingReport,
    /// The explicit evaluation instant.
    pub as_of: String,
}

/// The request accepted by `evidence.record_discharge`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RecordDischargeOpRequest {
    /// The repository root.
    pub repo: String,
    /// The live `clause-binding-v1` report the facts are made against.
    pub binding: ClauseBindingReport,
    /// Unvalidated discharge facts.
    pub facts: Vec<serde_json::Value>,
}

/// The result of an accepted `evidence.record_discharge`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RecordDischargePayload {
    /// The facts written, each with the absolute path it was written to.
    pub recorded: Vec<RecordedDischargeFact>,
}

/// One written fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RecordedDischargeFact {
    /// The clause the fact is about.
    pub clause_id: String,
    /// Where it was written, as an **absolute** path.
    pub path: String,
}

fn map_store_error(error: DischargeStoreError, op: &'static str) -> CoreError {
    match error {
        DischargeStoreError::Discharge(error) => {
            CoreError::new(CoreErrorCode::BadRequest, error.to_string()).with_context("op", op)
        }
        DischargeStoreError::Refused { kind, detail } => {
            CoreError::new(CoreErrorCode::BadRequest, detail)
                .with_context("op", op)
                .with_context("refusal", kind.as_str())
        }
        DischargeStoreError::Evidence(error) => map_error(&error, op),
    }
}

/// Answer an `evidence.record_discharge`.
///
/// # Errors
///
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_DISCHARGE_BYTES`] or the store cannot be written.
/// - [`CoreErrorCode::BadRequest`] when a fact or the instant is refused by the
///   FR-046 predicates, in which case nothing is written.
/// - [`CoreErrorCode::Io`] when no evidence host was granted.
pub fn record_discharge(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "evidence.record_discharge";
    bound(request, op, MAX_DISCHARGE_BYTES)?;
    let request: RecordDischargeOpRequest = parse(request, op)?;
    check_scalar(op, "repo", &request.repo)?;
    let record = RecordDischargeRequest {
        binding: request.binding,
        facts: request.facts,
    };

    let host = host(capabilities, op)?;
    let root = host.store_root(Path::new(&request.repo));
    let payload = host.with_store(Path::new(&request.repo), &mut |source| {
        let recorded =
            record_discharge_facts(source, &record).map_err(|e| map_store_error(e, op))?;
        value(&RecordDischargePayload {
            recorded: recorded
                .into_iter()
                .map(|fact| RecordedDischargeFact {
                    clause_id: fact.clause_id,
                    path: absolute(&root, &fact.path),
                })
                .collect(),
        })
    })?;
    Ok(Response::ok(payload))
}

/// Answer an `evidence.discharge_report`: the clause-discharge report for a
/// live binding report, with the stored facts as the facts.
///
/// # Errors
///
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_DISCHARGE_BYTES`] or a stored file cannot be read.
/// - [`CoreErrorCode::BadRequest`] when `asOf` is refused.
/// - [`CoreErrorCode::Io`] when no evidence host was granted.
pub fn discharge_report(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "evidence.discharge_report";
    bound(request, op, MAX_DISCHARGE_BYTES)?;
    let request: DischargeReportRequest = parse(request, op)?;
    check_scalar(op, "repo", &request.repo)?;

    let host = host(capabilities, op)?;
    let payload = host.with_store(Path::new(&request.repo), &mut |source| {
        let report = discharge_report_from_store(&*source, &request.binding, &request.as_of)
            .map_err(|e| map_store_error(e, op))?;
        value(&report)
    })?;
    Ok(Response::ok(payload))
}

/// The result of an `evidence.inward_completeness`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct InwardCompletenessPayload {
    /// The accounting of the binding population.
    pub completeness: quoin_assurance::InwardCompleteness,
    /// The same accounting as deterministic Markdown.
    pub rendered: String,
}

/// Answer an `evidence.inward_completeness`: account for the binding
/// population of a live binding report against the stored facts (FR-133).
///
/// The report is built inside the operation from the live binding report, so
/// the populations add up by construction and the caller supplies no
/// snapshot of the obligations.
///
/// # Errors
///
/// As [`discharge_report`].
pub fn inward_completeness(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "evidence.inward_completeness";
    bound(request, op, MAX_DISCHARGE_BYTES)?;
    let request: DischargeReportRequest = parse(request, op)?;
    check_scalar(op, "repo", &request.repo)?;

    let host = host(capabilities, op)?;
    let payload = host.with_store(Path::new(&request.repo), &mut |source| {
        let completeness =
            inward_completeness_from_store(&*source, &request.binding, &request.as_of)
                .map_err(|e| map_store_error(e, op))?;
        value(&InwardCompletenessPayload {
            rendered: render_inward_completeness(&completeness),
            completeness,
        })
    })?;
    Ok(Response::ok(payload))
}
