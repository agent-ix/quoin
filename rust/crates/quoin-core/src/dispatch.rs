// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Routing `<domain>.<op>` to an operation, and nothing else.
//!
//! This module holds no domain logic and must not acquire any. It is the one
//! place that knows which operations exist, so "which operations exist" is one
//! fact in one place — an exhaustive `match` the compiler checks, not a
//! registry assembled at run time.

use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::Response;

/// Every operation this build answers, in the wire spelling.
///
/// Exposed so a caller — and `quoin-difftest` — can enumerate the surface
/// rather than discover it by trying names.
pub const OPERATIONS: &[&str] = &["core.ping"];

/// Route one request.
///
/// `op` is the sole command-line argument; `request` is the parsed stdin
/// document.
///
/// # Errors
///
/// [`CoreErrorCode::UnknownOp`] when `op` names nothing this build implements,
/// or whatever the operation itself returns.
pub fn dispatch(op: &str, request: &serde_json::Value) -> Result<Response, CoreError> {
    match op {
        "assurance.requirement_of" => crate::ops::assurance::requirement_of(request),
        "assurance.build_case" => crate::ops::assurance::build_case(request),
        "assurance.render_case" => crate::ops::assurance::render_case(request),
        "core.ping" => crate::ops::core::ping(request),
        _ => Err(
            CoreError::new(CoreErrorCode::UnknownOp, "no such operation in this build")
                .with_context("op", op)
                .with_context("known", OPERATIONS.join(",")),
        ),
    }
}

/// Parse a stdin document.
///
/// An empty or whitespace-only stdin means the empty request `{}`. A request
/// that is not a JSON **object** is refused: the unit of IPC is a named
/// operation with named arguments, so a bare array or scalar is a caller
/// mistake worth naming rather than coercing.
///
/// # Errors
///
/// [`CoreErrorCode::BadJson`] when stdin is neither empty nor a JSON object.
pub fn parse_request(stdin: &str) -> Result<serde_json::Value, CoreError> {
    if stdin.trim().is_empty() {
        return Ok(serde_json::Value::Object(serde_json::Map::new()));
    }
    let value: serde_json::Value = serde_json::from_str(stdin)
        .map_err(|e| CoreError::new(CoreErrorCode::BadJson, e.to_string()))?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(
            CoreError::new(CoreErrorCode::BadJson, "a request must be a JSON object")
                .with_context("observed_type", json_type_name(&value).to_owned()),
        )
    }
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Read the operation name out of the argument vector.
///
/// # Errors
///
/// [`CoreErrorCode::BadUsage`] for anything but exactly one argument shaped
/// `<domain>.<op>`.
pub fn parse_operation(args: &[String]) -> Result<&str, CoreError> {
    let [op] = args else {
        return Err(CoreError::new(
            CoreErrorCode::BadUsage,
            "usage: quoin-core <domain>.<op>  (JSON request on stdin)",
        )
        .with_context("argument_count", args.len().to_string()));
    };
    if op.split('.').filter(|part| !part.is_empty()).count() == 2 && op.matches('.').count() == 1 {
        Ok(op)
    } else {
        Err(CoreError::new(
            CoreErrorCode::BadUsage,
            "an operation is spelled <domain>.<op>",
        )
        .with_context("argument", op.clone()))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_operation_names_the_ones_that_exist() {
        let error = dispatch("evidence.record", &serde_json::json!({})).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::UnknownOp);
        assert_eq!(error.outcome().code(), 3);
        assert_eq!(error.context["known"], "core.ping");
    }

    #[test]
    fn empty_stdin_is_the_empty_request() {
        assert_eq!(parse_request("   \n ").unwrap(), serde_json::json!({}));
    }

    #[test]
    fn malformed_json_is_invalid_not_internal() {
        let error = parse_request("{oops").unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadJson);
        assert_eq!(error.outcome().code(), 3);
    }

    #[test]
    fn a_non_object_request_is_refused_by_type() {
        let error = parse_request("[1,2]").unwrap_err();
        assert_eq!(error.context["observed_type"], "array");
    }

    #[test]
    fn usage_needs_exactly_one_dotted_argument() {
        assert_eq!(
            parse_operation(&["core.ping".to_owned()]).unwrap(),
            "core.ping"
        );
        for bad in [
            vec![],
            vec!["core".to_owned()],
            vec!["core.ping".to_owned(), "x".to_owned()],
        ] {
            assert_eq!(
                parse_operation(&bad).unwrap_err().code,
                CoreErrorCode::BadUsage
            );
        }
        assert_eq!(
            parse_operation(&["core.".to_owned()]).unwrap_err().code,
            CoreErrorCode::BadUsage
        );
    }
}
