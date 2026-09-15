// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The temporary Rust-to-Rust command boundary during Stage 8 (quoin#373).
//!
//! `quoin-core` remains the process that owns production filesystem grants
//! until Stage 9 folds it into the final `quoin` binary. Command families use
//! this one bridge rather than each reimplementing a host or inventing a second
//! diagnostic parser.

use std::io::Write as _;
use std::process::{Command, Stdio};

use quoin_core::protocol::{Diagnostic, Outcome, Response, canonical_json};

use crate::invocation;

/// Invoke one command-shaped core operation through the governed executable.
///
/// `QUOIN_CORE` selects an explicit candidate in verification; ordinary local
/// use resolves the core binary by its stable name on `PATH`.
pub(crate) fn invoke(operation: &str, request: &serde_json::Value) -> Result<Response, String> {
    let executable = std::env::var_os("QUOIN_CORE").unwrap_or_else(|| "quoin-core".into());
    let mut command = Command::new(executable);
    command.arg(operation);
    if let Some(root) = invocation::current().config_root() {
        command.env("IX_HOME", root);
    }
    if std::env::var_os("QUOIN_SEMANTIC_ROOT").is_none() {
        let vendored =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../src/semantic");
        if vendored.is_dir() {
            command.env("QUOIN_SEMANTIC_ROOT", vendored);
        }
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not start quoin-core {operation}: {error}"))?;
    let bytes = canonical_json(request)
        .map_err(|error| format!("could not serialize {operation} request: {error}"))?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| format!("quoin-core {operation} has no stdin"))?
        .write_all(bytes.as_bytes())
        .map_err(|error| format!("could not write {operation} request: {error}"))?;
    let output = child
        .wait_with_output()
        .map_err(|error| format!("could not wait for quoin-core {operation}: {error}"))?;
    let status = output
        .status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .and_then(Outcome::from_code)
        .ok_or_else(|| format!("quoin-core {operation} ended outside the protocol taxonomy"))?;
    let diagnostics = if output.stderr.is_empty() {
        Vec::new()
    } else {
        serde_json::from_slice::<Vec<Diagnostic>>(&output.stderr)
            .map_err(|error| format!("quoin-core {operation} wrote invalid diagnostics: {error}"))?
    };
    let payload = if status.carries_payload() {
        serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("quoin-core {operation} wrote invalid payload: {error}"))?
    } else if output.stdout.is_empty() {
        serde_json::Value::Null
    } else {
        return Err(format!(
            "quoin-core {operation} wrote a payload for a refused outcome"
        ));
    };
    Ok(Response {
        payload,
        diagnostics,
        outcome: status,
    })
}
