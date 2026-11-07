// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! No source file in the workspace grows past the size a reader can hold.
//!
//! The ceiling is a review criterion (quoin#464, PLAT-90): 700 lines hard.
//! A criterion enforced only by review is enforced only when someone is
//! looking, so it is read off every crate's own `src/` tree here. Crates that
//! already carry their own `tc_NNN_module_sizes` census are covered by it too;
//! this one exists so that every other crate is held to the same ceiling.
//!
//! The allow-list below is the measured debt at the time the census was
//! added: files that were already over the ceiling. Each entry carries the
//! reason it is exempt, and an entry whose file has since shrunk below the
//! ceiling fails the census so the entry is removed. Adding a new entry is a
//! decision to review, not a way to pass.
//!
//! This census carries no `Trace:` tag: it is an engineering criterion from
//! quoin#464, not an obligation any requirement states.
//!
//! Provenance: PLAT-90, quoin#464

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

/// The hard ceiling: no module may exceed this.
const HARD_CEILING: usize = 700;

/// Files already over the ceiling when the census was added, each with a reason.
const ALLOWED_ABOVE_HARD_CEILING: &[(&str, &str)] = &[
    (
        "quoin-core/src/dispatch.rs",
        "operation table plus its self-census; split pending",
    ),
    (
        "quoin-jev/src/verdict.rs",
        "verdict model and rendering together; split pending",
    ),
    (
        "quoin-jev/src/span_ballot.rs",
        "ballot model and tally together; split pending",
    ),
    (
        "quoin-assurance/src/authored/build.rs",
        "authored-argument builder; split pending",
    ),
    (
        "quoin-semantic/src/manifest.rs",
        "manifest model and parsing together; split pending",
    ),
    (
        "quoin-config/src/service.rs",
        "layered config service; split pending",
    ),
    (
        "quoin-jev/src/cassette.rs",
        "cassette record and replay together; split pending",
    ),
    (
        "quoin-jev/src/corpus_check.rs",
        "corpus checks in one file; split pending",
    ),
];

/// Every `src/**/*.rs` under `rust/crates`, as `(crate/relative path, line count)`.
fn modules() -> Vec<(String, usize)> {
    fn walk(crates: &Path, directory: &Path, into: &mut Vec<(String, usize)>) {
        for entry in std::fs::read_dir(directory).expect("directory is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(crates, &path, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let name = path
                    .strip_prefix(crates)
                    .expect("a path under crates/")
                    .to_string_lossy()
                    .replace('\\', "/");
                let contents = std::fs::read_to_string(&path).expect("a source file");
                into.push((name, contents.lines().count()));
            }
        }
    }
    let crates: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&crates).expect("crates/ is readable") {
        let src = entry.expect("a crate directory").path().join("src");
        if src.is_dir() {
            walk(&crates, &src, &mut found);
        }
    }
    found.sort();
    found
}

#[test]
fn tc_90_001_no_module_is_over_the_hard_ceiling_unless_listed() {
    let modules = modules();
    assert!(
        modules.len() > 100,
        "anti-vacuity: expected the whole workspace, saw {}",
        modules.len()
    );

    for (name, lines) in &modules {
        // `tests.rs` modules hold test code, which the ceiling does not target.
        if name.ends_with("/tests.rs") {
            continue;
        }
        if let Some((_, reason)) = ALLOWED_ABOVE_HARD_CEILING
            .iter()
            .find(|(allowed, _)| allowed == name)
        {
            assert!(
                *lines > HARD_CEILING,
                "{name} is {lines} lines, under the ceiling, but still carries an exemption \
                 ({reason}); remove the entry"
            );
            continue;
        }
        assert!(
            *lines <= HARD_CEILING,
            "{name} is {lines} lines, over the {HARD_CEILING}-line hard ceiling (quoin#464). \
             Split it on responsibility. Do not raise the ceiling."
        );
    }
}

#[test]
fn tc_90_002_every_exemption_names_a_file_that_exists() {
    let modules = modules();
    for (allowed, reason) in ALLOWED_ABOVE_HARD_CEILING {
        assert!(!reason.is_empty(), "{allowed} carries no reason");
        assert!(
            modules.iter().any(|(name, _)| name == allowed),
            "{allowed} is exempt but no such file exists; remove the entry"
        );
    }
}
