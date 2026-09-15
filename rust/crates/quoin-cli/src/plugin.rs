// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The deprecated Rust `quoin plugin` aliases (quoin#373, Stage 8).
//!
//! `plugin` has been an alias for `module` since FR-017. Keeping its warning
//! in the shell makes the alias removable without duplicating any module work.

use clap::{ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::module;

const DEPRECATION_NOTICE: &str =
    "warning: `quoin plugin` is deprecated and will be removed; use `quoin module` instead.";

/// The deprecated alias grammar, exactly matching `quoin module`.
pub(crate) fn command() -> Command {
    module::command()
        .name("plugin")
        .about("Deprecated alias for specification module commands")
}

/// Run an alias command and attach its required deprecation warning.
pub(crate) fn run(matches: &ArgMatches) -> Result<Response, String> {
    let mut response = module::run(matches)?;
    if let Some(payload) = response.payload.as_object_mut() {
        payload.insert(
            "warning".to_owned(),
            serde_json::Value::String(DEPRECATION_NOTICE.to_owned()),
        );
    } else {
        response.payload = serde_json::json!({ "warning": DEPRECATION_NOTICE });
    }
    Ok(response)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixtures may panic"
)]
mod tests {
    use super::command;

    /// Trace: FR-017, FR-101
    #[test]
    fn tc_373_plugin_alias_keeps_the_module_grammar() {
        assert!(command().try_get_matches_from(["plugin"]).is_ok());
        assert!(
            command()
                .try_get_matches_from(["plugin", "install", "path:custom"])
                .is_ok()
        );
        assert!(
            command()
                .try_get_matches_from(["plugin", "remove"])
                .is_err()
        );
    }
}
