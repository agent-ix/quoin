// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The identity newtypes, and the two string shapes the schema admits.
//!
//! Every identifier in a change-assurance record is a constrained string, and
//! the constraint is the same one in every slot: `IDENTITY` at
//! `src/change-assurance/records.ts:16`. That made it tempting in the oracle
//! to carry them all as `string`, and the consequence is visible at
//! `verify.ts:125` — `pair.attestation.proof_id !== proof.proof_id` is one
//! character away from comparing a proof id to a record id and still
//! type-checking.
//!
//! Here each slot is its own type. The shared validation lives once in
//! [`Identity`]; the newtypes around it carry no conversion between one
//! another, so a record id cannot reach a proof id's parameter.

use std::fmt;

use crate::error::{ChangeAssuranceError, FieldFailure, Subject};

/// The longest identity the schema admits, in characters.
///
/// `IDENTITY` is `^[A-Za-z0-9][A-Za-z0-9._:/-]{0,255}$`: one leading character
/// plus at most 255 more. The class is ASCII-only, so characters, UTF-16 code
/// units and bytes all agree and the bound does not depend on which is counted.
pub const MAX_IDENTITY_LENGTH: usize = 256;

/// A string satisfying the schema's one identity shape.
///
/// Construction is the only check: a value of this type has already passed
/// `IDENTITY`, so nothing downstream re-tests it.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Identity(String);

impl Identity {
    /// Read an identity, refusing anything `IDENTITY` refuses.
    ///
    /// # Errors
    ///
    /// [`ChangeAssuranceError::Shape`] naming `field` when the value is empty,
    /// longer than [`MAX_IDENTITY_LENGTH`], starts with a character outside
    /// `[A-Za-z0-9]`, or carries one outside `[A-Za-z0-9._:/-]`.
    pub fn parse(value: &str, subject: Subject, field: &str) -> Result<Self, ChangeAssuranceError> {
        if Self::is_valid(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(ChangeAssuranceError::shape(
                subject,
                FieldFailure::malformed(field),
            ))
        }
    }

    /// Whether `value` satisfies `IDENTITY`.
    #[must_use]
    pub fn is_valid(value: &str) -> bool {
        let mut characters = value.chars();
        let Some(first) = characters.next() else {
            return false;
        };
        if !first.is_ascii_alphanumeric() {
            return false;
        }
        if value.chars().count() > MAX_IDENTITY_LENGTH {
            return false;
        }
        characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | ':' | '/' | '-')
        })
    }

    /// The wrapped text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Identity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A non-empty string, the schema's other string shape.
///
/// `nonempty` (`records.ts:543`) admits any string of length one or more, with
/// no character class. Statements, repository names and revisions are this
/// shape, not [`Identity`].
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NonEmptyText(String);

impl NonEmptyText {
    /// Read a non-empty string.
    ///
    /// # Errors
    ///
    /// [`ChangeAssuranceError::Shape`] naming `field` when the value is empty.
    pub fn parse(value: &str, subject: Subject, field: &str) -> Result<Self, ChangeAssuranceError> {
        if value.is_empty() {
            Err(ChangeAssuranceError::shape(
                subject,
                FieldFailure::malformed(field),
            ))
        } else {
            Ok(Self(value.to_owned()))
        }
    }

    /// The wrapped text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NonEmptyText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Declare a distinct identity newtype with no conversion to any sibling.
macro_rules! identity_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name(Identity);

        impl $name {
            /// Read this identity from stored text.
            ///
            /// # Errors
            ///
            /// As [`Identity::parse`].
            pub fn parse(
                value: &str,
                subject: Subject,
                field: &str,
            ) -> Result<Self, ChangeAssuranceError> {
                Identity::parse(value, subject, field).map(Self)
            }

            /// The stored spelling.
            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::fmt::Display::fmt(&self.0, formatter)
            }
        }
    };
}

identity_newtype!(
    /// A change-assurance record's stable identity, constant across revisions.
    RecordId
);
identity_newtype!(
    /// A source connection's identity.
    SourceId
);
identity_newtype!(
    /// An impact snapshot's identity.
    ImpactIdentity
);
identity_newtype!(
    /// A requirement, preservation-constraint or unknown's identity within a
    /// record definition.
    StatementId
);
identity_newtype!(
    /// A proof obligation's identity.
    ProofId
);
identity_newtype!(
    /// An evidence obligation a proof discharges.
    ObligationId
);
identity_newtype!(
    /// A proof attestation's own identity, distinct from its digest.
    AttestationId
);
identity_newtype!(
    /// An ix-flow workflow run's identity.
    RunId
);
identity_newtype!(
    /// An ix-flow event's identity.
    EventId
);

/// A record revision: an integer of one or more.
///
/// The oracle carries this as a JSON number and checks `Number.isInteger`.
/// Every arithmetic use of it — `revision - 1`, `index + 1` — is over small
/// counts, so it is a `u64` here. See `DIVERGENCE.md` §2 for the one input
/// this refuses and the oracle does not.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Revision(u64);

impl Revision {
    /// The first revision of a record.
    pub const GENESIS: Self = Self(1);

    /// Read a revision, refusing zero.
    ///
    /// # Errors
    ///
    /// [`ChangeAssuranceError::Shape`] naming `field` when the value is below
    /// one.
    pub fn new(value: u64, subject: Subject, field: &str) -> Result<Self, ChangeAssuranceError> {
        if value >= 1 {
            Ok(Self(value))
        } else {
            Err(ChangeAssuranceError::shape(
                subject,
                FieldFailure::malformed(field),
            ))
        }
    }

    /// The wrapped count.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Whether this is the first revision of its record.
    #[must_use]
    pub const fn is_genesis(self) -> bool {
        self.0 == 1
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
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
    use super::{Identity, MAX_IDENTITY_LENGTH};

    #[test]
    fn identity_accepts_the_oracle_class_and_refuses_everything_else() {
        assert!(Identity::is_valid("FR-063"));
        assert!(Identity::is_valid("a"));
        assert!(Identity::is_valid("0:x/y.z_w-v"));
        assert!(!Identity::is_valid(""));
        assert!(!Identity::is_valid("-leading"));
        assert!(!Identity::is_valid(".leading"));
        assert!(!Identity::is_valid("has space"));
        assert!(!Identity::is_valid("has\u{e9}accent"));
        assert!(Identity::is_valid(&"a".repeat(MAX_IDENTITY_LENGTH)));
        assert!(!Identity::is_valid(&"a".repeat(MAX_IDENTITY_LENGTH + 1)));
    }
}

/// Declare a distinct 64-hex digest newtype with no conversion to any sibling.
///
/// `quoin-store` owns the two digest domains it computes — [`CanonicalDigest`]
/// over canonical JSON bytes, `RawBytesDigest` over bytes as supplied. The
/// domains declared here are neither: they are **references to a digest some
/// other producer computed**, carried through this crate verbatim and never
/// recomputed. Giving them `CanonicalDigest` would assert that this crate
/// could reproduce them, which it cannot.
///
/// [`CanonicalDigest`]: quoin_store::CanonicalDigest
macro_rules! hex_digest_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name(String);

        impl $name {
            /// Read this digest from stored text, refusing anything that is
            /// not exactly 64 lowercase hexadecimal characters.
            ///
            /// # Errors
            ///
            /// [`ChangeAssuranceError::Shape`] naming `field`.
            pub fn parse(
                value: &str,
                subject: Subject,
                field: &str,
            ) -> Result<Self, ChangeAssuranceError> {
                if is_stored_digest(value) {
                    Ok(Self(value.to_owned()))
                } else {
                    Err(ChangeAssuranceError::shape(
                        subject,
                        FieldFailure::malformed(field),
                    ))
                }
            }

            /// The stored spelling.
            #[must_use]
            pub fn as_hex(&self) -> &str {
                &self.0
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

/// Whether `value` is the schema's `DIGEST` shape: 64 lowercase hex digits.
#[must_use]
pub fn is_stored_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

hex_digest_newtype!(
    /// A digest of an artifact outside this crate: a source connection's
    /// revision, an impact snapshot's output, a tool configuration, or a
    /// retained FR-032 audit report. Never recomputed here.
    ArtifactDigest
);
hex_digest_newtype!(
    /// An ix-flow FR-013 event hash. sha256 over the event's canonical JSON,
    /// and so neither of `quoin-store`'s blake3 domains.
    EventHash
);

impl EventHash {
    /// The chain's genesis hash: 64 zeroes, the `prevHash` of a first event.
    #[must_use]
    pub fn genesis() -> Self {
        Self("0".repeat(64))
    }
}
