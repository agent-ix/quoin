// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `evidence`: the evidence store, its producer adapters, and the
//! trust and independence judgements read off it (quoin#458, Stage 9).
//!
//! Replaces `src/evidence/`'s TypeScript. What it decided — which adapter reads
//! a producer document, what a run record and a binding look like, when a
//! binding becomes suspect, which records `gc` may collect, whether a trust
//! decision is still in force and whether two evidence lines are separated —
//! is decided by `quoin-evidence` and reported through here.
//!
//! # No host capability, and how
//!
//! An evidence store is a directory tree: one directory per suite under each of
//! `runs/`, `scans/` and `mock-inspections/`, plus `trust/`, `experiments/`,
//! `operational/` and two checked-in JSON files. `gc` lists every one of them;
//! `mock_inspection_input` reads a record per suite; an inspection walks the
//! repository's source. Those bytes cannot ride on stdin, so this module does
//! not read them: it is **granted** an [`EvidenceHost`] by `main.rs`, and the
//! host opens the store for the length of one action and closes it again.
//!
//! The consequence is worth stating plainly, because it is the property the
//! containment audit protects: **nothing in this file names a path**. The
//! operations below receive a repository root as an opaque string and hand it
//! straight to the host; every path in a payload is one `quoin-evidence`
//! produced, and the two that are absolute are made absolute by
//! [`EvidenceHost::store_root`]. Every test in [`tests`] substitutes a
//! `MemoryEvidence`-backed host and touches no disk at all.
//!
//! # Two divergences from the retained TypeScript, declared
//!
//! 1. **`gc` paths.** `quoin_evidence::store::gc` returns STORE-RELATIVE paths;
//!    the retained `gc()` returned absolute ones. The library is right — it
//!    names no host capability and so cannot know where the store is — and the
//!    join happens in [`gc`] below, against the root the host reports. Without
//!    it every path the caller printed would be wrong.
//!
//! 2. **Typed deserialization skips malformed records.** The retained reader
//!    handed back a structurally malformed record half-formed, with whatever
//!    fields happened to parse; the port refuses to deserialize it, skips it,
//!    and names its path in `skipped`. That is an observable behaviour change
//!    and it is reported rather than hidden: a caller that used to get a record
//!    with an absent `entries` array now gets a path it can go and look at.
//!
//! The domain is split the way it reads: [`wire`] holds the request and payload
//! shapes together with the ceilings they are read under, [`taxonomy`] holds the
//! one place an `EvidenceError` becomes an exit status, and this file holds the
//! operations and the helpers they share.

mod documents;
mod records;
mod store;
mod taxonomy;
mod wire;

#[cfg(test)]
mod tests;

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::capabilities::{Capabilities, EvidenceHost};
use crate::error::{CoreError, CoreErrorCode};
use crate::ops::{refusal, request_size};
use crate::protocol::Response;

pub use self::wire::{
    AffirmPayload, AffirmRequest, AssuranceRecordPayload, AssuranceRecordRequest,
    AuditInputsPayload, AuditInputsRequest, GcPayload, GcRequest, InspectMocksPayload,
    InspectMocksRequest, MAX_AFFIRM_BYTES, MAX_ASSURANCE_RECORD_BYTES, MAX_AUDIT_INPUTS_BYTES,
    MAX_GC_BYTES, MAX_INSPECT_MOCKS_BYTES, MAX_PARSE_LINEAGE_BYTES, MAX_PARSE_POLICY_BYTES,
    MAX_PARSE_RESULTS_BYTES, MAX_READ_BASELINE_BYTES, MAX_RECORD_BYTES, MAX_SCALAR_BYTES,
    MAX_STORE_FACTS_BYTES, MAX_TRUST_ASSESSMENTS_BYTES, MAX_TRUST_DECISION_BYTES,
    MAX_WRITE_BASELINE_BYTES, ParseLineagePayload, ParseLineageRequest, ParsePolicyPayload,
    ParsePolicyRequest, ParseResultsPayload, ParseResultsRequest, ReadBaselinePayload,
    RecordPayload, RecordRequest, RepoRequest, StoreFactsPayload, TrustAssessmentsPayload,
    TrustDecisionPayload, TrustDecisionRequest, UnrepresentedView, WriteBaselinePayload,
    WriteBaselineRequest,
};

pub use self::documents::{parse_lineage, parse_policy, parse_results, store_facts};
pub use self::records::{
    inspect_mocks, record, record_experiment, record_operational, trust_decision,
};
pub use self::store::{
    affirm_binding, audit_inputs, gc, read_baseline, trust_assessments, write_baseline,
};

/// One store-relative path as the caller must see it: absolute.
///
/// `quoin_evidence` names no host capability, so every path it hands back is
/// relative to the store root. The retained implementation returned absolute
/// paths and the commands print them for a person to open, so the join happens
/// here — once, in one function, against the root the host reports. Doing it
/// per operation is how one of them ends up relative.
fn absolute(root: &Path, relative: impl AsRef<Path>) -> String {
    root.join(relative).to_string_lossy().into_owned()
}

/// Refuse an oversized request before it is deserialised.
fn bound(request: &serde_json::Value, op: &'static str, limit: usize) -> Result<(), CoreError> {
    let size = request_size(request)?;
    if size > limit {
        return Err(refusal(op, limit, size));
    }
    Ok(())
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
    Ok(Response::ok(value(payload)?))
}

/// Serialise a payload, without wrapping it in a response.
///
/// Separate from [`ok`] because the host seam returns a `Value`: the borrow of
/// the store has to end before the response leaves the closure.
fn value<T: Serialize>(payload: &T) -> Result<serde_json::Value, CoreError> {
    serde_json::to_value(payload).map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))
}

/// The granted evidence host, or the internal fault of having none.
fn host<'a>(
    capabilities: &Capabilities<'a>,
    op: &'static str,
) -> Result<&'a dyn EvidenceHost, CoreError> {
    capabilities.evidence.ok_or_else(|| {
        // Internal (4), not Refused (2): the caller did nothing wrong. This is
        // `main.rs` having failed to grant a capability the operation table
        // says this operation needs, and it must read as a build fault rather
        // than as "quoin declined to read your store".
        CoreError::new(
            CoreErrorCode::Io,
            "this build dispatched an evidence operation without granting an evidence host",
        )
        .with_context("op", op)
    })
}

/// Refuse an oversized field before any work is done on it.
fn check_scalar(op: &'static str, field: &str, value: &str) -> Result<(), CoreError> {
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
