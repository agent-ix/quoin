// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-070 `import_types`, `semantic.unknown-import-type` and
//! `semantic.unused-import`, FR-075 derived import exports and import-cycle
//! naming (PLAT-212).
//!
//! Every case is built the way the quoin#452 restatements are: copy the
//! `module-ok` fixture, change the manifest and the entity schema, read it back
//! and judge it against the providers the case installs.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use std::path::PathBuf;

use ix_trace_rs::trace;
use serde_json::{Value, json};

use common::{Scratch, codes, read, render, rewrite_entity_schema, semantic_root, validators};
use quoin_semantic::manifest::SemanticValidators;
use quoin_semantic::{
    SemanticModule, derive_package_manifest, resolve_imports, validate_package_manifest,
};

const FIXTURE: &str = "agent-ix/spec-objects-fixture";
const OTHER: &str = "agent-ix/spec-objects-other";

fn module(root: &std::path::Path, validators: &SemanticValidators) -> SemanticModule {
    read(root, validators)
        .module
        .unwrap_or_else(|| panic!("module did not read: {:?}", codes(root, validators)))
}

/// The provider `OTHER` at 0.1.0 exporting `entity`, with a schema of its own.
fn provider(scratch: &Scratch, imports: Value, refs: &[&str]) -> PathBuf {
    scratch.module_copy("other", |manifest, root| {
        manifest["semantic"]["package"] = json!(OTHER);
        manifest["semantic"]["imports"] = imports;
        rewrite_entity_schema(root, |schema| {
            schema["$id"] = json!(format!(
                "https://schemas.agent-ix.org/{OTHER}/0.1.0/Entity.json"
            ));
            add_refs(schema, refs);
        });
    })
}

/// A candidate importing `OTHER` at 0.1.0, whose entity schema carries `refs`.
fn candidate(
    scratch: &Scratch,
    import_types: Option<Value>,
    imports: Value,
    refs: &[&str],
) -> PathBuf {
    scratch.module_copy("cand", |manifest, root| {
        manifest["semantic"]["imports"] = imports;
        if let Some(types) = import_types {
            manifest["semantic"]["import_types"] = types;
        }
        rewrite_entity_schema(root, |schema| add_refs(schema, refs));
    })
}

fn add_refs(schema: &mut Value, refs: &[&str]) {
    for (i, reference) in refs.iter().enumerate() {
        schema["properties"][format!("ref{i}")] = json!({ "$ref": reference });
    }
}

fn other_ref(type_name: &str) -> String {
    format!("https://schemas.agent-ix.org/{OTHER}/{type_name}.json")
}

fn imports_other() -> Value {
    json!({ OTHER: "0.1.0" })
}

/// Diagnostics of `candidate` judged against `installed`, as `severity:code@path`.
fn judged(candidate: &SemanticModule, installed: &[SemanticModule]) -> Vec<String> {
    render(&resolve_imports(candidate, installed))
}

/// A manifest with imports and no `import_types` loads as before, and one with
/// `import_types` loads and reports the lists.
#[trace("FR-070-AC-8", "TC-2130")]
#[test]
fn tc_212_001_import_types_load_and_are_reported() {
    let validators = validators();
    let scratch = Scratch::new();
    let plain = scratch.module_copy("plain", |manifest, _| {
        manifest["semantic"]["imports"] = json!({ "agent-ix/spec-artifacts-iso": "0.2.0" });
    });
    assert!(
        common::errors(&plain, &validators).is_empty(),
        "{:?}",
        codes(&plain, &validators)
    );
    let block = read(&plain, &validators).module.unwrap().block;
    assert!(block.import_types.is_empty());
    assert!(
        serde_json::to_value(&block)
            .unwrap()
            .get("import_types")
            .is_none()
    );

    let typed = scratch.module_copy("typed", |manifest, _| {
        manifest["semantic"]["imports"] = json!({ "agent-ix/spec-artifacts-iso": "0.2.0" });
        manifest["semantic"]["import_types"] =
            json!({ "agent-ix/spec-artifacts-iso": ["StR", "FR"] });
    });
    assert!(
        common::errors(&typed, &validators).is_empty(),
        "{:?}",
        codes(&typed, &validators)
    );
    let block = read(&typed, &validators).module.unwrap().block;
    assert_eq!(
        serde_json::to_value(&block).unwrap()["import_types"],
        json!({ "agent-ix/spec-artifacts-iso": ["StR", "FR"] })
    );
}

/// Each malformed `import_types` is refused naming the package and the entry.
#[trace("FR-070-AC-9", "TC-2131")]
#[test]
fn tc_212_002_malformed_import_types_are_refused_naming_package_and_entry() {
    let validators = validators();
    let scratch = Scratch::new();
    let pkg = "agent-ix/spec-artifacts-iso";
    let cases: [(&str, Value, &str, &str); 7] = [
        (
            "absent",
            json!({ "agent-ix/elsewhere": ["A"] }),
            "semantic.import_types.agent-ix/elsewhere",
            "agent-ix/elsewhere",
        ),
        (
            "empty",
            json!({ pkg: [] }),
            "semantic.import_types.agent-ix/spec-artifacts-iso",
            pkg,
        ),
        (
            "duplicate",
            json!({ pkg: ["FR", "FR"] }),
            "semantic.import_types.agent-ix/spec-artifacts-iso",
            "FR",
        ),
        (
            "blank",
            json!({ pkg: [""] }),
            "semantic.import_types.agent-ix/spec-artifacts-iso",
            "empty type name",
        ),
        (
            "not-array",
            json!({ pkg: "FR" }),
            "semantic.import_types.agent-ix/spec-artifacts-iso",
            "must be an array",
        ),
        (
            "not-string",
            json!({ pkg: [7] }),
            "semantic.import_types.agent-ix/spec-artifacts-iso",
            "7",
        ),
        (
            "not-object",
            json!(["FR"]),
            "semantic.import_types",
            "semantic.import_types",
        ),
    ];
    for (name, value, locus, mention) in cases {
        let root = scratch.module_copy(name, |manifest, _| {
            manifest["semantic"]["imports"] = json!({ pkg: "0.2.0" });
            manifest["semantic"]["import_types"] = value.clone();
        });
        let result = read(&root, &validators);
        let found = result
            .diagnostics
            .iter()
            .find(|d| d.code.as_str() == "semantic.invalid-value" && d.path == locus)
            .unwrap_or_else(|| panic!("{name}: {:?}", codes(&root, &validators)));
        assert!(
            found.message.contains(mention)
                && (locus == "semantic.import_types"
                    || found.message.contains(locus_package(locus))),
            "{name}: {}",
            found.message
        );
        assert!(result.module.is_none(), "{name}");
    }
}

/// The package a case's reason must name: the locus's package.
fn locus_package(locus: &str) -> &str {
    locus
        .strip_prefix("semantic.import_types.")
        .unwrap_or(locus)
}

/// An `import_types` type the installed provider does not export fails install
/// with `semantic.unknown-import-type`; an exported one raises none.
#[trace("FR-070-AC-10", "TC-2132")]
#[test]
fn tc_212_003_an_unexported_import_type_is_refused() {
    let validators = validators();
    let scratch = Scratch::new();
    let provider = module(&provider(&scratch, json!({}), &[]), &validators);

    let bad = module(
        &candidate(
            &scratch,
            Some(json!({ OTHER: ["entity", "Nope"] })),
            imports_other(),
            &[&other_ref("entity")],
        ),
        &validators,
    );
    let found = resolve_imports(&bad, std::slice::from_ref(&provider));
    let unknown: Vec<_> = found
        .iter()
        .filter(|d| d.code.as_str() == "semantic.unknown-import-type")
        .collect();
    assert_eq!(unknown.len(), 1, "{found:?}");
    assert_eq!(unknown[0].path, format!("semantic.import_types.{OTHER}"));
    assert_eq!(unknown[0].severity.as_str(), "error");
    assert!(unknown[0].message.contains(OTHER) && unknown[0].message.contains("Nope"));

    let good = module(
        &candidate(
            &scratch,
            Some(json!({ OTHER: ["entity"] })),
            imports_other(),
            &[&other_ref("entity")],
        ),
        &validators,
    );
    assert_eq!(
        judged(&good, std::slice::from_ref(&provider)),
        Vec::<String>::new()
    );
}

/// With the provider absent, only the unresolved-import error is raised.
#[trace("FR-070-AC-11", "TC-2133")]
#[test]
fn tc_212_004_an_absent_provider_raises_only_the_unresolved_import() {
    let validators = validators();
    let scratch = Scratch::new();
    let cand = module(
        &candidate(
            &scratch,
            Some(json!({ OTHER: ["Nope"] })),
            imports_other(),
            &[&other_ref("Nope")],
        ),
        &validators,
    );
    let found = judged(&cand, &[]);
    assert_eq!(
        found,
        vec![format!(
            "error:semantic.import-unresolved@semantic.imports.{OTHER}"
        )]
    );
}

/// An unreferenced import warns, with and without a type list; a referenced one
/// does not; install-time severity is warning.
#[trace("FR-070-AC-12", "TC-2134")]
#[test]
fn tc_212_005_an_unreferenced_import_warns_and_a_referenced_one_does_not() {
    let validators = validators();
    let scratch = Scratch::new();
    let provider = module(&provider(&scratch, json!({}), &[]), &validators);
    let unused = format!("warning:semantic.unused-import@semantic.imports.{OTHER}");

    let no_list = module(
        &candidate(&scratch, None, imports_other(), &[]),
        &validators,
    );
    assert_eq!(
        judged(&no_list, std::slice::from_ref(&provider)),
        vec![unused.clone()]
    );

    let with_list = module(
        &candidate(
            &scratch,
            Some(json!({ OTHER: ["entity"] })),
            imports_other(),
            &[],
        ),
        &validators,
    );
    assert_eq!(
        judged(&with_list, std::slice::from_ref(&provider)),
        vec![unused.clone()]
    );

    // A `$ref` outside the imported base does not count as a reference.
    let elsewhere = module(
        &candidate(
            &scratch,
            None,
            imports_other(),
            &["https://schemas.agent-ix.org/agent-ix/elsewhere/entity.json"],
        ),
        &validators,
    );
    assert_eq!(
        judged(&elsewhere, std::slice::from_ref(&provider)),
        vec![unused]
    );

    let used = module(
        &candidate(&scratch, None, imports_other(), &[&other_ref("entity")]),
        &validators,
    );
    assert_eq!(
        judged(&used, std::slice::from_ref(&provider)),
        Vec::<String>::new()
    );
}

/// A `$ref` naming an unexported type is refused at the data-schema locus,
/// with and without an `import_types` list; an exported type raises none.
#[trace("FR-070-AC-13", "TC-2139")]
#[test]
fn tc_212_006_a_ref_to_an_unexported_type_is_refused_at_the_schema_locus() {
    let validators = validators();
    let scratch = Scratch::new();
    let provider = module(&provider(&scratch, json!({}), &[]), &validators);
    let locus = "object_types[entity].data_schema.schema";

    for list in [None, Some(json!({ OTHER: ["entity"] }))] {
        let cand = module(
            &candidate(&scratch, list, imports_other(), &[&other_ref("Nope")]),
            &validators,
        );
        let found = resolve_imports(&cand, std::slice::from_ref(&provider));
        let unknown: Vec<_> = found
            .iter()
            .filter(|d| d.code.as_str() == "semantic.unknown-import-type")
            .collect();
        assert_eq!(unknown.len(), 1, "{found:?}");
        assert_eq!(unknown[0].path, locus);
        assert!(unknown[0].message.contains(OTHER) && unknown[0].message.contains("Nope"));
    }

    let exported = module(
        &candidate(&scratch, None, imports_other(), &[&other_ref("entity")]),
        &validators,
    );
    assert_eq!(
        judged(&exported, std::slice::from_ref(&provider)),
        Vec::<String>::new()
    );
}

/// The derived package manifest's import entry carries the declared type list,
/// `[]` when none, and still validates.
#[trace("FR-075-AC-6", "TC-2136")]
#[test]
fn tc_212_007_derived_imports_carry_the_declared_type_list() {
    let validators = validators();
    let scratch = Scratch::new();
    let typed = module(
        &candidate(
            &scratch,
            Some(json!({ OTHER: ["StR", "FR"] })),
            imports_other(),
            &[],
        ),
        &validators,
    );
    let value = derive_package_manifest(&typed).to_value().unwrap();
    assert!(
        validate_package_manifest(&semantic_root(), &value)
            .unwrap()
            .is_ok()
    );
    assert_eq!(value["imports"][0]["exports"], json!(["StR", "FR"]));

    let untyped = module(
        &candidate(&scratch, None, imports_other(), &[]),
        &validators,
    );
    let value = derive_package_manifest(&untyped).to_value().unwrap();
    assert_eq!(value["imports"][0]["exports"], json!([]));
}

fn cycle_message(candidate: &SemanticModule, installed: &[SemanticModule]) -> String {
    resolve_imports(candidate, installed)
        .into_iter()
        .find(|d| d.code.as_str() == "semantic.import-cycle")
        .unwrap_or_else(|| panic!("no import cycle was reported"))
        .message
}

/// Two modules importing each other, with `$ref`s given to each side.
fn mutual(
    scratch: &Scratch,
    a_ref: Option<&str>,
    b_ref: Option<&str>,
) -> (SemanticModule, SemanticModule) {
    let validators = validators();
    let a_refs: Vec<String> = a_ref
        .map(|t| format!("https://schemas.agent-ix.org/{OTHER}/{t}.json"))
        .into_iter()
        .collect();
    let b_refs: Vec<String> = b_ref
        .map(|t| format!("https://schemas.agent-ix.org/{FIXTURE}/{t}.json"))
        .into_iter()
        .collect();
    let a_root = candidate(
        scratch,
        None,
        imports_other(),
        &a_refs.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let b_root = provider(
        scratch,
        json!({ FIXTURE: "0.1.0" }),
        &b_refs.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    (module(&a_root, &validators), module(&b_root, &validators))
}

/// A package cycle always fails install; it is named over type nodes only when
/// the `$ref`-derived type edges form a cycle themselves.
#[trace("FR-075-AC-7", "TC-2137")]
#[test]
fn tc_212_008_an_import_cycle_is_named_over_types_only_when_the_types_cycle() {
    let scratch = Scratch::new();

    // Type-level cycle: fixture#Entity -> other#Entity -> fixture#Entity.
    let (a, b) = mutual(&scratch, Some("Entity"), Some("Entity"));
    let message = cycle_message(&a, std::slice::from_ref(&b));
    assert!(
        message.contains(&format!(
            "{FIXTURE}#Entity -> {OTHER}#Entity -> {FIXTURE}#Entity"
        )),
        "{message}"
    );

    // No `$ref`s at all: named over packages.
    let scratch = Scratch::new();
    let (a, b) = mutual(&scratch, None, None);
    let message = cycle_message(&a, std::slice::from_ref(&b));
    assert!(
        message.contains(&format!("{FIXTURE} -> {OTHER} -> {FIXTURE}")),
        "{message}"
    );
    assert!(!message.contains('#'), "{message}");

    // Edges that form no type-level cycle: fixture#Entity -> other#Entity ->
    // fixture#Zed, and nothing leaves fixture#Zed.
    let scratch = Scratch::new();
    let (a, b) = mutual(&scratch, Some("Entity"), Some("Zed"));
    let message = cycle_message(&a, std::slice::from_ref(&b));
    assert!(!message.contains('#'), "{message}");

    // Only one side has `$ref`s into the cycle: still a package cycle.
    let scratch = Scratch::new();
    let (a, b) = mutual(&scratch, Some("Entity"), None);
    let message = cycle_message(&a, std::slice::from_ref(&b));
    assert!(!message.contains('#'), "{message}");
}
