// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! This crate owns no canonicalizer, no digest, no YAML reader, and exactly
//! one instant grammar.
//!
//! # Why a census over the crate's own sources
//!
//! FR-100-CON-4 and the Stage 6 plan §13.2 forbid a second implementation of
//! anything `quoin-store` already owns, and quoin#463/#464 exist because that
//! happened before. A rule enforced only by review is enforced only when
//! someone is looking, so it is read off the crate's own `src/` tree here.
//!
//! The forbidden tokens are spelled with a concatenation so that this file does
//! not itself contain the strings it forbids.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

/// The count below which this census is not measuring the crate at all.
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
    assert!(
        found.len() >= SOURCE_FLOOR,
        "anti-vacuity floor: at least {SOURCE_FLOOR} modules expected, saw {} — a census that \
         finds nothing proves nothing",
        found.len()
    );
    found
}

/// Trace: FR-100-AC-4, NFR-025-AC-3
/// Provenance: quoin#468
#[test]
fn tc_468_no_second_hasher_serializer_or_yaml_reader_appeared() {
    // (token, what owns it instead)
    let forbidden: [(String, &str); 6] = [
        (["sha", "2::"].concat(), "quoin_store::digest_file_sha256"),
        (["Sha", "256::"].concat(), "quoin_store::digest_file_sha256"),
        (["blake", "3"].concat(), "quoin_store::digest_raw_bytes"),
        (
            ["serde_json::to_", "string"].concat(),
            "quoin_store::canonical_json",
        ),
        (
            ["serde_json::to_", "vec"].concat(),
            "quoin_store::canonical_json_bytes",
        ),
        (["serde_", "yaml"].concat(), "quoin_yaml::from_str"),
    ];
    for (name, text) in sources() {
        for (token, owner) in &forbidden {
            assert!(
                !text.contains(token.as_str()),
                "{name} names `{token}`; that belongs to {owner} and this crate may not have a \
                 second one (FR-100-CON-4)"
            );
        }
    }
}

/// Trace: FR-100-AC-2, NFR-025-AC-3
/// Provenance: quoin#468
#[test]
fn tc_468_exactly_one_module_reads_an_instant() {
    // The civil-date arithmetic an instant reader needs. If a second module
    // ever grows one, this is where it shows up — the Stage 6 plan §13.1
    // counted six hand-rolled instant validators across the tree already, and
    // this crate contributes one, not two.
    let with_calendar: Vec<String> = sources()
        .into_iter()
        .filter(|(_, text)| text.contains("days_from_civil") || text.contains("is_leap_year"))
        .map(|(name, _)| name)
        .collect();
    assert_eq!(with_calendar, vec!["date_time.rs".to_owned()]);
}

/// Trace: FR-100-AC-4
/// Provenance: quoin#468
#[test]
fn tc_468_the_manifest_depends_on_nothing_that_would_duplicate_the_store() {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("the crate manifest is readable");
    for token in [
        ["sha", "2 ="].concat(),
        ["blake", "3 ="].concat(),
        ["serde_", "yaml ="].concat(),
        ["chrono", " ="].concat(),
        ["time", " ="].concat(),
        ["regex", " ="].concat(),
    ] {
        assert!(
            !manifest.contains(token.as_str()),
            "the manifest depends on `{token}`; the capability it brings is already \
             quoin-store's, quoin-yaml's, or this crate's own date_time module"
        );
    }
}
