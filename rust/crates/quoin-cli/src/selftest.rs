// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Run the self-test cases embedded in the installed binary (FR-215, PLAT-1639).

use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use quoin_core::protocol::Response;
use std::collections::BTreeSet;
use std::path::{Component, Path};

#[derive(Clone, Copy)]
struct EmbeddedCase {
    id: &'static str,
    files: &'static [EmbeddedCaseFile],
}

#[derive(Clone, Copy)]
struct EmbeddedCaseFile {
    path: &'static str,
    bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/embedded_selftest.rs"));

/// The concise command description used in root help.
pub(crate) const ABOUT: &str = "Run checks embedded in this quoin binary.";

/// The `quoin selftest` grammar.
pub(crate) fn command() -> Command {
    Command::new("selftest")
        .about(ABOUT)
        .arg(
            Arg::new("list")
                .long("list")
                .help("List embedded self-test case ids")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("run")
                .long("run")
                .help("Run all embedded cases or the selected cases")
                .action(ArgAction::SetTrue),
        )
        .group(
            ArgGroup::new("mode")
                .args(["list", "run"])
                .required(true)
                .multiple(false),
        )
        .arg(
            Arg::new("case")
                .long("case")
                .value_name("ID")
                .help("Select a case to run; may be repeated")
                .action(ArgAction::Append)
                .requires("run"),
        )
        .arg(
            Arg::new("out")
                .long("out")
                .value_name("PATH")
                .help("Write the run record to a file")
                .requires("run"),
        )
}

/// Execute `quoin selftest`.
pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    if arguments.get_flag("list") {
        return Ok(list());
    }

    let selected = select_cases(arguments)?;
    let _ = selected;
    Err("the self-test runner is not ready".to_owned())
}

fn list() -> Response {
    let rendered = EMBEDDED_CASES
        .iter()
        .map(|case| case.id)
        .collect::<Vec<_>>()
        .join("\n");
    Response::ok(serde_json::json!({
        "rendered": rendered,
        "renderedStream": "stdout",
    }))
}

fn select_cases(arguments: &ArgMatches) -> Result<Vec<&'static EmbeddedCase>, String> {
    let requested = arguments
        .get_many::<String>("case")
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if requested.is_empty() {
        return Ok(EMBEDDED_CASES.iter().collect());
    }

    let available = EMBEDDED_CASES
        .iter()
        .map(|case| case.id)
        .collect::<BTreeSet<_>>();
    if let Some(unknown) = requested
        .iter()
        .copied()
        .find(|id| !available.contains(id))
    {
        return Err(format!("unknown self-test case id {unknown:?}"));
    }

    let requested = requested.into_iter().collect::<BTreeSet<_>>();
    Ok(EMBEDDED_CASES
        .iter()
        .filter(|case| requested.contains(case.id))
        .collect())
}

fn materialize_case(case: &EmbeddedCase, root: &Path) -> Result<(), String> {
    for file in case.files {
        let relative = Path::new(file.path);
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!(
                "embedded case {:?} has an invalid path {:?}",
                case.id, file.path
            ));
        }
        let destination = root.join(relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("create embedded case directory: {error}"))?;
        }
        std::fs::write(destination, file.bytes)
            .map_err(|error| format!("write embedded case file: {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "in a test, a panic is the failure report"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("FR-215-AC-6")]
    #[test]
    fn tc_215_case_selection_is_unique_and_follows_embedded_order() {
        let matches = command()
            .try_get_matches_from([
                "selftest",
                "--run",
                "--case",
                EMBEDDED_CASES[0].id,
                "--case",
                EMBEDDED_CASES[0].id,
            ])
            .expect("the repeated case flags parse");
        let selected = select_cases(&matches).expect("the case id is embedded");
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, EMBEDDED_CASES[0].id);
    }

    #[trace("FR-215-AC-6")]
    #[test]
    fn tc_215_unknown_case_is_rejected_before_execution() {
        let matches = command()
            .try_get_matches_from(["selftest", "--run", "--case", "missing-case"])
            .expect("case selection parses before semantic validation");
        let error = select_cases(&matches).expect_err("unknown case ids are rejected");
        assert!(error.contains("missing-case"));
    }

    #[test]
    fn tc_215_list_shows_embedded_case_ids_in_declaration_order() {
        let listed = list();
        assert_eq!(
            listed.payload.get("rendered").and_then(serde_json::Value::as_str),
            Some("selftest-list")
        );
    }

    #[test]
    fn tc_215_materialization_writes_only_case_relative_paths() {
        let root = tempfile::tempdir().expect("a temporary case directory");
        let case = &EMBEDDED_CASES[0];
        materialize_case(case, root.path()).expect("embedded case files materialize");
        assert_eq!(
            std::fs::read(root.path().join("run.sh")).expect("the script was written"),
            case.files
                .iter()
                .find(|file| file.path == "run.sh")
                .expect("the case contains a script")
                .bytes
        );
    }
}
