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
use crate::ops::{refusal, request_size};
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
    let op = "assurance.build_case";
    let size = request_size(request)?;
    if size > MAX_BUILD_CASE_BYTES {
        return Err(refusal(op, MAX_BUILD_CASE_BYTES, size));
    }

    let input: quoin_assurance::CaseInput =
        serde_json::from_value(request.clone()).map_err(|e| {
            CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
        })?;

    let payload = serde_json::to_value(quoin_assurance::build_case(&input))
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    Ok(Response::ok(payload))
}

/// The payload `assurance.render_case` writes to stdout.
///
/// The rendered document is a JSON string field rather than raw markdown on
/// stdout, because the boundary's rule is that stdout carries a canonical JSON
/// payload and nothing else. A consumer wanting the file writes the field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderCasePayload {
    /// The rendered markdown.
    pub rendered: String,
}

/// Answer an `assurance.render_case`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`RenderableCase`].
/// - [`CoreErrorCode::Refused`] when the request exceeds [`MAX_BUILD_CASE_BYTES`].
///
/// [`RenderableCase`]: quoin_assurance::RenderableCase
pub fn render_case(request: &serde_json::Value) -> Result<Response, CoreError> {
    // The same ceiling as `build_case`, and deliberately the same constant:
    // this operation's input IS that operation's output, so a case that could
    // be built and then could not be rendered would be a boundary that
    // contradicts itself.
    let op = "assurance.render_case";
    let size = request_size(request)?;
    if size > MAX_BUILD_CASE_BYTES {
        return Err(refusal(op, MAX_BUILD_CASE_BYTES, size));
    }

    let assurance: quoin_assurance::RenderableCase = serde_json::from_value(request.clone())
        .map_err(|e| {
            CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
        })?;

    let payload = serde_json::to_value(RenderCasePayload {
        rendered: quoin_assurance::render_case(&assurance),
    })
    .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    Ok(Response::ok(payload))
}

/// Answer an `assurance.parse_argument`.
///
/// **The request IS the argument**, with no wrapper object. Every other
/// operation in this domain takes named fields because the retained function
/// does; this one hands the whole request to a validator whose first act is to
/// refuse any key outside a closed set of twelve. A wrapper would have added a
/// thirteenth key that exists on neither side of the retained API.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when the request fails any of the retained
///   contract's predicates.
/// - [`CoreErrorCode::Refused`] when the request exceeds [`MAX_BUILD_CASE_BYTES`].
pub fn parse_argument(request: &serde_json::Value) -> Result<Response, CoreError> {
    // The same ceiling as the two operations above. An authored argument is
    // far smaller than a built case, but a THIRD constant would be a third
    // thing to keep in agreement with the reference for no behaviour gained.
    let op = "assurance.parse_argument";
    let size = request_size(request)?;
    if size > MAX_BUILD_CASE_BYTES {
        return Err(refusal(op, MAX_BUILD_CASE_BYTES, size));
    }

    let argument = quoin_assurance::parse_assurance_argument(request).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
    })?;

    let payload = serde_json::to_value(argument)
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    Ok(Response::ok(payload))
}

/// The payload `assurance.render_authored_argument` writes to stdout.
///
/// A JSON string field rather than raw markdown, for the same reason
/// [`RenderCasePayload`] is one: stdout carries a canonical JSON payload and
/// nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderAuthoredArgumentPayload {
    /// The rendered markdown.
    pub rendered: String,
}

/// The payload `assurance.render_discharge` writes to stdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RenderDischargePayload {
    /// The rendered markdown.
    pub rendered: String,
}

/// Answer an `assurance.build_authored_argument`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when the request is not a
///   [`BuildAuthoredArgumentRequest`], or when the retained contract's
///   predicates refuse its contents.
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_BUILD_CASE_BYTES`].
///
/// [`BuildAuthoredArgumentRequest`]: quoin_assurance::BuildAuthoredArgumentRequest
pub fn build_authored_argument(request: &serde_json::Value) -> Result<Response, CoreError> {
    let op = "assurance.build_authored_argument";
    let size = request_size(request)?;
    if size > MAX_BUILD_CASE_BYTES {
        return Err(refusal(op, MAX_BUILD_CASE_BYTES, size));
    }

    let input: quoin_assurance::BuildAuthoredArgumentRequest =
        serde_json::from_value(request.clone()).map_err(|e| {
            CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
        })?;

    // The retained implementation's own refusals arrive here as one error
    // class carrying the authored message. They are `BadRequest` and not
    // `Refused`: the caller sent a document that fails the contract, which is
    // a different fact from a document this process declined to read.
    let view = quoin_assurance::build_authored_argument_view(&input).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
    })?;

    let payload =
        serde_json::to_value(view).map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    Ok(Response::ok(payload))
}

/// Answer an `assurance.render_authored_argument`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not an
///   [`AuthoredArgumentView`].
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_BUILD_CASE_BYTES`].
///
/// [`AuthoredArgumentView`]: quoin_assurance::AuthoredArgumentView
pub fn render_authored_argument(request: &serde_json::Value) -> Result<Response, CoreError> {
    let op = "assurance.render_authored_argument";
    let size = request_size(request)?;
    if size > MAX_BUILD_CASE_BYTES {
        return Err(refusal(op, MAX_BUILD_CASE_BYTES, size));
    }

    let view: quoin_assurance::AuthoredArgumentView = serde_json::from_value(request.clone())
        .map_err(|e| {
            CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
        })?;

    let payload = serde_json::to_value(RenderAuthoredArgumentPayload {
        rendered: quoin_assurance::render_authored_argument(&view),
    })
    .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    Ok(Response::ok(payload))
}

/// Answer an `assurance.build_discharge`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when the request is not a
///   [`BuildDischargeRequest`], or when the retained contract refuses its
///   contents — a malformed fact, a non-instant `asOf`, or two facts naming
///   one clause (FR-046-AC-5).
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_BUILD_CASE_BYTES`].
///
/// [`BuildDischargeRequest`]: quoin_assurance::BuildDischargeRequest
pub fn build_discharge(request: &serde_json::Value) -> Result<Response, CoreError> {
    let op = "assurance.build_discharge";
    let size = request_size(request)?;
    if size > MAX_BUILD_CASE_BYTES {
        return Err(refusal(op, MAX_BUILD_CASE_BYTES, size));
    }

    let input: quoin_assurance::BuildDischargeRequest = serde_json::from_value(request.clone())
        .map_err(|e| {
            CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
        })?;

    let report = quoin_assurance::build_discharge_report(&input).map_err(|e| {
        CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
    })?;

    let payload = serde_json::to_value(report)
        .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    Ok(Response::ok(payload))
}

/// Answer an `assurance.render_discharge`.
///
/// # Errors
///
/// - [`CoreErrorCode::BadRequest`] when stdin is not a [`DischargeReport`].
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_BUILD_CASE_BYTES`].
///
/// [`DischargeReport`]: quoin_assurance::DischargeReport
pub fn render_discharge(request: &serde_json::Value) -> Result<Response, CoreError> {
    let op = "assurance.render_discharge";
    let size = request_size(request)?;
    if size > MAX_BUILD_CASE_BYTES {
        return Err(refusal(op, MAX_BUILD_CASE_BYTES, size));
    }

    let report: quoin_assurance::DischargeReport = serde_json::from_value(request.clone())
        .map_err(|e| {
            CoreError::new(CoreErrorCode::BadRequest, e.to_string()).with_context("op", op)
        })?;

    let payload = serde_json::to_value(RenderDischargePayload {
        rendered: quoin_assurance::render_discharge_report(&report),
    })
    .map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;

    Ok(Response::ok(payload))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        reason = "in a test, a panic IS the failure report; the production lints stand"
    )]

    use super::*;
    use crate::protocol::Outcome;

    /// The smallest `clause-binding-v1` report the retained reader accepts.
    ///
    /// Written out rather than abbreviated because the deserialiser is half of
    /// what these tests exercise: an abbreviation that the reader refused
    /// would have made every assertion below pass for the wrong reason, which
    /// is what the first draft of `tc_447_320` did.
    fn binding() -> serde_json::Value {
        serde_json::json!({
            "schemaVersion": "clause-binding-v1",
            "clauseSet": { "authority": "quire", "id": "spec", "version": "1" },
            "clauseSetDigest":
                "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "context": {},
            "clauses": [],
        })
    }

    /// A request whose contents the retained contract refuses is a bad
    /// REQUEST, not a refusal by this process, and the two exit differently.
    #[test]
    fn tc_447_320_a_contract_failure_exits_as_a_bad_request() {
        let error = build_discharge(&serde_json::json!({
            "binding": binding(),
            "facts": [],
            "asOf": "not-an-instant",
        }))
        .unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest);
        assert_eq!(error.code.outcome(), Outcome::Invalid);
        assert_eq!(
            error.context.get("op").map(String::as_str),
            Some("assurance.build_discharge")
        );
    }

    /// An unknown key is refused rather than ignored: `deny_unknown_fields` on
    /// the request is what stops a caller's typo reading as a default.
    #[test]
    fn tc_447_321_an_unknown_request_key_is_refused() {
        let error = build_authored_argument(&serde_json::json!({
            "argument": {},
            "decisions": [],
            "asOf": "2026-01-01T00:00:00Z",
            "typo": true,
        }))
        .unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest);
    }

    /// The ceiling is checked BEFORE the work, and it names what it observed.
    #[test]
    fn tc_447_322_an_oversized_request_is_refused_before_it_is_parsed() {
        let filler = "x".repeat(MAX_BUILD_CASE_BYTES + 1);
        let error = render_discharge(&serde_json::json!({ "filler": filler })).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Refused);
        assert_eq!(error.code.outcome(), Outcome::Refused);
        assert_eq!(
            error.context.get("limit_bytes").map(String::as_str),
            Some(MAX_BUILD_CASE_BYTES.to_string().as_str())
        );
    }

    /// A discharge report this half produced reads back as the same value.
    ///
    /// The two operations are a pair — `build_discharge` hands its payload to
    /// `render_discharge`, and `build_authored_argument` takes it as a field —
    /// so a report that serialises one way and deserialises another would
    /// break the command that chains them, not this crate.
    #[test]
    fn tc_447_323_a_built_report_is_a_renderable_report() {
        let built = build_discharge(&serde_json::json!({
            "binding": binding(),
            "facts": [],
            "asOf": "2026-01-01T00:00:00Z",
        }))
        .unwrap();
        let rendered = render_discharge(&built.payload).unwrap();
        assert!(
            rendered.payload["rendered"]
                .as_str()
                .is_some_and(|text| !text.is_empty()),
            "{:?}",
            rendered.payload
        );
    }

    /// Both retained error classes leave this seam as `CORE_BAD_REQUEST`.
    ///
    /// **Why this is not the pinned-count shape**
    /// `a_third_completeness_error_code_would_need_a_mapping` (`ops::completeness`)
    /// counts `CompletenessErrorCode::all()`, which exists because that type is
    /// an enum with an enumerated set. [`quoin_assurance::ArgumentError`] and
    /// [`quoin_assurance::DischargeError`] are one-field tuple structs — the
    /// crate documents the choice: seventeen predicates leave the boundary as
    /// one code because the retained `parseAssuranceArgument` throws one
    /// `Error`, so "there is one discriminant, and it is spelled `Err`". There
    /// is no set to count, and no `all()` to call.
    ///
    /// What this replaces was `ArgumentError("x").to_string() == "x"`, which
    /// only restated the `Display` impl. The guard the doc comment claimed — a
    /// second variant becoming visible — is delivered by the construction below
    /// failing to COMPILE if either type stops being a one-field tuple struct,
    /// not by any assertion.
    ///
    /// So what is asserted instead is the thing that could silently change and
    /// that no compiler checks: the CODE each class is mapped to. Each error is
    /// obtained from the retained function by type, not by its prose, and then
    /// the same request is put through the handler.
    #[test]
    fn tc_447_324_both_retained_error_classes_map_to_bad_request() {
        let argument_request = serde_json::json!({
            "argument": {},
            "decisions": [],
            "asOf": "2026-01-01T00:00:00Z",
        });
        let input: quoin_assurance::BuildAuthoredArgumentRequest =
            serde_json::from_value(argument_request.clone()).unwrap();
        let raised: quoin_assurance::ArgumentError =
            quoin_assurance::build_authored_argument_view(&input).unwrap_err();
        // Named so the class is identified by its type rather than its message.
        let quoin_assurance::ArgumentError(_) = raised;
        let mapped = build_authored_argument(&argument_request).unwrap_err();
        assert_eq!(mapped.code, CoreErrorCode::BadRequest);
        assert_eq!(mapped.code.outcome(), Outcome::Invalid);

        let discharge_request = serde_json::json!({
            "binding": binding(),
            "facts": [],
            "asOf": "not-an-instant",
        });
        let input: quoin_assurance::BuildDischargeRequest =
            serde_json::from_value(discharge_request.clone()).unwrap();
        let raised: quoin_assurance::DischargeError =
            quoin_assurance::build_discharge_report(&input).unwrap_err();
        let quoin_assurance::DischargeError(_) = raised;
        let mapped = build_discharge(&discharge_request).unwrap_err();
        assert_eq!(mapped.code, CoreErrorCode::BadRequest);
        assert_eq!(mapped.code.outcome(), Outcome::Invalid);
    }
}
