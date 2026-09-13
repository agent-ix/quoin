// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Where this crate gets its bytes, and the only place it names a filesystem.
//!
//! # Why one trait
//!
//! Plan loading, profile loading, the collection read-back and the
//! raw-evidence check are the same analysis whether the bytes are files under
//! `<repo>/spec/assurance` and `<repo>/spec/evidence` or a map the caller
//! assembled. Only the source of the bytes differs, so only the source of the
//! bytes is abstracted, exactly as `quoin_evidence`'s `EvidenceSource` and
//! `quoin_validators`' `RepoSource` do. The alternative is two copies of the
//! analysis, and a port that duplicates the thing it ported has not ported it.
//!
//! [`DiskMeasurement`] is what the shipped command runs on, so the retained
//! corpus under `spec/evidence/measurements/` exercises the same code the
//! boundary runs. [`MemoryMeasurement`] is what a caller with no filesystem
//! uses, and is what lets this crate's tests state a repository in three lines.
//!
//! # Two path spaces, said out loud
//!
//! - Assurance documents are **repository relative**, `/`-separated:
//!   `spec/assurance/tier1.md`.
//! - Collection names are bare file names inside the measurements directory:
//!   `tier1-2026.json`. The store root is `quoin-store`'s
//!   ([`quoin_store::store::store_root`]) and is not spelled again here.
//! - A [`RawEvidencePath`] is **store-root relative** and has already passed
//!   the lexical guards of `intervention.ts:179-190` before it reaches a
//!   source.
//!
//! # `raw_evidence_file` is a host capability, not a hash function
//!
//! The retained `rawEvidenceFor` reads the file and hashes the bytes
//! (`intervention.ts:116-124`). Hashing is `quoin-store`'s, and `quoin-store`
//! exposes sha256 only over a **path** ([`quoin_store::digest_file_sha256`]) —
//! there is no public bytes-wise sha256 and this crate will not mint a second
//! one (FR-100-CON-4, Stage 6 plan §13.2). So the digest is taken by the
//! source that owns the file, and [`MemoryMeasurement`] refuses with
//! [`MeasurementErrorCode::RawEvidenceUnavailable`] rather than growing a
//! private `sha2` dependency. Closing that gap is `quoin-store`'s ticket.
//!
//! # Why the clock is here too
//!
//! [`Clock`] is the same kind of thing as [`MeasurementSource`]: a host
//! capability the analysis takes rather than reaches for. The store-wide
//! operational write lock (`operational.ts:296-318`) spins against
//! `Date.now() + 10_000`, so a test of its refusal written against the real
//! clock has to sleep ten seconds to see it. Injected, the deadline is stated
//! and the test is instant — which is the only way
//! `tests/tc_472_operational_lock.rs` can assert the deadline at all.
//!
//! # Why this module is a directory
//!
//! The trait and the two hosts that implement it are three responsibilities,
//! and after quoin#471 and quoin#472 both landed here the one file was 534
//! lines — over the 500-line soft ceiling `tests/tc_468_module_sizes.rs`
//! holds at zero. Split the way `validate.ts` was: the contract here, each
//! host beside it. The public paths are unchanged, because what a caller
//! names is `source::DiskMeasurement`, not which file it is written in.

mod disk;
mod memory;

use quoin_store::RawFileSha256Digest;

use crate::error::MeasurementError;
use crate::raw_evidence::RawEvidencePath;

pub use disk::DiskMeasurement;
pub use memory::MemoryMeasurement;

/// Wall time, and the wait between two attempts at a contended resource.
///
/// Two methods because the retained lock needs exactly two things — `Date.now()`
/// and `Atomics.wait(…, 5)` (`operational.ts:299,313`) — and a trait with a
/// method nothing calls is a trait that will grow one.
pub trait Clock {
    /// Milliseconds since the Unix epoch, as `Date.now()` reports them.
    fn now_millis(&self) -> i64;

    /// Pause before the next attempt.
    ///
    /// A [`SystemClock`] sleeps. A test clock advances its own reading instead,
    /// so a ten-second deadline is reached in no time at all.
    fn wait(&self, millis: u32);
}

/// The host's clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_millis(&self) -> i64 {
        // Before 1970 the duration is `Err`, and its magnitude is the answer
        // negated. A clock set behind the epoch is absurd and is still not a
        // reason to panic in a library.
        match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(since) => i64::try_from(since.as_millis()).unwrap_or(i64::MAX),
            Err(before) => {
                i64::try_from(before.duration().as_millis()).map_or(i64::MIN, |millis| -millis)
            }
        }
    }

    fn wait(&self, millis: u32) {
        std::thread::sleep(std::time::Duration::from_millis(u64::from(millis)));
    }
}

/// One retained file, as the raw-evidence accounting sees it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RawEvidenceFile {
    /// The file's size in bytes.
    pub size_bytes: u64,
    /// Its sha256 digest, in the `sha256:<64 hex>` spelling.
    pub digest: RawFileSha256Digest,
}

/// Where measurement inputs are read from.
pub trait MeasurementSource {
    /// Every `.md` document under the repository's assurance roots, as
    /// repository-relative `/`-separated paths, sorted.
    ///
    /// `plans.ts:29-33` looks under `spec/assurance` and then `assurance`,
    /// skipping either if absent, and sorts the union. An absent root is not an
    /// error.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::Io`] when a root exists and cannot be walked.
    fn assurance_documents(&self) -> Result<Vec<String>, MeasurementError>;

    /// The UTF-8 text of one assurance document.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::Io`] when the document cannot be read.
    fn document_text(&self, path: &str) -> Result<String, MeasurementError>;

    /// The `.json` file names directly inside the measurements directory,
    /// sorted.
    ///
    /// An absent directory lists as empty, which is `store.ts:79`'s
    /// `existsSync(root) ? … : []`.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::Io`] when the directory exists and cannot be
    /// listed.
    fn collection_names(&self) -> Result<Vec<String>, MeasurementError>;

    /// The bytes of one stored collection.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::Io`] when the file cannot be read.
    fn collection_bytes(&self, name: &str) -> Result<Vec<u8>, MeasurementError>;

    /// The `.json` file names directly inside the interventions directory,
    /// sorted.
    ///
    /// An absent directory lists as empty, which is `intervention.ts:87`'s
    /// `existsSync(root) ? … : []`.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::Io`] when the directory exists and cannot be
    /// listed.
    fn intervention_names(&self) -> Result<Vec<String>, MeasurementError>;

    /// The bytes of one retained intervention record.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::Io`] when the file cannot be read.
    fn intervention_bytes(&self, name: &str) -> Result<Vec<u8>, MeasurementError>;

    /// The UTF-8 text of one retained raw-evidence file.
    ///
    /// `agent-eval-intervention.ts:221-225` reads a retained report back after
    /// `rawEvidenceFor` has resolved it, so the same path guards apply and the
    /// same refusals are raised.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::RawEvidencePathUnsafe`] for a path that escapes
    /// the store or names something that is not a regular file,
    /// [`MeasurementErrorCode::RawEvidenceUnavailable`] from a source that
    /// holds no files, and [`MeasurementErrorCode::Io`] when the file cannot be
    /// read as UTF-8.
    fn raw_evidence_text(&self, path: &RawEvidencePath) -> Result<String, MeasurementError>;

    /// The size and sha256 digest of one retained raw-evidence file.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::RawEvidencePathUnsafe`] when the path resolves
    /// outside the evidence store or names something that is not a regular
    /// file, and [`MeasurementErrorCode::RawEvidenceUnavailable`] from a source
    /// that holds no files.
    fn raw_evidence_file(
        &self,
        path: &RawEvidencePath,
    ) -> Result<RawEvidenceFile, MeasurementError>;

    /// The UTF-8 text of one retained raw-evidence file.
    ///
    /// `github-release-operational.ts:216-223` reads the three retained
    /// GitHub exports this way, after accounting for them with
    /// [`raw_evidence_file`](Self::raw_evidence_file). Separate from that
    /// method because a producer needs the *content* and the accounting needs
    /// only the size and the digest, and a 15 MiB export should not be held in
    /// memory to be measured.
    ///
    /// # Errors
    ///
    /// [`MeasurementErrorCode::Io`] when the file cannot be read as UTF-8, and
    /// [`MeasurementErrorCode::RawEvidenceUnavailable`] from a source that
    /// holds no files.
    fn retained_evidence_text(&self, path: &RawEvidencePath) -> Result<String, MeasurementError>;
}

/// Whether `name` ends in `.<extension>`.
///
/// `plans.ts:92` and `store.ts:81` both compare the suffix **case
/// sensitively**, so `A.JSON` is not a collection there and is not one here
/// either. Written on the last dot rather than as `ends_with(".json")` because
/// the two agree on every input and only this spelling says which half is the
/// extension.
fn has_extension(name: &str, extension: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, found)| found == extension)
}
