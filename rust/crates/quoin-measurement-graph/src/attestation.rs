// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The invocation attestation a transcribed collection inherits its identity
//! from.
//!
//! Ports `verificationStackSchema` and `invocationAttestationSchema`
//! (`src/measurement/graph-adapters.ts:365-411`).
//!
//! # `timestamp` is the second declared divergence
//!
//! The retained refinement is `Number.isFinite(Date.parse(value.timestamp))`.
//! `Date.parse` is ECMA-262's *implementation-defined* fallback parser: beyond
//! the Date Time String Format it accepts whatever the engine's heuristic
//! accepts, which on V8 includes `"2026-01-01"`, `"Jan 1 2026"` and
//! `"2026/01/01 10:00"`.
//!
//! There is exactly one instant grammar in this workspace —
//! [`quoin_measurement::Rfc3339DateTime`], unified on the permissive RFC 3339
//! reading by quoin#440 — and the Stage 6 plan §5 forbids a seventh. So a
//! timestamp V8's heuristic accepts and RFC 3339 does not is newly refused.
//! That is declared in `tests/tc_475_parity.rs` and measured against the
//! oracle, not assumed: an attestation whose instant cannot be re-read the same
//! way by a second implementation is not an attestation of *when*.

use std::collections::BTreeMap;

use quoin_measurement::Rfc3339DateTime;
use serde::Deserialize;
use serde_json::Value;

use crate::scalars::{FullRevision, NonEmptyText, Sha256Reference};
use crate::wire::{CleanSourceState, ReleaseProfile, VerificationStackVersion};

/// The three language toolchains a producer pins.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Toolchains {
    /// The Node.js toolchain.
    pub node: NonEmptyText,
    /// The Rust toolchain.
    pub rust: NonEmptyText,
    /// The Python toolchain.
    pub python: NonEmptyText,
}

/// One source repository, pinned at a clean full revision.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceAttestation {
    /// The full revision the source was at.
    pub revision: FullRevision,
    /// Always `clean`: a dirty tree is not an attestation.
    pub source_state: CleanSourceState,
    /// Where the source came from.
    pub remote: NonEmptyText,
}

/// The immutable identity of everything that produced a collection.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct VerificationStack {
    /// Always `verification-stack-attestation-v1`.
    pub schema_version: VerificationStackVersion,
    /// The dependency lock's digest.
    pub lock_digest: Sha256Reference,
    /// The measured executable's digest.
    pub executable_digest: Sha256Reference,
    /// Always `release`.
    pub build_profile: ReleaseProfile,
    /// The pinned toolchains.
    pub toolchains: Toolchains,
    /// Every source repository.
    pub sources: BTreeMap<String, SourceAttestation>,
    /// The capabilities the producer exercised, non-empty.
    pub capabilities: Vec<NonEmptyText>,
    /// Every produced artifact and its digest.
    pub artifacts: BTreeMap<String, Sha256Reference>,
}

/// One producer invocation's subject, scope, instant and stack.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct InvocationAttestation {
    /// What was measured.
    pub subject: NonEmptyText,
    /// The producer-defined scope, opaque here.
    ///
    /// `z.unknown()` inside a `.strict()` object admits any *value* and still
    /// requires the *member*: zod 4.4.3 refuses an attestation with no `scope`
    /// with `expected nonoptional, received undefined`, measured on the
    /// captured corpus rather than assumed. So this is a bare
    /// [`serde_json::Value`] — `null` and every other value pass, absence does
    /// not — and not an `Option`, which would have admitted the one shape the
    /// retained module refuses.
    pub scope: Value,
    /// When it ran.
    pub timestamp: NonEmptyText,
    /// The environment the producer reported.
    pub environment: BTreeMap<String, String>,
    /// The stack that produced it.
    pub verification_stack: VerificationStack,
}

impl InvocationAttestation {
    /// Whether [`Self::timestamp`] names an instant.
    ///
    /// The declared divergence: see the module header.
    #[must_use]
    pub fn timestamp_is_an_instant(&self) -> bool {
        Rfc3339DateTime::parse(self.timestamp.as_str()).is_ok()
    }
}

impl InvocationAttestation {
    /// Read an invocation attestation.
    ///
    /// # Errors
    ///
    /// [`crate::error::GraphAdapterErrorCode::InvalidAttestation`] when the
    /// document fails its declaration, and when its `timestamp` does not name
    /// an instant. The second is the retained `.superRefine`, and it renders
    /// the retained sentence.
    pub fn parse(document: &Value) -> crate::error::Result<Self> {
        use serde::Deserialize as _;

        use crate::error::{GraphAdapterError, GraphAdapterErrorCode};

        let refuse = |detail: String| {
            GraphAdapterError::new(GraphAdapterErrorCode::InvalidAttestation, detail)
        };
        let parsed = Self::deserialize(document).map_err(|error| refuse(error.to_string()))?;
        if parsed.timestamp_is_an_instant() {
            Ok(parsed)
        } else {
            Err(refuse(
                "timestamp: timestamp is not a valid instant".to_owned(),
            ))
        }
    }
}
