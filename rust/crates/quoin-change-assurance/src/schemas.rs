// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The three normative JSON Schema assets for FR-063, FR-064 and FR-065.
//!
//! # Why they live here and not in the TypeScript tree
//!
//! They described shapes this crate produces while being stored beside the
//! reader that emitted them (`src/store/schemas/`, quoin#503). That is the
//! wrong way round: after the store cutover the directory holding them is
//! deleted, and a normative contract whose home disappears with a port stage
//! is a contract nothing is responsible for.
//!
//! They are **data**, not logic — `quoin change-assurance schema` prints one
//! byte-for-byte so a consumer validates against the same file the sealing
//! code was written against, and a re-serialization would defeat that. So they
//! are compiled in with `include_str!` rather than read off disk: a schema read
//! at run time is a schema that can differ from the one these tests measured.
//!
//! # The assets are asserted, not merely carried
//!
//! `tests/tc_503_schema_assets.rs` does not stop at "the file parses". Each
//! asset is compiled and a document this crate actually sealed is validated
//! against it, so a shape change in [`crate::model`] that nobody mirrored here
//! fails a test rather than shipping a schema that describes the previous
//! release. The tests live outside `src/` because they need a JSON reader and
//! a validator, and `tests/tc_455_boundary.rs` censuses this crate's own
//! sources for exactly that kind of dependency.

/// `change-assurance-record-v1`, the FR-063 sealed record.
pub const CHANGE_ASSURANCE_RECORD_V1: &str =
    include_str!("../schemas/change-assurance-record-v1.schema.json");

/// `proof-attestation-v1`, the FR-064 producer attestation.
pub const PROOF_ATTESTATION_V1: &str = include_str!("../schemas/proof-attestation-v1.schema.json");

/// `verification-receipt-v1`, the FR-065 receipt.
pub const VERIFICATION_RECEIPT_V1: &str =
    include_str!("../schemas/verification-receipt-v1.schema.json");

/// Every asset, by the file name a consumer asks for.
///
/// The names carry `.schema.json` because that is what the npm package ships
/// and what `quoin change-assurance schema --name` has always accepted; a
/// shorter spelling here would make this list a second vocabulary to keep in
/// step with the published one.
pub const ASSET_NAMES: [&str; 3] = [
    "change-assurance-record-v1.schema.json",
    "proof-attestation-v1.schema.json",
    "verification-receipt-v1.schema.json",
];

/// One asset's exact bytes, or `None` for a name that is not an asset.
///
/// `None` rather than a default: an unknown name is a caller mistake the
/// surface reports, and answering it with some other schema would validate a
/// document against a contract nobody asked for.
#[must_use]
pub fn asset(name: &str) -> Option<&'static str> {
    match name {
        "change-assurance-record-v1.schema.json" => Some(CHANGE_ASSURANCE_RECORD_V1),
        "proof-attestation-v1.schema.json" => Some(PROOF_ATTESTATION_V1),
        "verification-receipt-v1.schema.json" => Some(VERIFICATION_RECEIPT_V1),
        _ => None,
    }
}
