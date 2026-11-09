// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-071-AC-1 and AC-3 to AC-8, criterion by criterion (PLAT-278).
//!
//! Quire executes the Markdown to `FieldDecl[]` extraction against the
//! `quire-fixtures` goldens. What Quoin owns, and what these assert, is the
//! contract half: every expectation recorded beside a fixture says what FR-071
//! says, every recorded multiplicity is valid semantic-core, and every refusal
//! names a stable code, a severity and a locus. The fixtures are read from
//! `quire-fixtures`; Quoin holds no copy.
//!
//! Not asserted here: that the recorded `TypeRef` targets and constraint
//! declarations validate against the vendored semantic-core 0.3.0 schemas. They
//! do not. The `quire-fixtures` goldens record a tagged `{kind, value}` target
//! and constraint shape, and semantic-core 0.3.0 declares `target` as a plain
//! string and flattens constraints. That is a fixture-owner defect (FR-071-AC-1
//! as written cannot pass until `quire-fixtures` is regenerated), reported on
//! the ticket rather than weakened or asserted-as-broken here.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use std::fs;

use common::{mapping_fixture, mapping_json, schema_dir};
use quoin_semantic::schema::{SchemaValidator, SchemaValidatorExt};
use serde_json::Value;

/// A compiler for one semantic-core schema, with every sibling registered under
/// its `$id` so `$ref`s resolve in memory.
fn semantic_core_validator(name: &str) -> SchemaValidator {
    let dir = schema_dir().join("semantic-core");
    let mut resources: Vec<(String, Value)> = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let document: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let id = document["$id"].as_str().unwrap().to_owned();
            resources.push((id, document));
        }
    }
    let target = dir.join(format!("{name}.json"));
    let schema: Value = serde_json::from_str(&fs::read_to_string(&target).unwrap()).unwrap();
    let borrowed: Vec<(&str, Value)> = resources
        .iter()
        .map(|(id, doc)| (id.as_str(), doc.clone()))
        .collect();
    SchemaValidator::compile(&target, &schema, &borrowed).unwrap()
}

fn assert_valid(validator: &SchemaValidator, what: &str, instance: &Value) {
    let errors = validator.errors(instance);
    assert!(
        errors.is_empty(),
        "{what} is not valid semantic-core: {errors:?}"
    );
}

/// One case of `cell-cases.json` by id.
fn case(id: &str) -> Value {
    let file = mapping_json("cell-cases.json");
    file["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap_or_else(|| panic!("cell-cases.json has no case {id}"))
        .clone()
}

/// The single diagnostic a refusal case expects.
fn only_diagnostic(case: &Value) -> &Value {
    let diagnostics = case["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1, "one refusal per case: {case}");
    &diagnostics[0]
}

fn assert_refusal(case: &Value, code: &str, severity: &str, locus: &str) {
    let diagnostic = only_diagnostic(case);
    assert_eq!(diagnostic["code"], code, "{}", case["id"]);
    assert_eq!(diagnostic["severity"], severity, "{}", case["id"]);
    assert_eq!(diagnostic["locus"], locus, "{}", case["id"]);
}

/// The both-forms fixture carries a typed table and then a `sysml` fence under
/// one `## Properties`, and the recorded expectation is a failure at the
/// second form's line.
///
/// Trace: FR-071-AC-3
#[test]
fn tc_278_003_both_forms_fails_at_the_second_forms_locus() {
    let artifact = mapping_fixture("both-forms.md");
    let lines: Vec<&str> = artifact.lines().collect();
    let line_of = |needle: &str| {
        let index = lines
            .iter()
            .position(|l| l.starts_with(needle))
            .unwrap_or_else(|| panic!("both-forms.md has no line starting {needle}"));
        index + 1
    };
    let heading = line_of("## Properties");
    let table = line_of("| Field | Type | Multiplicity | Constraints |");
    let fence = line_of("```sysml");
    assert!(
        heading < table && table < fence,
        "table first, fence second"
    );

    let expected = mapping_json("both-forms.expected.json");
    assert_eq!(expected["firstForm"]["form"], "typed-table");
    assert_eq!(expected["firstForm"]["line"], table);
    let diagnostic = &expected["expectedDiagnostic"];
    assert_eq!(diagnostic["code"], "semantic.properties-both-forms");
    assert_eq!(diagnostic["severity"], "error");
    assert_eq!(diagnostic["secondForm"], "sysml-fence");
    assert_eq!(
        diagnostic["line"], fence,
        "the failure sits at the second form"
    );
}

/// `Type` tokens resolve as FR-071 orders them, and an unknown token is an
/// advisory finding on a placeholder identity rather than a string.
///
/// Trace: FR-071-AC-4
#[test]
fn tc_278_004_type_cells_resolve_in_the_specified_order() {
    let uuid = case("type-kernel");
    assert_eq!(uuid["cell"]["Type"], "UUID");
    assert_eq!(uuid["expected"]["target"]["kind"], "KernelScalar");
    assert_eq!(uuid["expected"]["target"]["value"], "UUID");

    let decimal = case("type-decimal");
    assert_eq!(decimal["cell"]["Type"], "Decimal(10,2)");
    assert_eq!(decimal["expected"]["target"]["value"], "Decimal");
    assert_eq!(decimal["expected"]["decimal"]["precision"], 10);
    assert_eq!(decimal["expected"]["decimal"]["scale"], 2);

    let unit = case("type-unit");
    assert_eq!(unit["cell"]["Type"], "Duration [ms]");
    assert_eq!(unit["expected"]["target"]["value"], "Duration");
    assert_eq!(unit["expected"]["unit"], "ms");

    let by_title = case("type-object-by-title");
    assert_eq!(by_title["cell"]["Type"], "ConfigOverlay");
    assert_eq!(by_title["bundle"]["objects"][0]["title"], "ConfigOverlay");
    assert_eq!(
        by_title["expected"]["target"]["value"],
        "ix://agent-ix/config-service/type/ConfigOverlay"
    );

    let by_id = case("type-enum-by-id");
    assert_eq!(by_id["cell"]["Type"], "Status");
    assert_eq!(by_id["bundle"]["enumerations"][0]["id"], "Status");
    assert_eq!(
        by_id["expected"]["target"]["value"],
        "ix://agent-ix/config-service/type/Status"
    );

    let import = case("type-import");
    assert_eq!(
        import["imports"]["agent-ix/spec-artifacts-iso"]["exports"][0],
        import["cell"]["Type"]
    );
    assert_eq!(
        import["expected"]["target"]["value"],
        "ix://agent-ix/spec-artifacts-iso/type/Requirement"
    );

    let mystery = case("type-unresolved");
    assert_eq!(mystery["cell"]["Type"], "Mystery");
    assert_eq!(
        mystery["expected"]["target"]["value"], "ix://agent-ix/config-service/unresolved/Mystery",
        "the placeholder identity, never a string"
    );
    assert_refusal(&mystery, "semantic.unresolved-type", "advisory", "row");
}

/// Multiplicity cells map to the expected `Multiplicity` objects; an inverted
/// range and a flag on a single value are errors with a row locus.
///
/// Trace: FR-071-AC-5
#[test]
fn tc_278_005_multiplicity_cells_map_and_invalid_cells_are_errors() {
    let multiplicity = semantic_core_validator("Multiplicity");
    let expected = |id: &str| {
        let c = case(id);
        let m = c["expected"]["multiplicity"].clone();
        assert_valid(&multiplicity, id, &m);
        (c["cell"]["Multiplicity"].as_str().unwrap().to_owned(), m)
    };

    let (cell, one) = expected("mult-one");
    assert_eq!(cell, "1");
    assert_eq!(
        (&one["lower"], &one["upper"]),
        (&Value::from(1), &Value::from(1))
    );

    let (cell, optional) = expected("mult-optional");
    assert_eq!(cell, "0..1");
    assert_eq!(
        (&optional["lower"], &optional["upper"]),
        (&Value::from(0), &Value::from(1))
    );

    let (cell, many) = expected("mult-many-flags");
    assert_eq!(cell, "1..* ordered unique");
    assert_eq!(many["lower"], 1);
    assert!(many.get("upper").is_none(), "an absent upper is unbounded");
    assert_eq!(
        (&many["ordered"], &many["unique"]),
        (&Value::Bool(true), &Value::Bool(true))
    );

    let (cell, range) = expected("mult-range");
    assert_eq!(cell, "2..5");
    assert_eq!(
        (&range["lower"], &range["upper"]),
        (&Value::from(2), &Value::from(5))
    );

    let (cell, empty) = expected("mult-empty");
    assert_eq!(cell, "");
    assert_eq!(empty, one, "an empty cell means 1");

    for id in ["mult-inverted", "mult-flag-on-single"] {
        let refused = case(id);
        assert!(
            refused.get("expected").is_none(),
            "{id} yields no multiplicity"
        );
        assert_refusal(&refused, "semantic.invalid-multiplicity", "error", "row");
    }
    assert_eq!(case("mult-inverted")["cell"]["Multiplicity"], "5..2");
    assert_eq!(
        case("mult-flag-on-single")["cell"]["Multiplicity"],
        "1 ordered"
    );
}

/// Constraint cells map to the closed IR v1.1 constraint models and the
/// `identity` flag; a keyword outside the vocabulary is an error with locus.
///
/// Trace: FR-071-AC-6
#[test]
fn tc_278_006_constraint_cells_map_to_the_closed_vocabulary() {
    let kinds = |id: &str| -> Vec<String> {
        let c = case(id);
        let list = c["expected"]["constraints"].as_array().unwrap();
        list.iter()
            .map(|item| item["kind"].as_str().unwrap().to_owned())
            .collect()
    };

    assert_eq!(
        kinds("con-min-maxlen-identity"),
        ["MinConstraint", "MaxLengthConstraint"]
    );
    let first = case("con-min-maxlen-identity");
    assert_eq!(
        first["cell"]["Constraints"],
        "min: 1, maxLength: 64, identity"
    );
    assert_eq!(
        first["expected"]["identity"], true,
        "identity is a field flag"
    );

    assert_eq!(kinds("con-pattern"), ["PatternConstraint"]);
    assert_eq!(
        case("con-pattern")["expected"]["constraints"][0]["value"]["regex"],
        "^[a-z]+$"
    );
    assert_eq!(kinds("con-enum-values"), ["EnumValuesConstraint"]);
    assert_eq!(
        case("con-enum-values")["expected"]["constraints"][0]["value"]["values"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(kinds("con-non-empty"), ["NonEmptyConstraint"]);
    assert_eq!(kinds("con-format"), ["FormatConstraint"]);
    assert_eq!(
        case("con-format")["expected"]["constraints"][0]["value"]["name"],
        "agent-ix:email"
    );

    let unknown = case("con-unknown-keyword");
    assert_eq!(unknown["cell"]["Constraints"], "mnimum: 1");
    assert_refusal(
        &unknown,
        "semantic.unknown-constraint-keyword",
        "error",
        "row",
    );
}

/// Fence lines outside the declarations-only subset are errors at the line.
///
/// Trace: FR-071-AC-7
#[test]
fn tc_278_007_fence_lines_outside_the_subset_are_errors_with_locus() {
    for (id, line) in [
        ("fence-item", "item x : Y;"),
        ("fence-part-def", "part def X;"),
        ("fence-specializes", ":> Y"),
    ] {
        let refused = case(id);
        assert_eq!(refused["fenceLine"], line);
        assert_refusal(
            &refused,
            "semantic.sysml-outside-subset",
            "error",
            "fence-line",
        );
    }
}

/// A row violating a semantic-core reader rule yields that rule's diagnostic at
/// the row, under the rule's own code.
///
/// Trace: FR-071-AC-8
#[test]
fn tc_278_008_a_reader_rule_violation_is_reported_at_the_row() {
    let refused = case("reader-rule-bare-decimal");
    assert_eq!(refused["cell"]["Type"], "Decimal");
    assert_refusal(
        &refused,
        "agent-ix.semantic-core.MISSING_DECIMAL_POLICY",
        "error",
        "row",
    );
}
