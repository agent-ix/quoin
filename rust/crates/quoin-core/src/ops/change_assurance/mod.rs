// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `change_assurance`: change records, proof attestations, retained
//! evidence and verification receipts (quoin#457, Stage 9).
//!
//! Replaces `src/change-assurance/`'s TypeScript. What it decided — whether a
//! record satisfies the FR-063 schema, whether an attestation's output has the
//! size it declares, whether a parent chain is strict N-1, and what verdict a
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
    Published, Revision, Subject, VerificationInput, attestations, intake_attestation,
    read_attestation, records, verify, write_change_record,
};
use quoin_store::store::{attestation_path, record_path};
use quoin_store::{JsonValue, parse_strict_json};

use crate::capabilities::Capabilities;
use crate::error::{CoreError, CoreErrorCode};
use crate::ops::{refusal, request_size};
use crate::protocol::Response;

use self::support::{
    attestation_id_of, check_bound, check_receipt_bounds, decode_hex, governing_plan, host,
    missing, ok, parse, read_audits, record_id_of, refuse_supplied, size_bytes_as_f64,
    stored_predecessors, stored_record, strict_document, to_serde,
};
use self::taxonomy::map_error;
pub use self::wire::{
    IntakePayload, IntakeRequest, MAX_DIFF_PATHS, MAX_INTAKE_BYTES, MAX_PREPARE_ATTESTATION_BYTES,
    MAX_RECEIPT_BYTES, MAX_RECOVER_BYTES, MAX_SCALAR_BYTES, MAX_SCHEMA_BYTES,
    MAX_VERIFY_RECEIPT_BYTES, MAX_WRITE_RECORD_BYTES, PrepareAttestationPayload,
    PrepareAttestationRequest, ReceiptPayload, ReceiptRequest, RecoverPayload, RecoverRequest,
    SchemaAssetPayload, SchemaAssetRequest, SelectionRequest, VerifyReceiptPayload,
    VerifyReceiptRequest, WriteRecordPayload, WriteRecordRequest,
};

/// Answer a `change_assurance.write_record`.
///
/// The record is addressed by the `(record_id, revision)` it declares; a body
/// carrying a `digest` or any other undeclared member is refused by the schema.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`WriteRecordRequest`],
///   when the record's bytes do not parse strictly, or when the record does not
///   satisfy the FR-063 schema.
/// - [`CoreErrorCode::Refused`] when the request or a field exceeds its
///   ceiling, when the revision already holds different bytes, or when the
///   store declines to retain it.
pub fn write_record(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    const OP: &str = "change_assurance.write_record";
    let size = request_size(request)?;
    if size > MAX_WRITE_RECORD_BYTES {
        return Err(refusal(OP, MAX_WRITE_RECORD_BYTES, size));
    }
    let request: WriteRecordRequest = parse(request, OP)?;
    check_bound(OP, "repo", &request.repo, MAX_SCALAR_BYTES)?;

    let body = strict_document(&request.record_hex, OP, "record_hex")?;
    let record = records::prepare_change_record(&body).map_err(|e| map_error(&e, OP))?;

    let host = host(capabilities, OP)?;
    let mut store = host.store(Path::new(&request.repo));
    let published = write_change_record(store.as_mut(), &record).map_err(|e| map_error(&e, OP))?;

    ok(&WriteRecordPayload {
        record: to_serde(&record.to_json().map_err(|e| map_error(&e, OP))?, OP)?,
        path: record_path(
            Path::new(&request.repo),
            record.record_id.as_str(),
            record.revision.get(),
        )
        .display()
        .to_string(),
        retained: published == Published::Retained,
    })
}

/// Answer a `change_assurance.prepare_attestation`.
///
/// The ONLY field derived here is `retained_output.size_bytes`, read from the
/// output bytes the caller sent, plus the `media_type` the caller declares.
/// Everything else is the caller's.
///
/// It needs no capability: the attestation is emitted, not retained, and
/// `change_assurance.intake` is the retention step.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a
///   [`PrepareAttestationRequest`], when either hex field is not hex, when the
///   body does not parse strictly, when it supplies `retained_output`, or when
///   the result does not satisfy the FR-064 schema.
/// - [`CoreErrorCode::Refused`] when the request or a field exceeds its
///   ceiling.
pub fn prepare_attestation(request: &serde_json::Value) -> Result<Response, CoreError> {
    const OP: &str = "change_assurance.prepare_attestation";
    let size = request_size(request)?;
    if size > MAX_PREPARE_ATTESTATION_BYTES {
        return Err(refusal(OP, MAX_PREPARE_ATTESTATION_BYTES, size));
    }
    let request: PrepareAttestationRequest = parse(request, OP)?;
    check_bound(OP, "media_type", &request.media_type, MAX_SCALAR_BYTES)?;

    let body = strict_document(&request.attestation_hex, OP, "attestation_hex")?;
    refuse_supplied(&body, &["retained_output"], OP)?;
    let output = decode_hex(&request.output_hex, OP, "output_hex")?;

    // The size is read off the bytes and attached before the schema is
    // checked: an attestation whose `retained_output` the caller wrote is
    // exactly what `refuse_supplied` above rejected, so there is no path by
    // which a declared size reaches the document.
    let size_bytes = u64::try_from(output.len()).unwrap_or(u64::MAX);
    let mut object = body
        .as_object()
        .map_err(|e| map_error(&e.into(), OP))?
        .clone();
    let mut retained = quoin_store::JsonObject::new();
    retained.set("media_type", JsonValue::string(&request.media_type));
    retained.set(
        "size_bytes",
        JsonValue::number(size_bytes_as_f64(size_bytes)).map_err(|e| map_error(&e.into(), OP))?,
    );
    object.set("retained_output", JsonValue::Object(retained));

    let attestation = attestations::read_attestation_json(&JsonValue::Object(object))
        .map_err(|e| map_error(&e, OP))?;
    ok(&PrepareAttestationPayload {
        attestation: to_serde(&attestation.to_json().map_err(|e| map_error(&e, OP))?, OP)?,
    })
}

/// Answer a `change_assurance.intake`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not an [`IntakeRequest`] or
///   the attestation does not satisfy the FR-064 schema.
/// - [`CoreErrorCode::Refused`] when a ceiling is exceeded, when the output
///   does not have the size the attestation declares, or when the attestation
///   id already names a different pair.
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
    let attestation_id = attestations::read_attestation_json(
        &parse_strict_json(&attestation).map_err(|e| map_error(&e.into(), OP))?,
    )
    .map_err(|e| map_error(&e, OP))?
    .attestation_id;

    let repo = Path::new(&request.repo);
    let host = host(capabilities, OP)?;
    let mut store = host.store(repo);
    let published =
        intake_attestation(store.as_mut(), &attestation, &output).map_err(|e| map_error(&e, OP))?;

    ok(&IntakePayload {
        directory: attestation_path(repo, attestation_id.as_str())
            .display()
            .to_string(),
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
/// - [`CoreErrorCode::Refused`] when a ceiling is exceeded, when a named
///   record or attestation is not retained in this store, or when no
///   `MeasurementPlan` in `repo` has the requested `plan` id.
/// - The mapped `quoin-measurement` refusal when `plan` is named and the
///   repository's plan documents cannot be loaded.
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
    let input = receipt_input(&request, capabilities, OP)?;
    let receipt = verify::verify_change_assurance(&input).map_err(|e| map_error(&e, OP))?;
    ok(&ReceiptPayload {
        receipt: to_serde(&receipt.to_json().map_err(|e| map_error(&e, OP))?, OP)?,
    })
}

/// Read the retained inputs a receipt request names.
fn receipt_input(
    request: &ReceiptRequest,
    capabilities: &Capabilities<'_>,
    op: &'static str,
) -> Result<VerificationInput, CoreError> {
    check_receipt_bounds(op, request)?;

    let decisions = strict_document(&request.decisions_hex, op, "decisions_hex")?;
    let audits = match request.audits_hex.as_deref() {
        None => Vec::new(),
        Some(hex) => read_audits(&strict_document(hex, op, "audits_hex")?, op)?,
    };

    let repo = Path::new(&request.repo);
    let host = host(capabilities, op)?;
    let store = host.store(repo);

    let record_id = record_id_of(&request.record_id, op)?;
    let revision = Revision::new(request.revision, Subject::Record, "revision")
        .map_err(|e| map_error(&e, op))?;
    let record = stored_record(store.as_ref(), &record_id, revision, op)?;
    let parents = stored_predecessors(store.as_ref(), &record_id, revision, op)?;

    let mut selections = Vec::with_capacity(request.selections.len());
    let mut retained = Vec::with_capacity(request.selections.len());
    for selection in &request.selections {
        let attestation_id =
            attestation_id_of(&selection.attestation_id, "selections.attestation_id", op)?;
        let found = read_attestation(store.as_ref(), &attestation_id)
            .map_err(|e| map_error(&e, op))?
            .ok_or_else(|| {
                missing(
                    op,
                    "selections.attestation_id",
                    &selection.attestation_id,
                    &request.repo,
                )
            })?;
        retained.push(RetainedAttestation {
            filed_as: selection.attestation_id.clone(),
            attestation: found.0.to_json().map_err(|e| map_error(&e, op))?,
            output: Some(found.1),
        });
        selections.push(Selection {
            proof_id: selection.proof_id.clone(),
            attestation_id: selection.attestation_id.clone(),
        });
    }

    // PLAT-997: `diff_paths` is the caller's own claim (see `wire::ReceiptRequest`'s
    // trust-boundary doc), carried straight through with no producer run here.
    // `plan` is resolved through `quoin-measurement`'s own plan intake so
    // `protected_apparatus`/`negative_controls` are read by the one parser
    // PLAT-975 already governs.
    let governing = request
        .plan
        .as_deref()
        .map(|plan| governing_plan(&request.repo, plan, op))
        .transpose()?;

    Ok(VerificationInput {
        record_id,
        record_revision: revision,
        record,
        parents,
        candidate_revision: request.candidate_revision.clone(),
        selections,
        attestations: retained,
        decision_history: Some(decisions),
        audits,
        diff_paths: request.diff_paths.clone(),
        // PLAT-1015: the resolved plan carries its own id, which the receipt
        // records as `governing_plan_id` regardless of outcome.
        governing_plan: governing,
    })
}

/// Answer a `change_assurance.verify_receipt`.
///
/// Re-derives the receipt from the named retained inputs and compares its
/// canonical bytes with the stored receipt's, byte for byte.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a
///   [`VerifyReceiptRequest`], when the bytes do not parse strictly, when the
///   receipt does not satisfy the FR-065 schema, or when it is not the receipt
///   re-derived from the named inputs.
/// - [`CoreErrorCode::Refused`] when a ceiling is exceeded or a named input is
///   not retained.
pub fn verify_receipt(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    const OP: &str = "change_assurance.verify_receipt";
    let size = request_size(request)?;
    if size > MAX_VERIFY_RECEIPT_BYTES {
        return Err(refusal(OP, MAX_VERIFY_RECEIPT_BYTES, size));
    }
    let request: VerifyReceiptRequest = parse(request, OP)?;
    let document = strict_document(&request.receipt_hex, OP, "receipt_hex")?;
    let input = receipt_input(&request.inputs, capabilities, OP)?;
    let verified = verify::verify_receipt(&document, &input).map_err(|e| map_error(&e, OP))?;
    ok(&VerifyReceiptPayload {
        receipt: to_serde(&verified.to_json().map_err(|e| map_error(&e, OP))?, OP)?,
    })
}

/// Answer a `change_assurance.schema`.
///
/// The three normative assets are compiled into `quoin-change-assurance` and
/// emitted from there, so the schema a consumer validates against and the
/// schema the writing code is tested against are one file (quoin#503). It
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
