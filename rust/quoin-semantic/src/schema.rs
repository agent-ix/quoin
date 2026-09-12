// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Agent-IX

//! JSON Schema validation, shaped so the ajv-era diagnostic mapping survives.
//!
//! The TypeScript reads `ajv`'s `ErrorObject` — `instancePath`, `keyword`,
//! `params.additionalProperty`, `params.missingProperty`, `params.allowedValues`,
//! `params.pattern`, and (under `verbose: true`) `data`. The Rust `jsonschema`
//! crate reports a different type with a different traversal. This module is the
//! adapter, and it is the one place the two libraries are allowed to differ.
//!
//! Three deliberate reshapings, each measured against the golden corpus and
//! written up in `DIVERGENCE.md`:
//!
//! 1. **`additionalProperties` is fanned out.** `jsonschema` emits one error
//!    carrying every unexpected key; ajv emits one error per key, and
//!    `mapAjvError` turns each into its own `semantic.unknown-key`. Emitting one
//!    diagnostic naming two keys would have silently changed the output for
//!    every manifest with more than one typo.
//! 2. **`anyOf` / `oneOf` sub-errors are flattened.** `jsonschema` nests the
//!    failing branches' errors inside the composite error's `context`; ajv
//!    reports them alongside it. Without flattening, an unknown `targets` entry
//!    loses the `enum` error that names the allowed values.
//! 3. **Order is `(instance location, keyword)`, not traversal order.** ajv
//!    reports in schema-declaration order, `jsonschema` in its own, and the two
//!    `jsonschema` releases measured do not even agree with each other. A
//!    deterministic sort is the only order that is stable under a dependency
//!    bump. Error order is explicitly not contractual for this port.
//!
//! No network and no filesystem read happens here: the crate depends on
//! `jsonschema` with `default-features = false`, which drops both `resolve-http`
//! and `resolve-file`, and every external schema is handed over in memory.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::error::SemanticError;

/// The ajv keyword a validation error is reported under.
///
/// Spellings are ajv's, because they are what the diagnostic mapping and the
/// golden corpus are written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum SchemaKeyword {
    /// `additionalItems`
    AdditionalItems,
    /// `additionalProperties`
    AdditionalProperties,
    /// `anyOf`
    AnyOf,
    /// `const`
    Const,
    /// `contains`
    Contains,
    /// `contentEncoding`
    ContentEncoding,
    /// `contentMediaType`
    ContentMediaType,
    /// `enum`
    Enum,
    /// `exclusiveMaximum`
    ExclusiveMaximum,
    /// `exclusiveMinimum`
    ExclusiveMinimum,
    /// `false schema`
    FalseSchema,
    /// `format`
    Format,
    /// `maxItems`
    MaxItems,
    /// `maximum`
    Maximum,
    /// `maxLength`
    MaxLength,
    /// `maxProperties`
    MaxProperties,
    /// `minItems`
    MinItems,
    /// `minimum`
    Minimum,
    /// `minLength`
    MinLength,
    /// `minProperties`
    MinProperties,
    /// `multipleOf`
    MultipleOf,
    /// `not`
    Not,
    /// `oneOf`
    OneOf,
    /// `pattern`
    Pattern,
    /// `propertyNames`
    PropertyNames,
    /// `required`
    Required,
    /// `type`
    Type,
    /// `unevaluatedItems`
    UnevaluatedItems,
    /// `unevaluatedProperties`
    UnevaluatedProperties,
    /// `uniqueItems`
    UniqueItems,
    /// A condition with no ajv counterpart: a `$ref` that did not resolve, a
    /// regex engine failure, a custom keyword.
    ///
    /// Reaching this is a **port defect or a dependency change**, never ordinary
    /// data: every schema this crate compiles is vendored, and a `$ref` in one
    /// of them that does not resolve is a broken vendored bundle. It maps to
    /// `semantic.invalid-value` so it is reported rather than swallowed.
    Unmappable,
}

impl SchemaKeyword {
    /// The ajv spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AdditionalItems => "additionalItems",
            Self::AdditionalProperties => "additionalProperties",
            Self::AnyOf => "anyOf",
            Self::Const => "const",
            Self::Contains => "contains",
            Self::ContentEncoding => "contentEncoding",
            Self::ContentMediaType => "contentMediaType",
            Self::Enum => "enum",
            Self::ExclusiveMaximum => "exclusiveMaximum",
            Self::ExclusiveMinimum => "exclusiveMinimum",
            Self::FalseSchema => "false schema",
            Self::Format => "format",
            Self::MaxItems => "maxItems",
            Self::Maximum => "maximum",
            Self::MaxLength => "maxLength",
            Self::MaxProperties => "maxProperties",
            Self::MinItems => "minItems",
            Self::Minimum => "minimum",
            Self::MinLength => "minLength",
            Self::MinProperties => "minProperties",
            Self::MultipleOf => "multipleOf",
            Self::Not => "not",
            Self::OneOf => "oneOf",
            Self::Pattern => "pattern",
            Self::PropertyNames => "propertyNames",
            Self::Required => "required",
            Self::Type => "type",
            Self::UnevaluatedItems => "unevaluatedItems",
            Self::UnevaluatedProperties => "unevaluatedProperties",
            Self::UniqueItems => "uniqueItems",
            Self::Unmappable => "unmappable",
        }
    }
}

impl std::fmt::Display for SchemaKeyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The parts of ajv's `params` the diagnostic mapping reads.
///
/// A typed variant per keyword rather than a `Map<String, Value>`: the mapping
/// reads exactly four fields and a map would let a fifth appear untested.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchemaErrorParams {
    /// `additionalProperties`: the one unexpected key this error concerns.
    AdditionalProperty(String),
    /// `required`: the absent key.
    MissingProperty(String),
    /// `enum`: the permitted values, in schema order.
    AllowedValues(Vec<Value>),
    /// `pattern`: the regular expression, as the schema spells it.
    Pattern(String),
    /// Every other keyword carries nothing the mapping reads.
    None,
}

/// One validation failure, in ajv's shape.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaError {
    /// ajv's `instancePath`: a JSON pointer into the validated document, `""`
    /// at the root.
    pub instance_path: String,
    /// The keyword that refused.
    pub keyword: SchemaKeyword,
    /// The typed detail the diagnostic mapping reads.
    pub params: SchemaErrorParams,
    /// ajv's `verbose: true` `data`: the sub-document that failed.
    pub instance: Value,
    /// The validator's own sentence. **Not contractual.**
    pub message: String,
}

impl SchemaError {
    /// The tuple diagnostic parity is measured on.
    #[must_use]
    pub fn identity(&self) -> (&str, SchemaKeyword) {
        (self.instance_path.as_str(), self.keyword)
    }
}

/// A compiled validator over one vendored schema.
pub struct SchemaValidator {
    validator: jsonschema::Validator,
}

impl std::fmt::Debug for SchemaValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SchemaValidator")
    }
}

impl SchemaValidator {
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
    pub fn compile(
        origin: &Path,
        schema: &Value,
        resources: &[(&str, Value)],
    ) -> Result<Self, SemanticError> {
        let mut registry = jsonschema::Registry::new();
        for (uri, document) in resources {
            registry = registry.add(*uri, document.clone()).map_err(|e| {
                SemanticError::VendoredSchemaInvalid {
                    path: origin.to_path_buf(),
                    detail: format!("resource {uri}: {e}"),
                }
            })?;
        }
        let registry = registry
            .prepare()
            .map_err(|e| SemanticError::VendoredSchemaInvalid {
                path: origin.to_path_buf(),
                detail: e.to_string(),
            })?;
        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .with_registry(&registry)
            .build(schema)
            .map_err(|e| SemanticError::VendoredSchemaInvalid {
                path: origin.to_path_buf(),
                detail: e.to_string(),
            })?;
        Ok(Self { validator })
    }

    /// True when `instance` satisfies the schema.
    ///
    /// This is the **verdict**, and the verdict is contractual: it is measured
    /// at exact parity with ajv over the golden corpus.
    #[must_use]
    pub fn is_valid(&self, instance: &Value) -> bool {
        self.validator.is_valid(instance)
    }

    /// Every failure, fanned out and flattened, in `(instance location, keyword)`
    /// order.
    ///
    /// Empty exactly when [`Self::is_valid`] is true.
    #[must_use]
    pub fn errors(&self, instance: &Value) -> Vec<SchemaError> {
        let mut out = Vec::new();
        for error in self.validator.iter_errors(instance) {
            push_error(&error, instance, &mut out);
        }
        out.sort_by(|a, b| {
            (a.instance_path.as_str(), a.keyword).cmp(&(b.instance_path.as_str(), b.keyword))
        });
        out
    }
}

/// Resolve a JSON pointer against `instance`, for ajv's `verbose` `data`.
fn at_pointer(instance: &Value, pointer: &str) -> Value {
    if pointer.is_empty() {
        return instance.clone();
    }
    instance.pointer(pointer).cloned().unwrap_or(Value::Null)
}

/// Flatten one `jsonschema` error into zero or more ajv-shaped errors.
#[allow(clippy::too_many_lines)] // one arm per `jsonschema` error kind; see the `match`.
fn push_error(error: &jsonschema::ValidationError<'_>, root: &Value, out: &mut Vec<SchemaError>) {
    use jsonschema::error::ValidationErrorKind as K;

    let instance_path = error.instance_path().to_string();
    let message = error.to_string();
    let data = at_pointer(root, &instance_path);

    let mut emit = |keyword: SchemaKeyword, params: SchemaErrorParams| {
        out.push(SchemaError {
            instance_path: instance_path.clone(),
            keyword,
            params,
            instance: data.clone(),
            message: message.clone(),
        });
    };

    // Exhaustive on purpose: no `_` arm. A variant added by a `jsonschema`
    // bump must be classified here deliberately, because a catch-all would map
    // a new refusal onto `Unmappable` and hide it behind a generic sentence.
    match error.kind() {
        // (1) fan-out: one ajv error per unexpected key.
        K::AdditionalProperties { unexpected } => {
            for key in unexpected {
                emit(
                    SchemaKeyword::AdditionalProperties,
                    SchemaErrorParams::AdditionalProperty(key.clone()),
                );
            }
        }
        K::UnevaluatedProperties { unexpected } => {
            for key in unexpected {
                emit(
                    SchemaKeyword::UnevaluatedProperties,
                    SchemaErrorParams::AdditionalProperty(key.clone()),
                );
            }
        }
        K::Required { property } => {
            let name = property
                .as_str()
                .map_or_else(|| property.to_string(), str::to_owned);
            emit(
                SchemaKeyword::Required,
                SchemaErrorParams::MissingProperty(name),
            );
        }
        K::Enum { options } => {
            let values = options.as_array().cloned().unwrap_or_default();
            emit(
                SchemaKeyword::Enum,
                SchemaErrorParams::AllowedValues(values),
            );
        }
        K::Pattern { pattern } => {
            emit(
                SchemaKeyword::Pattern,
                SchemaErrorParams::Pattern(pattern.clone()),
            );
        }
        // (2) flatten: report the composite AND each failing branch's errors,
        // which is what ajv does.
        K::AnyOf { context } => {
            emit(SchemaKeyword::AnyOf, SchemaErrorParams::None);
            for branch in context {
                for nested in branch {
                    push_error(nested, root, out);
                }
            }
        }
        K::OneOfNotValid { context } | K::OneOfMultipleValid { context } => {
            emit(SchemaKeyword::OneOf, SchemaErrorParams::None);
            for branch in context {
                for nested in branch {
                    push_error(nested, root, out);
                }
            }
        }
        K::PropertyNames { error: nested } => {
            emit(SchemaKeyword::PropertyNames, SchemaErrorParams::None);
            push_error(nested, root, out);
        }
        K::AdditionalItems { .. } => emit(SchemaKeyword::AdditionalItems, SchemaErrorParams::None),
        K::Constant { .. } => emit(SchemaKeyword::Const, SchemaErrorParams::None),
        K::Contains => emit(SchemaKeyword::Contains, SchemaErrorParams::None),
        K::ContentEncoding { .. } => emit(SchemaKeyword::ContentEncoding, SchemaErrorParams::None),
        K::ContentMediaType { .. } => {
            emit(SchemaKeyword::ContentMediaType, SchemaErrorParams::None);
        }
        K::ExclusiveMaximum { .. } => {
            emit(SchemaKeyword::ExclusiveMaximum, SchemaErrorParams::None);
        }
        K::ExclusiveMinimum { .. } => {
            emit(SchemaKeyword::ExclusiveMinimum, SchemaErrorParams::None);
        }
        K::FalseSchema => emit(SchemaKeyword::FalseSchema, SchemaErrorParams::None),
        K::Format { .. } => emit(SchemaKeyword::Format, SchemaErrorParams::None),
        K::MaxItems { .. } => emit(SchemaKeyword::MaxItems, SchemaErrorParams::None),
        K::Maximum { .. } => emit(SchemaKeyword::Maximum, SchemaErrorParams::None),
        K::MaxLength { .. } => emit(SchemaKeyword::MaxLength, SchemaErrorParams::None),
        K::MaxProperties { .. } => emit(SchemaKeyword::MaxProperties, SchemaErrorParams::None),
        K::MinItems { .. } => emit(SchemaKeyword::MinItems, SchemaErrorParams::None),
        K::Minimum { .. } => emit(SchemaKeyword::Minimum, SchemaErrorParams::None),
        K::MinLength { .. } => emit(SchemaKeyword::MinLength, SchemaErrorParams::None),
        K::MinProperties { .. } => emit(SchemaKeyword::MinProperties, SchemaErrorParams::None),
        K::MultipleOf { .. } => emit(SchemaKeyword::MultipleOf, SchemaErrorParams::None),
        K::Not { .. } => emit(SchemaKeyword::Not, SchemaErrorParams::None),
        K::Type { .. } => emit(SchemaKeyword::Type, SchemaErrorParams::None),
        K::UnevaluatedItems { .. } => {
            emit(SchemaKeyword::UnevaluatedItems, SchemaErrorParams::None);
        }
        K::UniqueItems => emit(SchemaKeyword::UniqueItems, SchemaErrorParams::None),
        // No ajv counterpart. Reported, never swallowed — see `Unmappable`.
        K::BacktrackLimitExceeded { .. }
        | K::RegexEngineFailure { .. }
        | K::Custom { .. }
        | K::FromUtf8 { .. }
        | K::Referencing(_) => emit(SchemaKeyword::Unmappable, SchemaErrorParams::None),
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

/// Count errors by `(instance location, keyword)`, for parity assertions.
#[must_use]
pub fn identity_counts(errors: &[SchemaError]) -> BTreeMap<(String, &'static str), usize> {
    let mut counts = BTreeMap::new();
    for error in errors {
        *counts
            .entry((error.instance_path.clone(), error.keyword.as_str()))
            .or_insert(0) += 1;
    }
    counts
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
