// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for retained `quoin validate`.

use std::collections::BTreeMap;
use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::{Outcome, Response};

use crate::core_runtime::invoke;

pub(crate) fn command() -> Command {
    Command::new("validate")
        .about("Validate repository QA gates and report located defects")
        .arg(Arg::new("repo").long("repo").default_value("."))
        .arg(Arg::new("strict").long("strict").action(ArgAction::SetTrue))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let repo = required(arguments, "repo")?;
    let response = invoke("validators.run", &snapshot(Path::new(&repo)))?;
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let findings = response
        .payload
        .get("findings")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "validators.run did not return findings".to_owned())?;
    let rendered = if arguments.get_flag("json") {
        serde_json::to_string_pretty(&serde_json::json!({ "findings": findings }))
            .map_err(|error| error.to_string())?
    } else if findings.is_empty() {
        "repository QA gates: no findings".to_owned()
    } else {
        let mut lines = findings
            .iter()
            .map(|finding| {
                format!(
                    "[warning] {}: {}:{}: {}",
                    text(finding, "kind"),
                    text(finding, "path"),
                    number(finding, "line"),
                    text(finding, "summary")
                )
            })
            .collect::<Vec<_>>();
        lines.push(format!("{} gate finding(s)", findings.len()));
        lines.join("\n")
    };
    Ok(Response {
        payload: serde_json::json!({ "rendered": rendered }),
        diagnostics: response.diagnostics,
        outcome: if arguments.get_flag("strict") && !findings.is_empty() {
            Outcome::Partial
        } else {
            response.outcome
        },
    })
}

fn snapshot(root: &Path) -> serde_json::Value {
    let mut files = BTreeMap::<String, Option<Vec<String>>>::new();
    let mut unlistable = Vec::new();
    walk(root, root, &mut files, &mut unlistable);
    serde_json::json!({ "files": files, "unlistable": unlistable })
}

fn walk(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<String, Option<Vec<String>>>,
    unlistable: &mut Vec<String>,
) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        unlistable.push(relative(root, directory));
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if !excluded(entry.file_name().as_os_str()) {
                walk(root, &path, files, unlistable);
            }
            continue;
        }
        if kind.is_file() {
            let name = relative(root, &path);
            if could_matter(&name) {
                files.insert(
                    name,
                    std::fs::read_to_string(path)
                        .ok()
                        .map(|raw| raw.split('\n').map(str::to_owned).collect()),
                );
            }
        }
    }
}

fn excluded(name: &std::ffi::OsStr) -> bool {
    matches!(
        name.to_str(),
        Some(".git" | "dist" | "node_modules" | "spec" | "target" | "vendor")
    )
}
fn could_matter(path: &str) -> bool {
    let name = path
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    Path::new(&name)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("sh"))
        || name.starts_with("makefile")
        || name.starts_with("taskfile")
        || name == "justfile"
        || name == "package.json"
        || path.starts_with(".github/workflows/")
}
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("--{name} is required"))
}
fn text<'a>(value: &'a serde_json::Value, name: &str) -> &'a str {
    value
        .get(name)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
}
fn number(value: &serde_json::Value, name: &str) -> u64 {
    value
        .get(name)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default()
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures may panic")]
mod tests {
    use super::command;

    /// Trace: FR-096, FR-062
    #[test]
    fn tc_412_742_preserves_the_validate_grammar() {
        let matches = command()
            .try_get_matches_from(["validate", "--repo", "other", "--strict", "--json"])
            .expect("grammar parses");
        assert_eq!(
            matches.get_one::<String>("repo").map(String::as_str),
            Some("other")
        );
        assert!(matches.get_flag("strict"));
        assert!(matches.get_flag("json"));
    }
}
