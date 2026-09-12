// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Digest domains, and the one digest function.
//!
//! # One function per algorithm, each in one place
//!
//! [`blake3_hex`] and [`sha256_hex`] are private and are the only two places
//! bytes are hashed in this crate or in anything built on it. Every public
//! entry point below routes through one of them. If a third hashing call site
//! appears, this module's guarantee is gone.
//!
//! There are two algorithms because Quoin already has two, and modelling one
//! would not have removed the other:
//!
//! * **blake3**, bare 64-hex with no prefix — the change-assurance family.
//! * **sha256**, always `sha256:`-prefixed *in the role this crate models* —
//!   measurement raw evidence (`rawEvidenceFor`), intervention records, and
//!   the `quire` binary pin. sha256 carries other roles in the retained tree
//!   that this crate does not model; see *The sha256 roles this crate does not
//!   cover* below.
//!
//! In the retained stores those two are told apart *only* by the presence of a
//! prefix. That is an invariant nothing enforces, and
//! `src/measurement/intervention-schema.ts:9` already admits a `blake3:`
//! spelling in a slot whose sibling `digest` schema is `sha256:`-only. Here the
//! algorithm is part of the type, so the distinction survives without the
//! spelling.
//!
//! # Three domains, and why they are separate types
//!
//! `quire-protocol`'s `FR-201-canonical-identity-domain` states the rule this
//! module obeys: a digest over *raw supplied bytes* and a digest over *RFC 8785
//! canonical bytes* answer different questions and are **non-substitutable**,
//! even when they are the same algorithm and even when, for some particular
//! input, they are the same 64 characters.
//!
//! Quoin holds all three:
//!
//! * [`RawBytesDigest`] — `ProofAttestation.retained_output.digest`, taken over
//!   a producer's `output.bin` exactly as supplied. These bytes are never
//!   recanonicalized; they may not even be JSON.
//! * [`CanonicalDigest`] — `ChangeAssuranceRecord.digest`, taken over the
//!   canonical bytes of the record with its own `digest` member removed.
//! * [`RawFileSha256Digest`] — the `sha256:`-prefixed references measurement
//!   records carry for files on disk: the *measurement raw-evidence* role
//!   specifically, and only that role. A different algorithm *and* a different
//!   domain from the first two.
//!
//! They are distinct types with no `From`, no `Into`, no shared trait that
//! yields a value another accepts, and no constructor that turns one into
//! another. Passing a record digest where a retained-output digest belongs does
//! not compile. That is the entire point: the two blake3 domains are
//! indistinguishable as text, so text is the wrong place to enforce the
//! distinction.
//!
//! Note carefully that `digest_raw_bytes(&canonical_bytes(v))` and
//! `digest_canonical_value(v)` produce the **same 64 characters**. That is not
//! a flaw in the design, it is the reason the design is needed: nothing about
//! the value tells you which question it answers.
//!
//! # The sha256 roles this crate does not cover
//!
//! Three domains are modelled here; the retained tree has more. Do not read
//! the list above as a census of sha256 in Quoin — [`RawFileSha256Digest`]
//! covers the measurement raw-evidence role alone, and at least two further
//! sha256 roles exist that this crate does not implement:
//!
//! * **Assurance-record identity and file names.**
//!   `src/evidence/assurance-records.ts` hashes an assurance record's own
//!   canonical JSON with sha256 (`createHash("sha256").update(canonicalJson(input))`),
//!   stores the result as `record.recordId` and checks it back on read, names
//!   the record file `sha256-<64 hex>.json` from that id, and validates every
//!   cross-record citation against the same `sha256:<64 lowercase hex>`
//!   pattern. That is an *identity over canonical bytes* domain — the sha256
//!   analogue of [`CanonicalDigest`], not of [`RawFileSha256Digest`] — and the
//!   `src/evidence/assurance-records.ts` store it belongs to is a different
//!   store from the `src/change-assurance/` one this crate reads.
//! * **Schema pinning.** `src/quire/contract.ts` hashes a vendored schema file
//!   with sha256 (`schemaHash`) to pin the contract to the schema bytes on
//!   disk.
//!
//! Neither has a type here, and neither should be expressed with one of the
//! three above: a reader who substitutes [`RawFileSha256Digest`] for an
//! assurance-record id would be crossing exactly the domain boundary
//! `FR-201-canonical-identity-domain` forbids.
//!
//! # Stored spelling
//!
//! The two blake3 domains are stored as bare lowercase hex, 64 characters, with
//! no algorithm prefix. The sha256 domain is stored with its `sha256:` prefix.
//! Both spellings are frozen (see `COMPATIBILITY.md`).
//!
//! For the blake3 types, [`Display`](std::fmt::Display) renders the *labelled*
//! form (`blake3:…` / `blake3-jcs:…`) for cross-ecosystem reference; it is never
//! what goes on disk, so use [`RawBytesDigest::as_hex`] /
//! [`CanonicalDigest::as_hex`] when writing. For
//! [`RawFileSha256Digest`], `Display` and
//! [`to_stored`](RawFileSha256Digest::to_stored) agree, because there the
//! prefix *is* the stored identity.

use std::fmt;

use crate::error::StoreError;
use crate::json::jcs::canonical_bytes;
use crate::json::value::{JsonObject, JsonValue};

/// The member name a sealed record carries its own digest under.
pub const DIGEST_MEMBER: &str = "digest";

/// The one blake3 call site. Private on purpose.
fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// The one sha256 call site. Private on purpose.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in out {
        use std::fmt::Write as _;
        // `write!` to a String cannot fail; the result is discarded rather than
        // unwrapped so no formatting call can panic on a persistence path.
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// The question a digest value answers.
///
/// The domain is fixed by *what was hashed*, never by how the value is spelled,
/// so a cross-domain substitution is invisible to any syntactic check.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DigestDomain {
    /// Over the complete supplied bytes of an artifact, including layout.
    /// Producer-owned; copied exactly and never recanonicalized.
    RawBytes,
    /// Over the RFC 8785 canonical UTF-8 bytes of a JSON value Quoin produced.
    CanonicalJcs,
    /// Over the complete bytes of a file on disk, under SHA-256, as
    /// measurement records and contract pins reference them.
    RawFileSha256,
}

impl DigestDomain {
    /// Every domain, in declaration order.
    pub const ALL: [Self; 3] = [Self::RawBytes, Self::CanonicalJcs, Self::RawFileSha256];

    /// Whether values in this domain are copied exactly rather than recomputed
    /// from a canonical form.
    #[must_use]
    pub const fn is_opaque_bytes(self) -> bool {
        match self {
            Self::RawBytes | Self::RawFileSha256 => true,
            Self::CanonicalJcs => false,
        }
    }

    /// The stable domain label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RawBytes => "quoin.raw-artifact-bytes",
            Self::CanonicalJcs => "quoin.canonical-jcs",
            Self::RawFileSha256 => "quoin.raw-file-sha256",
        }
    }

    /// The algorithm label a value in this domain is spelled with when it is
    /// spelled at all. Deliberately different spellings: rendering both as
    /// `blake3:` is what makes a cross-domain value look merely divergent.
    #[must_use]
    pub const fn algorithm_label(self) -> &'static str {
        match self {
            Self::RawBytes => "blake3",
            Self::CanonicalJcs => "blake3-jcs",
            Self::RawFileSha256 => "sha256",
        }
    }
}

impl fmt::Display for DigestDomain {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Accept only the stored spelling: 64 lowercase hexadecimal characters.
fn parse_stored_hex(value: &str) -> Result<String, StoreError> {
    let well_formed = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if well_formed {
        Ok(value.to_owned())
    } else {
        Err(StoreError::DigestMalformed {
            value: value.to_owned(),
        })
    }
}

/// A digest over bytes supplied by a producer, hashed exactly as supplied.
///
/// There is no constructor that takes a [`JsonValue`]: a raw-bytes digest is
/// never computed from a canonicalized value.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RawBytesDigest(String);

impl RawBytesDigest {
    /// This value's domain. Always [`DigestDomain::RawBytes`].
    pub const DOMAIN: DigestDomain = DigestDomain::RawBytes;

    /// Read a stored raw-bytes digest.
    ///
    /// Naming the domain is mandatory: there is no domain-free parse that a
    /// caller could accidentally feed to the other type.
    ///
    /// # Errors
    ///
    /// Refuses a value that is not exactly 64 lowercase hexadecimal
    /// characters, including one that carries a `blake3:` or `sha256:` label:
    /// the stored spelling of this domain is bare.
    pub fn parse_stored(value: &str) -> Result<Self, StoreError> {
        parse_stored_hex(value).map(Self)
    }

    /// The stored spelling: 64 lowercase hex characters, no prefix.
    #[must_use]
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// The labelled form, `blake3:<hex>`, for diagnostics and cross-ecosystem
    /// reference. **Never** what goes on disk — see [`Self::as_hex`].
    #[must_use]
    pub fn to_labelled(&self) -> String {
        format!("{}:{}", Self::DOMAIN.algorithm_label(), self.0)
    }

    /// This value's domain.
    #[must_use]
    pub const fn domain(&self) -> DigestDomain {
        Self::DOMAIN
    }
}

/// A digest over the RFC 8785 canonical bytes of a JSON value.
///
/// There is no constructor that takes arbitrary bytes: a canonical digest is
/// never computed from bytes that did not come out of the canonicalizer.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CanonicalDigest(String);

impl CanonicalDigest {
    /// This value's domain. Always [`DigestDomain::CanonicalJcs`].
    pub const DOMAIN: DigestDomain = DigestDomain::CanonicalJcs;

    /// Read a stored canonical digest.
    ///
    /// # Errors
    ///
    /// Refuses a value that is not exactly 64 lowercase hexadecimal
    /// characters, including one that carries a label: the stored spelling of
    /// this domain is bare.
    pub fn parse_stored(value: &str) -> Result<Self, StoreError> {
        parse_stored_hex(value).map(Self)
    }

    /// The stored spelling: 64 lowercase hex characters, no prefix.
    #[must_use]
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// The labelled form, `blake3-jcs:<hex>`, for diagnostics and
    /// cross-ecosystem reference. **Never** what goes on disk — see
    /// [`Self::as_hex`].
    #[must_use]
    pub fn to_labelled(&self) -> String {
        format!("{}:{}", Self::DOMAIN.algorithm_label(), self.0)
    }

    /// This value's domain.
    #[must_use]
    pub const fn domain(&self) -> DigestDomain {
        Self::DOMAIN
    }
}

/// The largest file this crate will digest.
///
/// A ceiling, not a guess: without one, a path that happens to name a device or
/// a very large artifact turns a digest call into an unbounded read on whatever
/// thread asked for it. 256 MiB is four times the largest payload Quoin has
/// been measured to move (a 1,090,714-byte producer output, quoin#164) with
/// room to spare.
pub const MAX_DIGESTED_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// A SHA-256 digest of a file's complete bytes, as measurement records spell it.
///
/// Minted only by [`digest_file_sha256`], which refuses a symlink, a
/// non-regular file, an oversized file and a short read. The TypeScript this
/// replaces (`rawEvidenceFor`, `src/measurement/intervention.ts:115`) is a bare
/// `readFileSync` with none of those guards, so a symlink pointing anywhere is
/// digested happily. That is a deliberate behaviour change, and it can make an
/// input that previously digested now refuse.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RawFileSha256Digest(String);

impl RawFileSha256Digest {
    /// This value's domain. Always [`DigestDomain::RawFileSha256`].
    pub const DOMAIN: DigestDomain = DigestDomain::RawFileSha256;

    /// Read a stored reference, which carries its `sha256:` prefix.
    ///
    /// The prefix is part of the stored identity here, unlike the two blake3
    /// domains, which are stored bare.
    ///
    /// # Errors
    ///
    /// Refuses a value that does not carry the `sha256:` prefix, and one whose
    /// remainder is not exactly 64 lowercase hexadecimal characters.
    pub fn parse_stored(value: &str) -> Result<Self, StoreError> {
        let Some(hex) = value.strip_prefix("sha256:") else {
            return Err(StoreError::DigestMalformed {
                value: value.to_owned(),
            });
        };
        parse_stored_hex(hex).map(Self)
    }

    /// The bare hex, without the prefix.
    #[must_use]
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// The stored spelling, `sha256:` prefix included.
    #[must_use]
    pub fn to_stored(&self) -> String {
        format!("sha256:{}", self.0)
    }

    /// This value's domain.
    #[must_use]
    pub const fn domain(&self) -> DigestDomain {
        Self::DOMAIN
    }
}

impl fmt::Display for RawFileSha256Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_stored())
    }
}

/// Digest a file's complete bytes under SHA-256, with the guards a digest over
/// a path needs and the TypeScript does not have.
///
/// Refuses, in order: a symbolic link (a link names a target that can change
/// after the digest is recorded), anything that is not a regular file, a file
/// over [`MAX_DIGESTED_FILE_BYTES`], and a file whose length changed between
/// the stat and the read.
///
/// # Errors
///
/// Refuses, in that order: a symbolic link, anything that is not a regular
/// file, a file larger than [`MAX_DIGESTED_FILE_BYTES`], and a file whose
/// length changed between the stat and the read. Also refuses on any I/O
/// failure reading the file.
pub fn digest_file_sha256(path: &std::path::Path) -> Result<RawFileSha256Digest, StoreError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| StoreError::Io {
        operation: "stat",
        path: path.to_path_buf(),
        source,
    })?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return Err(StoreError::DigestSourceIsSymlink {
            path: path.to_path_buf(),
        });
    }
    if !file_type.is_file() {
        return Err(StoreError::DigestSourceNotRegularFile {
            path: path.to_path_buf(),
            kind: if file_type.is_dir() {
                "directory"
            } else {
                "not a regular file"
            },
        });
    }
    let declared = metadata.len();
    if declared > MAX_DIGESTED_FILE_BYTES {
        return Err(StoreError::DigestSourceTooLarge {
            path: path.to_path_buf(),
            size: declared,
            limit: MAX_DIGESTED_FILE_BYTES,
        });
    }
    let bytes = std::fs::read(path).map_err(|source| StoreError::Io {
        operation: "read",
        path: path.to_path_buf(),
        source,
    })?;
    let actual = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if actual != declared {
        return Err(StoreError::DigestSourceChangedWhileReading {
            path: path.to_path_buf(),
            expected: declared,
            actual,
        });
    }
    Ok(RawFileSha256Digest(sha256_hex(&bytes)))
}

/// Hash bytes exactly as supplied.
#[must_use]
pub fn digest_raw_bytes(bytes: &[u8]) -> RawBytesDigest {
    RawBytesDigest(blake3_hex(bytes))
}

/// Hash the RFC 8785 canonical bytes of a value.
///
/// # Errors
///
/// [`StoreError::JsonNestingTooDeep`] when `value` nests deeper than the
/// canonicalizer's budget; there is no digest for bytes that cannot be
/// produced.
pub fn digest_canonical_value(value: &JsonValue) -> Result<CanonicalDigest, StoreError> {
    canonical_bytes(value).map(|bytes| CanonicalDigest(blake3_hex(&bytes)))
}

/// Hash a record the way a sealed record's own `digest` member is computed:
/// the top-level `digest` member is removed first, then the remainder is
/// canonicalized and hashed.
///
/// # Errors
///
/// As [`digest_canonical_value`].
pub fn digest_record(record: &JsonObject) -> Result<CanonicalDigest, StoreError> {
    let mut unsigned = record.clone();
    unsigned.remove(DIGEST_MEMBER);
    digest_canonical_value(&JsonValue::Object(unsigned))
}

/// Check a sealed record against its own `digest` member.
///
/// Returns the recomputed digest on agreement.
///
/// # Errors
///
/// Refuses a record carrying no `digest` member, one whose `digest` is not a
/// stored canonical digest, and one whose recomputed digest disagrees with
/// the stored one. Also propagates [`digest_canonical_value`]'s refusals.
pub fn verify_record_digest(record: &JsonObject) -> Result<CanonicalDigest, StoreError> {
    let stored = record
        .get(DIGEST_MEMBER)
        .ok_or(StoreError::DigestMissing)?
        .as_str()
        .ok_or(StoreError::DigestMalformed {
            value: String::new(),
        })?;
    let stored = CanonicalDigest::parse_stored(stored)?;
    let recomputed = digest_record(record)?;
    if stored == recomputed {
        Ok(recomputed)
    } else {
        Err(StoreError::DigestMismatch {
            stored: stored.as_hex().to_owned(),
            recomputed: recomputed.as_hex().to_owned(),
        })
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
    use super::{
        CanonicalDigest, DigestDomain, RawBytesDigest, RawFileSha256Digest, digest_canonical_value,
        digest_file_sha256, digest_raw_bytes, digest_record, verify_record_digest,
    };
    use crate::error::StoreErrorCode;
    use crate::json::jcs::canonical_bytes;
    use crate::json::parse::parse_strict_json_str;

    /// Trace: FR-098-CON-1
    #[test]
    fn tc_380_every_domain_carries_a_distinct_label() {
        let mut labels: Vec<&str> = DigestDomain::ALL
            .iter()
            .map(|domain| domain.algorithm_label())
            .collect();
        labels.sort_unstable();
        let before = labels.len();
        labels.dedup();
        assert_eq!(before, labels.len(), "two domains share an algorithm label");
        assert!(DigestDomain::RawBytes.is_opaque_bytes());
        assert!(DigestDomain::RawFileSha256.is_opaque_bytes());
        assert!(!DigestDomain::CanonicalJcs.is_opaque_bytes());
    }

    /// The sha256 domain carries its prefix in the stored spelling and the
    /// blake3 domains do not, so a value from one cannot be read as the other.
    ///
    /// Trace: FR-098-CON-1
    #[test]
    fn tc_380_sha256_references_are_prefixed_and_blake3_ones_are_not() {
        let bare = digest_raw_bytes(b"x");
        assert!(RawFileSha256Digest::parse_stored(bare.as_hex()).is_err());
        let prefixed = format!("sha256:{}", "0".repeat(64));
        assert!(RawBytesDigest::parse_stored(&prefixed).is_err());
        assert!(CanonicalDigest::parse_stored(&prefixed).is_err());
        let parsed = RawFileSha256Digest::parse_stored(&prefixed).expect("parses");
        assert_eq!(parsed.to_stored(), prefixed);
    }

    /// The two algorithms produce different values for the same bytes, which is
    /// exactly why a bare-hex reference is not self-describing.
    ///
    /// Trace: FR-098-CON-1
    #[test]
    fn tc_380_file_digest_refuses_symlinks_directories_and_reports_sha256() {
        let dir = std::env::temp_dir().join(format!("quoin-store-sha-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let file = dir.join("payload.bin");
        std::fs::write(&file, b"abc").expect("write");
        let digest = digest_file_sha256(&file).expect("digests");
        assert_eq!(
            digest.to_stored(),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_ne!(digest.as_hex(), digest_raw_bytes(b"abc").as_hex());

        let error = digest_file_sha256(&dir).expect_err("a directory has no bytes");
        assert_eq!(error.code(), StoreErrorCode::DigestSourceNotRegularFile);

        let link = dir.join("payload.link");
        let _ = std::fs::remove_file(&link);
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&file, &link).expect("symlink");
            let error = digest_file_sha256(&link).expect_err("a link is not its target");
            assert_eq!(error.code(), StoreErrorCode::DigestSourceIsSymlink);
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The hex is identical across domains for the same bytes; only the type
    /// keeps them apart. This test exists to document why the type-level guard
    /// is necessary rather than decorative.
    ///
    /// Trace: FR-098-CON-1
    #[test]
    fn tc_380_identical_hex_across_domains_is_kept_apart_only_by_type() {
        let value = parse_strict_json_str(r#"{"a":1}"#).expect("parses");
        let canonical = digest_canonical_value(&value).expect("shallow");
        let raw = digest_raw_bytes(&canonical_bytes(&value).expect("shallow"));
        assert_eq!(canonical.as_hex(), raw.as_hex());
        assert_ne!(canonical.to_labelled(), raw.to_labelled());
        // `canonical == raw` does not compile: they are different types, and
        // there is no `From`/`Into` between them.
    }

    /// Trace: FR-098-CON-1
    #[test]
    fn tc_380_stored_spelling_is_lowercase_hex_of_exactly_64() {
        let digest = digest_raw_bytes(b"");
        assert_eq!(digest.as_hex().len(), 64);
        assert!(RawBytesDigest::parse_stored(digest.as_hex()).is_ok());
        assert!(RawBytesDigest::parse_stored(&digest.as_hex().to_uppercase()).is_err());
        assert!(CanonicalDigest::parse_stored("abc").is_err());
    }

    /// Trace: FR-098-CON-1
    #[test]
    fn tc_380_record_digest_excludes_only_the_top_level_digest_member() {
        let sealed = parse_strict_json_str(
            r#"{"a":1,"digest":"0000000000000000000000000000000000000000000000000000000000000000","b":{"digest":"x"}}"#,
        )
        .expect("parses");
        let object = sealed.as_object().expect("object");
        let without = parse_strict_json_str(r#"{"a":1,"b":{"digest":"x"}}"#).expect("parses");
        assert_eq!(
            digest_record(object).expect("shallow").as_hex(),
            digest_canonical_value(&without).expect("shallow").as_hex()
        );
        assert!(verify_record_digest(object).is_err());
    }
}
