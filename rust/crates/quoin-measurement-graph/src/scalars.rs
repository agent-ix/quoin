// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The constrained strings the adapter schemas declare, as types.
//!
//! # Parse, don't validate
//!
//! Every `z.string().regex(...)` and `z.string().min(1)` in
//! `src/measurement/graph-adapters.ts` becomes a newtype here whose only
//! constructor is its parser. A field typed
//! [`FullRevision`] cannot hold something that is not one, so no consumer
//! re-checks and no branch can forget to.
//!
//! Each carries `#[serde(try_from = "String")]`, so the check happens during
//! deserialization rather than in a second pass a caller could skip.
//!
//! # No regex engine
//!
//! The six patterns are a fixed-length hex test, a `..`-segment test and a
//! prefix test. Each is written as the predicate it is: adding a regex
//! dependency to spell `[0-9a-f]{64}` would put a backtracking engine on an
//! untrusted-input path to save nine lines.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A value that did not satisfy the pattern its field declares.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{0}")]
pub struct MalformedScalar(String);

type Parsed<T> = std::result::Result<T, MalformedScalar>;

/// Build a refusal. The one constructor, so the message shape is one shape.
#[must_use]
pub fn malformed(detail: impl Into<String>) -> MalformedScalar {
    MalformedScalar(detail.into())
}

/// Whether `text` is exactly `length` lowercase hexadecimal characters.
///
/// Lowercase only, as every `[0-9a-f]` in the retained file is: an uppercase
/// digest is a different string and would not match the retained schema either.
fn is_lower_hex(text: &str, length: usize) -> bool {
    text.len() == length
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

macro_rules! scalar {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, PartialEq, Hash, Deserialize, Serialize)]
        #[serde(try_from = "String")]
        pub struct $name(String);

        impl $name {
            /// The value as text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = MalformedScalar;

            fn try_from(value: String) -> Parsed<Self> {
                Self::parse(&value)
            }
        }
    };
}

scalar!(
    NonEmptyText,
    "A `z.string().min(1)` value: at least one character."
);
scalar!(
    FullRevision,
    "A `^[0-9a-f]{40}$` value: a full git revision."
);
scalar!(RevisionIdentity, "A full revision or a semantic version.");
scalar!(
    LocatorPath,
    "A relative path with no `..` segment, as a source locator carries."
);
scalar!(
    ScorerOutputPath,
    "A relative path that is neither rooted nor drive-qualified."
);
scalar!(ArtifactUuid, "A `z.string().uuid()` value.");

impl NonEmptyText {
    /// Read a non-empty value.
    ///
    /// # Errors
    ///
    /// [`MalformedScalar`] for the empty string. Whitespace is a character:
    /// `z.string().min(1)` accepts `" "` and so does this.
    pub fn parse(value: &str) -> Parsed<Self> {
        if value.is_empty() {
            return Err(malformed("expected at least one character"));
        }
        Ok(Self(value.to_owned()))
    }
}

impl FullRevision {
    /// Read a full git revision.
    ///
    /// # Errors
    ///
    /// [`MalformedScalar`] for anything but 40 lowercase hexadecimal characters.
    pub fn parse(value: &str) -> Parsed<Self> {
        if is_lower_hex(value, 40) {
            Ok(Self(value.to_owned()))
        } else {
            Err(malformed(format!(
                "expected a 40-character revision, got `{value}`"
            )))
        }
    }
}

/// Whether `value` matches `v?[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?`.
fn is_semantic_version(value: &str) -> bool {
    let value = value.strip_prefix('v').unwrap_or(value);
    let (core, tail) = match value.find(['-', '+']) {
        Some(at) => value.split_at(at),
        None => (value, ""),
    };
    let mut parts = core.split('.');
    let numeric = |part: Option<&str>| {
        part.is_some_and(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
    };
    if !(numeric(parts.next()) && numeric(parts.next()) && numeric(parts.next()))
        || parts.next().is_some()
    {
        return false;
    }
    // The pattern's tail is `[-+]` followed by one or more of `[0-9A-Za-z.-]`,
    // so a bare trailing `-` does not match.
    tail.is_empty()
        || (tail.len() > 1
            && tail[1..]
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-'))
}

impl RevisionIdentity {
    /// Read either spelling a producer may name a revision with.
    ///
    /// # Errors
    ///
    /// [`MalformedScalar`] when the value is neither a 40-character revision nor a
    /// semantic version with an optional `v` prefix.
    pub fn parse(value: &str) -> Parsed<Self> {
        let accepted = is_lower_hex(value, 40) || is_semantic_version(value);
        if accepted {
            Ok(Self(value.to_owned()))
        } else {
            Err(malformed(format!(
                "expected a revision or a semantic version, got `{value}`"
            )))
        }
    }
}

impl LocatorPath {
    /// Read a locator path.
    ///
    /// # Errors
    ///
    /// [`MalformedScalar`] for the empty string, a value beginning `/`, and any
    /// value carrying a `..` path segment. A `..` *inside* a segment — `a..b` —
    /// is accepted, as the retained lookahead accepts it.
    pub fn parse(value: &str) -> Parsed<Self> {
        if value.is_empty() {
            return Err(malformed("expected at least one character"));
        }
        if value.starts_with('/') {
            return Err(malformed(format!("`{value}` is not relative")));
        }
        if value.split('/').any(|segment| segment == "..") {
            return Err(malformed(format!("`{value}` escapes its root")));
        }
        Ok(Self(value.to_owned()))
    }
}

impl ScorerOutputPath {
    /// Read a raw-scorer-output path.
    ///
    /// # Errors
    ///
    /// [`MalformedScalar`] for the empty string, a rooted path, and a
    /// drive-qualified Windows path such as `C:\out`.
    pub fn parse(value: &str) -> Parsed<Self> {
        if value.is_empty() {
            return Err(malformed("expected at least one character"));
        }
        if value.starts_with('/') {
            return Err(malformed(format!("`{value}` is not relative")));
        }
        let mut bytes = value.bytes();
        if bytes.next().is_some_and(|b| b.is_ascii_alphabetic()) && bytes.next() == Some(b':') {
            return Err(malformed(format!("`{value}` is drive qualified")));
        }
        Ok(Self(value.to_owned()))
    }
}

impl ArtifactUuid {
    /// Read a UUID in the form zod 4.4.3's `.uuid()` accepts.
    ///
    /// That is *not* RFC 9562: zod accepts versions 1 through 8 with variant
    /// `8`, `9`, `a` or `b` in either case, plus the nil and max UUIDs as named
    /// exceptions. The retained schema is the contract, so the retained
    /// library's grammar is what is ported; the boundaries were measured
    /// against zod 4.4.3 rather than read off its source.
    ///
    /// # Errors
    ///
    /// [`MalformedScalar`] for anything that grammar refuses.
    pub fn parse(value: &str) -> Parsed<Self> {
        const NIL: &str = "00000000-0000-0000-0000-000000000000";
        const MAX: &str = "ffffffff-ffff-ffff-ffff-ffffffffffff";
        let lowered = value.to_ascii_lowercase();
        if lowered == NIL || lowered == MAX {
            return Ok(Self(value.to_owned()));
        }
        let groups: Vec<&str> = lowered.split('-').collect();
        let shaped = groups.len() == 5
            && [8, 4, 4, 4, 12]
                .iter()
                .zip(&groups)
                .all(|(width, group)| is_lower_hex(group, *width))
            && groups
                .get(2)
                .is_some_and(|group| group.starts_with(|c: char| ('1'..='8').contains(&c)))
            && groups
                .get(3)
                .is_some_and(|group| group.starts_with(['8', '9', 'a', 'b']));
        if shaped {
            Ok(Self(value.to_owned()))
        } else {
            Err(malformed(format!("`{value}` is not a UUID")))
        }
    }
}
