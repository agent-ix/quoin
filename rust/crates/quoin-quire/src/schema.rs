// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! `assurance-v1` schema validation (quoin#474).
//!
//! The schema itself is `quire-rs`'s: this crate already links `quire-rs` as
//! its engine (`Cargo.toml`), and `assurance-v1.schema.json`'s `$id` names
//! `agent-ix.github.io/quire-rs/...` -- it is quire-rs's document, not
//! quoin's. A second copy under `schemas/` here, kept in step by hand, could
//! only drift from the one the linked engine actually emits against; using
//! [`quire_rs::assurance::ASSURANCE_V1_SCHEMA`] directly means the schema this
//! crate validates against and the schema the linked engine was built from
//! are the same bytes by construction.
//!
//! ## Why the schema check exists here at all
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
//! `format`, `uuid` on `$defs.artifact.uuid`
//! (`quire_rs::assurance::ASSURANCE_V1_SCHEMA`), and under that ajv it is an
//! annotation: `"not-a-uuid"` is accepted today.
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

/// A name for the schema's origin, for a compile failure to cite. Not a path
/// in this working tree: the schema comes from the linked `quire-rs` crate,
/// not from a file quoin ships.
pub const SOURCE_PATH: &str = "quire_rs::assurance::ASSURANCE_V1_SCHEMA";

/// The schema bytes, straight from the linked `quire-rs` engine.
///
/// Not `include_str!` of a local copy: `quire-rs` already embeds this schema
/// (`quire_rs::assurance::ASSURANCE_V1_SCHEMA`, itself an `include_str!` in
/// quire-rs's own source) and re-embedding it here a second time is exactly
/// the drift a vendored copy risks — the crate this document validates
/// against and the crate that produces it would then be two different
/// pinned revisions unless someone remembered to update both. Consuming the
/// constant means they cannot drift: whatever `Cargo.toml` links is what this
/// module validates against.
pub const SOURCE: &str = quire_rs::assurance::ASSURANCE_V1_SCHEMA;

/// A document that satisfied `assurance-v1`.
///
/// Parse, don't validate — modelled on `quoin_store::RawFileSha256Digest`
/// and [`quoin_jsonschema::ValidDocument`]: holding the value *is* the proof,
/// so it is constructed by [`validate_assurance`] and nowhere else and no
/// caller re-checks what it was handed.
///
/// It is a distinct type rather than a reuse of [`quoin_jsonschema::ValidDocument`]
/// because that one is indexed by `quoin_jsonschema::MeasurementSchema`, the
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
            SchemaValidator::compile_vendored(Path::new(SOURCE_PATH), &document, &[])
                .map_err(|source| source.to_string())
        })
        .as_ref()
        .map_err(|detail| Error::VendoredSchemaInvalid {
            path: Path::new(SOURCE_PATH).to_path_buf(),
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
