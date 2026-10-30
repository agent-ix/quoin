// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `evidence` operations that READ or MAINTAIN the store.
//!
//! Collecting superseded records and the two bulk reads the command layer
//! makes one call each for — the trust assessments and the pure auditor's
//! whole input.

use std::path::Path;

use quoin_evidence::independence::assess_independence;
use quoin_evidence::mock_inspection::mock_inspection_input;
use quoin_evidence::store::{
    latest_runs, latest_scans, read_bindings, read_trust_decisions, scan_is_vacuous,
};
use quoin_evidence::trust::assess_trust;
use quoin_evidence::types::Binding;
use quoin_evidence::{Commit, SuiteId};

use crate::capabilities::Capabilities;
use crate::error::CoreError;
use crate::protocol::Response;

use super::taxonomy::map_error;
use super::wire::{
    AuditInputsPayload, AuditInputsRequest, GcPayload, GcRequest, MAX_AUDIT_INPUTS_BYTES,
    MAX_GC_BYTES, MAX_TRUST_ASSESSMENTS_BYTES, RepoRequest, TrustAssessmentsPayload,
};
use super::{absolute, bound, check_scalar, host, parse, value};

// ─────────────────────────── store, host-granted ───────────────────────────

/// Answer an `evidence.gc`.
///
/// # Errors
///
/// - [`CoreErrorCode::Refused`] when the request exceeds [`MAX_GC_BYTES`], or
///   when the store cannot be listed or a file cannot be removed.
/// - [`CoreErrorCode::Io`] when no evidence host was granted.
pub fn gc(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "evidence.gc";
    bound(request, op, MAX_GC_BYTES)?;
    let request: GcRequest = parse(request, op)?;
    check_scalar(op, "repo", &request.repo)?;

    let host = host(capabilities, op)?;
    let root = host.store_root(Path::new(&request.repo));
    let dry_run = request.dry_run;
    let payload = host.with_store(Path::new(&request.repo), &mut |source| {
        let relative = quoin_evidence::store::gc(source, dry_run).map_err(|e| map_error(&e, op))?;
        // `gc` returns store-relative paths because it names no host
        // capability; the caller prints them for a person to act on, and a
        // relative path there is a wrong path.
        let deleted = relative
            .into_iter()
            .map(|path| absolute(&root, path))
            .collect();
        value(&GcPayload { deleted })
    })?;
    Ok(Response::ok(payload))
}

/// Answer an `evidence.trust_assessments`.
///
/// # Errors
///
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_TRUST_ASSESSMENTS_BYTES`], or when `trust/` cannot be listed.
/// - [`CoreErrorCode::Io`] when no evidence host was granted.
pub fn trust_assessments(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "evidence.trust_assessments";
    bound(request, op, MAX_TRUST_ASSESSMENTS_BYTES)?;
    let request: RepoRequest = parse(request, op)?;
    check_scalar(op, "repo", &request.repo)?;

    let host = host(capabilities, op)?;
    let payload = host.with_store(Path::new(&request.repo), &mut |source| {
        let mut unreadable = Vec::new();
        let decisions =
            read_trust_decisions(source, &mut unreadable).map_err(|e| map_error(&e, op))?;
        let mut assessments = Vec::with_capacity(decisions.len());
        for decision in &decisions {
            assessments.push(assess_trust(decision).map_err(|e| map_error(&e, op))?);
        }
        value(&TrustAssessmentsPayload {
            assessments,
            unreadable,
        })
    })?;
    Ok(Response::ok(payload))
}

/// Answer an `evidence.audit_inputs`.
///
/// Everything the pure auditor reads from the store, in one call — including
/// the two questions it would otherwise ask inside per-obligation loops: which
/// suites' newest scan evaluated no rules, and how each policy requirement
/// assesses over its obligation's bindings. One subprocess per obligation would
/// have made the cost of the boundary proportional to the corpus.
///
/// # Errors
///
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_AUDIT_INPUTS_BYTES`], or when the store cannot be read.
/// - [`CoreErrorCode::Io`] when no evidence host was granted.
pub fn audit_inputs(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "evidence.audit_inputs";
    bound(request, op, MAX_AUDIT_INPUTS_BYTES)?;
    let request: AuditInputsRequest = parse(request, op)?;
    check_scalar(op, "repo", &request.repo)?;
    check_scalar(
        op,
        "head_commit",
        request.head_commit.as_deref().unwrap_or(""),
    )?;

    let host = host(capabilities, op)?;
    let payload = host.with_store(Path::new(&request.repo), &mut |source| {
        let mut skipped = Vec::new();
        let bindings = read_bindings(source)
            .map_err(|e| map_error(&e, op))?
            .bindings;
        let runs = latest_runs(source, &mut skipped).map_err(|e| map_error(&e, op))?;
        let scans = latest_scans(source, &mut skipped).map_err(|e| map_error(&e, op))?;
        let head = request.head_commit.as_deref().map(Commit::new);
        let (mock_inspection_suites, injections) =
            mock_inspection_input(source, head.as_ref()).map_err(|e| map_error(&e, op))?;

        // `None` — the tool reported no rule count — stays out of this list.
        // The check is silent for it rather than guessing, which is the same
        // posture method conformance takes when no evidence kind is declared.
        let vacuous_scan_suites: Vec<SuiteId> = scans
            .iter()
            .filter(|scan| scan_is_vacuous(scan) == Some(true))
            .map(|scan| scan.suite.clone())
            .collect();

        let independence = request
            .independence_policy
            .as_ref()
            .map_or_else(Vec::new, |policy| {
                policy
                    .requirements
                    .iter()
                    .map(|requirement| {
                        let for_obligation: Vec<Binding> = bindings
                            .iter()
                            .filter(|binding| binding.obligation == requirement.obligation)
                            .cloned()
                            .collect();
                        assess_independence(&policy.profile, requirement, &for_obligation)
                    })
                    .collect()
            });

        skipped.sort();
        skipped.dedup();
        value(&AuditInputsPayload {
            bindings: bindings.clone(),
            runs,
            scans,
            injections,
            mock_inspection_suites,
            vacuous_scan_suites,
            independence,
            skipped,
        })
    })?;
    Ok(Response::ok(payload))
}
