// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `auditor`: the FR-032 audit, the baseline it accepts, and the
//! FR-054 advice (quoin#501).
//!
//! Replaces `src/auditor/` (1,160 lines) and `src/advisor/` (712), which
//! `src/commands/{advise,assurance}.ts` and `src/commands/evidence/{audit,
//! baseline}.ts` reached directly. What they computed is `quoin-auditor`'s;
//! what this module does is read a request, call one crate function, and hand
//! back its answer.
//!
//! # No host capability, by construction
//!
//! `quoin_auditor` is [`Inert`](quoin_auditor::Inert): no handle, no path to
//! open, no command to run reaches any function here. So unlike
//! [`crate::ops::graph`], this domain is granted nothing and `main.rs` has
//! nothing to give it. The whole evidence store rides on stdin as data the
//! caller already read, which is the property that lets the auditor be audited
//! — an advisor that could reach the filesystem could also disagree with the
//! auditor about what it found (ADR-0011).
//!
//! # Three operations, not six functions
//!
//! The retained commands imported six symbols: `audit`, `ratchet`,
//! `findingKey`, `scoresFor`, `advise` and `uncataloguedAuthoredMethods`. Only
//! three questions are asked of them, one per command, and the unit of IPC is a
//! command-shaped operation rather than a function (quoin#373). So `ratchet`
//! rides inside [`audit`], `findingKey` inside [`baseline`], and `scoresFor`
//! and `uncataloguedAuthoredMethods` inside [`advise`] — where
//! `quoin_auditor::advise_all` keeps the advisor and the auditor sharing one
//! definition of a fault-detection score.
//!
//! Provenance: quoin#501

mod taxonomy;
mod wire;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use quoin_auditor::{
    advise_all, audit as run_audit, finding_key, mintable_characteristics, ratchet,
    uncatalogued_authored_methods,
};
use quoin_evidence::types::IndependenceAssessment;

use crate::error::{CoreError, CoreErrorCode};
use crate::ops::{refusal, request_size};
use crate::protocol::Response;

use self::taxonomy::map_error;
pub use self::wire::{
    AdvisePayload, AdviseRequest, AuditPayload, AuditRequest, BaselinePayload, BaselineRequest,
    MAX_AUDITOR_REQUEST_BYTES, VocabularyPayload,
};

/// Answer an `auditor.audit`: every FR-032 finding over the store, ratcheted
/// against an accepted baseline when the caller read one.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not an [`AuditRequest`].
/// - [`CoreErrorCode::Refused`] when the request exceeds its ceiling, or when
///   the auditor declines the input it was given.
pub fn audit(request: &serde_json::Value) -> Result<Response, CoreError> {
    let op = "auditor.audit";
    let request: AuditRequest = read(request, op)?;
    let mut report = run_audit(&request.input).map_err(|error| map_error(&error, op))?;
    // MOVED, not copied. `quoin-auditor` writes its assessments into the
    // report's passthrough map because the RETAINED store bytes are wider than
    // the declared type (quoin#383); the boundary is answering what it just
    // computed, so it declares the member instead of passing an undeclared key
    // no generated TypeScript type carries. Leaving a copy behind would encode
    // the same list twice.
    let independence = report
        .other
        .remove("independence")
        .map(serde_json::from_value::<Vec<IndependenceAssessment>>)
        .transpose()
        .map_err(|error| {
            CoreError::new(
                CoreErrorCode::Io,
                "the audit's independence assessments are not readable back",
            )
            .with_context("operation", op)
            .with_context("detail", error.to_string())
        })?;
    // `Some([])` and `None` are different answers: an empty baseline accepted
    // nothing, so every finding is new; no baseline means none was read.
    let reported = request
        .accepted
        .as_ref()
        .map(|accepted| ratchet(&report, accepted).into_iter().cloned().collect());
    ok(&AuditPayload {
        report,
        independence,
        reported,
    })
}

/// Answer an `auditor.baseline`: the key of every finding the store shows
/// today, sorted, as `quoin evidence baseline` writes them.
///
/// # Errors
///
/// As [`audit`], for a [`BaselineRequest`].
pub fn baseline(request: &serde_json::Value) -> Result<Response, CoreError> {
    let op = "auditor.baseline";
    let request: BaselineRequest = read(request, op)?;
    let report = run_audit(&request.input).map_err(|error| map_error(&error, op))?;
    let mut accepted: Vec<String> = report.findings.iter().map(finding_key).collect();
    accepted.sort();
    ok(&BaselinePayload { accepted })
}

/// Answer an `auditor.vocabulary`: what the fact set can ever mint.
///
/// # Errors
///
/// Only if the payload will not serialise. There is nothing to parse — the
/// request carries no members, because the answer is a property of this build
/// and not of anything a caller could ask about.
pub fn vocabulary(_request: &serde_json::Value) -> Result<Response, CoreError> {
    ok(&VocabularyPayload {
        mintable_characteristics: mintable_characteristics().into_iter().collect(),
    })
}

/// Answer an `auditor.advise`: one FR-054 recommendation per obligation.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not an [`AdviseRequest`].
/// - [`CoreErrorCode::Refused`] when the request exceeds its ceiling.
///
/// `advise_all` itself is total: an obligation no rule matched is advised as
/// inconclusive, which is an answer rather than a failure.
pub fn advise(request: &serde_json::Value) -> Result<Response, CoreError> {
    let op = "auditor.advise";
    let request: AdviseRequest = read(request, op)?;
    let uncatalogued = uncatalogued_authored_methods(&request.diagnostics);
    let advice = advise_all(
        &request.catalog,
        &request.obligations,
        &request.shapes,
        &request.bindings,
        &request.runs,
        &uncatalogued,
    );
    ok(&AdvisePayload {
        advice,
        degraded: uncatalogued.degraded,
    })
}

/// Bound a request, then parse it.
fn read<T: for<'de> Deserialize<'de>>(
    request: &serde_json::Value,
    op: &'static str,
) -> Result<T, CoreError> {
    let size = request_size(request)?;
    if size > MAX_AUDITOR_REQUEST_BYTES {
        return Err(refusal(op, MAX_AUDITOR_REQUEST_BYTES, size));
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
