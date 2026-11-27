// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-072's Quoin-side contract checks consume the mapping fixtures owned by
//! the pinned Quire engine dependency.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "these test assertions intentionally fail by panicking"
)]

use ix_trace_rs::trace;
use quire_rs::Registry;
use quire_rs::semantic::{
    AvailabilityState, BundleEntry, BundleIndex, SemanticContext, SemanticSeverity,
    extract_clauses, extract_operations,
};
use serde_json::{Value, json};

fn context(path: &str) -> SemanticContext {
    let registry = Registry::load_module(&quire_fixtures::module_ok_dir())
        .expect("the shared valid module fixture loads");
    let module = registry
        .semantic_module("spec-objects-fixture")
        .expect("the fixture declares its semantic module")
        .clone();
    let bundle = BundleIndex {
        package: "agent-ix/config-service".to_owned(),
        objects: vec![
            BundleEntry {
                id: "FR-006".to_owned(),
                names: vec!["ConfigVersion".to_owned()],
            },
            BundleEntry {
                id: "FR-005".to_owned(),
                names: vec!["ConfigOverlay".to_owned()],
            },
        ],
        ..BundleIndex::default()
    };
    SemanticContext::new(module, path, bundle)
        .with_source_identity("ix://agent-ix/config-service/spec")
}

fn case(id: &str) -> Value {
    quire_fixtures::mapping_json("operations-cases.json")["cases"]
        .as_array()
        .expect("case list")
        .iter()
        .find(|case| case["id"] == id)
        .unwrap_or_else(|| panic!("fixture case {id} exists"))
        .clone()
}

fn invariants_document(section: &str) -> String {
    format!("# Mapping fixture\n\n## Invariants\n\n{section}")
}

fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|line| line == needle)
        .map(|index| index + 1)
        .unwrap_or_else(|| panic!("line {needle:?} exists in fixture"))
}

#[trace("FR-072-AC-1")]
#[test]
fn quire_clauses_match_the_shared_golden_and_retain_verbatim_text_and_source_spans() {
    let raw = quire_fixtures::mapping_fixture("operations.md");
    let expected = quire_fixtures::mapping_json("operations.expected.json");
    let clauses = extract_clauses(&raw, &context("operations.md"));

    assert_eq!(clauses.availability.state, AvailabilityState::Available);
    assert!(!clauses.availability.lossy);
    assert!(clauses.diagnostics.is_empty(), "{:?}", clauses.diagnostics);
    assert_eq!(
        serde_json::to_value(clauses.clauses.as_ref().expect("clauses available")).unwrap(),
        expected["clauses"]
    );
    assert_eq!(
        serde_json::to_value(&clauses.clause_text).unwrap(),
        expected["clauseText"]
    );
    for clause in clauses.clauses.as_ref().expect("clauses available") {
        let span = clause.source_span.as_ref().expect("source span");
        assert_eq!(
            clauses.clause_text[&clause.clause_id],
            raw.lines()
                .skip(span.start_line)
                .take(span.end_line.expect("closed fence") - span.start_line - 1)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

#[trace("FR-072-AC-2")]
#[test]
fn clause_language_fences_refuse_missing_and_bare_languages_and_advice_on_open_languages() {
    for (id, code, severity) in [
        (
            "fence-no-language",
            "semantic.clause-language-missing",
            SemanticSeverity::Error,
        ),
        (
            "fence-bare-unknown",
            "semantic.clause-language-invalid",
            SemanticSeverity::Error,
        ),
        (
            "fence-sysml-advisory",
            "semantic.clause-language-unchecked",
            SemanticSeverity::Advisory,
        ),
        (
            "fence-fretish-advisory",
            "semantic.clause-language-unchecked",
            SemanticSeverity::Advisory,
        ),
        (
            "fence-namespaced-advisory",
            "semantic.clause-language-unchecked",
            SemanticSeverity::Advisory,
        ),
    ] {
        let source = case(id)["invariants"]
            .as_str()
            .expect("invariants fragment")
            .to_owned();
        let raw = invariants_document(&source);
        let fence_line = raw
            .lines()
            .position(|line| line.starts_with("```"))
            .map(|line| line + 1)
            .expect("fence line");
        let outcome = extract_clauses(&raw, &context("languages.md"));
        assert_eq!(
            outcome.diagnostics.len(),
            1,
            "{id}: {:?}",
            outcome.diagnostics
        );
        assert_eq!(outcome.diagnostics[0].code, code, "{id}");
        assert_eq!(outcome.diagnostics[0].severity, severity, "{id}");
        assert_eq!(outcome.diagnostics[0].line, Some(fence_line), "{id}");
    }
}

#[trace("FR-072-AC-3")]
#[test]
fn duplicate_clause_ids_and_non_identifier_headings_fail_at_their_declarations() {
    let duplicate = invariants_document(
        case("duplicate-clause-id")["invariants"]
            .as_str()
            .expect("duplicate case fragment"),
    );
    let outcome = extract_clauses(&duplicate, &context("duplicate.md"));
    let second_heading = duplicate
        .match_indices("### immutable")
        .nth(1)
        .map(|(offset, _)| duplicate[..offset].lines().count() + 1)
        .expect("second duplicate heading");
    assert_eq!(outcome.diagnostics.len(), 1, "{:?}", outcome.diagnostics);
    assert_eq!(outcome.diagnostics[0].code, "semantic.duplicate-clause-id");
    assert_eq!(outcome.diagnostics[0].line, Some(second_heading));

    let invalid = invariants_document(
        case("clause-id-not-identifier")["invariants"]
            .as_str()
            .expect("invalid id fragment"),
    );
    let outcome = extract_clauses(&invalid, &context("invalid-id.md"));
    assert_eq!(outcome.diagnostics.len(), 1, "{:?}", outcome.diagnostics);
    assert_eq!(
        outcome.diagnostics[0].code,
        "semantic.clause-id-not-identifier"
    );
    assert_eq!(
        outcome.diagnostics[0].line,
        Some(line_of(&invalid, "### not-archived"))
    );
}

#[trace("FR-072-AC-4")]
#[test]
fn operation_declaration_matches_the_shared_two_parameter_golden() {
    let raw = quire_fixtures::mapping_fixture("operations.md");
    let expected = quire_fixtures::mapping_json("operations.expected.json");
    let ctx = context("operations.md");
    let clauses = extract_clauses(&raw, &ctx);
    let operations = extract_operations(
        &raw,
        &ctx,
        clauses.clauses.as_deref().expect("clause declarations"),
    );

    assert_eq!(operations.availability.state, AvailabilityState::Available);
    assert!(
        operations.diagnostics.is_empty(),
        "{:?}",
        operations.diagnostics
    );
    assert_eq!(
        serde_json::to_value(
            operations
                .operations
                .as_ref()
                .expect("operations available")
        )
        .unwrap(),
        expected["operations"]
    );
    let operation = &operations
        .operations
        .as_ref()
        .expect("operations available")[0];
    assert_eq!(operation.name, "archive");
    assert_eq!(operation.params.len(), 2);
    assert_eq!(
        operation
            .returns
            .as_ref()
            .and_then(|value| value.multiplicity.as_ref())
            .map(|multiplicity| multiplicity.lower),
        Some(1)
    );
    assert_eq!(operation.pre[0].clause_id, "notArchived");
    assert_eq!(operation.post[0].clause_id, "archived");
}

#[trace("FR-072-AC-5")]
#[test]
fn dangling_post_clause_is_reported_on_its_post_line() {
    let fixture = case("dangling-post");
    let fragment = fixture["operations"]
        .as_str()
        .expect("dangling post fragment");
    let raw = format!("# Mapping fixture\n\n## Operations\n\n{fragment}");
    let outcome = extract_operations(&raw, &context("dangling.md"), &[]);
    let post_line = line_of(&raw, "Post: missing");
    assert_eq!(outcome.diagnostics.len(), 1, "{:?}", outcome.diagnostics);
    assert_eq!(outcome.diagnostics[0].code, "semantic.dangling-clause-ref");
    assert_eq!(outcome.diagnostics[0].line, Some(post_line));
}

#[trace("FR-072-AC-6")]
#[test]
fn a_fence_and_external_reference_for_one_clause_fail_at_the_second_authority() {
    let fixture = case("inline-and-external");
    let source = fixture["invariants"]
        .as_str()
        .expect("duplicate authority fragment");
    let raw = invariants_document(source);
    let outcome = extract_clauses(&raw, &context("duplicate-authority.md"));
    let reference_line = line_of(&raw, "Clause: ./clauses.md#immutable");
    assert_eq!(outcome.diagnostics.len(), 1, "{:?}", outcome.diagnostics);
    assert_eq!(
        outcome.diagnostics[0].code,
        "semantic.duplicate-clause-authority"
    );
    assert_eq!(outcome.diagnostics[0].line, Some(reference_line));
}

#[trace("FR-072-AC-7")]
#[test]
fn unchecked_ocl_clause_is_carried_verbatim_as_available_and_lossy() {
    let fixture = case("fence-ocl-carried");
    let source = fixture["invariants"]
        .as_str()
        .expect("ocl fixture fragment");
    let raw = invariants_document(source);
    let fence_line = raw
        .lines()
        .position(|line| line == "```ocl")
        .map(|line| line + 1)
        .expect("ocl fence");
    let outcome = extract_clauses(&raw, &context("ocl.md"));

    assert_eq!(outcome.availability.state, AvailabilityState::Available);
    assert!(outcome.availability.lossy);
    let clauses = outcome.clauses.as_ref().expect("carried clause");
    assert_eq!(clauses.len(), 1);
    assert_eq!(clauses[0].language, "ocl");
    assert_eq!(clauses[0].clause_id, "immutable");
    assert_eq!(outcome.clause_text["immutable"], "x");
    assert_eq!(outcome.diagnostics.len(), 1, "{:?}", outcome.diagnostics);
    assert_eq!(
        outcome.diagnostics[0].code,
        "semantic.clause-language-unchecked"
    );
    assert_eq!(outcome.diagnostics[0].severity, SemanticSeverity::Advisory);
    assert_eq!(outcome.diagnostics[0].line, Some(fence_line));
    assert_eq!(
        serde_json::to_value(&outcome.diagnostics[0]).unwrap(),
        json!({
            "code": "semantic.clause-language-unchecked",
            "severity": "advisory",
            "message": "clause immutable: language ocl is carried unchecked",
            "line": fence_line,
            "column": 1
        })
    );
}
