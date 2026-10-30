// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The measurement-side sha256 helpers, and the bounded no-follow file read.
//!
//! Change-assurance records, attestations and receipts are addressed by
//! `(record_id, revision)` or an assigned identity, never by content, so this
//! crate computes no digest over them and holds no second algorithm for them.
//! Experiment and operational records carry an opaque UUID, not a digest of
//! their content.
//!
//! BLOCKED-BY QSL-1275 (owner: planner titan:QSL:plan): `digest_file_sha256`,
//! `digest_bytes_sha256`, `digest_record_file_name` and
//! `digest_record_pair_file_name` stay until their last callers in
//! `quoin-measurement` (`source/`, `campaign/`, `operational/paths.rs`), the
//! `tc_1942`/`tc_1946`/`tc_1947` tests and `quoin-cli` are removed by the
//! measurement-digest PRs that stack on this one; `sha256_hex` is the one call
//! site so that removal is a single edit here.
//!
//! [`read_regular_file_bounded`] is not a digest: it is the no-follow,
//! nonblocking, size-bounded read the campaign store also uses.

use std::fmt;

use crate::error::StoreError;

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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DigestDomain {
    /// Over the complete bytes of a file on disk, under SHA-256, as
    /// measurement records reference them.
    RawFileSha256,
    /// Over a record identity's UTF-8 bytes, under SHA-256: the file name an
    /// operational record or record pair is stored under.
    RecordFileNameSha256,
}

impl DigestDomain {
    /// Every domain, in declaration order.
    pub const ALL: [Self; 2] = [
        Self::RawFileSha256,
        Self::RecordFileNameSha256,
    ];

    /// The stable domain label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RawFileSha256 => "quoin.raw-file-sha256",
            Self::RecordFileNameSha256 => "quoin.record-file-name-sha256",
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
    /// The prefix is part of the stored identity here.
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

/// The basename a record is stored under, a SHA-256 over its identity string.
///
/// `src/measurement/operational.ts:459` names an operational record file
/// `createHash("sha256").update(recordId).digest("hex")`, and `:104` names a
/// record *pair* file the same way over `` `${a}\0${b}` ``. Minted only by
/// [`digest_record_file_name`] and [`digest_record_pair_file_name`], so a
/// caller holding one is holding a name that was derived, not assembled.
///
/// Stored **bare**: it is a file name, so it carries no `sha256:` prefix and
/// has no `to_stored`. A record's own identity is a different thing: an opaque
/// UUID.
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

/// Read one regular file through the same no-follow, nonblocking descriptor
/// used by [`digest_file_sha256`], with an additional caller-owned byte limit.
///
/// # Errors
/// Refuses a linked, nonregular, oversized, changed, or unreadable file.
pub fn read_regular_file_bounded(
    path: &std::path::Path,
    limit: u64,
) -> Result<Vec<u8>, StoreError> {
    read_opened_file_bounded(&open_for_digest(path)?, path, limit)
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
    Ok(RawFileSha256Digest(sha256_hex(&read_opened_file_bounded(
        file,
        path,
        MAX_DIGESTED_FILE_BYTES,
    )?)))
}

fn read_opened_file_bounded(
    file: &std::fs::File,
    path: &std::path::Path,
    limit: u64,
) -> Result<Vec<u8>, StoreError> {
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
    if declared > limit {
        return Err(StoreError::DigestSourceTooLarge {
            path: path.to_path_buf(),
            size: declared,
            limit,
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
    Ok(bytes)
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
/// the assurance-record domain and substituting it for one is the
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

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]
    use ix_trace_rs::trace;

    use super::{digest_bytes_sha256, digest_file_sha256, digest_opened_file, open_for_digest};
    use crate::error::StoreErrorCode;

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
    #[trace("FR-100-AC-11")]
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
    }
}
