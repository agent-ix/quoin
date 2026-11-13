// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The in-process Stage 9 command boundary (quoin#521).
//!
//! Command families hand parsed requests to the one production runtime. This
//! preserves the core response/diagnostic taxonomy while eliminating the
//! Rust-to-Rust subprocess and its JSON reparse.

use quoin_core::error::{CoreError, CoreErrorCode};
use quoin_core::protocol::{Diagnostic, Response};
use quoin_core::runtime::{RuntimeSettings, dispatch};

use crate::invocation;

fn settings() -> RuntimeSettings {
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
pub(crate) fn invoke(operation: &str, request: &serde_json::Value) -> Result<Response, String> {
    invoke_typed(operation, request).map_err(|error| failed(operation, &error))
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
            Ok(Response {
                payload: serde_json::Value::Null,
                diagnostics: vec![Diagnostic::from(&error)],
                outcome: error.outcome(),
            })
        }
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
