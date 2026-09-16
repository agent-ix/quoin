// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The in-process Stage 9 command boundary (quoin#521).
//!
//! Command families hand parsed requests to the one production runtime. This
//! preserves the core response/diagnostic taxonomy while eliminating the
//! Rust-to-Rust subprocess and its JSON reparse.

use quoin_core::protocol::Response;
use quoin_core::runtime::{RuntimeSettings, dispatch};

use crate::invocation;

/// Invoke one command-shaped core operation through the governed runtime.
pub(crate) fn invoke(operation: &str, request: &serde_json::Value) -> Result<Response, String> {
    // An explicit root remains a developer override. Release binaries instead
    // materialize their compiled contract under IX_HOME; `CARGO_MANIFEST_DIR`
    // points at a build checkout and is invalid after installation (#527).
    let semantic_root = std::env::var_os("QUOIN_SEMANTIC_ROOT").map(std::path::PathBuf::from);
    let settings = RuntimeSettings {
        ix_home: invocation::current().config_root().cloned(),
        semantic_root,
    };
    dispatch(operation, request, &settings)
        .map_err(|error| format!("native runtime {operation} failed: {error}"))
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
        assert_eq!(
            response
                .payload
                .get("protocol_version")
                .and_then(serde_json::Value::as_u64),
            Some(1)
        );
    }
}
