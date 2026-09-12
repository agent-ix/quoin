// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Domain `assurance`: the read-only assurance-case view (quoin#384).
//!
//! The first REAL quoin capability across the boundary. Stage 0's `core.ping`
//! proved the mechanism against a purpose-built oracle; every operation here
//! is compared against the retained `src/assurance/` itself, which is the
//! whole reason that module is retained rather than deleted (FR-101).

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::Response;

/// The largest obligation id this domain will accept, in bytes.
///
/// A bound rather than "as much as arrives": stdin is an untrusted stream
/// (rust-review §11). Real ids are under 32 bytes; 4 KiB is far past anything
/// a corpus mints and still refuses a stream.
pub const MAX_OBLIGATION_ID_BYTES: usize = 4 * 1024;

/// The request accepted by `assurance.requirement_of`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementOfRequest {
    /// The obligation id to take the requirement prefix of.
    pub obligation_id: String,
}

/// The payload `assurance.requirement_of` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequirementOfPayload {
    /// The requirement the obligation belongs to, or the id unchanged when it
    /// matches no requirement pattern.
    pub requirement: String,
}

/// Answer an `assurance.requirement_of`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`RequirementOfRequest`].
/// - [`CoreErrorCode::Refused`] when the id exceeds [`MAX_OBLIGATION_ID_BYTES`].
pub fn requirement_of(request: &serde_json::Value) -> Result<Response, CoreError> {
    let request: RequirementOfRequest = serde_json::from_value(request.clone()).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string())
            .with_context("op", "assurance.requirement_of")
    })?;

    if request.obligation_id.len() > MAX_OBLIGATION_ID_BYTES {
        return Err(CoreError::new(
            CoreErrorCode::Refused,
            "obligation id exceeds the accepted size",
        )
        .with_context("op", "assurance.requirement_of")
        .with_context("limit_bytes", MAX_OBLIGATION_ID_BYTES.to_string())
        .with_context("observed_bytes", request.obligation_id.len().to_string()));
    }

    let payload = serde_json::to_value(RequirementOfPayload {
        requirement: quoin_assurance::requirement_of(&request.obligation_id).to_owned(),
    })
    .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    Ok(Response::ok(payload))
}

/// The largest `assurance.build_case` request this domain will accept, in bytes.
///
/// A whole bundle's frontmatter, obligations and findings arrive in one
/// request, so the ceiling is far above `assurance.requirement_of`'s — quoin's
/// own bundle is 991 obligations and roughly 350 documents. 16 MiB is past
/// anything a real corpus produces and still refuses a stream, which is the
/// property the bound exists for (rust-style §11).
pub const MAX_BUILD_CASE_BYTES: usize = 16 * 1024 * 1024;

/// Answer an `assurance.build_case`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`CaseInput`].
/// - [`CoreErrorCode::Refused`] when the request exceeds [`MAX_BUILD_CASE_BYTES`].
///
/// [`CaseInput`]: quoin_assurance::CaseInput
pub fn build_case(request: &serde_json::Value) -> Result<Response, CoreError> {
    // Measured on the parsed value rather than on raw stdin: the dispatcher
    // has already read and parsed the stream, so this is the honest place to
    // state a size the DOMAIN refuses, distinct from any transport ceiling.
    let size = serde_json::to_vec(request)
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?
        .len();
    if size > MAX_BUILD_CASE_BYTES {
        return Err(
            CoreError::new(CoreErrorCode::Refused, "request exceeds the accepted size")
                .with_context("op", "assurance.build_case")
                .with_context("limit_bytes", MAX_BUILD_CASE_BYTES.to_string())
                .with_context("observed_bytes", size.to_string()),
        );
    }

    let input: quoin_assurance::CaseInput =
        serde_json::from_value(request.clone()).map_err(|e| {
            CoreError::new(CoreErrorCode::BadRequest, e.to_string())
                .with_context("op", "assurance.build_case")
        })?;

    let payload = serde_json::to_value(quoin_assurance::build_case(&input))
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    Ok(Response::ok(payload))
}
