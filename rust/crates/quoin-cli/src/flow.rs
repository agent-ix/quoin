// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust launchers for the `ix-flow` specification workflows shipped by the
//! published `@agent-ix/ix-spec-workflows` npm package (PLAT-157). Nothing
//! here bundles a copy of that package: `skill_path` resolves into an
//! installed `node_modules/@agent-ix/ix-spec-workflows`, which
//! `package.json` at the repository root declares as a dependency.

use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use clap::{Arg, ArgAction, ArgMatches, Command};
use quoin_core::protocol::Response;

use crate::invocation;

/// `(quoin subcommand, @agent-ix/ix-spec-workflows `spec/` directory name)`.
const FLOWS: [(&str, &str); 3] = [
    ("review", "review"),
    ("matrix", "matrix"),
    ("to-plan", "to-plan"),
];

pub(crate) fn run(name: &str, arguments: &ArgMatches) -> Result<Response, String> {
    let (_, runtime_skill) = FLOWS
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .ok_or_else(|| "an unknown flow command reached dispatch".to_owned())?;
    let path = skill_path(runtime_skill)?;
    let home = ix_home();
    let mut command = ProcessCommand::new("ix-flow");
    command
        .arg("run")
        .arg(name)
        .arg("--path")
        .arg(path)
        .arg("--config-root")
        .arg(&home)
        .arg("--state-dir")
        .arg(home.join("flows"));
    if let Some(id) = arguments.get_one::<String>("id") {
        command.arg("--id").arg(id);
    }
    if arguments.get_flag("json") {
        command.arg("--json");
    }
    if let Some(targets) = arguments.get_many::<String>("target") {
        for target in targets {
            command.arg("--target").arg(target);
        }
    }
    let output = command
        .output()
        .map_err(|error| format!("could not start ix-flow: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if detail.is_empty() {
            format!("ix-flow {name} failed")
        } else {
            detail
        });
    }
    let rendered = String::from_utf8(output.stdout)
        .map_err(|error| format!("ix-flow wrote non-UTF-8 output: {error}"))?;
    Ok(Response::ok(
        serde_json::json!({ "rendered": rendered.trim_end() }),
    ))
}

/// The shared grammar for one bundled workflow launcher.
pub(crate) fn command(name: &'static str) -> Command {
    Command::new(name)
        .about("Start the bundled specification workflow")
        .arg(Arg::new("target").long("target").action(ArgAction::Append))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
        .arg(Arg::new("id").long("id"))
}

fn skill_path(runtime_skill: &str) -> Result<PathBuf, String> {
    let mut roots = Vec::new();
    if let Some(root) = std::env::var_os("IX_SPEC_WORKFLOWS_ROOT") {
        roots.push(PathBuf::from(root));
    }
    // `@agent-ix/ix-spec-workflows` (PLAT-157): quoin depends on the published
    // package rather than carrying a copy of its `dist/` bundle and its
    // `SKILL.md`/`workflows/`/`scripts/` sources under `skills/*/workflow-assets/`.
    // `package.json` at the repository root names the one dependency, so a plain
    // `node_modules/@agent-ix/ix-spec-workflows` lookup is enough -- there is no
    // second dependency that could get it hoisted somewhere else.
    roots.push(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../node_modules/@agent-ix/ix-spec-workflows/spec"),
    );
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.join("node_modules/@agent-ix/ix-spec-workflows/spec"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join(".ix/plugins/ix-spec-workflows/spec"));
    }
    roots
        .into_iter()
        .map(|root| root.join(runtime_skill))
        .find(|candidate| candidate.is_dir())
        .ok_or_else(|| {
            format!(
                "could not find the {runtime_skill} workflow from @agent-ix/ix-spec-workflows; \
                 run `pnpm install` at the repository root, or set IX_SPEC_WORKFLOWS_ROOT to a \
                 checkout of agent-ix/ix-spec-workflows's `spec/` directory"
            )
        })
}

fn ix_home() -> PathBuf {
    if let Some(root) = invocation::current().config_root() {
        return root.clone();
    }
    std::env::var_os("IX_HOME")
        .filter(|home| !home.is_empty())
        .map_or_else(
            || {
                std::env::var_os("HOME").map_or_else(
                    || PathBuf::from(".ix"),
                    |home| PathBuf::from(home).join(".ix"),
                )
            },
            PathBuf::from,
        )
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures may panic")]
mod tests {
    use super::command;

    /// Trace: FR-062
    #[test]
    fn tc_373_742_flow_launchers_keep_the_shared_grammar() {
        let matches = command("review")
            .try_get_matches_from([
                "review", "--target", "spec", "--target", "other", "--id", "run-1", "--json",
            ])
            .expect("grammar parses");
        assert_eq!(
            matches
                .get_many::<String>("target")
                .expect("targets")
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["spec", "other"]
        );
        assert_eq!(
            matches.get_one::<String>("id").map(String::as_str),
            Some("run-1")
        );
        assert!(matches.get_flag("json"));
    }
}
