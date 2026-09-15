// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The Rust `quoin semantic sweep` command (quoin#373, Stage 8).

use std::path::{Component, Path, PathBuf};
use std::process::Command as ProcessCommand;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::core_runtime::invoke;

/// The semantic command grammar.
pub(crate) fn command() -> Command {
    Command::new("semantic")
        .about("Read and assess semantic module data")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("sweep")
                .about("Classify Markdown Properties forms across corpus roots")
                .arg(Arg::new("package").long("package").required(true))
                .arg(
                    Arg::new("module-version")
                        .long("module-version")
                        .required(true),
                )
                .arg(Arg::new("out").long("out"))
                .arg(
                    Arg::new("roots")
                        .required(true)
                        .num_args(1..)
                        .action(ArgAction::Append),
                ),
        )
}

/// Execute a semantic command.
pub(crate) fn run(matches: &ArgMatches) -> Result<Response, String> {
    let (name, arguments) = matches
        .subcommand()
        .ok_or_else(|| "a semantic subcommand is required".to_owned())?;
    if name != "sweep" {
        return Err("an unknown semantic command reached dispatch".to_owned());
    }
    let roots = arguments
        .get_many::<String>("roots")
        .ok_or_else(|| "semantic sweep requires at least one root".to_owned())?
        .map(|root| absolute(root))
        .collect::<Result<Vec<_>, _>>()?;
    let package = required(arguments, "package")?;
    let version = required(arguments, "module-version")?;
    let generated_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| format!("could not format sweep timestamp: {error}"))?;
    let requested_roots = roots
        .iter()
        .map(|root| {
            serde_json::json!({
                "root": root,
                "repository": root.file_name().and_then(|name| name.to_str()).unwrap_or(""),
                "revision": revision_of(root),
            })
        })
        .collect::<Vec<_>>();
    let response = invoke(
        "semantic.sweep_corpus",
        &serde_json::json!({
            "roots": requested_roots,
            "package": package,
            "version": version,
            "generated_at": generated_at,
        }),
    )?;
    if !response.outcome.carries_payload() {
        return Ok(response);
    }
    let report = response
        .payload
        .get("report")
        .ok_or_else(|| "semantic.sweep_corpus did not return report".to_owned())?;
    let text = serde_json::to_string_pretty(report).map_err(|error| error.to_string())?;
    if let Some(requested_out) = arguments.get_one::<String>("out") {
        let out = absolute(requested_out)?;
        let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
        if !inside(&out, &cwd) && !roots.iter().any(|root| inside(&out, root)) {
            return Err(format!(
                "--out {requested_out} is outside the working directory and every corpus root"
            ));
        }
        std::fs::write(&out, format!("{text}\n"))
            .map_err(|error| format!("could not write {}: {error}", out.display()))?;
        let counts = report
            .get("counts")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| "semantic sweep report has no counts".to_owned())?;
        let count = |name| {
            counts
                .get(name)
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
        };
        let legacy = counts.get("legacy").and_then(serde_json::Value::as_object);
        let legacy_count = |name| {
            legacy
                .and_then(|items| items.get(name))
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
        };
        let summary = format!(
            "sweep: {} artifacts, {} bullet-list, {} free-column-table → {}",
            count("artifacts"),
            legacy_count("bullet-list"),
            legacy_count("free-column-table"),
            requested_out
        );
        return Ok(rendered(response, &summary));
    }
    Ok(rendered(response, &text))
}

fn required(arguments: &ArgMatches, name: &str) -> Result<String, String> {
    arguments
        .get_one::<String>(name)
        .cloned()
        .ok_or_else(|| format!("--{name} is required"))
}

fn absolute(value: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|error| error.to_string())?
    };
    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(component) => normalized.push(component),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized)
}

fn inside(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

fn revision_of(root: &Path) -> String {
    ProcessCommand::new("git")
        .args(["-C", &root.to_string_lossy(), "rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|revision| revision.trim().to_owned())
        .filter(|revision| !revision.is_empty())
        .unwrap_or_else(|| "worktree".to_owned())
}

fn rendered(response: Response, text: &str) -> Response {
    Response {
        payload: serde_json::json!({ "rendered": text }),
        diagnostics: response.diagnostics,
        outcome: response.outcome,
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "test fixtures may panic"
)]
mod tests {
    use std::path::PathBuf;

    use super::{absolute, command};

    /// Trace: FR-074, FR-101
    #[test]
    fn tc_373_sweep_requires_identity_and_one_or_more_roots() {
        assert!(
            command()
                .try_get_matches_from([
                    "semantic",
                    "sweep",
                    "--package",
                    "agent-ix/spec",
                    "--module-version",
                    "1.0.0"
                ])
                .is_err()
        );
        let parsed = command()
            .try_get_matches_from([
                "semantic",
                "sweep",
                "--package",
                "agent-ix/spec",
                "--module-version",
                "1.0.0",
                "a",
                "b",
            ])
            .expect("grammar parses");
        let (_, arguments) = parsed.subcommand().expect("sweep command");
        assert_eq!(
            arguments
                .get_many::<String>("roots")
                .expect("roots")
                .count(),
            2
        );
    }

    /// Trace: FR-074
    #[test]
    fn tc_373_output_paths_are_lexically_resolved_before_boundary_checks() {
        assert_eq!(
            absolute("/tmp/quoin-semantic-root/../outside/report.json").expect("path resolves"),
            PathBuf::from("/tmp/outside/report.json")
        );
    }
}
