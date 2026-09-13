// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading an intervention record, schema first and semantics beside it.
//!
//! A port of `validateInterventionRecord` (`intervention.ts:47-63`).
//!
//! # One refusal, both passes
//!
//! The oracle runs the compiled ajv validator and `semanticFindings` and puts
//! **both** result sets into one `invalid_record` refusal, deduplicated and
//! sorted. A caller therefore sees every disagreement at once rather than
//! fixing them one round trip at a time, and that is reproduced exactly: the
//! two passes run unconditionally, the findings merge, and the sort is the
//! oracle's plain string sort.
//!
//! # Parse, don't validate
//!
//! `validateInterventionRecord` is a TypeScript assertion function: it narrows
//! the caller's `unknown` and returns nothing, so every caller keeps handling
//! the loosely-typed object it passed in. Here it returns an
//! [`InterventionExperimentRecord`], the way
//! `quoin_store::RawFileSha256Digest::parse_stored` returns the digest. What is
//! refused is the same; what a caller holds afterwards is not.
//!
//! # The schema is compiled once
//!
//! `intervention.ts:22-27` compiles at module load. Here it is a [`OnceLock`]
//! over [`VendoredSchema::InterventionExperimentV1`], which is quoin#470's
//! vendored document — this crate does not carry a second copy of it, and the
//! `date-time` format is registered with this crate's one RFC 3339 grammar
//! rather than a private second one.

use std::sync::OnceLock;

use quoin_jsonschema::{JsonSchemaError, VendoredSchema, VendoredValidator};
use serde_json::Value;

use crate::common::schema::{findings as schema_findings, sorted_unique};
use crate::date_time::Rfc3339DateTime;
use crate::intervention::intake::{InterventionIntakeError, InterventionRefusalCode};
use crate::intervention::record::InterventionExperimentRecord;
use crate::intervention::semantics::semantic_findings;

/// The compiled intervention-experiment validator, built once per process.
fn validator() -> Result<&'static VendoredValidator, &'static JsonSchemaError> {
    static COMPILED: OnceLock<Result<VendoredValidator, JsonSchemaError>> = OnceLock::new();
    COMPILED
        .get_or_init(|| {
            VendoredSchema::InterventionExperimentV1
                .compile(|value| Rfc3339DateTime::parse(value).is_ok())
        })
        .as_ref()
}

/// Read a candidate as an intervention record.
///
/// # Errors
///
/// [`InterventionRefusalCode::InvalidRecord`], carrying every schema and
/// semantic finding, deduplicated and sorted, exactly as
/// `intervention.ts:58-62` assembles them.
pub fn validate_intervention_record(
    candidate: &Value,
) -> Result<InterventionExperimentRecord, InterventionIntakeError> {
    let mut findings = schema_pass(candidate)?;
    if let Some(object) = candidate.as_object() {
        findings.extend(semantic_findings(object));
    }
    if !findings.is_empty() {
        return Err(refusal(findings));
    }
    // Reached only once the schema has accepted the document, so a failure
    // here is a disagreement between the schema and the typed model rather
    // than ordinary bad input — see `DIVERGENCE.md` §8 for the one shape where
    // that can happen (`"size_bytes": 12.0`).
    serde_json::from_value(candidate.clone()).map_err(|error| {
        refusal(vec![format!(
            "/: schema-valid but unreadable as an intervention record: {error}"
        )])
    })
}

/// The schema pass, over the validator this family compiles.
///
/// The rendering is [`crate::common::schema::findings`]'s, shared with the
/// operational family: ajv and the Rust `jsonschema` crate agree on the
/// **verdict** and on `instancePath`; the sentence after the colon is each
/// library's own and is not contractual, which quoin#470's `DIVERGENCE.md`
/// already records.
fn schema_pass(candidate: &Value) -> Result<Vec<String>, InterventionIntakeError> {
    let validator = validator().map_err(|error| {
        refusal(vec![format!(
            "/: the vendored intervention schema does not compile: {error}"
        )])
    })?;
    Ok(schema_findings(validator, candidate))
}

/// `[...new Set(findings)].sort(compare)`, the crate's one spelling of it.
fn refusal(findings: Vec<String>) -> InterventionIntakeError {
    InterventionIntakeError::new(
        InterventionRefusalCode::InvalidRecord,
        sorted_unique(findings),
    )
}
