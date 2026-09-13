// SPDX-License-Identifier: AGPL-3.0-or-later
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
//!
//! # `format` asserts exactly when ajv asserted it
//!
//! ajv under `strict: false` treats an unknown `format` as an annotation and
//! asserts one the moment it is registered — `intervention.ts:22-27` passes
//! `formats: {"date-time": isRfc3339DateTime}`, `operational.ts:30-34` calls
//! `ajv.addFormat("date-time", ...)`. The `jsonschema` crate is the other way
//! round: under Draft 2020-12 `format` is annotation-only until
//! `should_validate_formats(true)` is set.
//!
//! So the two constructors differ in exactly that:
//!
//! - [`SchemaValidator::compile_vendored`] registers nothing and asserts
//!   nothing. It is what `quoin-semantic` compiles its bundle with, and it is
//!   byte-for-byte the behaviour that crate's goldens were captured against.
//! - [`SchemaValidator::compile_with_formats`] registers the named checks and
//!   turns assertion on. A schema carrying `format: "date-time"` compiled
//!   without it would silently accept what ajv refused, which is a verdict
//!   difference and therefore a defect, not a nuance.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::error::JsonSchemaError;
use crate::keyword::SchemaKeyword;

/// A named `format` check, in the shape `jsonschema` takes it.
///
/// The name is the schema's spelling (`"date-time"`), and the check is the one
/// the retained ajv instance registered for it.
#[derive(Debug, Clone, Copy)]
pub struct FormatCheck {
    /// The `format` name as the schema spells it.
    pub name: &'static str,
    /// The predicate ajv registered under that name.
    pub check: fn(&str) -> bool,
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
    /// is fetched. No `format` is registered and none is asserted — see the
    /// module documentation.
    ///
    /// # Errors
    ///
    /// [`JsonSchemaError::VendoredSchemaInvalid`] when the schema or one of the
    /// resources does not compile.
    pub fn compile_vendored(
        origin: &Path,
        schema: &Value,
        resources: &[(&str, Value)],
    ) -> Result<Self, JsonSchemaError> {
        Self::build(origin, schema, resources, &[])
    }

    /// Compile `schema` with `formats` registered and `format` assertion on.
    ///
    /// This is the shape the measurement schemas need: ajv asserted
    /// `date-time` because the retained code registered a check for it, and
    /// the `jsonschema` crate asserts nothing until told to.
    ///
    /// # Errors
    ///
    /// [`JsonSchemaError::VendoredSchemaInvalid`] when the schema or one of the
    /// resources does not compile.
    pub fn compile_with_formats(
        origin: &Path,
        schema: &Value,
        resources: &[(&str, Value)],
        formats: &[FormatCheck],
    ) -> Result<Self, JsonSchemaError> {
        Self::build(origin, schema, resources, formats)
    }

    fn build(
        origin: &Path,
        schema: &Value,
        resources: &[(&str, Value)],
        formats: &[FormatCheck],
    ) -> Result<Self, JsonSchemaError> {
        let mut registry = jsonschema::Registry::new();
        for (uri, document) in resources {
            registry = registry.add(*uri, document.clone()).map_err(|e| {
                JsonSchemaError::VendoredSchemaInvalid {
                    path: origin.to_path_buf(),
                    detail: format!("resource {uri}: {e}"),
                }
            })?;
        }
        let registry = registry
            .prepare()
            .map_err(|e| JsonSchemaError::VendoredSchemaInvalid {
                path: origin.to_path_buf(),
                detail: e.to_string(),
            })?;
        let mut options = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .with_registry(&registry);
        if !formats.is_empty() {
            for format in formats {
                let check = format.check;
                options = options.with_format(format.name, move |value: &str| check(value));
            }
            options = options.should_validate_formats(true);
        }
        let validator =
            options
                .build(schema)
                .map_err(|e| JsonSchemaError::VendoredSchemaInvalid {
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
