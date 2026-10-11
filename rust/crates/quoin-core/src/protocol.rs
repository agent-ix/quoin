// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The wire contract: what an exit status means, and what is on each stream.
//!
//! # Streams
//!
//! - **stdout** is the payload and nothing else. On a run that carries no
//!   payload, stdout is empty — never a half-written object, never a log line.
//! - **stderr** is diagnostics, as one canonical JSON array. A caller that
//!   wants to report what went wrong parses it; a caller that does not can
//!   print it.
//!
//! # Exit taxonomy
//!
//! The load-bearing distinction is [`Outcome::Partial`]: a non-zero status
//! whose stdout is nevertheless a complete, valid payload. `runQuireAllowFailure`
//! in `src/quire/exec.ts` exists because `quire properties` exits 1 while
//! writing a full result for every document that did resolve, and treating
//! that as total failure silently discarded the whole property axis over two
//! untyped files (agent-ix/quoin#103). The same shape recurs here, so it is in
//! the taxonomy rather than discovered later.

use std::collections::BTreeMap;

use ix_cli_kit::exit::Outcome;

/// The IPC protocol revision the boundary speaks.
///
/// Lives beside the wire contract it versions. It used to sit in
/// `quoin-schemas` on the premise that the generated TypeScript side is
/// generated from that crate — but FR-097 makes the canonical Rust type the
/// source, so `quoin-schemas` now reads FROM here and the constant belongs
/// with the taxonomy, the diagnostic and the payloads it revises together.
/// The generated `src/core/types.ts` carries this number, so a bump reaches
/// the TypeScript side only through a regeneration whose digest is asserted.
pub const PROTOCOL_VERSION: u32 = 1;

/// The largest request, in bytes, that may be read off stdin.
///
/// **The transport ceiling, enforced while the stream is being read** —
/// `dispatch::read_request` stops at `MAX_REQUEST_BYTES + 1` and refuses, so
/// nothing larger is ever held, parsed or re-serialised. A ceiling that is
/// measured after the whole document is in memory is a statement about the
/// input, not a bound on the process: rust-style's rule is "every accumulator
/// from the stream has a ceiling, and a named refusal at it", and an
/// accumulator whose ceiling is checked afterwards had no ceiling.
///
/// It lives here, once, rather than per operation, because it is a property of
/// the stream and not of any domain. Every op's own bound is therefore a bound
/// on something it understands — `ops::core::MAX_ECHO_BYTES` is a bound on a
/// token, `ops::assurance::MAX_BUILD_CASE_BYTES` on a case document — and never
/// a second spelling of this one.
///
/// **It is deliberately larger than every domain bound**, and
/// `tc_412_the_transport_ceiling_stays_above_every_domain_bound` holds it
/// there. A domain refusal carries the `op` that refused and the quantity it
/// measured; a transport refusal cannot, because it happens before the request
/// is a request. Setting this to the same number as a domain bound would make
/// the transport verdict shadow the domain's, and the domain's check — still
/// compiled, still tested in isolation — would be one no caller could ever
/// reach. `ops::validators::run` has no bound of its own precisely because
/// this one is the whole of its answer.
///
/// 64 MiB is far past what any caller produces, and the number is measured
/// rather than guessed. `validators.run` is the largest request the boundary
/// accepts today and it carries only the files `quoin-validators` can
/// classify: `repoSnapshot(".")` on this repository walks 1056 files and puts
/// **13 of them, 71.1 KiB, on the wire** — 0.1% of this ceiling (measured
/// 2026-09-13 on this tree). A repository that reached 64 MiB of shell scripts
/// and build wiring would carry some 900 times quoin's gate surface.
///
/// The refusal is therefore a real one a real caller can hit only by sending
/// something no repository looks like, which is what a ceiling on an untrusted
/// stream is for.
pub const MAX_REQUEST_BYTES: usize = 64 * 1024 * 1024;

/// One entry of the stderr array.
///
/// A `Serialize` struct rather than a hand-built `serde_json::Value`: the
/// field list is then reviewable, and the compiler checks that every branch
/// populated it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    /// The stable code, from the catalogued enum — never a literal invented
    /// at the call site.
    pub code: String,
    /// A sentence for an operator.
    pub message: String,
    /// Ordered context. `BTreeMap` for byte-stable serialisation.
    pub context: BTreeMap<String, String>,
}

impl From<&crate::error::CoreError> for Diagnostic {
    fn from(error: &crate::error::CoreError) -> Self {
        Self {
            code: error.code.as_str().to_owned(),
            message: error.message.to_string(),
            context: error.context.clone(),
        }
    }
}

/// What an operation produced: the payload, and anything it wants said about
/// it.
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    /// The JSON written to stdout.
    pub payload: serde_json::Value,
    /// The JSON array written to stderr.
    pub diagnostics: Vec<Diagnostic>,
    /// The exit status.
    pub outcome: Outcome,
}

impl Response {
    /// A clean success.
    #[must_use]
    pub fn ok(payload: serde_json::Value) -> Self {
        Self {
            payload,
            diagnostics: Vec::new(),
            outcome: Outcome::Ok,
        }
    }

    /// A complete payload qualified by one diagnostic — exit 1.
    #[must_use]
    pub fn partial(payload: serde_json::Value, diagnostic: &crate::error::CoreError) -> Self {
        Self {
            payload,
            diagnostics: vec![Diagnostic::from(diagnostic)],
            outcome: Outcome::Partial,
        }
    }
}

/// Serialize the boundary through the shared canonical encoder.
///
/// Member names follow RFC 8785 UTF-16 order and numbers follow ECMAScript.
/// Exact Rust integers outside the shared encoder's lossless range are refused,
/// rather than rounded. Domain failures retain the boundary's stable Io code.
///
/// # Errors
///
/// Returns [`crate::error::CoreErrorCode::Io`] if encoding fails.
pub fn canonical_json<T: serde::Serialize>(value: &T) -> Result<String, crate::error::CoreError> {
    let value = serde_json::to_value(value).map_err(|error| {
        crate::error::CoreError::new(crate::error::CoreErrorCode::Io, error.to_string())
    })?;
    let bytes = quire_canonical::to_vec(&value, quire_canonical::Limits::new(u64::MAX)).map_err(
        |error| crate::error::CoreError::new(crate::error::CoreErrorCode::Io, error.to_string()),
    )?;
    String::from_utf8(bytes).map_err(|error| {
        crate::error::CoreError::new(crate::error::CoreErrorCode::Io, error.to_string())
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    /// Trace: FR-096-AC-10
    #[test]
    fn exit_statuses_are_the_documented_taxonomy() {
        assert_eq!(
            [
                Outcome::Ok.code(),
                Outcome::Partial.code(),
                Outcome::Refused.code(),
                Outcome::Invalid.code(),
                Outcome::Internal.code(),
            ],
            [0, 1, 2, 3, 4]
        );
    }

    /// Trace: FR-096-AC-10
    #[test]
    fn every_status_round_trips() {
        for outcome in [
            Outcome::Ok,
            Outcome::Partial,
            Outcome::Refused,
            Outcome::Invalid,
            Outcome::Internal,
        ] {
            assert_eq!(Outcome::from_code(outcome.code()), Some(outcome));
        }
        assert_eq!(Outcome::from_code(5), None);
    }

    /// Trace: FR-096-AC-10
    #[test]
    fn non_zero_but_valid_is_distinguishable_from_failed() {
        assert!(Outcome::Partial.carries_payload());
        assert_ne!(Outcome::Partial.code(), 0);
        for dead in [Outcome::Refused, Outcome::Invalid, Outcome::Internal] {
            assert!(!dead.carries_payload());
        }
    }

    /// Trace: FR-096-AC-10
    #[test]
    fn canonical_json_sorts_keys_and_emits_no_whitespace() {
        let value = serde_json::json!({ "z": 1, "a": { "y": 2, "b": 3 } });
        assert_eq!(
            canonical_json(&value).unwrap(),
            r#"{"a":{"b":3,"y":2},"z":1}"#
        );
    }

    /// Trace: FR-100-AC-10
    /// PLAT-989: literal failure vectors and an ASCII healthy control.
    #[test]
    fn canonical_boundary_matches_rfc8785_vectors() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/plat_989_canonical.json"))
                .unwrap();
        for case in cases.as_array().unwrap() {
            assert_eq!(
                canonical_json(&case["input"]).unwrap(),
                case["canonical"].as_str().unwrap(),
                "{}",
                case["name"]
            );
        }
    }

    /// Provenance: PLAT-837 review. `sort_object_keys`'s `Array` branch was
    /// untested -- it recurses into array elements but was never asserted to.
    /// An object nested inside an array element must come out sorted exactly
    /// like a top-level or nested-object one does.
    /// Trace: FR-096-AC-10
    /// Trace: FR-096-AC-2, FR-100-AC-10
    #[test]
    fn canonical_boundary_refuses_inexact_integer_identity() {
        assert_eq!(
            canonical_json(&9_007_199_254_740_992_u64).unwrap(),
            "9007199254740992"
        );
        let refusal = canonical_json(&9_007_199_254_740_993_u64).unwrap_err();
        assert_eq!(refusal.code, crate::error::CoreErrorCode::Io);
    }

    #[test]
    fn canonical_json_sorts_keys_inside_array_elements() {
        let value = serde_json::json!({ "z": [ { "b": 1, "a": 2 }, { "d": 3, "c": 4 } ] });
        assert_eq!(
            canonical_json(&value).unwrap(),
            r#"{"z":[{"a":2,"b":1},{"c":4,"d":3}]}"#
        );
    }

    /// Trace: FR-096-AC-10
    #[test]
    fn serialization_failure_retains_the_domain_io_code() {
        let value = BTreeMap::from([(vec![1, 2], "invalid JSON map key")]);
        let error = canonical_json(&value).unwrap_err();
        assert_eq!(error.code, crate::error::CoreErrorCode::Io);
        assert_eq!(error.message.as_ref(), "key must be a string");
    }
}
