// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The identity newtypes this domain refuses to carry as bare `String`.
//!
//! # Parse, don't validate
//!
//! `quoin_store::RawFileSha256Digest::parse_stored` is the in-repo model: it
//! returns the type, and holding the type *is* the proof. Every constructor
//! here does the same. The retained TypeScript instead validates with
//! predicates and keeps passing the `string` around
//! (`store.ts:104-109`'s `safeId`, `validate.ts:118-124`'s digest regexes), so
//! every consumer re-checks or forgets to.
//!
//! `RawFileSha256Digest` is used directly for the three digest-bearing fields
//! rather than wrapped again — `verificationStack.lockDigest`,
//! `.executableDigest` and each `.artifacts` member all carry the
//! `sha256:<64 hex>` spelling that type already owns.

use std::fmt;

use crate::error::{MeasurementError, MeasurementErrorCode};

/// A non-empty string, which is what the retained validator checks for a dozen
/// members it otherwise leaves as `string`.
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct NonEmptyText(String);

impl NonEmptyText {
    /// Read a non-empty string.
    ///
    /// # Errors
    ///
    /// `code` with `field` as the subject when `value` is empty.
    pub fn parse(
        value: &str,
        code: MeasurementErrorCode,
        field: &str,
    ) -> Result<Self, MeasurementError> {
        if value.is_empty() {
            return Err(MeasurementError::new(
                code,
                format!("requires non-empty `{field}`"),
            ));
        }
        Ok(Self(value.to_owned()))
    }

    /// The text.
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

/// A measurement collection's identity, which is also its file name.
///
/// `store.ts:104-109` refuses anything outside `[A-Za-z0-9._-]+` because the id
/// is interpolated straight into a path. The refusal belongs to the value, not
/// to the write, so it is here and the write cannot forget it.
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct CollectionId(String);

impl CollectionId {
    /// Read a collection id.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::CollectionIdUnsafe`] for an empty id or one
    /// carrying any byte outside `[A-Za-z0-9._-]`.
    pub fn parse(value: &str) -> Result<Self, MeasurementError> {
        let safe = !value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
        if safe {
            Ok(Self(value.to_owned()))
        } else {
            Err(MeasurementError::new(
                MeasurementErrorCode::CollectionIdUnsafe,
                format!("unsafe measurement collection id `{value}`"),
            ))
        }
    }

    /// The id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// This collection's store-root-relative file path.
    #[must_use]
    pub fn store_path(&self) -> String {
        format!("measurements/{}.json", self.0)
    }
}

impl fmt::Display for CollectionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The largest record identity that may be turned into a file name.
///
/// `operational.ts:455` and `intervention.ts:34` both cap the identity at a
/// leading character plus 127 more. The schema caps identity bytes at 128
/// ASCII characters for the same reason: either encoding's basename has to
/// stay under `NAME_MAX`.
pub const MAX_RECORD_ID_BYTES: usize = 128;

/// A record identity that is safe to derive a file name from.
///
/// `^[A-Za-z0-9][A-Za-z0-9._:/-]{0,127}$`, the **shared** first guard of
/// `operational.ts:455` and `intervention.ts:34`. The two files differ only in
/// what they do next — operational hashes the identity, intervention encodes
/// it into `p-`/`b-` namespaces — so the guard is declared once here rather
/// than twice in `operational/` and `intervention/`.
///
/// Holding this type is the proof. Note what it does **not** assert: that the
/// identity is free of `/` or `:`. Those are admitted by the retained regex
/// and neither consumer interpolates the identity into a path unencoded.
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct SafeRecordId(String);

impl SafeRecordId {
    /// Read a record identity a file name may be derived from.
    ///
    /// # Errors
    ///
    /// `code` when the identity is empty, over [`MAX_RECORD_ID_BYTES`], starts
    /// with anything but an ASCII alphanumeric, or carries a byte outside
    /// `[A-Za-z0-9._:/-]`.
    pub fn parse(value: &str, code: MeasurementErrorCode) -> Result<Self, MeasurementError> {
        let mut bytes = value.bytes();
        let safe = value.len() <= MAX_RECORD_ID_BYTES
            && bytes
                .next()
                .is_some_and(|byte| byte.is_ascii_alphanumeric())
            && bytes.all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
            });
        if safe {
            Ok(Self(value.to_owned()))
        } else {
            Err(MeasurementError::new(
                code,
                format!("unsafe record id {value:?}"),
            ))
        }
    }

    /// The identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SafeRecordId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// A full 40-character lowercase git object name.
///
/// `validate.ts:137` checks `^[0-9a-f]{40}$` on every
/// `verificationStack.sources.*.revision`; an abbreviated revision is not a
/// source attestation, it is a hint.
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct FullGitRevision(String);

impl FullGitRevision {
    /// Read a full git revision.
    ///
    /// # Errors
    ///
    /// `code` when `value` is not exactly 40 lowercase hexadecimal characters.
    pub fn parse(
        value: &str,
        code: MeasurementErrorCode,
        subject: &str,
    ) -> Result<Self, MeasurementError> {
        let full = value.len() == 40
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        if full {
            Ok(Self(value.to_owned()))
        } else {
            Err(MeasurementError::new(code, subject.to_owned()))
        }
    }

    /// The revision.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FullGitRevision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#[cfg(test)]
mod tests {
    use super::{CollectionId, FullGitRevision, MAX_RECORD_ID_BYTES, NonEmptyText, SafeRecordId};
    use crate::error::MeasurementErrorCode;

    #[test]
    fn a_collection_id_that_could_name_another_directory_is_refused() {
        assert!(CollectionId::parse("tier1-2026").is_ok());
        // `..` is accepted, because `safeId`'s `^[A-Za-z0-9._-]+$` accepts it
        // and `..` names the file `..json`, not a parent directory. The port
        // keeps the oracle's predicate rather than tightening it.
        assert!(CollectionId::parse("..").is_ok());
        for unsafe_id in ["", "a/b", "a b", "a:b", "a\\b", "tier1+2026"] {
            let error = CollectionId::parse(unsafe_id).unwrap_err();
            assert_eq!(error.code(), MeasurementErrorCode::CollectionIdUnsafe);
        }
    }

    #[test]
    fn a_collection_id_names_its_own_store_path() {
        assert_eq!(
            CollectionId::parse("tier1-2026").unwrap().store_path(),
            "measurements/tier1-2026.json"
        );
    }

    #[test]
    fn an_abbreviated_or_upper_case_revision_is_not_a_full_one() {
        let code = MeasurementErrorCode::CollectionInvalid;
        assert!(FullGitRevision::parse(&"a".repeat(40), code, "s").is_ok());
        assert!(FullGitRevision::parse(&"a".repeat(39), code, "s").is_err());
        assert!(FullGitRevision::parse(&"A".repeat(40), code, "s").is_err());
        assert!(FullGitRevision::parse(&"g".repeat(40), code, "s").is_err());
    }

    /// The cap is 128 bytes inclusive, and it is measured in bytes.
    #[test]
    fn a_record_id_is_refused_at_the_byte_the_regex_refuses_it() {
        let code = MeasurementErrorCode::CollectionInvalid;
        assert_eq!(MAX_RECORD_ID_BYTES, 128);
        assert!(SafeRecordId::parse(&"a".repeat(128), code).is_ok());
        assert!(SafeRecordId::parse(&"a".repeat(129), code).is_err());
        // The retained regex admits `:` and `/`; the encoding downstream is
        // what keeps them out of a path.
        assert!(SafeRecordId::parse("a/b:c-d_e.f", code).is_ok());
        for refused in [
            "", "-leading", ".leading", "/leading", "a b", "a+b", "a\\b", "é",
        ] {
            assert!(
                SafeRecordId::parse(refused, code).is_err(),
                "{refused:?} was accepted"
            );
        }
    }

    #[test]
    fn empty_text_is_refused_and_names_its_field() {
        let error = NonEmptyText::parse("", MeasurementErrorCode::CollectionInvalid, "subject")
            .unwrap_err();
        assert!(error.subject().contains("subject"));
    }
}
