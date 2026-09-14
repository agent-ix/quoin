// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `change_assurance` domain's wire shapes, and the ceilings they are read
//! under.
//!
//! Request and payload types only: what a caller may say, and what it gets
//! back. No decision is taken here. The operations that take them live in
//! [`super`], and the exit-taxonomy mapping in [`super::taxonomy`].
//!
//! # Why every document arrives as hex
//!
//! `src/change-assurance/` read its inputs with `parseStrictJson`, which
//! refuses a duplicate member, a byte-order mark, a non-finite number and
//! trailing content — decisions taken over the **bytes the producer supplied**.
//! A request that carried a pre-parsed JSON value would have had those bytes
//! read by `JSON.parse` on the caller's side first, where a duplicate key is
//! silently resolved last-wins; the strict reader would then never see the
//! document that was actually on disk, and a record that the retained code
//! refused would seal cleanly.
//!
//! So the documents cross as **lowercase hexadecimal of the exact file bytes**
//! and [`quoin_store::parse_strict_json`] reads them here. The same encoding
//! carries `output.bin`, which is not text at all. Hex rather than base64
//! because it needs no dependency, it is the spelling every digest in this
//! store already uses, and its cost is a factor of two on a transport whose
//! ceiling is 64 MiB.
//!
//! The size ceilings sit with the shapes rather than with the operations
//! because each is a property of a field and not of the work: they bound what
//! may be read off stdin at all, and [`super`] applies them before any host is
//! consulted. stdin is untrusted (rust-style §11).

use serde::{Deserialize, Serialize};

/// The largest `change_assurance.seal_record` request this domain will decide
/// over, in bytes.
///
/// A record is a reviewed change definition: identities, statements and
/// connections. 4 MiB is far past any record the retained TypeScript ever
/// sealed and still refuses a stream.
pub const MAX_SEAL_RECORD_BYTES: usize = 4 * 1024 * 1024;

/// The largest `change_assurance.seal_attestation` request this domain will
/// decide over, in bytes.
///
/// It carries the retained result file, hex-encoded, so the effective file
/// ceiling is half of this. A ceiling the retained TypeScript did not have:
/// `readFileSync` read whatever was named. See `DIVERGENCE.md`.
pub const MAX_SEAL_ATTESTATION_BYTES: usize = 32 * 1024 * 1024;

/// The largest `change_assurance.intake` request this domain will decide over,
/// in bytes.
///
/// It carries the attestation and the output it attests to, both hex-encoded.
pub const MAX_INTAKE_BYTES: usize = 32 * 1024 * 1024;

/// The largest `change_assurance.recover` request this domain will decide over,
/// in bytes.
///
/// A repository root and nothing else.
pub const MAX_RECOVER_BYTES: usize = 8 * 1024;

/// The largest `change_assurance.receipt` request this domain will decide over,
/// in bytes.
///
/// The record, its parents and the selected attestations are read from the
/// store by digest and never ride on the request. What does ride on it is the
/// decision history and the retained audit reports.
pub const MAX_RECEIPT_BYTES: usize = 32 * 1024 * 1024;

/// The largest `change_assurance.verify_receipt` request this domain will
/// decide over, in bytes.
pub const MAX_VERIFY_RECEIPT_BYTES: usize = 4 * 1024 * 1024;

/// The largest `change_assurance.schema` request this domain will decide over,
/// in bytes.
///
/// An asset name and nothing else. The assets themselves are compiled in, so
/// nothing about their size bears on what may arrive.
pub const MAX_SCHEMA_BYTES: usize = 8 * 1024;

/// The largest single path, digest or identity string this domain will accept,
/// in bytes.
///
/// A repository root, a 64-character digest, a revision and a proof id are all
/// one of these. 4 KiB is past every platform's `PATH_MAX` and still refuses a
/// stream.
pub const MAX_SCALAR_BYTES: usize = 4 * 1024;

/// The request accepted by `change_assurance.seal_record`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SealRecordRequest {
    /// Repository root holding the evidence store.
    pub repo: String,
    /// The record body, without its `digest`, as hex of the exact input bytes.
    pub record_hex: String,
}

/// The payload `change_assurance.seal_record` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SealRecordPayload {
    /// The sealed record, `digest` included.
    pub record: serde_json::Value,
    /// Where it was retained, relative to the repository root as supplied.
    pub path: String,
    /// Whether this call retained the record, or found the same bytes already
    /// there. Re-sealing an identical record is not an error and never was.
    pub retained: bool,
}

/// The request accepted by `change_assurance.seal_attestation`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SealAttestationRequest {
    /// The attestation body, without `digest` and without `retained_output`,
    /// as hex of the exact input bytes.
    pub attestation_hex: String,
    /// The retained result file's exact bytes, as hex.
    pub output_hex: String,
    /// The media type the caller declares for that file.
    ///
    /// Stated rather than sniffed, so a producer's own content type is
    /// preserved exactly.
    pub media_type: String,
}

/// The payload `change_assurance.seal_attestation` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SealAttestationPayload {
    /// The sealed attestation. Emitted, not retained.
    pub attestation: serde_json::Value,
}

/// The request accepted by `change_assurance.intake`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct IntakeRequest {
    /// Repository root holding the evidence store.
    pub repo: String,
    /// The sealed attestation's exact bytes, as hex.
    pub attestation_hex: String,
    /// The output's exact bytes, as hex.
    pub output_hex: String,
}

/// The payload `change_assurance.intake` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct IntakePayload {
    /// The directory the pair became visible in.
    pub directory: String,
    /// How many bytes of output were retained.
    pub size_bytes: u64,
    /// Whether this call retained the pair, or found the same pair already
    /// there.
    pub retained: bool,
}

/// The request accepted by `change_assurance.recover`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RecoverRequest {
    /// Repository root holding the evidence store.
    pub repo: String,
}

/// The payload `change_assurance.recover` writes to stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RecoverPayload {
    /// How many interrupted-intake staging directories were removed.
    pub removed: u64,
}

/// One `<proof-id>=<attestation-digest>` selection.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SelectionRequest {
    /// The obligation the attestation is offered against.
    pub proof_id: String,
    /// The digest of the attestation offered.
    pub attestation_digest: String,
}

/// The request accepted by `change_assurance.receipt`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReceiptRequest {
    /// Repository root holding the evidence store.
    pub repo: String,
    /// Digest of the stored record to verify.
    pub record_digest: String,
    /// The candidate revision the selected attestations must be bound to.
    pub candidate_revision: String,
    /// Digests of stored parent records, named rather than walked.
    pub parent_digests: Vec<String>,
    /// Which attestation is offered for which obligation. Only these are read.
    pub selections: Vec<SelectionRequest>,
    /// The retained ix-flow decision history, as hex of the exact input bytes.
    pub decisions_hex: String,
    /// The retained FR-032 audit reports as a JSON array, as hex of the exact
    /// input bytes. Absent means no audit was retained, which stays distinct
    /// from an audit with no findings.
    pub audits_hex: Option<String>,
}

/// One retained FR-032 audit report, as `--audits` spells it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuditInput {
    /// The obligation the report covers.
    pub proof_id: String,
    /// The report's digest, as the auditor filed it.
    pub report_digest: String,
    /// The report itself.
    pub report: AuditReportInput,
}

/// A retained audit report's body.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuditReportInput {
    /// What the auditor found wrong.
    #[serde(default)]
    pub findings: Vec<AuditFindingInput>,
    /// Which obligations it found discharged.
    #[serde(default)]
    pub healthy: Vec<String>,
    /// Which obligations it did not evaluate.
    #[serde(default)]
    pub unevaluated: Vec<AuditUnevaluatedInput>,
}

/// One finding from a retained audit report.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuditFindingInput {
    /// The obligation the finding is about.
    pub obligation: String,
    /// The finding's kind, as the auditor spelled it.
    pub kind: String,
}

/// One obligation a retained audit report left unevaluated.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuditUnevaluatedInput {
    /// The obligation left unevaluated.
    pub obligation: String,
}

/// The payload `change_assurance.receipt` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReceiptPayload {
    /// The sealed verification receipt, in full.
    ///
    /// The whole receipt rather than a verdict: the caller prints the proof
    /// rows and the reasons, and a payload that carried only `outcome` would
    /// make "why" a second call.
    pub receipt: serde_json::Value,
}

/// The request accepted by `change_assurance.verify_receipt`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct VerifyReceiptRequest {
    /// The sealed receipt's exact bytes, as hex.
    pub receipt_hex: String,
}

/// The payload `change_assurance.verify_receipt` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct VerifyReceiptPayload {
    /// The receipt as re-read and re-verified, in full.
    pub receipt: serde_json::Value,
}

/// The request accepted by `change_assurance.schema`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SchemaAssetRequest {
    /// Which asset to emit, or absent to ask only for the vocabulary.
    ///
    /// Absent and "emit nothing" are the same answer here because the
    /// vocabulary rides on every response: a caller listing the assets and a
    /// caller fetching one both learn what the build ships.
    #[serde(default)]
    pub name: Option<String>,
}

/// The payload `change_assurance.schema` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SchemaAssetPayload {
    /// Every asset name this build ships, in the order the vocabulary declares.
    pub schemas: Vec<String>,
    /// The requested asset's exact bytes, or `null` when none was requested.
    ///
    /// The text as compiled in, trailing newline included — a consumer
    /// validates against the same bytes the sealing code was written against,
    /// and a re-serialization here would defeat that.
    pub schema: Option<String>,
}
