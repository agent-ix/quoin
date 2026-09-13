// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The intervention record identity, and the file name it becomes.
//!
//! # Why this is its own module
//!
//! `intervention.ts:33-45` is eleven lines of TypeScript and the highest-risk
//! encoding in this crate: it turns an identity that may contain `/` and `:`
//! into a single path component, and it must do so **injectively**. If two
//! distinct record ids ever produced one basename, intake's write-once
//! collision refusal would fire on records that are not the same record, and
//! the store would refuse to retain evidence it has never seen.
//!
//! The retained code states the property in a comment:
//!
//! > Keep the two namespaces disjoint: a portable id can never alias the
//! > base64url encoding of a slash/colon-bearing id.
//!
//! Here it is a type. [`InterventionRecordBasename`] is minted only by
//! [`InterventionRecordId::basename`], carries which of the two namespaces it
//! came from, and the disjointness is asserted rather than commented.
//!
//! # Why the namespaces are disjoint
//!
//! A `p-` basename's tail matches `[A-Za-z0-9][A-Za-z0-9._-]{0,127}`, so it may
//! contain `.` and `_`. A `b-` basename's tail is base64url, which is
//! `[A-Za-z0-9_-]` only. The two alphabets overlap, so the prefix is what
//! separates them — and it separates them completely, because the prefix is a
//! single byte outside both tails' first-character class only by construction.
//! The real guarantee is simpler than that: `p-` and `b-` are different
//! prefixes, one is chosen per id by a total predicate on the id, and the
//! predicate is the same one both times. Two ids sharing a basename would have
//! to share a namespace, and within a namespace the encoding is injective —
//! identity for `p-`, base64url for `b-`.
//!
//! # The 128-byte cap
//!
//! The schema caps identity at 128 ASCII characters, which the parse grammar
//! restates as `{0,127}` after a mandatory first character. At the cap a `b-`
//! basename is `2 + ceil(128 * 4 / 3) = 173` bytes before `.json` — unpadded,
//! so the trailing group is three symbols and not four — which is 178 with the
//! extension, well under the 255-byte `NAME_MAX` every filesystem this runs on
//! enforces. That arithmetic is asserted in `tests/tc_471_intake.rs` rather
//! than left as a remark.
//!
//! # The one base64 in this crate
//!
//! There is no base64 anywhere else in `quoin-measurement`, and the workspace
//! exposes none to reuse, so the encoder is here — 20 lines, RFC 4648 §5, no
//! padding, which is what Node's `"base64url"` emits. `tests/tc_471_intake.rs`
//! measures it against RFC 4648's own published vectors, not against itself.

use crate::intervention::intake::{InterventionIntakeError, InterventionRefusalCode};

/// The longest record id the schema admits, in ASCII characters.
///
/// `intervention.ts:34`'s `{0,127}` after one mandatory leading character.
pub const MAX_RECORD_ID_BYTES: usize = 128;

/// Which of the two disjoint basename namespaces an identity encodes into.
///
/// `intervention.ts:41-43`. A `Copy` enum rather than a `bool`, so the two
/// arms are named at every use and a third namespace would be a compile error
/// rather than a silently inverted flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RecordIdNamespace {
    /// The id is already a safe path component and is carried verbatim.
    Portable,
    /// The id carries `/` or `:` and is carried base64url-encoded.
    Encoded,
}

impl RecordIdNamespace {
    /// Every namespace, for the censuses that must not measure nothing.
    pub const ALL: [Self; 2] = [Self::Portable, Self::Encoded];

    /// The basename prefix this namespace writes.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Portable => "p-",
            Self::Encoded => "b-",
        }
    }
}

/// An intervention record's identity, checked against `intervention.ts:34`.
///
/// Parse, don't validate: holding one of these is the proof that the identity
/// names a file, so [`crate::store::intervention_path`] cannot be
/// called with one that does not.
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct InterventionRecordId(String);

/// Whether `value` matches `^[A-Za-z0-9][<tail>]{0,127}$` for the given tail.
///
/// Written once and applied twice, because `intervention.ts:34` and `:40` are
/// the same grammar over two alphabets. Porting them as two hand-walked loops
/// would be the duplication this crate's review criteria forbid.
fn matches_id_grammar(value: &str, tail: fn(u8) -> bool) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_alphanumeric() && value.len() <= MAX_RECORD_ID_BYTES && bytes.all(tail)
}

/// The tail alphabet of `intervention.ts:34` — `[A-Za-z0-9._:/-]`.
fn is_identity_tail(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
}

/// The tail alphabet of `intervention.ts:40` — `[A-Za-z0-9._-]`.
fn is_portable_tail(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
}

impl InterventionRecordId {
    /// Read a record identity.
    ///
    /// # Errors
    ///
    /// [`InterventionRefusalCode::InvalidRecord`] carrying the
    /// `/record_id: unsafe record id …` finding `intervention.ts:35-37`
    /// emits, for every spelling outside the grammar or over the cap.
    pub fn parse(value: &str) -> Result<Self, InterventionIntakeError> {
        if matches_id_grammar(value, is_identity_tail) {
            Ok(Self(value.to_owned()))
        } else {
            Err(InterventionIntakeError::new(
                InterventionRefusalCode::InvalidRecord,
                // `JSON.stringify` of a string is a JSON string literal, which
                // is what `serde_json`'s `Display` for a `str` value emits.
                vec![format!(
                    "/record_id: unsafe record id {}",
                    serde_json::Value::String(value.to_owned())
                )],
            ))
        }
    }

    /// The identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Which namespace this identity encodes into.
    #[must_use]
    pub fn namespace(&self) -> RecordIdNamespace {
        if matches_id_grammar(&self.0, is_portable_tail) {
            RecordIdNamespace::Portable
        } else {
            RecordIdNamespace::Encoded
        }
    }

    /// The single path component this identity is stored under.
    #[must_use]
    pub fn basename(&self) -> InterventionRecordBasename {
        let namespace = self.namespace();
        let tail = match namespace {
            RecordIdNamespace::Portable => self.0.clone(),
            RecordIdNamespace::Encoded => base64url_no_pad(self.0.as_bytes()),
        };
        InterventionRecordBasename {
            namespace,
            text: format!("{}{tail}", namespace.prefix()),
        }
    }
}

impl std::fmt::Display for InterventionRecordId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The file name, without extension, one record is retained under.
///
/// Minted only by [`InterventionRecordId::basename`].
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct InterventionRecordBasename {
    namespace: RecordIdNamespace,
    text: String,
}

impl InterventionRecordBasename {
    /// Which namespace minted it.
    #[must_use]
    pub const fn namespace(&self) -> RecordIdNamespace {
        self.namespace
    }

    /// The basename, prefix included, extension excluded.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The retained file name, as it appears in the interventions directory.
    #[must_use]
    pub fn file_name(&self) -> String {
        format!("{}.json", self.text)
    }
}

impl std::fmt::Display for InterventionRecordBasename {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.text)
    }
}

/// The RFC 4648 §5 base64url alphabet, in code-point order.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// The symbol for one six-bit group.
///
/// `bits & 0x3f` is in `0..64` and the alphabet has exactly 64 entries, so the
/// fallback arm is unreachable. It is written rather than asserted so this
/// function cannot panic on any input; `tests/tc_471_intake.rs` checks the
/// alphabet is complete, which is what makes the arm dead.
fn symbol(bits: usize) -> char {
    ALPHABET
        .get(bits & 0x3f)
        .map_or('\u{fffd}', |byte| char::from(*byte))
}

/// RFC 4648 §5 base64url without padding — Node's `"base64url"` encoding.
///
/// Public so `tests/tc_471_intake.rs` can measure it against RFC 4648 §10's
/// published vectors. A private encoder could only be tested through
/// [`InterventionRecordId::basename`], which would test the encoder against
/// basenames this crate produced itself and prove nothing.
#[must_use]
pub fn base64url_no_pad(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let (triples, remainder) = bytes.as_chunks::<3>();
    for &[one, two, three] in triples {
        let triple = (usize::from(one) << 16) | (usize::from(two) << 8) | usize::from(three);
        for shift in [18_usize, 12, 6, 0] {
            out.push(symbol(triple >> shift));
        }
    }
    match remainder {
        [one] => {
            let triple = usize::from(*one) << 16;
            out.push(symbol(triple >> 18));
            out.push(symbol(triple >> 12));
        }
        [one, two] => {
            let triple = (usize::from(*one) << 16) | (usize::from(*two) << 8);
            for shift in [18_usize, 12, 6] {
                out.push(symbol(triple >> shift));
            }
        }
        _ => {}
    }
    out
}

/// The alphabet, for the test that proves `symbol`'s fallback arm is dead.
#[must_use]
pub const fn base64url_alphabet() -> &'static [u8; 64] {
    ALPHABET
}
