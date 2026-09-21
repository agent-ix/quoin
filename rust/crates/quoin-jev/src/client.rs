// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Building a [`typesafe_sdk_client::Client`] over the real or a scripted
//! transport (PLAT-837).
//!
//! **On the mock seam.** The task brief that opened this ticket says "the
//! sibling `typesafe-sdk` crate ships a `mock` feature backed by `wiremock`".
//! That is a discrepancy worth recording precisely: `typesafe-sdk` (0.1.2,
//! `codeitlikemiley/typesafe-sdk-rust`) is a DIFFERENT, unrelated crate from
//! the `typesafe-sdk-client`/`-answers`/`-questions`/... family (0.6.2,
//! `douglance/typesafe-sdk-rs`) this ticket pins -- different author,
//! different repository, and its own `Cargo.toml` depends on none of the
//! 0.6.2 family. It is not a "sibling" of anything pinned here.
//!
//! The mock capability this ticket actually needs lives one level down, in
//! `typesafe-sdk-http` itself (part of the pinned 0.6.2 family, no feature
//! flag required): [`typesafe_sdk_http::Mock`] and
//! [`typesafe_sdk_http::Exchange`] are a hand-rolled scripted [`Transport`],
//! not a `wiremock` server. Every test in this crate uses that seam --
//! exactly "the seam the SDK provides" the ticket asked for, just not the
//! crate the ticket named.

use std::sync::Arc;

use typesafe_sdk_client::Client;
use typesafe_sdk_config::Config;
use typesafe_sdk_http::{Reqwest, Transport};

use crate::error::{JevError, JevErrorCode, Result};

/// Builds a client against the real network, using `config`'s resolved
/// settings (base URL, key, retry policy) unchanged.
///
/// # Errors
/// [`JevErrorCode::TransportInit`] if the TLS backend cannot be initialised.
pub fn production(config: Config) -> Result<Client> {
    let transport = Reqwest::new()
        .map_err(|error| JevError::new(JevErrorCode::TransportInit, error.to_string()))?;
    Ok(Client::with_transport(config, Arc::new(transport)))
}

/// Builds a client over an arbitrary [`Transport`], for a scripted
/// [`typesafe_sdk_http::Mock`] in tests or any other injected seam.
#[must_use]
pub fn with_transport(config: Config, transport: Arc<dyn Transport>) -> Client {
    Client::with_transport(config, transport)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use std::sync::Arc;

    use typesafe_sdk_env::Fixed;
    use typesafe_sdk_http::{Exchange, Mock};

    use super::with_transport;
    use crate::config::resolve;

    /// Provenance: PLAT-837. No network, no key beyond a fixed test value:
    /// a client built over the mock transport must actually round-trip a
    /// request through it, proving the wiring (not just that types compile).
    #[tokio::test]
    async fn a_client_over_the_mock_transport_reaches_the_scripted_answer() {
        let env = Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")]);
        let config = resolve(&env).expect("the key is present");
        let mock = Arc::new(Mock::new(vec![Exchange::ok(
            r#"{"model":"jev-1.0.0","answers":{},"usage":{"input_tokens":1,"output_tokens":0}}"#,
        )]));
        let client = with_transport(config, mock.clone());
        let request = typesafe_sdk_client::SystemOneRequest::new(
            "state",
            typesafe_sdk_questions::questions([(
                "q",
                typesafe_sdk_questions::noul("is this true?"),
            )]),
        );
        let response = client.system_one(request).await.expect("mocked 200");
        assert_eq!(response.model, "jev-1.0.0");
        assert_eq!(mock.attempts(), 1);
    }
}
