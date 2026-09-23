// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading the closed shapes out of an I-JSON value.
//!
//! A faithful port of the validator vocabulary at
//! `src/change-assurance/records.ts:515-593` — `object`, `exact`, `array`,
//! `nonempty`, `identity`, `digest`, `stringArray`, `unique`, `sorted`,
//! `oneOf`, `equal` and `compareUtf16`. It is in one module because the oracle
//! exports the same set to `attestations.ts` and `verify.ts`, and a port that
//! copied it three times would be three shapes that agree by coincidence.
//!
//! **The member-name ordering is UTF-16, not UTF-8.** `compareUtf16` compares
//! JavaScript strings, and a JavaScript string orders by UTF-16 code unit.
//! Rust's `str` ordering is by Unicode scalar, and the two disagree for every
//! pair that straddles `U+E000`-`U+FFFF` against an astral character. Every
//! `sorted` check in the oracle uses `compareUtf16`, so [`compare_utf16`] is
//! what implements them here.

use std::cmp::Ordering;

use quoin_store::{CanonicalDigest, JsonObject, JsonValue, RawBytesDigest};

use crate::error::{ChangeAssuranceError, FieldFailure, Subject};
use crate::ids::{ArtifactDigest, EventHash, Identity, NonEmptyText};

/// Compare two strings by UTF-16 code unit, as JavaScript's `<` does.
///
/// Not `str::cmp`. Rust orders by Unicode scalar value, JavaScript by UTF-16
/// code unit, and the two disagree whenever one side has a character above
/// `U+FFFF` and the other has one in `U+E000`-`U+FFFF`: `"\u{10000}" <
/// "\u{ffff}"` is true in JavaScript and false in Rust. Every ordering the
/// schema asserts is the oracle's, so it is this one.
#[must_use]
pub fn compare_utf16(left: &str, right: &str) -> Ordering {
    let mut left_units = left.encode_utf16();
    let mut right_units = right.encode_utf16();
    loop {
        match (left_units.next(), right_units.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left_unit), Some(right_unit)) => {
                if left_unit != right_unit {
                    return left_unit.cmp(&right_unit);
                }
            }
        }
    }
}

/// Sort in place by UTF-16 code unit, as `Array.prototype.sort` does.
pub fn sort_utf16(values: &mut [String]) {
    values.sort_by(|left, right| compare_utf16(left, right));
}

/// A closed object being read field by field.
///
/// Holds the subject so every refusal it raises is already attributed, which
/// is what keeps the classification in `verify` a `match` rather than a
/// reconstruction.
#[derive(Clone, Copy)]
pub struct Fields<'a> {
    object: &'a JsonObject,
    subject: Subject,
    prefix: &'a str,
}

impl<'a> Fields<'a> {
    /// Read `value` as an object of `subject`, refusing any other type.
    ///
    /// # Errors
    ///
    /// [`ChangeAssuranceError::Shape`] naming `field` when `value` is not an
    /// object.
    pub fn read(
        value: &'a JsonValue,
        subject: Subject,
        field: &'a str,
    ) -> Result<Self, ChangeAssuranceError> {
        match value {
            JsonValue::Object(object) => Ok(Self {
                object,
                subject,
                prefix: field,
            }),
            _ => Err(ChangeAssuranceError::shape(
                subject,
                FieldFailure::malformed(format!("{field} object")),
            )),
        }
    }

    /// The subject these fields belong to.
    #[must_use]
    pub const fn subject(self) -> Subject {
        self.subject
    }

    /// The object being read.
    #[must_use]
    pub const fn object(self) -> &'a JsonObject {
        self.object
    }

    /// Refuse any member outside `allowed`, and any member of `allowed`
    /// absent.
    ///
    /// # Errors
    ///
    /// [`FieldFailure::Extra`] for an undeclared member, then
    /// [`FieldFailure::Missing`] for an absent one.
    pub fn exact(self, allowed: &[&str]) -> Result<(), ChangeAssuranceError> {
        for name in self.object.names() {
            if !allowed.contains(&name) {
                return Err(ChangeAssuranceError::shape(
                    self.subject,
                    FieldFailure::extra(name),
                ));
            }
        }
        for name in allowed {
            if !self.object.contains(name) {
                return Err(ChangeAssuranceError::shape(
                    self.subject,
                    FieldFailure::missing(*name),
                ));
            }
        }
        Ok(())
    }

    /// [`Fields::exact`], except `optional` names may be present or absent
    /// without either counting as an error.
    ///
    /// For a member added to a shape after documents were already sealed
    /// under it (PLAT-1015): a document sealed before the addition carries
    /// neither name, and a document sealed after always carries both, so
    /// `required` alone would refuse every document from before the change.
    /// `optional` is read with [`Fields::optional`], never [`Fields::value`],
    /// so its absence is a fact the caller decides what to default to rather
    /// than a shape refusal.
    ///
    /// # Errors
    ///
    /// [`FieldFailure::Extra`] for a member outside `required` and `optional`,
    /// then [`FieldFailure::Missing`] for a `required` member absent.
    pub fn exact_with_optional(
        self,
        required: &[&str],
        optional: &[&str],
    ) -> Result<(), ChangeAssuranceError> {
        for name in self.object.names() {
            if !required.contains(&name) && !optional.contains(&name) {
                return Err(ChangeAssuranceError::shape(
                    self.subject,
                    FieldFailure::extra(name),
                ));
            }
        }
        for name in required {
            if !self.object.contains(name) {
                return Err(ChangeAssuranceError::shape(
                    self.subject,
                    FieldFailure::missing(*name),
                ));
            }
        }
        Ok(())
    }

    /// The value under `name`.
    ///
    /// # Errors
    ///
    /// [`FieldFailure::Missing`] when the member is absent.
    pub fn value(self, name: &str) -> Result<&'a JsonValue, ChangeAssuranceError> {
        self.object
            .get(name)
            .ok_or_else(|| ChangeAssuranceError::shape(self.subject, FieldFailure::missing(name)))
    }

    /// The optional value under `name`.
    #[must_use]
    pub fn optional(self, name: &str) -> Option<&'a JsonValue> {
        self.object.get(name)
    }

    /// A nested object under `name`.
    ///
    /// # Errors
    ///
    /// As [`Fields::value`], plus a refusal when the member is not an object.
    pub fn nested(self, name: &'a str) -> Result<Self, ChangeAssuranceError> {
        Fields::read(self.value(name)?, self.subject, name)
    }

    /// The raw string under `name`.
    ///
    /// # Errors
    ///
    /// As [`Fields::value`], plus a refusal when the member is not a string.
    pub fn text(self, name: &str) -> Result<&'a str, ChangeAssuranceError> {
        self.value(name)?
            .as_str()
            .ok_or_else(|| ChangeAssuranceError::shape(self.subject, self.failure(name)))
    }

    /// A non-empty string under `name`.
    ///
    /// # Errors
    ///
    /// As [`Fields::text`], plus a refusal when the string is empty.
    pub fn non_empty(self, name: &str) -> Result<NonEmptyText, ChangeAssuranceError> {
        NonEmptyText::parse(self.text(name)?, self.subject, &self.path(name))
    }

    /// An [`Identity`]-shaped string under `name`.
    ///
    /// # Errors
    ///
    /// As [`Fields::text`], plus [`Identity::parse`]'s refusals.
    pub fn identity(self, name: &str) -> Result<Identity, ChangeAssuranceError> {
        Identity::parse(self.text(name)?, self.subject, &self.path(name))
    }

    /// A stored [`CanonicalDigest`] under `name`.
    ///
    /// # Errors
    ///
    /// As [`Fields::text`], plus a refusal when the value is not 64 lowercase
    /// hexadecimal characters.
    pub fn digest(self, name: &str) -> Result<CanonicalDigest, ChangeAssuranceError> {
        CanonicalDigest::parse_stored(self.text(name)?).map_err(|_| {
            ChangeAssuranceError::shape(self.subject, FieldFailure::malformed(self.path(name)))
        })
    }

    /// An optional stored [`CanonicalDigest`] under `name`, `null` admitted.
    ///
    /// # Errors
    ///
    /// As [`Fields::digest`] when the member is present and not `null`.
    pub fn nullable_digest(
        self,
        name: &str,
    ) -> Result<Option<CanonicalDigest>, ChangeAssuranceError> {
        match self.value(name)? {
            JsonValue::Null => Ok(None),
            _ => self.digest(name).map(Some),
        }
    }

    /// A boolean under `name`.
    ///
    /// # Errors
    ///
    /// As [`Fields::value`], plus a refusal when the member is not a boolean.
    pub fn boolean(self, name: &str) -> Result<bool, ChangeAssuranceError> {
        match self.value(name)? {
            JsonValue::Bool(value) => Ok(*value),
            _ => Err(ChangeAssuranceError::shape(
                self.subject,
                self.failure(name),
            )),
        }
    }

    /// A non-negative integer under `name`, as a `u64`.
    ///
    /// # Errors
    ///
    /// As [`Fields::value`], plus a refusal when the member is not a number,
    /// is not an integer, is negative, or does not fit a `u64`
    /// (`DIVERGENCE.md` §2).
    pub fn whole_number(self, name: &str) -> Result<u64, ChangeAssuranceError> {
        let refuse = || ChangeAssuranceError::shape(self.subject, self.failure(name));
        let value = self.value(name)?.as_f64().ok_or_else(refuse)?;
        if !value.is_finite() || value.fract() != 0.0 || value < 0.0 {
            return Err(refuse());
        }
        // Through the decimal spelling rather than an `as` cast: the workspace
        // lints every `as` crossing a persistence boundary, and `f64` has no
        // `TryFrom` for `u64` in std. `{:.0}` on a non-negative integral
        // double is its exact decimal digits, so this parse is lossless when
        // it succeeds and refuses when the value is out of range.
        format!("{value:.0}").parse::<u64>().map_err(|_| refuse())
    }

    /// The array under `name`.
    ///
    /// # Errors
    ///
    /// As [`Fields::value`], plus a refusal when the member is not an array,
    /// and when `require_non_empty` and the array is empty.
    pub fn array(
        self,
        name: &str,
        require_non_empty: bool,
    ) -> Result<&'a [JsonValue], ChangeAssuranceError> {
        let refuse = || ChangeAssuranceError::shape(self.subject, self.failure(name));
        match self.value(name)? {
            JsonValue::Array(items) if !(require_non_empty && items.is_empty()) => Ok(items),
            _ => Err(refuse()),
        }
    }

    /// An array of non-empty strings under `name`, optionally required to be
    /// non-empty and to hold no duplicate.
    ///
    /// # Errors
    ///
    /// As [`Fields::array`], plus a refusal for a non-string or empty entry
    /// and, when `require_unique`, for a repeated one.
    pub fn string_array(
        self,
        name: &str,
        require_non_empty: bool,
        require_unique: bool,
    ) -> Result<Vec<String>, ChangeAssuranceError> {
        let refuse = || ChangeAssuranceError::shape(self.subject, self.failure(name));
        let mut values = Vec::new();
        for item in self.array(name, require_non_empty)? {
            match item.as_str() {
                Some(text) if !text.is_empty() => values.push(text.to_owned()),
                _ => return Err(refuse()),
            }
        }
        if require_unique {
            require_unique_values(values.iter().map(String::as_str), self.subject, name)?;
        }
        Ok(values)
    }

    /// Refuse a member whose value is not exactly `expected`.
    ///
    /// # Errors
    ///
    /// As [`Fields::value`], plus a refusal on any other value.
    pub fn equals(self, name: &str, expected: &JsonValue) -> Result<(), ChangeAssuranceError> {
        if self.value(name)? == expected {
            Ok(())
        } else {
            Err(ChangeAssuranceError::shape(
                self.subject,
                self.failure(name),
            ))
        }
    }

    /// An [`ArtifactDigest`] under `name`.
    ///
    /// # Errors
    ///
    /// As [`Fields::text`], plus a refusal when the value is not 64 lowercase
    /// hexadecimal characters.
    pub fn artifact_digest(self, name: &str) -> Result<ArtifactDigest, ChangeAssuranceError> {
        ArtifactDigest::parse(self.text(name)?, self.subject, &self.path(name))
    }

    /// An [`EventHash`] under `name`.
    ///
    /// # Errors
    ///
    /// As [`Fields::artifact_digest`].
    pub fn event_hash(self, name: &str) -> Result<EventHash, ChangeAssuranceError> {
        EventHash::parse(self.text(name)?, self.subject, &self.path(name))
    }

    /// A [`RawBytesDigest`] under `name` — a digest this crate recomputes over
    /// bytes as supplied, and so one of `quoin-store`'s own domains.
    ///
    /// # Errors
    ///
    /// As [`Fields::artifact_digest`].
    pub fn raw_bytes_digest(self, name: &str) -> Result<RawBytesDigest, ChangeAssuranceError> {
        RawBytesDigest::parse_stored(self.text(name)?).map_err(|_| {
            ChangeAssuranceError::shape(self.subject, FieldFailure::malformed(self.path(name)))
        })
    }

    /// The refusal this object raises for a malformed `name`.
    fn failure(self, name: &str) -> FieldFailure {
        FieldFailure::malformed(self.path(name))
    }

    /// The dotted path of `name` inside this object.
    fn path(self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{}.{name}", self.prefix)
        }
    }
}

/// Refuse a repeated value.
///
/// # Errors
///
/// [`ChangeAssuranceError::Shape`] naming `field` on the first repeat.
pub fn require_unique_values<'a>(
    values: impl Iterator<Item = &'a str>,
    subject: Subject,
    field: &str,
) -> Result<(), ChangeAssuranceError> {
    let mut seen = std::collections::BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(ChangeAssuranceError::shape(
                subject,
                FieldFailure::malformed(format!("duplicate {field}")),
            ));
        }
    }
    Ok(())
}

/// Refuse a sequence not already in UTF-16 order.
///
/// # Errors
///
/// [`ChangeAssuranceError::Shape`] naming `field` at the first inversion.
pub fn require_sorted<'a>(
    values: impl Iterator<Item = &'a str>,
    subject: Subject,
    field: &str,
) -> Result<(), ChangeAssuranceError> {
    let mut previous: Option<&str> = None;
    for value in values {
        if let Some(previous) = previous
            && compare_utf16(previous, value) == Ordering::Greater
        {
            return Err(ChangeAssuranceError::shape(
                subject,
                FieldFailure::malformed(format!("{field} order")),
            ));
        }
        previous = Some(value);
    }
    Ok(())
}

/// Build a JSON array of strings.
#[must_use]
pub fn string_values<S: AsRef<str>>(values: &[S]) -> JsonValue {
    JsonValue::Array(
        values
            .iter()
            .map(|value| JsonValue::string(value.as_ref()))
            .collect(),
    )
}

/// Build a JSON object from ordered pairs.
///
/// # Panics
///
/// Never: [`JsonObject::set`] replaces rather than refusing, so a repeated
/// name in the caller's list is the caller's last value rather than an error.
#[must_use]
pub fn object(members: Vec<(&str, JsonValue)>) -> JsonValue {
    let mut built = JsonObject::new();
    for (name, value) in members {
        built.set(name, value);
    }
    JsonValue::Object(built)
}

/// Build a JSON number from a count.
///
/// Routed through the decimal spelling rather than an `as` cast, for the
/// reason [`Fields::whole_number`] states in the other direction. A `u64`
/// always has a finite double nearest it, so the parse cannot fail and cannot
/// produce a non-finite value.
///
/// # Errors
///
/// Never: the result of parsing a decimal integer is finite, and
/// [`JsonValue::number`] refuses only non-finite doubles.
pub fn number(value: u64) -> Result<JsonValue, ChangeAssuranceError> {
    let widened: f64 = format!("{value}").parse().unwrap_or(f64::MAX);
    Ok(JsonValue::number(widened)?)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]
    use std::cmp::Ordering;

    use super::{compare_utf16, number};

    #[test]
    fn utf16_order_disagrees_with_scalar_order_across_the_surrogate_boundary() {
        // JavaScript: "\u{10000}" < "￿" is true, because the astral
        // character is the surrogate pair D800 DC00 and D800 < FFFF.
        assert_eq!(compare_utf16("\u{10000}", "\u{ffff}"), Ordering::Less);
        assert_eq!("\u{10000}".cmp("\u{ffff}"), Ordering::Greater);
        assert_eq!(compare_utf16("a", "b"), Ordering::Less);
        assert_eq!(compare_utf16("ab", "a"), Ordering::Greater);
        assert_eq!(compare_utf16("a", "a"), Ordering::Equal);
    }

    #[test]
    fn counts_round_trip_through_the_double_the_oracle_would_have_parsed() {
        for value in [
            0_u64,
            1,
            42,
            4_294_967_295,
            4_294_967_296,
            9_007_199_254_740_991,
        ] {
            let json = number(value).unwrap();
            assert_eq!(json, number(value).unwrap());
            assert_eq!(format!("{:.0}", json.as_f64().unwrap()), format!("{value}"));
        }
    }
}
