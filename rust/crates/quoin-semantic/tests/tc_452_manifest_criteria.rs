// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-070 and FR-073, criterion by criterion (quoin#452).
//!
//! These restate the acceptance criteria `tests/semantic-manifest.test.ts`
//! carried before the cutover deleted it. They are NOT the golden-parity suite
//! next door: `golden_parity.rs` proves this crate answers what the TypeScript
//! answered over a captured corpus, which is a statement about the *port*. A
//! criterion is a statement about the *contract*, and it has to survive the
//! oracle's deletion — so each test below names one criterion and asserts the
//! rule it states, against the live vendored contract and the shipped
//! `module-ok` fixture.
//!
//! Every case is built the way the TypeScript built it: copy the fixture,
//! change one thing, read it back. See `common/mod.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use std::fs;

use common::{
    Scratch, codes, digest_of, errors, message_for, read, rewrite_entity_schema, semantic_root,
    validators,
};
use quoin_semantic::data_schema::DataSchemaForm;
use quoin_semantic::{CompatibilityPosture, LegacyForms, SEMANTIC_CONTRACT, read_semantic_block};
use serde_json::{Value, json};

/// A module with no `semantic` block is read clean, and reports no module.
///
/// The "unchanged" half of the criterion: the block is optional, so a manifest
/// without one must be neither refused nor decorated with advice.
///
/// Trace: FR-070-AC-1
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_610_a_module_without_a_semantic_block_reads_clean_and_unjudged() {
    let validators = validators();
    let scratch = Scratch::new();
    let root = scratch.module_copy("no-block", |manifest, _| {
        manifest.as_object_mut().unwrap().remove("semantic");
    });
    let result = read(&root, &validators);
    assert_eq!(codes(&root, &validators), Vec::<String>::new());
    assert!(result.module.is_none(), "no block means no semantic module");

    // The same, one level down: a manifest document with no `semantic` key at
    // all is nothing to read, not an empty block to validate.
    let bare = json!({ "name": "m", "version": "0.1.0" });
    let bare_result = read_semantic_block(&bare, scratch.path(), &validators);
    assert!(bare_result.diagnostics.is_empty());
    assert!(bare_result.module.is_none());
}

/// A minimal block is accepted and the optional keys take their documented
/// defaults.
///
/// The defaults are the criterion: a reader that forwarded the manifest
/// verbatim would answer with absent posture and absent legacy-form policy,
/// and every downstream rule keyed on them would silently not apply.
///
/// Trace: FR-070-AC-2
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_611_a_minimal_block_is_read_with_the_documented_defaults() {
    let validators = validators();
    let scratch = Scratch::new();
    let root = scratch.module_copy("minimal", |manifest, _| {
        manifest["semantic"] = json!({
            "contract_version": "1.0.0",
            "semantic_core": "0.3.0",
            "package": "agent-ix/spec-objects-fixture",
        });
    });
    assert_eq!(errors(&root, &validators), Vec::<String>::new());
    let block = read(&root, &validators).module.expect("a module").block;
    assert_eq!(block.contract_version.as_str(), "1.0.0");
    assert_eq!(block.semantic_core.as_str(), "0.3.0");
    assert_eq!(block.package.as_str(), "agent-ix/spec-objects-fixture");
    assert_eq!(block.compatibility_posture, CompatibilityPosture::Additive);
    assert_eq!(block.legacy_forms, LegacyForms::Warning);
    assert!(block.exports.is_empty());
    assert!(block.imports.is_empty());
    assert!(block.targets.is_empty());
    assert!(block.mappings.is_empty());
    assert_eq!(block.sweep_report, None);
}

/// An unadmitted key is refused BY NAME, and every admitted key is accepted.
///
/// Both halves, because either alone is satisfiable by a broken reader: a
/// reader that refused everything passes the first, and one that admitted
/// everything passes the second. The accepted set is `SEMANTIC_CONTRACT`'s own
/// key list, so a key added to the contract and not to the reader fails here.
///
/// Trace: FR-070-AC-3
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_612_an_unknown_key_is_refused_by_name_and_every_admitted_key_is_accepted() {
    let validators = validators();
    let scratch = Scratch::new();
    let bad = scratch.module_copy("unknown-key", |manifest, _| {
        manifest["semantic"]["foo"] = json!(1);
    });
    assert!(
        codes(&bad, &validators).contains(&"error:semantic.unknown-key@semantic.foo".to_owned()),
        "{:?}",
        codes(&bad, &validators)
    );

    let admitted = json!({
        "contract_version": "1.0.0",
        "semantic_core": "0.3.0",
        "package": "agent-ix/spec-objects-fixture",
        "exports": ["entity"],
        "imports": { "agent-ix/spec-artifacts-iso": "0.4.0" },
        "targets": ["json-schema", "markdown"],
        "mappings": ["typed-table"],
        "compatibility_posture": "strict",
        "legacy_forms": "warning",
        "sweep_report": "semantic/sweep.json",
    });
    // The population this case is judged over is the contract's own key list,
    // not a list retyped here: a key the contract admits and this object does
    // not would make the acceptance assertion below prove nothing about it.
    let mut declared: Vec<&str> = admitted
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    declared.sort_unstable();
    let mut contract: Vec<&str> = SEMANTIC_CONTRACT.semantic_keys.to_vec();
    contract.sort_unstable();
    assert_eq!(declared, contract, "every admitted key is exercised");

    let good = scratch.module_copy("all-keys", |manifest, _| {
        manifest["semantic"] = admitted.clone();
    });
    assert_eq!(errors(&good, &validators), Vec::<String>::new());
}

/// An export no `object_types` entry declares is refused, and so is an export
/// whose `data_schema` is not a `{ schema, digest }` reference.
///
/// Trace: FR-070-AC-4
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_613_an_export_must_name_a_declared_type_that_ships_a_referenced_schema() {
    let validators = validators();
    let scratch = Scratch::new();
    let undeclared = scratch.module_copy("bad-export", |manifest, _| {
        manifest["semantic"]["exports"] = json!(["endpoint"]);
    });
    assert!(
        codes(&undeclared, &validators)
            .contains(&"error:semantic.unknown-export@semantic.exports.endpoint".to_owned()),
        "{:?}",
        codes(&undeclared, &validators)
    );

    // `enumeration` is declared, but carries an INLINE `data_schema`, so there
    // is nothing to pin a digest to.
    let inline = scratch.module_copy("export-inline", |manifest, _| {
        manifest["semantic"]["exports"] = json!(["entity", "enumeration"]);
    });
    assert!(
        codes(&inline, &validators).contains(
            &"error:semantic.export-without-schema@semantic.exports.enumeration".to_owned()
        ),
        "{:?}",
        codes(&inline, &validators)
    );
    assert!(
        message_for(&inline, &validators, "semantic.export-without-schema").contains("enumeration"),
        "the refusal names the export"
    );
}

/// Artifact declarations are public semantic types too: the default process
/// module exports authored document types such as `SpecReview`, not only its
/// internal object types.
///
/// Trace: FR-070-AC-4
/// Provenance: quoin#537
#[test]
fn tc_537_001_an_export_may_name_an_artifact_type_with_a_pinned_schema() {
    let validators = validators();
    let scratch = Scratch::new();
    let artifact = scratch.module_copy("artifact-export", |manifest, root| {
        manifest["object_types"] = json!([]);
        manifest["artifact_types"] = json!([{
            "name": "SpecReview",
            "data_schema": {
                "schema": "schemas/Entity.json",
                "digest": digest_of(&root.join("schemas/Entity.json")),
            },
        }]);
        manifest["semantic"]["exports"] = json!(["SpecReview"]);
    });
    assert_eq!(errors(&artifact, &validators), Vec::<String>::new());
}

/// An unsupported `contract_version` is refused BEFORE any other key is read.
///
/// The "before" is the criterion and it is what the second defect in the same
/// block measures: the fixture also carries an unadmitted key, and a reader
/// that validated the whole block first would report two diagnostics. Exactly
/// one is the assertion.
///
/// Trace: FR-070-AC-5
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_614_an_unsupported_contract_version_is_refused_before_any_other_key() {
    let validators = validators();
    let scratch = Scratch::new();
    let root = scratch.module_copy("bad-version", |manifest, _| {
        manifest["semantic"]["contract_version"] = json!("2.0.0");
        manifest["semantic"]["foo"] = json!(1);
    });
    assert_eq!(
        codes(&root, &validators),
        vec!["error:semantic.unsupported-contract-version@semantic.contract_version".to_owned()]
    );
}

/// A target outside the declared registry is refused and named, and a package
/// that is not `<org>/<repo>` is refused.
///
/// Trace: FR-070-AC-7
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_615_an_unregistered_target_and_a_non_org_repo_package_are_refused() {
    let validators = validators();
    let scratch = Scratch::new();
    let target = scratch.module_copy("bad-target", |manifest, _| {
        manifest["semantic"]["targets"] = json!(["go"]);
    });
    assert!(
        codes(&target, &validators)
            .iter()
            .any(|c| c.starts_with("error:semantic.unknown-target@semantic.targets")),
        "{:?}",
        codes(&target, &validators)
    );
    assert!(
        message_for(&target, &validators, "semantic.unknown-target").contains("go"),
        "the refusal names the target it refused"
    );

    for (index, value) in ["ix://agent-ix/x", "https://example.org/pkg"]
        .into_iter()
        .enumerate()
    {
        let root = scratch.module_copy(&format!("bad-package-{index}"), |manifest, _| {
            manifest["semantic"]["package"] = json!(value);
        });
        assert!(
            codes(&root, &validators)
                .contains(&"error:semantic.invalid-package@semantic.package".to_owned()),
            "{value}: {:?}",
            codes(&root, &validators)
        );
    }
}

/// A reference-form `data_schema` resolves to the shipped file, against the
/// vendored semantic-core bundle at the version the manifest records.
///
/// Trace: FR-073-AC-1
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_616_a_reference_data_schema_resolves_against_the_vendored_bundle() {
    let validators = validators();
    let scratch = Scratch::new();
    let root = scratch.plain_copy("ref-ok");
    assert_eq!(errors(&root, &validators), Vec::<String>::new());

    let module = read(&root, &validators).module.expect("a module");
    let resolved = module
        .data_schemas
        .iter()
        .find(|(name, _)| name.as_str() == "entity")
        .map(|(_, r)| r)
        .expect("the entity export resolved");
    assert_eq!(resolved.kind, DataSchemaForm::Reference);
    let schema = resolved.schema.as_ref().expect("a resolved document");
    assert!(
        schema["$id"]
            .as_str()
            .unwrap()
            .ends_with("/spec-objects-fixture/0.1.0/Entity.json"),
        "{schema}"
    );
    // Resolution is against the VENDORED bundle: the document's own `$ref`s
    // name semantic-core at the version the block records, and that version is
    // one this quoin ships. A resolver that ignored `$ref`s entirely would
    // pass an assertion about `$id` alone.
    let refs = schema.to_string();
    assert!(
        refs.contains(&format!(
            "https://schemas.agent-ix.org/semantic-core/{}/FieldDecl.json",
            SEMANTIC_CONTRACT.semantic_core.version
        )),
        "{refs}"
    );
    assert!(SEMANTIC_CONTRACT.ships_semantic_core(&module.block.semantic_core));
}

/// A shipped schema file that does not hash, does not exist, is not JSON, or
/// is not a JSON Schema is refused, naming the path and the reason.
///
/// Four cases and not one: each is a distinct code, and a resolver that
/// collapsed them into one generic refusal would tell an author nothing about
/// which of the four happened.
///
/// Trace: FR-073-AC-2
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_617_an_unhashable_missing_unparsable_or_non_schema_file_is_refused() {
    let validators = validators();
    let scratch = Scratch::new();

    let mismatch = scratch.module_copy("mismatch", |_, root| {
        let file = root.join("schemas").join("Entity.json");
        let text = fs::read_to_string(&file).unwrap();
        fs::write(&file, text + "\n").unwrap();
    });
    assert!(
        codes(&mismatch, &validators).contains(
            &"error:semantic.data-schema-digest-mismatch@object_types[entity].data_schema.digest"
                .to_owned()
        ),
        "{:?}",
        codes(&mismatch, &validators)
    );
    let detail = message_for(
        &mismatch,
        &validators,
        "semantic.data-schema-digest-mismatch",
    );
    assert!(detail.contains("schemas/Entity.json"), "{detail}");
    assert!(detail.contains("sha256:"), "{detail}");

    let missing = scratch.module_copy("missing", |_, root| {
        fs::remove_file(root.join("schemas").join("Entity.json")).unwrap();
    });
    assert!(
        codes(&missing, &validators).contains(
            &"error:semantic.data-schema-missing@object_types[entity].data_schema.schema"
                .to_owned()
        ),
        "{:?}",
        codes(&missing, &validators)
    );

    let not_json = scratch.module_copy("not-json", |manifest, root| {
        let file = root.join("schemas").join("Entity.json");
        fs::write(&file, "{ nope").unwrap();
        manifest["object_types"][0]["data_schema"]["digest"] = Value::String(digest_of(&file));
    });
    assert!(
        codes(&not_json, &validators).contains(
            &"error:semantic.data-schema-not-json@object_types[entity].data_schema.schema"
                .to_owned()
        ),
        "{:?}",
        codes(&not_json, &validators)
    );

    let no_id = scratch.module_copy("no-id", |manifest, root| {
        let file = root.join("schemas").join("Entity.json");
        fs::write(&file, r#"{"type":"object"}"#).unwrap();
        manifest["object_types"][0]["data_schema"]["digest"] = Value::String(digest_of(&file));
    });
    assert!(
        codes(&no_id, &validators).contains(
            &"error:semantic.data-schema-not-schema@object_types[entity].data_schema.schema"
                .to_owned()
        ),
        "{:?}",
        codes(&no_id, &validators)
    );
}

/// A `$ref` to the document's own `$id` is a fragment, not a cycle; a `$ref`
/// to another semantic-core version, to a file nothing ships, or around a
/// cycle is refused.
///
/// The first half is the control. Treating a self-reference as a cycle would
/// refuse a correct schema, and a cycle detector that never fires would accept
/// an incorrect one; both halves have to hold at once.
///
/// Trace: FR-073-AC-3
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_618_a_self_reference_is_a_fragment_and_the_three_bad_refs_are_refused() {
    let validators = validators();
    let scratch = Scratch::new();

    let self_ref = scratch.module_copy("self-ref", |manifest, root| {
        rewrite_entity_schema(manifest, root, |schema| {
            let id = schema["$id"].as_str().unwrap().to_owned();
            schema["$defs"] = json!({ "marker": { "type": "string" } });
            schema["properties"]["marker"] = json!({ "$ref": format!("{id}#/$defs/marker") });
        });
    });
    assert_eq!(errors(&self_ref, &validators), Vec::<String>::new());

    let version = scratch.module_copy("core-version", |manifest, root| {
        rewrite_entity_schema(manifest, root, |schema| {
            schema["properties"]["fields"]["items"] = json!({
                "$ref": "https://schemas.agent-ix.org/semantic-core/0.2.0/FieldDecl.json"
            });
        });
    });
    assert!(
        codes(&version, &validators).contains(
            &"error:semantic.schema-ref-version@object_types[entity].data_schema.schema".to_owned()
        ),
        "{:?}",
        codes(&version, &validators)
    );

    let unshipped = scratch.module_copy("unshipped", |manifest, root| {
        rewrite_entity_schema(manifest, root, |schema| {
            schema["properties"]["fields"]["items"] = json!({
                "$ref": "https://schemas.agent-ix.org/agent-ix/spec-objects-fixture/0.1.0/Missing.json"
            });
        });
    });
    assert!(
        codes(&unshipped, &validators).contains(
            &"error:semantic.schema-ref-unshipped@object_types[entity].data_schema.schema"
                .to_owned()
        ),
        "{:?}",
        codes(&unshipped, &validators)
    );

    let cycle = scratch.module_copy("cycle", |manifest, root| {
        rewrite_entity_schema(manifest, root, |schema| {
            schema["properties"]["fields"]["items"] = json!({
                "$ref": "https://schemas.agent-ix.org/agent-ix/spec-objects-fixture/0.1.0/Other.json"
            });
        });
        fs::write(
            root.join("schemas").join("Other.json"),
            r#"{"$id":"https://schemas.agent-ix.org/agent-ix/spec-objects-fixture/0.1.0/Other.json","$ref":"https://schemas.agent-ix.org/agent-ix/spec-objects-fixture/0.1.0/Entity.json"}"#,
        )
        .unwrap();
    });
    assert!(
        codes(&cycle, &validators).contains(
            &"error:semantic.schema-ref-cycle@object_types[entity].data_schema.schema".to_owned()
        ),
        "{:?}",
        codes(&cycle, &validators)
    );
}

/// An inline `data_schema` under a `semantic` block is a WARNING; without a
/// block it is silent.
///
/// Trace: FR-073-AC-4
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_619_an_inline_data_schema_warns_only_under_a_semantic_block() {
    let validators = validators();
    let scratch = Scratch::new();
    let with_block = scratch.plain_copy("inline-warn");
    let warnings: Vec<String> = codes(&with_block, &validators)
        .into_iter()
        .filter(|c| c.starts_with("warning:"))
        .collect();
    assert_eq!(
        warnings,
        vec![
            "warning:semantic.inline-data-schema@object_types[enumeration].data_schema".to_owned()
        ]
    );

    let without = scratch.module_copy("inline-silent", |manifest, _| {
        manifest.as_object_mut().unwrap().remove("semantic");
    });
    assert_eq!(codes(&without, &validators), Vec::<String>::new());
}

/// A `data_schema.schema` that leaves the module root — by `..` or by symlink
/// — is refused, and a value mixing the reference and inline forms is refused
/// as ambiguous rather than silently preferring one.
///
/// Trace: FR-073-AC-5
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_620_a_path_escape_or_an_ambiguous_data_schema_is_refused() {
    let validators = validators();
    let scratch = Scratch::new();

    let dotdot = scratch.module_copy("dotdot", |manifest, _| {
        manifest["object_types"][0]["data_schema"]["schema"] = json!("../Entity.json");
    });
    assert!(
        codes(&dotdot, &validators).contains(
            &"error:semantic.data-schema-escape@object_types[entity].data_schema.schema".to_owned()
        ),
        "{:?}",
        codes(&dotdot, &validators)
    );

    // A symlink that resolves outside the module root. The bytes it points at
    // are a VALID schema, so only the escape rule can refuse this one.
    let outside = scratch.path().join("outside.json");
    fs::copy(
        common::fixture().join("schemas").join("Entity.json"),
        &outside,
    )
    .unwrap();
    let link = scratch.module_copy("symlink", |manifest, root| {
        let target = root.join("schemas").join("Linked.json");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &target).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&outside, &target).unwrap();
        manifest["object_types"][0]["data_schema"]["schema"] = json!("schemas/Linked.json");
    });
    assert!(
        codes(&link, &validators).contains(
            &"error:semantic.data-schema-escape@object_types[entity].data_schema.schema".to_owned()
        ),
        "{:?}",
        codes(&link, &validators)
    );

    let ambiguous = scratch.module_copy("ambiguous", |manifest, _| {
        manifest["object_types"][0]["data_schema"]["type"] = json!("object");
    });
    assert!(
        codes(&ambiguous, &validators).contains(
            &"error:semantic.data-schema-ambiguous@object_types[entity].data_schema".to_owned()
        ),
        "{:?}",
        codes(&ambiguous, &validators)
    );
}

/// Resolution reads no network, and a module resolves with none available.
///
/// A source guard, so it carries its own anti-vacuity floor: the file it reads
/// must be the resolution path and must be substantial, or "contains no HTTP
/// client" is a statement about an empty string. `include_str!` and not
/// `std::fs`, so the guard reads the bytes compiled into this binary rather
/// than whatever is on disk when it runs.
///
/// Trace: FR-073-CON-1
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_621_the_data_schema_resolution_path_names_no_network_client() {
    const SOURCE: &str = include_str!("../src/data_schema.rs");
    assert!(
        SOURCE.len() > 10_000,
        "the resolution path is {} bytes; this guard is reading the wrong file",
        SOURCE.len()
    );
    assert!(
        SOURCE.contains("pub fn resolve_data_schema"),
        "the guard must read the file that resolves references"
    );
    for forbidden in [
        "reqwest",
        "ureq",
        "hyper",
        "TcpStream",
        "std::net",
        "http://",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "the resolution path names {forbidden}"
        );
    }

    // And the behaviour the guard is about: a module resolves with no network.
    let validators = validators();
    let scratch = Scratch::new();
    let root = scratch.plain_copy("offline");
    assert_eq!(errors(&root, &validators), Vec::<String>::new());
    assert!(
        semantic_root().is_dir(),
        "resolution reads the vendored tree"
    );
}

/// The inline form stays valid and silent for a module with no `semantic`
/// block — the pre-contract modules keep working.
///
/// Trace: FR-073-CON-2
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_622_the_inline_form_stays_valid_for_a_module_with_no_semantic_block() {
    let validators = validators();
    let scratch = Scratch::new();
    let root = scratch.module_copy("legacy-inline", |manifest, _| {
        manifest.as_object_mut().unwrap().remove("semantic");
        manifest["object_types"][0]["data_schema"] = json!({ "type": "object" });
    });
    assert_eq!(codes(&root, &validators), Vec::<String>::new());
    assert!(read(&root, &validators).module.is_none());
}
