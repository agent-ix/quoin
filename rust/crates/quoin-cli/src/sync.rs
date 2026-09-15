// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Native successor for the former `filament-plan-sync` oclif extension.
//!
//! The published extension exposed the engine through an in-memory driver,
//! tree, and base store. Preserve that deliberately narrow contract while
//! moving its grammar and execution into the Rust binary. A durable Quoin
//! target composition is a separate concern; this adapter must not invent
//! credentials, tracker configuration, or an oclif/Node fallback.

use clap::{Arg, ArgMatches, Command, value_parser};
use filament_plan_sync::{Scope, SyncMode, SyncOptions, Tree, doubles::FakeDriver, sync};
use quoin_core::protocol::Response;

/// The native `sync` command grammar retained from the extension.
pub(crate) fn command() -> Command {
    Command::new("sync")
        .about("Synchronize the local plan tree with an external tracker target")
        .arg(
            Arg::new("target")
                .short('t')
                .long("target")
                .value_name("TARGET")
                .required(true),
        )
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .value_name("MODE")
                .default_value("full")
                .value_parser(["full", "lazy"]),
        )
        .arg(
            Arg::new("scope")
                .short('s')
                .long("scope")
                .value_name("IDS")
                .value_parser(value_parser!(String)),
        )
}

/// Run the Rust engine with the extension's current in-memory composition.
pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let target = required(arguments, "target")?;
    if !target.starts_with("jira") && !target.starts_with("gh") && !target.starts_with("github") {
        return Err(format!(
            "SyncCommand: no driver registered for target `{target}`"
        ));
    }
    let mode = match required(arguments, "mode")?.as_str() {
        "full" => SyncMode::Full,
        "lazy" => SyncMode::Lazy,
        _ => return Err("an unknown sync mode reached dispatch".to_owned()),
    };
    let scope = arguments.get_one::<String>("scope").map_or_else(
        || Scope::All,
        |scope| Scope::ids(scope.split(',').map(str::trim).filter(|id| !id.is_empty())),
    );
    let driver = FakeDriver::new();
    let store = filament_plan_sync::doubles::InMemoryBaseStore::new();
    let mut local = Tree::new();
    let result = sync(
        &driver,
        &mut local,
        &store,
        &target,
        SyncOptions {
            mode,
            scope,
            ..SyncOptions::default()
        },
    )
    .map_err(|error| format!("sync {target} failed: {error}"))?;
    Ok(Response::ok(serde_json::json!({
        "rendered": format!(
            "synced {target}: applied={} conflicts={} failed={}",
            result.applied.len(),
            result.conflicts.len(),
            result.failed.len(),
        ),
    })))
}

fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("--{name} is required"))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixtures may panic"
)]
mod tests {
    use super::{command, run};

    /// Trace: FR-026, FR-102, TC-1650
    #[test]
    fn tc_1650_sync_keeps_the_extension_grammar_and_empty_composition_result() {
        let matches = command()
            .try_get_matches_from([
                "sync",
                "-t",
                "gh/agent-ix/quoin",
                "-m",
                "lazy",
                "-s",
                "A, B",
            ])
            .expect("retained sync grammar parses");
        assert_eq!(matches.get_one::<String>("mode"), Some(&"lazy".to_owned()));
        let response = run(&matches).expect("in-memory extension composition succeeds");
        assert_eq!(
            response.payload.pointer("/rendered"),
            Some(&serde_json::json!(
                "synced gh/agent-ix/quoin: applied=0 conflicts=0 failed=0"
            )),
        );
        assert!(
            command()
                .try_get_matches_from(["sync", "--target", "gh/x", "--mode", "other"])
                .is_err()
        );
    }

    #[test]
    fn tc_1650_sync_refuses_an_unregistered_target_family() {
        let matches = command()
            .try_get_matches_from(["sync", "--target", "linear/team"])
            .expect("target spelling parses before dispatch");
        assert!(
            run(&matches)
                .expect_err("the retained command has no linear driver")
                .contains("no driver registered")
        );
    }
}
