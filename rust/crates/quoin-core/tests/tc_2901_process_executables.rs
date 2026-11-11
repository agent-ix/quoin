// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! quoin transcribes; the consumer's CI executes (ADR-0011 invariant 1).
//!
//! The executables production code starts are asserted as a whole, so a new
//! binary fails by default instead of passing until someone extends a list.
//! Build scripts and the two developer tools (`quoin-suite-trace`,
//! `quoin-workflow-check`) are not part of the shipped engine and are skipped.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;

const DEVELOPER_TOOLS: &[&str] = &["quoin-suite-trace", "quoin-workflow-check"];

/// Executables quoin starts. `ix-flow` is named by `runner_program()` in
/// `quoin-cli/src/flow.rs`; `config edit` starts the operator's own `$EDITOR`.
const EXPECTED: &[&str] = &["$EDITOR", "git", "ix-flow"];

fn rust_files(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("directory readable") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            rust_files(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            into.push(path);
        }
    }
}

/// The executables a source text starts through `std::process::Command`.
/// Text after `#[cfg(test)]` is test scaffolding and is not read. A bare
/// `Command::new` counts only where the file imports the process `Command`,
/// since clap's `Command` shares the name.
fn executables_in(source: &str) -> BTreeSet<String> {
    let production = source.split("#[cfg(test)]").next().unwrap_or(source);
    let imports_process = production.contains("use std::process::Command")
        || production.contains("use std::process::{Command");
    let mut found = BTreeSet::new();
    for line in production.lines() {
        let Some((head, tail)) = line.split_once("Command::new(") else {
            continue;
        };
        let process_qualified = head.ends_with("process::")
            || head.ends_with("Process")
            || (imports_process && !head.ends_with("::"));
        if !process_qualified {
            continue;
        }
        let argument = tail.split(')').next().unwrap_or("").trim();
        let name = match argument {
            "runner_program()" => "ix-flow".to_owned(),
            "&editor" => "$EDITOR".to_owned(),
            literal if literal.starts_with('"') => literal.trim_matches('"').to_owned(),
            other => format!("<unreadable executable: {other}>"),
        };
        found.insert(name);
    }
    found
}

/// FR-036-AC-8 / CON-1.
#[trace("FR-036-AC-8", "TC-203")]
#[test]
fn quoin_executes_exactly_git_and_ix_flow_and_the_operators_editor() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut files = Vec::new();
    for entry in std::fs::read_dir(crates).unwrap() {
        let krate = entry.unwrap().path();
        let name = krate.file_name().unwrap().to_string_lossy().into_owned();
        if !DEVELOPER_TOOLS.contains(&name.as_str()) && krate.join("src").is_dir() {
            rust_files(&krate.join("src"), &mut files);
        }
    }
    assert!(
        files.len() > 50,
        "the walk must reach the production sources"
    );
    let mut found = BTreeSet::new();
    for file in files {
        found.extend(executables_in(&std::fs::read_to_string(file).unwrap()));
    }
    let expected: BTreeSet<String> = EXPECTED.iter().map(|name| (*name).to_owned()).collect();
    assert_eq!(found, expected);
}

/// The check must be able to fail: a new executable and one it cannot read are
/// both reported, and clap's `Command` is not mistaken for a process.
#[trace("FR-036-AC-8", "TC-203")]
#[test]
fn the_executable_check_sees_an_injected_binary() {
    let injected = "use std::process::Command;\nfn f() { Command::new(\"curl\"); std::process::Command::new(tool); }\n";
    assert_eq!(
        executables_in(injected),
        BTreeSet::from([
            "curl".to_owned(),
            "<unreadable executable: tool>".to_owned()
        ])
    );
    let clap = "use clap::Command;\nfn f() { Command::new(\"config\"); }\n";
    assert!(executables_in(clap).is_empty());
}
