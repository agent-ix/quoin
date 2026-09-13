// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! What the six operations in [`super`] share.
//!
//! Split out at the module-size ceiling (quoin#457): the operations file states
//! the six answers and this one states the machinery they are built from. The
//! seam is real and not just a line count — nothing here knows which operation
//! called it, and everything here is decided with no disk at all.
//!
//! None of it duplicates `quoin-store`. Canonicalization, digests, store paths
//! and atomic writes are that crate's, and are used from it; what remains is
//! the hex transport this boundary defines, the refusals the wire shapes cannot
//! express, and the small adapters between `JsonValue` and `serde_json`.

use serde::{Deserialize, Serialize};

use quoin_change_assurance::verify::{
    AuditReport, ReportFinding, ReportUnevaluated, RetainedAudit,
};
use quoin_change_assurance::{EvidenceStore, read_change_record};
use quoin_store::{CanonicalDigest, JsonValue, canonical_json_bytes, parse_strict_json};

use crate::capabilities::{Capabilities, ChangeAssuranceHost};
use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::Response;

use super::taxonomy::map_error;
use super::wire::AuditInput;

/// Read one stored record, refusing an unknown digest rather than skipping it.
pub(super) fn stored_record(
    store: &dyn EvidenceStore,
    digest: &str,
    field: &'static str,
    op: &'static str,
) -> Result<JsonValue, CoreError> {
    let parsed = digest_of(digest, field, op)?;
    let record = read_change_record(store, &parsed)
        .map_err(|e| map_error(&e, op))?
        .ok_or_else(|| missing(op, field, digest, ""))?;
    record.to_json().map_err(|e| map_error(&e, op))
}

/// A stored digest, or the caller's spelling refused.
pub(super) fn digest_of(
    value: &str,
    field: &'static str,
    op: &'static str,
) -> Result<CanonicalDigest, CoreError> {
    CanonicalDigest::parse_stored(value).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string())
            .with_context("op", op)
            .with_context("field", field)
    })
}

/// Evidence the caller named and the store does not hold.
///
/// `Refused` (2) and not `BadRequest` (3): the request was understood and the
/// spelling was fine. What is missing is in the store, which is the world.
pub(super) fn missing(
    op: &'static str,
    field: &'static str,
    digest: &str,
    repo: &str,
) -> CoreError {
    let error = CoreError::new(
        CoreErrorCode::Refused,
        "no evidence is retained under that digest",
    )
    .with_context("op", op)
    .with_context("field", field)
    .with_context("digest", digest.to_owned());
    if repo.is_empty() {
        error
    } else {
        error.with_context("repo", repo.to_owned())
    }
}

/// Read `--audits` into the shape a verification takes.
///
/// Declarative rather than hand-walked: `serde` states the shape once and
/// refuses everything else, including an undeclared member. The retained
/// TypeScript asserted nothing here and passed whatever it parsed straight in;
/// see `DIVERGENCE.md`.
pub(super) fn read_audits(
    document: &JsonValue,
    op: &'static str,
) -> Result<Vec<RetainedAudit>, CoreError> {
    let value = to_serde(document, op)?;
    let parsed: Vec<AuditInput> = serde_json::from_value(value).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string())
            .with_context("op", op)
            .with_context("field", "audits_hex")
    })?;
    Ok(parsed
        .into_iter()
        .map(|audit| RetainedAudit {
            proof_id: audit.proof_id,
            report_digest: audit.report_digest,
            report: AuditReport {
                findings: audit
                    .report
                    .findings
                    .into_iter()
                    .map(|finding| ReportFinding {
                        obligation: finding.obligation,
                        kind: finding.kind,
                    })
                    .collect(),
                healthy: audit.report.healthy,
                unevaluated: audit
                    .report
                    .unevaluated
                    .into_iter()
                    .map(|entry| ReportUnevaluated {
                        obligation: entry.obligation,
                    })
                    .collect(),
            },
        })
        .collect())
}

/// Refuse a body that supplies a field the caller must not state.
///
/// The sealing functions derive these members, so without this check a caller
/// could hand in a wrong digest and receive a cleanly sealed value back, having
/// been told nothing.
pub(super) fn refuse_supplied(
    body: &JsonValue,
    fields: &[&str],
    op: &'static str,
) -> Result<(), CoreError> {
    let object = body.as_object().map_err(|e| map_error(&e.into(), op))?;
    let supplied: Vec<&str> = fields
        .iter()
        .copied()
        .filter(|field| object.get(field).is_some())
        .collect();
    if supplied.is_empty() {
        return Ok(());
    }
    Err(
        CoreError::new(CoreErrorCode::BadRequest, "a derived member was supplied")
            .with_context("op", op)
            .with_context("supplied", supplied.join(",")),
    )
}

/// Decode one hex-encoded document and read it with the strict JSON reader.
///
/// The strict reader is applied to the bytes the producer supplied, which is
/// the whole reason they cross as hex — see [`wire`].
pub(super) fn strict_document(
    hex: &str,
    op: &'static str,
    field: &'static str,
) -> Result<JsonValue, CoreError> {
    let bytes = decode_hex(hex, op, field)?;
    parse_strict_json(&bytes).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string())
            .with_context("op", op)
            .with_context("field", field)
    })
}

/// Decode lowercase or uppercase hexadecimal into bytes.
pub(super) fn decode_hex(
    hex: &str,
    op: &'static str,
    field: &'static str,
) -> Result<Vec<u8>, CoreError> {
    let bytes = hex.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err(
            CoreError::new(CoreErrorCode::BadRequest, "a hex field has an odd length")
                .with_context("op", op)
                .with_context("field", field),
        );
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for &[first, second] in bytes.as_chunks::<2>().0 {
        let (Some(high), Some(low)) = (nibble(first), nibble(second)) else {
            return Err(CoreError::new(
                CoreErrorCode::BadRequest,
                "a hex field holds a character that is not hexadecimal",
            )
            .with_context("op", op)
            .with_context("field", field));
        };
        out.push(high * 16 + low);
    }
    Ok(out)
}

/// One hexadecimal digit's value.
fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// A byte count as the JSON number a retained output's `size_bytes` is.
///
/// Sizes are bounded by [`MAX_INTAKE_BYTES`] long before they reach this, so
/// the conversion is exact for every value that can actually arrive; the
/// saturating form is what keeps it total.
#[allow(
    clippy::cast_precision_loss,
    reason = "bounded by MAX_INTAKE_BYTES, far below 2^53"
)]
pub(super) fn size_bytes_as_f64(size: u64) -> f64 {
    size as f64
}

/// Convert a `quoin-store` document into the `serde_json` value a payload
/// carries.
///
/// Through the canonical bytes rather than field by field: the canonicaliser is
/// the one this store already seals with, so a payload cannot disagree with the
/// digest that was taken over the same value.
pub(super) fn to_serde(
    value: &JsonValue,
    op: &'static str,
) -> Result<serde_json::Value, CoreError> {
    let bytes = canonical_json_bytes(value)
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()).with_context("op", op))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()).with_context("op", op))
}

/// Parse a request, naming the operation on the refusal.
pub(super) fn parse<T: for<'de> Deserialize<'de>>(
    request: &serde_json::Value,
    op: &'static str,
) -> Result<T, CoreError> {
    serde_json::from_value(request.clone()).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
    })
}

/// Serialise a payload into a clean success.
pub(super) fn ok<T: Serialize>(payload: &T) -> Result<Response, CoreError> {
    let value = serde_json::to_value(payload)
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;
    Ok(Response::ok(value))
}

/// The granted change-assurance host, or the internal fault of having none.
pub(super) fn host<'a>(
    capabilities: &Capabilities<'a>,
    op: &'static str,
) -> Result<&'a dyn ChangeAssuranceHost, CoreError> {
    capabilities.change_assurance.ok_or_else(|| {
        // Internal (4), not Refused (2): the caller did nothing wrong. This is
        // `main.rs` having failed to grant a capability the operation table
        // says this operation needs, and it must read as a build fault rather
        // than as "quoin declined to read your store".
        CoreError::new(
            CoreErrorCode::Io,
            "this build dispatched a change-assurance operation without granting a store",
        )
        .with_context("op", op)
    })
}

/// Refuse an oversized field before any work is done on it.
pub(super) fn check_bound(
    op: &'static str,
    field: &str,
    value: &str,
    limit: usize,
) -> Result<(), CoreError> {
    if value.len() > limit {
        return Err(CoreError::new(
            CoreErrorCode::Refused,
            "a request field exceeds the accepted size",
        )
        .with_context("op", op)
        .with_context("field", field.to_owned())
        .with_context("limit_bytes", limit.to_string())
        .with_context("observed_bytes", value.len().to_string()));
    }
    Ok(())
}
