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
//!
//! # `Toolchains` is a divergence this crate cannot measure
//!
//! `graph-adapters.ts:365-374`'s `verificationStackSchema` required `node`,
//! `rust` and `python` as non-empty strings; that would make each `Option`
//! below a **third** divergence from it, by the same reasoning PLAT-930 gave
//! for [`quoin_measurement::types::collection::Toolchains`] (the sibling type
//! in the crate this one does not share code with): a measurement rarely
//! touches every language, and there is deliberately no sentinel spelling of
//! "not applicable" alongside the real absence.
//!
//! Unlike the timestamp divergence above, this one is **not** declared in
//! `tests/tc_475_parity.rs` against a captured verdict, because there is
//! nothing left to capture one from: quoin#480 (commit `eb0ca08`) deleted
//! `graph-adapters.ts` and its oracle capture script months before PLAT-930
//! was filed, and the retired TypeScript governed graph portfolio has run
//! nowhere since. This crate is the only implementation left, so relaxing its
//! own rule is this crate evolving, not two live systems disagreeing — there
//! is no oracle for a change made after retirement to diverge *from*.
//! Recorded here, rather than silently, so a reader does not go looking for a
//! `DECLARED_DIVERGENCES` entry or a golden case that cannot exist.
//!
//! See the same three-field change and its reasoning in
//! `rust/crates/quoin-measurement/src/types/collection.rs`.

use std::collections::BTreeMap;

use quoin_measurement::Rfc3339DateTime;
use serde::Deserialize;
use serde_json::Value;

use crate::scalars::{FullRevision, NonEmptyText, Sha256Reference};
use crate::wire::{CleanSourceState, ReleaseProfile, VerificationStackVersion};

/// The three language toolchains a producer pins.
///
/// Each member is `Option` rather than required (PLAT-930): a producer rarely
/// touches every language, and `None` — an absent key or an explicit `null`
/// — is the one spelling for "not applicable". See this module's header for
/// why that is not tracked as a measured divergence the way the timestamp
/// grammar above is.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Toolchains {
    /// The Node.js toolchain, when this measurement touched it.
    pub node: Option<NonEmptyText>,
    /// The Rust toolchain, when this measurement touched it.
    pub rust: Option<NonEmptyText>,
    /// The Python toolchain, when this measurement touched it.
    pub python: Option<NonEmptyText>,
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
    /// document fails its declaration, when its `timestamp` does not name an
    /// instant (the retained `.superRefine`, rendering the retained
    /// sentence), or when `verificationStack.toolchains` names no language at
    /// all.
    pub fn parse(document: &Value) -> crate::error::Result<Self> {
        use serde::Deserialize as _;

        use crate::error::{GraphAdapterError, GraphAdapterErrorCode};

        let refuse = |detail: String| {
            GraphAdapterError::new(GraphAdapterErrorCode::InvalidAttestation, detail)
        };
        let parsed = Self::deserialize(document).map_err(|error| refuse(error.to_string()))?;
        if !parsed.timestamp_is_an_instant() {
            return Err(refuse(
                "timestamp: timestamp is not a valid instant".to_owned(),
            ));
        }
        // PLAT-930 made each language optional (this module's header), but a
        // stack that pins none of them names nothing: an empty `{}` must not
        // satisfy "toolchains was supplied" any more than an absent member
        // would (the same gap quoin-measurement's own type closed).
        let toolchains = &parsed.verification_stack.toolchains;
        if toolchains.node.is_none() && toolchains.rust.is_none() && toolchains.python.is_none() {
            return Err(refuse(
                "verificationStack.toolchains: must name at least one language; mark a \
                 language not used as absent or null rather than omitting all three"
                    .to_owned(),
            ));
        }
        Ok(parsed)
    }
}
