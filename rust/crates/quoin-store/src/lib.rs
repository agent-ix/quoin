// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! Canonical JSON and durable evidence-store writes for Quoin.
//!
//! # Layout
//!
//! * [`json`] - the strict I-JSON reader and the two canonical writers.
//! * [`digest`] - the sha256 helpers the measurement crates still call, and the
//!   bounded no-follow file read (see that module for what remains and why).
//! * [`store`] - store paths, [`store::STORE_SCHEMA_VERSION`], and durable
//!   atomic writes.
//! * [`error`] - one error enum, one stable code per refusal.
//!
//! # Records are addressed by identity, not content
//!
//! A change-assurance record is stored at
//! `change-assurance/records/<record_id>/<revision>.json` and an attestation at
//! `change-assurance/attestations/<attestation_id>/`. Both identities are
//! assigned by the caller; nothing in a path is derived from the bytes it
//! holds, so this crate computes no digest over either and git history is the
//! tamper evidence.
//!
//! # The frozen compatibility surface
//!
//! These are facts about evidence that already exists. They are not open for
//! improvement, and each is asserted by a test:
//!
//! 1. **`STORE_SCHEMA_VERSION == 1`.**
//! 2. **Two canonical serializations, not one.** RFC 8785 JCS for the
//!    change-assurance family's on-disk form; `canonicalJson` (two-space
//!    indent, `": "` separator, trailing newline) for every other store file.
//! 3. **JCS member order is UTF-16 code unit order**, not Unicode scalar order.
//! 4. **The pretty form's member order is ECMAScript own-property order**:
//!    array-index names first in ascending numeric order, then the rest in
//!    UTF-16 sorted order. This is not the same as (3) and never will be.
//! 5. **Numbers are ECMAScript `Number::toString`**, including `1e+21` with its
//!    `+`, `100000000000000000000` for `1e20`, and `0` for negative zero.
//! 6. **Every JSON number is an IEEE-754 double**, integers included. Integer
//!    precision beyond 2^53 is already lost in retained evidence.
//! 7. **No Unicode normalization**, ever, on member names or string values.

pub mod digest;
pub mod error;
pub mod json;
pub mod store;

pub use digest::{
    DigestDomain, MAX_DIGESTED_FILE_BYTES, RawFileSha256Digest, RecordFileName, digest_bytes_sha256,
    digest_file_sha256, digest_record_file_name, digest_record_pair_file_name,
    read_regular_file_bounded,
};
pub use error::{StoreError, StoreErrorCode};
pub use json::canonical::{canonical_bytes, canonicalize_jcs};
pub use json::parse::{parse_strict_json, parse_strict_json_str};
pub use json::pretty::{canonical_json, canonical_json_bytes};
pub use json::value::{JsonNumber, JsonObject, JsonValue};
pub use store::STORE_SCHEMA_VERSION;
