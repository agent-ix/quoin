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
    DischargeStoreError, RecordDischargeRequest, build_pack_requirements_mapping,
    build_requirements_mapping, discharge_report_from_store, inward_completeness_from_store,
    record_discharge_facts, render_inward_completeness, render_pack_requirements_mapping,
    render_requirements_mapping,
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

/// The result of an `evidence.requirements_matrix`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RequirementsMatrixPayload {
    /// One row per clause of the binding report.
    pub matrix: quoin_assurance::RequirementsMapping,
    /// The same matrix as deterministic Markdown.
    pub rendered: String,
}

/// Answer an `evidence.requirements_matrix`: the requirements mapping matrix
/// of a live binding report against the stored facts (FR-135).
///
/// # Errors
///
/// As [`discharge_report`].
pub fn requirements_matrix(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "evidence.requirements_matrix";
    bound(request, op, MAX_DISCHARGE_BYTES)?;
    let request: DischargeReportRequest = parse(request, op)?;
    check_scalar(op, "repo", &request.repo)?;

    let host = host(capabilities, op)?;
    let payload = host.with_store(Path::new(&request.repo), &mut |source| {
        let report = discharge_report_from_store(&*source, &request.binding, &request.as_of)
            .map_err(|e| map_store_error(e, op))?;
        let matrix = build_requirements_mapping(&request.binding, &report)
            .map_err(|e| map_store_error(DischargeStoreError::Discharge(e), op))?;
        value(&RequirementsMatrixPayload {
            rendered: render_requirements_mapping(&matrix),
            matrix,
        })
    })?;
    Ok(Response::ok(payload))
}

/// The request accepted by `evidence.pack_requirements_matrix`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PackRequirementsMatrixRequest {
    /// The installed module name.
    pub pack: String,
    /// The repository root holding the evidence store.
    pub repo: String,
    /// The explicit evaluation instant.
    pub as_of: String,
    /// `KEY=VALUE` context entries shared by every clause set.
    #[serde(default)]
    pub context: Vec<String>,
}

/// The result of an `evidence.pack_requirements_matrix`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PackRequirementsMatrixPayload {
    /// The index and one matrix per clause set.
    pub report: quoin_assurance::PackRequirementsMapping,
    /// The same report as deterministic Markdown.
    pub rendered: String,
}

/// Answer an `evidence.pack_requirements_matrix`: one FR-135 matrix for every
/// clause set of an installed pack, behind an index (FR-140).
///
/// # Errors
///
/// - [`CoreErrorCode::Refused`] when the name is not an installed module, the
///   module is not a pack, or the pack fails to load.
/// - [`CoreErrorCode::BadRequest`] when a context entry is malformed or a
///   stored fact is refused.
/// - [`CoreErrorCode::Io`] when no module or evidence host was granted.
pub fn pack_matrix(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "evidence.pack_requirements_matrix";
    bound(request, op, MAX_DISCHARGE_BYTES)?;
    let request: PackRequirementsMatrixRequest = parse(request, op)?;
    check_scalar(op, "repo", &request.repo)?;
    check_scalar(op, "pack", &request.pack)?;

    let context = quoin_quire::clauses::Context::from_entries(&request.context).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
    })?;
    let refused = |message: String| {
        CoreError::new(CoreErrorCode::Refused, message)
            .with_context("op", op)
            .with_context("pack", request.pack.as_str())
    };
    let modules = capabilities.modules.ok_or_else(|| {
        CoreError::new(
            CoreErrorCode::Io,
            "this build dispatched a pack operation without granting a module host",
        )
        .with_context("op", op)
    })?;
    let installed = modules
        .list(None)
        .map_err(|e| refused(format!("the installed modules cannot be listed: {e}")))?
        .into_iter()
        .find(|module| module.name.as_str() == request.pack)
        .ok_or_else(|| refused(format!("{} is not an installed module", request.pack)))?;
    let is_pack = modules
        .is_pack(&installed)
        .map_err(|e| refused(format!("module {} failed to load: {e}", request.pack)))?;
    if !is_pack {
        return Err(refused(format!("module {} is not a pack", request.pack)));
    }
    let bindings = modules
        .evaluate_clause_sets(&installed, &context)
        .map_err(|e| refused(format!("pack {} failed to load: {e}", request.pack)))?;

    let host = host(capabilities, op)?;
    let payload = host.with_store(Path::new(&request.repo), &mut |source| {
        let report = build_pack_requirements_mapping(
            &*source,
            &request.pack,
            &request.as_of,
            context.as_map(),
            &bindings,
        )
        .map_err(|e| map_store_error(e, op))?;
        value(&PackRequirementsMatrixPayload {
            rendered: render_pack_requirements_mapping(&report),
            report,
        })
    })?;
    Ok(Response::ok(payload))
}
