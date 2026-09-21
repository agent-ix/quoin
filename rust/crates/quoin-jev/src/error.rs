// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The one error envelope this crate returns (PLAT-837).
//!
//! Follows `quoin-core`'s pattern (`crates/quoin-core/src/error.rs`): a
//! `Copy` code enum whose wire spelling is the contract, plus a `thiserror`
//! envelope carrying the code and a message. This crate has no process
//! boundary of its own -- it is a library a future CLI surface wraps -- so
//! there is no `Outcome`/exit-status mapping here; a caller that needs one
//! maps `JevErrorCode` to its own exit taxonomy at that boundary.

/// A stable, machine-matchable reason a Jev call or the lens pipeline around
/// it did not succeed.
///
/// Codes are never renamed and never reused, matching the convention
/// `quoin-core::error::CoreErrorCode` documents: a consumer that learned
/// `JEV_MISSING_KEY` keeps it forever.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum JevErrorCode {
    /// No API key was resolved through `typesafe-sdk-env`/`typesafe-sdk-config`
    /// precedence (explicit argument, then `TYPESAFE_API_KEY`, then nothing).
    MissingKey,
    /// The SDK rejected the configuration itself (an invalid retry policy, a
    /// zero timeout) before any request was built.
    InvalidConfig,
    /// The transport could not be constructed (TLS backend init failure).
    TransportInit,
    /// The API returned 401: the key was present but rejected.
    Unauthorized,
    /// The API returned 422: the request failed validation.
    Validation,
    /// The API returned 429: the rate limit was exceeded.
    RateLimited,
    /// The API returned another non-2xx status (`5xx` including a nonstandard
    /// `529`, or any status the SDK's `ApiErrorKind` does not name
    /// specifically -- see `client.rs`'s module doc for why `529` has no
    /// distinct code of its own here).
    ApiError,
    /// The request or its response could not be delivered (timeout, socket
    /// failure) after the configured retries were exhausted.
    Connection,
    /// The installed `SpecReview.analysis` schema does not yet carry
    /// `criterion-strength` (PLAT-891). Per `spec-review`'s own rule, this is
    /// an unavailable dependency, not permission to emit an invalid document.
    SchemaUnavailable,
    /// The installed schema could not be read at all (missing file, unparsable
    /// JSON) -- distinct from `SchemaUnavailable`, where the file parses fine
    /// and simply does not list the value yet.
    SchemaUnreadable,
}

impl JevErrorCode {
    /// The wire spelling. This string is the contract; do not change one.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingKey => "JEV_MISSING_KEY",
            Self::InvalidConfig => "JEV_INVALID_CONFIG",
            Self::TransportInit => "JEV_TRANSPORT_INIT",
            Self::Unauthorized => "JEV_UNAUTHORIZED",
            Self::Validation => "JEV_VALIDATION",
            Self::RateLimited => "JEV_RATE_LIMITED",
            Self::ApiError => "JEV_API_ERROR",
            Self::Connection => "JEV_CONNECTION",
            Self::SchemaUnavailable => "JEV_SCHEMA_UNAVAILABLE",
            Self::SchemaUnreadable => "JEV_SCHEMA_UNREADABLE",
        }
    }

    /// Every code this crate can emit, for a caller enumerating the catalog.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::MissingKey,
            Self::InvalidConfig,
            Self::TransportInit,
            Self::Unauthorized,
            Self::Validation,
            Self::RateLimited,
            Self::ApiError,
            Self::Connection,
            Self::SchemaUnavailable,
            Self::SchemaUnreadable,
        ]
    }

    /// Parse a wire spelling back to a code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::all().iter().copied().find(|c| c.as_str() == code)
    }
}

/// The envelope every fallible function in this crate returns.
#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}")]
pub struct JevError {
    /// The stable code.
    pub code: JevErrorCode,
    /// A human-readable message. Never the sole thing a caller matches on.
    pub message: Box<str>,
}

impl JevError {
    /// Builds an envelope from a code and a message.
    pub fn new(code: JevErrorCode, message: impl Into<Box<str>>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for JevErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, JevError>;

/// Classifies a [`typesafe_sdk_error::Error`] into this crate's own code.
///
/// The mapping against the ticket's documented error list, verified against
/// `typesafe-sdk-error 0.6.2`'s actual `ApiErrorKind` (see the crate's module
/// doc): the SDK has no distinct kind for HTTP `529`. Its classifier
/// (`ApiErrorKind::of`) matches `401`/`403`/`404`/`422`/`429` by exact value
/// and folds every other `>= 500` status, `529` included, into
/// `InternalServer` with no upper bound. `529` is therefore reported here as
/// [`JevErrorCode::ApiError`], the same as any other `5xx` -- not as a
/// distinct "overloaded" code, because the crate this wiring is built on
/// draws no such distinction. It IS retried automatically: the SDK's default
/// `RetryPolicy` retries every status in `500..600`.
#[must_use]
pub fn classify(error: &typesafe_sdk_error::Error) -> JevError {
    use typesafe_sdk_error::ApiErrorKind;

    if let Some(api) = error.api() {
        let code = match api.kind {
            ApiErrorKind::Authentication => JevErrorCode::Unauthorized,
            ApiErrorKind::UnprocessableEntity => JevErrorCode::Validation,
            ApiErrorKind::RateLimit => JevErrorCode::RateLimited,
            ApiErrorKind::BadRequest
            | ApiErrorKind::PermissionDenied
            | ApiErrorKind::NotFound
            | ApiErrorKind::InternalServer
            | ApiErrorKind::Other => JevErrorCode::ApiError,
        };
        return JevError::new(code, api.message.clone());
    }
    JevError::new(JevErrorCode::Connection, error.to_string())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{JevErrorCode, classify};

    /// Provenance: PLAT-837
    #[test]
    fn every_code_round_trips_through_its_spelling() {
        for code in JevErrorCode::all() {
            assert_eq!(JevErrorCode::from_code(code.as_str()), Some(*code));
        }
    }

    /// Provenance: PLAT-837
    #[test]
    fn no_two_codes_share_one_spelling() {
        let all = JevErrorCode::all();
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a.as_str(), b.as_str());
            }
        }
    }

    /// Provenance: PLAT-837. A 529 has no distinct `ApiErrorKind` in
    /// typesafe-sdk-error 0.6.2 -- it folds into `InternalServer` -- so this
    /// crate reports it as the generic `ApiError` code, not a bespoke one.
    #[test]
    fn a_529_status_classifies_as_the_generic_api_error_code() {
        let headers = typesafe_sdk_headers::Headers::new();
        let error = typesafe_sdk_error::Error::from_response(
            529,
            typesafe_sdk_error::Body::Empty,
            &headers,
        );
        assert_eq!(classify(&error).code, JevErrorCode::ApiError);
    }

    /// Provenance: PLAT-837
    #[test]
    fn a_401_classifies_as_unauthorized() {
        let headers = typesafe_sdk_headers::Headers::new();
        let error = typesafe_sdk_error::Error::from_response(
            401,
            typesafe_sdk_error::Body::Empty,
            &headers,
        );
        assert_eq!(classify(&error).code, JevErrorCode::Unauthorized);
    }

    /// Provenance: PLAT-837
    #[test]
    fn a_422_classifies_as_validation() {
        let headers = typesafe_sdk_headers::Headers::new();
        let error = typesafe_sdk_error::Error::from_response(
            422,
            typesafe_sdk_error::Body::Empty,
            &headers,
        );
        assert_eq!(classify(&error).code, JevErrorCode::Validation);
    }

    /// Provenance: PLAT-837
    #[test]
    fn a_429_classifies_as_rate_limited() {
        let headers = typesafe_sdk_headers::Headers::new();
        let error = typesafe_sdk_error::Error::from_response(
            429,
            typesafe_sdk_error::Body::Empty,
            &headers,
        );
        assert_eq!(classify(&error).code, JevErrorCode::RateLimited);
    }
}
