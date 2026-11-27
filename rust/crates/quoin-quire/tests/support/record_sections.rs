// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Quoin FR-122/FR-123 record-section fixtures owned by quire-fixtures.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "invalid pinned fixtures are test failures"
)]

use std::collections::BTreeMap;

use quire_rs::semantic::python_entry::extract_semantic_json;
use quire_rs::semantic::{compile_documents, module_schema_documents};
use serde_json::{Value, json};

pub(crate) fn fixture_case(id: &str) -> (Value, Value, BTreeMap<String, Value>) {
    let cases = quire_fixtures::record_sections_json("cases.json");
    let case = cases["cases"]
        .as_array()
        .and_then(|cases| cases.iter().find(|case| case["id"] == id))
        .unwrap_or_else(|| panic!("quire-fixtures has no record-sections case {id}"))
        .clone();
    let name = schema_name_for_case(&case);
    let schema_dir = schema_dir_for_case(&case);
    let schema_text = std::fs::read_to_string(schema_dir.join(format!("{name}.json")))
        .expect("fixture schema is readable");
    let root: Value = serde_json::from_str(&schema_text).expect("fixture schema is valid JSON");
    let id = root["$id"].as_str().expect("schema id");
    let base = &id[..=id.rfind('/').expect("schema id path")];
    let documents = module_schema_documents(
        &root,
        &|file| {
            let text = std::fs::read_to_string(schema_dir.join(file)).ok()?;
            Some(serde_json::from_str(&text).map_err(|error| error.to_string()))
        },
        base,
    )
    .expect("fixture schema documents compile");
    (case, root, documents)
}

pub(crate) fn run_fixture(id: &str) -> (Value, Value) {
    let (case, output, _, _) = run_fixture_full(id);
    (case, output)
}

pub(crate) fn run_fixture_full(id: &str) -> (Value, Value, Value, BTreeMap<String, Value>) {
    let (case, root, documents) = fixture_case(id);
    let bundle = case
        .get("bundle")
        .cloned()
        .unwrap_or_else(|| json!({"package": "acme/safety"}));
    let package = bundle["package"].as_str().unwrap_or("acme/safety");
    let exported_type = case.get("objectType").and_then(Value::as_str).or_else(|| {
        case.get("relationVocabulary")
            .and_then(|vocabulary| vocabulary["objectType"].as_str())
    });
    let exports = exported_type.map_or_else(Vec::new, |object_type| vec![object_type]);
    let mut request = json!({
        "markdown": case["artifact"],
        "module": {
            "contractVersion": "1.0.0",
            "package": case.get("package").unwrap_or(&json!("acme/safety")),
            "exports": exports,
            "mappings": case.get("mappings").unwrap_or(&json!(["record-sections"])),
        },
        "path": "artifact.md",
        "sourceIdentity": format!("ix://{package}/spec"),
        "bundle": bundle,
        "dataSchema": root,
        "schemaDocuments": documents,
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
    let declaration = serde_json::to_value(
        extraction
            .declaration_record()
            .expect("the fixture composes a declaration record"),
    )
    .expect("declaration serializes");
    let output = serde_json::to_value(extraction).expect("extraction serializes");
    (case, output, declaration, documents)
}

pub(crate) fn assert_fixture_record(id: &str) -> (Value, Value) {
    let (case, output, declaration, documents) = run_fixture_full(id);
    assert_eq!(output["record"], case["record"], "{id}: extracted record");
    assert_eq!(
        record_source_keys(&output),
        case["recordSources"]
            .as_array()
            .expect("fixture source keys")
            .iter()
            .map(|key| key.as_str().expect("source key").to_owned())
            .collect::<Vec<_>>(),
        "{id}: source keys"
    );
    assert!(
        record_is_valid(&case, &declaration, &documents),
        "{id}: {declaration:#}"
    );
    (case, output)
}

pub(crate) fn schema_dir_for_case(case: &Value) -> std::path::PathBuf {
    let fixture_schemas = quire_fixtures::record_sections_dir().join("schemas");
    let fixture_name = case["schema"].as_str().expect("fixture schema name");
    if fixture_schemas
        .join(format!("{fixture_name}.json"))
        .is_file()
    {
        return fixture_schemas;
    }
    match case["package"].as_str() {
        Some(package) if package.ends_with("spec-objects-architecture") => {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/record-owner-modules/architecture/schemas")
        }
        Some(package) if package.ends_with("spec-objects-business") => {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/record-owner-modules/business/schemas")
        }
        _ => quire_fixtures::record_sections_dir().join("schemas"),
    }
}

pub(crate) fn schema_name_for_case(case: &Value) -> String {
    let name = case["schema"].as_str().expect("fixture schema name");
    if quire_fixtures::record_sections_dir()
        .join("schemas")
        .join(format!("{name}.json"))
        .is_file()
    {
        return name.to_owned();
    }
    match name {
        "Machine" => "StateMachine".to_owned(),
        "Workflow" => "Process".to_owned(),
        "Aggregate" => "AggregateRoot".to_owned(),
        "Nested" => "NestedEntity".to_owned(),
        name => name.to_owned(),
    }
}

pub(crate) fn record_is_valid(
    case: &Value,
    declaration: &Value,
    documents: &BTreeMap<String, Value>,
) -> bool {
    let schema_name = schema_name_for_case(case);
    let schema_dir = schema_dir_for_case(case);
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string(schema_dir.join(format!("{schema_name}.json")))
            .expect("schema is readable"),
    )
    .expect("schema is valid JSON");
    compile_documents(&schema, documents)
        .expect("schema compiles")
        .is_valid(declaration)
}

pub(crate) fn record_source_keys(output: &Value) -> Vec<String> {
    output["recordSources"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|source| source["key"].as_str().map(str::to_owned))
        .collect()
}

pub(crate) fn assert_error(id: &str, code: &str, reason: &str) {
    let (case, output) = run_fixture(id);
    let diagnostics = output["diagnostics"].as_array().expect("diagnostics array");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic["code"] == code
                && (reason.is_empty() || diagnostic["reason"] == reason)
                && diagnostic["severity"] == "error"
        }),
        "{id}: expected {code}/{reason}, got {diagnostics:#?}; fixture expected {:#?}",
        case["diagnostics"]
    );
}

pub(crate) fn assert_schema_ownership(
    module_dir: &str,
    module_name: &str,
    expected_objects: usize,
    expected_properties: usize,
    business: bool,
) {
    use quire_rs::semantic::{RelationVocabulary, SemanticContext, record_key_owners};

    let module_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/record-owner-modules")
        .join(module_dir);
    let registry = quire_rs::Registry::load_module(&module_path).expect("module snapshot loads");
    let module = registry
        .semantic_module(module_name)
        .expect("snapshot declares its semantic module")
        .clone();
    let schemas_dir = module_path.join("schemas");
    let mut object_count = 0usize;
    let mut inventory_property_count = 0usize;
    let mut record_object_count = 0usize;
    let mut record_property_count = 0usize;
    let mut unmapped = Vec::new();
    let mut unowned = Vec::new();
    let active_record_types = registry
        .active_archetypes()
        .filter(|archetype| archetype.data_schema.is_some())
        .map(|archetype| archetype.name.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let mut schema_files = std::fs::read_dir(&schemas_dir)
        .expect("schema snapshot is readable")
        .map(|entry| entry.expect("schema directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    schema_files.sort();
    for path in schema_files {
        let schema: Value = serde_json::from_str(
            &std::fs::read_to_string(&path).expect("schema snapshot file is readable"),
        )
        .expect("schema snapshot is valid JSON");
        if schema["type"] != "object" || !schema["properties"].is_object() {
            continue;
        }
        let schema_id = schema["$id"].as_str().expect("schema id");
        let base = &schema_id[..=schema_id.rfind('/').expect("schema id path")];
        let documents = module_schema_documents(
            &schema,
            &|name| {
                let text = std::fs::read_to_string(schemas_dir.join(name)).ok()?;
                Some(serde_json::from_str(&text).map_err(|error| error.to_string()))
            },
            base,
        )
        .expect("schema references resolve from the pinned module snapshot");
        let schema_name = path
            .file_stem()
            .and_then(std::ffi::OsStr::to_str)
            .expect("schema filename");
        let object_type = snake_case(schema_name);
        object_count += 1;
        let schema_properties = schema["properties"].as_object().expect("object properties");
        inventory_property_count += schema_properties.len();
        if !active_record_types.contains(&object_type) {
            continue;
        }
        record_object_count += 1;
        let archetype = registry
            .active_archetypes()
            .find(|archetype| archetype.name == object_type);
        let mut context =
            SemanticContext::new(module.clone(), "<schema-ownership>", Default::default())
                .with_record_schema(&schema, &documents)
                .with_object_type(object_type);
        if let Some(archetype) = archetype {
            context = context
                .with_relation_vocabulary(RelationVocabulary::from_registry(&registry, archetype));
            if let Some(body_extraction) = archetype.body_extraction() {
                context = context.with_body_extraction(body_extraction);
            }
        }
        let properties = record_key_owners(&context);
        assert_eq!(
            properties.len(),
            schema_properties.len(),
            "{}: owner entries",
            path.display()
        );
        record_property_count += properties.len();
        for (key, owner) in properties {
            let Some(owner) = owner else {
                unowned.push(format!("{}.{key}", path.display()));
                continue;
            };
            if owner == quire_rs::semantic::KeyOwner::Unmapped {
                let name = path
                    .file_stem()
                    .and_then(std::ffi::OsStr::to_str)
                    .expect("schema name");
                unmapped.push(format!("{}.{key}", snake_case(name)));
            }
        }
    }
    assert_eq!(object_count, expected_objects, "object schema inventory");
    assert_eq!(
        inventory_property_count, expected_properties,
        "all object schema properties inventoried"
    );
    assert_eq!(
        record_object_count,
        active_record_types.len(),
        "every active object-type record schema was audited"
    );
    assert!(record_property_count > 0, "record properties were audited");
    assert!(
        unowned.is_empty(),
        "properties without an owner: {unowned:?}"
    );
    if business {
        let allowed = [
            "domain.members",
            "domain.ubiquitous_language",
            "nested_entity.owner",
            "population.members",
        ];
        assert!(
            unmapped
                .iter()
                .all(|property| allowed.contains(&property.as_str())),
            "only FR-123's explicit exclusions may be unmapped: {unmapped:?}"
        );
        assert!(
            unmapped
                .iter()
                .any(|property| property == "population.members"),
            "the pinned business module exposes its explicit population.members exclusion"
        );
    } else {
        assert!(
            unmapped.is_empty(),
            "architecture has no not-authored properties: {unmapped:?}"
        );
    }
}

pub(crate) fn snake_case(value: &str) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len() + 4);
    for (index, character) in characters.iter().copied().enumerate() {
        let previous = index
            .checked_sub(1)
            .and_then(|i| characters.get(i))
            .copied();
        let next = characters.get(index + 1).copied();
        if character.is_ascii_uppercase()
            && previous
                .is_some_and(|previous| previous.is_ascii_lowercase() || previous.is_ascii_digit())
        {
            output.push('_');
        } else if character.is_ascii_uppercase()
            && previous.is_some_and(|previous| previous.is_ascii_uppercase())
            && next.is_some_and(|next| next.is_ascii_lowercase())
        {
            output.push('_');
        }
        output.push(character.to_ascii_lowercase());
    }
    output
}
