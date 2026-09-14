// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `change_assurance`: sealed change records, proof attestations,
//! retained evidence and verification receipts (quoin#457, Stage 9).
//!
//! Replaces `src/change-assurance/`'s TypeScript. What it decided — whether a
//! record satisfies the FR-063 schema, whether an attestation's bytes reproduce
//! what it declares, whether a parent chain is strict N-1, and what verdict a
//! candidate's retained evidence comes to — is decided by
//! `quoin-change-assurance` and reported through here.
//!
//! # What is a decision and what is a capability
//!
//! Everything this module decides on its own is decided with no disk at all: a
//! well-formed request, a field within its ceiling, a document that parses
//! strictly, and how a `ChangeAssuranceError` maps onto the exit taxonomy. The
//! one thing it cannot do is touch the evidence store, so it is **granted** a
//! [`ChangeAssuranceHost`] by `main.rs`, whose single method hands back an
//! [`EvidenceStore`] rooted at the repository the caller named. The unit tests
//! below run the whole domain against an in-memory store. See
//! [`crate::capabilities`].
//!
//! # A verdict is not a failure
//!
//! `change_assurance.receipt` answers `Ok` (0) whether the receipt is `valid`,
//! `invalid` or `incomplete`. A receipt IS the answer, and the exit-1 policy
//! that `quoin change-assurance receipt` applies to a non-`valid` one is the
//! caller's, derived from the payload — the same split `validators.run` makes
//! between a finding and a failure.
//!
//! The domain is split the way it reads: [`wire`] holds the request and payload
//! shapes together with the ceilings they are read under, [`taxonomy`] holds
//! the one place a `ChangeAssuranceError` becomes an exit status, [`support`]
//! holds what the operations share, and this file holds the seven operations
//! themselves.

mod support;
mod taxonomy;
mod wire;

#[cfg(test)]
mod tests;

use std::path::Path;

use quoin_change_assurance::schemas;
use quoin_change_assurance::verify::{RetainedAttestation, Selection};
use quoin_change_assurance::{
    Published, VerificationInput, attestations, intake_attestation, read_attestation, records,
    verify, write_change_record,
};
use quoin_store::store::{attestation_path, record_path};
use quoin_store::{JsonValue, parse_strict_json};

use crate::capabilities::Capabilities;
use crate::error::{CoreError, CoreErrorCode};
use crate::ops::{refusal, request_size};
use crate::protocol::Response;

use self::support::{
    check_bound, decode_hex, digest_of, host, missing, ok, parse, read_audits, refuse_supplied,
    size_bytes_as_f64, stored_record, strict_document, to_serde,
};
use self::taxonomy::map_error;
pub use self::wire::{
    IntakePayload, IntakeRequest, MAX_INTAKE_BYTES, MAX_RECEIPT_BYTES, MAX_RECOVER_BYTES,
    MAX_SCALAR_BYTES, MAX_SCHEMA_BYTES, MAX_SEAL_ATTESTATION_BYTES, MAX_SEAL_RECORD_BYTES,
    MAX_VERIFY_RECEIPT_BYTES, ReceiptPayload, ReceiptRequest, RecoverPayload, RecoverRequest,
    SchemaAssetPayload, SchemaAssetRequest, SealAttestationPayload, SealAttestationRequest,
    SealRecordPayload, SealRecordRequest, SelectionRequest, VerifyReceiptPayload,
    VerifyReceiptRequest,
};

/// Answer a `change_assurance.seal_record`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`SealRecordRequest`],
///   when the record's bytes do not parse strictly, or when the record does not
///   satisfy the FR-063 schema.
/// - [`CoreErrorCode::Refused`] when the request or a field exceeds its
///   ceiling, or when the store declines to retain it.
pub fn seal_record(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    const OP: &str = "change_assurance.seal_record";
    let size = request_size(request)?;
    if size > MAX_SEAL_RECORD_BYTES {
        return Err(refusal(OP, MAX_SEAL_RECORD_BYTES, size));
    }
    let request: SealRecordRequest = parse(request, OP)?;
    check_bound(OP, "repo", &request.repo, MAX_SCALAR_BYTES)?;

    let body = strict_document(&request.record_hex, OP, "record_hex")?;
    // The seal derives `digest`. A body that supplies one is refused rather
    // than overwritten: a caller holding a wrong digest must be told so, not
    // quietly corrected into agreement.
    refuse_supplied(&body, &["digest"], OP)?;
    let sealed = records::seal_change_record(&body).map_err(|e| map_error(&e, OP))?;

    let host = host(capabilities, OP)?;
    let mut store = host.store(Path::new(&request.repo));
    let published = write_change_record(store.as_mut(), &sealed).map_err(|e| map_error(&e, OP))?;

    ok(&SealRecordPayload {
        record: to_serde(&sealed.to_json().map_err(|e| map_error(&e, OP))?, OP)?,
        path: record_path(Path::new(&request.repo), &sealed.digest)
            .display()
            .to_string(),
        retained: published == Published::Retained,
    })
}

/// Answer a `change_assurance.seal_attestation`.
///
/// The ONLY fields derived here are `retained_output.digest` and
/// `retained_output.size_bytes`, read from the output bytes the caller sent,
/// plus the `media_type` the caller declares. Everything else is the caller's.
///
/// It needs no capability: the attestation is emitted, not retained, and
/// `change_assurance.intake` is the retention step.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a
///   [`SealAttestationRequest`], when either hex field is not hex, when the
///   body does not parse strictly, when it supplies `digest` or
///   `retained_output`, or when the result does not satisfy the FR-064 schema.
/// - [`CoreErrorCode::Refused`] when the request or a field exceeds its
///   ceiling.
pub fn seal_attestation(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "change_assurance.seal_attestation";
    let size = request_size(request)?;
    if size > MAX_SEAL_ATTESTATION_BYTES {
        return Err(refusal(OP, MAX_SEAL_ATTESTATION_BYTES, size));
    }
    let request: SealAttestationRequest = parse(request, OP)?;
    check_bound(OP, "media_type", &request.media_type, MAX_SCALAR_BYTES)?;

    let body = strict_document(&request.attestation_hex, OP, "attestation_hex")?;
    refuse_supplied(&body, &["digest", "retained_output"], OP)?;
    let output = decode_hex(&request.output_hex, OP, "output_hex")?;

    // The digest and the size are read off the bytes, and both are attached
    // before the schema is checked: an attestation whose `retained_output` the
    // caller wrote is exactly what `refuse_supplied` above rejected, so there
    // is no path by which a declared digest reaches the seal.
    let size_bytes = u64::try_from(output.len()).unwrap_or(u64::MAX);
    let mut object = body
        .as_object()
        .map_err(|e| map_error(&e.into(), OP))?
        .clone();
    let mut retained = quoin_store::JsonObject::new();
    retained.set("media_type", JsonValue::string(&request.media_type));
    retained.set(
        "digest",
        JsonValue::string(quoin_store::digest_raw_bytes(&output).as_hex()),
    );
    retained.set(
        "size_bytes",
        JsonValue::number(size_bytes_as_f64(size_bytes)).map_err(|e| map_error(&e.into(), OP))?,
    );
    object.set("retained_output", JsonValue::Object(retained));

    let sealed = attestations::seal_attestation(&JsonValue::Object(object))
        .map_err(|e| map_error(&e, OP))?;
    ok(&SealAttestationPayload {
        attestation: to_serde(&sealed.to_json().map_err(|e| map_error(&e, OP))?, OP)?,
    })
}

/// Answer a `change_assurance.intake`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not an [`IntakeRequest`] or
///   the attestation does not satisfy the FR-064 schema.
/// - [`CoreErrorCode::Refused`] when a ceiling is exceeded, when the output
///   does not reproduce what the attestation declares, or when the digest
///   already names a different pair.
pub fn intake(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    const OP: &str = "change_assurance.intake";
    let size = request_size(request)?;
    if size > MAX_INTAKE_BYTES {
        return Err(refusal(OP, MAX_INTAKE_BYTES, size));
    }
    let request: IntakeRequest = parse(request, OP)?;
    check_bound(OP, "repo", &request.repo, MAX_SCALAR_BYTES)?;

    let attestation = decode_hex(&request.attestation_hex, OP, "attestation_hex")?;
    let output = decode_hex(&request.output_hex, OP, "output_hex")?;

    // Read once here for the directory the caller is told about. `intake_attestation`
    // re-reads and re-checks it against the output before anything is retained;
    // this call cannot widen what that one accepts.
    let digest = attestations::verify_attestation(
        &parse_strict_json(&attestation).map_err(|e| map_error(&e.into(), OP))?,
    )
    .map_err(|e| map_error(&e, OP))?
    .digest;

    let repo = Path::new(&request.repo);
    let host = host(capabilities, OP)?;
    let mut store = host.store(repo);
    let published =
        intake_attestation(store.as_mut(), &attestation, &output).map_err(|e| map_error(&e, OP))?;

    ok(&IntakePayload {
        directory: attestation_path(repo, &digest).display().to_string(),
        size_bytes: u64::try_from(output.len()).unwrap_or(u64::MAX),
        retained: published == Published::Retained,
    })
}

/// Answer a `change_assurance.recover`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`RecoverRequest`].
/// - [`CoreErrorCode::Refused`] when a ceiling is exceeded or the staging
///   directories cannot be listed or removed.
pub fn recover(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    const OP: &str = "change_assurance.recover";
    let size = request_size(request)?;
    if size > MAX_RECOVER_BYTES {
        return Err(refusal(OP, MAX_RECOVER_BYTES, size));
    }
    let request: RecoverRequest = parse(request, OP)?;
    check_bound(OP, "repo", &request.repo, MAX_SCALAR_BYTES)?;

    let host = host(capabilities, OP)?;
    let mut store = host.store(Path::new(&request.repo));
    let removed = store.recover_staging().map_err(|e| map_error(&e, OP))?;
    ok(&RecoverPayload {
        removed: u64::try_from(removed).unwrap_or(u64::MAX),
    })
}

/// Answer a `change_assurance.receipt`.
///
/// Nothing is discovered: a stored attestation that no selection names has no
/// effect on the receipt, so a stray upload cannot discharge a proof.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`ReceiptRequest`] or
///   one of its documents does not parse strictly.
/// - [`CoreErrorCode::Refused`] when a ceiling is exceeded, or when a named
///   record or attestation is not retained in this store.
pub fn receipt(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    const OP: &str = "change_assurance.receipt";
    let size = request_size(request)?;
    if size > MAX_RECEIPT_BYTES {
        return Err(refusal(OP, MAX_RECEIPT_BYTES, size));
    }
    let request: ReceiptRequest = parse(request, OP)?;
    check_bound(OP, "repo", &request.repo, MAX_SCALAR_BYTES)?;
    check_bound(
        OP,
        "candidate_revision",
        &request.candidate_revision,
        MAX_SCALAR_BYTES,
    )?;
    check_bound(
        OP,
        "record_digest",
        &request.record_digest,
        MAX_SCALAR_BYTES,
    )?;
    for parent in &request.parent_digests {
        check_bound(OP, "parent_digests", parent, MAX_SCALAR_BYTES)?;
    }
    for selection in &request.selections {
        check_bound(
            OP,
            "selections.proof_id",
            &selection.proof_id,
            MAX_SCALAR_BYTES,
        )?;
        check_bound(
            OP,
            "selections.attestation_digest",
            &selection.attestation_digest,
            MAX_SCALAR_BYTES,
        )?;
    }

    let decisions = strict_document(&request.decisions_hex, OP, "decisions_hex")?;
    let audits = match request.audits_hex.as_deref() {
        None => Vec::new(),
        Some(hex) => read_audits(&strict_document(hex, OP, "audits_hex")?, OP)?,
    };

    let repo = Path::new(&request.repo);
    let host = host(capabilities, OP)?;
    let store = host.store(repo);

    let record = stored_record(store.as_ref(), &request.record_digest, "record_digest", OP)?;
    let mut parents = Vec::with_capacity(request.parent_digests.len());
    for digest in &request.parent_digests {
        parents.push(stored_record(store.as_ref(), digest, "parent_digests", OP)?);
    }

    let mut selections = Vec::with_capacity(request.selections.len());
    let mut retained = Vec::with_capacity(request.selections.len());
    for selection in &request.selections {
        let digest = digest_of(
            &selection.attestation_digest,
            "selections.attestation_digest",
            OP,
        )?;
        let found = read_attestation(store.as_ref(), &digest)
            .map_err(|e| map_error(&e, OP))?
            .ok_or_else(|| {
                missing(
                    OP,
                    "selections.attestation_digest",
                    &selection.attestation_digest,
                    &request.repo,
                )
            })?;
        retained.push(RetainedAttestation {
            attestation: found.0.to_json().map_err(|e| map_error(&e, OP))?,
            output: Some(found.1),
        });
        selections.push(Selection {
            proof_id: selection.proof_id.clone(),
            attestation_digest: selection.attestation_digest.clone(),
        });
    }

    let input = VerificationInput {
        record,
        parents,
        candidate_revision: request.candidate_revision.clone(),
        selections,
        attestations: retained,
        decision_history: Some(decisions),
        audits,
    };
    let sealed = verify::verify_change_assurance(&input).map_err(|e| map_error(&e, OP))?;
    ok(&ReceiptPayload {
        receipt: to_serde(&sealed.to_json().map_err(|e| map_error(&e, OP))?, OP)?,
    })
}

/// Answer a `change_assurance.verify_receipt`.
///
/// Re-verifies the receipt DOCUMENT. It does not re-run verification: the
/// underlying record, attestations and decisions are not consulted.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a
///   [`VerifyReceiptRequest`], when the bytes do not parse strictly, or when
///   the receipt does not satisfy the FR-065 schema or its own digest.
/// - [`CoreErrorCode::Refused`] when a ceiling is exceeded.
pub fn verify_receipt(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "change_assurance.verify_receipt";
    let size = request_size(request)?;
    if size > MAX_VERIFY_RECEIPT_BYTES {
        return Err(refusal(OP, MAX_VERIFY_RECEIPT_BYTES, size));
    }
    let request: VerifyReceiptRequest = parse(request, OP)?;
    let document = strict_document(&request.receipt_hex, OP, "receipt_hex")?;
    let verified = verify::verify_receipt(&document).map_err(|e| map_error(&e, OP))?;
    ok(&VerifyReceiptPayload {
        receipt: to_serde(&verified.to_json().map_err(|e| map_error(&e, OP))?, OP)?,
    })
}

/// Answer a `change_assurance.schema`.
///
/// The three normative assets are compiled into `quoin-change-assurance` and
/// emitted from there, so the schema a consumer validates against and the
/// schema the sealing code is tested against are one file (quoin#503). It
/// needs no capability: nothing is read from disk and nothing is written.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`SchemaAssetRequest`], or
///   when it names an asset this build does not ship.
/// - [`CoreErrorCode::Refused`] when the request exceeds its ceiling.
pub fn schema(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "change_assurance.schema";
    let size = request_size(request)?;
    if size > MAX_SCHEMA_BYTES {
        return Err(refusal(OP, MAX_SCHEMA_BYTES, size));
    }
    let request: SchemaAssetRequest = parse(request, OP)?;
    let names: Vec<String> = schemas::ASSET_NAMES.iter().map(|&n| n.to_owned()).collect();
    let asset = match request.name {
        None => None,
        Some(name) => {
            check_bound(OP, "name", &name, MAX_SCALAR_BYTES)?;
            // Named rather than defaulted: a caller holding a name this build
            // does not ship is told so, because answering with some other
            // schema would validate its document against a contract nobody
            // asked for.
            let Some(text) = schemas::asset(&name) else {
                return Err(CoreError::new(
                    CoreErrorCode::BadRequest,
                    format!("{OP}: no schema asset named {name}"),
                )
                .with_context("op", OP)
                .with_context("name", name)
                .with_context("known", names.join(",")));
            };
            Some(text.to_owned())
        }
    };
    ok(&SchemaAssetPayload {
        schemas: names,
        schema: asset,
    })
}
