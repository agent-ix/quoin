// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Agent-IX

// The crates deny `panic!` and indexing because a request path must not take a
// process down. A test *is* the failure path: a golden that will not load is a
// failing test by design, and an out-of-range index is the assertion doing its
// job. Scoped to this file, never to the library.
#![allow(
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation
)]

//! Parity with the TypeScript oracle, over the committed golden corpus.
//!
//! Every expectation here was captured once from `src/semantic/` by
//! `scripts/capture-semantic-goldens.mjs` and committed under `tests/goldens/`
//! with the quoin revision it ran at. **Nothing in this file invokes Node.**
//!
//! What is asserted, and what deliberately is not:
//!
//! - **Verdicts are contractual and exact.** Every document in the schema
//!   corpora must get the same valid/invalid answer as ajv. A single
//!   disagreement fails the suite and is a finding, not something to allowlist.
//! - **Diagnostics are compared normalized on `(instance location, keyword)`**
//!   as a multiset, per the port's acceptance criterion.
//! - **Error text is not compared.** `DIVERGENCE.md` records where the two
//!   libraries word things differently and what it costs.
//! - **Error order is not compared.** It is not stable across `jsonschema`
//!   releases, let alone across libraries; `SchemaValidator::errors` imposes its
//!   own deterministic order instead.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quoin_semantic::contract::{
    common_schema_path, module_manifest_schema_path, package_manifest_schema_path,
    semantic_core_bundle_digest, semantic_core_dir, sweep_report_schema_path, COMMON_SCHEMA_URI,
    SEMANTIC_CONTRACT,
};
use quoin_semantic::data_schema::{classify_data_schema, DataSchemaForm};
use quoin_semantic::manifest::{
    duplicate_package_diagnostic, read_semantic_block, SemanticValidators,
};
use quoin_semantic::package_manifest::{
    derive_package_manifest, mapping_identity, resolve_imports, type_identity,
    PackageManifestValidator,
};
use quoin_semantic::schema::{read_json, SchemaValidator};
use quoin_semantic::sweep::{classify_artifact, classify_properties, PropertiesForm};
use quoin_semantic::{
    CompatibilityPosture, LegacyForms, MappingName, ModuleName, ModuleVersion, ObjectTypeName,
    PackageIdentity, SemanticBlock, SemanticCoreVersion, SemanticModule,
};
use serde_json::Value;

/// The vendored `src/semantic/` tree the TypeScript also reads.
///
/// The port does not copy the schemas: one tree, so the two implementations
/// cannot drift onto different bytes during the coexistence window NFR-024
/// bounds.
fn semantic_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/semantic")
}

fn golden(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/goldens")
        .join(name);
    match read_json(&path) {
        Ok(value) => value,
        Err(error) => panic!("golden {name} must be readable: {error}"),
    }
}

fn array<'a>(value: &'a Value, key: &str) -> &'a Vec<Value> {
    match value.get(key).and_then(Value::as_array) {
        Some(items) => items,
        None => panic!("golden has no array at {key}"),
    }
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

/// The `(instance location, keyword)` multiset ajv recorded for one document.
fn ajv_identities(errors: &[Value]) -> BTreeMap<(String, String), usize> {
    let mut counts = BTreeMap::new();
    for error in errors {
        let key = (
            text(error, "instancePath").to_owned(),
            text(error, "keyword").to_owned(),
        );
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

/// The same multiset, from this crate.
fn rust_identities(
    errors: &[quoin_semantic::schema::SchemaError],
) -> BTreeMap<(String, String), usize> {
    let mut counts = BTreeMap::new();
    for error in errors {
        let key = (
            error.instance_path.clone(),
            error.keyword.as_str().to_owned(),
        );
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

/// Run one schema corpus and return `(verdict mismatches, shape mismatches)`.
fn run_schema_corpus(validator: &SchemaValidator, corpus: &Value) -> (Vec<String>, Vec<String>) {
    let mut verdicts = Vec::new();
    let mut shapes = Vec::new();
    for case in array(corpus, "documents") {
        let id = text(case, "id");
        let document = case.get("document").cloned().unwrap_or(Value::Null);
        let expected_valid = case
            .get("valid")
            .and_then(Value::as_bool)
            .unwrap_or_default();
        let actual_valid = validator.is_valid(&document);
        if expected_valid != actual_valid {
            verdicts.push(format!("{id}: ajv={expected_valid} rust={actual_valid}"));
            continue;
        }
        if expected_valid {
            assert!(
                validator.errors(&document).is_empty(),
                "{id}: a valid document must produce no errors"
            );
            continue;
        }
        let expected = ajv_identities(array(case, "errors"));
        let actual = rust_identities(&validator.errors(&document));
        if expected != actual {
            shapes.push(format!("{id}: ajv={expected:?} rust={actual:?}"));
        }
    }
    (verdicts, shapes)
}

fn validators() -> SemanticValidators {
    match SemanticValidators::load(&semantic_root()) {
        Ok(v) => v,
        Err(error) => panic!("the vendored schemas must compile: {error}"),
    }
}

/// Trace: FR-070, FR-096
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_100_semantic_block_schema_reaches_exact_verdict_parity() {
    let corpus = golden("schema-semantic-block.json");
    let (verdicts, shapes) = run_schema_corpus(validators().semantic_block(), &corpus);

    assert!(
        array(&corpus, "documents").len() >= 50,
        "the corpus must be a population, not a sample"
    );
    assert!(
        verdicts.is_empty(),
        "verdict parity is contractual; disagreements:\n{}",
        verdicts.join("\n")
    );
    assert!(
        shapes.is_empty(),
        "normalized diagnostic parity failed:\n{}",
        shapes.join("\n")
    );
}

/// Trace: FR-074, FR-096
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_101_sweep_report_schema_reaches_exact_verdict_parity() {
    let corpus = golden("schema-sweep-report.json");
    let (verdicts, shapes) = run_schema_corpus(validators().sweep_report(), &corpus);

    assert!(array(&corpus, "documents").len() >= 45);
    assert!(
        verdicts.is_empty(),
        "verdict parity is contractual; disagreements:\n{}",
        verdicts.join("\n")
    );
    assert!(
        shapes.is_empty(),
        "normalized diagnostic parity failed:\n{}",
        shapes.join("\n")
    );
}

/// Trace: FR-075, FR-096
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_102_package_manifest_schema_reaches_exact_verdict_parity() {
    let root = semantic_root();
    let schema_path = package_manifest_schema_path(&root);
    let schema = match read_json(&schema_path) {
        Ok(value) => value,
        Err(error) => panic!("package-manifest schema must be readable: {error}"),
    };
    let common = match read_json(&common_schema_path(&root)) {
        Ok(value) => value,
        Err(error) => panic!("common schema must be readable: {error}"),
    };
    let validator =
        match SchemaValidator::compile(&schema_path, &schema, &[(COMMON_SCHEMA_URI, common)]) {
            Ok(v) => v,
            Err(error) => panic!("package-manifest schema must compile: {error}"),
        };

    let corpus = golden("schema-package-manifest.json");
    let (verdicts, shapes) = run_schema_corpus(&validator, &corpus);

    assert!(array(&corpus, "documents").len() >= 50);
    assert!(
        verdicts.is_empty(),
        "verdict parity is contractual; disagreements:\n{}",
        verdicts.join("\n")
    );
    assert!(
        shapes.is_empty(),
        "normalized diagnostic parity failed:\n{}",
        shapes.join("\n")
    );
}

/// Trace: FR-070
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_103_read_semantic_block_matches_the_oracle() {
    let corpus = golden("read-semantic-block.json");
    let validators = validators();
    let mut checked = 0usize;

    for case in array(&corpus, "cases") {
        let id = text(case, "id");
        let temp = match tempfile::tempdir() {
            Ok(dir) => dir,
            Err(error) => panic!("{id}: tempdir: {error}"),
        };
        let root = temp.path();
        if let Some(files) = case.get("files").and_then(Value::as_object) {
            for (relative, body) in files {
                let path = root.join(relative);
                if let Some(parent) = path.parent() {
                    if let Err(error) = std::fs::create_dir_all(parent) {
                        panic!("{id}: mkdir: {error}");
                    }
                }
                if let Err(error) = std::fs::write(&path, body.as_str().unwrap_or_default()) {
                    panic!("{id}: write: {error}");
                }
            }
        }

        let manifest = case.get("manifest").cloned().unwrap_or(Value::Null);
        let result = read_semantic_block(&manifest, root, &validators);

        let expected: BTreeMap<(String, String, String), usize> =
            count_diagnostics(array(case, "diagnostics"));
        let actual: BTreeMap<(String, String, String), usize> =
            result
                .diagnostics
                .iter()
                .fold(BTreeMap::new(), |mut counts, d| {
                    *counts
                        .entry((
                            d.code.as_str().to_owned(),
                            d.severity.as_str().to_owned(),
                            d.path.clone(),
                        ))
                        .or_insert(0) += 1;
                    counts
                });
        assert_eq!(expected, actual, "{id}: diagnostics diverged");

        match (case.get("module"), result.module.as_ref()) {
            (Some(Value::Null) | None, None) => {}
            (Some(expected), Some(module)) => {
                assert_eq!(
                    text(expected, "name"),
                    module.name.as_str(),
                    "{id}: module name"
                );
                assert_eq!(
                    text(expected, "version"),
                    module.version.as_str(),
                    "{id}: module version"
                );
                assert_block(
                    id,
                    expected.get("block").unwrap_or(&Value::Null),
                    &module.block,
                );
            }
            (expected, actual) => panic!(
                "{id}: module presence diverged (oracle {}, rust {})",
                expected.is_some_and(|v| !v.is_null()),
                actual.is_some()
            ),
        }
        checked += 1;
    }

    assert!(
        checked >= 20,
        "checked {checked} cases, expected a population"
    );
}

fn count_diagnostics(diagnostics: &[Value]) -> BTreeMap<(String, String, String), usize> {
    let mut counts = BTreeMap::new();
    for diagnostic in diagnostics {
        *counts
            .entry((
                text(diagnostic, "code").to_owned(),
                text(diagnostic, "severity").to_owned(),
                text(diagnostic, "path").to_owned(),
            ))
            .or_insert(0) += 1;
    }
    counts
}

fn assert_block(id: &str, expected: &Value, block: &SemanticBlock) {
    assert_eq!(
        text(expected, "contract_version"),
        block.contract_version.as_str(),
        "{id}: contract_version"
    );
    assert_eq!(
        text(expected, "semantic_core"),
        block.semantic_core.as_str(),
        "{id}: semantic_core"
    );
    assert_eq!(
        text(expected, "package"),
        block.package.as_str(),
        "{id}: package"
    );
    let expected_exports: Vec<&str> = expected
        .get("exports")
        .and_then(Value::as_array)
        .map(|v| v.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let actual_exports: Vec<&str> = block.exports.iter().map(ObjectTypeName::as_str).collect();
    assert_eq!(expected_exports, actual_exports, "{id}: exports");

    let expected_targets: Vec<&str> = expected
        .get("targets")
        .and_then(Value::as_array)
        .map(|v| v.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let actual_targets: Vec<&str> = block.targets.iter().map(String::as_str).collect();
    assert_eq!(expected_targets, actual_targets, "{id}: targets");

    let expected_mappings: Vec<&str> = expected
        .get("mappings")
        .and_then(Value::as_array)
        .map(|v| v.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let actual_mappings: Vec<&str> = block.mappings.iter().map(MappingName::as_str).collect();
    assert_eq!(expected_mappings, actual_mappings, "{id}: mappings");

    let expected_imports: BTreeMap<String, String> = expected
        .get("imports")
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned()))
                .collect()
        })
        .unwrap_or_default();
    let actual_imports: BTreeMap<String, String> = block
        .imports
        .iter()
        .map(|(k, v)| (k.as_str().to_owned(), v.as_str().to_owned()))
        .collect();
    assert_eq!(expected_imports, actual_imports, "{id}: imports");

    assert_eq!(
        text(expected, "compatibility_posture"),
        block.compatibility_posture.as_str(),
        "{id}: compatibility_posture"
    );
    assert_eq!(
        text(expected, "legacy_forms"),
        block.legacy_forms.as_str(),
        "{id}: legacy_forms"
    );
    assert_eq!(
        expected.get("sweep_report").and_then(Value::as_str),
        block.sweep_report.as_deref(),
        "{id}: sweep_report"
    );
}

/// Trace: FR-074
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_104_properties_classifier_matches_the_oracle() {
    let corpus = golden("classify-properties.json");
    let cases = array(&corpus, "cases");
    assert!(cases.len() >= 25);

    for case in cases {
        let id = text(case, "id");
        let markdown = text(case, "markdown");
        let classification = classify_properties(markdown);

        assert_eq!(
            text(case, "form"),
            classification.form.as_str(),
            "{id}: form"
        );
        let expected_line = case.get("line").and_then(Value::as_u64).map(|n| n as usize);
        assert_eq!(expected_line, classification.line, "{id}: line");

        let finding = classify_artifact(&format!("fixture:{id}.md"), markdown);
        let expected_finding = case.get("finding").unwrap_or(&Value::Null);
        assert_eq!(
            text(expected_finding, "form"),
            finding.form.as_str(),
            "{id}: finding form"
        );
        let expected_diagnostic = expected_finding.get("diagnostic");
        match (expected_diagnostic, finding.diagnostic.as_ref()) {
            (None | Some(Value::Null), None) => {}
            (Some(expected), Some(actual)) => {
                assert_eq!(text(expected, "code"), actual.code, "{id}: diagnostic code");
                assert_eq!(
                    text(expected, "severity"),
                    actual.severity,
                    "{id}: diagnostic severity"
                );
                assert_eq!(
                    text(expected, "form"),
                    actual.form.as_str(),
                    "{id}: diagnostic form"
                );
                assert_eq!(
                    expected
                        .get("line")
                        .and_then(Value::as_u64)
                        .map(|n| n as usize),
                    Some(actual.line),
                    "{id}: diagnostic line"
                );
                assert_eq!(
                    text(expected, "migration"),
                    actual.migration,
                    "{id}: diagnostic migration"
                );
            }
            (expected, actual) => panic!(
                "{id}: diagnostic presence diverged (oracle {}, rust {})",
                expected.is_some_and(|v| !v.is_null()),
                actual.is_some()
            ),
        }
    }
}

/// Build a [`SemanticModule`] from a golden's block description.
fn module_from(golden_module: &Value) -> SemanticModule {
    let block = golden_module.get("block").unwrap_or(&Value::Null);
    SemanticModule {
        name: ModuleName::new(text(golden_module, "name")),
        version: ModuleVersion::new(text(golden_module, "version")),
        root: PathBuf::from(
            golden_module
                .get("root")
                .and_then(Value::as_str)
                .unwrap_or("/modules/unknown"),
        ),
        block: SemanticBlock {
            contract_version: text(block, "contract_version").into(),
            semantic_core: SemanticCoreVersion::new(text(block, "semantic_core")),
            package: PackageIdentity::new(text(block, "package")),
            exports: block
                .get("exports")
                .and_then(Value::as_array)
                .map(|v| {
                    v.iter()
                        .filter_map(Value::as_str)
                        .map(ObjectTypeName::from)
                        .collect()
                })
                .unwrap_or_default(),
            imports: block
                .get("imports")
                .and_then(Value::as_object)
                .map(|map| {
                    map.iter()
                        .map(|(k, v)| {
                            (
                                PackageIdentity::new(k.clone()),
                                ModuleVersion::new(v.as_str().unwrap_or_default()),
                            )
                        })
                        .collect()
                })
                .unwrap_or_default(),
            targets: block
                .get("targets")
                .and_then(Value::as_array)
                .map(|v| {
                    v.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
            mappings: block
                .get("mappings")
                .and_then(Value::as_array)
                .map(|v| {
                    v.iter()
                        .filter_map(Value::as_str)
                        .map(MappingName::from)
                        .collect()
                })
                .unwrap_or_default(),
            compatibility_posture: CompatibilityPosture::from_str_opt(text(
                block,
                "compatibility_posture",
            ))
            .unwrap_or(CompatibilityPosture::Additive),
            legacy_forms: LegacyForms::from_str_opt(text(block, "legacy_forms"))
                .unwrap_or(LegacyForms::Warning),
            sweep_report: block
                .get("sweep_report")
                .and_then(Value::as_str)
                .map(str::to_owned),
        },
        data_schemas: BTreeMap::new(),
    }
}

/// Trace: FR-075
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_105_derived_package_manifest_matches_the_oracle() {
    let corpus = golden("derive-package-manifest.json");
    let validator = match PackageManifestValidator::load(&semantic_root()) {
        Ok(v) => v,
        Err(error) => panic!("package-manifest schema must compile: {error}"),
    };

    for case in array(&corpus, "cases") {
        let id = text(case, "id");
        let module = module_from(case.get("module").unwrap_or(&Value::Null));
        let derived = derive_package_manifest(&module);
        let actual = match derived.to_value() {
            Ok(value) => value,
            Err(error) => panic!("{id}: derived manifest must serialize: {error}"),
        };
        let expected = case.get("derived").unwrap_or(&Value::Null);
        assert_eq!(expected, &actual, "{id}: derived package manifest");

        // The oracle also recorded whether ajv accepted the derived document.
        let expected_valid = case
            .get("validAgainstSchema")
            .and_then(Value::as_bool)
            .unwrap_or_default();
        assert_eq!(
            expected_valid,
            validator.is_valid(&actual),
            "{id}: derived manifest validity"
        );
    }
}

/// Trace: FR-075
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_106_resolve_imports_matches_the_oracle() {
    let corpus = golden("resolve-imports.json");
    for case in array(&corpus, "cases") {
        let id = text(case, "id");
        let candidate = module_from(case.get("candidate").unwrap_or(&Value::Null));
        let installed: Vec<SemanticModule> =
            array(case, "installed").iter().map(module_from).collect();
        let diagnostics = resolve_imports(&candidate, &installed);

        let expected = count_diagnostics(array(case, "diagnostics"));
        let actual = diagnostics.iter().fold(BTreeMap::new(), |mut counts, d| {
            *counts
                .entry((
                    d.code.as_str().to_owned(),
                    d.severity.as_str().to_owned(),
                    d.path.clone(),
                ))
                .or_insert(0) += 1;
            counts
        });
        assert_eq!(expected, actual, "{id}: import diagnostics");
    }
}

/// Trace: FR-070
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_107_duplicate_package_matches_the_oracle() {
    let corpus = golden("duplicate-package.json");
    for case in array(&corpus, "cases") {
        let id = text(case, "id");
        let candidate = module_from(case.get("candidate").unwrap_or(&Value::Null));
        let installed: Vec<SemanticModule> =
            array(case, "installed").iter().map(module_from).collect();
        let actual = duplicate_package_diagnostic(&candidate, &installed);
        match (case.get("diagnostic"), actual.as_ref()) {
            (None | Some(Value::Null), None) => {}
            (Some(expected), Some(diagnostic)) => {
                assert_eq!(text(expected, "code"), diagnostic.code.as_str(), "{id}");
                assert_eq!(text(expected, "path"), diagnostic.path, "{id}");
            }
            (expected, got) => panic!(
                "{id}: presence diverged (oracle {}, rust {})",
                expected.is_some_and(|v| !v.is_null()),
                got.is_some()
            ),
        }
    }
}

/// Trace: FR-075
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_108_identities_match_the_oracle() {
    let corpus = golden("identities.json");
    for case in array(&corpus, "cases") {
        let package = PackageIdentity::new(text(case, "package"));
        let name = text(case, "name");
        assert_eq!(
            text(case, "typeIdentity"),
            type_identity(&package, &ObjectTypeName::new(name))
        );
        assert_eq!(
            text(case, "mappingIdentity"),
            mapping_identity(&package, name)
        );
    }
}

/// Trace: FR-073
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_109_classify_data_schema_matches_the_oracle() {
    let corpus = golden("classify-data-schema.json");
    for case in array(&corpus, "cases") {
        let id = text(case, "id");
        let value = case.get("value").cloned().unwrap_or(Value::Null);
        let (form, diagnostics) = classify_data_schema(&value, "object_types[x].data_schema");
        assert_eq!(text(case, "form"), form.as_str(), "{id}: form");

        let expected = count_diagnostics(array(case, "diagnostics"));
        let actual = diagnostics.iter().fold(BTreeMap::new(), |mut counts, d| {
            *counts
                .entry((
                    d.code.as_str().to_owned(),
                    d.severity.as_str().to_owned(),
                    d.path.clone(),
                ))
                .or_insert(0) += 1;
            counts
        });
        assert_eq!(expected, actual, "{id}: diagnostics");
    }
}

/// Trace: FR-070, NFR-025
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_110_vendored_bytes_still_hash_to_the_recorded_provenance() {
    let root = semantic_root();
    let golden = golden("contract.json");

    let digest = match semantic_core_bundle_digest(&semantic_core_dir(&root)) {
        Ok(digest) => digest,
        Err(error) => panic!("semantic-core bundle must be readable: {error}"),
    };
    assert_eq!(
        SEMANTIC_CONTRACT.semantic_core.bundle_digest, digest,
        "the vendored semantic-core bundle no longer hashes to its recorded digest"
    );
    assert_eq!(text(&golden, "semanticCoreBundleDigest"), digest);

    for (label, path, recorded) in [
        (
            "module-manifest",
            module_manifest_schema_path(&root),
            SEMANTIC_CONTRACT.module_manifest_schema.sha256,
        ),
        (
            "package-manifest",
            package_manifest_schema_path(&root),
            SEMANTIC_CONTRACT.package_manifest_schema.sha256,
        ),
        (
            "common",
            common_schema_path(&root),
            SEMANTIC_CONTRACT.common_schema.sha256,
        ),
    ] {
        let actual = match quoin_semantic::contract::file_sha256(&path) {
            Ok(hash) => hash,
            Err(error) => panic!("{label} schema must be readable: {error}"),
        };
        assert_eq!(recorded, actual, "{label} schema bytes changed");
    }

    // The sweep-report schema is quoin's own; it has no vendored provenance,
    // but it must exist and compile.
    assert!(sweep_report_schema_path(&root).exists());
}

/// Trace: FR-070
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_111_the_contract_matches_the_oracle_exactly() {
    let golden = golden("contract.json");
    let contract = golden.get("contract").unwrap_or(&Value::Null);
    assert_eq!(
        text(contract, "contractVersion"),
        SEMANTIC_CONTRACT.contract_version
    );
    let versions: Vec<&str> = contract
        .get("semanticCoreVersions")
        .and_then(Value::as_array)
        .map(|v| v.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    assert_eq!(versions, SEMANTIC_CONTRACT.semantic_core_versions);

    let keys: Vec<&str> = contract
        .get("semanticKeys")
        .and_then(Value::as_array)
        .map(|v| v.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    assert_eq!(keys, SEMANTIC_CONTRACT.semantic_keys);
}

/// Trace: FR-074
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_112_every_properties_form_is_exercised_by_the_corpus() {
    let corpus = golden("classify-properties.json");
    let seen: std::collections::BTreeSet<&str> = array(&corpus, "cases")
        .iter()
        .map(|case| text(case, "form"))
        .collect();
    for form in PropertiesForm::all() {
        assert!(
            seen.contains(form.as_str()),
            "no corpus document classifies as {form:?}; a check over an empty \
             population is not evidence"
        );
    }
}

/// Trace: FR-073
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_113_every_data_schema_form_is_exercised_by_the_corpus() {
    let corpus = golden("classify-data-schema.json");
    let seen: std::collections::BTreeSet<&str> = array(&corpus, "cases")
        .iter()
        .map(|case| text(case, "form"))
        .collect();
    for form in [
        DataSchemaForm::Inline,
        DataSchemaForm::Reference,
        DataSchemaForm::Invalid,
    ] {
        assert!(
            seen.contains(form.as_str()),
            "no corpus document is {form:?}"
        );
    }
}

/// Trace: FR-096
/// Provenance: agent-ix/quoin#378
#[test]
fn tc_378_114_the_goldens_name_the_revision_they_were_captured_from() {
    for name in [
        "schema-semantic-block.json",
        "schema-sweep-report.json",
        "schema-package-manifest.json",
        "read-semantic-block.json",
        "classify-properties.json",
        "derive-package-manifest.json",
        "resolve-imports.json",
        "duplicate-package.json",
        "identities.json",
        "classify-data-schema.json",
        "contract.json",
    ] {
        let golden = golden(name);
        let provenance = golden.get("provenance").unwrap_or(&Value::Null);
        let revision = text(provenance, "quoinRevision");
        assert_eq!(
            revision.len(),
            40,
            "{name}: provenance must name a full quoin revision, got {revision:?}"
        );
        assert!(
            !text(provenance, "ajvVersion").is_empty(),
            "{name}: provenance must name the ajv the oracle ran"
        );
    }
}
