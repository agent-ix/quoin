// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Parity with the TypeScript oracle (quoin#381, FR-101).
//!
//! Expectations are read from `rust/goldens/ts-oracle.json`, captured once from
//! the TypeScript implementation and committed. See `rust/goldens/PROVENANCE.md`
//! for the revision and the capture command. Nothing here runs Node.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::fs;

use quoin_modules::manifest::MarketplaceManifest;
use quoin_modules::registry::{InstalledModule, ModuleRegistry};
use quoin_modules::{IxHome, Source, parse_source_arg, read_module_name, to_git_url};
use serde_json::Value as Json;

const GOLDENS: &str = include_str!("../../../goldens/ts-oracle.json");
const DEFAULT_MODULES: &str = include_str!("../../../goldens/default-modules.yaml");

fn section(name: &str) -> Json {
    let all: Json = serde_json::from_str(GOLDENS).expect("committed goldens parse");
    all.get(name)
        .unwrap_or_else(|| panic!("goldens carry a `{name}` section"))
        .clone()
}

fn cases(name: &str) -> Vec<Json> {
    section(name)
        .as_array()
        .unwrap_or_else(|| panic!("`{name}` is an array"))
        .clone()
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_230_parse_source_arg_matches_the_typescript_oracle() {
    let cases = cases("parse_source_arg");
    assert!(cases.len() >= 12, "golden set shrank: {}", cases.len());
    for case in cases {
        let arg = case["arg"].as_str().expect("arg");
        let parsed = parse_source_arg(arg).unwrap_or_else(|e| panic!("{arg}: {e}"));
        let actual = serde_json::to_value(&parsed).expect("source serializes");
        // The oracle writes absent optionals as an explicit null; this port
        // omits them. Compare on the keys the oracle actually set.
        let expected = case["source"].as_object().expect("source object");
        for (key, value) in expected {
            if value.is_null() {
                assert!(
                    actual.get(key).is_none(),
                    "{arg}: {key} should be absent, got {actual}"
                );
            } else {
                assert_eq!(actual.get(key), Some(value), "{arg}: key {key}");
            }
        }
        assert_eq!(
            actual.as_object().expect("object").len(),
            expected.values().filter(|v| !v.is_null()).count(),
            "{arg}: no extra keys — got {actual}"
        );
    }
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_231_to_git_url_matches_the_typescript_oracle() {
    let cases = cases("to_git_url");
    assert!(cases.len() >= 6, "golden set shrank: {}", cases.len());
    for case in cases {
        let raw = case["raw"].as_str().expect("raw");
        assert_eq!(
            to_git_url(raw),
            case["url"].as_str().expect("url"),
            "raw {raw:?}"
        );
    }
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_232_source_validation_accepts_and_refuses_what_the_oracle_did() {
    let cases = cases("normalize_source");
    assert!(cases.len() >= 12, "golden set shrank: {}", cases.len());
    for case in cases {
        let input = &case["source"];
        let expected_ok = case["result"]["ok"].as_bool().expect("ok flag");
        let actual_ok =
            serde_json::from_value::<Source>(input.clone()).is_ok_and(|s| s.validate().is_ok());
        assert_eq!(actual_ok, expected_ok, "source {input}");
    }
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_233_the_committed_default_module_set_parses_to_the_same_entries() {
    let manifest = MarketplaceManifest::from_yaml(DEFAULT_MODULES).expect("default set is valid");
    let expected = section("default_modules_manifest");
    let expected = expected["value"].as_object().expect("manifest value");
    assert_eq!(
        manifest.name.as_deref(),
        expected["name"].as_str(),
        "manifest name"
    );
    let expected_entries = expected["entries"].as_array().expect("entries");
    assert_eq!(
        manifest.entries.len(),
        expected_entries.len(),
        "entry count"
    );
    for (actual, expected) in manifest.entries.iter().zip(expected_entries) {
        assert_eq!(
            actual.name.as_str(),
            expected["name"].as_str().expect("name")
        );
        assert_eq!(actual.version.as_deref(), expected["version"].as_str());
        assert_eq!(
            actual.is_enabled(),
            expected["defaultEnabled"].as_bool().unwrap_or(true)
        );
        let actual_source = serde_json::to_value(&actual.source).expect("source serializes");
        assert_eq!(
            actual_source, expected["source"],
            "entry {} source",
            actual.name
        );
    }
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_234_manifest_refusals_match_the_oracle() {
    let cases = cases("validate_manifest");
    assert!(cases.len() >= 8, "golden set shrank: {}", cases.len());
    for case in cases {
        let input = &case["input"];
        let expected_ok = case["result"]["ok"].as_bool().expect("ok flag");
        // JSON text, not `serde_yaml_ng::to_string(input)`. YAML 1.2 is a
        // superset of JSON, so the golden's own bytes are already legal input,
        // and feeding them directly is the more faithful oracle. Re-serialising
        // a `serde_json::Value` through a NON-serde_json serializer is not
        // faithful: under `serde_json/arbitrary_precision` every number leaves
        // the serde data model as `{"$serde_json::private::Number": "7"}`, and
        // `from_yaml` then correctly refuses a manifest that is in fact legal.
        // That feature is global and unifying — any crate in the workspace
        // turns it on for every other crate — so this bridge is only ever one
        // dependency edge away from breaking. See quoin#440.
        let yaml = serde_json::to_string(input).expect("golden serialises");
        let actual_ok = MarketplaceManifest::from_yaml(&yaml).is_ok();
        assert_eq!(actual_ok, expected_ok, "manifest {input}");
    }
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_235_registry_bytes_match_the_typescript_writer() {
    // The registry file is shared with the retained TypeScript during staged
    // coexistence, so its bytes are a contract, not formatting.
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("registry.json");
    let mut registry = ModuleRegistry::new();
    registry.upsert(InstalledModule {
        name: quoin_modules::ModuleName::new("m").expect("legal name"),
        source: Source::Path {
            path: "/p".to_owned(),
        },
        r#ref: None,
        sha: None,
        resolved_path: "/p".to_owned(),
        target_path: "/t/m".to_owned(),
        installed_at: "2026-01-02T03:04:05.678Z".to_owned(),
        semantic: None,
    });
    registry.write(&path).expect("write");

    let expected = section("registry_file_bytes");
    assert_eq!(
        fs::read_to_string(&path).expect("read back"),
        expected.as_str().expect("golden is a string")
    );
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_239_registry_bytes_match_with_every_optional_field_present() {
    // `tc_381_235` writes a record whose every optional field is absent, so it
    // pins nothing about where `ref`, `sha` and `semantic` go or whether they
    // appear at all — exactly the keys a serde attribute could reorder or drop
    // without any golden noticing. This is that record with all of them set.
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("registry.json");
    let sha = "978111c6884ccc0f5e6ca100fb17be5361a98713";
    let mut registry = ModuleRegistry::new();
    registry.upsert(InstalledModule {
        name: quoin_modules::ModuleName::new("m").expect("legal name"),
        source: Source::GitSubdir {
            url: "https://github.com/acme/widgets.git".to_owned(),
            path: "modules/m".to_owned(),
            r#ref: Some("v1.2.3".to_owned()),
            sha: Some(sha.to_owned()),
        },
        r#ref: Some("v1.2.3".to_owned()),
        sha: Some(quoin_modules::CommitSha::new(sha).expect("full sha")),
        resolved_path: "/c/978111c/modules/m".to_owned(),
        target_path: "/t/m".to_owned(),
        installed_at: "2026-01-02T03:04:05.678Z".to_owned(),
        semantic: Some(quoin_modules::SemanticPin {
            package: "acme.widgets".to_owned(),
            semantic_core: "quire-core@1".to_owned(),
            exports: [
                ("Gadget".to_owned(), "sha256:bb".to_owned()),
                ("Widget".to_owned(), "sha256:aa".to_owned()),
            ]
            .into_iter()
            .collect(),
        }),
    });
    registry.write(&path).expect("write");

    let expected = section("registry_file_bytes_full");
    assert_eq!(
        fs::read_to_string(&path).expect("read back"),
        expected.as_str().expect("golden is a string")
    );
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_236_registry_reads_match_the_oracles_tolerances() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("registry.json");

    // Absent -> empty, matching `registry_read_absent`.
    assert_eq!(
        ModuleRegistry::read(&path).expect("absent is fine"),
        ModuleRegistry::new()
    );

    // `plugins` not an array -> empty, matching `registry_read_plugins_not_array`.
    fs::write(&path, r#"{"schemaVersion":1,"plugins":"nope"}"#).expect("write");
    assert_eq!(
        ModuleRegistry::read(&path).expect("tolerated"),
        ModuleRegistry::new()
    );

    // Malformed JSON: the oracle throws, and so does this — deliberately, since
    // silently forgetting every entry would lose the rollback snapshot.
    fs::write(&path, "{ not json").expect("write");
    let err = ModuleRegistry::read(&path).expect_err("malformed JSON is an error");
    assert_eq!(
        err.code(),
        quoin_modules::ModulesErrorCode::RegistryUnreadable
    );
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_237_read_module_name_matches_the_oracle_for_every_layout() {
    let dir = tempfile::tempdir().expect("temp dir");

    let top = dir.path().join("top");
    fs::create_dir_all(&top).expect("dir");
    fs::write(top.join("manifest.yaml"), "name: spec-objects-business\n").expect("manifest");
    assert_eq!(
        read_module_name(&top).expect("top-level manifest").as_str(),
        "spec-objects-business"
    );

    let nested = dir.path().join("nested-mod");
    fs::create_dir_all(nested.join("nested-mod")).expect("dir");
    fs::write(
        nested.join("nested-mod").join("manifest.yaml"),
        "name: nested-mod\n",
    )
    .expect("manifest");
    assert_eq!(
        read_module_name(&nested).expect("nested manifest").as_str(),
        "nested-mod"
    );

    let absent = dir.path().join("absent");
    fs::create_dir_all(&absent).expect("dir");
    assert_eq!(
        read_module_name(&absent).expect_err("no manifest").code(),
        quoin_modules::ModulesErrorCode::ManifestNotFound
    );

    for (label, body) in [
        ("non-string", "name: 123\n"),
        ("empty-name", "name: \"\"\n"),
        ("no-name-key", "version: 1\n"),
        ("traversing-name", "name: ../elsewhere\n"),
    ] {
        let root = dir.path().join(label);
        fs::create_dir_all(&root).expect("dir");
        fs::write(root.join("manifest.yaml"), body).expect("manifest");
        assert_eq!(
            read_module_name(&root)
                .expect_err("must not yield a name")
                .code(),
            quoin_modules::ModulesErrorCode::ManifestHasNoName,
            "case {label}"
        );
    }
}

/// Trace: FR-019-AC-2
#[test]
fn tc_381_238_install_paths_match_the_oracle() {
    let expected = section("paths");
    let home = IxHome::new("/h");
    assert_eq!(
        home.registry_path().to_string_lossy(),
        expected["registry_path"].as_str().expect("registry_path")
    );
    assert_eq!(
        home.modules_dir().to_string_lossy(),
        expected["modules_dir"].as_str().expect("modules_dir")
    );
}
