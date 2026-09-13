// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading a candidate as an operational evidence record.
//!
//! Ports `operational.ts:50-68` (`validateOperationalRecord`).
//!
//! # Parse, don't validate
//!
//! The retained function is a TypeScript assertion — it narrows a type and
//! returns nothing, so every caller after it still holds the `unknown` it
//! started with and the narrowing is a promise the compiler cannot check twice.
//! Here it returns a [`ValidOperationalRecord`]: holding one *is* the proof
//! that the schema accepted the document, that the semantic pass found nothing,
//! and that the document deserialised into the typed record. Nothing downstream
//! re-validates, and nothing downstream can skip the check, because there is no
//! other way to make one.
//!
//! # One schema, and it is the vendored one
//!
//! The schema document is [`VendoredSchema::OperationalEvidenceV1`], compiled
//! with this crate's one RFC 3339 grammar registered as `format: "date-time"` —
//! the same wiring the retained `ajv.addFormat` does at
//! `operational.ts:31-34`. It is not re-vendored here, and the
//! `format` check is not a second grammar: it is
//! [`crate::date_time::Rfc3339DateTime`], passed in.

use std::sync::OnceLock;

use quoin_jsonschema::{VendoredSchema, VendoredValidator};
use serde_json::Value;

use crate::common::schema::{findings as schema_findings, sorted_unique};
use crate::date_time::Rfc3339DateTime;
use crate::error::MeasurementErrorCode;
use crate::intervention::intake::{InterventionIntakeError, InterventionRefusalCode};
use crate::json_bridge::from_serde;
use crate::operational::record::OperationalEvidenceRecord;
use crate::operational::semantics::semantic_findings;
use crate::types::ids::SafeRecordId;

/// A document that is an operational evidence record.
///
/// Modelled on [`quoin_jsonschema::ValidDocument`] and
/// `quoin_store::RawFileSha256Digest::parse_stored`: constructed in exactly one
/// place, below, and nowhere else.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidOperationalRecord {
    document: Value,
    record: OperationalEvidenceRecord,
    record_id: SafeRecordId,
}

impl ValidOperationalRecord {
    /// The document exactly as it was handed in.
    ///
    /// **This, not a re-serialisation of [`record`](Self::record), is what is
    /// written to disk.** `operational.ts:271` does the same, and it is what
    /// keeps a member this crate does not model from being dropped on the way
    /// to the store.
    #[must_use]
    pub const fn document(&self) -> &Value {
        &self.document
    }

    /// The typed record.
    #[must_use]
    pub const fn record(&self) -> &OperationalEvidenceRecord {
        &self.record
    }

    /// The record's identity, already safe to name a file with.
    #[must_use]
    pub const fn record_id(&self) -> &SafeRecordId {
        &self.record_id
    }

    /// The document's canonical bytes, as the store writes them.
    ///
    /// # Errors
    ///
    /// [`InterventionRefusalCode::InvalidRecord`] for a number no double can
    /// hold, which the strict JSON reader would already have refused.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, InterventionIntakeError> {
        let stored =
            from_serde(&self.document).map_err(|error| refusal(vec![error.to_string()]))?;
        quoin_store::canonical_json_bytes(&stored).map_err(|error| refusal(vec![error.to_string()]))
    }
}

/// The compiled operational-evidence validator, compiled once per process.
///
/// # Errors
///
/// A committed schema that does not compile is a defect in this repository,
/// never ordinary data, so it is reported as a refusal naming the schema rather
/// than panicking in a library.
fn validator() -> Result<&'static VendoredValidator, InterventionIntakeError> {
    static COMPILED: OnceLock<Result<VendoredValidator, String>> = OnceLock::new();
    COMPILED
        .get_or_init(|| {
            VendoredSchema::OperationalEvidenceV1
                .compile(|text| Rfc3339DateTime::parse(text).is_ok())
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|detail| refusal(vec![detail.clone()]))
}

/// Read a candidate as an operational evidence record.
///
/// # Errors
///
/// [`InterventionRefusalCode::InvalidRecord`], carrying every schema failure
/// and every semantic finding, deduplicated and sorted, exactly as
/// `operational.ts:63-66` reports them.
pub fn validate_operational_record(
    candidate: &Value,
) -> Result<ValidOperationalRecord, InterventionIntakeError> {
    // The rendering and the ordering are `common::schema`'s, shared with the
    // intervention family: a caller splits `/<pointer>: <sentence>` on the
    // first colon and must not have to learn which family wrote it.
    let mut findings = schema_findings(validator()?, candidate);
    semantic_findings(candidate, &mut findings);
    if !findings.is_empty() {
        return Err(refusal(sorted_unique(findings)));
    }
    // Past here the schema has accepted the document, so the typed read cannot
    // fail on a shape — only on a shape this port models more narrowly than the
    // schema does, which is a defect in this crate and is reported as one
    // rather than hidden.
    let record: OperationalEvidenceRecord =
        serde_json::from_value(candidate.clone()).map_err(|error| {
            refusal(vec![format!(
                "/: schema-valid record is unreadable by this port: {error}"
            )])
        })?;
    // The schema's `identity` pattern is character for character the guard of
    // `operational.ts:455`, so this parse cannot fail here; it is done anyway
    // because the *type* is what the path minter takes, and a proof carried in
    // a comment is the thing this crate does not do.
    let record_id = SafeRecordId::parse(
        record.base().record_id.as_str(),
        MeasurementErrorCode::CollectionInvalid,
    )
    .map_err(|error| refusal(vec![error.to_string()]))?;
    Ok(ValidOperationalRecord {
        document: candidate.clone(),
        record,
        record_id,
    })
}

fn refusal(findings: Vec<String>) -> InterventionIntakeError {
    InterventionIntakeError::new(InterventionRefusalCode::InvalidRecord, findings)
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::validate_operational_record;
    use crate::intervention::intake::InterventionRefusalCode;

    #[test]
    fn a_value_that_is_not_a_record_is_refused_under_the_intake_code() {
        let error = validate_operational_record(&json!([])).unwrap_err();
        assert_eq!(error.code(), InterventionRefusalCode::InvalidRecord);
        assert!(!error.findings().is_empty());
    }

    /// Findings arrive deduplicated and sorted, which is what makes a refusal
    /// comparable between two runs.
    #[test]
    fn findings_are_sorted_and_carry_no_duplicate() {
        let error =
            validate_operational_record(&json!({ "record_shape": "exercise" })).unwrap_err();
        let findings = error.findings();
        let mut sorted = findings.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(findings, sorted.as_slice());
    }
}
