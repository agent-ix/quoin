// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A retained evidence file has exactly two representations in this crate.
//!
//! # The concept, and how many types it needs
//!
//! A pointer at a retained evidence file is one concept at two trust levels:
//! **as recorded** — read by serde off a stored record, unchecked — and **as
//! verified** — minted from a path that passed the lexical guards and a digest
//! `quoin_store` took itself. Both are needed, because intake's whole job is
//! comparing the first against the second.
//!
//! A third is not. quoin#468 landed one anyway (`RawEvidenceClaim`, bare
//! `String`s) because quoin#469's serde-read type did not exist yet, so for two
//! waves the crate carried the same idea three ways and every intake converted
//! between them. quoin#472 deleted the claim; this census is what stops a
//! fourth wave from adding it back under another name.
//!
//! # Why a source census and not a review note
//!
//! "Do not add a third" enforced by review is enforced only when someone is
//! looking. So the crate's own `src/` tree is read here: every `struct`
//! declaring all four members a retained-evidence pointer carries — a path, a
//! media type, a size in bytes and a digest — is found by name, and the set
//! must be exactly the two below.
//!
//! # The anti-vacuity floor
//!
//! A scanner that matched nothing would pass. So the file count has a floor and
//! the found set is compared by name, not by size: a census that found two
//! *different* types would be as wrong as one that found three.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

/// The two representations, and the only two.
const REPRESENTATIONS: [&str; 2] = ["RawEvidenceReference", "RecordedEvidenceReference"];

/// The members that together make a declaration a retained-evidence pointer.
///
/// All four, because two of them alone describe something else:
/// `RawEvidenceFile` carries a size and a digest and is a *file*, not a
/// reference to one.
const MEMBERS: [&str; 4] = ["path:", "media_type:", "size_bytes:", "digest:"];

/// The count below which this census is not reading the crate at all.
const SOURCE_FLOOR: usize = 20;

fn sources() -> Vec<(String, String)> {
    fn walk(root: &Path, directory: &Path, into: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(directory).expect("src/ is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(root, &path, into);
            } else if path.extension().is_some_and(|end| end == "rs") {
                let name = path
                    .strip_prefix(root)
                    .expect("a path under src/")
                    .to_string_lossy()
                    .replace('\\', "/");
                into.push((name, std::fs::read_to_string(&path).expect("a source file")));
            }
        }
    }
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    walk(&root, &root, &mut found);
    found.sort();
    found
}

/// Every `struct` declaration in `text`, as `(name, body)`.
fn declarations(text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(offset) = rest.find("struct ") {
        let after = &rest[offset + "struct ".len()..];
        let name: String = after
            .chars()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        let body = after
            .find('{')
            .and_then(|open| {
                after[open..]
                    .find('}')
                    .map(|close| &after[open..open + close])
            })
            .unwrap_or("");
        if !name.is_empty() {
            found.push((name, body.to_owned()));
        }
        rest = after;
    }
    found
}

/// Trace: FR-100-AC-4, NFR-025-AC-3
/// Provenance: quoin#472
#[test]
fn tc_472_a_retained_evidence_file_has_exactly_two_representations() {
    let sources = sources();
    assert!(
        sources.len() >= SOURCE_FLOOR,
        "anti-vacuity floor: at least {SOURCE_FLOOR} modules expected, saw {} — a census that \
         reads nothing proves nothing",
        sources.len()
    );

    let mut found: Vec<String> = Vec::new();
    for (module, text) in &sources {
        for (name, body) in declarations(text) {
            if MEMBERS.iter().all(|member| body.contains(member)) {
                found.push(format!("{name} ({module})"));
            }
        }
    }
    found.sort();

    let named: Vec<String> = found
        .iter()
        .map(|entry| {
            entry
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    assert_eq!(
        named, REPRESENTATIONS,
        "a retained evidence file is one concept at two trust levels — as recorded and as \
         verified — and quoin#472 deleted the third spelling quoin#468 had left behind. The \
         declarations found were {found:?}. If a new one is genuinely a different concept, say \
         which trust level it adds and why the two above cannot carry it; do not delete this \
         test."
    );
}

/// The deleted third spelling has not come back under its own name.
///
/// Trace: FR-100-AC-4
/// Provenance: quoin#472
#[test]
fn tc_472_the_deleted_claim_type_is_still_deleted() {
    let sources = sources();
    assert!(sources.len() >= SOURCE_FLOOR);
    for (module, text) in &sources {
        // Comments may name it — the module headers explain the deletion, and
        // this very test file is not scanned. Code may not.
        let code = text
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect::<Vec<&str>>()
            .join("\n");
        assert!(
            !code.contains("RawEvidenceClaim"),
            "{module} declares or uses RawEvidenceClaim, which quoin#472 deleted"
        );
    }
}
