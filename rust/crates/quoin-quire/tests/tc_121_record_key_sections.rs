// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Quoin's record-key mapping exercised through the pinned Quire engine.
//! Golden markdown and expected outputs remain owned by quire-fixtures.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quire_rs::semantic::{compile_documents, module_schema_documents};
use quire_rs::semantic::python_entry::extract_semantic_json;
use serde_json::{json, Value};

fn fixture_case(id: &str) -> (Value, Value, BTreeMap<String, Value>) {
    let fixtures = quire_fixtures::record_sections_json("cases.json");
    let case = fixtures["cases"]
        .as_array()
        .and_then(|cases| cases.iter().find(|case| case["id"] == id))
        .unwrap_or_else(|| panic!("quire-fixtures has no record-sections case {id}"))
        .clone();
    let root_name = case["schema"].as_str().expect("case schema name");
    let schema_dir = quire_fixtures::record_sections_dir().join("schemas");
    let root_text = std::fs::read_to_string(schema_dir.join(format!("{root_name}.json")))
        .expect("fixture schema is readable from quire-fixtures");
    let root: Value = serde_json::from_str(&root_text).expect("fixture schema is valid JSON");
    let id = root["$id"].as_str().expect("fixture schema id");
    let base = &id[..=id.rfind('/').expect("schema id has path")];
    let docs = module_schema_documents(
        &root,
        &|name| {
            let text = std::fs::read_to_string(schema_dir.join(name)).ok()?;
            Some(serde_json::from_str(&text).map_err(|error| error.to_string()))
        },
        base,
    )
    .expect("fixture schema documents compile");
    (case, root, docs)
}

fn run_fixture(id: &str) -> (Value, Value, Value) {
    let (case, root, docs) = fixture_case(id);
    let request = json!({
        "markdown": case["artifact"],
        "module": {
            "contractVersion": "1.0.0",
            "package": case.get("package").unwrap_or(&json!("acme/safety")),
            "exports": [],
            "mappings": case.get("mappings").unwrap_or(&json!(["record-sections"])),
        },
        "path": "artifact.md",
        "sourceIdentity": "ix://acme/safety/spec",
        "bundle": case.get("bundle").unwrap_or(&json!({"package": "acme/safety"})),
        "dataSchema": root,
        "schemaDocuments": docs,
    });
    let extraction = extract_semantic_json(&request).expect("Quire extracts fixture case");
    (
        case,
        serde_json::to_value(extraction).expect("semantic output serializes"),
        root,
    )
}

fn diagnostics(output: &Value) -> Vec<(String, String)> {
    output["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|diagnostic| {
            (
                diagnostic["code"].as_str().unwrap_or_default().to_string(),
                diagnostic["reason"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

fn synthetic(markdown: &str, schema: Value, mappings: &[&str], body_extraction: Option<Value>) -> Value {
    let mut request = json!({
        "markdown": markdown,
        "module": {
            "contractVersion": "1.0.0",
            "package": "acme/synthetic",
            "exports": [],
            "mappings": mappings,
        },
        "path": "synthetic.md",
        "sourceIdentity": "ix://acme/synthetic/spec",
        "bundle": {"package": "acme/synthetic"},
        "dataSchema": schema,
        "schemaDocuments": {},
    });
    if let Some(body_extraction) = body_extraction {
        request["bodyExtraction"] = body_extraction;
    }
    serde_json::to_value(extract_semantic_json(&request).expect("Quire extracts synthetic case"))
        .expect("semantic output serializes")
}

#[trace("FR-121-AC-1")]
#[test]
fn hazard_assessment_uses_the_shared_golden_and_reports_its_source() {
    let (case, output, root) = run_fixture("assessment-row");
    assert_eq!(output["record"], case["record"]);
    assert_eq!(output["recordSources"][0]["key"], "assessment");
    let span = &output["recordSources"][0]["sourceSpan"];
    assert_eq!(span["startLine"], 9);
    assert_eq!(span["endLine"], 11);
    let validator = compile_documents(&root, &fixture_case("assessment-row").2)
        .expect("fixture record schema compiles");
    assert!(validator.is_valid(&output["record"]));
}

#[trace("FR-121-AC-2")]
#[test]
fn failure_analysis_table_maps_using_the_schema_property() {
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://schemas.example.test/FailureMode.json",
        "type": "object",
        "properties": {"analysis": {
            "type": "object",
            "properties": {
                "effect": {"type": "string"},
                "cause": {"type": "string"},
                "detection": {"type": "string"}
            },
            "required": ["effect", "cause", "detection"],
            "additionalProperties": false
        }},
        "required": ["analysis"],
        "additionalProperties": false
    });
    let output = synthetic(
        "# FM-1\n\n## Analysis\n\n| Effect | Cause | Detection |\n|---|---|---|\n| Injury | Seal failure | Visual inspection |\n",
        schema,
        &["record-sections"],
        None,
    );
    assert_eq!(output["record"]["analysis"], json!({
        "effect": "Injury", "cause": "Seal failure", "detection": "Visual inspection"
    }));
    assert!(diagnostics(&output).is_empty());
}

#[trace("FR-121-AC-3")]
#[test]
fn not_assessed_empty_cells_and_absent_assessment_follow_the_golden() {
    let (case, output, _) = run_fixture("assessment-row");
    assert_eq!(output["record"], case["record"]);
    assert_eq!(output["record"]["assessment"]["likelihood"], "not_assessed");
    assert!(output["record"]["assessment"].get("exposure").is_none());
    let (absent, output, _) = run_fixture("assessment-absent");
    assert!(absent["record"].is_null());
    assert!(output["record"].get("assessment").is_none());
    assert_eq!(output["availability"]["recordKeys"]["assessment"]["state"], "not_applicable");
}

#[trace("FR-121-AC-4")]
#[test]
fn accepted_status_requires_provenance_in_the_schema() {
    let (valid_case, valid, root) = run_fixture("lifecycle-keys");
    let (invalid_case, invalid, _) = run_fixture("accepted-without-provenance");
    let (_, _, docs) = fixture_case("lifecycle-keys");
    let validator = compile_documents(&root, &docs).expect("fixture schema compiles");
    assert_eq!(valid["record"], valid_case["record"]);
    assert!(validator.is_valid(&valid["record"]));
    assert_eq!(invalid["record"], invalid_case["record"]);
    assert!(!validator.is_valid(&invalid["record"]));
    assert!(invalid["record"].get("provenance").is_none());
}

#[trace("FR-121-AC-5")]
#[test]
fn evidence_order_empty_table_and_frontmatter_status_use_shared_goldens() {
    let (case, output, _) = run_fixture("evidence-rows");
    assert_eq!(output["record"]["evidence"], case["record"]["evidence"]);
    let (_, empty, _) = run_fixture("evidence-header-only");
    assert_eq!(empty["record"]["evidence"], json!([]));
    assert_eq!(empty["availability"]["recordKeys"]["evidence"]["state"], "available");
    let (_, frontmatter, _) = run_fixture("frontmatter-status-not-body-reader");
    assert!(frontmatter["record"].get("status").is_some());
    assert!(!frontmatter["recordSources"].as_array().unwrap().iter().any(|source| source["key"] == "status"));
}

#[trace("FR-121-AC-6")]
#[test]
fn malformed_key_tables_report_their_specific_refusals() {
    for (id, reason) in [
        ("column-unknown", "column-unknown"),
        ("column-duplicate", "column-duplicate"),
        ("second-row", "second-row"),
        ("no-row", "no-row"),
        ("nested-object-cell", "nested-object-cell"),
        ("cell-type", "cell-type"),
    ] {
        let (_, output, _) = run_fixture(id);
        assert!(diagnostics(&output).iter().any(|(code, actual)| {
            code == "semantic.invalid-model-cell" && actual == reason
        }), "{id}: {:?}", diagnostics(&output));
        assert_eq!(output["availability"]["recordKeys"]["assessment"]["state"], "unavailable", "{id}");
        assert!(output["record"].get("assessment").is_none(), "{id}");
    }
    let (_, output, _) = run_fixture("second-section");
    assert!(diagnostics(&output).contains(&("semantic.duplicate-section".to_string(), "second-section".to_string())));
}

#[trace("FR-121-AC-7")]
#[test]
fn wrong_blocks_and_prose_only_sections_are_reported() {
    for id in ["list-under-table-key", "list-after-table"] {
        let (_, output, _) = run_fixture(id);
        assert!(diagnostics(&output).iter().any(|(code, reason)| {
            code == "semantic.feature-not-extractable" && reason == "assessment"
        }), "{id}: {:?}", diagnostics(&output));
    }
    let (_, output, _) = run_fixture("prose-only-section");
    assert!(diagnostics(&output).contains(&("semantic.record-key-no-block".to_string(), "no-block".to_string())));
    assert_eq!(output["availability"]["recordKeys"]["assessment"]["state"], "not_applicable");
}

#[trace("FR-121-AC-8")]
#[test]
fn token_gate_and_unowned_headings_are_prose() {
    let (_, no_token, _) = run_fixture("no-token");
    assert!(no_token.get("record").is_none() || no_token["record"].is_null());
    assert!(diagnostics(&no_token).is_empty());
    assert!(no_token.get("availability").and_then(|a| a.get("recordKeys")).is_none());
    let (_, prose, _) = run_fixture("prose-headings");
    assert!(diagnostics(&prose).is_empty());
}

#[trace("FR-121-AC-9")]
#[test]
fn normalized_camel_case_and_ambiguous_keys_follow_goldens() {
    let (case, output, _) = run_fixture("camel-case-heading");
    assert_eq!(output["record"], case["record"]);
    assert_eq!(output["record"]["retryPolicy"]["attempts"], 4);
    let (_, ambiguous, _) = run_fixture("key-ambiguous");
    assert!(diagnostics(&ambiguous).contains(&("semantic.feature-not-extractable".to_string(), "key-ambiguous".to_string())));
}

fn status_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://schemas.example.test/StatusRecord.json",
        "type": "object",
        "properties": {"status": {"type": "string"}, "label": {"type": "string"},
            "thresholds": {"type": "array", "items": {"type": "object"}}},
        "additionalProperties": false
    })
}

fn status_locator() -> Value {
    json!({"yield_pattern": {"match": {
        "status": {"from": "frontmatter_field", "path": ["status"], "required": false}
    }}})
}

#[trace("FR-121-AC-10")]
#[test]
fn section_cannot_supply_a_missing_frontmatter_owned_key() {
    let output = synthetic("# Item\n\n## Status\n\n`accepted`\n", status_schema(), &["record-sections"], Some(status_locator()));
    assert!(diagnostics(&output).contains(&("semantic.feature-not-extractable".to_string(), "frontmatter-owned".to_string())));
    assert!(output["record"].get("status").is_none());
    assert!(output["recordSources"].as_array().unwrap().iter().all(|source| source["key"] != "status"));
    assert!(output["availability"]["recordKeys"].get("status").is_none());
}

#[trace("FR-121-AC-11")]
#[test]
fn frontmatter_value_wins_over_a_duplicate_section_without_duplicate_key_error() {
    let output = synthetic("---\nstatus: accepted\n---\n# Item\n\n## Status\n\n`rejected`\n", status_schema(), &["record-sections"], Some(status_locator()));
    assert!(diagnostics(&output).contains(&("semantic.feature-not-extractable".to_string(), "frontmatter-owned".to_string())));
    assert!(!diagnostics(&output).iter().any(|(code, _)| code == "semantic.duplicate-record-key"));
    assert_eq!(output["record"]["status"], "accepted");
}

#[trace("FR-121-AC-12")]
#[test]
fn frontmatter_projection_is_retained_without_a_section() {
    let output = synthetic("---\nstatus: accepted\n---\n# Item\n", status_schema(), &["record-sections"], Some(status_locator()));
    assert_eq!(output["record"]["status"], "accepted");
    assert!(diagnostics(&output).is_empty());
    assert!(output["recordSources"].as_array().unwrap().iter().all(|source| source["key"] != "status"));
    assert!(output["availability"]["recordKeys"].get("status").is_none());
}

#[trace("FR-121-AC-13")]
#[test]
fn absent_optional_frontmatter_key_is_not_defaulted() {
    let output = synthetic("# Item\n", status_schema(), &["record-sections"], Some(status_locator()));
    assert!(output["record"].get("status").is_none());
    assert!(diagnostics(&output).is_empty());
    assert!(output["recordSources"].as_array().unwrap().iter().all(|source| source["key"] != "status"));
    assert!(output["availability"]["recordKeys"].get("status").is_none());
}

#[trace("FR-121-AC-14")]
#[test]
fn another_declared_table_mapping_owns_its_heading() {
    let extraction = json!({"yield_pattern": {"match": {
        "label": {"from": "frontmatter_field", "path": ["label"], "required": false},
        "thresholds": {"from": "table_row", "under_section": "Thresholds", "required": true,
            "assert": {"columns": ["Name"], "min_rows": 1}}
    }}});
    let output = synthetic(
        "---\nlabel: sample\n---\n# Item\n\n## Thresholds\n\n| Name |\n|---|\n| high |\n",
        status_schema(), &["record-sections"], Some(extraction));
    assert!(!diagnostics(&output).iter().any(|(_, reason)| reason == "thresholds"));
}

#[trace("FR-121-AC-15")]
#[test]
fn lifecycle_key_without_frontmatter_locator_still_reads_its_section() {
    let output = synthetic("# Item\n\n## Status\n\n`accepted`\n", status_schema(), &["record-sections"], Some(json!({"yield_pattern": {"match": {
        "label": {"from": "frontmatter_field", "path": ["label"], "required": false}
    }}})));
    assert_eq!(output["record"]["status"], "accepted");
    assert!(diagnostics(&output).is_empty());
}

#[trace("FR-121-AC-16")]
#[test]
fn frontmatter_locator_does_not_enable_section_diagnostics_without_token() {
    let output = synthetic("# Item\n\n## Status\n\n`accepted`\n", status_schema(), &[], Some(status_locator()));
    assert!(diagnostics(&output).is_empty());
    assert!(output.get("record").is_none() || output["record"].is_null());
}
