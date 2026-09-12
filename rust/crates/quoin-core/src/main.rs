// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin-core <domain>.<op>` — the I/O shell.
//!
//! Everything decidable lives in the library. This file does four things and
//! must keep doing only four: read argv, read stdin, call
//! [`quoin_core::dispatch::dispatch`], write the two streams and exit.
//!
//! The discipline that matters here is that **stdout is written only when the
//! outcome carries a payload**. A caller reading a half-written object off a
//! failed run is the defect this ordering prevents: the payload is serialised
//! in full, in memory, before a byte reaches stdout.

use std::io::{Read as _, Write as _};

use quoin_core::dispatch::{dispatch, parse_operation, parse_request};
use quoin_core::error::{CoreError, CoreErrorCode};
use quoin_core::protocol::{Diagnostic, Response, canonical_json};

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(response) => emit(&response),
        Err(error) => emit(&Response {
            payload: serde_json::Value::Null,
            diagnostics: vec![Diagnostic::from(&error)],
            outcome: error.outcome(),
        }),
    }
}

fn run(args: &[String]) -> Result<Response, CoreError> {
    let op = parse_operation(args)?;
    let mut stdin = String::new();
    std::io::stdin().read_to_string(&mut stdin).map_err(|e| {
        CoreError::new(CoreErrorCode::Io, e.to_string()).with_context("stream", "stdin")
    })?;
    let request = parse_request(&stdin)?;
    dispatch(op, &request)
}

/// Write the streams and choose the status.
///
/// A serialisation or write failure downgrades the run to `Internal` (4) and
/// writes NO payload, rather than reporting success over a truncated stream.
fn emit(response: &Response) -> std::process::ExitCode {
    let payload = if response.outcome.carries_payload() {
        match canonical_json(&response.payload) {
            Ok(text) => Some(text),
            Err(error) => return emit_internal(&error),
        }
    } else {
        None
    };
    let diagnostics = match canonical_json(&response.diagnostics) {
        Ok(text) => text,
        Err(error) => return emit_internal(&error),
    };

    if let Some(payload) = payload {
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();
        if writeln!(stdout, "{payload}").is_err() || stdout.flush().is_err() {
            return std::process::ExitCode::from(quoin_core::protocol::Outcome::Internal.code());
        }
    }
    if !response.diagnostics.is_empty() {
        let _ = writeln!(std::io::stderr(), "{diagnostics}");
    }
    std::process::ExitCode::from(response.outcome.code())
}

fn emit_internal(error: &CoreError) -> std::process::ExitCode {
    let _ = writeln!(std::io::stderr(), "[{}] {}", error.code, error.message);
    std::process::ExitCode::from(quoin_core::protocol::Outcome::Internal.code())
}
