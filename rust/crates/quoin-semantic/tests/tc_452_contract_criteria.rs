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

use std::collections::BTreeMap;
use std::fs;

use common::semantic_root;
use quoin_semantic::SEMANTIC_CONTRACT;
use quoin_semantic::contract::{common_schema_path, module_manifest_schema_path};
use serde_json::Value;

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

/// The module-manifest schema admits exactly the contract's keys.
///
/// Trace: FR-070-CON-2
/// Provenance: agent-ix/quoin#452
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

/// The vendored module-manifest schema adds no required key anywhere against
/// the pre-CR-003 schema, so a manifest that validated before still validates.
///
/// The census has a floor: the pre-CR-003 baseline must itself carry required
/// arrays, or "every one of them is unchanged" is a statement about none of
/// them.
///
/// The baseline is the 14 `(path, required)` pairs `required_arrays` produced
/// over filament-core-service's `module-manifest.schema.json` at the commit
/// before CR-003 (agent-ix/filament-core-service#21) added the `semantic`
/// block — recorded here as data rather than as a second copy of the whole
/// schema document beside the current one. The commit CR-003 landed against
/// is quoin's own git history, not this working tree; the point of this
/// constant is that nothing here needs to re-read it to keep asserting the
/// constraint.
///
/// Trace: FR-070-CON-1, NFR-017-AC-4
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_641_the_vendored_schema_adds_no_required_key_versus_pre_cr003() {
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
        ("/$defs/LintRuleEntry/properties/pattern", &["kind"]),
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
    assert!(
        before.len() >= 3,
        "the pre-CR-003 schema carries {} required arrays; the comparison would be vacuous",
        before.len()
    );

    for (path, required) in &before {
        assert_eq!(after.get(path), Some(required), "{path}");
    }
    for path in after.keys().filter(|path| !before.contains_key(*path)) {
        assert!(
            path.contains("/properties/semantic")
                || path.contains("/data_schema/")
                // `$defs/ConstructDeclaration` (filament-core-service#33): an optional
                // semantic-IR construct declaration a manifest may attach to an object
                // type via `ObjectTypeEntry.properties.construct`, which is itself
                // optional -- a manifest that never sets `construct` never has this
                // def's own `required` array evaluated, so it adds no obligation to a
                // manifest that validated before.
                || path.contains("/$defs/ConstructDeclaration"),
            "new required array outside the optional nodes: {path}"
        );
    }
    assert_eq!(
        after.get(""),
        Some(&vec![
            "manifest_version".to_owned(),
            "name".to_owned(),
            "version".to_owned()
        ])
    );
}

/// The common schema's target registry is the declared five.
///
/// Trace: FR-075-AC-1
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_643_the_common_schema_declares_the_five_targets() {
    let common_path = common_schema_path(&semantic_root());
    let common = read_json(&common_path);
    assert_eq!(
        common["$defs"]["target"]["enum"],
        serde_json::json!([
            "json-schema",
            "rust",
            "typescript",
            "python-pydantic-v2",
            "python-dataclass"
        ])
    );
}
