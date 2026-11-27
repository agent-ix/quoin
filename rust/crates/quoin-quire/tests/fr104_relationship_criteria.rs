// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-104 relationship mapping acceptance criteria, exercised through the
//! exact Quire engine revision linked by Quoin and quire-fixtures oracles.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "fixture contract mismatches are test failures"
)]

use ix_trace_rs::trace;
use quire_rs::semantic::python_entry::extract_semantic_json;
use serde_json::{Value, json};

fn cases() -> Value {
    quire_fixtures::mapping_json("relationships-cases.json")
}

fn request(file: &Value, markdown: &str, case: Option<&Value>) -> Value {
    let context = &file["context"];
    let flag = |key: &str| case.is_some_and(|c| c[key] == json!(true));
    let bundle = &context["bundle"];
    let imports: serde_json::Map<String, Value> = bundle["imports"]
        .as_array()
        .expect("relationship fixture imports")
        .iter()
        .map(|package| (package.as_str().unwrap().to_string(), json!("*")))
        .collect();
    let artifacts: Vec<Value> = bundle["artifacts"]
        .as_array()
        .expect("relationship fixture bundle artifacts")
        .iter()
        .map(|artifact| {
            let mut artifact = artifact.as_object().unwrap().clone();
            artifact.insert("operations".to_string(), json!([]));
            Value::Object(artifact)
        })
        .collect();
    let mut request = json!({
        "markdown": markdown,
        "module": {
            "contractVersion": "1.0.0",
            "package": "agent-ix/spec-objects-business",
            "exports": ["entity"],
            "imports": imports,
            "mappings": case.and_then(|c| c.get("mappings")).unwrap_or(&context["mappings"]),
        },
        "path": "case.md",
        "sourceIdentity": context["sourceIdentity"],
        "bundle": { "package": bundle["package"], "artifacts": artifacts },
        "relationVocabulary": {
            "objectType": context["objectType"],
            "edgeTypes": context["edgeTypes"],
            "roles": context["roles"],
            "allowedLinks": context["allowedLinks"],
        },
    });
    let request = request.as_object_mut().unwrap();
    if flag("withoutBundlePackage") {
        request["bundle"]["package"] = json!("");
        request.remove("sourceIdentity");
    }
    if flag("withoutRelationVocabulary") {
        request.remove("relationVocabulary");
    }
    if flag("withoutBundleIndex") {
        request["bundle"]
            .as_object_mut()
            .unwrap()
            .remove("artifacts");
    }
    if let Some(package) = case.and_then(|c| c["importedPackage"].as_str()) {
        request["module"]["imports"][package] = json!("*");
    }
    if flag("withoutImportedArtifactIndex") {
        request["bundle"]["importedArtifacts"] = json!([]);
    }
    Value::Object(request.clone())
}

fn markdown(file: &Value, case: &Value) -> String {
    match case["artifact"].as_str() {
        Some(artifact) => artifact.to_string(),
        None => format!(
            "{}{}",
            file["artifactHead"].as_str().unwrap(),
            case["relationships"].as_str().unwrap()
        ),
    }
}

fn locus_holds(md: &str, line: u64, locus: &str) -> bool {
    let lines: Vec<&str> = md.split('\n').collect();
    let at = |number: u64| lines.get(number as usize - 1).copied().unwrap_or("");
    let separator = |text: &str| text.starts_with('|') && text.contains("---");
    match locus {
        "header" | "second-header" => at(line).starts_with('|') && separator(at(line + 1)),
        "row" | "second-row" => at(line).starts_with('|') && !separator(at(line)),
        "list" => at(line).starts_with("- "),
        "heading" | "second-heading" => at(line) == "## Relationships",
        other => panic!("unknown fixture locus {other:?}"),
    }
}

fn assert_case(file: &Value, case: &Value) {
    let id = case["id"].as_str().unwrap();
    let md = markdown(file, case);
    let value = request(file, &md, Some(case));
    let record = serde_json::to_value(
        extract_semantic_json(&value).unwrap_or_else(|error| panic!("{id}: {error}")),
    )
    .unwrap();
    let actual = record["diagnostics"].as_array().unwrap();
    assert_eq!(
        actual.len() as u64,
        case["exactDiagnostics"].as_u64().unwrap(),
        "{id}"
    );
    let expected = case["diagnostics"].as_array().unwrap();
    assert_eq!(actual.len(), expected.len(), "{id}: diagnostics");
    let lines: Vec<&str> = md.split('\n').collect();
    for (got, want) in actual.iter().zip(expected) {
        for key in ["code", "severity", "line", "section", "reason"] {
            assert_eq!(got[key], want[key], "{id}: diagnostic {key}");
        }
        let line = want["line"].as_u64().unwrap();
        assert!(
            locus_holds(&md, line, want["locus"].as_str().unwrap()),
            "{id}: line {line}"
        );
        let span = &got["sourceSpan"];
        assert_eq!(span["startLine"], line, "{id}");
        assert_eq!(span["endLine"], line, "{id}");
        assert_eq!(span["startColumn"], 1, "{id}");
        assert_eq!(
            span["endColumn"].as_u64().unwrap(),
            lines[line as usize - 1].len() as u64 + 1,
            "{id}"
        );
        let message = got["message"].as_str().unwrap();
        for needle in ["case.md", "relationships"] {
            assert!(message.contains(needle), "{id}: {message}");
        }
        for needle in want["messageContains"].as_array().into_iter().flatten() {
            let needle = needle.as_str().unwrap();
            assert!(
                message.contains(needle),
                "{id}: {message:?} lacks {needle:?}"
            );
        }
    }
    if let Some(resolved) = case.get("resolvedReferences") {
        let actual = record
            .get("resolvedReferences")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(Value::Array(actual), *resolved, "{id}: resolvedReferences");
    }
    if case["relations"].is_null() {
        assert!(record.get("relations").is_none(), "{id}: relations emitted");
        assert!(
            record.get("relationSources").is_none(),
            "{id}: relation sources emitted"
        );
    } else {
        assert_eq!(record["relations"], case["relations"], "{id}: relations");
        assert_eq!(
            record["relationSources"], case["relationSources"],
            "{id}: relationSources"
        );
    }
    match case["availability"].get("relations") {
        None => assert!(record["availability"].get("relations").is_none(), "{id}"),
        Some(want) => {
            let got = &record["availability"]["relations"];
            if let Some(loci) = want.get("reasonLoci") {
                assert_eq!(got["state"], want["state"], "{id}: availability state");
                assert_eq!(got["lossy"], want["lossy"], "{id}: availability lossy");
                let expected_lines: Vec<u64> = loci
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|locus| {
                        case["diagnostics"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|diagnostic| diagnostic["locus"] == *locus)
                            .unwrap_or_else(|| panic!("{id}: no diagnostic for {locus}"))["line"]
                            .as_u64()
                            .unwrap()
                    })
                    .collect();
                assert_eq!(
                    got["reason"],
                    format!(
                        "entry-errors: lines {}",
                        expected_lines
                            .iter()
                            .map(u64::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    "{id}: availability reason"
                );
            } else {
                assert_eq!(got, want, "{id}: availability");
            }
        }
    }
}

fn run(ids: &[&str]) {
    let file = cases();
    for id in ids {
        let case = file["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == *id)
            .unwrap_or_else(|| panic!("quire-fixtures has no relationship case {id}"));
        assert_case(&file, case);
    }
}

const VERBS: &[&str] = &[
    "verb-unknown",
    "verb-inverse-label",
    "verb-not-allowed",
    "verb-specializes",
];
const TARGETS: &[&str] = &[
    "target-title",
    "target-undeclared-id",
    "target-own-package-undeclared-id",
    "target-qualified-not-imported",
    "target-not-allowed-object-type",
    "target-not-allowed-behavioral",
    "target-qualified-import",
];
const MULTIPLICITIES: &[&str] = &[
    "multiplicity-malformed",
    "multiplicity-inverted",
    "multiplicity-empty",
];
const NAMES: &[&str] = &[
    "name-not-identifier",
    "duplicate-name",
    "declared-in-frontmatter",
];
const SHAPES: &[&str] = &[
    "column-missing",
    "column-extra",
    "bullet-list",
    "second-table",
    "second-section",
    "list-after-table",
];
const MAPPING_GATING: &[&str] = &[
    "mapping-not-declared",
    "subset-header-unowned-section",
    "preamble-table",
    "prose-without-mapping",
];
const TOKEN_GATING: &[&str] = &[
    "token-table-other-section",
    "token-preamble-table",
    "prose-only-section",
];
const AVAILABILITY: &[&str] = &[
    "good-and-bad-rows",
    "two-error-lines",
    "header-only",
    "section-absent",
    "first-failing-check-only",
];
const NO_VOCABULARY: &[&str] = &[
    "no-relation-vocabulary",
    "no-relation-vocabulary-second-table",
];
const NO_BUNDLE_INDEX: &[&str] = &[
    "no-bundle-index",
    "no-bundle-index-title-target",
    "no-bundle-index-own-package-identity",
    "no-bundle-index-bad-multiplicity",
];
const MIXED_ROWS: &[&str] = &["no-bundle-index-mixed-rows"];

#[trace("FR-104-AC-2")]
#[test]
fn verb_refusals_follow_the_registry_and_check_order() {
    run(VERBS);
}

#[trace("FR-104-AC-3")]
#[test]
fn target_identity_import_and_allowed_link_cases() {
    run(TARGETS);
}

#[trace("FR-104-AC-4")]
#[test]
fn malformed_and_inverted_multiplicities_are_refused() {
    run(MULTIPLICITIES);
}

#[trace("FR-104-AC-5")]
#[test]
fn relationship_names_and_frontmatter_duplicates_are_checked() {
    run(NAMES);
}

#[trace("FR-104-AC-6")]
#[test]
fn relationship_table_shape_and_duplicate_sections_are_checked() {
    run(SHAPES);
}

#[trace("FR-104-AC-7")]
#[test]
fn mapping_token_and_unowned_table_gates_are_checked() {
    run(MAPPING_GATING);
}

#[trace("FR-104-AC-8")]
#[test]
fn relationship_availability_and_first_failure_cases_are_checked() {
    run(AVAILABILITY);
}

#[trace("FR-104-AC-9")]
#[test]
fn token_tables_outside_relationships_and_prose_only_are_checked() {
    run(TOKEN_GATING);
}

#[trace("FR-104-AC-10")]
#[test]
fn missing_vocabulary_and_bundle_index_cases_are_checked() {
    run(NO_VOCABULARY);
    run(NO_BUNDLE_INDEX);
}

#[trace("FR-104-AC-11")]
#[test]
fn a_row_error_suppresses_the_unresolved_target_advisory() {
    run(MIXED_ROWS);
}
