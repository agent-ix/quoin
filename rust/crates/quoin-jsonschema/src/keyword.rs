// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The ajv keyword vocabulary a validation failure is reported under.
//!
//! Spellings are ajv's throughout, because they are what the diagnostic
//! mappings and the golden corpora in this workspace are written in. Nothing
//! here knows about `jsonschema`'s own error kinds — [`crate::validator`] owns
//! that translation.

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
