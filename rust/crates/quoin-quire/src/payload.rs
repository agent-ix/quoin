// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Bounded ingest and emission of quire payloads (quoin#379).
//!
//! ## Why a bound still exists
//!
//! `src/quire/exec.ts` carries `QUIRE_MAX_BUFFER = 64 MiB` because Node's
//! default `maxBuffer` is 1 MiB and a real corpus already exceeded it:
//! `agent-ix/filament-ide-rs` (268 spec files, 1,107 obligations) emits
//! **1,090,714 bytes** of `quire coverage --json` — 4% over — which killed all
//! six commands that shelled out (agent-ix/quoin#164).
//!
//! A direct library call **changes the failure mode and does not remove the
//! need for a bound**. There is no pipe, so there is no `ENOBUFS`; the report
//! this crate computes is a Rust value whose size is bounded by the corpus on
//! disk. But two surfaces still take bytes whose size the caller does not
//! control:
//!
//! - **reading** a stored artifact (an evidence-store coverage payload, a
//!   supplied `assurance-v1` export, a clause-binding report), which is
//!   untrusted input of arbitrary length (rust-review §10, §11);
//! - **encoding** a computed report to JSON, which allocates.
//!
//! Both go through [`PayloadLimit`]. The default is deliberately the same
//! 64 MiB the TypeScript chose, for the reason it chose it: `#53` projects
//! ~2.5x payload growth, putting the near-term ceiling near 2.7 MB, and 64 MiB
//! is ~25x that again. Unlike `maxBuffer` this is a refusal rather than a kill,
//! so exceeding it names itself.
//!
//! ## Why no schema validator
//!
//! `src/quire/validate.ts` compiled five vendored JSON Schemas with ajv and
//! reported a `ContractViolation` naming each failing instance path. The
//! replacement is `serde`: the payload is read into the engine's own type, so
//! the declaration the payload is checked against and the declaration the
//! engine emits from are the same declaration. `serde_json`'s error names the
//! field and the byte position, which is the actionable half of the ajv report.
//!
//! ### Unknown keys, deliberately tolerated
//!
//! rust-review §10 asks for `#[serde(deny_unknown_fields)]` on externally
//! supplied payloads, and these envelopes do **not** carry it. Two reasons, and
//! both are decisions rather than oversights. The engine's own types do not
//! declare it, and this crate does not get to add a stricter contract to
//! somebody else's type. And the payload contract is additive by design: a
//! consumer written against an older field list must keep reading a payload
//! from a newer engine (that is what `#[serde(default)]` on every optional
//! field is for), so refusing an unknown key would turn every engine release
//! into a consumer outage. The vendored schemas took the opposite posture —
//! `"additionalProperties": false` — and it is precisely why a new engine field
//! was a contract violation in quoin before anybody had added it to
//! `types.ts`. `#[serde(flatten)]` on these two envelopes also disables the
//! attribute outright, so declaring it would be inert as well as wrong.
//!
//! The half that is **not** reproduced is the full ajv error list and its
//! order. That divergence belongs to quoin#378 (`semantic/manifest.ts`'s
//! `mapAjvError`), is not solved here, and is not silently absorbed: this
//! module reports the first shape failure, because that is what a typed
//! deserializer knows.

use std::io::Read;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::engine::Provenance;
use crate::error::{Error, Result};

/// A byte ceiling for one payload.
///
/// A newtype rather than a `u64` parameter so a limit cannot be swapped with
/// an offset or a length at a call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PayloadLimit(u64);

impl PayloadLimit {
    /// 64 MiB — the ceiling `src/quire/exec.ts` set after quoin#164.
    pub const DEFAULT: Self = Self(64 * 1024 * 1024);

    /// A ceiling of `bytes`.
    ///
    /// # Errors
    /// Returns `None` for zero: a zero ceiling refuses every payload,
    /// including a valid empty one, which is a configuration mistake rather
    /// than a policy.
    #[must_use]
    pub const fn bytes(bytes: u64) -> Option<Self> {
        if bytes == 0 { None } else { Some(Self(bytes)) }
    }

    /// The ceiling in bytes.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Refuse a slice that exceeds this ceiling.
    ///
    /// # Errors
    /// [`Error::PayloadTooLarge`].
    pub fn admit_slice(self, subject: &str, bytes: &[u8]) -> Result<()> {
        self.admit(subject, widen(bytes.len()))
    }

    /// Refuse a length that exceeds this ceiling.
    fn admit(self, subject: &str, observed: u64) -> Result<()> {
        if observed > self.0 {
            return Err(Error::PayloadTooLarge {
                subject: subject.to_string(),
                observed,
                limit: self.0,
            });
        }
        Ok(())
    }
}

impl Default for PayloadLimit {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A `quire coverage --json` payload: the engine's report plus the provenance
/// block the emitting surface appends.
///
/// `engine` is `Option` because a report computed in process by
/// `quire_rs::compute_coverage` has none until a surface attaches one — the
/// same reason the published schema makes it optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoveragePayload {
    /// The engine's report, inlined at the top level exactly as quire emits it.
    #[serde(flatten)]
    pub report: quire_rs::CoverageReport,
    /// Which instrument produced it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<Provenance>,
}

impl CoveragePayload {
    /// Attach this build's provenance to a freshly computed report.
    #[must_use]
    pub fn from_report(report: quire_rs::CoverageReport) -> Self {
        Self {
            report,
            engine: Some(Provenance::current()),
        }
    }
}

/// A `quire clauses evaluate --format json` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClauseBindingPayload {
    /// The engine's binding report.
    #[serde(flatten)]
    pub report: quire_rs::ClauseBindingReport,
    /// Which instrument produced it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<Provenance>,
}

impl ClauseBindingPayload {
    /// Attach this build's provenance to a freshly computed report.
    #[must_use]
    pub fn from_report(report: quire_rs::ClauseBindingReport) -> Self {
        Self {
            report,
            engine: Some(Provenance::current()),
        }
    }
}

/// Read a payload of `bytes` into `T`, under `limit`.
///
/// # Errors
/// [`Error::PayloadTooLarge`], [`Error::PayloadNotJson`] or
/// [`Error::PayloadShape`]. The three are distinct variants because a caller
/// acts differently on each: raise the ceiling, look at what the tool printed,
/// or re-measure with a matching engine.
pub fn from_slice<T: DeserializeOwned>(
    subject: &str,
    shape: &'static str,
    bytes: &[u8],
    limit: PayloadLimit,
) -> Result<T> {
    limit.admit(subject, widen(bytes.len()))?;
    // Told apart the way `parseThen` in `src/quire/validate.ts` could not: it
    // reported a JSON syntax error and a shape mismatch as one
    // `ContractViolation`, on the argument that both mean "the tool's output
    // cannot be consumed". They do — but the operator's next move differs, so
    // they are separate variants here. `serde_json` already draws the line, in
    // `Error::classify`; deserializing once and reading that is both cheaper
    // and more precise than parsing to a `Value` first, which would discard
    // the line and column a shape failure carries.
    serde_json::from_slice(bytes).map_err(|error| match error.classify() {
        serde_json::error::Category::Data => Error::PayloadShape {
            subject: subject.to_string(),
            shape,
            reason: error.to_string(),
        },
        _ => Error::PayloadNotJson {
            subject: subject.to_string(),
            reason: error.to_string(),
        },
    })
}

/// `usize` to `u64` without a bare `as` at a boundary (rust-review §7).
///
/// Saturating rather than panicking: a length that does not fit `u64` cannot
/// be under any ceiling this crate will ever be given, so `u64::MAX` is the
/// right answer and not an approximation.
fn widen(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// Read a payload from a file, under `limit`.
///
/// The size is checked twice on purpose: `metadata` gives a precise `observed`
/// for the diagnostic, and the bounded read is the ceiling that actually holds
/// when the file grows between the two calls or reports no length at all
/// (a pipe, a `/proc` entry).
///
/// # Errors
/// [`Error::Io`] plus everything [`from_slice`] reports.
pub fn from_file<T: DeserializeOwned>(
    shape: &'static str,
    path: impl AsRef<Path>,
    limit: PayloadLimit,
) -> Result<T> {
    let path = path.as_ref();
    let bytes = read_bounded(path, limit)?;
    from_slice(&path.display().to_string(), shape, &bytes, limit)
}

/// Read a file's bytes under `limit`.
///
/// The size is checked twice on purpose: `metadata` gives a precise `observed`
/// for the diagnostic, and the bounded read is the ceiling that actually holds
/// when the file grows between the two calls or reports no length at all
/// (a pipe, a `/proc` entry).
///
/// # Errors
/// [`Error::Io`] and [`Error::PayloadTooLarge`].
pub fn read_bounded(path: impl AsRef<Path>, limit: PayloadLimit) -> Result<Vec<u8>> {
    let path = path.as_ref();
    let subject = path.display().to_string();
    let file = std::fs::File::open(path).map_err(|error| Error::Io {
        path: path.to_path_buf(),
        reason: error.to_string(),
    })?;
    if let Ok(metadata) = file.metadata() {
        limit.admit(&subject, metadata.len())?;
    }
    let mut bytes = Vec::new();
    // `+ 1` so an over-long file is detected by having read one byte past the
    // ceiling, rather than being silently truncated to exactly the ceiling and
    // then failing as a JSON syntax error — which is the wrong diagnosis.
    let read = file
        .take(limit.get().saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| Error::Io {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
    limit.admit(&subject, widen(read))?;
    Ok(bytes)
}

/// Encode a payload as JSON under `limit`.
///
/// Bounded **while** encoding, not after: a check on the finished `String`
/// would already have allocated the thing the ceiling exists to prevent.
///
/// # Errors
/// [`Error::PayloadTooLarge`] at the ceiling, or [`Error::PayloadShape`] if the
/// value cannot serialize.
pub fn encode<T: Serialize>(subject: &str, value: &T, limit: PayloadLimit) -> Result<String> {
    let mut writer = LimitedWriter::new(limit);
    match serde_json::to_writer(&mut writer, value) {
        Ok(()) => {}
        Err(error) => {
            if writer.overflowed {
                return Err(Error::PayloadTooLarge {
                    subject: subject.to_string(),
                    observed: limit.get().saturating_add(1),
                    limit: limit.get(),
                });
            }
            return Err(Error::PayloadShape {
                subject: subject.to_string(),
                shape: "JSON",
                reason: error.to_string(),
            });
        }
    }
    String::from_utf8(writer.buffer).map_err(|error| Error::PayloadShape {
        subject: subject.to_string(),
        shape: "JSON",
        reason: error.to_string(),
    })
}

/// A sink that stops at a ceiling instead of growing without one.
struct LimitedWriter {
    buffer: Vec<u8>,
    limit: u64,
    overflowed: bool,
}

impl LimitedWriter {
    fn new(limit: PayloadLimit) -> Self {
        Self {
            buffer: Vec::new(),
            limit: limit.get(),
            overflowed: false,
        }
    }
}

impl std::io::Write for LimitedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let projected = widen(self.buffer.len()).saturating_add(widen(buf.len()));
        if projected > self.limit {
            self.overflowed = true;
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "payload exceeded its ceiling",
            ));
        }
        self.buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    /// Trace: NFR-024
    #[test]
    fn tc_379_030_a_zero_ceiling_is_refused_as_a_configuration_mistake() {
        assert_eq!(PayloadLimit::bytes(0), None);
        assert_eq!(PayloadLimit::bytes(1).expect("non-zero").get(), 1);
        assert_eq!(PayloadLimit::DEFAULT.get(), 64 * 1024 * 1024);
    }

    /// Trace: NFR-024
    #[test]
    fn tc_379_031_an_oversized_slice_names_the_ceiling_not_a_syntax_error() {
        let limit = PayloadLimit::bytes(8).expect("non-zero");
        let error = from_slice::<serde_json::Value>(
            "coverage",
            "CoverageReport",
            b"{\"a\": 1234567890}",
            limit,
        )
        .expect_err("must refuse");
        assert_eq!(error.code(), crate::ErrorCode::PayloadTooLarge);
        let message = error.to_string();
        assert!(message.contains("17"), "{message}");
        assert!(message.contains('8'), "{message}");
    }

    /// Trace: NFR-024
    #[test]
    fn tc_379_032_bad_json_and_bad_shape_are_different_codes() {
        let limit = PayloadLimit::DEFAULT;
        let not_json = from_slice::<CoveragePayload>("coverage", "CoverageReport", b"{", limit)
            .expect_err("must refuse");
        assert_eq!(not_json.code(), crate::ErrorCode::PayloadNotJson);

        let bad_shape =
            from_slice::<CoveragePayload>("coverage", "CoverageReport", b"{\"totals\": 3}", limit)
                .expect_err("must refuse");
        assert_eq!(bad_shape.code(), crate::ErrorCode::PayloadShape);
    }

    /// Trace: NFR-024
    #[test]
    fn tc_379_033_encoding_stops_at_the_ceiling_rather_than_allocating_past_it() {
        let value = vec!["x".repeat(64); 64];
        let error = encode(
            "coverage",
            &value,
            PayloadLimit::bytes(16).expect("non-zero"),
        )
        .expect_err("must refuse");
        assert_eq!(error.code(), crate::ErrorCode::PayloadTooLarge);

        let fine = encode("coverage", &value, PayloadLimit::DEFAULT).expect("encodes");
        assert!(fine.starts_with('['));
    }
}
