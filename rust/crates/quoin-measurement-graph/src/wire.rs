// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The four zod shapes serde does not spell the same way, said once.
//!
//! Every one of these exists because the obvious serde spelling accepts
//! something zod refuses, which would be a silent verdict divergence rather
//! than a nuance:
//!
//! | zod | the obvious serde spelling | what it would wrongly accept |
//! | --- | --- | --- |
//! | `.optional()` | `Option<T>` | an explicit `null` |
//! | `.nullable()` | `Option<T>` | an **absent** member |
//! | `z.literal("x")` | `String` | every other string |
//! | `z.literal(1)` | `u64` | every other number |
//!
//! zod's `.optional()` admits *absence*, and its `.nullable()` admits *null*;
//! they are different modifiers and the retained file uses both. serde's
//! `Option<T>` admits either, so neither maps onto it directly.

use serde::{Deserialize, Deserializer};

/// A `z.string().nullable()` member: required to be present, allowed to be null.
///
/// A newtype rather than a bare `Option<String>` because serde's derive treats
/// a struct field of type `Option<T>` as defaulting to `None` when the member
/// is absent — which is zod's `.optional()`, not its `.nullable()`.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize)]
#[serde(transparent)]
pub struct NullableText(Option<String>);

impl NullableText {
    /// The text, when the member was not null.
    #[must_use]
    pub fn as_deref(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

/// A `.optional()` member: absent is `None`, and an explicit `null` is refused.
///
/// Used as `#[serde(default, deserialize_with = "absent_or")]`. The `default`
/// supplies `None` for an absent member; when the member *is* present, `T`
/// deserializes it directly, so `null` fails on `T` exactly as zod fails it.
///
/// # Errors
///
/// Whatever `T` refuses, including `null`.
pub fn absent_or<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// Declare a `z.literal("…")` as a type with exactly one inhabitant.
macro_rules! string_literal {
    ($name:ident, $spelling:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Deserialize)]
        #[serde(try_from = "String")]
        pub struct $name;

        impl $name {
            /// The one accepted spelling.
            pub const SPELLING: &'static str = $spelling;
        }

        impl TryFrom<String> for $name {
            type Error = crate::scalars::MalformedScalar;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value == $spelling {
                    Ok(Self)
                } else {
                    Err(crate::scalars::malformed(format!(
                        "expected `{}`, got `{value}`",
                        $spelling
                    )))
                }
            }
        }
    };
}

/// Declare a `z.literal(<integer>)` as a type with exactly one inhabitant.
macro_rules! integer_literal {
    ($name:ident, $value:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, serde::Deserialize)]
        #[serde(try_from = "u64")]
        pub struct $name;

        impl $name {
            /// The one accepted value.
            pub const VALUE: u64 = $value;
        }

        impl TryFrom<u64> for $name {
            type Error = crate::scalars::MalformedScalar;

            fn try_from(value: u64) -> Result<Self, Self::Error> {
                if value == $value {
                    Ok(Self)
                } else {
                    Err(crate::scalars::malformed(format!(
                        "expected {}, got {value}",
                        $value
                    )))
                }
            }
        }
    };
}

string_literal!(
    QuireAssuranceFormat,
    "quire-assurance",
    "`format: \"quire-assurance\"`."
);
string_literal!(
    AvailableLiteral,
    "available",
    "`availability: \"available\"` on a declared relation kind."
);
string_literal!(
    GraphQualityRecordType,
    "graph_quality_observation",
    "`record_type: \"graph_quality_observation\"`."
);
string_literal!(
    MeasurementPlanRef,
    "ix://agent-ix/quire-code-rs/MP-001",
    "The one measurement plan this adapter transcribes against."
);
string_literal!(
    GraphQualityDefinitionVersion,
    "quire-code.graph-quality-v1",
    "The one definition version this adapter transcribes against."
);
string_literal!(
    VerificationStackVersion,
    "verification-stack-attestation-v1",
    "`schemaVersion` on a verification-stack attestation."
);
string_literal!(
    ReleaseProfile,
    "release",
    "`buildProfile: \"release\"`: a debug build is not an attestation."
);
string_literal!(
    CleanSourceState,
    "clean",
    "`sourceState: \"clean\"`: a dirty tree is not an attestation."
);

integer_literal!(FormatVersion1, 1, "`format_version: 1`.");
integer_literal!(SchemaVersion1, 1, "`schema_version: 1`.");
