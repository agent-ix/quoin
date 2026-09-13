// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The store's identities, as types rather than as `String`.
//!
//! Every one of these is a `String` on disk and they are freely interchangeable
//! there, which is the problem: a suite name, a commit sha, an obligation id
//! and a record digest all cross the same function boundaries, and the retained
//! TypeScript distinguishes them only by parameter order. `runPath(repo, suite,
//! commit)` called with the arguments swapped is a well-typed call producing a
//! path nobody meant.
//!
//! The newtypes carry the validation that belongs to the identity — a trust
//! decision id is `ETD-<digits>` and nothing else — so the refusal happens
//! where the value is minted rather than at the filesystem.

use crate::error::EvidenceError;

/// The 12-character commit prefix run, scan and inspection filenames use.
///
/// `commit.slice(0, 12)` in the retained source. Not a hash of the commit and
/// not shortened by git's rules: a fixed prefix, frozen because it is the file
/// name (NFR-025).
pub const SHORT_COMMIT_LENGTH: usize = 12;

macro_rules! opaque_id {
    ($name:ident, $what:literal) => {
        #[doc = concat!("A ", $what, ".")]
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Take a ", $what, " from a caller that already has one.")]
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// The value as it is spelled on disk.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

opaque_id!(SuiteId, "suite identity, as the suite registry declares it");
opaque_id!(ObligationId, "obligation id, the join the matrix keys on");
opaque_id!(SymbolId, "FR-051 stable symbol identity");
opaque_id!(
    StatementHash,
    "statement hash as quire computed it; quoin only ever compares these"
);

/// A full commit sha, as the caller reported it.
///
/// Not validated as hexadecimal: the retained store accepts whatever the
/// caller's CI reports, and refusing here would refuse records that already
/// exist on disk.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct Commit(String);

impl Commit {
    /// Take a commit from a caller that already has one.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The full sha.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The 12-character prefix the filenames carry.
    ///
    /// Character-wise, not byte-wise: `slice(0, 12)` in the retained source
    /// counts UTF-16 code units, and for the hexadecimal shas this is ever
    /// called with the two agree. Slicing bytes would panic on a non-ASCII
    /// boundary for an input that the TypeScript merely truncated oddly.
    #[must_use]
    pub fn short(&self) -> String {
        self.0.chars().take(SHORT_COMMIT_LENGTH).collect()
    }
}

impl std::fmt::Display for Commit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A trust decision id, `ETD-<digits>`.
///
/// Validated at construction because it becomes a file name: the retained
/// `trustDecisionPath` refuses anything else rather than letting an id escape
/// the `trust/` directory.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct TrustDecisionId(String);

impl TrustDecisionId {
    /// Read a trust decision id.
    ///
    /// # Errors
    ///
    /// [`EvidenceError::InvalidTrustDecisionId`] for anything that is not
    /// `ETD-` followed by at least one ASCII digit and nothing else.
    pub fn parse(value: impl Into<String>) -> Result<Self, EvidenceError> {
        let value = value.into();
        let digits = value.strip_prefix("ETD-").unwrap_or_default();
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(EvidenceError::InvalidTrustDecisionId { id: value });
        }
        Ok(Self(value))
    }

    /// The id as it is spelled on disk.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TrustDecisionId {
    type Error = EvidenceError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<TrustDecisionId> for String {
    fn from(value: TrustDecisionId) -> Self {
        value.0
    }
}

impl std::fmt::Display for TrustDecisionId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// An assurance profile id, `AP-<digits>`.
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct ProfileId(String);

impl ProfileId {
    /// Read a profile id.
    ///
    /// # Errors
    ///
    /// [`EvidenceError::InvalidProfileId`] for anything that is not `AP-`
    /// followed by at least one ASCII digit and nothing else.
    pub fn parse(value: impl Into<String>) -> Result<Self, EvidenceError> {
        let value = value.into();
        let trimmed = value.trim().to_owned();
        let digits = trimmed.strip_prefix("AP-").unwrap_or_default();
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(EvidenceError::InvalidProfileId { id: value });
        }
        Ok(Self(trimmed))
    }

    /// The id as it is spelled on disk.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ProfileId {
    type Error = EvidenceError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<ProfileId> for String {
    fn from(value: ProfileId) -> Self {
        value.0
    }
}

impl std::fmt::Display for ProfileId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{Commit, ProfileId, TrustDecisionId};

    #[test]
    fn short_takes_the_first_twelve_characters() {
        assert_eq!(Commit::new("0123456789abcdef").short(), "0123456789ab");
        assert_eq!(Commit::new("abc").short(), "abc");
        assert_eq!(Commit::new("").short(), "");
    }

    #[test]
    fn trust_decision_ids_accept_only_the_stored_spelling() {
        assert_eq!(TrustDecisionId::parse("ETD-1").unwrap().as_str(), "ETD-1");
        for bad in ["ETD-", "etd-1", "ETD-1a", "../ETD-1", "ETD-1/x", ""] {
            assert!(TrustDecisionId::parse(bad).is_err(), "accepted {bad}");
        }
    }

    #[test]
    fn profile_ids_are_trimmed_and_validated() {
        assert_eq!(ProfileId::parse(" AP-7 ").unwrap().as_str(), "AP-7");
        assert!(ProfileId::parse("AP-").is_err());
        assert!(ProfileId::parse("BP-1").is_err());
    }
}
