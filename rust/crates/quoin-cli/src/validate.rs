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
    use std::fs::{create_dir_all, write};

    use super::{command, snapshot};

    fn gate(obligation: &str) -> String {
        format!(
            "#!/bin/sh\n# Gate for {obligation}: no production symbol shall call `unwrap`.\ngrep -rn \"unwrap()\" src/ | wc -l\n"
        )
    }

    fn materialize(root: &std::path::Path) {
        let tree = [
            (
                "Makefile",
                "gate:\n\t./scripts/check_unwrap.sh\n\t./third_party/gate.sh\n".to_owned(),
            ),
            ("Makefile.ci", "ci:\n\t./tools/ci_gate.sh\n".to_owned()),
            (
                "Taskfile.yaml",
                "version: '3'\ntasks:\n  gate:\n    cmds:\n      - deep/nested/verify.sh\n"
                    .to_owned(),
            ),
            ("justfile", "audit:\n\t./ops/audit.sh\n".to_owned()),
            (
                "makefile.sh",
                "#!/bin/sh\n./ops/wired_by_a_script.sh\n".to_owned(),
            ),
            (
                ".github/workflows/nested/ci.yaml",
                "jobs:\n  gate:\n    steps:\n      - run: ./ci/workflow_gate.sh\n".to_owned(),
            ),
            ("scripts/check_unwrap.sh", gate("FR-001-AC-1")),
            ("third_party/gate.sh", gate("FR-002-AC-1")),
            ("tools/ci_gate.sh", gate("FR-003-AC-1")),
            ("deep/nested/verify.sh", gate("FR-004-AC-1")),
            ("ops/audit.sh", gate("FR-005-AC-1")),
            ("ops/wired_by_a_script.sh", gate("FR-006-AC-1")),
            ("ci/workflow_gate.sh", gate("FR-007-AC-1")),
            ("vendor/Makefile", "gate:\n\t./vendor/gate.sh\n".to_owned()),
            ("vendor/gate.sh", gate("FR-900-AC-1")),
            (
                "node_modules/pkg/Makefile",
                "gate:\n\t./node_modules/pkg/gate.sh\n".to_owned(),
            ),
            ("node_modules/pkg/gate.sh", gate("FR-901-AC-1")),
            ("README.md", "# fixture\n".to_owned()),
            ("package-lock.json", "{}\n".to_owned()),
            ("src/index.ts", "export {};\n".to_owned()),
        ];
        for (relative, content) in tree {
            let path = root.join(relative);
            create_dir_all(path.parent().expect("fixture path has a parent"))
                .expect("fixture parent is created");
            write(path, content).expect("fixture file is written");
        }
    }

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

    /// The native snapshot walker is a transport optimisation only: it must
    /// reach every gate the independent on-disk validator would classify.
    ///
    /// Trace: FR-096, FR-101, TC-1650
    #[test]
    fn tc_1650_native_validate_snapshot_matches_the_independent_disk_reader() {
        let scratch = tempfile::tempdir().expect("scratch tree is created");
        materialize(scratch.path());

        let response = quoin_core::ops::validators::run(&snapshot(scratch.path()))
            .expect("the native snapshot is accepted");
        let through_native = response
            .payload
            .get("findings")
            .and_then(serde_json::Value::as_array)
            .expect("validators.run returns findings");
        let through_disk = quoin_validators::inspect_empty_gates(scratch.path())
            .expect("the independent disk reader succeeds");

        assert!(
            through_native.len() >= 7,
            "snapshot population is non-vacuous"
        );
        assert!(
            through_native
                .iter()
                .filter_map(|finding| finding.get("wiredBy").and_then(serde_json::Value::as_str))
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                >= 4,
            "snapshot exercises multiple wiring shapes"
        );
        assert_eq!(
            serde_json::to_value(through_disk).expect("disk findings serialize"),
            serde_json::Value::Array(through_native.clone())
        );
        assert!(
            through_native.iter().any(|finding| {
                finding.get("path").and_then(serde_json::Value::as_str)
                    == Some("third_party/gate.sh")
            }),
            "a non-excluded directory remains visible"
        );
        assert!(
            through_native.iter().all(|finding| {
                !finding
                    .get("path")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|path| {
                        path.starts_with("vendor/") || path.starts_with("node_modules/")
                    })
            }),
            "excluded directories remain invisible"
        );
    }
}
