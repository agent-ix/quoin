// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! What this crate is not allowed to contain, asserted over its own sources.
//!
//! # Why a source census rather than a behavioural test
//!
//! Three of the constraints ported here are about *absence*: no second
//! canonicalizer, no host capability outside one module, and no vocabulary
//! that would overstate what retained evidence proves. Absence cannot be
//! observed by calling a function - a second canonicalizer used on one rare
//! path would pass every behavioural test in this crate. So these read the crate's
//! own `src/` tree, and each is a source census.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
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

/// The wire identifiers that name a member and are not a claim of their own.
/// Everything else in prose stays subject to the census.
const WIRE_IDENTIFIERS: [&str; 5] = [
    "signed_approval",
    "signed_at",
    "signature_kinds",
    "signature_kind",
    "signed_bad",
];

/// The lowercase words of one line, hyphenated compounds kept whole, with the
/// [`WIRE_IDENTIFIERS`] removed first.
fn words(line: &str) -> Vec<String> {
    let mut line = line.to_owned();
    for identifier in WIRE_IDENTIFIERS {
        line = line.replace(identifier, " ");
    }
    line.split(|character: char| !character.is_ascii_alphabetic() && character != '-')
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// The census still fails on prose that uses a claim word, even beside an
/// allowlisted identifier.
#[trace("FR-063-AC-11")]
#[test]
fn tc_1511_the_claim_word_census_stays_strict_for_prose() {
    assert!(!words("the `signed_approval` policy").is_empty());
    assert!(!words("the `signed_approval` policy").contains(&"signed".to_owned()));
    assert!(words("this record was signed by a person").contains(&"signed".to_owned()));
}

/// The library half names no host capability. `std::fs`, the process id, the
/// clock, the environment and the network appear in exactly one module —
/// `intake/disk.rs`, whose whole purpose is to be that module — and nowhere
/// else, so every other analysis in this crate is a pure function of the
/// evidence it was handed.
#[trace("FR-100-AC-8")]
#[test]
fn tc_455_only_the_disk_store_names_a_host_capability() {
    let sources = sources();
    let capabilities = [
        "std::fs",
        "std::process",
        "std::env",
        "std::net",
        "std::time",
        "Command::new",
    ];
    for (name, contents) in &sources {
        for capability in capabilities {
            if contents.contains(capability) {
                assert_eq!(
                    name, "intake/disk.rs",
                    "`{capability}` appears in {name}, which is not the one module allowed to \
                     name a host capability"
                );
            }
        }
    }
}

/// Provenance: quoin#503
///
/// `include_str!` is bounded too, but to a different module and for a
/// different reason. It is not a host capability: it reads nothing at run
/// time, so a module using it is still a pure function of what it was handed.
/// What it *is* is the one way a file enters the binary, and the crate's
/// answer to "which file, and asserted how" has to stay a single answer —
/// `schemas.rs`, whose three assets `tests/tc_503_schema_assets.rs` validates
/// written documents against. A second module embedding a second file would be
/// a second contract with no test pointing at it.
#[trace("FR-100-AC-8")]
#[test]
fn tc_503_only_the_asset_module_embeds_a_file() {
    let sources = sources();
    for (name, contents) in &sources {
        if contents.contains("include_str!") {
            assert_eq!(
                name, "schemas.rs",
                "`include_str!` appears in {name}, which is not the one module allowed to embed \
                 a file"
            );
        }
    }
}

/// There is one canonicalizer in this workspace, and it lives in
/// `quoin-store`. This crate declares exactly three dependencies: the store,
/// `thiserror`, and `engineering-assurance` (PLAT-964) for the
/// `ProtectedApparatus`/`NegativeControls` types and entry grammar FR-024
/// owns and (PLAT-972) the FR-022 claim-strength vocabulary - domain types,
/// not a JSON implementation. Nothing here serializes JSON on its own, and the
/// manifest is asserted so that adding a second implementation cannot be done
/// quietly.
#[trace("FR-100-CON-4")]
#[test]
fn tc_455_no_second_canonicalization_implementation_exists_here() {
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
            "thiserror".to_owned(),
            "engineering-assurance".to_owned()
        ],
        "the dependency set is part of the constraint: a second JSON crate here would be a \
         second implementation"
    );

    for (name, contents) in sources() {
        assert!(
            !contents.contains("serde_json"),
            "{name} reaches for its own JSON implementation"
        );
    }
}

/// The golden-terminology check. A record here is retained content and
/// recorded attribution; it is never authentication, authorization, a
/// signature or non-repudiation. Every place this crate's prose reaches for
/// one of those words, it is to deny it.
#[trace("FR-063-AC-11", "FR-065-AC-12")]
#[test]
fn tc_455_the_integrity_boundary_is_stated_and_never_overstated() {
    assert_eq!(
        INTEGRITY_BOUNDARY, "retained content and recorded attribution only",
        "the boundary is stated verbatim"
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
    for (name, contents) in sources() {
        let lines: Vec<&str> = contents.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let spoken = words(line);
            if !spoken.iter().any(|word| claims.contains(&word.as_str())) {
                continue;
            }
            let window = lines[index.saturating_sub(2)..lines.len().min(index + 2)].join(" ");
            let around = words(&window);
            assert!(
                around.iter().any(|word| denials.contains(&word.as_str())),
                "{name}:{}: a claim word is used without denying it: {line}",
                index + 1
            );
        }
    }
}

/// Intake runs nothing. No module in this crate spawns a process, opens a
/// socket, or reads the environment, so an attestation's contents can only
/// have come from the producer that wrote it — which is the whole reason a
/// recorded result is evidence rather than a rerun.
#[trace("FR-064-AC-9", "FR-065-AC-18")]
#[test]
fn tc_455_intake_runs_nothing_and_reports_only_what_a_producer_wrote() {
    let forbidden = [
        "std::process::Command",
        "Command::new",
        "TcpStream",
        "reqwest",
        "std::env::var",
    ];
    for (name, contents) in sources() {
        for pattern in forbidden {
            assert!(
                !contents.contains(pattern),
                "{name} reaches for `{pattern}`; intake runs nothing"
            );
        }
    }
}

/// The approval check is a comparison of strings and a local hash. The modules
/// that read and judge a claimed approval name no `std` item at all, so they
/// cannot start a process, touch a file, or open a socket; the digest is the one
/// computation they ask of `quoin-store`, and verification passes it in.
#[trace("FR-065-AC-18", "FR-063-AC-11")]
#[test]
fn tc_1511_the_approval_modules_start_no_process_and_name_no_host_capability() {
    let approval_modules = ["model/approval.rs", "verify/approval.rs"];
    let sources = sources();
    for module in approval_modules {
        let (_, contents) = sources
            .iter()
            .find(|(name, _)| name == module)
            .unwrap_or_else(|| panic!("{module} is part of this crate"));
        for forbidden in ["std::", "Command", "git ", "ssh-keygen", "TcpStream"] {
            assert!(
                !contents.contains(forbidden),
                "{module} reaches for `{forbidden}`; the claim is compared, never verified"
            );
        }
    }
}
