// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The operation table.
//!
//! The unit of IPC is a command-shaped operation (`<domain>.<op>`), never a
//! function (quoin#373). One module per domain; one entry per operation in
//! [`crate::dispatch`].

pub mod assurance;
pub mod completeness;
pub mod config;
pub mod core;
pub mod modules;
pub mod semantic;
pub mod validators;

use crate::error::{CoreError, CoreErrorCode};

/// The size of a request as it would be written back out, in bytes.
///
/// The dispatcher has already read and parsed the stream, so this is the honest
/// place to state a size the DOMAIN refuses, distinct from the transport
/// ceiling in [`crate::protocol::MAX_REQUEST_BYTES`].
///
/// # Errors
///
/// [`CoreErrorCode::Io`] when the parsed request cannot be re-serialised.
pub(crate) fn request_size(request: &serde_json::Value) -> Result<usize, CoreError> {
    Ok(serde_json::to_vec(request)
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?
        .len())
}

/// The one refusal shape every whole-request bound raises.
///
/// The LIMIT is a parameter and not a constant baked in per domain: this
/// function stood twice, once in `ops::assurance` and once in
/// `ops::completeness`, identical but for the constant each closed over, and a
/// third domain would have written it a third time. The context keys are the
/// caller-visible contract — `tc_445_103` asserts `observed_bytes` is present
/// and `stream` is not, because a domain refusal is not a stream refusal — so
/// there is one place they are spelled.
pub(crate) fn refusal(op: &'static str, limit: usize, size: usize) -> CoreError {
    CoreError::new(CoreErrorCode::Refused, "request exceeds the accepted size")
        .with_context("op", op)
        .with_context("limit_bytes", limit.to_string())
        .with_context("observed_bytes", size.to_string())
}
