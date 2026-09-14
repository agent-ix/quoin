// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! ADR-0011 invariant 1: the auditor runs nothing.
//!
//! Trace: quoin#383-AC-3 — execution is unrepresentable in the auditor's
//! inputs, not merely unused by its code.
//! Trace: FR-032-CON-2 — the auditor reads no clock, walks no filesystem and
//! spawns no subprocess. Carried over from `tests/arch-boundaries.test.ts`,
//! deleted with `src/auditor/` at cutover (quoin#501); the Rust form is the
//! stronger one, since an impure input cannot be `Inert` at all.
//! Provenance: quoin#383, quoin#501
//!
//! # The type-system half
//!
//! `src/auditor/audit.ts:14` says *"the auditor runs nothing; it reads the
//! store and reports"*. [`quoin_auditor::Inert`] is a **sealed** marker trait
//! whose supertrait is `serde::de::DeserializeOwned`, and every auditor and
//! advisor input implements it. That is what makes the invariant mechanical:
//! `std::process::Command`, a `std::fs::File`, a `TcpStream`, a raw fd, a
//! closure and a `dyn Trait` are none of them `DeserializeOwned`, so **none of
//! them can be a field of an `Inert` type**. Adding one stops the build. The
//! `const _` census in `src/inert.rs` is what forces every input through the
//! bound, and the two tests below are what stop that census from being
//! quietly emptied.
//!
//! # The source half
//!
//! A type-level proof covers the inputs. It does not stop someone writing
//! `Command::new` in the middle of `audit()` against a path it built itself,
//! so the second test reads this crate's own `src/audit/` and `src/advise/`
//! trees and refuses the tokens that reach the world. Its anti-vacuity floor
//! is a positive control: the same scanner must FIND those tokens in
//! `src/catalog/source.rs`, which is the one module allowed to have them.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_auditor::advise::ObligationFacts;
use quoin_auditor::audit::AuditInput;
use quoin_auditor::catalog::MethodCatalog;
use quoin_auditor::inert::Inert;

/// Every auditor input is [`Inert`], checked by rustc rather than by review.
///
/// This function does not run anything; it exists so that deleting an
/// `inert!` entry fails to compile here as well as in `src/inert.rs`.
fn assert_inert<T: Inert>() {}

#[test]
fn tc_383_040_every_auditor_input_is_inert() {
    assert_inert::<AuditInput>();
    assert_inert::<ObligationFacts>();
    assert_inert::<MethodCatalog>();
}

/// Tokens that reach the world. None may appear in the auditor or the advisor.
const EXECUTION_TOKENS: &[&str] = &[
    "std::process",
    "Command::new",
    "std::fs",
    "std::net",
    "File::open",
    "read_to_string",
    "std::env",
];

/// The trees that must be free of them.
const READING_ONLY: &[&str] = &["audit", "advise"];

/// The one module that is allowed them — the filesystem seam itself.
const THE_SEAM: &str = "catalog/source.rs";

/// Every `.rs` file under `src/`, as `(relative path, contents)`.
fn sources() -> Vec<(String, String)> {
    fn walk(root: &Path, directory: &Path, into: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(directory).expect("src/ is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(root, &path, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let name = path
                    .strip_prefix(root)
                    .expect("a path under src/")
                    .to_string_lossy()
                    .replace('\\', "/");
                let text = std::fs::read_to_string(&path).expect("a source file");
                into.push((name, text));
            }
        }
    }
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    walk(&root, &root, &mut found);
    found.sort();
    found
}

#[test]
fn tc_383_041_the_auditor_and_the_advisor_name_nothing_that_reaches_the_world() {
    let sources = sources();
    assert!(
        sources.len() >= 18,
        "anti-vacuity floor: at least 18 modules expected, saw {} — a census that \
         finds nothing proves nothing",
        sources.len()
    );

    // Positive control. If the scanner has stopped working, it stops here
    // rather than reporting a clean auditor it never looked at.
    let seam = sources
        .iter()
        .find(|(name, _)| name == THE_SEAM)
        .unwrap_or_else(|| panic!("{THE_SEAM} is the filesystem seam and must exist"));
    let found_in_seam: Vec<&str> = EXECUTION_TOKENS
        .iter()
        .copied()
        .filter(|token| seam.1.contains(token))
        .collect();
    assert!(
        found_in_seam.len() >= 3,
        "the scanner must find at least three world-reaching tokens in {THE_SEAM}, \
         which is where they belong; saw {found_in_seam:?}"
    );

    let mut scanned = 0usize;
    for (name, text) in &sources {
        if !READING_ONLY
            .iter()
            .any(|tree| name.starts_with(&format!("{tree}/")))
        {
            continue;
        }
        scanned += 1;
        for token in EXECUTION_TOKENS {
            // Skipping `//`-prefixed lines would let a comment hide a call; the
            // check is deliberately over the whole text, and the prose in this
            // crate spells these names with backticks and paths rather than as
            // Rust tokens.
            assert!(
                !text.contains(token),
                "{name} names `{token}`. ADR-0011 invariant 1: the auditor reads the \
                 store and reports. If a new input must come from the world, it goes \
                 behind `ModuleCatalogSource` and is handed in — it does not get read \
                 here."
            );
        }
    }
    assert!(
        scanned >= 12,
        "anti-vacuity floor: at least 12 auditor and advisor modules expected, \
         saw {scanned}"
    );
}

#[test]
fn tc_383_042_the_inert_census_still_names_every_input() {
    // `src/inert.rs` carries the `inert!` list and a `const _` that forces each
    // entry through the bound. An entry deleted there would leave the type
    // unproved while `assert_inert` above still compiled for the others, so the
    // census itself is asserted to still name all three.
    let sources = sources();
    let census = sources
        .iter()
        .find(|(name, _)| name == "inert.rs")
        .expect("src/inert.rs exists");
    for input in [
        "crate::audit::AuditInput",
        "crate::advise::ObligationFacts",
        "crate::catalog::MethodCatalog",
    ] {
        assert!(
            census.1.contains(input),
            "{input} is an auditor input and must be in the `inert!` census"
        );
    }
    assert!(
        census.1.contains("serde::de::DeserializeOwned"),
        "the supertrait IS the proof: without it `Inert` is a promise rather \
         than a bound"
    );
}
