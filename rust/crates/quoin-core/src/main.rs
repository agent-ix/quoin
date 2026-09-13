// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin-core <domain>.<op>` — the I/O shell.
//!
//! Everything decidable lives in the library. This file does four things and
//! must keep doing only four: read argv, read stdin, call
//! [`quoin_core::dispatch::dispatch`], write the two streams and exit.
//!
//! Even "read stdin" is a library decision here: the ceiling on an untrusted
//! stream is [`quoin_core::protocol::MAX_REQUEST_BYTES`] and the bounded read
//! is [`quoin_core::dispatch::read_request`], so the refusal is unit-testable
//! against a `&[u8]` without a process. This file supplies the handle and
//! nothing else.
//!
//! The discipline that matters here is that **stdout is written only when the
//! outcome carries a payload**. A caller reading a half-written object off a
//! failed run is the defect this ordering prevents: the payload is serialised
//! in full, in memory, before a byte reaches stdout.

use std::io::Write as _;

use quoin_core::dispatch::{dispatch, parse_operation, read_request};
use quoin_core::error::CoreError;
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
    // Bounded, and bounded HERE: `read_request` stops the stream one byte past
    // `protocol::MAX_REQUEST_BYTES` rather than reading whatever arrives and
    // measuring it afterwards. See quoin#448's review — an uncapped
    // `read_to_string` in this function made every operation's own size limit
    // a remark about an allocation that had already happened.
    let request = read_request(std::io::stdin())?;
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
