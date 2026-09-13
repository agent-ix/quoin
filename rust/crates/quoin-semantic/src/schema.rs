// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! JSON Schema validation — re-exported from `quoin-jsonschema`.
//!
//! Everything this module used to define now lives in `quoin-jsonschema`
//! (quoin#470), because the measurement port became the second consumer of the
//! same ajv-parity adapter and a second copy of it would have been the seventh
//! duplicated helper in this workspace. The move was a move: the types, the
//! `jsonschema`-to-ajv error reshaping and the ordering rule are unchanged, and
//! the tests at the bottom of this file are the ones that were here before,
//! running against the re-export without an edit.
//!
//! Read `quoin_jsonschema::validator`'s module documentation for the three
//! reshapings ajv parity needs, and `DIVERGENCE.md` before changing any of them.
//!
//! # What stayed here
//!
//! - [`read_json`], because reading a vendored document off disk is this
//!   crate's boundary and reports this crate's errors.
//! - [`SchemaValidatorExt::compile`], which maps `quoin-jsonschema`'s
//!   crate-local compile refusal onto [`SemanticError::VendoredSchemaInvalid`].
//!   It is an extension trait rather than a wrapper type so that there is still
//!   exactly one `SchemaValidator` in the workspace.

use std::path::Path;

use serde_json::Value;

pub use quoin_jsonschema::{
    SchemaError, SchemaErrorParams, SchemaKeyword, SchemaValidator, identity_counts,
};

use crate::error::SemanticError;

/// Compiling a vendored schema, reported in this crate's error envelope.
///
/// `quoin-jsonschema` refuses with its own error, as a crate below this one
/// must. This is the mapping, and it is the whole of the entanglement the
/// extraction had to break.
pub trait SchemaValidatorExt: Sized {
    /// Compile `schema`, resolving any `$ref` against `resources`.
    ///
    /// `origin` names the vendored file, so a compile failure says which one.
    /// `resources` are `(absolute URI, document)` pairs held in memory; nothing
    /// is fetched.
    ///
    /// # Errors
    ///
    /// [`SemanticError::VendoredSchemaInvalid`] when the schema or one of the
    /// resources does not compile.
    fn compile(
        origin: &Path,
        schema: &Value,
        resources: &[(&str, Value)],
    ) -> Result<Self, SemanticError>;
}

impl SchemaValidatorExt for SchemaValidator {
    fn compile(
        origin: &Path,
        schema: &Value,
        resources: &[(&str, Value)],
    ) -> Result<Self, SemanticError> {
        Self::compile_vendored(origin, schema, resources).map_err(|error| {
            SemanticError::VendoredSchemaInvalid {
                path: origin.to_path_buf(),
                detail: error.to_string(),
            }
        })
    }
}

/// Read a vendored JSON document from disk.
///
/// # Errors
///
/// [`SemanticError::VendoredSchemaUnreadable`] or
/// [`SemanticError::VendoredSchemaNotJson`].
pub fn read_json(path: &Path) -> Result<Value, SemanticError> {
    let bytes = std::fs::read(path).map_err(|source| SemanticError::VendoredSchemaUnreadable {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|source| SemanticError::VendoredSchemaNotJson {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
// Indexing and `unreachable!` are a test-only convenience: an out-of-range
// index in a test is a failing test, not a downed worker.
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use serde_json::json;

    fn validator(schema: &Value) -> SchemaValidator {
        match SchemaValidator::compile(Path::new("test"), schema, &[]) {
            Ok(v) => v,
            Err(e) => unreachable!("test schema must compile: {e}"),
        }
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_030_additional_properties_fan_out_is_one_error_per_key() {
        let v = validator(&json!({
            "type": "object",
            "properties": {"a": {"type": "string"}},
            "additionalProperties": false
        }));
        let errors = v.errors(&json!({"a": "x", "b": 1, "c": 2}));
        assert_eq!(errors.len(), 2, "{errors:#?}");
        let keys: Vec<_> = errors
            .iter()
            .map(|e| match &e.params {
                SchemaErrorParams::AdditionalProperty(k) => k.clone(),
                other => unreachable!("expected AdditionalProperty, got {other:?}"),
            })
            .collect();
        assert_eq!(keys, vec!["b".to_owned(), "c".to_owned()]);
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_031_any_of_reports_the_composite_and_each_branch() {
        let v = validator(&json!({
            "anyOf": [{"enum": ["a", "b"]}, {"enum": ["c"]}]
        }));
        let errors = v.errors(&json!("z"));
        let keywords: Vec<_> = errors.iter().map(|e| e.keyword).collect();
        assert_eq!(
            keywords,
            vec![
                SchemaKeyword::AnyOf,
                SchemaKeyword::Enum,
                SchemaKeyword::Enum
            ],
            "{errors:#?}"
        );
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_032_required_carries_the_missing_property() {
        let v = validator(&json!({"type": "object", "required": ["a", "b"]}));
        let errors = v.errors(&json!({}));
        let mut missing: Vec<_> = errors
            .iter()
            .map(|e| match &e.params {
                SchemaErrorParams::MissingProperty(k) => k.clone(),
                other => unreachable!("expected MissingProperty, got {other:?}"),
            })
            .collect();
        missing.sort();
        assert_eq!(missing, vec!["a".to_owned(), "b".to_owned()]);
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_033_enum_carries_allowed_values_in_schema_order() {
        let v = validator(&json!({"enum": ["z", "a"]}));
        let errors = v.errors(&json!("q"));
        assert_eq!(errors.len(), 1);
        match &errors[0].params {
            SchemaErrorParams::AllowedValues(values) => {
                assert_eq!(values, &vec![json!("z"), json!("a")]);
            }
            other => unreachable!("expected AllowedValues, got {other:?}"),
        }
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_034_pattern_carries_the_regex_as_written() {
        let v = validator(&json!({"type": "string", "pattern": "^[a-z]+$"}));
        let errors = v.errors(&json!("ABC"));
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].params,
            SchemaErrorParams::Pattern("^[a-z]+$".to_owned())
        );
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_035_instance_is_the_failing_sub_document() {
        let v = validator(&json!({
            "type": "object",
            "properties": {"a": {"enum": ["x"]}}
        }));
        let errors = v.errors(&json!({"a": "y"}));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].instance_path, "/a");
        assert_eq!(errors[0].instance, json!("y"));
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_036_error_order_is_deterministic_and_sorted() {
        let v = validator(&json!({
            "type": "object",
            "properties": {
                "z": {"type": "string"},
                "a": {"type": "string"}
            },
            "additionalProperties": false
        }));
        let errors = v.errors(&json!({"z": 1, "a": 1, "q": 1}));
        let paths: Vec<_> = errors.iter().map(|e| e.instance_path.as_str()).collect();
        assert_eq!(paths, vec!["", "/a", "/z"]);
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_037_valid_document_produces_no_errors() {
        let v = validator(&json!({"type": "string"}));
        assert!(v.is_valid(&json!("x")));
        assert!(v.errors(&json!("x")).is_empty());
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_038_external_resource_resolves_from_memory_only() {
        let common = json!({
            "$id": "https://example.test/common.json",
            "$defs": {"name": {"type": "string", "minLength": 1}}
        });
        let schema = json!({
            "$id": "https://example.test/root.json",
            "type": "object",
            "properties": {"n": {"$ref": "common.json#/$defs/name"}}
        });
        let v = SchemaValidator::compile(
            Path::new("test"),
            &schema,
            &[("https://example.test/common.json", common)],
        );
        let v = match v {
            Ok(v) => v,
            Err(e) => unreachable!("must compile: {e}"),
        };
        assert!(v.is_valid(&json!({"n": "ok"})));
        let errors = v.errors(&json!({"n": ""}));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].identity(), ("/n", SchemaKeyword::MinLength));
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_039_unresolvable_ref_is_refused_at_compile_time() {
        let schema = json!({"$ref": "https://absent.test/nope.json"});
        let result = SchemaValidator::compile(Path::new("vendored.json"), &schema, &[]);
        let Err(error) = result else {
            unreachable!("an unresolvable $ref must not compile");
        };
        assert_eq!(
            error.code(),
            crate::error::SemanticErrorCode::VendoredSchemaInvalid
        );
    }
}
