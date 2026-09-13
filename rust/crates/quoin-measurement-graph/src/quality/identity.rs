// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A producer observation's self-identity, re-derived from its own content.
//!
//! Ports `graphQualityObservationId`, `compactCanonicalProducerJson`,
//! `sortProducerKeys` and `compareUnicodeCodePoints`
//! (`src/measurement/graph-adapters.ts:461-468, 731-756`).
//!
//! # The third declared divergence: which order the member names sort in
//!
//! The retained identity is `JSON.stringify(sortProducerKeys(record))` — no
//! indentation, member names sorted. That is RFC 8785 in every respect but
//! one: `sortProducerKeys` sorts by **Unicode code point**
//! (`[...a]` iterates code points), and RFC 8785 §3.2.3 sorts by **UTF-16 code
//! unit**. The two orders disagree for exactly one pair of names: one whose
//! first differing character is astral (`U+10000` and above, a high surrogate
//! `D800..DBFF` in UTF-16) against one whose first differing character is in
//! `U+E000..U+FFFF`. Code points put the astral name second; code units put it
//! first.
//!
//! This crate does not add a fourth canonicalizer to reproduce that. The
//! bytes come from [`quoin_store::canonical_bytes`], the workspace's one JCS
//! writer, and the difference is declared in `tests/tc_475_parity.rs` and
//! fired there against the oracle rather than argued about here.
//!
//! It is unobservable for every record this adapter can accept: a record that
//! reaches this function has already satisfied
//! [`crate::quality::GraphQualityObservationV1`], whose member names are all
//! schema-fixed ASCII, and the only producer-chosen names in it are census
//! *values*, not names. A name that could separate the two orders cannot be
//! present.
//!
//! # The digest is `quoin-store`'s
//!
//! [`quoin_store::digest_bytes_sha256`] (quoin#484) hashes the bytes in the
//! raw-file sha256 domain. This crate mints no sha256 of its own.

use quoin_store::digest_bytes_sha256;
use serde_json::Value;

use crate::canonical::compact_bytes;
use crate::error::Result;
use crate::scalars::Sha256Reference;

/// The member the identity is taken over its own absence of.
const SELF_MEMBER: &str = "observation_id";

/// Re-derive a producer observation's `observation_id` from its content.
///
/// The member itself is removed first, so the identity is of everything *but*
/// the claim about the identity. A non-object is digested as it stands, which
/// is what the retained `isRecord` guard does.
///
/// # Errors
///
/// [`crate::error::GraphAdapterErrorCode::Store`] for a value the canonical
/// writer cannot express.
pub fn graph_quality_observation_id(value: &Value) -> Result<Sha256Reference> {
    let without_self = match value {
        Value::Object(members) => {
            let mut members = members.clone();
            members.remove(SELF_MEMBER);
            Value::Object(members)
        }
        other => other.clone(),
    };
    Ok(digest_bytes_sha256(&compact_bytes(&without_self)?).into())
}
