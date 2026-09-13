// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The vendored semantic contract, criterion by criterion (quoin#452).
//!
//! These restate what `tests/semantic-contract.test.ts` carried before the
//! cutover deleted it. The subject is the 35 JSON files under `src/semantic/`,
//! which are DATA and stay there: the TypeScript that read them is gone, this
//! crate reads them now, and what keeps the copies honest is that every hash
//! `SEMANTIC_CONTRACT` records is re-derived here from the live bytes.

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

use common::{fixtures_dir, semantic_core_dir, semantic_root};
use quoin_semantic::SEMANTIC_CONTRACT;
use quoin_semantic::contract::{
    common_schema_path, file_sha256, module_manifest_schema_path, package_manifest_schema_path,
    semantic_core_bundle_digest,
};
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

/// The module-manifest schema records filament-core-service provenance, the
/// vendored bytes hash to it, and it admits exactly the contract's keys.
///
/// Trace: FR-070-CON-2
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_640_the_module_manifest_schema_carries_its_provenance_and_the_bytes_match() {
    let record = SEMANTIC_CONTRACT.module_manifest_schema;
    assert_eq!(record.repository, "agent-ix/filament-core-service");
    assert_eq!(record.source_revision.len(), 40);
    assert!(
        record
            .source_revision
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "{}",
        record.source_revision
    );
    assert_eq!(
        record.source_path,
        "filament_core_service/schemas/module-manifest.schema.json"
    );
    let path = module_manifest_schema_path(&semantic_root());
    assert_eq!(file_sha256(&path).unwrap(), record.sha256);

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
/// The census has a floor: the pre-CR-003 document must itself carry required
/// arrays, or "every one of them is unchanged" is a statement about none of
/// them.
///
/// Trace: FR-070-CON-1, NFR-017-AC-4
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_641_the_vendored_schema_adds_no_required_key_versus_pre_cr003() {
    let mut before = BTreeMap::new();
    required_arrays(
        &read_json(
            &fixtures_dir()
                .join("vendored")
                .join("module-manifest.schema.pre-cr003.json"),
        ),
        "",
        &mut before,
    );
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
            path.contains("/properties/semantic") || path.contains("/data_schema/"),
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

/// The vendored semantic-core bundle's digest equals what filament-core-data
/// recorded in its own `toolchain.json`, its file list is the directory, and it
/// is a version this quoin declares it ships.
///
/// Trace: FR-073-AC-6
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_642_the_vendored_semantic_core_bundle_matches_its_recorded_digest() {
    let record = SEMANTIC_CONTRACT.semantic_core;
    assert_eq!(record.repository, "agent-ix/filament-core-data");
    assert_eq!(record.source_revision.len(), 40);
    let dir = semantic_core_dir();
    assert_eq!(
        semantic_core_bundle_digest(&dir).unwrap(),
        record.bundle_digest
    );

    let toolchain = read_json(&dir.join("toolchain.json"));
    assert_eq!(toolchain["digest"], record.bundle_digest);
    assert_eq!(
        toolchain["base"],
        format!(
            "https://schemas.agent-ix.org/semantic-core/{}/",
            record.version
        )
    );

    let mut shipped: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| {
            std::path::Path::new(name)
                .extension()
                .is_some_and(|ext| ext == "json")
                && name != "toolchain.json"
        })
        .collect();
    shipped.sort();
    let mut recorded: Vec<String> = toolchain["files"]
        .as_array()
        .expect("toolchain.json lists its files")
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    recorded.sort();
    assert_eq!(shipped, recorded);
    for name in [
        "FieldDecl.json",
        "TypeRef.json",
        "ClauseRef.json",
        "OperationDecl.json",
    ] {
        assert!(shipped.iter().any(|s| s == name), "{name} is not vendored");
    }
    assert!(
        SEMANTIC_CONTRACT
            .semantic_core_versions
            .contains(&record.version)
    );
}

/// The filament-core-data package-manifest and common schemas are vendored at
/// the hashes the contract records, and the common schema's target registry is
/// the declared five.
///
/// Trace: FR-075-AC-1
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_643_the_package_manifest_and_common_schemas_are_vendored_at_their_hashes() {
    assert_eq!(
        file_sha256(&package_manifest_schema_path(&semantic_root())).unwrap(),
        SEMANTIC_CONTRACT.package_manifest_schema.sha256
    );
    let common_path = common_schema_path(&semantic_root());
    assert_eq!(
        file_sha256(&common_path).unwrap(),
        SEMANTIC_CONTRACT.common_schema.sha256
    );
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
