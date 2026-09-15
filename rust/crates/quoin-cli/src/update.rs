// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for `quoin update`.

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

const REGISTRY: &str = "https://registry.npmjs.org/";

pub(crate) fn command() -> Command {
    Command::new("update")
        .about("Check for and install the latest published quoin")
        .arg(Arg::new("check").long("check").action(ArgAction::SetTrue))
        .arg(Arg::new("registry").long("registry"))
}
pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let registry = arguments
        .get_one::<String>("registry")
        .map_or(REGISTRY, String::as_str);
    let latest = npm(["view", "@agent-ix/quoin", "version", "--registry", registry])?;
    let latest = latest.trim();
    let current = env!("CARGO_PKG_VERSION");
    if arguments.get_flag("check") {
        return Ok(Response::ok(
            serde_json::json!({"rendered": format!("quoin update: current {current}, latest {latest}")}),
        ));
    }
    if latest == current {
        return Ok(Response::ok(
            serde_json::json!({"rendered": format!("quoin update: already at {current}")}),
        ));
    }
    npm(["install", "-g", "@agent-ix/quoin", "--registry", registry])?;
    Ok(Response::ok(
        serde_json::json!({"rendered": format!("quoin update: updated {current} → {latest}")}),
    ))
}
fn npm<const N: usize>(args: [&str; N]) -> Result<String, String> {
    let output = std::process::Command::new("npm")
        .args(args)
        .output()
        .map_err(|error| format!("cannot start npm: {error}"))?;
    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|error| error.to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures may panic")]
mod tests {
    use super::command;

    /// Trace: FR-062
    #[test]
    fn tc_373_745_update_preserves_check_and_registry_grammar() {
        let matches = command()
            .try_get_matches_from(["update", "--check", "--registry", "http://npm.ix/"])
            .expect("grammar parses");
        assert!(matches.get_flag("check"));
        assert_eq!(
            matches.get_one::<String>("registry").map(String::as_str),
            Some("http://npm.ix/")
        );
    }
}
