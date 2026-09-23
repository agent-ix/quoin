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

/// Lowercase hex of the blake3 hash of `bytes`. The one blake3 call site.
///
/// Public, and the one exception to "a digest is taken through a typed
/// wrapper". `@agent-ix/quoin` exports `blake3Hex` from its package root, so a
/// library consumer can hash bytes of its own choosing without claiming they
/// are a record, an attestation or a canonicalized value — and the port may
/// not quietly drop a published export (quoin#503). Everything inside this
/// crate goes through [`RawBytesDigest`], [`CanonicalDigest`] and the
/// `digest_*` functions, which is what keeps a domain from being applied to
/// the wrong bytes.
#[must_use]
pub fn blake3_hex(bytes: &[u8]) -> String {
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
    /// Over the evidence store's canonical JSON text of a record, under
    /// SHA-256: the identity an assurance record carries as its own
    /// `recordId` and is named by on disk.
    ///
    /// A different domain from [`Self::RawFileSha256`] even though both are
    /// SHA-256 and both are stored `sha256:`-prefixed. That one answers "what
    /// bytes are in this file"; this one answers "what value is this record",
    /// over bytes recomputed from the value rather than copied. Substituting
    /// one for the other crosses exactly the boundary
    /// `FR-201-canonical-identity-domain` forbids.
    AssuranceRecordSha256,
    /// Over a record identity's UTF-8 bytes, under SHA-256: the *file name* an
    /// operational record or record pair is stored under
    /// (`src/measurement/operational.ts:104,459`).
    ///
    /// A fourth question again. [`Self::RawFileSha256`] answers "what bytes are
    /// in this file" and [`Self::AssuranceRecordSha256`] answers "what value is
    /// this record"; this one answers "what do I call the file", over an
    /// identity string that is not a file and not a canonical record. It is
    /// never stored inside a record and never compared against a digest
    /// member — it is spelled bare, because it is a basename.
    RecordFileNameSha256,
}

impl DigestDomain {
    /// Every domain, in declaration order.
    pub const ALL: [Self; 5] = [
        Self::RawBytes,
        Self::CanonicalJcs,
        Self::RawFileSha256,
        Self::AssuranceRecordSha256,
        Self::RecordFileNameSha256,
    ];

    /// Whether values in this domain are copied exactly rather than recomputed
    /// from a canonical form.
    #[must_use]
    pub const fn is_opaque_bytes(self) -> bool {
        match self {
            Self::RawBytes | Self::RawFileSha256 => true,
            Self::CanonicalJcs | Self::AssuranceRecordSha256 | Self::RecordFileNameSha256 => false,
        }
    }

    /// The stable domain label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RawBytes => "quoin.raw-artifact-bytes",
            Self::CanonicalJcs => "quoin.canonical-jcs",
            Self::RawFileSha256 => "quoin.raw-file-sha256",
            Self::AssuranceRecordSha256 => "quoin.assurance-record-sha256",
            Self::RecordFileNameSha256 => "quoin.record-file-name-sha256",
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
            // Distinct from `RawFileSha256`'s label on purpose: the two are
            // both SHA-256 and both STORED as `sha256:` (which `to_stored`
            // hardcodes, and NFR-025 freezes), so the label is the only place
            // the difference can be said out loud.
            Self::AssuranceRecordSha256 => "sha256-canonical",
            // Never spelled with a prefix on disk — a basename carries no
            // algorithm label. The label exists so a diagnostic can still say
            // which question the value answers.
            Self::RecordFileNameSha256 => "sha256-identity",
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

/// An assurance record's own identity: SHA-256 over the evidence store's
/// canonical JSON text of the record without its `recordId`.
///
/// Minted only by [`digest_assurance_record`]. This is the domain
/// `src/evidence/assurance-records.ts` computes as
/// `createHash("sha256").update(canonicalJson(input))`, stores as
/// `record.recordId`, checks back on read, and names the file
/// `sha256-<64 hex>.json` from. The canonical text includes its trailing
/// newline, because that is what the retained implementation hashes — dropping
/// it would rename every record in every store (NFR-025).
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct AssuranceRecordId(String);

impl AssuranceRecordId {
    /// This value's domain. Always [`DigestDomain::AssuranceRecordSha256`].
    pub const DOMAIN: DigestDomain = DigestDomain::AssuranceRecordSha256;

    /// Read a stored id, which carries its `sha256:` prefix.
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

    /// The bare hex, without the prefix — what the file name carries.
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

impl fmt::Display for AssuranceRecordId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_stored())
    }
}

/// The basename a record is stored under, a SHA-256 over its identity string.
///
/// `src/measurement/operational.ts:459` names an operational record file
/// `createHash("sha256").update(recordId).digest("hex")`, and `:104` names a
/// record *pair* file the same way over `` `${a}\0${b}` ``. Minted only by
/// [`digest_record_file_name`] and [`digest_record_pair_file_name`], so a
/// caller holding one is holding a name that was derived, not assembled.
///
/// Stored **bare**: it is a file name, so it carries no `sha256:` prefix and
/// has no `to_stored` — the absence is the difference from
/// [`AssuranceRecordId`], which is a prefixed member *inside* a record.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RecordFileName(String);

impl RecordFileName {
    /// This value's domain. Always [`DigestDomain::RecordFileNameSha256`].
    pub const DOMAIN: DigestDomain = DigestDomain::RecordFileNameSha256;

    /// The bare hex, which is the file's stem.
    #[must_use]
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    /// This value's domain.
    #[must_use]
    pub const fn domain(&self) -> DigestDomain {
        Self::DOMAIN
    }
}

impl fmt::Display for RecordFileName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The file name one record is stored under.
///
/// Takes the identity as text because that is what is hashed; whether the
/// identity is safe to put in a path is the caller's guard, not this one's.
#[must_use]
pub fn digest_record_file_name(identity: &str) -> RecordFileName {
    RecordFileName(sha256_hex(identity.as_bytes()))
}

/// The file name a linked record *pair* is stored under.
///
/// The two identities are joined by a NUL, which no record identity may
/// contain, so `(a, b)` and `(a\0b, "")` cannot collide.
#[must_use]
pub fn digest_record_pair_file_name(first: &str, second: &str) -> RecordFileName {
    let mut joined = Vec::with_capacity(first.len() + second.len() + 1);
    joined.extend_from_slice(first.as_bytes());
    joined.push(0);
    joined.extend_from_slice(second.as_bytes());
    RecordFileName(sha256_hex(&joined))
}

/// Compute an assurance record's identity from the value it will be stored as.
///
/// Hashes [`canonical_json`](crate::json::pretty::canonical_json) — the
/// two-space, key-sorted, newline-terminated form the evidence store writes —
/// under SHA-256. Pass the record WITHOUT its `recordId` member: the id is
/// taken over the record's content, and including it would make the identity
/// depend on itself.
///
/// # Errors
///
/// As [`canonical_json`](crate::json::pretty::canonical_json): a value with no
/// canonical spelling has no identity either.
pub fn digest_assurance_record(value: &JsonValue) -> Result<AssuranceRecordId, StoreError> {
    crate::json::pretty::canonical_json(value)
        .map(|text| AssuranceRecordId(sha256_hex(text.as_bytes())))
}

/// Digest a file's complete bytes under SHA-256, with the guards a digest over
/// a path needs and the TypeScript does not have.
///
/// Refuses, in order: a symbolic link (a link names a target that can change
/// after the digest is recorded), anything that is not a regular file, a file
/// over [`MAX_DIGESTED_FILE_BYTES`], and a file whose length changed between
/// the open and the read.
///
/// # The open is race-safe (PLAT-985, quoin#600 review)
///
/// An earlier version called [`std::fs::symlink_metadata`] and then
/// [`std::fs::read`] as two separate syscalls, naming the same path twice.
/// Between them, whatever the path names on disk can be swapped: a caller who
/// controls the directory replaces a regular file with a symlink after the
/// stat and before the read, and the read follows it — the same swap
/// `crate::store::apparatus`'s `descend` guards against for the walk, but not,
/// until now, for the digest itself. Worse, if the swap lands a FIFO, a
/// blocking `read` on it never returns until some other process opens the
/// other end, so a hostile checkout can hang the digesting process
/// indefinitely.
///
/// This function opens the path exactly once, with `O_NOFOLLOW` (refuse a
/// symlink at the final component, atomically with the open) and
/// `O_NONBLOCK` (a FIFO opens immediately rather than blocking for a writer),
/// then inspects the *open handle*'s metadata (`fstat`, not `stat`) so the
/// file-type check that follows cannot itself race a second swap. Every
/// subsequent read is against that same handle, and is bounded one byte past
/// the size `fstat` reported, so a file that grows after the check cannot
/// make the read allocate past [`MAX_DIGESTED_FILE_BYTES`].
///
/// On Windows — a release target, where neither flag exists — the open
/// passes `FILE_FLAG_OPEN_REPARSE_POINT` instead, so a symlink is opened as
/// itself rather than followed, and the handle's metadata reports it as a
/// symlink, which the type check refuses.
///
/// # Errors
///
/// Refuses, in that order: a symbolic link, anything that is not a regular
/// file, a file larger than [`MAX_DIGESTED_FILE_BYTES`], and a file whose
/// length changed between the open and the read. Also refuses on any I/O
/// failure opening or reading the file.
pub fn digest_file_sha256(path: &std::path::Path) -> Result<RawFileSha256Digest, StoreError> {
    digest_opened_file(&open_for_digest(path)?, path)
}

/// Open `path` for [`digest_file_sha256`], once, without following a symlink
/// at the final component and without blocking on a FIFO.
fn open_for_digest(path: &std::path::Path) -> Result<std::fs::File, StoreError> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        // From `winbase.h`; std names neither constant, and two flags do not
        // justify a `windows-sys` pin. `BACKUP_SEMANTICS` lets a directory
        // open, so it is refused by the type check as on unix rather than as
        // an I/O failure.
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS);
    }
    options.open(path).map_err(|source| {
        // `O_NOFOLLOW` turns a symlink at the final component into ELOOP
        // rather than a followed open, so that failure is reported under the
        // same code a pre-open `symlink_metadata` check would have used, not
        // folded into the generic `Io` variant.
        if refused_as_symlink(&source) {
            StoreError::DigestSourceIsSymlink {
                path: path.to_path_buf(),
            }
        } else {
            StoreError::Io {
                operation: "open",
                path: path.to_path_buf(),
                source,
            }
        }
    })
}

/// Whether an open failed because `O_NOFOLLOW` met a symlink.
#[cfg(unix)]
fn refused_as_symlink(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(libc::ELOOP)
}

/// Whether an open failed because it met a symlink: never, off unix, where the
/// open does not refuse one and the handle's type check does.
#[cfg(not(unix))]
fn refused_as_symlink(_error: &std::io::Error) -> bool {
    false
}

/// Digest the file `file` is open on. `path` only names it in a refusal:
/// nothing here reads the path again, which is what makes the type check and
/// the read one observation of one file (PLAT-985).
fn digest_opened_file(
    file: &std::fs::File,
    path: &std::path::Path,
) -> Result<RawFileSha256Digest, StoreError> {
    let metadata = file.metadata().map_err(|source| StoreError::Io {
        operation: "fstat",
        path: path.to_path_buf(),
        source,
    })?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        // `O_NOFOLLOW` already refuses a symlink at open on unix; on Windows
        // the reparse point is opened as itself and refused here.
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
    let mut bytes = Vec::new();
    // One byte past the declared size is enough to see that it grew.
    std::io::Read::read_to_end(
        &mut std::io::Read::take(file, declared.saturating_add(1)),
        &mut bytes,
    )
    .map_err(|source| StoreError::Io {
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

/// Hash bytes exactly as supplied, in the raw-file sha256 domain.
///
/// The bytes-wise counterpart of [`digest_file_sha256`], for a caller that
/// already holds the bytes and has no path to hand over — an in-memory
/// measurement source, or a producer accounting for an attachment it just
/// assembled (quoin#484).
///
/// It routes through the same private `sha256_hex` every other sha256 entry
/// point here routes through, so the module header's guarantee — one call site
/// per algorithm — is unchanged by its existence. It is what stops a caller
/// growing a private `sha2` dependency, which FR-100-CON-4 forbids.
///
/// **The domain is the one [`digest_file_sha256`] mints**, deliberately: the
/// question "what are the bytes of this raw evidence" has one answer whether
/// the bytes arrived from a file or from memory. It is *not*
/// [`CanonicalDigest`]'s domain and substituting it for one is the
/// non-substitutability error this module exists to make unspellable.
///
/// Unlike [`digest_file_sha256`] there is no symlink, file-type, size or
/// short-read guard, because there is no file: the caller already holds the
/// bytes and the guards have nothing to protect. A caller that has a *path*
/// must use [`digest_file_sha256`] and keep them.
#[must_use]
pub fn digest_bytes_sha256(bytes: &[u8]) -> RawFileSha256Digest {
    RawFileSha256Digest(sha256_hex(bytes))
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
        AssuranceRecordId, CanonicalDigest, DigestDomain, RawBytesDigest, RawFileSha256Digest,
        digest_assurance_record, digest_bytes_sha256, digest_canonical_value, digest_file_sha256,
        digest_opened_file, digest_raw_bytes, digest_record, open_for_digest, verify_record_digest,
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
        assert!(!DigestDomain::AssuranceRecordSha256.is_opaque_bytes());
    }

    /// An assurance record's id is SHA-256 over the store's canonical JSON
    /// text — the two-space, key-sorted, newline-terminated form — and not
    /// over the RFC 8785 bytes the blake3 canonical domain uses.
    ///
    /// The literal below is the digest `src/evidence/assurance-records.ts`
    /// computes for the same value, so a change to either serializer or to the
    /// trailing newline renames every record in every store (NFR-025).
    ///
    /// Trace: FR-048-AC-1, FR-100-CON-4
    #[test]
    fn tc_456_assurance_record_ids_hash_the_canonical_json_text() {
        let value = parse_strict_json_str(r#"{"b":1,"a":[true,null]}"#).unwrap();
        let id = digest_assurance_record(&value).unwrap();
        let text = crate::json::pretty::canonical_json(&value).unwrap();
        assert_eq!(
            text,
            "{\n  \"a\": [\n    true,\n    null\n  ],\n  \"b\": 1\n}\n"
        );
        assert_eq!(
            id.to_stored(),
            format!("sha256:{}", super::sha256_hex(text.as_bytes()))
        );
        assert_eq!(id.domain(), DigestDomain::AssuranceRecordSha256);
        // Not the JCS bytes: a different question, and a different answer.
        assert_ne!(
            id.as_hex(),
            super::sha256_hex(&canonical_bytes(&value).unwrap())
        );
        assert_eq!(
            AssuranceRecordId::parse_stored(&id.to_stored()).unwrap(),
            id
        );
        assert!(AssuranceRecordId::parse_stored(id.as_hex()).is_err());
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

    /// A FIFO with no writer is refused as not a regular file, promptly:
    /// `O_NONBLOCK` makes the open itself return instead of waiting for a
    /// writer, and the type check on the open handle refuses it before any
    /// read. Without `O_NONBLOCK` the open blocks until a writer appears, so
    /// the call runs on a thread with a bounded wait and a regression reports
    /// as a failed assertion, not a hung test process. The refusal must be
    /// the type refusal: an `Io` failure would mean the open or read failed
    /// some other way and proves nothing about the type check.
    ///
    /// Trace: FR-110-AC-10
    /// Provenance: PLAT-985, quoin#600
    #[cfg(unix)]
    #[test]
    fn tc_985_a_fifo_with_no_writer_fails_fast_rather_than_blocking() {
        let dir = tempdir("fifo");
        let fifo = dir.join("blocking.pipe");
        make_fifo(&fifo);

        let (sender, receiver) = std::sync::mpsc::channel();
        let path = fifo.clone();
        std::thread::spawn(move || {
            let _ = sender.send(digest_file_sha256(&path));
        });
        let outcome = receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect(
                "digest_file_sha256 blocked past the timeout instead of failing fast on a FIFO \
                 with no writer",
            );
        let error = outcome.expect_err("a FIFO is not a regular file and must be refused");
        assert_eq!(error.code(), StoreErrorCode::DigestSourceNotRegularFile);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// The type check and the read observe the file that was opened, not
    /// whatever the path names afterwards. The file is opened, then the path
    /// is swapped — first for a FIFO, then for a symlink to other bytes —
    /// before the handle is digested; the digest is still the opened file's.
    /// A digest that re-read the path (the `symlink_metadata`-then-`read`
    /// shape this replaced) would see the swap and refuse, block, or digest
    /// the other bytes.
    ///
    /// Trace: FR-110-AC-10
    /// Provenance: PLAT-985, quoin#600
    #[cfg(unix)]
    #[test]
    fn tc_985_the_digest_reads_the_opened_handle_not_the_path() {
        let dir = tempdir("swap");
        let path = dir.join("answers.json");
        let other = dir.join("other.json");
        std::fs::write(&other, b"xyz").expect("write");
        let abc = "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

        std::fs::write(&path, b"abc").expect("write");
        let file = open_for_digest(&path).expect("opens");
        std::fs::remove_file(&path).expect("unlink");
        make_fifo(&path);
        let digest = digest_opened_file(&file, &path).expect("digests the opened file");
        assert_eq!(digest.to_stored(), abc);

        std::fs::remove_file(&path).expect("unlink");
        std::fs::write(&path, b"abc").expect("write");
        let file = open_for_digest(&path).expect("opens");
        std::fs::remove_file(&path).expect("unlink");
        std::os::unix::fs::symlink(&other, &path).expect("symlink");
        let digest = digest_opened_file(&file, &path).expect("digests the opened file");
        assert_eq!(digest.to_stored(), abc);

        // The path as it now reads is refused, so the two answers above came
        // from the handle and not from a second look at the path.
        let error = digest_file_sha256(&path).expect_err("a link is not its target");
        assert_eq!(error.code(), StoreErrorCode::DigestSourceIsSymlink);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A fresh directory under the system temporary directory, unique to this
    /// process and `label`.
    #[cfg(unix)]
    fn tempdir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("quoin-store-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    #[cfg(unix)]
    fn make_fifo(path: &std::path::Path) {
        let made = std::process::Command::new("mkfifo")
            .arg(path)
            .status()
            .expect("mkfifo is on PATH in this test environment");
        assert!(made.success(), "mkfifo {path:?} failed");
    }

    /// The bytes-wise sha256 and the file-wise one answer with the same value
    /// for the same bytes, and both agree with the published NIST vector for
    /// `"abc"`.
    ///
    /// The literal is asserted rather than re-derived: comparing
    /// `digest_bytes_sha256` only against `digest_file_sha256` would agree with
    /// itself if `sha256_hex` were wrong, because both route through it.
    ///
    /// Trace: FR-098-CON-1, FR-100-CON-4
    /// Provenance: quoin#484
    #[test]
    fn tc_484_the_bytes_wise_sha256_agrees_with_the_file_wise_one() {
        const NIST_ABC: &str =
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(digest_bytes_sha256(b"abc").to_stored(), NIST_ABC);

        let dir = std::env::temp_dir().join(format!("quoin-store-bytes-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let file = dir.join("payload.bin");
        std::fs::write(&file, b"abc").expect("write");
        assert_eq!(
            digest_file_sha256(&file).expect("digests"),
            digest_bytes_sha256(b"abc")
        );
        std::fs::remove_dir_all(&dir).ok();

        // Empty input is a value, not a missing one: the accounting must be
        // able to state that a zero-byte attachment was seen.
        assert_eq!(
            digest_bytes_sha256(b"").to_stored(),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        // Same bytes, different algorithm, different answer: the domain is not
        // decorative.
        assert_ne!(
            digest_bytes_sha256(b"abc").as_hex(),
            digest_raw_bytes(b"abc").as_hex()
        );
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
