// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `matrix` domain: the evidence-backed test matrix (FR-115).
//!
//! A shell over [`quoin_assurance::matrix`], in the shape of
//! `assurance.build_case`: a byte ceiling, a typed request that refuses
//! unknown fields, and a pure call. The operation reads nothing and runs
//! nothing, so it takes no capability.

use crate::error::{CoreError, CoreErrorCode};
use crate::ops::{refusal, request_size};
use crate::protocol::Response;

/// The largest `matrix.build` request, in bytes.
///
/// The same ceiling as `assurance.build_case`, for the same population: a
/// whole repository's criteria, its binding graph and its audit report arrive
/// in one request.
pub const MAX_MATRIX_BUILD_BYTES: usize = 16 * 1024 * 1024;

/// The `context.reason` a contradictory audit report is refused with.
pub const CONTRADICTORY_AUDIT: &str = "contradictory-audit";

/// Answer a `matrix.build`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a
///   [`MatrixInput`](quoin_assurance::matrix::MatrixInput).
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_MATRIX_BUILD_BYTES`], or when the audit report names one
///   obligation both healthy and not (`context.reason` is
///   [`CONTRADICTORY_AUDIT`]).
pub fn build(request: &serde_json::Value) -> Result<Response, CoreError> {
    let op = "matrix.build";
    let size = request_size(request)?;
    if size > MAX_MATRIX_BUILD_BYTES {
        return Err(refusal(op, MAX_MATRIX_BUILD_BYTES, size));
    }

    let input: quoin_assurance::matrix::MatrixInput = serde_json::from_value(request.clone())
        .map_err(|e| {
            CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
        })?;

    let matrix = quoin_assurance::matrix::build(&input).map_err(|contradiction| {
        CoreError::new(CoreErrorCode::Refused, contradiction.to_string())
            .with_context("op", op)
            .with_context("reason", CONTRADICTORY_AUDIT)
            .with_context("obligation", contradiction.obligation)
    })?;

    let payload = serde_json::to_value(matrix)
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;
    Ok(Response::ok(payload))
}
