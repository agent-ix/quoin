// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What this crate is not allowed to contain, asserted over its own sources.
//!
//! # Why a source census rather than a behavioural test
//!
//! Three of the constraints ported here are about *absence*: no second
//! canonicalizer, no host capability outside one module, and no vocabulary
//! that would overstate what a digest proves. Absence cannot be observed by
//! calling a function — a second blake3 implementation used on one rare path
//! would pass every behavioural test in this crate. So these read the crate's
//! own `src/` tree, and each carries a floor so that a census over an empty
//! or mis-rooted population fails instead of passing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_change_assurance::intake::INTEGRITY_BOUNDARY;

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
                into.push((name, std::fs::read_to_string(&path).expect("a source file")));
            }
        }
    }
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&root, &root, &mut files);
    files.sort();
    files
}

/// The lowercase words of one line, hyphenated compounds kept whole.
fn words(line: &str) -> Vec<String> {
    line.split(|character: char| !character.is_ascii_alphabetic() && character != '-')
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Trace: FR-100-AC-8
///
/// The library half names no host capability. `std::fs`, the process id, the
/// clock, the environment and the network appear in exactly one module —
/// `intake/disk.rs`, whose whole purpose is to be that module — and nowhere
/// else, so every other analysis in this crate is a pure function of the
/// evidence it was handed.
#[test]
fn tc_455_only_the_disk_store_names_a_host_capability() {
    let sources = sources();
    assert!(
        sources.len() >= 14,
        "anti-vacuity floor: at least 14 source files, saw {}",
        sources.len()
    );
    let capabilities = [
        "std::fs",
        "std::process",
        "std::env",
        "std::net",
        "std::time",
        "Command::new",
    ];
    let mut named = 0_usize;
    for (name, contents) in &sources {
        for capability in capabilities {
            if contents.contains(capability) {
                assert_eq!(
                    name, "intake/disk.rs",
                    "`{capability}` appears in {name}, which is not the one module allowed to \
                     name a host capability"
                );
                named += 1;
            }
        }
    }
    assert!(
        named >= 2,
        "anti-vacuity floor: the disk store must actually name a capability, saw {named}"
    );
}

/// Trace: FR-100-AC-8
/// Provenance: quoin#503
///
/// `include_str!` is bounded too, but to a different module and for a
/// different reason. It is not a host capability: it reads nothing at run
/// time, so a module using it is still a pure function of what it was handed.
/// What it *is* is the one way a file enters the binary, and the crate's
/// answer to "which file, and asserted how" has to stay a single answer —
/// `schemas.rs`, whose three assets `tests/tc_503_schema_assets.rs` validates
/// sealed documents against. A second module embedding a second file would be
/// a second contract with no test pointing at it.
#[test]
fn tc_503_only_the_asset_module_embeds_a_file() {
    let sources = sources();
    let mut embedded = 0_usize;
    for (name, contents) in &sources {
        if contents.contains("include_str!") {
            assert_eq!(
                name, "schemas.rs",
                "`include_str!` appears in {name}, which is not the one module allowed to embed \
                 a file"
            );
            embedded += 1;
        }
    }
    assert_eq!(
        embedded, 1,
        "anti-vacuity floor: the asset module must actually embed its assets, saw {embedded}"
    );
}

/// Trace: FR-100-CON-4
///
/// There is one canonicalizer and one record digest in this workspace, and
/// they live in `quoin-store`. This crate declares exactly three dependencies:
/// the store, `sha2` for the single ix-flow event hash the store does not
/// expose, and `thiserror`. Nothing here serializes JSON or hashes a record on
/// its own, and the manifest is asserted so that adding a second
/// implementation cannot be done quietly.
#[test]
fn tc_455_no_second_canonicalization_or_digest_implementation_exists_here() {
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("the manifest is readable");
    let declared: Vec<String> = manifest
        .split("[dependencies]")
        .nth(1)
        .expect("the manifest declares dependencies")
        .split("[dev-dependencies]")
        .next()
        .expect("the dependency section ends")
        .lines()
        .filter(|line| !line.trim_start().starts_with('#') && line.contains('='))
        .map(|line| line.split('=').next().unwrap_or_default().trim().to_owned())
        .collect();
    assert_eq!(
        declared,
        vec![
            "quoin-store".to_owned(),
            "sha2".to_owned(),
            "thiserror".to_owned()
        ],
        "the dependency set is part of the constraint: a second JSON or digest crate here \
         would be a second implementation"
    );

    let mut hashing = Vec::new();
    for (name, contents) in sources() {
        if contents.contains("sha2::") || contents.contains("Sha256") {
            hashing.push(name.clone());
        }
        assert!(
            !contents.contains("blake3::") && !contents.contains("serde_json"),
            "{name} reaches for its own digest or JSON implementation"
        );
    }
    assert_eq!(
        hashing,
        vec!["model/decision.rs".to_owned()],
        "the one sha256 in this crate is the ix-flow event hash, and it has one home"
    );
}

/// Trace: FR-063-AC-11
///
/// The golden-terminology check. A digest here is content integrity and
/// recorded attribution; it is never authentication, authorization, a
/// signature or non-repudiation. Every place this crate's prose reaches for
/// one of those words, it is to deny it — and the census floor means a crate
/// that simply stopped saying anything about the boundary would fail this
/// rather than pass it.
#[test]
fn tc_455_the_integrity_boundary_is_stated_and_never_overstated() {
    assert_eq!(
        INTEGRITY_BOUNDARY, "content integrity and recorded attribution only",
        "the boundary is retained verbatim from the TypeScript"
    );

    let claims = [
        "authentication",
        "authenticate",
        "authenticated",
        "authenticity",
        "authorization",
        "authorize",
        "authorized",
        "non-repudiation",
        "nonrepudiation",
        "signature",
        "signatures",
        "signing",
        "signed",
    ];
    let denials = ["not", "never", "no", "nor", "rather", "without"];
    let mut occurrences = 0_usize;
    for (name, contents) in sources() {
        let lines: Vec<&str> = contents.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let spoken = words(line);
            if !spoken.iter().any(|word| claims.contains(&word.as_str())) {
                continue;
            }
            occurrences += 1;
            let window = lines[index.saturating_sub(2)..lines.len().min(index + 2)].join(" ");
            let around = words(&window);
            assert!(
                around.iter().any(|word| denials.contains(&word.as_str())),
                "{name}:{}: a claim word is used without denying it: {line}",
                index + 1
            );
        }
    }
    assert!(
        occurrences >= 4,
        "anti-vacuity floor: the boundary must actually be stated in prose, saw {occurrences} \
         places naming a claim this family does not make"
    );
}

/// Trace: FR-064-AC-9
///
/// Intake runs nothing. No module in this crate spawns a process, opens a
/// socket, or reads the environment, so an attestation's contents can only
/// have come from the producer that wrote it — which is the whole reason a
/// recorded result is evidence rather than a rerun.
#[test]
fn tc_455_intake_runs_nothing_and_reports_only_what_a_producer_wrote() {
    let forbidden = [
        "std::process::Command",
        "Command::new",
        "TcpStream",
        "reqwest",
        "std::env::var",
    ];
    let mut scanned = 0_usize;
    for (name, contents) in sources() {
        for pattern in forbidden {
            assert!(
                !contents.contains(pattern),
                "{name} reaches for `{pattern}`; intake runs nothing"
            );
        }
        scanned += 1;
    }
    assert!(
        scanned >= 14,
        "anti-vacuity floor: at least 14 source files scanned, saw {scanned}"
    );
}
