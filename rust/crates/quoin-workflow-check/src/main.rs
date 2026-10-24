// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Release checks exercise the actual packaged binary and published ix-flow.
//! Audit inventories receipts without validating or rewriting historical evidence.

mod audit;
mod smoke;

use std::process::ExitCode;

type Result<T> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = match args.as_slice() {
        [operation, binary, runner] if operation == "smoke" => smoke::run(binary, runner),
        [operation, state, repos @ ..] if operation == "audit" => audit::run(state, repos),
        _ => Err("usage: quoin-workflow-check smoke <quoin-executable> <ix-flow.js> | audit <state-directory> <repo>...".to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn require(condition: bool, detail: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(detail.into())
    }
}
