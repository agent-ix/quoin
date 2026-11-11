// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The vendored semantic contract, criterion by criterion (quoin#452).
//!
//! These restate what `tests/semantic-contract.test.ts` carried before the
//! cutover deleted it. The subject is the 35 JSON files under `src/semantic/`,
//! which are DATA and stay there: the TypeScript that read them is gone, this
//! crate reads them now.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use ix_trace_rs::trace;
use std::collections::BTreeMap;
use std::fs;

use common::semantic_root;
use quoin_semantic::SEMANTIC_CONTRACT;
use quoin_semantic::contract::{common_schema_path, module_manifest_schema_path};
use quoin_semantic::schema::{SchemaValidator, SchemaValidatorExt};
use serde_json::{Value, json};

/// Parse one JSON file.
fn read_json(path: &std::path::Path) -> Value {
    let display = path.display();
    serde_json::from_str(&fs::read_to_string(path).unwrap_or_else(|e| panic!("{display}: {e}")))
        .unwrap_or_else(|e| panic!("{display}: {e}"))
}

/// Every `required: [..]` array in a schema document, by JSON-pointer-ish
/// path — the same walk the deleted TypeScript performed, so the two answers
/// are comparable if this ever has to be re-derived.
fn required_arrays(node: &Value, path: &str, found: &mut BTreeMap<String, Vec<String>>) {
    match node {
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                required_arrays(child, &format!("{path}/{index}"), found);
            }
        }
        Value::Object(map) => {
            if let Some(Value::Array(required)) = map.get("required")
                && required.iter().all(Value::is_string)
            {
                found.insert(
                    path.to_owned(),
                    required
                        .iter()
                        .map(|v| v.as_str().unwrap().to_owned())
                        .collect(),
                );
            }
            for (key, child) in map {
                required_arrays(child, &format!("{path}/{key}"), found);
            }
        }
        _ => {}
    }
}

/// `$defs` entries added after CR-003, each the value type of an optional
/// top-level manifest property (`plain_language_profiles`, `lint_rules`,
/// `lexicon`, `ambiguity_terms`, `property_idioms`, `verification_catalog`,
/// `traceability` and its column, obligation and relation helpers). The
/// top-level `required` stays the original three keys, so a manifest that never
/// sets such a property never evaluates the entry's `required` array.
///
/// `LintRuleEntry` is accepted upstream evolution of semantic-schema 0.4.2: its
/// root `required` is `[type, id, section]` with conditional `column`/`allowed`
/// and `pattern` fields, replacing the pre-CR-003 object-form `pattern`. A new
/// `required` array under `ObjectTypeEntry`, `ArchetypeEntry` or
/// `ArtifactTypeEntry` is not on this list and still fails.
const NEW_OPTIONAL_DEFS: &[&str] = &[
    // An optional semantic-IR construct declaration
    // attached through the optional `ObjectTypeEntry.properties.construct`.
    "ConstructDeclaration",
    "PlainLanguageProfile",
    "LintRuleEntry",
    "VocabularyTerm",
    "AmbiguityTermEntry",
    "PropertyIdiomEntry",
    "VerificationMethodEntry",
    "TraceTarget",
    "DocumentReference",
    "StatusVocabulary",
    "TraceTagGrammar",
    "TraceMarkerForm",
    "ColumnVocabularies",
    "CombinatorialColumns",
    "ObligationSource",
    "RequiredRelation",
    "VocabularyCoverage",
    "TraceImplementsForm",
];

/// The module-manifest schema admits exactly the contract's keys.
///
/// Provenance: agent-ix/quoin#452
#[trace("FR-070-AC-3")]
#[test]
fn tc_452_640_the_module_manifest_schema_admits_the_contract_keys() {
    let path = module_manifest_schema_path(&semantic_root());
    let schema = read_json(&path);
    let semantic = &schema["properties"]["semantic"];
    assert_eq!(semantic["additionalProperties"], Value::Bool(false));
    let mut declared: Vec<&str> = semantic["properties"]
        .as_object()
        .expect("the semantic block declares properties")
        .keys()
        .map(String::as_str)
        .collect();
    declared.sort_unstable();
    let mut admitted: Vec<&str> = SEMANTIC_CONTRACT.semantic_keys.to_vec();
    admitted.sort_unstable();
    assert_eq!(declared, admitted);
}

/// The pinned module-manifest schema adds no required key anywhere against
/// the pre-CR-003 schema, so a manifest that validated before still validates.
///
/// The census has a floor: the pre-CR-003 baseline must itself carry required
/// arrays, or "every one of them is unchanged" is a statement about none of
/// them.
///
/// The baseline is the 13 `(path, required)` pairs `required_arrays` produced
/// over `module-manifest.schema.json` at the commit
/// before CR-003 added the `semantic`
/// block — recorded here as data rather than as a second copy of the whole
/// schema document beside the current one. The commit CR-003 landed against
/// is quoin's own git history, not this working tree; the point of this
/// constant is that nothing here needs to re-read it to keep asserting the
/// constraint.
///
/// Provenance: agent-ix/quoin#452
#[trace("FR-070-CON-1", "NFR-017-AC-4", "TC-1342", "TC-1382")]
#[test]
fn tc_452_641_the_pinned_schema_adds_no_required_key_versus_pre_cr003() {
    const PRE_CR003_REQUIRED_ARRAYS: &[(&str, &[&str])] = &[
        ("", &["manifest_version", "name", "version"]),
        ("/$defs/ArchetypeEntry", &["kind"]),
        (
            "/$defs/ArtifactTypeEntry",
            &["name", "grammar_ref", "frontmatter_schema_ref"],
        ),
        ("/$defs/BodyExtraction", &["yield_pattern"]),
        (
            "/$defs/BodyExtraction/properties/emit_edges/items",
            &["type", "target"],
        ),
        (
            "/$defs/BodyExtraction/properties/yield_pattern/properties/iterate_over",
            &["section_path", "kind"],
        ),
        ("/$defs/EdgeTypeEntry", &["description", "category"]),
        ("/$defs/GrammarEntry", &["name"]),
        ("/$defs/LocatorPrimitive", &["from"]),
        ("/$defs/ObjectTypeEntry", &["name"]),
        ("/$defs/RoleEntry", &["description"]),
        ("/properties/depends_on/items", &["name", "version_range"]),
        ("/properties/nav/properties/category", &["slug", "label"]),
    ];

    let before: BTreeMap<String, Vec<String>> = PRE_CR003_REQUIRED_ARRAYS
        .iter()
        .map(|(path, required)| {
            (
                (*path).to_owned(),
                required.iter().map(|s| (*s).to_owned()).collect(),
            )
        })
        .collect();
    let mut after = BTreeMap::new();
    required_arrays(
        &read_json(&module_manifest_schema_path(&semantic_root())),
        "",
        &mut after,
    );

    for (path, required) in &before {
        assert_eq!(after.get(path), Some(required), "{path}");
    }
    for path in after.keys().filter(|path| !before.contains_key(*path)) {
        assert!(
            path.contains("/properties/semantic")
                || path.contains("/data_schema/")
                || NEW_OPTIONAL_DEFS.iter().any(|name| {
                    *path == format!("/$defs/{name}")
                        || path.starts_with(&format!("/$defs/{name}/"))
                }),
            "new required array outside the optional nodes: {path}"
        );
    }
    // The pre-CR-003 object-form `pattern` baseline is now a plain string.
    let schema = read_json(&module_manifest_schema_path(&semantic_root()));
    assert_eq!(
        schema["$defs"]["LintRuleEntry"]["properties"]["pattern"]["type"],
        "string"
    );
    assert_eq!(
        after.get(""),
        Some(&vec![
            "manifest_version".to_owned(),
            "name".to_owned(),
            "version".to_owned()
        ])
    );
}

/// The common schema's target registry is the published six (semantic-schema
/// 0.4.x added `sysml-v2-textual`).
///
/// Provenance: agent-ix/quoin#452
#[trace("FR-075-AC-1")]
#[test]
fn tc_452_643_the_common_schema_declares_the_published_targets() {
    let common_path = common_schema_path(&semantic_root());
    let common = read_json(&common_path);
    assert_eq!(
        common["$defs"]["target"]["enum"],
        json!([
            "json-schema",
            "rust",
            "typescript",
            "python-pydantic-v2",
            "python-dataclass",
            "sysml-v2-textual"
        ])
    );
}

/// A manifest carrying all eight registry keys validates against the pinned
/// module-manifest schema, and an unknown root key is still refused, so the
/// pass is the schema admitting the keys rather than admitting everything.
#[trace("FR-070-AC-14")]
#[test]
fn tc_452_644_a_manifest_with_all_eight_registry_keys_validates() {
    let path = module_manifest_schema_path(&semantic_root());
    let validator = SchemaValidator::compile(&path, &read_json(&path), &[])
        .expect("the pinned module-manifest schema compiles");

    let term = json!({ "definition": "a documented term" });
    let manifest = json!({
        "manifest_version": 1,
        "name": "registry-keys",
        "version": "0.1.0",
        "lexicon": { "latency": term },
        "observable_verbs": { "emit": term },
        "ambiguity_terms": { "fast": term },
        "vacuous_predicates": { "works": term },
        "grammar_severity": { "ears:vague-response": "warning" },
        "property_idioms": {
            "round trip": { "definition": "decode inverts encode", "shape": "round-trip" }
        },
        "traceability": {
            "trace_targets": [
                { "name": "fr", "archetype": "FR", "section": "Requirements", "id_column": "ID" }
            ]
        },
        "verification_catalog": {
            "unit": { "name": "Unit test", "class": "test", "definition": "an executable test" }
        },
    });
    assert_eq!(
        validator.errors(&manifest),
        Vec::new(),
        "a manifest with the eight registry keys validates"
    );

    let mut unknown = manifest;
    unknown["not_a_registry"] = json!({});
    assert!(
        !validator.is_valid(&unknown),
        "an unknown root key is refused"
    );
}
