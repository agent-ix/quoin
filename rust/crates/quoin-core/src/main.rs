// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The temporary quoin-core protocol executable.
//!
//! Stage 9 exposes the production host construction through
//! `quoin_core::runtime` so the native quoin CLI calls it in-process. This
//! retained executable exists only for the wire-parity and deletion cutover.

use std::io::Write as _;

use quoin_core::dispatch::{parse_operation, read_request};
use quoin_core::error::CoreError;
use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_core::runtime::{RuntimeSettings, dispatch};

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
    let operation = parse_operation(args)?;
    let request = read_request(std::io::stdin())?;
    dispatch(operation, &request, &RuntimeSettings::default())
}

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
