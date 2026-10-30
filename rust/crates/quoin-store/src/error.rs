// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The one error type this crate surfaces, and its stable code catalogue.
//!
//! Every distinguishable refusal is its own variant with typed fields. A caller
//! never parses a message to learn what happened: it matches a variant, or
//! compares [`StoreErrorCode`], which is stable and never renamed or reused.

use std::path::PathBuf;

/// A stable, never-reused code for each refusal this crate can produce.
///
/// Codes are part of the compatibility surface: they appear in
/// downstream diagnostics. Add members; never rename or repurpose one.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum StoreErrorCode {
    /// The shared canonical encoder refused an output.
    CanonicalEncoding,
    /// Input bytes were not valid UTF-8.
    JsonNotUtf8,
    /// Input began with a byte-order mark.
    JsonByteOrderMark,
    /// A JSON value was expected and the input did not provide one.
    JsonExpectedValue,
    /// Content followed the top-level value.
    JsonTrailingContent,
    /// An object member name was not a string.
    JsonObjectKeyNotString,
    /// An object declared the same member name twice.
    JsonDuplicateName,
    /// A `:` was expected after an object member name.
    JsonExpectedColon,
    /// A `,` or `}` was expected inside an object.
    JsonExpectedObjectSeparator,
    /// A `,` or `]` was expected inside an array.
    JsonExpectedArraySeparator,
    /// A string was not terminated before end of input.
    JsonUnterminatedString,
    /// A string contained an unescaped control character below `U+0020`.
    JsonUnescapedControl,
    /// A string escape sequence was malformed.
    JsonInvalidEscape,
    /// A `\uD800`-`\uDBFF` escape was not followed by a low-surrogate escape.
    JsonLoneHighSurrogate,
    /// A `\uDC00`-`\uDFFF` escape appeared without a preceding high surrogate.
    JsonLoneLowSurrogate,
    /// A literal (`true`, `false`, `null`) did not match.
    JsonInvalidLiteral,
    /// A number was syntactically valid but not finite as an IEEE-754 double.
    JsonNumberNotFinite,
    /// A document nested deeper than the canonicalizers will read or write.
    JsonNestingTooDeep,
    /// A value that must be a JSON object was not one.
    NotAnObject,
    /// A filesystem operation failed.
    Io,
    /// A content-addressed path already held different bytes.
    ContentCollision,
    /// A path offered for a bounded read was a symbolic link.
    ReadSourceIsSymlink,
    /// A path offered for a bounded read was not a regular file.
    ReadSourceNotRegularFile,
    /// A file offered for a bounded read exceeded the limit.
    ReadSourceTooLarge,
    /// A file changed size while it was being read.
    ReadSourceChangedWhileReading,
}

impl StoreErrorCode {
    /// Every code, in declaration order.
    pub const ALL: &'static [Self] = &[
        Self::CanonicalEncoding,
        Self::JsonNotUtf8,
        Self::JsonByteOrderMark,
        Self::JsonExpectedValue,
        Self::JsonTrailingContent,
        Self::JsonObjectKeyNotString,
        Self::JsonDuplicateName,
        Self::JsonExpectedColon,
        Self::JsonExpectedObjectSeparator,
        Self::JsonExpectedArraySeparator,
        Self::JsonUnterminatedString,
        Self::JsonUnescapedControl,
        Self::JsonInvalidEscape,
        Self::JsonLoneHighSurrogate,
        Self::JsonLoneLowSurrogate,
        Self::JsonInvalidLiteral,
        Self::JsonNumberNotFinite,
        Self::JsonNestingTooDeep,
        Self::NotAnObject,
        Self::Io,
        Self::ContentCollision,
        Self::ReadSourceIsSymlink,
        Self::ReadSourceNotRegularFile,
        Self::ReadSourceTooLarge,
        Self::ReadSourceChangedWhileReading,
    ];

    /// The stable wire spelling of this code.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CanonicalEncoding => "QSTORE-CANONICAL-ENCODING",
            Self::JsonNotUtf8 => "QSTORE-JSON-NOT-UTF8",
            Self::JsonByteOrderMark => "QSTORE-JSON-BOM",
            Self::JsonExpectedValue => "QSTORE-JSON-EXPECTED-VALUE",
            Self::JsonTrailingContent => "QSTORE-JSON-TRAILING-CONTENT",
            Self::JsonObjectKeyNotString => "QSTORE-JSON-KEY-NOT-STRING",
            Self::JsonDuplicateName => "QSTORE-JSON-DUPLICATE-NAME",
            Self::JsonExpectedColon => "QSTORE-JSON-EXPECTED-COLON",
            Self::JsonExpectedObjectSeparator => "QSTORE-JSON-EXPECTED-OBJECT-SEPARATOR",
            Self::JsonExpectedArraySeparator => "QSTORE-JSON-EXPECTED-ARRAY-SEPARATOR",
            Self::JsonUnterminatedString => "QSTORE-JSON-UNTERMINATED-STRING",
            Self::JsonUnescapedControl => "QSTORE-JSON-UNESCAPED-CONTROL",
            Self::JsonInvalidEscape => "QSTORE-JSON-INVALID-ESCAPE",
            Self::JsonLoneHighSurrogate => "QSTORE-JSON-LONE-HIGH-SURROGATE",
            Self::JsonLoneLowSurrogate => "QSTORE-JSON-LONE-LOW-SURROGATE",
            Self::JsonInvalidLiteral => "QSTORE-JSON-INVALID-LITERAL",
            Self::JsonNumberNotFinite => "QSTORE-JSON-NUMBER-NOT-FINITE",
            Self::JsonNestingTooDeep => "QSTORE-JSON-NESTING-TOO-DEEP",
            Self::NotAnObject => "QSTORE-NOT-AN-OBJECT",
            Self::Io => "QSTORE-IO",
            Self::ContentCollision => "QSTORE-CONTENT-COLLISION",
            Self::ReadSourceIsSymlink => "QSTORE-READ-SOURCE-SYMLINK",
            Self::ReadSourceNotRegularFile => "QSTORE-READ-SOURCE-NOT-REGULAR-FILE",
            Self::ReadSourceTooLarge => "QSTORE-READ-SOURCE-TOO-LARGE",
            Self::ReadSourceChangedWhileReading => "QSTORE-READ-SOURCE-CHANGED",
        }
    }

    /// Recover a code from its stable spelling.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|known| known.as_str() == code)
    }
}

impl std::fmt::Display for StoreErrorCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Byte offset into the input at which a parse refusal was decided.
///
/// The TypeScript oracle reports a UTF-16 code-unit offset; this is a UTF-8
/// byte offset. Offsets are diagnostics, never a contract: the accept/refuse
/// decision and the code are what must agree across implementations.
pub type Offset = usize;

/// Everything this crate can refuse, as one enum.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StoreError {
    /// The shared encoder refused canonical output; no bytes are returned.
    #[error("canonical JSON encoding failed: {source}")]
    CanonicalEncoding {
        /// The underlying typed encoder refusal.
        #[source]
        source: quire_canonical::Error,
    },
    /// Input bytes were not valid UTF-8.
    #[error("input is not valid UTF-8")]
    JsonNotUtf8,

    /// Input began with a byte-order mark.
    #[error("a byte-order mark is not permitted")]
    JsonByteOrderMark,

    /// A JSON value was expected and the input did not provide one.
    #[error("expected a JSON value at byte {offset}")]
    JsonExpectedValue {
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// Content followed the top-level value.
    #[error("trailing JSON content at byte {offset}")]
    JsonTrailingContent {
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// An object member name was not a string.
    #[error("object key must be a string at byte {offset}")]
    JsonObjectKeyNotString {
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// An object declared the same member name twice.
    #[error("duplicate object name {name:?} at byte {offset}")]
    JsonDuplicateName {
        /// The repeated member name.
        name: String,
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// A `:` was expected after an object member name.
    #[error("expected ':' after object key at byte {offset}")]
    JsonExpectedColon {
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// A `,` or `}` was expected inside an object.
    #[error("expected ',' or '}}' in object at byte {offset}")]
    JsonExpectedObjectSeparator {
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// A `,` or `]` was expected inside an array.
    #[error("expected ',' or ']' in array at byte {offset}")]
    JsonExpectedArraySeparator {
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// A string was not terminated before end of input.
    #[error("unterminated JSON string at byte {offset}")]
    JsonUnterminatedString {
        /// Where the string began.
        offset: Offset,
    },

    /// A string contained an unescaped control character below `U+0020`.
    #[error("unescaped control character U+{code:04X} at byte {offset}")]
    JsonUnescapedControl {
        /// The offending code point.
        code: u32,
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// A string escape sequence was malformed.
    #[error("invalid JSON string escape at byte {offset}")]
    JsonInvalidEscape {
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// A high-surrogate escape was not followed by a low-surrogate escape.
    #[error("invalid Unicode lone high surrogate at byte {offset}")]
    JsonLoneHighSurrogate {
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// A low-surrogate escape appeared without a preceding high surrogate.
    #[error("invalid Unicode lone low surrogate at byte {offset}")]
    JsonLoneLowSurrogate {
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// A literal (`true`, `false`, `null`) did not match.
    #[error("expected {expected} at byte {offset}")]
    JsonInvalidLiteral {
        /// The literal the parser was committed to.
        expected: &'static str,
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// A number was syntactically valid but not finite as an IEEE-754 double.
    #[error("number is not finite I-JSON at byte {offset}")]
    JsonNumberNotFinite {
        /// The literal text that overflowed.
        literal: String,
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// A document nested deeper than the canonicalizers will read or write.
    ///
    /// The reader and both writers recurse, so an unbounded document is an
    /// abort rather than a refusal. See
    /// [`MAX_NESTING_DEPTH`](crate::json::MAX_NESTING_DEPTH) for the budget and
    /// for the measured band in which this crate refuses input the TypeScript
    /// oracle accepts.
    #[error("JSON nests deeper than the {limit}-level budget, at byte {offset}")]
    JsonNestingTooDeep {
        /// The budget that was exceeded.
        limit: usize,
        /// Where the refusal was decided.
        offset: Offset,
    },

    /// A value that must be a JSON object was not one.
    #[error("expected a JSON object, found {found}")]
    NotAnObject {
        /// The JSON type actually present.
        found: &'static str,
    },

    /// A filesystem operation failed.
    #[error("{operation} failed for {path}")]
    Io {
        /// What was attempted, for example `open`, `rename`, `fsync`.
        operation: &'static str,
        /// The path the operation targeted.
        path: PathBuf,
        /// The underlying failure.
        #[source]
        source: std::io::Error,
    },

    /// A content-addressed path already held different bytes.
    #[error("content collision at {path}: existing bytes differ from the bytes offered")]
    ContentCollision {
        /// The content-addressed path.
        path: PathBuf,
    },

    /// A path offered for a bounded read was a symbolic link.
    ///
    /// A link names whatever it currently points at, which can change after
    /// the read, so a source must be the file itself.
    #[error("{path} is a symbolic link; a read source must be the file itself")]
    ReadSourceIsSymlink {
        /// The offered path.
        path: PathBuf,
    },

    /// A path offered for a bounded read was not a regular file.
    #[error("{path} is not a regular file ({kind}); it has no stable bytes to read")]
    ReadSourceNotRegularFile {
        /// The offered path.
        path: PathBuf,
        /// What it was instead.
        kind: &'static str,
    },

    /// A file offered for a bounded read exceeded the limit.
    #[error("{path} is {size} bytes, over the {limit}-byte limit")]
    ReadSourceTooLarge {
        /// The offered path.
        path: PathBuf,
        /// Its size.
        size: u64,
        /// The limit.
        limit: u64,
    },

    /// A file changed size while it was being read.
    #[error("{path} held {expected} bytes when opened and {actual} when read")]
    ReadSourceChangedWhileReading {
        /// The offered path.
        path: PathBuf,
        /// The size reported before the read.
        expected: u64,
        /// The size actually read.
        actual: u64,
    },
}

impl StoreError {
    /// The stable code for this refusal.
    #[must_use]
    pub const fn code(&self) -> StoreErrorCode {
        match self {
            Self::CanonicalEncoding { .. } => StoreErrorCode::CanonicalEncoding,
            Self::JsonNotUtf8 => StoreErrorCode::JsonNotUtf8,
            Self::JsonByteOrderMark => StoreErrorCode::JsonByteOrderMark,
            Self::JsonExpectedValue { .. } => StoreErrorCode::JsonExpectedValue,
            Self::JsonTrailingContent { .. } => StoreErrorCode::JsonTrailingContent,
            Self::JsonObjectKeyNotString { .. } => StoreErrorCode::JsonObjectKeyNotString,
            Self::JsonDuplicateName { .. } => StoreErrorCode::JsonDuplicateName,
            Self::JsonExpectedColon { .. } => StoreErrorCode::JsonExpectedColon,
            Self::JsonExpectedObjectSeparator { .. } => StoreErrorCode::JsonExpectedObjectSeparator,
            Self::JsonExpectedArraySeparator { .. } => StoreErrorCode::JsonExpectedArraySeparator,
            Self::JsonUnterminatedString { .. } => StoreErrorCode::JsonUnterminatedString,
            Self::JsonUnescapedControl { .. } => StoreErrorCode::JsonUnescapedControl,
            Self::JsonInvalidEscape { .. } => StoreErrorCode::JsonInvalidEscape,
            Self::JsonLoneHighSurrogate { .. } => StoreErrorCode::JsonLoneHighSurrogate,
            Self::JsonLoneLowSurrogate { .. } => StoreErrorCode::JsonLoneLowSurrogate,
            Self::JsonInvalidLiteral { .. } => StoreErrorCode::JsonInvalidLiteral,
            Self::JsonNumberNotFinite { .. } => StoreErrorCode::JsonNumberNotFinite,
            Self::JsonNestingTooDeep { .. } => StoreErrorCode::JsonNestingTooDeep,
            Self::NotAnObject { .. } => StoreErrorCode::NotAnObject,
            Self::Io { .. } => StoreErrorCode::Io,
            Self::ContentCollision { .. } => StoreErrorCode::ContentCollision,
            Self::ReadSourceIsSymlink { .. } => StoreErrorCode::ReadSourceIsSymlink,
            Self::ReadSourceNotRegularFile { .. } => StoreErrorCode::ReadSourceNotRegularFile,
            Self::ReadSourceTooLarge { .. } => StoreErrorCode::ReadSourceTooLarge,
            Self::ReadSourceChangedWhileReading { .. } => {
                StoreErrorCode::ReadSourceChangedWhileReading
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{StoreError, StoreErrorCode};

    #[test]
    fn tc_380_every_code_round_trips_through_its_stable_spelling() {
        for &code in StoreErrorCode::ALL {
            assert_eq!(StoreErrorCode::from_code(code.as_str()), Some(code));
        }
    }

    #[test]
    fn tc_380_code_spellings_are_unique() {
        let mut seen: Vec<&'static str> = StoreErrorCode::ALL.iter().map(|c| c.as_str()).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(before, seen.len(), "two codes share a spelling");
    }

    #[test]
    fn tc_380_error_variants_report_their_own_code() {
        assert_eq!(
            StoreError::JsonByteOrderMark.code(),
            StoreErrorCode::JsonByteOrderMark
        );
        assert_eq!(
            StoreError::NotAnObject { found: "array" }.code(),
            StoreErrorCode::NotAnObject
        );
    }
}
