// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Downstream Quoin checks for the shared Quire invariants and operations mapping.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "invalid pinned fixtures must fail the integration test"
)]

use ix_trace_rs::trace;
use quire_rs::Registry;
use quire_rs::semantic::{
    AvailabilityState, BundleEntry, BundleIndex, SemanticContext, SemanticSeverity,
    extract_clauses, extract_operations,
};
use serde_json::Value;

fn mapping(name: &str) -> String {
    quire_fixtures::mapping_fixture(name)
}

fn mapping_json(name: &str) -> Value {
    serde_json::from_str(&mapping(name)).unwrap()
}

fn context(path: &str) -> SemanticContext {
    let registry = Registry::load_module(&quire_fixtures::module_ok_dir()).unwrap();
    let module = registry
        .semantic_module("spec-objects-fixture")
        .unwrap()
        .clone();
    let bundle = BundleIndex {
        package: "agent-ix/config-service".to_string(),
        objects: vec![BundleEntry {
            id: "FR-006".to_string(),
            names: vec!["ConfigVersion".to_string()],
        }],
        ..BundleIndex::default()
    };
    SemanticContext::new(module, path, bundle)
        .with_source_identity("ix://agent-ix/config-service/spec")
}

fn artifact(invariants: &str, operations: &str) -> String {
    let mut markdown = String::from(
        "---\nid: FR-901\ntitle: Ops\nobject: entity\n---\n# FR-901\n\n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|---|---|---|---|\n| id | UUID | 1 | identity |\n\n",
    );
    if !invariants.is_empty() {
        markdown.push_str("## Invariants\n\n");
        markdown.push_str(invariants);
        markdown.push('\n');
    }
    if !operations.is_empty() {
        markdown.push_str("\n## Operations\n\n");
        markdown.push_str(operations);
        markdown.push('\n');
    }
    markdown
}

fn case(id: &str) -> Value {
    mapping_json("operations-cases.json")["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|value| value["id"] == id)
        .unwrap_or_else(|| panic!("missing shared operations fixture {id}"))
        .clone()
}

fn line_of(markdown: &str, needle: &str, occurrence: usize) -> usize {
    markdown
        .lines()
        .enumerate()
        .filter(|(_, line)| line.starts_with(needle))
        .nth(occurrence - 1)
        .map(|(index, _)| index + 1)
        .unwrap()
}

fn assert_error_at(id: &str, code: &str, needle: &str, occurrence: usize) {
    let fixture = case(id);
    let markdown = artifact(
        fixture["invariants"].as_str().unwrap_or(""),
        fixture["operations"].as_str().unwrap_or(""),
    );
    let ctx = context("operations-case.md");
    let clauses = extract_clauses(&markdown, &ctx);
    let operations = extract_operations(&markdown, &ctx, clauses.clauses.as_deref().unwrap_or(&[]));
    let diagnostics = clauses
        .diagnostics
        .iter()
        .chain(operations.diagnostics.iter());
    let expected_line = line_of(&markdown, needle, occurrence);
    assert!(
        diagnostics.into_iter().any(|diagnostic| {
            diagnostic.code == code
                && diagnostic.severity == SemanticSeverity::Error
                && diagnostic.line == Some(expected_line)
        }),
        "{id}: expected {code} at {expected_line}"
    );
}

#[test]
#[trace("FR-072-AC-1")]
fn checked_invariant_fences_match_shared_clause_refs_spans_and_text() {
    let markdown = mapping("operations.md");
    let expected = mapping_json("operations.expected.json");
    let clauses = extract_clauses(&markdown, &context("operations.md"));
    assert!(clauses.diagnostics.is_empty(), "{:?}", clauses.diagnostics);
    assert_eq!(clauses.availability.state, AvailabilityState::Available);
    assert!(!clauses.availability.lossy);
    assert_eq!(
        serde_json::to_value(clauses.clauses.as_ref().unwrap()).unwrap(),
        expected["clauses"]
    );
    assert_eq!(
        serde_json::to_value(&clauses.clause_text).unwrap(),
        expected["clauseText"]
    );
    let lines: Vec<&str> = markdown.lines().collect();
    for clause in clauses.clauses.as_ref().unwrap() {
        let span = clause.source_span.as_ref().unwrap();
        assert_eq!(
            lines[span.start_line],
            clauses.clause_text[&clause.clause_id]
        );
        assert_eq!(lines[span.end_line.unwrap() - 1], "```");
    }
}

#[test]
#[trace("FR-072-AC-2")]
fn invalid_fence_languages_fail_and_admitted_unknown_languages_are_advisories() {
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
        let fixture = case(id);
        let markdown = artifact(fixture["invariants"].as_str().unwrap(), "");
        let result = extract_clauses(&markdown, &context("language-case.md"));
        let fence_line = markdown
            .lines()
            .position(|line| line.starts_with("```"))
            .unwrap()
            + 1;
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == code
                    && diagnostic.severity == severity
                    && diagnostic.line == Some(fence_line)),
            "{id}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
#[trace("FR-072-AC-3")]
fn duplicate_clause_ids_and_non_identifier_headings_fail_at_their_loci() {
    assert_error_at(
        "duplicate-clause-id",
        "semantic.duplicate-clause-id",
        "### ",
        2,
    );
    assert_error_at(
        "clause-id-not-identifier",
        "semantic.clause-id-not-identifier",
        "### ",
        1,
    );
}

#[test]
#[trace("FR-072-AC-4")]
fn archive_operation_matches_the_shared_typed_operation_fixture() {
    let markdown = mapping("operations.md");
    let expected = mapping_json("operations.expected.json");
    let ctx = context("operations.md");
    let clauses = extract_clauses(&markdown, &ctx);
    let operations = extract_operations(&markdown, &ctx, clauses.clauses.as_ref().unwrap());
    assert_eq!(operations.availability.state, AvailabilityState::Available);
    assert_eq!(
        serde_json::to_value(operations.operations.as_ref().unwrap()).unwrap(),
        expected["operations"]
    );
}

#[test]
#[trace("FR-072-AC-5")]
fn dangling_post_reference_fails_on_the_post_line() {
    assert_error_at("dangling-post", "semantic.dangling-clause-ref", "Post:", 1);
}

#[test]
#[trace("FR-072-AC-6")]
fn inline_and_external_clause_authorities_fail_at_the_second_occurrence() {
    assert_error_at(
        "inline-and-external",
        "semantic.duplicate-clause-authority",
        "Clause:",
        1,
    );
}

#[test]
#[trace("FR-072-AC-7")]
fn ocl_clause_is_verbatim_available_lossy_and_advisory_once() {
    let fixture = case("fence-ocl-carried");
    let markdown = artifact(fixture["invariants"].as_str().unwrap(), "");
    let result = extract_clauses(&markdown, &context("ocl-case.md"));
    let fence_line = line_of(&markdown, "```ocl", 1);
    assert_eq!(result.availability.state, AvailabilityState::Available);
    assert!(result.availability.lossy);
    let clause = &result.clauses.as_ref().unwrap()[0];
    assert_eq!(clause.language, "ocl");
    assert_eq!(clause.clause_id, "immutable");
    let span = clause.source_span.as_ref().unwrap();
    let span = serde_json::to_value(span).unwrap();
    assert_eq!(span["sourceIdentity"], "ix://agent-ix/config-service/spec");
    assert_eq!(span["path"], "ocl-case.md");
    assert_eq!(span["startLine"], fence_line);
    assert_eq!(span["endLine"], line_of(&markdown, "```", 2));
    assert_eq!(result.clause_text["immutable"], "x");
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].code,
        "semantic.clause-language-unchecked"
    );
    assert_eq!(result.diagnostics[0].severity, SemanticSeverity::Advisory);
    assert_eq!(result.diagnostics[0].line, Some(fence_line));
}
