// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The I-JSON value model the canonical serializers operate on.
//!
//! Two refusals of the TypeScript oracle are removed here by construction
//! rather than by a runtime check:
//!
//! * **Non-finite numbers.** [`JsonNumber`] cannot hold one, so neither
//!   canonical serializer needs a `Result`.
//! * **Lone surrogates.** A Rust `String` is well-formed UTF-8, so an
//!   unpaired surrogate cannot reach a serializer. It is refused at the
//!   parser, where it can actually occur (`\uD800` in the source text).

use std::collections::BTreeMap;

use crate::error::StoreError;

/// A finite IEEE-754 double — the only numeric type I-JSON admits.
///
/// Quoin's oracle parses *every* JSON number into a double, including integers,
/// and re-serializes from that double. Integer precision beyond 2^53 is
/// therefore already lost in the retained evidence; this type reproduces that
/// behaviour deliberately rather than improving on it. See `COMPATIBILITY.md`.
#[derive(Clone, Copy, Debug)]
pub struct JsonNumber(f64);

impl JsonNumber {
    /// Wrap a double, refusing NaN and both infinities.
    ///
    /// # Errors
    ///
    /// Refuses NaN and both infinities: neither has a canonical JSON spelling.
    pub fn new(value: f64) -> Result<Self, StoreError> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(StoreError::JsonNumberNotFinite {
                literal: value.to_string(),
                offset: 0,
            })
        }
    }

    /// The wrapped double. Always finite.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl PartialEq for JsonNumber {
    /// Equality is on the *canonical serialization*, not on the bit pattern.
    ///
    /// `0.0` and `-0.0` are distinct doubles that canonicalize to the same
    /// `0`, and a digest cannot distinguish them; treating them as unequal
    /// here would make equality disagree with identity.
    fn eq(&self, other: &Self) -> bool {
        crate::json::number::format_number(self.0) == crate::json::number::format_number(other.0)
    }
}

impl Eq for JsonNumber {}

/// A JSON object: member names to values, with duplicate names refused.
///
/// Backed by a `BTreeMap` because neither canonical serializer preserves
/// insertion order — JCS sorts by UTF-16 code unit, and the evidence store's
/// pretty form uses ECMAScript own-property order. Storing insertion order
/// would be storing a fact no output reads.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JsonObject {
    members: BTreeMap<String, JsonValue>,
}

impl JsonObject {
    /// An object with no members.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a member, refusing a name already present.
    ///
    /// # Errors
    ///
    /// Refuses a member name already present. A duplicate name has no canonical
    /// ordering, so it is a defect in the caller rather than a value to store.
    pub fn insert(&mut self, name: impl Into<String>, value: JsonValue) -> Result<(), StoreError> {
        let name = name.into();
        if self.members.contains_key(&name) {
            return Err(StoreError::JsonDuplicateName { name, offset: 0 });
        }
        self.members.insert(name, value);
        Ok(())
    }

    /// Insert a member, replacing any value already stored under that name.
    pub fn set(&mut self, name: impl Into<String>, value: JsonValue) -> Option<JsonValue> {
        self.members.insert(name.into(), value)
    }

    /// Remove a member, returning it.
    pub fn remove(&mut self, name: &str) -> Option<JsonValue> {
        self.members.remove(name)
    }

    /// The value stored under `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&JsonValue> {
        self.members.get(name)
    }

    /// Whether `name` is present.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.members.contains_key(name)
    }

    /// Member count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Whether the object has no members.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Members in Rust `str` order. Neither canonical serializer uses this
    /// order directly; both re-order for their own rule.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &JsonValue)> {
        self.members.iter()
    }

    /// Member names in Rust `str` order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.members.keys().map(String::as_str)
    }
}

impl<'a> IntoIterator for &'a JsonObject {
    type Item = (&'a String, &'a JsonValue);
    type IntoIter = std::collections::btree_map::Iter<'a, String, JsonValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.members.iter()
    }
}

/// An I-JSON value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JsonValue {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A finite double.
    Number(JsonNumber),
    /// A well-formed Unicode string.
    String(String),
    /// An ordered sequence.
    Array(Vec<JsonValue>),
    /// A name/value map.
    Object(JsonObject),
}

impl JsonValue {
    /// The JSON type name, for diagnostics.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "boolean",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::Array(_) => "array",
            Self::Object(_) => "object",
        }
    }

    /// Build a number value, refusing a non-finite double.
    ///
    /// # Errors
    ///
    /// As [`JsonNumber::new`]: refuses NaN and both infinities.
    pub fn number(value: f64) -> Result<Self, StoreError> {
        JsonNumber::new(value).map(Self::Number)
    }

    /// Build a string value.
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    /// Borrow as an object, or refuse.
    ///
    /// # Errors
    ///
    /// Refuses any value that is not an object, naming the type found.
    pub fn as_object(&self) -> Result<&JsonObject, StoreError> {
        match self {
            Self::Object(object) => Ok(object),
            other => Err(StoreError::NotAnObject {
                found: other.type_name(),
            }),
        }
    }

    /// Borrow as a string, if it is one.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    /// The wrapped double, if this is a number.
    #[must_use]
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(number.get()),
            _ => None,
        }
    }
}
