// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-075, criterion by criterion (quoin#452).
//!
//! These restate what `tests/semantic-package-manifest.test.ts` carried before
//! the cutover deleted it: the derived filament-core-data package manifest, the
//! per-export digests the registry pins, import resolution, and the `ix://`
//! identity shape. The install-path halves of the same criteria — the manifest
//! written beside an installed module, and the pin recorded in `registry.json`
//! — are restated in `quoin-modules`, which owns the install.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use std::fs;

use common::{Scratch, codes, digest_of, errors, read, semantic_root, validators};
use quoin_semantic::{
    SemanticModule, derive_package_manifest, export_digests, mapping_identity, registry_pin,
    resolve_imports, type_identity, validate_package_manifest,
};
use serde_json::json;

/// The module read out of one root, which must have produced one.
fn module(
    root: &std::path::Path,
    validators: &quoin_semantic::manifest::SemanticValidators,
) -> SemanticModule {
    read(root, validators)
        .module
        .expect("the fixture declares a valid semantic block")
}

/// The derived package manifest validates against the vendored
/// filament-core-data schema, and carries the module's own declarations.
///
/// Both halves: a derivation that emitted a valid but empty manifest would
/// satisfy the schema and describe nothing.
///
/// Trace: FR-075-AC-1
/// Provenance: agent-ix/quoin#452
#[test]
#[ignore = "PLAT-887: blocked on filament-core-data publishing schema/semantic/v1/module-manifest.schema.json"]
fn tc_452_630_the_derived_package_manifest_validates_and_carries_the_declarations() {
    let validators = validators();
    let scratch = Scratch::new();
    let root = scratch.plain_copy("derive");
    let derived = derive_package_manifest(&module(&root, &validators));
    let value = derived.to_value().unwrap();
    let verdict = validate_package_manifest(&semantic_root(), &value)
        .expect("the vendored package-manifest schema compiles");
    assert!(verdict.is_ok(), "{verdict:?}");

    assert_eq!(value["contractVersion"], "1.0.0");
    assert_eq!(
        value["package"],
        json!({ "identity": "agent-ix/spec-objects-fixture", "version": "0.1.0" })
    );
    assert_eq!(
        value["schemaDialect"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(value["sourceRoots"], json!(["schemas/"]));
    assert_eq!(value["targets"], json!(["json-schema", "markdown"]));
    assert_eq!(
        value["imports"],
        json!([{
            "packageIdentity": "agent-ix/semantic-core",
            "versionConstraint": "=0.1.0",
            "exports": [],
            "capabilities": [],
        }])
    );
    assert_eq!(
        value["exports"],
        json!([{
            "name": "entity",
            "typeIdentity": "ix://agent-ix/spec-objects-fixture/type/entity",
            "visibility": "public",
        }])
    );
    let profile = &value["profiles"][0];
    assert_eq!(profile["name"], "default");
    assert_eq!(profile["version"], "0.1.0");
    assert_eq!(profile["exports"], json!(["entity"]));
    assert_eq!(profile["mappings"], json!([]));
    assert_eq!(profile["compatibilityPosture"], "additive");
    assert_eq!(profile["options"], json!({}));
}

/// Declared mappings become `ix://` mapping identities, sorted, at the root
/// and in the profile alike.
///
/// Trace: FR-075-AC-1
/// Provenance: agent-ix/quoin#452
#[test]
#[ignore = "PLAT-887: blocked on filament-core-data publishing schema/semantic/v1/module-manifest.schema.json"]
fn tc_452_631_declared_mappings_become_sorted_identities_at_the_root_and_the_profile() {
    let validators = validators();
    let scratch = Scratch::new();
    // Declared out of order on purpose: the contract sorts them, and a
    // derivation that forwarded manifest order would pass a single-mapping
    // case.
    let root = scratch.module_copy("mapped", |manifest, _| {
        manifest["semantic"]["mappings"] = json!(["typed-table", "sysml-fence"]);
    });
    let derived = derive_package_manifest(&module(&root, &validators));
    let value = derived.to_value().unwrap();
    assert!(
        validate_package_manifest(&semantic_root(), &value)
            .unwrap()
            .is_ok()
    );
    let expected = json!([
        mapping_identity(&"agent-ix/spec-objects-fixture".into(), "sysml-fence"),
        "ix://agent-ix/spec-objects-fixture/mapping/typed-table",
    ]);
    assert_eq!(value["mappings"], expected);
    assert_eq!(value["profiles"][0]["mappings"], expected);
}

/// One digest per exported object type, over the shipped schema's bytes, and a
/// changed schema yields a changed digest.
///
/// The second half is the criterion's point: a pin that did not move when the
/// bytes moved would pin nothing.
///
/// Trace: FR-075-AC-2
/// Provenance: agent-ix/quoin#452
#[test]
#[ignore = "PLAT-887: blocked on filament-core-data publishing schema/semantic/v1/module-manifest.schema.json"]
fn tc_452_632_one_digest_per_export_moves_when_the_shipped_schema_moves() {
    let validators = validators();
    let scratch = Scratch::new();
    let root = scratch.plain_copy("pins");
    let pin = registry_pin(&module(&root, &validators)).unwrap();
    assert_eq!(pin.package, "agent-ix/spec-objects-fixture");
    assert_eq!(pin.semantic_core, "0.1.0");
    assert_eq!(pin.exports.keys().collect::<Vec<_>>(), vec!["entity"]);
    let digest = pin.exports["entity"].clone();
    assert!(
        digest.starts_with("sha256:") && digest.len() == 7 + 64,
        "{digest}"
    );
    assert_eq!(
        digest,
        digest_of(&root.join("schemas").join("Entity.json")),
        "the pin is over the shipped file's own bytes"
    );

    let changed = scratch.module_copy("pins-changed", |manifest, module_root| {
        common::rewrite_entity_schema(manifest, module_root, |schema| {
            schema["description"] = json!("changed");
        });
    });
    assert_eq!(errors(&changed, &validators), Vec::<String>::new());
    let moved = export_digests(&module(&changed, &validators)).unwrap();
    assert_ne!(moved["entity"], digest);
}

/// An import no installed module provides is refused, naming the versions that
/// ARE installed; an import cycle is refused, naming the cycle.
///
/// Trace: FR-075-AC-3
/// Provenance: agent-ix/quoin#452
#[test]
#[ignore = "PLAT-887: blocked on filament-core-data publishing schema/semantic/v1/module-manifest.schema.json"]
fn tc_452_633_an_unresolved_import_names_the_installed_versions_and_a_cycle_names_the_cycle() {
    let validators = validators();
    let scratch = Scratch::new();
    let needy_root = scratch.module_copy("needy", |manifest, _| {
        manifest["semantic"]["imports"] = json!({ "agent-ix/spec-objects-other": "0.2.0" });
    });
    let needy = module(&needy_root, &validators);

    let against_nothing = resolve_imports(&needy, &[]);
    let rendered: String = against_nothing
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        against_nothing
            .iter()
            .any(|d| d.code.as_str() == "semantic.import-unresolved"),
        "{rendered}"
    );
    assert!(
        rendered.contains("agent-ix/spec-objects-other@0.2.0"),
        "{rendered}"
    );
    assert!(rendered.contains("installed: none"), "{rendered}");

    // The same import, with the package installed at a DIFFERENT version: the
    // refusal stands and reports what is there.
    let provider_root = scratch.module_copy("provider", |manifest, module_root| {
        common::rewrite_entity_schema(manifest, module_root, |schema| {
            schema["$id"] =
                json!("https://schemas.agent-ix.org/agent-ix/spec-objects-other/0.1.0/Entity.json");
        });
        manifest["semantic"]["package"] = json!("agent-ix/spec-objects-other");
        manifest["version"] = json!("0.1.0");
    });
    assert_eq!(errors(&provider_root, &validators), Vec::<String>::new());
    let mut provider = module(&provider_root, &validators);
    let against_provider = resolve_imports(&needy, std::slice::from_ref(&provider));
    let rendered: String = against_provider
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("installed: 0.1.0"), "{rendered}");

    // Cycle: the provider imports the candidate, which imports the provider.
    provider
        .block
        .imports
        .insert("agent-ix/spec-objects-fixture".into(), "0.1.0".into());
    let cyclic_root = scratch.module_copy("cyclic", |manifest, _| {
        manifest["semantic"]["imports"] = json!({ "agent-ix/spec-objects-other": "0.1.0" });
    });
    let cyclic = module(&cyclic_root, &validators);
    let diagnostics = resolve_imports(&cyclic, std::slice::from_ref(&provider));
    let cycle = diagnostics
        .iter()
        .find(|d| d.code.as_str() == "semantic.import-cycle")
        .expect("the cycle is reported");
    assert!(
        cycle.message.contains(
            "agent-ix/spec-objects-fixture -> agent-ix/spec-objects-other -> \
             agent-ix/spec-objects-fixture"
        ),
        "{}",
        cycle.message
    );
}

/// The identity a consumer computes from the loaded module and the identity
/// the derived manifest exports are the same identity.
///
/// The criterion is the AGREEMENT. Either identity asserted alone is a
/// statement about one function's output; asserted against each other they
/// pin the two surfaces together, which is what a consumer joining on them
/// depends on.
///
/// Trace: FR-075-AC-4
/// Provenance: agent-ix/quoin#452
#[test]
#[ignore = "PLAT-887: blocked on filament-core-data publishing schema/semantic/v1/module-manifest.schema.json"]
fn tc_452_634_the_loaded_and_the_derived_type_identities_agree() {
    let validators = validators();
    let scratch = Scratch::new();
    let root = scratch.plain_copy("parity");
    let module = module(&root, &validators);

    let loaded: Vec<String> = module
        .block
        .exports
        .iter()
        .map(|name| type_identity(&module.block.package, name))
        .collect();
    let derived: Vec<String> = derive_package_manifest(&module)
        .exports
        .into_iter()
        .map(|export| export.type_identity)
        .collect();
    assert_eq!(
        loaded,
        vec!["ix://agent-ix/spec-objects-fixture/type/entity"]
    );
    assert_eq!(derived, loaded);
}

/// A URL or `ix://` package is refused, and every identity derived from an
/// admitted `<org>/<repo>` package is an `ix://` identity of the declared
/// shape.
///
/// Trace: FR-075-AC-5, FR-075-CON-2
/// Provenance: agent-ix/quoin#452
#[test]
#[ignore = "PLAT-887: blocked on filament-core-data publishing schema/semantic/v1/module-manifest.schema.json"]
fn tc_452_635_a_url_package_is_refused_and_admitted_packages_derive_ix_identities() {
    let validators = validators();
    let scratch = Scratch::new();
    for (index, value) in ["ix://agent-ix/x", "https://example.org/pkg"]
        .into_iter()
        .enumerate()
    {
        let root = scratch.module_copy(&format!("bad-pkg-{index}"), |manifest, _| {
            manifest["semantic"]["package"] = json!(value);
        });
        assert!(
            codes(&root, &validators)
                .contains(&"error:semantic.invalid-package@semantic.package".to_owned()),
            "{value}"
        );
    }

    assert_eq!(
        type_identity(&"agent-ix/spec-objects-business".into(), &"entity".into()),
        "ix://agent-ix/spec-objects-business/type/entity"
    );

    let root = scratch.module_copy("derived-ids", |manifest, _| {
        manifest["semantic"]["mappings"] = json!(["typed-table"]);
    });
    let text = serde_json::to_string(
        &derive_package_manifest(&module(&root, &validators))
            .to_value()
            .unwrap(),
    )
    .unwrap();
    let identities: Vec<&str> = text
        .split('"')
        .filter(|piece| piece.starts_with("ix://"))
        .collect();
    // A floor, so "every identity is well formed" is not a claim about zero
    // identities: one type and one mapping are declared.
    assert!(identities.len() >= 2, "{text}");
    for identity in identities {
        let parts: Vec<&str> = identity.trim_start_matches("ix://").split('/').collect();
        assert_eq!(parts.len(), 4, "{identity}");
        assert!(parts[2] == "type" || parts[2] == "mapping", "{identity}");
        assert!(parts.iter().all(|part| !part.is_empty()), "{identity}");
    }
}

/// The derivation compiles nothing, publishes nothing and fetches nothing.
///
/// A source guard with its own anti-vacuity floor: `include_str!` rather than
/// `std::fs`, and the file must be the derivation and must be substantial, or
/// the absence it asserts is an absence from an empty string.
///
/// Trace: FR-075-CON-1
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_636_the_derivation_names_no_network_publish_or_subprocess_path() {
    const SOURCE: &str = include_str!("../src/package_manifest.rs");
    assert!(
        SOURCE.len() > 10_000,
        "the derivation is {} bytes; this guard is reading the wrong file",
        SOURCE.len()
    );
    assert!(
        SOURCE.contains("pub fn derive_package_manifest"),
        "the guard must read the file that derives the manifest"
    );
    for forbidden in [
        "reqwest",
        "ureq",
        "std::net",
        "std::process",
        "Command::new",
        "npm publish",
        "http://",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "the derivation names {forbidden}"
        );
    }
    // The one write it does perform is a local file beside the module, and it
    // is named, so "writes nothing" is not overclaimed.
    assert!(SOURCE.contains("pub fn write_package_manifest"));
    let _ = fs::metadata(semantic_root()).expect("the vendored tree is on disk");
}
