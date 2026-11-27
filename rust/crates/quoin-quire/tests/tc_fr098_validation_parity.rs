// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Golden validation fixtures keep their verdict and diagnostic boundary (FR-098).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "fixture contract failures panic to report the violated case"
)]

use ix_trace_rs::trace;
use quoin_jsonschema::{SchemaKeyword, SchemaValidator};
use quoin_quire::schema::validate_assurance;
use serde_json::{Value, json};
use std::path::Path;

fn assurance_bases() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("goldens")
        .join("quoin-quire")
        .join("assurance-bases.json");
    serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read golden corpus {}: {error}", path.display())),
    )
    .expect("golden assurance corpus is valid JSON")
}

#[trace("FR-098-AC-1")]
#[test]
fn native_validator_replays_each_committed_assurance_base_and_reports_population() {
    let corpus = assurance_bases();
    let bases = corpus["bases"]
        .as_array()
        .expect("the golden corpus has a bases array");
    let mut compared = 0;

    for base in bases {
        let name = base["name"].as_str().expect("each base names its fixture");
        validate_assurance(base["document"].clone()).unwrap_or_else(|error| {
            panic!("{name}: the retained expected-valid base refuses: {error}")
        });
        compared += 1;
    }

    eprintln!("FR-098-AC-1 compared assurance fixture population: {compared}");
    assert!(compared > 0, "inconclusive: the golden corpus has no bases");
    assert_eq!(compared, bases.len(), "every committed base is compared");
}

#[trace("FR-098-AC-8")]
#[test]
fn validation_emits_instance_diagnostics_without_format_or_schema_authoring_findings() {
    let schema = json!({
        "type": "object",
        "required": ["id"],
        "properties": {
            "id": { "type": "string", "format": "uuid" }
        }
    });
    let validator = SchemaValidator::compile_vendored(Path::new("schema.json"), &schema, &[])
        .expect("schema with an unsupported annotation format compiles");

    let retained_valid = json!({"id": "not-a-uuid"});
    assert!(
        validator.is_valid(&retained_valid),
        "format is annotation-only when no format validator is registered"
    );
    assert!(validator.errors(&retained_valid).is_empty());

    let invalid_instance = json!({"id": 42});
    let errors = validator.errors(&invalid_instance);
    let reported: Vec<_> = errors
        .iter()
        .map(|error| (error.instance_path.as_str(), error.keyword))
        .collect();
    assert_eq!(reported, [("/id", SchemaKeyword::Type)]);
    assert!(
        errors
            .iter()
            .all(|error| error.keyword != SchemaKeyword::Format),
        "format annotation must not become an instance diagnostic"
    );
    assert!(
        errors.iter().all(|error| !error.message.contains("strict")),
        "schema-authoring strict-mode diagnostics are outside the compared instance-error set"
    );
}
