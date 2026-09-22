// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! This crate owns no RFC 3339 grammar, no digest and no schema file read.
//!
//! # Why a census over the crate's own sources
//!
//! The Stage 6 plan §13.1 counted six hand-rolled instant validators across
//! three crates, accepting three different languages, and §5's whole ruling
//! exists because the measurement domain carried two more. The one grammar is
//! `quoin_measurement::date_time`, and this crate sits *below* it: the
//! `date-time` check is a parameter of `MeasurementSchema::compile` precisely so
//! that a second one is never written here.
//!
//! A rule enforced only by review is enforced only when someone is looking, so
//! it is read off the crate's own `src/` tree instead. The forbidden tokens are
//! spelled with a concatenation so that this file does not itself contain the
//! strings it forbids.
//!
//! Trace: FR-098

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

/// The count below which this census is not measuring the crate at all.
const SOURCE_FLOOR: usize = 5;

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

/// No second instant grammar, hasher or canonicalizer grew here.
///
/// Trace: FR-098
/// Provenance: quoin#470
#[test]
fn tc_470_no_second_grammar_hasher_or_canonicalizer_appeared() {
    // (token, what owns it instead)
    let forbidden: [(String, &str); 7] = [
        (
            ["days_from_", "civil"].concat(),
            "quoin_measurement::date_time",
        ),
        (
            ["is_leap_", "year"].concat(),
            "quoin_measurement::date_time",
        ),
        (["RFC", "3339"].concat(), "quoin_measurement::date_time"),
        (["sha", "2::"].concat(), "quoin_store::digest_file_sha256"),
        (["blake", "3::"].concat(), "quoin_store::digest_raw_bytes"),
        (
            ["serde_json::to_", "string"].concat(),
            "quoin_store::canonical_json",
        ),
        (["serde_", "yaml"].concat(), "quoin_yaml::from_str"),
    ];
    for (name, text) in sources() {
        for (token, owner) in &forbidden {
            assert!(
                !text.contains(token.as_str()),
                "{name} names `{token}`; that belongs to {owner} and this crate may not have a \
                 second one (Stage 6 plan §13.1/§13.2)"
            );
        }
    }
}

/// The vendored schemas arrive through `include_str!`, never `std::fs`.
///
/// A schema read off disk at run time is a schema that can differ from the one
/// the tests measured.
///
/// Trace: FR-098
/// Provenance: quoin#470
#[test]
fn tc_470_no_module_reads_a_schema_from_the_filesystem() {
    let including: Vec<String> = sources()
        .into_iter()
        .filter(|(_, text)| text.contains(&["include_", "str!"].concat()))
        .map(|(name, _)| name)
        .collect();
    assert_eq!(
        including,
        vec!["measurement_schemas.rs".to_owned()],
        "exactly one module may embed the vendored documents"
    );
    for (name, text) in sources() {
        assert!(
            !text.contains(&["std::fs", "::read"].concat()),
            "{name} reads the filesystem; the vendored schemas are embedded and this crate has \
             no other file to read"
        );
    }
}

/// The manifest takes on nothing that would duplicate what the workspace owns.
///
/// In particular there is no build-time dependency on `quoin-measurement`: the
/// dependency runs the other way, which is why the `date-time` check is passed
/// in rather than looked up.
///
/// Trace: FR-098
/// Provenance: quoin#470
#[test]
fn tc_470_the_manifest_holds_no_grammar_and_no_cycle() {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("the crate manifest is readable");
    let (dependencies, dev_dependencies) = manifest
        .split_once("[dev-dependencies]")
        .expect("the manifest declares dev-dependencies");
    for token in [
        ["chrono", " ="].concat(),
        ["time", " ="].concat(),
        ["sha", "2 ="].concat(),
        ["blake", "3 ="].concat(),
        ["serde_", "yaml ="].concat(),
        ["quoin-measurement", " ="].concat(),
    ] {
        assert!(
            !dependencies.contains(token.as_str()),
            "the crate manifest declares `{token}` as a build dependency; this crate sits below \
             quoin-measurement and owns no instant, digest or YAML implementation"
        );
    }
    assert!(
        dev_dependencies.contains("quoin-measurement"),
        "the tests must wire the real RFC 3339 grammar, not a second one written for them"
    );
}
