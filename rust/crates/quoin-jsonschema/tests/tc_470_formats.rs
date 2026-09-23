// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! `format: "date-time"` asserts, on both schemas, against the one grammar.
//!
//! # The trap this closes
//!
//! ajv under `strict: false` asserts a `format` the moment a check is
//! registered for it, and both retained sites register one. The `jsonschema`
//! crate under Draft 2020-12 does the opposite: `format` is an *annotation*
//! until `should_validate_formats(true)` is set, so a straight transliteration
//! compiles cleanly, passes every structural test, and silently accepts
//! `"2026-02-30T00:00:00Z"` — a date the calendar does not have, refused by ajv
//! and accepted by Rust. That is a verdict difference, and verdicts are the
//! contractual half.
//!
//! So the wiring is tested directly and from both sides: the same document,
//! through a validator built with the format registered and through one built
//! without it, must get different verdicts. A test that only asserted the
//! refusal would pass even if some other keyword were doing the work.
//!
//! The operational schema is the sharper probe: its nine `date-time` fields
//! carry **no** `pattern`, so `format` is the only thing that can refuse them.
//!
//! Trace: FR-100-AC-4, FR-098

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::Path;

use quoin_jsonschema::{
    JsonSchemaError, JsonSchemaErrorCode, MeasurementSchema, SchemaKeyword, SchemaValidator,
};
use quoin_measurement::date_time::Rfc3339DateTime;
use serde_json::{Value, json};

fn is_rfc3339_date_time(value: &str) -> bool {
    Rfc3339DateTime::parse(value).is_ok()
}

/// A retained record of this shape, from the captured corpus.
///
/// The probe is a document that actually occurs with one field changed, not a
/// stub: a hand-built object would be refused by a dozen structural keywords
/// and `format` would never be the thing under test. The capture predates
/// PLAT-972, so the family's one admissible `strength` (DIVERGENCE.md §7) is
/// added here rather than written into the ajv golden.
fn retained_base(schema: MeasurementSchema) -> Value {
    let (name, strength) = match schema {
        MeasurementSchema::InterventionExperimentV1 => ("intervention_experiment_v1", "tested"),
        MeasurementSchema::OperationalEvidenceV1 => ("operational_evidence_v1", "observed"),
        other => panic!("no retained base recorded for {other}"),
    };
    let goldens: Value = serde_json::from_str(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("goldens")
                .join("ajv-verdicts.json"),
        )
        .expect("the captured verdicts are readable"),
    )
    .expect("the captured verdicts are JSON");
    let entry = goldens["entries"]
        .as_array()
        .expect("entries is an array")
        .iter()
        .find(|e| {
            e["schema"].as_str() == Some(name)
                && e["mutation"].as_str() == Some("none")
                && e["ajv_valid"].as_bool() == Some(true)
        })
        .unwrap_or_else(|| panic!("the corpus retains an accepted {name} record"))
        .clone();
    let mut document = entry["document"].clone();
    document
        .as_object_mut()
        .expect("a record is an object")
        .insert("strength".to_owned(), json!(strength));
    document
}

fn with_observed_at(schema: MeasurementSchema, value: &str) -> Value {
    let mut document = retained_base(schema);
    document
        .as_object_mut()
        .expect("a record is an object")
        .insert("observed_at".to_owned(), json!(value));
    document
}

fn asserted(schema: MeasurementSchema) -> SchemaValidator {
    SchemaValidator::compile_with_formats(
        Path::new(schema.vendored_path()),
        &schema.document().expect("the vendored schema parses"),
        &[],
        &[quoin_jsonschema::FormatCheck {
            name: "date-time",
            check: is_rfc3339_date_time,
        }],
    )
    .expect("compiles with formats")
}

fn annotated(schema: MeasurementSchema) -> SchemaValidator {
    SchemaValidator::compile_vendored(
        Path::new(schema.vendored_path()),
        &schema.document().expect("the vendored schema parses"),
        &[],
    )
    .expect("compiles without formats")
}

/// With the format registered, an impossible calendar day is refused; without
/// it, the same document is accepted.
///
/// Trace: FR-100-AC-4
#[test]
fn tc_470_date_time_is_asserted_on_both_schemas_and_annotation_only_without_it() {
    // 2026-02-30 matches the intervention schema's `pattern`, and the
    // operational schema carries no pattern on any of its nine `date-time`
    // fields, so in both cases the only thing that can refuse it is the
    // registered check.
    for schema in MeasurementSchema::ALL {
        let impossible = with_observed_at(*schema, "2026-02-30T00:00:00Z");

        let errors = asserted(*schema).errors(&impossible);
        assert_eq!(
            errors
                .iter()
                .map(quoin_jsonschema::SchemaError::identity)
                .collect::<Vec<_>>(),
            vec![("/observed_at", SchemaKeyword::Format)],
            "{schema}: a date the calendar does not have must be refused by `format`, and by \
             nothing else — {errors:#?}"
        );

        assert!(
            annotated(*schema).is_valid(&impossible),
            "{schema}: without a registered check, `format` must stay an annotation — if this \
             fails, the two constructors no longer differ and `quoin-semantic`'s goldens were \
             captured against the other one"
        );
    }
}

/// The lowercase RFC 3339 form the ruling widened to is accepted end to end.
///
/// The pattern and the registered check must accept the same language: if the
/// widening had landed in only one of them, this passes on one schema and fails
/// on the other.
///
/// Trace: FR-098
#[test]
fn tc_470_the_widened_lowercase_form_is_accepted_by_pattern_and_by_format() {
    for schema in MeasurementSchema::ALL {
        let validator = asserted(*schema);
        for value in [
            "2026-01-01t00:00:00z",
            "2026-01-01T00:00:00z",
            "2026-01-01t00:00:00Z",
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00+01:00",
        ] {
            let document = with_observed_at(*schema, value);
            assert!(
                validator.is_valid(&document),
                "{schema}: {value} is RFC 3339 §5.6 and must be accepted — {:#?}",
                validator.errors(&document)
            );
        }
        let spaced = with_observed_at(*schema, "2026-01-01 00:00:00Z");
        assert!(
            !validator.is_valid(&spaced),
            "{schema}: a space separator is not RFC 3339 and must stay refused"
        );
    }
}

/// A refused document comes back as a refusal, and an accepted one as a value.
///
/// Parse, don't validate: the caller cannot hold a [`quoin_jsonschema::ValidDocument`]
/// without having passed the schema, so nothing downstream re-validates.
///
/// Trace: FR-100-AC-4
#[test]
fn tc_470_parsing_returns_the_document_or_the_refusal() {
    let validator = MeasurementSchema::InterventionExperimentV1
        .compile(is_rfc3339_date_time)
        .expect("compiles");

    let refusal = validator
        .parse(json!({}))
        .expect_err("an empty object is not an intervention experiment");
    assert_eq!(refusal.code(), JsonSchemaErrorCode::DocumentRefused);
    assert!(
        !refusal.errors().is_empty(),
        "a refusal must carry the failures it refused on"
    );
    match &refusal {
        JsonSchemaError::DocumentRefused { schema, .. } => {
            assert_eq!(*schema, MeasurementSchema::InterventionExperimentV1);
        }
        other => panic!("expected a refusal, got {other:?}"),
    }

    let retained: Value = serde_json::from_str(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("..")
                .join("spec/evidence/interventions/quoin-270-cli-eval-sentinel-contract.json"),
        )
        .expect("the retained record is readable"),
    )
    .expect("the retained record is JSON");
    let parsed = validator
        .parse(retained.clone())
        .expect("the retained record satisfies the schema it was written against");
    assert_eq!(parsed.schema(), MeasurementSchema::InterventionExperimentV1);
    assert_eq!(parsed.value(), &retained);
    assert_eq!(parsed.into_value(), retained);
}

/// A vendored document that does not compile is refused under its own code.
///
/// Trace: FR-100-AC-4
#[test]
fn tc_470_a_schema_that_does_not_compile_names_itself() {
    let error = SchemaValidator::compile_vendored(
        Path::new("vendored.json"),
        &json!({"$ref": "https://absent.test/nope.json"}),
        &[],
    )
    .expect_err("an unresolvable $ref must not compile");
    assert_eq!(error.code(), JsonSchemaErrorCode::VendoredSchemaInvalid);
    assert!(
        error.to_string().contains("vendored.json"),
        "a compile failure must say which document: {error}"
    );
    assert!(error.errors().is_empty());
}

/// Every error code round-trips through its wire spelling.
///
/// Trace: FR-100-AC-4
#[test]
fn tc_470_every_error_code_round_trips() {
    assert!(
        !JsonSchemaErrorCode::ALL.is_empty(),
        "anti-vacuity floor: the code census must not be empty"
    );
    for code in JsonSchemaErrorCode::ALL {
        assert_eq!(
            JsonSchemaErrorCode::from_code(code.as_str()),
            Some(*code),
            "{code} does not round-trip"
        );
        assert!(
            code.as_str().starts_with("jsonschema."),
            "{code} must be namespaced to this crate"
        );
    }
    assert_eq!(JsonSchemaErrorCode::from_code("jsonschema.absent"), None);
}
