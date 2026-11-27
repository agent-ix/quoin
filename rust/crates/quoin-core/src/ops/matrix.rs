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
use ix_trace_rs::implements;

/// The largest `matrix.build` request, in bytes.
///
/// Matrix requests carry a whole repository's criteria, binding graph and audit
/// report. The 80 MiB domain ceiling leaves 16 MiB below the 96 MiB transport
/// ceiling for the operation envelope and keeps transport refusals unreachable
/// for a request that fits this domain.
pub const MAX_MATRIX_BUILD_BYTES: usize = 80 * 1024 * 1024;

/// The `context.reason` a contradictory audit report is refused with.
pub const CONTRADICTORY_AUDIT: &str = "contradictory-audit";

/// The `context.reason` an invalid stale-evidence cause is refused with.
pub const INVALID_STALE_CAUSE: &str = "invalid-stale-cause";

/// Answer a `matrix.build`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a
///   [`MatrixInput`](quoin_assurance::matrix::MatrixInput).
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_MATRIX_BUILD_BYTES`], or when the audit report names one
///   obligation both healthy and not (`context.reason` is
///   [`CONTRADICTORY_AUDIT`]), or when a finding carries an invalid
///   stale-evidence cause (`context.reason` is [`INVALID_STALE_CAUSE`]).
pub fn build(request: &serde_json::Value) -> Result<Response, CoreError> {
    let op = "matrix.build";
    let size = request_size(request)?;
    check_request_size(size)?;

    let input: quoin_assurance::matrix::MatrixInput = serde_json::from_value(request.clone())
        .map_err(|e| {
            CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
        })?;

    let matrix = quoin_assurance::matrix::build(&input).map_err(|refusal| match refusal {
        quoin_assurance::matrix::MatrixRefusal::ContradictoryAudit(contradiction) => {
            CoreError::new(CoreErrorCode::Refused, contradiction.to_string())
                .with_context("op", op)
                .with_context("reason", CONTRADICTORY_AUDIT)
                .with_context("obligation", contradiction.obligation)
        }
        quoin_assurance::matrix::MatrixRefusal::InvalidStaleCause(invalid) => {
            CoreError::new(CoreErrorCode::Refused, invalid.to_string())
                .with_context("op", op)
                .with_context("reason", INVALID_STALE_CAUSE)
                .with_context("obligation", invalid.obligation)
        }
    })?;

    let payload = serde_json::to_value(matrix)
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;
    Ok(Response::ok(payload))
}

#[implements("FR-115-AC-27")]
fn check_request_size(size: usize) -> Result<(), CoreError> {
    if size > MAX_MATRIX_BUILD_BYTES {
        return Err(refusal("matrix.build", MAX_MATRIX_BUILD_BYTES, size));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::indexing_slicing,
        reason = "the boundary test uses failures as its report"
    )]

    use super::{MAX_MATRIX_BUILD_BYTES, check_request_size};
    use ix_trace_rs::trace;

    #[trace("FR-115-AC-27")]
    #[test]
    fn matrix_request_limit_accepts_just_under_and_refuses_just_over() {
        assert!(check_request_size(MAX_MATRIX_BUILD_BYTES - 1).is_ok());

        let error = check_request_size(MAX_MATRIX_BUILD_BYTES + 1)
            .expect_err("a request over the matrix ceiling is refused");
        assert_eq!(
            error.context["limit_bytes"],
            MAX_MATRIX_BUILD_BYTES.to_string()
        );
        assert_eq!(
            error.context["observed_bytes"],
            (MAX_MATRIX_BUILD_BYTES + 1).to_string()
        );
    }
}
