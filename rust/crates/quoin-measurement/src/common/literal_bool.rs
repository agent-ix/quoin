// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A boolean field that discriminates a union by its value.
//!
//! `operational-types.ts:59-66` discriminates a capability's clock support on
//! `supported: false` versus `supported: true`. serde tags unions on strings,
//! not on booleans, so the discriminant is carried in the type instead: a
//! [`LiteralBool<true>`] refuses to deserialize from `false` and there is no
//! constructor that produces a mismatched one. Holding the type is the proof
//! the field said what the variant claims — the plan's §13.3 rule 2, applied to
//! the one place in these four files where the tag is not a string.

use core::fmt;
use core::marker::PhantomData;

use serde::de::{Error as DeError, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A boolean field whose only accepted value is `VALUE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct LiteralBool<const VALUE: bool>(PhantomData<()>);

impl<const VALUE: bool> LiteralBool<VALUE> {
    /// The only inhabitant.
    pub const VALUE: Self = Self(PhantomData);
}

impl<const VALUE: bool> fmt::Display for LiteralBool<VALUE> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(if VALUE { "true" } else { "false" })
    }
}

impl<const VALUE: bool> Serialize for LiteralBool<VALUE> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(VALUE)
    }
}

impl<'de, const VALUE: bool> Deserialize<'de> for LiteralBool<VALUE> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let observed = bool::deserialize(deserializer)?;
        if observed == VALUE {
            Ok(Self::VALUE)
        } else {
            Err(D::Error::invalid_value(
                Unexpected::Bool(observed),
                &if VALUE { "true" } else { "false" },
            ))
        }
    }
}
