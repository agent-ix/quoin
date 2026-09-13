// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `core`: operations about the boundary itself, with no domain
//! semantics whatsoever.
//!
//! Stage 0 (quoin#375) implements exactly one operation end to end, so the
//! path is proven before any logic is ported onto it. `core.ping` is that
//! operation: it identifies the build, echoes what it was given, and — because
//! the taxonomy is only a contract if every status is reachable — exercises
//! [`crate::protocol::Outcome::Partial`] and [`crate::protocol::Outcome::Refused`]
//! through real rules rather
//! than through a test hook.

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::Response;

/// The largest `echo` this operation will accept, in bytes.
///
/// A bound rather than "as much as arrives": stdin is an untrusted stream and
/// an accumulator on one needs a ceiling (rust-review §11). 4 KiB is ample for
/// a correlation token, which is all `echo` is for.
pub const MAX_ECHO_BYTES: usize = 4 * 1024;

/// The request accepted by `core.ping`.
///
/// `deny_unknown_fields` so a caller that misspells a field is refused rather
/// than silently ignored — a field the boundary drops is a field the caller
/// believes it sent.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PingRequest {
    /// An opaque token returned unchanged, for correlating a call with its
    /// answer across the pipe. Absent is fine.
    #[serde(default)]
    pub echo: Option<String>,
    /// The protocol revision the caller believes it is speaking. When it
    /// disagrees with this build's, the answer is still complete — it is how
    /// the caller learns which revision it is actually talking to — so the
    /// disagreement is reported as [`crate::protocol::Outcome::Partial`], not as a
    /// failure.
    #[serde(default)]
    pub expect_protocol: Option<u32>,
}

/// The payload `core.ping` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PingPayload {
    /// The protocol revision this build speaks.
    pub protocol_version: u32,
    /// The `quoin-core` crate version.
    pub core_version: &'static str,
    /// Whatever `echo` held, unchanged.
    pub echo: Option<String>,
}

/// Answer a `core.ping`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a `PingRequest`.
/// - [`CoreErrorCode::Refused`] when `echo` exceeds [`MAX_ECHO_BYTES`].
pub fn ping(request: &serde_json::Value) -> Result<Response, CoreError> {
    let request: PingRequest = serde_json::from_value(request.clone()).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", "core.ping")
    })?;

    if let Some(echo) = &request.echo
        && echo.len() > MAX_ECHO_BYTES
    {
        return Err(CoreError::new(
            CoreErrorCode::Refused,
            "echo exceeds the accepted size for a correlation token",
        )
        .with_context("op", "core.ping")
        .with_context("limit_bytes", MAX_ECHO_BYTES.to_string())
        .with_context("observed_bytes", echo.len().to_string()));
    }

    let payload = serde_json::to_value(PingPayload {
        protocol_version: crate::protocol::PROTOCOL_VERSION,
        core_version: env!("CARGO_PKG_VERSION"),
        echo: request.echo,
    })
    .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    match request.expect_protocol {
        Some(expected) if expected != crate::protocol::PROTOCOL_VERSION => Ok(Response::partial(
            payload,
            &CoreError::new(
                CoreErrorCode::ProtocolSkew,
                "caller expects a protocol revision this build does not speak; \
                 the payload is complete and names the revision it does speak",
            )
            .with_context("op", "core.ping")
            .with_context("expected", expected.to_string())
            .with_context("actual", crate::protocol::PROTOCOL_VERSION.to_string()),
        )),
        _ => Ok(Response::ok(payload)),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use crate::protocol::Outcome;

    #[test]
    fn an_empty_request_identifies_the_build() {
        let response = ping(&serde_json::json!({})).unwrap();
        assert_eq!(response.outcome, Outcome::Ok);
        assert_eq!(response.payload["protocol_version"], 1);
        assert_eq!(response.payload["echo"], serde_json::Value::Null);
        assert!(response.diagnostics.is_empty());
    }

    #[test]
    fn the_echo_token_comes_back_unchanged() {
        let response = ping(&serde_json::json!({ "echo": "corr-7" })).unwrap();
        assert_eq!(response.payload["echo"], "corr-7");
    }

    #[test]
    fn a_protocol_disagreement_still_returns_a_complete_payload() {
        // The taxonomy's load-bearing case: exit 1, and stdout is worth
        // reading anyway — it is how the caller learns the real revision.
        let response = ping(&serde_json::json!({ "expect_protocol": 99 })).unwrap();
        assert_eq!(response.outcome, Outcome::Partial);
        assert_eq!(response.outcome.code(), 1);
        assert!(response.outcome.carries_payload());
        assert_eq!(response.payload["protocol_version"], 1);
        assert_eq!(response.diagnostics.len(), 1);
        assert_eq!(response.diagnostics[0].code, "CORE_PROTOCOL_SKEW");
        assert_eq!(response.diagnostics[0].context["actual"], "1");
    }

    #[test]
    fn an_agreeing_protocol_expectation_is_a_plain_success() {
        let response = ping(&serde_json::json!({ "expect_protocol": 1 })).unwrap();
        assert_eq!(response.outcome, Outcome::Ok);
    }

    #[test]
    fn an_oversized_echo_is_refused_rather_than_accumulated() {
        let echo = "x".repeat(MAX_ECHO_BYTES + 1);
        let error = ping(&serde_json::json!({ "echo": echo })).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Refused);
        assert_eq!(error.outcome().code(), 2);
        assert_eq!(
            error.context["observed_bytes"],
            (MAX_ECHO_BYTES + 1).to_string()
        );
    }

    #[test]
    fn an_echo_at_exactly_the_limit_is_accepted() {
        let echo = "x".repeat(MAX_ECHO_BYTES);
        let response = ping(&serde_json::json!({ "echo": echo })).unwrap();
        assert_eq!(response.outcome, Outcome::Ok);
    }

    #[test]
    fn a_misspelled_field_is_refused_not_ignored() {
        let error = ping(&serde_json::json!({ "eco": "typo" })).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest);
        assert_eq!(error.outcome().code(), 3);
    }

    #[test]
    fn a_wrongly_typed_field_is_refused() {
        let error = ping(&serde_json::json!({ "echo": 7 })).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest);
    }
}
