// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Quoin's record-key mapping exercised through the pinned Quire engine.
//! Golden markdown and expected outputs remain owned by quire-fixtures.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "Integration cases treat invalid pinned test fixtures as test failures"
)]

use std::collections::BTreeMap;
use std::path::Path;

use ix_trace_rs::trace;
use quire_rs::semantic::python_entry::extract_semantic_json;
use quire_rs::semantic::{compile_documents, module_schema_documents};
use serde_json::{Value, json};

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

fn safety_schemas_available() -> Option<std::path::PathBuf> {
    if quire_fixtures::safety_module_available() {
        Some(quire_fixtures::safety_schemas_dir())
    } else {
        eprintln!(
            "SKIPPED: published safety module unavailable: {}",
            quire_fixtures::safety_module_unavailable_reason().unwrap_or("unknown fetch failure")
        );
        None
    }
}

fn safety_schema(schema_dir: &Path, name: &str) -> Value {
    let text = std::fs::read_to_string(schema_dir.join(name))
        .unwrap_or_else(|error| panic!("authentic safety schema {name} is readable: {error}"));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("authentic safety schema {name} is valid JSON: {error}"))
}

fn safety_schema_documents(root: &Value, schema_dir: &Path) -> BTreeMap<String, Value> {
    let id = root["$id"].as_str().expect("authentic schema has an id");
    let base = &id[..=id.rfind('/').expect("authentic schema id has a path")];
    module_schema_documents(
        root,
        &|name| {
            let text = std::fs::read_to_string(schema_dir.join(name)).ok()?;
            Some(serde_json::from_str(&text).map_err(|error| error.to_string()))
        },
        base,
    )
    .expect("authentic safety schema references resolve from the complete fixture bundle")
}

fn extract_with_schema(
    markdown: &str,
    root: &Value,
    docs: &BTreeMap<String, Value>,
) -> (Value, Value) {
    let request = json!({
        "markdown": markdown,
        "module": {
            "contractVersion": "1.0.0",
            "package": "acme/safety",
            "exports": [],
            "mappings": ["record-sections"],
        },
        "path": "safety.md",
        "sourceIdentity": "ix://acme/safety/spec",
        "bundle": {"package": "acme/safety"},
        "dataSchema": root,
        "schemaDocuments": docs,
    });
    let extraction = extract_semantic_json(&request).expect("Quire extracts safety artifact");
    let output = serde_json::to_value(&extraction).expect("safety output serializes");
    let declaration = extraction
        .declaration_record()
        .expect("Quire composes the extracted declaration record");
    (output, declaration)
}

fn run_fixture(id: &str) -> (Value, Value, Value) {
    let (case, root, docs) = fixture_case(id);
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
        "bundle": case.get("bundle").unwrap_or(&json!({"package": "acme/safety"})),
        "dataSchema": root,
        "schemaDocuments": docs,
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
    let extraction = extract_semantic_json(&request).expect("Quire extracts fixture case");
    (
        case,
        serde_json::to_value(extraction).expect("semantic output serializes"),
        root,
    )
}

fn line_of(markdown: &str, needle: &str) -> usize {
    let (needle, occurrence) = needle
        .rsplit_once('@')
        .map_or((needle, 1), |(text, count)| {
            (text, count.parse().expect("line occurrence"))
        });
    markdown
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(needle))
        .nth(occurrence - 1)
        .map(|(index, _)| index)
        .expect("diagnostic snippet occurs in fixture")
        + 1
}

#[derive(Debug)]
struct TestDiagnostic {
    code: String,
    severity: String,
    reason: String,
    line: Option<usize>,
    section: Option<String>,
}

fn diagnostics(output: &Value) -> Vec<TestDiagnostic> {
    output["diagnostics"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|diagnostic| TestDiagnostic {
            code: diagnostic["code"].as_str().unwrap_or_default().to_string(),
            severity: diagnostic["severity"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            reason: diagnostic["reason"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            line: diagnostic["line"]
                .as_u64()
                .and_then(|line| usize::try_from(line).ok()),
            section: diagnostic["section"].as_str().map(str::to_string),
        })
        .collect()
}

fn has_diagnostic(output: &Value, code: &str, severity: &str, reason: &str, line: usize) -> bool {
    diagnostics(output).iter().any(|diagnostic| {
        diagnostic.code == code
            && diagnostic.severity == severity
            && diagnostic.reason == reason
            && diagnostic.line == Some(line)
            && diagnostic.section.is_some()
    })
}

fn synthetic_for(
    markdown: &str,
    schema: &Value,
    mappings: &[&str],
    body_extraction: Option<&Value>,
    package: &str,
    object_type: Option<&str>,
) -> Value {
    let mut request = json!({
        "markdown": markdown,
        "module": {
            "contractVersion": "1.0.0",
            "package": package,
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
        request["bodyExtraction"] = body_extraction.clone();
    }
    if let Some(object_type) = object_type {
        request["objectType"] = json!(object_type);
    }
    serde_json::to_value(extract_semantic_json(&request).expect("Quire extracts synthetic case"))
        .expect("semantic output serializes")
}

fn synthetic(
    markdown: &str,
    schema: &Value,
    mappings: &[&str],
    body_extraction: Option<&Value>,
) -> Value {
    synthetic_for(
        markdown,
        schema,
        mappings,
        body_extraction,
        "acme/synthetic",
        None,
    )
}

fn synthetic_with_relations(
    markdown: &str,
    schema: &Value,
    body_extraction: Option<&Value>,
    relation_vocabulary: &Value,
) -> Value {
    let mut request = json!({
        "markdown": markdown,
        "module": {
            "contractVersion": "1.0.0",
            "package": "acme/synthetic",
            "exports": [],
            "mappings": ["record-sections"],
        },
        "path": "synthetic.md",
        "sourceIdentity": "ix://acme/synthetic/spec",
        "bundle": {"package": "acme/synthetic"},
        "dataSchema": schema,
        "schemaDocuments": {},
        "relationVocabulary": relation_vocabulary,
    });
    if let Some(body_extraction) = body_extraction {
        request["bodyExtraction"] = body_extraction.clone();
    }
    serde_json::to_value(extract_semantic_json(&request).expect("Quire extracts owned-key case"))
        .expect("semantic output serializes")
}

#[trace("FR-121-AC-1")]
#[test]
fn assessment_section_extracts_a_record_validated_by_the_authentic_schema() {
    let Some(schema_dir) = safety_schemas_available() else {
        return;
    };
    let (case, _, _) = fixture_case("assessment-row");
    let markdown = case["artifact"]
        .as_str()
        .expect("shared assessment fixture markdown")
        .replace("| S3 |", "| catastrophic |");
    let hazard_schema = safety_schema(&schema_dir, "Hazard.json");
    let hazard_docs = safety_schema_documents(&hazard_schema, &schema_dir);
    let (output, _) = extract_with_schema(&markdown, &hazard_schema, &hazard_docs);
    assert_eq!(
        output["record"]["assessment"],
        json!({
            "severity": "catastrophic",
            "likelihood": "not_assessed",
            "controllability": "C2",
            "rationale": "Crush injury"
        })
    );
    assert_eq!(output["recordSources"][0]["key"], "assessment");
    let span = &output["recordSources"][0]["sourceSpan"];
    assert_eq!(span["startLine"], 8);
    assert_eq!(span["endLine"], 12);
    let assessment_schema = safety_schema(&schema_dir, "HazardAssessment.json");
    let assessment_docs = safety_schema_documents(&assessment_schema, &schema_dir);
    let validator = compile_documents(&assessment_schema, &assessment_docs)
        .expect("authentic HazardAssessment.json compiles");
    assert!(validator.is_valid(&output["record"]["assessment"]));
    let mut missing_rationale = output["record"]["assessment"].clone();
    missing_rationale
        .as_object_mut()
        .expect("assessment is an object")
        .remove("rationale");
    assert!(
        !validator.is_valid(&missing_rationale),
        "HazardAssessment requires rationale"
    );
}

#[trace("FR-121-AC-2")]
#[test]
fn analysis_section_extracts_a_record_validated_by_the_authentic_schema() {
    let Some(schema_dir) = safety_schemas_available() else {
        return;
    };
    let failure_mode_schema = safety_schema(&schema_dir, "FailureMode.json");
    let failure_mode_docs = safety_schema_documents(&failure_mode_schema, &schema_dir);
    let markdown = "# FM-1\n\n## Analysis\n\n| Effect | Cause | Detection |\n|---|---|---|\n| Injury | Seal failure | direct |\n";
    let (output, _) = extract_with_schema(markdown, &failure_mode_schema, &failure_mode_docs);
    let analysis_schema = safety_schema(&schema_dir, "FailureAnalysis.json");
    let analysis_docs = safety_schema_documents(&analysis_schema, &schema_dir);
    let validator = compile_documents(&analysis_schema, &analysis_docs)
        .expect("authentic FailureAnalysis.json compiles");
    assert_eq!(
        output["record"]["analysis"],
        json!({
            "effect": "Injury", "cause": "Seal failure", "detection": "direct"
        })
    );
    assert!(validator.is_valid(&output["record"]["analysis"]));
    let mut missing_detection = output["record"]["analysis"].clone();
    missing_detection
        .as_object_mut()
        .expect("failure analysis is an object")
        .remove("detection");
    assert!(
        !validator.is_valid(&missing_detection),
        "FailureAnalysis requires detection"
    );
    assert!(
        diagnostics(&output).is_empty(),
        "{:?}",
        diagnostics(&output)
    );
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
    assert_eq!(
        output["availability"]["recordKeys"]["assessment"]["state"],
        "not_applicable"
    );
}

#[trace("FR-121-AC-4")]
#[test]
fn accepted_hazard_with_properties_validates_against_the_authentic_schema() {
    let Some(schema_dir) = safety_schemas_available() else {
        return;
    };
    let hazard_schema = safety_schema(&schema_dir, "Hazard.json");
    let hazard_docs = safety_schema_documents(&hazard_schema, &schema_dir);
    let markdown = "---\nid: H-1\ntitle: Hazard one\nobject: entity\n---\n# H-1 Hazard one\n\n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|---|---|---|---|\n| hazard_id | String | 1 | identity |\n\n## Assessment\n\n| Severity | Likelihood | Exposure | Controllability | Rationale |\n|---|---|---|---|---|\n| catastrophic | probable | E2 | C2 | Crush injury |\n\n## Status\n\n`accepted`\n\n## Provenance\n\n| Asserted By | Asserted At |\n|---|---|\n| Alice | 2026-11-26T00:00:00Z |\n";
    let (output, declaration) = extract_with_schema(markdown, &hazard_schema, &hazard_docs);
    let validator =
        compile_documents(&hazard_schema, &hazard_docs).expect("authentic Hazard.json compiles");
    assert_eq!(
        output["fields"][0],
        json!({
            "name": "hazard_id",
            "type": {
                "target": {"kind": "KernelScalar", "value": "String"},
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false}
            },
            "identity": true
        })
    );
    assert_eq!(output["record"]["assessment"]["severity"], "catastrophic");
    assert_eq!(output["record"]["status"], "accepted");
    assert_eq!(output["record"]["provenance"]["assertedBy"], "Alice");
    assert!(validator.is_valid(&declaration), "{declaration:#}");

    let (without_provenance, provenance_section) = markdown
        .split_once("\n## Provenance\n")
        .expect("source markdown has a final Provenance section");
    assert!(provenance_section.contains("| Alice | 2026-11-26T00:00:00Z |"));
    let markdown_without_provenance = format!("{without_provenance}\n");
    let (without_provenance_output, without_provenance_declaration) =
        extract_with_schema(&markdown_without_provenance, &hazard_schema, &hazard_docs);
    assert_eq!(
        without_provenance_output["fields"], output["fields"],
        "Properties still extracts when Provenance is absent"
    );
    assert_eq!(
        without_provenance_output["record"]["status"], "accepted",
        "the accepted status remains extracted"
    );
    assert_eq!(
        without_provenance_output["record"]["assessment"], output["record"]["assessment"],
        "Assessment still extracts when Provenance is absent"
    );
    assert!(
        without_provenance_output["record"]
            .get("provenance")
            .is_none(),
        "no Provenance record is extracted without its section: {}",
        without_provenance_output["record"]
    );
    let provenance_errors = validator
        .iter_errors(&without_provenance_declaration)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(
        provenance_errors
            .iter()
            .any(|error| error.contains("provenance")),
        "accepted Hazard without source Provenance fails the authentic required-property schema: {provenance_errors:?}"
    );

    let mut without_provenance = declaration;
    without_provenance
        .as_object_mut()
        .expect("hazard declaration is an object")
        .remove("provenance");
    assert!(
        !validator.is_valid(&without_provenance),
        "accepted Hazard records require provenance"
    );
}

#[trace("FR-121-AC-5")]
#[test]
fn evidence_order_empty_table_and_frontmatter_status_use_shared_goldens() {
    let (case, output, _) = run_fixture("evidence-rows");
    assert_eq!(output["record"]["evidence"], case["record"]["evidence"]);
    let (_, empty, _) = run_fixture("evidence-header-only");
    assert_eq!(empty["record"]["evidence"], json!([]));
    assert_eq!(
        empty["availability"]["recordKeys"]["evidence"]["state"],
        "available"
    );
    let (_, frontmatter, _) = run_fixture("frontmatter-status-not-body-reader");
    assert!(frontmatter["record"].get("status").is_some());
    assert!(
        !frontmatter
            .get("recordSources")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .any(|source| source["key"] == "status")
    );
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
        let (case, output, _) = run_fixture(id);
        let diagnostic = &case["diagnostics"][0];
        let line = line_of(
            case["artifact"].as_str().expect("fixture artifact"),
            diagnostic["at"].as_str().expect("fixture locus"),
        );
        assert!(
            has_diagnostic(
                &output,
                "semantic.invalid-model-cell",
                diagnostic["severity"].as_str().expect("fixture severity"),
                reason,
                line,
            ),
            "{id}: {:?}",
            diagnostics(&output)
        );
        let key = if id == "nested-object-cell" || id == "cell-type" {
            "provenance"
        } else {
            "assessment"
        };
        assert_eq!(
            output["availability"]["recordKeys"][key]["state"], "unavailable",
            "{id}"
        );
        assert!(output["record"].get(key).is_none(), "{id}");
        let mut error_lines: Vec<usize> = case["diagnostics"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|diagnostic| diagnostic["severity"] == "error")
            .map(|diagnostic| {
                line_of(
                    case["artifact"].as_str().expect("fixture artifact"),
                    diagnostic["at"].as_str().expect("fixture locus"),
                )
            })
            .collect();
        error_lines.sort_unstable();
        error_lines.dedup();
        let expected_lines = error_lines
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        assert_eq!(
            output["availability"]["recordKeys"][key]["reason"],
            format!("entry-errors: lines {expected_lines}"),
            "{id}"
        );
    }
    let (case, output, _) = run_fixture("second-section");
    let diagnostic = &case["diagnostics"][0];
    let line = line_of(
        case["artifact"].as_str().expect("fixture artifact"),
        diagnostic["at"].as_str().expect("fixture locus"),
    );
    assert!(has_diagnostic(
        &output,
        "semantic.duplicate-section",
        diagnostic["severity"].as_str().expect("fixture severity"),
        "second-section",
        line,
    ));
}

#[trace("FR-121-AC-7")]
#[test]
fn wrong_blocks_and_prose_only_sections_are_reported() {
    for id in ["list-under-table-key", "list-after-table"] {
        let (case, output, _) = run_fixture(id);
        let diagnostic = &case["diagnostics"][0];
        assert!(
            has_diagnostic(
                &output,
                "semantic.feature-not-extractable",
                "error",
                "assessment",
                line_of(
                    case["artifact"].as_str().expect("fixture artifact"),
                    diagnostic["at"].as_str().expect("fixture locus"),
                ),
            ),
            "{id}: {:?}",
            diagnostics(&output)
        );
    }
    let (case, output, _) = run_fixture("prose-only-section");
    assert!(has_diagnostic(
        &output,
        "semantic.record-key-no-block",
        "warning",
        "no-block",
        line_of(
            case["artifact"].as_str().expect("fixture artifact"),
            "## Assessment",
        ),
    ));
    assert_eq!(
        output["availability"]["recordKeys"]["assessment"]["state"],
        "not_applicable"
    );
}

#[trace("FR-121-AC-8")]
#[test]
fn token_gate_and_unowned_headings_are_prose() {
    let (_, no_token, _) = run_fixture("no-token");
    assert!(no_token.get("record").is_none() || no_token["record"].is_null());
    assert!(diagnostics(&no_token).is_empty());
    assert!(
        no_token
            .get("availability")
            .and_then(|a| a.get("recordKeys"))
            .is_none()
    );
    let (_, prose, _) = run_fixture("prose-headings");
    assert!(diagnostics(&prose).is_empty());
    assert!(prose.get("record").is_none() || prose["record"].is_null());
    assert_eq!(record_source_keys(&prose), [] as [&str; 0]);
    for key in ["notes", "thresholds", "requires", "endianness"] {
        assert!(prose["record"].get(key).is_none(), "{key}");
        assert!(!record_source_keys(&prose).contains(&key), "{key}");
    }

    let schema = owned_relation_schema();
    let vocabulary = owned_relation_vocabulary();
    let headings = synthetic_with_relations(
        "# Artifact\n\n## Thresholds\n\n- target-1\n\n## Requires\n\n- target-1\n",
        &schema,
        None,
        &vocabulary,
    );
    assert!(diagnostics(&headings).is_empty());
    assert!(headings.get("record").is_none() || headings["record"].is_null());
    assert_eq!(record_source_keys(&headings), [] as [&str; 0]);

    let endian_schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://schemas.example.test/BinaryFormat.json",
        "type": "object",
        "properties": {"endianness": {"type": "string"}},
        "additionalProperties": false
    });
    let endian = synthetic_for(
        "# Artifact\n\n## Endianness\n\n`little`\n",
        &endian_schema,
        &["record-sections"],
        None,
        "agent-ix/spec-objects-architecture",
        Some("binary_format"),
    );
    assert!(diagnostics(&endian).is_empty());
    assert!(endian.get("record").is_none() || endian["record"].is_null());
    assert_eq!(record_source_keys(&endian), [] as [&str; 0]);
}

fn record_source_keys(output: &Value) -> Vec<&str> {
    output
        .get("recordSources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|source| source["key"].as_str())
        .collect()
}

#[trace("FR-121-AC-9")]
#[test]
fn normalized_camel_case_and_ambiguous_keys_follow_goldens() {
    let (case, output, _) = run_fixture("camel-case-heading");
    assert_eq!(output["record"], case["record"]);
    assert_eq!(output["record"]["retryPolicy"]["attempts"], 3);
    let (case, ambiguous, _) = run_fixture("key-ambiguous");
    assert!(has_diagnostic(
        &ambiguous,
        "semantic.feature-not-extractable",
        "error",
        "key-ambiguous",
        line_of(
            case["artifact"].as_str().expect("fixture artifact"),
            "## Retry Policy",
        ),
    ));
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

fn owned_relation_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://schemas.example.test/OwnedRecord.json",
        "type": "object",
        "properties": {
            "label": {"type": "string"},
            "thresholds": {"type": "array", "items": {
                "$ref": "https://schemas.agent-ix.org/semantic-core/0.4.0/SemanticId.json"
            }},
            "requires": {"type": "array", "items": {
                "$ref": "https://schemas.agent-ix.org/semantic-core/0.4.0/SemanticId.json"
            }}
        },
        "additionalProperties": false
    })
}

fn owned_relation_vocabulary() -> Value {
    json!({
        "objectType": "synthetic",
        "edgeTypes": {
            "thresholds": {"category": "dependency"},
            "requires": {"category": "dependency"}
        },
        "roles": {"synthetic": []},
        "allowedLinks": {"synthetic": {"thresholds": ["*"], "requires": ["*"]}}
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
    let output = synthetic(
        "# Item\n\n## Status\n\n`accepted`\n",
        &status_schema(),
        &["record-sections"],
        Some(&status_locator()),
    );
    assert!(has_diagnostic(
        &output,
        "semantic.feature-not-extractable",
        "error",
        "frontmatter-owned",
        3,
    ));
    assert!(output["record"].get("status").is_none());
    assert!(
        output
            .get("recordSources")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .all(|source| source["key"] != "status")
    );
    assert!(output["availability"]["recordKeys"].get("status").is_none());
}

#[trace("FR-121-AC-11")]
#[test]
fn frontmatter_value_wins_over_a_duplicate_section_without_duplicate_key_error() {
    let output = synthetic(
        "---\nstatus: accepted\n---\n# Item\n\n## Status\n\n`rejected`\n",
        &status_schema(),
        &["record-sections"],
        Some(&status_locator()),
    );
    assert!(has_diagnostic(
        &output,
        "semantic.feature-not-extractable",
        "warning",
        "frontmatter-owned",
        6,
    ));
    assert!(
        !diagnostics(&output)
            .iter()
            .any(|diagnostic| diagnostic.code == "semantic.duplicate-record-key")
    );
    assert_eq!(output["record"]["status"], "accepted");
}

#[trace("FR-121-AC-12")]
#[test]
fn frontmatter_projection_is_retained_without_a_section() {
    let output = synthetic(
        "---\nstatus: accepted\n---\n# Item\n",
        &status_schema(),
        &["record-sections"],
        Some(&status_locator()),
    );
    assert_eq!(output["record"]["status"], "accepted");
    assert!(
        diagnostics(&output).is_empty(),
        "{:?}",
        diagnostics(&output)
    );
    assert!(
        output
            .get("recordSources")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .all(|source| source["key"] != "status")
    );
    assert!(output["availability"]["recordKeys"].get("status").is_none());
}

#[trace("FR-121-AC-13")]
#[test]
fn absent_optional_frontmatter_key_is_not_defaulted() {
    let output = synthetic(
        "# Item\n",
        &status_schema(),
        &["record-sections"],
        Some(&status_locator()),
    );
    assert!(output["record"].get("status").is_none());
    assert!(diagnostics(&output).is_empty());
    assert!(
        output
            .get("recordSources")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .all(|source| source["key"] != "status")
    );
    assert!(output["availability"]["recordKeys"].get("status").is_none());
}

#[trace("FR-121-AC-14")]
#[test]
fn another_declared_table_mapping_owns_its_heading() {
    let schema = owned_relation_schema();
    let vocabulary = owned_relation_vocabulary();
    let extraction = json!({"yield_pattern": {"match": {
        "label": {"from": "frontmatter_field", "path": ["label"], "required": false}
    }}});
    let output = synthetic_with_relations(
        "---\nlabel: sample\n---\n# Item\n\n## Thresholds\n\n- target-1\n",
        &schema,
        Some(&extraction),
        &vocabulary,
    );
    assert!(
        diagnostics(&output).is_empty(),
        "{:?}",
        diagnostics(&output)
    );
    assert!(output["record"].get("thresholds").is_none());
    assert_eq!(record_source_keys(&output), [] as [&str; 0]);
}

#[trace("FR-121-AC-15")]
#[test]
fn lifecycle_key_without_frontmatter_locator_still_reads_its_section() {
    let output = synthetic(
        "# Item\n\n## Status\n\n`accepted`\n",
        &status_schema(),
        &["record-sections"],
        Some(&json!({"yield_pattern": {"match": {
            "label": {"from": "frontmatter_field", "path": ["label"], "required": false}
        }}})),
    );
    assert_eq!(output["record"]["status"], "accepted");
    assert!(diagnostics(&output).is_empty());
}

#[trace("FR-121-AC-16")]
#[test]
fn frontmatter_locator_does_not_enable_section_diagnostics_without_token() {
    let output = synthetic(
        "# Item\n\n## Status\n\n`accepted`\n",
        &status_schema(),
        &[],
        Some(&status_locator()),
    );
    assert!(diagnostics(&output).is_empty());
    assert!(output.get("record").is_none() || output["record"].is_null());
}
