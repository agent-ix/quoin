// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! There is one canonical-JSON writer in this crate, and this crate does not
//! own it.
//!
//! FR-100-CON-4 and the ticket both say the same thing: the canonical bytes
//! are `quoin_store::canonical_json_bytes`' and the sort key is
//! `quoin_store::canonicalize_jcs`', called and never reimplemented. A second
//! implementation would not announce itself — it would appear as a
//! `to_string_pretty` with a two-space indent, or a hand-rolled member sort in
//! whichever module needed bytes that week, and it would agree with the first
//! one until the day it did not.
//!
//! So the source tree is read here and two facts are asserted about it:
//!
//! 1. every call into the store's canonicalizers is made from `src/json.rs`,
//!    the one module whose job that is;
//! 2. no module reaches for a JSON writer or a member sort of its own.
//!
//! A check like this is only worth its line count if it can fail, so it
//! carries a floor: if it stops finding modules, or stops finding the calls it
//! is guarding, it fails rather than passing on an empty tree.
//!
//! Trace: FR-100-CON-4
//!
//! Provenance: quoin#385

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

/// The one module allowed to call the store's canonicalizers.
const THE_ONE_MODULE: &str = "json.rs";

/// The store entry points that produce canonical bytes.
const CANONICALIZERS: &[&str] = &["canonical_json", "canonical_json_bytes", "canonicalize_jcs"];

/// Spellings that would mean a second writer had appeared.
///
/// `(fragment, what it would be)`.
const SECOND_WRITER_SPELLINGS: &[(&str, &str)] = &[
    (
        "to_string_pretty",
        "serde_json's pretty printer, which is not the canonical form",
    ),
    (
        "to_vec_pretty",
        "serde_json's pretty printer, which is not the canonical form",
    ),
    (
        "PrettyFormatter",
        "a hand-configured serde_json formatter, which is a second canonical form",
    ),
    (
        "sort_keys",
        "a member sort outside the canonicalizer that owns member order",
    ),
];

/// The count below which this check is guarding nothing.
const SOURCE_FLOOR: usize = 10;

/// Every `.rs` file under `src/`, as `(file name, relative path, contents)`.
fn modules() -> Vec<(String, String, String)> {
    fn walk(root: &Path, directory: &Path, into: &mut Vec<(String, String, String)>) {
        for entry in std::fs::read_dir(directory).expect("src/ is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(root, &path, into);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let name = path
                    .file_name()
                    .expect("a file name")
                    .to_string_lossy()
                    .into_owned();
                let relative = path
                    .strip_prefix(root)
                    .expect("a path under src/")
                    .to_string_lossy()
                    .replace('\\', "/");
                let contents = std::fs::read_to_string(&path).expect("a source file");
                into.push((name, relative, contents));
            }
        }
    }
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    walk(&root, &root, &mut found);
    found.sort();
    assert!(
        found.len() >= SOURCE_FLOOR,
        "anti-vacuity floor: at least {SOURCE_FLOOR} modules expected, saw {}",
        found.len()
    );
    found
}

/// The store's canonicalizers are called from exactly one module.
///
/// Trace: FR-100-CON-4
#[test]
fn tc_385_020_the_canonicalizers_are_called_from_one_module() {
    let mut calls = 0usize;
    for (name, relative, contents) in modules() {
        for line in contents.lines() {
            // The doc comments name these functions constantly, and naming one
            // is not calling it.
            if line.trim_start().starts_with("//") {
                continue;
            }
            for canonicalizer in CANONICALIZERS {
                if !line.contains(&format!("quoin_store::{canonicalizer}(")) {
                    continue;
                }
                assert_eq!(
                    name, THE_ONE_MODULE,
                    "{relative} calls quoin_store::{canonicalizer}. Canonical bytes leave this \
                     crate through {THE_ONE_MODULE} and nowhere else, so that there is one \
                     answer to what they are."
                );
                calls += 1;
            }
        }
    }
    // Two, not three: the crate writes canonical bytes with
    // `canonical_json_bytes` and sort keys with `canonicalize_jcs`, and
    // `canonical_json` is guarded here so that a third route cannot open
    // quietly, not because it is called.
    assert!(
        calls >= 2,
        "only {calls} canonicalizer calls found: this check is no longer guarding the calls it \
         was written for"
    );
}

/// No module writes JSON, or orders members, on its own.
///
/// Trace: FR-100-CON-4
#[test]
fn tc_385_021_no_module_grows_a_second_canonical_form() {
    for (_, relative, contents) in modules() {
        for (index, line) in contents.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for (fragment, what) in SECOND_WRITER_SPELLINGS {
                assert!(
                    !line.contains(fragment),
                    "{relative}:{}: {fragment} is {what}. The canonical form is \
                     quoin_store::canonical_json's; call it.",
                    index + 1
                );
            }
        }
    }
}
