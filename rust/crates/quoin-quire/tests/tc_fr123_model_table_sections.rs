// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Quoin's FR-123 mapping contract against the canonical quire-fixtures and
//! the pinned quire-rs semantic extractor.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "invalid shared fixtures are test failures"
)]

use ix_trace_rs::trace;
use quire_rs::semantic::python_entry::extract_semantic_json;
use quire_rs::semantic::{compile_documents, module_schema_documents};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn fixture_case(id: &str) -> (Value, Value, BTreeMap<String, Value>) {
    let fixtures = quire_fixtures::record_sections_json("cases.json");
    let case = fixtures["cases"]
        .as_array()
        .and_then(|cases| cases.iter().find(|case| case["id"] == id))
        .unwrap_or_else(|| panic!("quire-fixtures has no FR-123 record-sections case {id}"))
        .clone();
    let schema_dir = quire_fixtures::record_sections_dir().join("schemas");
    let schema_name = case["schema"].as_str().expect("case schema name");
    let schema_text = std::fs::read_to_string(schema_dir.join(format!("{schema_name}.json")))
        .expect("fixture schema is readable from quire-fixtures");
    let schema: Value = serde_json::from_str(&schema_text).expect("fixture schema is valid JSON");
    let id = schema["$id"].as_str().expect("fixture schema id");
    let base = &id[..=id.rfind('/').expect("schema id has path")];
    let documents = module_schema_documents(
        &schema,
        &|name| {
            let text = std::fs::read_to_string(schema_dir.join(name)).ok()?;
            Some(serde_json::from_str(&text).map_err(|error| error.to_string()))
        },
        base,
    )
    .expect("fixture schema documents compile");
    (case, schema, documents)
}

fn run_fixture(id: &str) -> (Value, Value) {
    let (case, schema, schema_documents) = fixture_case(id);
    let mut request = json!({
        "markdown": case["artifact"],
        "module": {
            "contractVersion": "1.0.0",
            "package": case.get("package").unwrap_or(&json!("acme/safety")),
            "exports": [],
            "mappings": case.get("mappings").unwrap_or(&json!(["record-sections"])),
        },
        "path": "artifact.md",
        "sourceIdentity": "ix://acme/safety/spec",
        "bundle": case
            .get("bundle")
            .unwrap_or(&json!({"package": "acme/safety"})),
        "dataSchema": schema,
        "schemaDocuments": schema_documents,
    });
    for key in [
        "relationVocabulary",
        "bodyExtraction",
        "constructReferences",
        "objectType",
    ] {
        if let Some(value) = case.get(key) {
            request[key] = value.clone();
        }
    }
    let extraction = extract_semantic_json(&request).expect("pinned Quire extracts fixture case");
    (
        case,
        serde_json::to_value(extraction).expect("semantic output serializes"),
    )
}

fn is_valid(schema_name: &str, instance: &Value) -> bool {
    let schema_dir = quire_fixtures::record_sections_dir().join("schemas");
    let schema_text =
        std::fs::read_to_string(schema_dir.join(schema_name)).expect("fixture schema is readable");
    let schema: Value = serde_json::from_str(&schema_text).expect("fixture schema is valid JSON");
    let id = schema["$id"].as_str().expect("fixture schema id");
    let base = &id[..=id.rfind('/').expect("schema id has path")];
    let documents = module_schema_documents(
        &schema,
        &|name| {
            let text = std::fs::read_to_string(schema_dir.join(name)).ok()?;
            Some(serde_json::from_str(&text).map_err(|error| error.to_string()))
        },
        base,
    )
    .expect("fixture schema references resolve");
    let validator = compile_documents(&schema, &documents).expect("fixture schema compiles");
    validator.is_valid(instance)
}

fn validate(schema_name: &str, instance: &Value) {
    assert!(
        is_valid(schema_name, instance),
        "{schema_name}: {instance:#}"
    );
}

fn line_of(markdown: &str, snippet: &str) -> usize {
    markdown
        .lines()
        .position(|line| line.contains(snippet))
        .expect("fixture contains diagnostic snippet")
        + 1
}

fn assert_diagnostic(case: &Value, output: &Value, code: &str, reason: &str, snippet: &str) {
    let line = line_of(
        case["artifact"].as_str().expect("fixture artifact"),
        snippet,
    );
    assert!(
        output["diagnostics"].as_array().is_some_and(|diagnostics| {
            diagnostics.iter().any(|diagnostic| {
                diagnostic["code"] == code
                    && diagnostic["reason"].as_str().unwrap_or_default() == reason
                    && diagnostic["line"] == line
            })
        }),
        "expected {code} ({reason}) at line {line}: {:?}",
        output["diagnostics"]
    );
}

fn record_source_count(output: &Value, key: &str) -> usize {
    output["recordSources"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|source| source["key"] == key)
        .count()
}

#[trace("FR-123-AC-1")]
#[test]
fn enumeration_values_map_and_validate_against_the_golden_schemas() {
    let (case, output) = run_fixture("values-table");
    assert_eq!(output["record"]["values"], case["record"]["values"]);
    assert_eq!(
        output["record"]["values"],
        json!([
            {"value": "draft", "doc": "Not yet reviewed"},
            {"value": "final"}
        ])
    );
    for value in output["record"]["values"].as_array().unwrap() {
        validate("EnumValue.json", value);
    }
    validate("Enumeration.json", &output["record"]);
    assert_eq!(output["diagnostics"], json!([]));
}

#[trace("FR-123-AC-2")]
#[test]
fn values_refuse_legacy_form_duplicates_and_non_identifiers() {
    let (case, output) = run_fixture("values-list-refused");
    assert_diagnostic(
        &case,
        &output,
        "semantic.feature-not-extractable",
        "values",
        "- draft",
    );
    assert!(output["record"].is_null() || output["record"].get("values").is_none());

    let (case, output) = run_fixture("values-duplicate");
    assert_diagnostic(
        &case,
        &output,
        "semantic.duplicate-model-entry",
        "",
        "| draft | b |",
    );
    assert!(output["record"].is_null() || output["record"].get("values").is_none());

    let (case, output) = run_fixture("values-invalid");
    assert_diagnostic(
        &case,
        &output,
        "semantic.invalid-model-cell",
        "",
        "| not valid |",
    );
    assert!(output["record"].is_null() || output["record"].get("values").is_none());
}

#[trace("FR-123-AC-3")]
#[test]
fn state_machine_and_process_tables_map_to_their_declared_record_keys() {
    let (case, output) = run_fixture("states-and-transitions");
    assert_eq!(output["record"]["states"], case["record"]["states"]);
    assert_eq!(
        output["record"]["transitions"],
        case["record"]["transitions"]
    );
    assert_eq!(
        output["record"]["transitions"][0]["guard"],
        json!({"language": "quire", "clauseId": "NotCancelled"})
    );
    validate("Machine.json", &output["record"]);

    let (case, output) = run_fixture("steps-copied-states-stay-in-model");
    assert_eq!(output["record"]["steps"], case["record"]["steps"]);
    assert_eq!(
        output["record"]["steps"][0]["consumes"],
        json!(["ix://acme/shop/order-placed"])
    );
    assert_eq!(
        output["record"]["steps"][0]["emits"],
        json!(["ix://acme/shop/order-placed", "ix://acme/shop/money"])
    );
    assert!(output["record"].get("states").is_none());
    assert_eq!(case["modelStatesAbsentFromRecord"], true);
    validate("Workflow.json", &output["record"]);
}

#[trace("FR-123-AC-4")]
#[test]
fn aggregate_members_and_domain_vocabulary_follow_the_golden_mapping() {
    let (case, output) = run_fixture("members-composite-verbs");
    assert_eq!(output["record"]["members"], case["record"]["members"]);
    assert_eq!(
        output["record"]["members"][0]["target"],
        "ix://acme/shop/order-line"
    );
    assert_eq!(output["record"]["members"][0]["verb"], "aggregates");
    assert_eq!(
        output["record"]["members"][1]["target"],
        "ix://acme/shop/money"
    );
    assert_eq!(output["record"]["members"][1]["verb"], "contains");
    assert!(
        output["record"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .all(|member| member["composite"] == true)
    );
    validate("Aggregate.json", &output["record"]);

    let (case, output) = run_fixture("members-member-verb");
    assert_diagnostic(
        &case,
        &output,
        "semantic.invalid-model-cell",
        "member-verb",
        "| Odd |",
    );

    let (case, output) = run_fixture("members-declared-in-members");
    assert_diagnostic(
        &case,
        &output,
        "semantic.duplicate-model-entry",
        "declared-in-members",
        "| lines |",
    );

    let (case, output) = run_fixture("domain-vocabulary");
    assert_eq!(output["record"]["vocabulary"], case["record"]["vocabulary"]);
    assert_eq!(
        output["record"]["vocabulary"][0],
        json!({"term": "Backorder", "doc": "Ordered but not in stock"})
    );
    validate("Domain.json", &output["record"]);

    let (case, output) = run_fixture("domain-vocabulary-empty-description");
    assert_eq!(output["record"]["vocabulary"], case["record"]["vocabulary"]);
    assert!(output["record"]["vocabulary"][0].get("doc").is_none());
    assert!(
        !is_valid("Domain.json", &output["record"]),
        "Term requires doc"
    );
    assert_eq!(output["diagnostics"], json!([]));
}

#[trace("FR-123-AC-5")]
#[test]
fn declared_table_locators_read_each_record_key_once() {
    for (case_id, keys) in [
        ("values-table", &["values"][..]),
        ("states-and-transitions", &["states", "transitions"][..]),
        ("steps-copied-states-stay-in-model", &["steps"][..]),
        ("members-composite-verbs", &["members"][..]),
    ] {
        let (_, output) = run_fixture(case_id);
        for key in keys {
            assert_eq!(
                record_source_count(&output, key),
                1,
                "{case_id} reads {key} once by its table locator"
            );
        }
        assert!(
            !output["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| {
                    diagnostic["code"] == "semantic.invalid-model-cell"
                        && diagnostic["reason"] == "column-unknown"
                }),
            "{case_id} does not reread its table through record-sections"
        );
    }
}

#[trace("FR-123-AC-7")]
#[test]
fn owner_and_domain_prose_do_not_create_record_keys() {
    let (_, nested) = run_fixture("nested-entity-owner-is-prose");
    assert!(nested["record"].is_null() || nested["record"].get("owner").is_none());
    assert_eq!(nested["diagnostics"], json!([]));

    let (_, domain) = run_fixture("domain-vocabulary");
    assert!(domain["record"].get("ubiquitous_language").is_none());
    assert!(domain["record"].get("ubiquitousLanguage").is_none());
    assert_eq!(domain["diagnostics"], json!([]));
}

#[trace("FR-123-AC-9")]
#[test]
fn header_only_values_table_is_available_and_empty() {
    let (_, output) = run_fixture("values-header-only");
    assert_eq!(output["record"]["values"], json!([]));
    assert_eq!(
        output["availability"]["recordKeys"]["values"]["state"],
        "available"
    );
    assert_eq!(output["diagnostics"], json!([]));
}
