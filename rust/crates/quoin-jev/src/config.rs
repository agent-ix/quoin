// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Resolving Jev configuration through the SDK's own precedence (PLAT-837).
//!
//! The ticket is explicit that the key is the LAST thing plugged in, and that
//! this crate must not invent an env var name or a config file: every setting
//! is read through `typesafe-sdk-config`'s `Builder`, which resolves
//! explicit argument, then `typesafe-sdk-env`'s `TYPESAFE_API_KEY` (and
//! siblings), then the SDK's own default -- verified against
//! `typesafe-sdk-config 0.6.2`'s `build.rs`/`config.rs` and
//! `typesafe-sdk-env 0.6.2`'s `lib.rs`, not assumed.

use typesafe_sdk_config::{Builder, Config};
use typesafe_sdk_env::Source;

use crate::error::{JevError, JevErrorCode, Result};

/// Resolves a [`Config`] from `env`, applying no override of our own.
///
/// Production callers pass [`typesafe_sdk_env::Process`]; tests pass a
/// [`typesafe_sdk_env::Fixed`] environment, so a test exercising the missing-
/// or present-key paths never touches the real process environment (which
/// would be both untestable in parallel and exactly the ambient-state
/// dependency `rust-review` flags).
///
/// # Errors
/// [`JevErrorCode::MissingKey`] when no key is resolved anywhere in the
/// precedence chain; [`JevErrorCode::InvalidConfig`] when the SDK rejects a
/// setting (its own default retry policy and timeout always validate, so
/// this path is only reachable once a caller starts overriding them).
pub fn resolve(env: &impl Source) -> Result<Config> {
    Builder::new().build(env).map_err(|error| {
        // typesafe-sdk-config's own `missing_key()` message names the env var;
        // a key genuinely absent is the one case worth a distinct code, since
        // "there is no key yet" is this ticket's documented, expected state
        // rather than a bug to alarm on.
        let code = if error.to_string().contains("No API key was provided") {
            JevErrorCode::MissingKey
        } else {
            JevErrorCode::InvalidConfig
        };
        JevError::new(code, error.to_string())
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use typesafe_sdk_env::Fixed;

    use super::resolve;
    use crate::error::JevErrorCode;

    /// Provenance: PLAT-837. No network, no key: an empty environment must
    /// fail loudly with a named code, never silently produce a usable client.
    #[test]
    fn an_empty_environment_reports_the_missing_key_code() {
        let env = Fixed::new(&[]);
        let error = resolve(&env).expect_err("no key was provided");
        assert_eq!(error.code, JevErrorCode::MissingKey);
    }

    /// Provenance: PLAT-837
    #[test]
    fn a_key_from_the_documented_env_var_resolves() {
        let env = Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")]);
        let config = resolve(&env).expect("the key is present");
        assert_eq!(config.key_hint(), "***_key");
    }

    /// Provenance: PLAT-837. Whitespace-only counts as unset, per
    /// typesafe-sdk-env's own rule -- exercised here rather than assumed, so
    /// a future SDK bump that changed this would fail this test rather than
    /// silently accept an empty credential.
    #[test]
    fn a_whitespace_only_key_is_treated_as_absent() {
        let env = Fixed::new(&[("TYPESAFE_API_KEY", "   ")]);
        let error = resolve(&env).expect_err("whitespace is not a key");
        assert_eq!(error.code, JevErrorCode::MissingKey);
    }

    /// Provenance: PLAT-837. The default base URL and model come from the
    /// SDK, not from anything this crate invents.
    #[test]
    fn defaults_come_from_the_sdk_not_this_crate() {
        let env = Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")]);
        let config = resolve(&env).expect("the key is present");
        assert_eq!(config.base_url, typesafe_sdk_config::DEFAULT_BASE_URL);
        assert_eq!(config.default_model, typesafe_sdk_config::DEFAULT_MODEL);
    }
}
