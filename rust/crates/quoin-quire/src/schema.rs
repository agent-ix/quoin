// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The one vendored schema this crate kept: `assurance-v1` (quoin#474).
//!
//! ## Why a vendored schema exists here at all
//!
//! [`crate::payload`] explains why the other four vendored schemas did not
//! come across: a payload read into the engine's own type is checked against
//! the same declaration the engine emits from, so a second declaration in JSON
//! Schema could only drift from it. That argument holds for every payload
//! **quire produced in this process**, and [`crate::assurance::read`] is the
//! stronger form of it — `quire_rs::read_assurance_export` is a fail-closed
//! reader that also checks the caller's module and schema-digest premises.
//!
//! It does not hold for the one caller that has no premises to state.
//! `src/measurement/graph-adapters.ts` and `src/graph-analysis/load.ts` read an
//! `assurance-v1` document that arrived from somewhere else — a portfolio of
//! other repositories' exports — and ask one question: *is this the published
//! shape?* `validateAssurance` (`src/quire/validate.ts:99`) is that question,
//! and quoin#387 is blocked without it. An answer of "supply premises you do
//! not have" is not an answer, so the schema check survives, alone, here.
//!
//! ## What this module is not
//!
//! It is not a second JSON Schema validator. `quoin-jsonschema` is the one
//! ajv-parity validator in the workspace (quoin#470) and this module calls it.
//! The `jsonschema` pin lives in that crate's manifest and nowhere else.
//!
//! ## `format` is annotated, never asserted
//!
//! `src/quire/validate.ts:99` builds `new Ajv2020({ allErrors: true, strict:
//! false })` and registers **no** format checks — unlike the two measurement
//! ajv instances, which each register `date-time`. `assurance-v1` carries one
//! `format`, `uuid` on `$defs.artifact.uuid` (`schemas/assurance-v1.schema.json:110`),
//! and under that ajv it is an annotation: `"not-a-uuid"` is accepted today.
//!
//! So the constructor is [`quoin_jsonschema::SchemaValidator::compile_vendored`],
//! which registers nothing and asserts nothing. Reaching for
//! `compile_with_formats` because the schema *has* a format would refuse
//! documents the oracle accepts — a verdict difference, which is a defect and
//! not a nuance. `tests/tc_474_assurance_schema.rs` asserts the verdict from
//! both sides so the choice cannot quietly invert.
//!
//! ## Verdicts are contractual; diagnostics are not
//!
//! quoin#403 asked whether ajv's error *ordering* could be promised. It is not
//! promised here, for anything: error text, error count and error order are all
//! outside the contract, and no test asserts them. What is contractual is
//! whether a document satisfies the schema, measured against the oracle on a
//! named corpus in `tests/tc_474_assurance_parity.rs`.

use std::path::Path;
use std::sync::OnceLock;

use quoin_jsonschema::SchemaValidator;
use serde_json::Value;

use crate::error::{Error, Result};

/// The committed document's path, relative to the repository root.
pub const VENDORED_PATH: &str = "rust/crates/quoin-quire/schemas/assurance-v1.schema.json";

/// The `quire-rs` revision these bytes were copied out of.
///
/// Not the revision `Cargo.toml` links, and deliberately so: the linked engine
/// may be newer while emitting the same schema, which is the case the retained
/// relock stated as *"a newer engine with unchanged schemas may retain the
/// older, exact contract source"*. The two numbers are the whole provenance
/// the deleted `src/quire/contract.ts` carried for this document (quoin#502);
/// they live here because this crate is now the only thing that vendors it.
pub const VENDORED_SOURCE_REVISION: &str = "42326bbdf8f6641203eebf7a5faaa2b22bc19b0f";

/// SHA-256 of [`SOURCE`] at [`VENDORED_SOURCE_REVISION`].
///
/// Derived from the upstream git object, never from this working tree: an edit
/// here without a matching upstream refresh fails `tc_474_011` rather than
/// quietly teaching quoin a contract quire does not emit.
pub const VENDORED_SHA256: &str =
    "441e1b8324fe64e234a007216b11867ec937ac4779fcebcdd84b30fa064367e5";

/// The committed schema bytes.
///
/// `include_str!`, never `std::fs`: a schema read off disk at run time is a
/// schema that can differ from the one the tests measured.
pub const SOURCE: &str = include_str!("../schemas/assurance-v1.schema.json");

/// A document that satisfied `assurance-v1`.
///
/// Parse, don't validate — modelled on `quoin_store::RawFileSha256Digest`
/// and [`quoin_jsonschema::ValidDocument`]: holding the value *is* the proof,
/// so it is constructed by [`validate_assurance`] and nowhere else and no
/// caller re-checks what it was handed.
///
/// It is a distinct type rather than a reuse of [`quoin_jsonschema::ValidDocument`]
/// because that one is indexed by `quoin_jsonschema::VendoredSchema`, the
/// closed enum of the two **measurement** documents. `assurance-v1` is a quire
/// output contract owned by this crate; widening that enum would put it in the
/// measurement crate's namespace and hand it that enum's format policy, which
/// is the opposite of the one this schema needs. The validator is shared; only
/// the proof token is local.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidAssuranceDocument(Value);

impl ValidAssuranceDocument {
    /// The document.
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.0
    }

    /// The document, by value.
    #[must_use]
    pub fn into_value(self) -> Value {
        self.0
    }
}

/// Compiled once per process, as `src/quire/validate.ts:41` compiles once per
/// process: compilation is the expensive half and the vendored document never
/// changes at run time.
///
/// The error is kept as its rendered text because [`SchemaValidator`] is not
/// `Clone` and a compile failure is a defect in this repository rather than
/// ordinary data — every caller after the first gets the same sentence.
static VALIDATOR: OnceLock<std::result::Result<SchemaValidator, String>> = OnceLock::new();

fn validator() -> Result<&'static SchemaValidator> {
    VALIDATOR
        .get_or_init(|| {
            let document: Value = serde_json::from_str(SOURCE)
                .map_err(|source| format!("the vendored document is not JSON: {source}"))?;
            SchemaValidator::compile_vendored(Path::new(VENDORED_PATH), &document, &[])
                .map_err(|source| source.to_string())
        })
        .as_ref()
        .map_err(|detail| Error::VendoredSchemaInvalid {
            path: Path::new(VENDORED_PATH).to_path_buf(),
            detail: detail.clone(),
        })
}

/// Validate a supplied `assurance-v1` export against the published schema.
///
/// The Rust successor to `validateAssurance` (`src/quire/validate.ts:99`), for
/// the callers that hold an export from another repository and no premises to
/// read it under. A caller that *does* have premises wants
/// [`crate::assurance::read`], which is strictly stronger.
///
/// # Errors
///
/// [`Error::AssuranceContract`] when the document does not satisfy the
/// contract, carrying one line per failing instance path — the shape
/// `ContractViolation.errors` had. [`Error::VendoredSchemaInvalid`] when the
/// committed schema does not compile, which is a defect in this repository.
pub fn validate_assurance(document: Value) -> Result<ValidAssuranceDocument> {
    let errors = validator()?.errors(&document);
    if errors.is_empty() {
        return Ok(ValidAssuranceDocument(document));
    }
    Err(Error::AssuranceContract {
        violations: errors
            .iter()
            .map(|error| {
                let at = if error.instance_path.is_empty() {
                    "<root>"
                } else {
                    error.instance_path.as_str()
                };
                format!("{at}: {}", error.message)
            })
            .collect(),
    })
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    /// Trace: FR-099-AC-2
    /// Provenance: quoin#474
    #[test]
    fn tc_474_001_the_vendored_document_compiles() {
        validator().expect("the committed assurance-v1 schema compiles");
    }

    /// Trace: FR-099-AC-2
    /// Provenance: quoin#474
    #[test]
    fn tc_474_002_a_refusal_names_every_failing_instance_path() {
        let error = validate_assurance(serde_json::json!({})).expect_err("an empty object fails");
        assert_eq!(error.code(), crate::ErrorCode::AssuranceContract);
        let crate::Error::AssuranceContract { violations } = error else {
            panic!("the variant carries the lines");
        };
        assert!(
            !violations.is_empty() && violations.iter().all(|line| line.contains(": ")),
            "each line is `<path>: <reason>`, the shape `describe()` produced: {violations:?}"
        );
    }
}
