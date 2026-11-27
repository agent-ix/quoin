// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Run the shared relationship mapping cases through Quoin's pinned Quire edge.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "malformed pinned fixtures are test failures"
)]

use ix_trace_rs::trace;
use quire_rs::semantic::python_entry::extract_semantic_json;
use serde_json::{Value, json};

fn case(id: &str) -> (Value, Value, Value) {
    let fixtures = quire_fixtures::mapping_json("relationships-cases.json");
    let context = fixtures["context"].clone();
    let item = fixtures["cases"]
        .as_array()
        .expect("relationship fixture cases")
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("the shared relationship fixture includes {id}"))
        .clone();

    let artifact = item.get("artifact").and_then(Value::as_str).map_or_else(
        || {
            format!(
                "{}{}",
                fixtures["artifactHead"].as_str().expect("artifact head"),
                item["relationships"]
                    .as_str()
                    .expect("case relationship body")
            )
        },
        str::to_owned,
    );

    let mut mappings = context["mappings"].clone();
    if let Some(value) = item.get("mappings") {
        mappings = value.clone();
    }
    let bundle_context = &context["bundle"];
    let imports = bundle_context["imports"]
        .as_array()
        .expect("bundle imports")
        .iter()
        .map(|package| {
            (
                package.as_str().expect("import package").to_owned(),
                json!("*"),
            )
        })
        .collect::<serde_json::Map<String, Value>>();
    let artifacts = bundle_context["artifacts"]
        .as_array()
        .expect("bundle artifacts")
        .iter()
        .map(|artifact| {
            let mut artifact = artifact.as_object().expect("bundle artifact").clone();
            artifact.insert("operations".to_owned(), json!([]));
            Value::Object(artifact)
        })
        .collect::<Vec<_>>();
    let bundle = json!({"package": bundle_context["package"]});

    let mut request = json!({
        "markdown": artifact,
        "module": {
            "contractVersion": "1.0.0",
            "package": "agent-ix/spec-objects-business",
            "exports": ["entity"],
            "imports": imports,
            "mappings": mappings,
        },
        "path": "case.md",
        "sourceIdentity": context["sourceIdentity"],
        "bundle": bundle,
        "relationVocabulary": {
            "objectType": context["objectType"],
            "edgeTypes": context["edgeTypes"],
            "roles": context["roles"],
            "allowedLinks": context["allowedLinks"],
        },
    });
    request["bundle"]["artifacts"] = json!(artifacts);
    if item["withoutBundlePackage"] == true {
        request["bundle"]["package"] = json!("");
        request
            .as_object_mut()
            .expect("request object")
            .remove("sourceIdentity");
    }
    if item["withoutBundleIndex"] == true {
        request["bundle"]
            .as_object_mut()
            .expect("bundle object")
            .remove("artifacts");
    }
    if let Some(package) = item.get("importedPackage").and_then(Value::as_str) {
        request["module"]["imports"][package] = json!("*");
    }
    if item["withoutImportedArtifactIndex"] == true {
        request["bundle"]["importedArtifacts"] = json!([]);
    }
    if item["withoutRelationVocabulary"] == true {
        request
            .as_object_mut()
            .expect("request object")
            .remove("relationVocabulary");
    }
    let output = serde_json::to_value(
        extract_semantic_json(&request).expect("the pinned engine extracts each case"),
    )
    .expect("semantic extraction serializes");
    (item, output, context)
}

fn assert_cases(ids: &[&str]) {
    for id in ids {
        let (expected, actual, _) = case(id);
        let expected_diagnostics = expected["diagnostics"]
            .as_array()
            .expect("fixture diagnostics");
        let actual_diagnostics = actual["diagnostics"]
            .as_array()
            .expect("runtime diagnostics");
        let expected_count = usize::try_from(
            expected["exactDiagnostics"]
                .as_u64()
                .expect("fixture diagnostic count"),
        )
        .expect("fixture diagnostic count fits the host");
        assert_eq!(
            actual_diagnostics.len(),
            expected_count,
            "exact diagnostic count for {id}"
        );
        for (actual_diagnostic, expected_diagnostic) in
            actual_diagnostics.iter().zip(expected_diagnostics)
        {
            for key in ["code", "severity", "line", "section", "reason"] {
                assert_eq!(
                    actual_diagnostic.get(key),
                    expected_diagnostic.get(key),
                    "diagnostic {key} for {id}"
                );
            }
            let message = actual_diagnostic["message"]
                .as_str()
                .expect("runtime diagnostic message");
            for needle in ["case.md", "relationships"] {
                assert!(message.contains(needle), "{id}: {message}");
            }
            for needle in expected_diagnostic["messageContains"]
                .as_array()
                .into_iter()
                .flatten()
            {
                let needle = needle.as_str().expect("message substring");
                assert!(
                    message.contains(needle),
                    "{id}: {message:?} lacks {needle:?}"
                );
            }
        }
        assert_eq!(
            actual["relations"], expected["relations"],
            "relations for {id}"
        );
        let expected_availability = &expected["availability"]["relations"];
        let actual_availability = &actual["availability"]["relations"];
        if let Some(loci) = expected_availability.get("reasonLoci") {
            assert!(expected_availability.get("reason").is_none());
            let lines = loci
                .as_array()
                .expect("reason loci")
                .iter()
                .map(|locus| {
                    let locus = locus.as_str().expect("diagnostic locus");
                    expected_diagnostics
                        .iter()
                        .find(|diagnostic| diagnostic["locus"] == locus)
                        .unwrap_or_else(|| panic!("{id}: no diagnostic at locus {locus}"))["line"]
                        .as_u64()
                        .expect("diagnostic line")
                })
                .collect::<Vec<_>>();
            let reason_lines = actual_availability["reason"]
                .as_str()
                .and_then(|reason| reason.strip_prefix("entry-errors: lines "))
                .expect("runtime entry-error reason")
                .split(", ")
                .map(|line| line.parse::<u64>().expect("entry-error line"))
                .collect::<Vec<_>>();
            assert_eq!(
                reason_lines, lines,
                "entry-error availability lines for {id}"
            );
            for key in ["state", "lossy"] {
                assert_eq!(
                    actual_availability[key], expected_availability[key],
                    "{id}: {key}"
                );
            }
        } else {
            assert_eq!(
                actual_availability, expected_availability,
                "availability for {id}"
            );
        }
        for key in ["relationSources", "resolvedReferences"] {
            if let Some(value) = expected.get(key) {
                let observed = actual.get(key).cloned().unwrap_or_else(|| json!([]));
                assert_eq!(observed, *value, "{key} for {id}");
            }
        }
    }
}

#[trace("FR-104-AC-2")]
#[test]
fn relation_verbs_use_vocabulary_and_reject_inverse_aliases() {
    assert_cases(&[
        "verb-unknown",
        "verb-inverse-label",
        "verb-not-allowed",
        "verb-specializes",
    ]);
}

#[trace("FR-104-AC-3")]
#[test]
fn relationship_targets_resolve_only_declared_and_allowed_objects() {
    assert_cases(&[
        "target-title",
        "target-undeclared-id",
        "target-own-package-undeclared-id",
        "target-qualified-not-imported",
        "target-not-allowed-object-type",
        "target-not-allowed-behavioral",
        "target-qualified-import",
    ]);
}

#[trace("FR-104-AC-4")]
#[test]
fn relationship_multiplicity_must_parse_and_be_ordered() {
    assert_cases(&[
        "multiplicity-malformed",
        "multiplicity-inverted",
        "multiplicity-empty",
    ]);
}

#[trace("FR-104-AC-5")]
#[test]
fn relation_names_are_unique_identifiers_and_frontmatter_declarations_win() {
    assert_cases(&[
        "name-not-identifier",
        "duplicate-name",
        "declared-in-frontmatter",
    ]);
}

#[trace("FR-104-AC-6")]
#[test]
fn relationship_tables_require_the_single_exact_table_shape() {
    assert_cases(&[
        "column-missing",
        "column-extra",
        "bullet-list",
        "list-after-table",
        "second-table",
        "second-section",
    ]);
}

#[trace("FR-104-AC-7")]
#[test]
fn relationship_mapping_must_be_declared_and_own_the_section() {
    assert_cases(&[
        "mapping-not-declared",
        "subset-header-unowned-section",
        "preamble-table",
        "prose-without-mapping",
    ]);
}

#[trace("FR-104-AC-8")]
#[test]
fn relationship_diagnostics_report_only_the_first_failing_check_per_row() {
    assert_cases(&[
        "good-and-bad-rows",
        "two-error-lines",
        "header-only",
        "section-absent",
        "first-failing-check-only",
    ]);
}

#[trace("FR-104-AC-9")]
#[test]
fn relationship_tokens_are_not_found_in_unowned_sections_or_prose() {
    assert_cases(&[
        "token-table-other-section",
        "token-preamble-table",
        "prose-only-section",
    ]);
}

#[trace("FR-104-AC-10")]
#[test]
fn absent_relationship_vocabulary_or_bundle_index_is_reported_as_partial() {
    assert_cases(&[
        "no-relation-vocabulary",
        "no-bundle-index",
        "no-bundle-index-title-target",
        "no-bundle-index-own-package-identity",
        "no-relation-vocabulary-second-table",
        "no-bundle-index-bad-multiplicity",
    ]);
}

#[trace("FR-104-AC-11")]
#[test]
fn bundle_index_loss_does_not_drop_resolvable_rows() {
    assert_cases(&["no-bundle-index-mixed-rows"]);
}
