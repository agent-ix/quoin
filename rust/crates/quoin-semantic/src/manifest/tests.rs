// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

use super::*;
use ix_trace_rs::trace;
use serde_json::json;

#[trace("FR-070")]
#[test]
fn tc_378_060_targets_locus_detection_matches_the_regex() {
    assert!(is_targets_locus("semantic.targets"));
    assert!(is_targets_locus("semantic.targets.0"));
    assert!(is_targets_locus("semantic.targets.12"));
    assert!(!is_targets_locus("semantic.targets.0.x"));
    assert!(!is_targets_locus("semantic.mappings.0"));
    assert!(!is_targets_locus("semantic.targetsx"));
}

#[trace("FR-070")]
#[test]
fn tc_378_061_map_schema_error_names_each_unknown_key_separately() {
    let diagnostic = map_schema_error(&SchemaError {
        instance_path: String::new(),
        keyword: SchemaKeyword::AdditionalProperties,
        params: SchemaErrorParams::AdditionalProperty("nope".to_owned()),
        instance: json!({}),
        message: "irrelevant".to_owned(),
    });
    assert_eq!(diagnostic.code, DiagnosticCode::UnknownKey);
    assert_eq!(diagnostic.path, "semantic.nope");
    assert_eq!(diagnostic.message, "unknown key inside semantic: nope");
}

#[trace("FR-070")]
#[test]
fn tc_378_062_map_schema_error_distinguishes_target_from_value() {
    let target = map_schema_error(&SchemaError {
        instance_path: "/targets/0".to_owned(),
        keyword: SchemaKeyword::Enum,
        params: SchemaErrorParams::AllowedValues(vec![json!("rust")]),
        instance: json!("cobol"),
        message: String::new(),
    });
    assert_eq!(target.code, DiagnosticCode::UnknownTarget);
    assert_eq!(target.path, "semantic.targets.0");
    assert_eq!(
        target.message,
        r#"semantic.targets.0 is "cobol", not one of ["rust"]"#
    );

    let posture = map_schema_error(&SchemaError {
        instance_path: "/compatibility_posture".to_owned(),
        keyword: SchemaKeyword::Enum,
        params: SchemaErrorParams::AllowedValues(vec![json!("strict")]),
        instance: json!("lossy"),
        message: String::new(),
    });
    assert_eq!(posture.code, DiagnosticCode::InvalidValue);
}

#[trace("FR-070")]
#[test]
fn tc_378_063_map_schema_error_distinguishes_package_from_value() {
    let package = map_schema_error(&SchemaError {
        instance_path: "/package".to_owned(),
        keyword: SchemaKeyword::Pattern,
        params: SchemaErrorParams::Pattern("^a$".to_owned()),
        instance: json!("X"),
        message: String::new(),
    });
    assert_eq!(package.code, DiagnosticCode::InvalidPackage);
    assert_eq!(package.message, "semantic.package does not match ^a$");

    let other = map_schema_error(&SchemaError {
        instance_path: "/other".to_owned(),
        keyword: SchemaKeyword::Pattern,
        params: SchemaErrorParams::Pattern("^a$".to_owned()),
        instance: json!("X"),
        message: String::new(),
    });
    assert_eq!(other.code, DiagnosticCode::InvalidValue);
}

#[trace("FR-070")]
#[test]
fn tc_378_064_map_schema_error_falls_through_to_invalid_value() {
    let diagnostic = map_schema_error(&SchemaError {
        instance_path: "/exports/0".to_owned(),
        keyword: SchemaKeyword::MinLength,
        params: SchemaErrorParams::None,
        instance: json!(""),
        message: "is shorter than 1 character".to_owned(),
    });
    assert_eq!(diagnostic.code, DiagnosticCode::InvalidValue);
    assert_eq!(diagnostic.path, "semantic.exports.0");
}

#[trace("FR-070")]
#[test]
fn tc_378_065_has_errors_ignores_warnings() {
    let warning = SemanticDiagnostic::warning(DiagnosticCode::InlineDataSchema, "x", "y");
    let error = SemanticDiagnostic::error(DiagnosticCode::UnknownKey, "x", "y");
    assert!(!has_errors(std::slice::from_ref(&warning)));
    assert!(has_errors(&[warning, error]));
}
