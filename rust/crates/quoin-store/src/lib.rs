// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! Canonical JSON, digest domains, and durable evidence-store writes for Quoin.
//!
//! # What this crate is for
//!
//! Digests here are not checksums. They are **identifiers inside retained
//! evidence records**: a change-assurance record is named by its digest, an
//! attestation references its retained output by digest, and both already exist
//! on disk in repositories across the ecosystem. A digest this crate computes
//! differently from the TypeScript it replaces is not a failing test; it is
//! evidence that can no longer be found by the name it was filed under, with no
//! migration back.
//!
//! Everything in the layout below follows from that.
//!
//! # Layout
//!
//! * [`json`] — the strict I-JSON reader and the two canonical writers.
//! * [`digest`] — the one digest function, and the two non-substitutable
//!   digest domains as separate types.
//! * [`store`] — store paths, [`store::STORE_SCHEMA_VERSION`], and durable
//!   atomic writes.
//! * [`replay`] — the cutover gate.
//! * [`error`] — one error enum, one stable code per refusal.
//!
//! # The hard gate (`COMPATIBILITY.md` restates this in full)
//!
//! **No cutover may depend on this crate until every digest in every reachable
//! store has been replayed through both the TypeScript and the Rust
//! implementation with zero mismatches.** The tool that does it is
//! [`replay`], driven by `oracle/capture-store-oracle.mjs` on the TypeScript
//! side and the `quoin-store-replay` binary on the Rust side. "The replay tool
//! works" is not the gate; a reported count of digests replayed with a
//! mismatch count of zero is.
//!
//! # The frozen compatibility surface
//!
//! These are facts about evidence that already exists. They are not open for
//! improvement, and each is asserted by a test:
//!
//! 1. **`STORE_SCHEMA_VERSION == 1`.**
//! 2. **Two canonical serializations, not one.** RFC 8785 JCS for digest bytes
//!    and for the change-assurance family's on-disk form; `canonicalJson`
//!    (two-space indent, `": "` separator, trailing newline) for every other
//!    store file.
//! 3. **JCS member order is UTF-16 code unit order**, not Unicode scalar order.
//! 4. **The pretty form's member order is ECMAScript own-property order**:
//!    array-index names first in ascending numeric order, then the rest in
//!    UTF-16 sorted order. This is not the same as (3) and never will be.
//! 5. **Numbers are ECMAScript `Number::toString`**, including `1e+21` with its
//!    `+`, `100000000000000000000` for `1e20`, and `0` for negative zero.
//! 6. **Every JSON number is an IEEE-754 double**, integers included. Integer
//!    precision beyond 2^53 is already lost in retained evidence.
//! 7. **No Unicode normalization**, ever, on member names or string values.
//! 8. **Change-assurance digests are blake3, stored as 64 lowercase hex
//!    characters with no algorithm prefix. Measurement raw-evidence and
//!    contract-pin digests are sha256, stored `sha256:`-prefixed.** Two
//!    algorithms, told apart in the retained stores only by that prefix; here
//!    the algorithm is part of the type.
//! 9. **A sealed record's digest is taken over the record with its top-level
//!    `digest` member removed** — and only the top-level one.

pub mod digest;
pub mod error;
pub mod json;
pub mod replay;
pub mod store;

pub use digest::{
    CanonicalDigest, DigestDomain, MAX_DIGESTED_FILE_BYTES, RawBytesDigest, RawFileSha256Digest,
    digest_canonical_value, digest_file_sha256, digest_raw_bytes, digest_record,
    verify_record_digest,
};
pub use error::{StoreError, StoreErrorCode};
pub use json::jcs::{canonical_bytes, canonicalize_jcs};
pub use json::parse::{parse_strict_json, parse_strict_json_str};
pub use json::pretty::{canonical_json, canonical_json_bytes};
pub use json::value::{JsonNumber, JsonObject, JsonValue};
pub use store::STORE_SCHEMA_VERSION;
