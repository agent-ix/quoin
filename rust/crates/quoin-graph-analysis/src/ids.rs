// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The identifier newtypes, and the one comparison every one of them uses.
//!
//! # Why these are types and not `String`
//!
//! `analysis.ts` passes eight different kinds of identifier through functions
//! that all take `string`: an artifact id, an obligation id, a suite name, a
//! document path, a relation kind, an author, a commit and a statement hash.
//! `verdictFor(report, obligation, gaps)` and
//! `selectedCorpusEdges(export, selection, requirementIds, allArtifactIds, …)`
//! are both call sites where two same-typed arguments could be transposed
//! without a compiler noticing.
//!
//! # Why they carry their own `Ord`
//!
//! Every ordering in the retained source is
//! `compare(left, right) = left === right ? 0 : left < right ? -1 : 1` —
//! JavaScript `<` on strings, which is **UTF-16 code unit order**. Rust's
//! `str: Ord` is Unicode scalar order, and the two disagree: `U+1F600` sorts
//! *after* `U+FFFD` in Rust and *before* it in JavaScript. That is not a
//! hypothetical for a corpus of spec ids and author names, and
//! `text/escapes-and-utf16-order` in the golden corpus makes it observable.
//!
//! So [`cmp_utf16`] — `quoin-store`'s, the one this workspace already owns —
//! is the `Ord` of every type here. A `BTreeMap` keyed on one of them iterates
//! in the order the oracle sorts in, with no sort step to forget.

use std::cmp::Ordering;
use std::fmt;

use quoin_store::json::order::cmp_utf16;
use serde::{Deserialize, Serialize};

/// The shared half: wrapping, borrowing, rendering, and UTF-16 ordering.
macro_rules! ordered_text {
    ($name:ident) => {
        impl $name {
            /// The wrapped text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// The wrapped text, by value.
            #[must_use]
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl Ord for $name {
            fn cmp(&self, other: &Self) -> Ordering {
                cmp_utf16(&self.0, &other.0)
            }
        }

        impl PartialOrd for $name {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }
    };
}

/// An identifier the producer already wrote, wrapped for ordering and for the
/// argument it is.
///
/// There is no `parse` here on purpose. These arrive inside a quire assurance
/// export that has already been validated against `assurance-v1`; a second,
/// stricter check at this boundary would refuse exports the retained
/// implementation analyses.
macro_rules! producer_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Wrap text the producer wrote.
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
        }

        ordered_text!($name);
    };
}

producer_id!(
    /// One accepted artifact's id — `FR-001`, `US-014`.
    ArtifactId
);
producer_id!(
    /// One obligation's id. The join key between the export, the bindings
    /// store and the audit report.
    ObligationId
);
producer_id!(
    /// One test suite's name, as the bindings store spells it.
    SuiteId
);
producer_id!(
    /// One document path, as the export's locators spell it.
    ///
    /// `obligation.document` is matched against `artifact.locator.path` by
    /// string equality (`analysis.ts:466`) — no normalisation, no separator
    /// rewriting. This type carries that: it is the export's spelling, not a
    /// filesystem path.
    DocumentPath
);
producer_id!(
    /// One relationship kind — `depends_on`, `traces_to`.
    RelationKind
);
producer_id!(
    /// Who re-affirmed a binding.
    Author
);
producer_id!(
    /// A commit, as the bindings store recorded it.
    ///
    /// Not [`Revision`]: the bindings store's `commit` carries no shape
    /// constraint at any boundary the retained implementation checks, and
    /// giving it one here would refuse stores that already exist.
    Commit
);
producer_id!(
    /// The statement hash a binding was made against.
    StatementHash
);

/// An identifier a boundary in this crate refuses to accept malformed.
///
/// The constructor is `parse`, it returns the type or the reason, and there is
/// no `bool` anywhere: the proof that the text is well-formed *is* the value.
/// `RawFileSha256Digest::parse_stored` (`quoin-store/src/digest.rs`) is the
/// in-repo model.
macro_rules! checked_id {
    ($(#[$meta:meta])* $name:ident, $check:expr, $what:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Parse one, or say why it is not one.
            ///
            /// # Errors
            ///
            /// The reason, for a caller to render against the member it came
            /// from.
            pub fn parse(value: impl Into<String>) -> std::result::Result<Self, String> {
                let text: String = value.into();
                #[allow(clippy::redundant_closure_call)]
                if ($check)(text.as_str()) {
                    Ok(Self(text))
                } else {
                    Err(format!("{} is not {}", Debugged(&text), $what))
                }
            }
        }

        ordered_text!($name);
    };
}

/// A rejected value, rendered short enough to read in a diagnostic.
struct Debugged<'a>(&'a str);

impl fmt::Display for Debugged<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const LIMIT: usize = 48;
        if self.0.chars().count() <= LIMIT {
            return write!(f, "{:?}", self.0);
        }
        let head: String = self.0.chars().take(LIMIT).collect();
        write!(f, "{head:?}…")
    }
}

/// `true` when `text` is exactly `width` lowercase hexadecimal digits.
fn is_lowercase_hex(text: &str, width: usize) -> bool {
    text.len() == width
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

checked_id!(
    /// A source revision: 40 lowercase hexadecimal digits (`input.ts:39`).
    Revision,
    |text| is_lowercase_hex(text, 40),
    "40 lowercase hexadecimal digits"
);
checked_id!(
    /// An archetype's semantic schema digest: 64 lowercase hexadecimal digits
    /// (`input.ts:16`).
    SchemaDigest,
    |text| is_lowercase_hex(text, 64),
    "64 lowercase hexadecimal digits"
);
checked_id!(
    /// A repository identity. Non-empty (`input.ts:38`).
    RepositoryId,
    |text: &str| !text.is_empty(),
    "a non-empty repository identity"
);
checked_id!(
    /// A loaded module's name. Non-empty (`input.ts:22`).
    ModuleName,
    |text: &str| !text.is_empty(),
    "a non-empty module name"
);
checked_id!(
    /// A loaded module's version. Non-empty (`input.ts:23`).
    ModuleVersion,
    |text: &str| !text.is_empty(),
    "a non-empty module version"
);
checked_id!(
    /// One active archetype's name. Non-empty (`input.ts:15`).
    Archetype,
    |text: &str| !text.is_empty(),
    "a non-empty archetype name"
);

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{Archetype, ArtifactId, Revision, SchemaDigest};

    /// The ordering is UTF-16's, which is not Rust's.
    ///
    /// `U+1F600` encodes as the surrogate pair `D83D DE00`, and `D83D` is less
    /// than `FFFD`. Rust's `str: Ord` puts it the other way round. If this
    /// test ever passes with `a.0 < b.0`, the ordering has been quietly
    /// swapped for the wrong one.
    ///
    /// Provenance: quoin#385
    #[test]
    fn an_astral_id_sorts_before_a_high_bmp_one() {
        let astral = ArtifactId::new("\u{1F600}");
        let replacement = ArtifactId::new("\u{FFFD}");
        assert!(
            astral < replacement,
            "UTF-16 order puts the surrogate first"
        );
        assert!(
            astral.as_str() > replacement.as_str(),
            "and Rust's own str order does not, which is why this type has its own"
        );
    }

    /// Provenance: quoin#385
    #[test]
    fn a_revision_is_forty_lowercase_hex_digits_and_nothing_else() {
        assert!(Revision::parse("0".repeat(40)).is_ok());
        assert!(Revision::parse("0".repeat(39)).is_err());
        assert!(Revision::parse("0".repeat(41)).is_err());
        assert!(Revision::parse(format!("{}A", "0".repeat(39))).is_err());
        assert!(Revision::parse(format!("{}g", "0".repeat(39))).is_err());
    }

    /// Provenance: quoin#385
    #[test]
    fn a_schema_digest_is_sixty_four_and_an_archetype_is_merely_present() {
        assert!(SchemaDigest::parse("a".repeat(64)).is_ok());
        assert!(SchemaDigest::parse("a".repeat(40)).is_err());
        assert!(Archetype::parse("FR").is_ok());
        assert!(Archetype::parse("").is_err());
    }

    /// A rejection names the value it rejected, and does not paste a megabyte
    /// of it into the diagnostic.
    ///
    /// Provenance: quoin#385
    #[test]
    fn a_rejection_truncates_what_it_quotes() {
        let long = "z".repeat(4096);
        let message = Revision::parse(long).expect_err("not a revision");
        assert!(message.len() < 200, "the diagnostic is readable: {message}");
        assert!(message.contains('…'), "and says it was cut: {message}");
    }
}
