// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The in-process Stage 9 command boundary (quoin#521).
//!
//! Command families hand parsed requests to the one production runtime. This
//! preserves the core response/diagnostic taxonomy while eliminating the
//! Rust-to-Rust subprocess and its JSON reparse.

use quoin_core::error::{CoreError, CoreErrorCode};
use quoin_core::protocol::Response;
use quoin_core::runtime::{RuntimeSettings, dispatch};

use crate::invocation;

pub(crate) fn settings() -> RuntimeSettings {
    // An explicit root remains a developer override. Release binaries instead
    // materialize their compiled contract under IX_HOME; `CARGO_MANIFEST_DIR`
    // points at a build checkout and is invalid after installation (#527).
    let semantic_root = std::env::var_os("QUOIN_SEMANTIC_ROOT").map(std::path::PathBuf::from);
    RuntimeSettings {
        ix_home: invocation::current().config_root().cloned(),
        semantic_root,
    }
}

fn failed(operation: &str, error: &CoreError) -> String {
    format!("native runtime {operation} failed: {error}")
}

/// Invoke one command-shaped core operation through the governed runtime.
///
/// A dropped-declaration refusal (`QQ-1022`, FR-029) is not an operational
/// failure: it comes back as the payload-free refusal response, so the shell
/// exits `Refused` with the code and every dropped declaration on stderr and
/// nothing on stdout. Every caller already tests `outcome.carries_payload()`
/// before it reads a payload.
pub(crate) fn invoke(operation: &str, request: &serde_json::Value) -> Result<Response, String> {
    match invoke_typed(operation, request) {
        Err(error) if quoin_core::ops::quire::declarations_dropped(&error) => {
            Ok(Response::refusal(&error))
        }
        Err(error) => Err(failed(operation, &error)),
        Ok(response) => Ok(response),
    }
}

/// Why a command path that holds the engine's typed error stopped.
///
/// A path that turns every failure into a string loses the one distinction the
/// shell needs: a dropped declaration (`QQ-1022`, FR-029) is a refusal, exit
/// `Refused`, not an operational failure, exit 1.
pub(crate) enum Failure {
    /// An operational failure, stated as a message.
    Message(String),
    /// A refusal to return as the command's own answer.
    Refused(Response),
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

impl Failure {
    /// Classify an engine error raised by `operation`.
    pub(crate) fn from_quire(error: &quoin_quire::Error, operation: &'static str) -> Self {
        let envelope = quoin_core::ops::quire::map_error(error, operation);
        if quoin_core::ops::quire::declarations_dropped(&envelope) {
            Self::Refused(Response::refusal(&envelope))
        } else {
            Self::Message(error.to_string())
        }
    }

    /// The command's result for a failure that ends it here.
    pub(crate) fn settle(self) -> Result<Response, String> {
        match self {
            Self::Message(message) => Err(message),
            Self::Refused(response) => Ok(response),
        }
    }
}

/// As [`invoke`], keeping the core's typed error so a caller can tell a
/// request the core rejected from a failure of the host.
pub(crate) fn invoke_typed(
    operation: &str,
    request: &serde_json::Value,
) -> Result<Response, CoreError> {
    dispatch(operation, request, &settings())
}

/// As [`invoke`], except that a refusal naming the pack (a `pack` context entry) is returned as a
/// payload-free response carrying the refusal's own status, so the shell exits
/// with it rather than with the generic command failure.
pub(crate) fn invoke_refusable(
    operation: &str,
    request: &serde_json::Value,
) -> Result<Response, String> {
    match dispatch(operation, request, &settings()) {
        Err(error)
            if error.code == CoreErrorCode::Refused && error.context.contains_key("pack") =>
        {
            Ok(Response::refusal(&error))
        }
        Err(error) => Err(failed(operation, &error)),
        Ok(response) => Ok(response),
    }
}

/// Invoke an operation that verifies approvals. A rejected allowed-signers file
/// (exit 2) and an unavailable verifier or a failing fact (exit 3) are returned
/// as payload-free responses so the shell exits with that status.
pub(crate) fn invoke_verifying(
    operation: &str,
    request: &serde_json::Value,
) -> Result<Response, String> {
    match dispatch(operation, request, &settings()) {
        Err(error)
            if matches!(
                error.context.get("refusal").map(String::as_str),
                Some(
                    "allowed-signers-invalid"
                        | "approval-verifier-unavailable"
                        | "discharge-facts-failed"
                )
            ) =>
        {
            Ok(Response::refusal(&error))
        }
        Err(error) => Err(failed(operation, &error)),
        Ok(response) => Ok(response),
    }
}

/// Invoke an operation whose repository refusal is the command's own status.
pub(crate) fn invoke_declining(
    operation: &str,
    request: &serde_json::Value,
) -> Result<Response, String> {
    match dispatch(operation, request, &settings()) {
        Err(error) if error.code == CoreErrorCode::Refused => Ok(Response::refusal(&error)),
        Err(error) => Err(failed(operation, &error)),
        Ok(response) => Ok(response),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test assertions report failures")]
mod tests {
    use super::invoke;

    /// Trace: quoin#521
    #[test]
    fn tc_521_core_operations_run_in_process_without_the_protocol_executable() {
        let response = invoke("core.ping", &serde_json::json!({}))
            .expect("the embedded runtime answers without a quoin-core process on PATH");
        assert!(response.outcome.carries_payload());
        assert!(response.payload.get("core_version").is_some());
    }
}
