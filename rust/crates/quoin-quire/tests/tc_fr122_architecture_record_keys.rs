// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Quoin's FR-122 architecture-key contract against quire-fixtures goldens.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "Invalid shared golden fixtures are test failures"
)]

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quire_rs::semantic::python_entry::extract_semantic_json;
use quire_rs::semantic::{
    KeyOwner, SemanticContext, compile_documents, compile_module_schema, module_schema_documents,
};
use serde_json::{Value, json};

fn architecture_schemas_available() -> std::path::PathBuf {
    assert!(
        spec_objects_architecture_fixture::is_available(),
        "published architecture module unavailable: {:?}",
        spec_objects_architecture_fixture::unavailable_reason()
    );
    spec_objects_architecture_fixture::module_dir().join("schemas")
}

fn schema_at(schema_dir: &std::path::Path, name: &str) -> Value {
    let text = std::fs::read_to_string(schema_dir.join(name))
        .unwrap_or_else(|error| panic!("published architecture schema {name}: {error}"));
    serde_json::from_str(&text).unwrap_or_else(|error| {
        panic!("published architecture schema {name} is valid JSON: {error}")
    })
}

fn schema_documents(schema_dir: &std::path::Path, root: &Value) -> BTreeMap<String, Value> {
    let id = root["$id"].as_str().expect("schema id");
    let base = &id[..=id.rfind('/').expect("schema id path")];
    module_schema_documents(
        root,
        &|name| {
            let text = std::fs::read_to_string(schema_dir.join(name)).ok()?;
            Some(serde_json::from_str(&text).map_err(|error| error.to_string()))
        },
        base,
    )
    .expect("published architecture schema references resolve")
}

fn core_validator(name: &str) -> impl Fn(&Value) -> bool {
    let core = quire_rs::semantic::embedded::embedded_semantic_core_version();
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://schemas.agent-ix.org/agent-ix/quire-rs/0.1.0/Gate.json",
        "$ref": format!("https://schemas.agent-ix.org/semantic-core/{core}/{name}.json")
    });
    let validator = compile_module_schema(
        &schema,
        &|_| None,
        "https://schemas.agent-ix.org/agent-ix/quire-rs/",
    )
    .expect("semantic-core schema compiles");
    move |value| validator.is_valid(value)
}

fn case(id: &str) -> Value {
    quire_fixtures::record_sections_json("cases.json")["cases"]
        .as_array()
        .expect("shared cases array")
        .iter()
        .find(|case| case["id"] == id)
        .unwrap_or_else(|| panic!("quire-fixtures has no record-sections case {id}"))
        .clone()
}

fn line_of(markdown: &str, snippet: &str) -> usize {
    markdown
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(snippet))
        .map(|(index, _)| index + 1)
        .unwrap_or_else(|| panic!("fixture has no line containing {snippet:?}"))
}

fn extract(case: &Value) -> (Value, Value, BTreeMap<String, Value>) {
    let schema_dir = architecture_schemas_available();
    let root = schema_at(
        &schema_dir,
        &format!("{}.json", case["schema"].as_str().expect("schema name")),
    );
    let docs = schema_documents(&schema_dir, &root);
    let mut request = json!({
        "markdown": case["artifact"],
        "module": {
            "contractVersion": "1.0.0",
            "package": case["package"],
            "exports": [],
            "mappings": case.get("mappings").unwrap_or(&json!(["record-sections"])),
        },
        "path": "architecture.md",
        "sourceIdentity": "ix://acme/arch/spec",
        "bundle": case.get("bundle").unwrap_or(&json!({"package": "acme/arch"})),
        "dataSchema": root,
        "schemaDocuments": docs,
    });
    let request_map = request.as_object_mut().expect("request object");
    for key in [
        "relationVocabulary",
        "bodyExtraction",
        "constructReferences",
        "objectType",
    ] {
        if let Some(value) = case.get(key) {
            request_map.insert(key.to_string(), value.clone());
        }
    }
    let output = serde_json::to_value(
        extract_semantic_json(&request).expect("pinned Quire extracts published golden"),
    )
    .expect("semantic output serializes");
    (output, root, docs)
}

fn assert_diagnostic(case: &Value, output: &Value, code: &str, reason: &str, line_snippet: &str) {
    let markdown = case["artifact"].as_str().expect("fixture artifact");
    let line = line_of(markdown, line_snippet);
    let diagnostics = output["diagnostics"].as_array().expect("diagnostics array");
    assert_eq!(
        diagnostics.len(),
        1,
        "unexpected diagnostic set: {output:#}"
    );
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic["code"], code);
    assert_eq!(diagnostic["severity"], "error");
    assert_eq!(diagnostic["reason"], reason);
    assert_eq!(diagnostic["line"], line);
    assert!(diagnostic["section"].is_string());
}

fn assert_key_schema(case: &Value, output: &Value, schema_dir: &std::path::Path) {
    for (key, schema_name) in case["validates"].as_object().into_iter().flatten() {
        let mut schema_name = schema_name.as_str().expect("schema reference");
        let many = schema_name.ends_with("[]");
        if many {
            schema_name = schema_name.trim_end_matches("[]");
        }
        let is_valid: Box<dyn for<'a> Fn(&'a Value) -> bool> =
            if let Some(name) = schema_name.strip_prefix("core:") {
                Box::new(core_validator(name))
            } else {
                let schema = schema_at(schema_dir, &format!("{schema_name}.json"));
                let validator = compile_documents(&schema, &schema_documents(schema_dir, &schema))
                    .expect("published value schema compiles");
                Box::new(move |value: &Value| validator.is_valid(value))
            };
        let value = &output["record"][key];
        if many {
            let items = value.as_array().expect("array-valued schema key");
            assert!(!items.is_empty(), "{}: no items for {key}", case["id"]);
            for item in items {
                assert!(
                    is_valid(item),
                    "{}: {key} fails {schema_name}: {item:#}",
                    case["id"]
                );
            }
        } else {
            assert!(
                is_valid(value),
                "{}: {key} fails {schema_name}: {value:#}",
                case["id"]
            );
        }
    }
}

fn assert_golden(
    case: &Value,
    expected_record: Option<&Value>,
    expected_diag: Option<(&str, &str, &str)>,
) -> (Value, Value, BTreeMap<String, Value>) {
    let (output, root, docs) = extract(case);
    assert_eq!(
        output.get("record").unwrap_or(&Value::Null),
        expected_record.unwrap_or(&Value::Null),
        "{}: full record golden",
        case["id"]
    );
    match expected_diag {
        Some((code, reason, snippet)) => assert_diagnostic(case, &output, code, reason, snippet),
        None => assert!(
            output["diagnostics"]
                .as_array()
                .expect("diagnostics array")
                .is_empty(),
            "{}: unexpected diagnostics {:#}",
            case["id"],
            output["diagnostics"]
        ),
    }
    assert_key_schema(case, &output, &architecture_schemas_available());
    (output, root, docs)
}

#[trace("FR-122-AC-1")]
#[test]
fn provider_exposes_and_policy_tables_follow_shared_goldens() {
    for (id, key, value) in [
        ("provider", "provider", "ix://acme/arch/ext-001"),
        ("exposes", "exposes", "ix://acme/arch/pt-001"),
    ] {
        let fixture = case(id);
        let (output, _, _) = assert_golden(&fixture, Some(&fixture["record"]), None);
        assert_eq!(output["record"][key], value);
        assert!(core_validator("SemanticId")(&output["record"][key]));
    }

    for id in ["delivery", "versioning", "registration-stability"] {
        let fixture = case(id);
        let (output, _, _) = assert_golden(&fixture, Some(&fixture["record"]), None);
        assert_eq!(
            output["record"], fixture["record"],
            "{id}: optional empty cells omitted"
        );
    }

    let fixture = case("registration-bullet-list");
    let (output, _, _) = assert_golden(
        &fixture,
        None,
        Some((
            "semantic.feature-not-extractable",
            "registration",
            "- Mechanism",
        )),
    );
    assert!(output.get("record").is_none() || output["record"].is_null());
}

#[trace("FR-122-AC-2")]
#[test]
fn route_table_validates_and_unknown_operation_is_refused() {
    let fixture = case("routes");
    let (output, _, _) = assert_golden(&fixture, Some(&fixture["record"]), None);
    assert_eq!(output["record"]["routes"], fixture["record"]["routes"]);

    let fixture = case("route-operation-unknown");
    assert_golden(
        &fixture,
        None,
        Some((
            "semantic.invalid-model-cell",
            "route-operation-unknown",
            "| POST |",
        )),
    );
}

#[trace("FR-122-AC-3")]
#[test]
fn associated_types_come_from_the_table_not_the_contract_fence() {
    let fixture = case("associated-types");
    let (output, _, _) = assert_golden(&fixture, Some(&fixture["record"]), None);
    assert_eq!(
        output["record"]["associated_types"],
        fixture["record"]["associated_types"]
    );
    assert!(core_validator("TypeRef")(
        &output["record"]["associated_types"][0]
    ));

    let fixture = case("contract-fence-not-a-source");
    let (output, _, _) = assert_golden(&fixture, None, None);
    assert!(output.get("record").is_none() || output["record"].is_null());
}

#[trace("FR-122-AC-4")]
#[test]
fn layout_fence_maps_cross_record_size_and_refusals() {
    let fixture = case("layout-fence");
    let (output, _, _) = assert_golden(&fixture, Some(&fixture["record"]), None);
    let record = &output["record"];
    assert_eq!(record["endianness"], "little");
    assert_eq!(record["records"].as_array().expect("records").len(), 2);
    assert_eq!(record["records"][0]["name"], "metadata_block");
    assert_eq!(record["records"][1]["name"], "posting_tuple");
    assert_eq!(record["records"][0]["tag"], "0x21");
    assert_eq!(record["records"][1]["fields"][1]["size"], "payload_len");
    assert_eq!(record["records"][0]["fields"][0]["name"], "payload_len");
    assert!(record.get("magic").is_none());

    for (id, reason) in [
        ("layout-size-unknown-field", "size-unknown-field"),
        ("layout-size-ambiguous", "size-ambiguous"),
    ] {
        let fixture = case(id);
        assert_golden(
            &fixture,
            None,
            Some(("semantic.invalid-model-cell", reason, "```yaml")),
        );
    }

    let fixture = case("endianness-section-is-prose");
    let (output, _, _) = assert_golden(&fixture, None, None);
    assert!(output.get("record").is_none() || output["record"].is_null());
}

#[trace("FR-122-AC-5")]
#[test]
fn threshold_empty_optional_cells_and_required_status_follow_goldens() {
    let fixture = case("thresholds-row");
    let (output, _, _) = assert_golden(&fixture, Some(&fixture["record"]), None);
    let threshold = &output["record"]["thresholds"][0];
    assert_eq!(threshold["on_exceeded"]["status"], 429);
    assert_eq!(threshold["on_exceeded"]["retry_after"], true);
    assert!(threshold["on_exceeded"].get("behavior").is_none());

    let fixture = case("thresholds-status-required");
    assert_golden(
        &fixture,
        None,
        Some((
            "semantic.invalid-model-cell",
            "status-required",
            "| per_token |",
        )),
    );
}

#[trace("FR-122-AC-6")]
#[test]
fn requires_is_lowered_from_relationship_rows_and_bullets_are_prose() {
    let fixture = case("typed-relation-vs-requires-section");
    let (output, _, _) = assert_golden(&fixture, None, None);
    let artifact = fixture["artifact"].as_str().expect("artifact");
    assert!(artifact.contains("| needs | requires | pt-001 | 1..1 |"));
    assert!(artifact.contains("## Requires\n\n- ext-001"));
    let expected = &fixture["typedRelations"]["requires"];
    assert_eq!(&output["requires"], expected);
    let requires = output["requires"]
        .as_array()
        .expect("requires is SemanticId[]");
    assert_eq!(requires.len(), 1);
    let semantic_id = core_validator("SemanticId");
    assert!(requires.iter().all(semantic_id));
    assert!(
        output["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .is_empty()
    );
}

#[trace("FR-122-AC-7")]
#[test]
fn every_architecture_record_property_has_one_supported_owner() {
    use quire_rs::semantic::{RelationVocabulary, record_key_owners};

    let registry = quire_rs::Registry::load_module(spec_objects_architecture_fixture::module_dir())
        .expect("published architecture module loads");
    let module = registry
        .semantic_module("spec-objects-architecture")
        .expect("architecture semantic module exists")
        .clone();
    let mut audited = 0;
    let mut errors = Vec::new();

    for object_type in registry.active_archetypes() {
        let (Some(schema), Some(documents)) = (
            object_type.data_schema.as_deref(),
            object_type.data_schema_documents.as_deref(),
        ) else {
            continue;
        };
        let mut context = SemanticContext::new(
            module.clone(),
            "<FR-122 ownership census>",
            Default::default(),
        )
        .with_relation_vocabulary(RelationVocabulary::from_registry(&registry, object_type))
        .with_record_schema(schema, documents);
        if let Some(body_extraction) = object_type.body_extraction() {
            context = context.with_body_extraction(body_extraction);
        }
        let owners = record_key_owners(&context);
        let schema_property_count = schema["properties"]
            .as_object()
            .map_or(0, serde_json::Map::len);
        assert_eq!(
            owners.len(),
            schema_property_count,
            "{} property census size",
            object_type.name
        );
        let mut seen = std::collections::BTreeSet::new();
        for (property, owner) in owners {
            audited += 1;
            if !seen.insert(property.clone()) {
                errors.push(format!(
                    "{}.{} is listed more than once",
                    object_type.name, property
                ));
            }
            match owner {
                Some(
                    KeyOwner::Fields
                    | KeyOwner::Clauses
                    | KeyOwner::Operations
                    | KeyOwner::Relations
                    | KeyOwner::FeatureOrder
                    | KeyOwner::SystemsMember
                    | KeyOwner::TypedRelation
                    | KeyOwner::ModelTable
                    | KeyOwner::RecordSections,
                ) => {}
                Some(KeyOwner::Unmapped) => errors.push(format!(
                    "{}.{} is declared not-authored",
                    object_type.name, property
                )),
                Some(KeyOwner::Frontmatter) => errors.push(format!(
                    "{}.{} is owned by FR-069, outside the FR-122 owner set",
                    object_type.name, property
                )),
                None => errors.push(format!("{}.{} has no owner", object_type.name, property)),
            }
        }
    }

    assert!(
        audited > 0,
        "no architecture object record properties were inspected"
    );
    assert!(
        errors.is_empty(),
        "architecture record-key ownership errors: {errors:?}"
    );
}
