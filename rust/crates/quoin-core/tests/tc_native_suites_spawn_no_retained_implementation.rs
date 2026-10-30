// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! After cutover no integration suite in the workspace starts a JavaScript
//! runtime. The retained TypeScript implementation is gone; a test that spawned
//! one would be a second, unowned oracle.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use ix_trace_rs::trace;

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            rust_files(&path, into);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            into.push(path);
        }
    }
}

/// No `tests/` tree of any workspace crate spawns a JavaScript runtime or
/// package runner.
#[trace("FR-098-AC-11")]
#[test]
fn no_integration_suite_spawns_a_retained_implementation() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate sits inside the crates directory");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(crates).expect("the crates directory is readable") {
        let tests = entry.expect("a crate directory").path().join("tests");
        rust_files(&tests, &mut files);
    }
    assert!(
        !files.is_empty(),
        "the walk must reach the integration suites"
    );

    // Assembled from parts so this file is not its own offence.
    let runtimes = ["no", "de"].concat();
    let runners = ["np", "x"].concat();
    let transpilers = ["ts", "x"].concat();
    let managers = ["pn", "pm"].concat();
    let needles: Vec<String> = [runtimes, runners, transpilers, managers]
        .iter()
        .flat_map(|name| [format!("new(\"{name}\")"), format!("new(\"{name}.exe\")")])
        .collect();

    let mut offences = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("a readable test source");
        for needle in &needles {
            if text.contains(needle.as_str()) {
                offences.push(format!("{}: {needle}", file.display()));
            }
        }
    }
    assert!(
        offences.is_empty(),
        "suites spawning a runtime: {offences:#?}"
    );
}
