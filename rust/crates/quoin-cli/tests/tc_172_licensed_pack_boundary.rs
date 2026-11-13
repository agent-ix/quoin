// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `spec-security-analysis` skill serves licence-restricted sources only from
//! a locally installed pack, by clause id (FR-172).
//!
//! A live skill run over a synthetic pack is a demonstration scenario
//! (`TC-EV-061`, `TC-EV-062` in `spec/evals.md`) and is not claimed here. These
//! tests check what is deterministic: what the skill ships and the fixtures hold,
//! and how the engine lists the synthetic pack fixture.

mod pack_support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use pack_support::{SENTINEL, quoin, scratch, semantic_root, text};

const SKILL: &str = "skills/spec-security-analysis";
const FIXTURES: &str = "tests/fixtures/security-analysis";
const SAFE_REFERENCE_PATHS: &[&str] = &[
    "component-type-mapping.md",
    "gdpr",
    "gdpr/ChII-Principles.md",
    "gdpr/ChIII-DataSubjectRights.md",
    "gdpr/ChIV-ControllerProcessor.md",
    "gdpr/ChV-DataTransfers.md",
    "gdpr/index.md",
    "nist-800-53",
    "nist-800-53/AC-AccessControl.md",
    "nist-800-53/AU-AuditAccountability.md",
    "nist-800-53/CM-ConfigurationManagement.md",
    "nist-800-53/IA-IdentificationAuthentication.md",
    "nist-800-53/SA-SystemServicesAcquisition.md",
    "nist-800-53/SC-SystemCommunications.md",
    "nist-800-53/SI-SystemInformationIntegrity.md",
    "nist-800-53/index.md",
    "owasp-asvs",
    "owasp-asvs/V1-EncodingSanitization.md",
    "owasp-asvs/V2-ValidationBusinessLogic.md",
    "owasp-asvs/V3-WebFrontend.md",
    "owasp-asvs/V4-APIWebService.md",
    "owasp-asvs/V5-FileHandling.md",
    "owasp-asvs/V6-Authentication.md",
    "owasp-asvs/V7-SessionManagement.md",
    "owasp-asvs/V8-Authorization.md",
    "owasp-asvs/V9-Tokens.md",
    "owasp-asvs/V10-OAuthOIDC.md",
    "owasp-asvs/V11-Cryptography.md",
    "owasp-asvs/V12-SecureCommunication.md",
    "owasp-asvs/V13-Configuration.md",
    "owasp-asvs/V14-DataProtection.md",
    "owasp-asvs/V15-SecureCodingArchitecture.md",
    "owasp-asvs/V16-LoggingErrorHandling.md",
    "owasp-asvs/V17-WebRTC.md",
    "owasp-asvs/index.md",
    "standards-index.md",
];

fn repo_root() -> PathBuf {
    std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.."))
        .expect("repository root")
}

fn scan_root() -> PathBuf {
    std::env::var_os("QUOIN_LICENSE_SCAN_ROOT").map_or_else(repo_root, PathBuf::from)
}

/// Every file and directory path under `dir`; an unreadable entry panics, so a
/// negative scan cannot pass on an I/O failure.
fn paths_under(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry
            .unwrap_or_else(|e| panic!("entry of {}: {e}", dir.display()))
            .path();
        found.push(path.clone());
        if std::fs::metadata(&path)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
            .is_dir()
        {
            paths_under(&path, found);
        }
    }
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut all = Vec::new();
    paths_under(dir, &mut all);
    let files: Vec<_> = all
        .into_iter()
        .filter(|path| {
            std::fs::metadata(path)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
                .is_file()
        })
        .collect();
    assert!(!files.is_empty(), "{} holds no file", dir.display());
    files
}

fn read(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    String::from_utf8(bytes).unwrap_or_else(|e| panic!("{} is not UTF-8: {e}", path.display()))
}

fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// Identifier shapes used by the removed licence-restricted reference tables.
fn is_restricted_id(word: &str) -> bool {
    let word = word.to_ascii_uppercase();
    if let Some(rest) = word.strip_prefix("CC") {
        let parts: Vec<&str> = rest.split('.').collect();
        return parts.len() >= 2 && parts.iter().all(|p| digits(p));
    }
    if let Some(rest) = word.strip_prefix("A.") {
        let parts: Vec<&str> = rest.split('.').collect();
        return parts.len() >= 2 && parts.iter().all(|p| digits(p));
    }
    false
}

/// The restricted-source identifiers `text` carries.
fn restricted_ids(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.'))
        .map(|w| w.trim_matches('.'))
        .filter(|w| is_restricted_id(w))
        .map(str::to_ascii_uppercase)
        .collect()
}

fn reference_path(candidate: &str) -> Option<PathBuf> {
    let candidate = candidate.replace('\\', "/");
    let (_, path) = candidate.rsplit_once("references/")?;
    let path = path
        .split(['?', '#'])
        .next()
        .unwrap_or(path)
        .trim_end_matches(['.', ',', ';', ':']);
    Some(PathBuf::from(path))
}

/// Find local reference paths in Markdown links, inline code and path tokens.
fn reference_paths(text: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut rest = text;
    while let Some(link_start) = rest.find("](") {
        rest = &rest[link_start + 2..];
        let Some(link_end) = rest.find(')') else {
            break;
        };
        if let Some(path) = reference_path(&rest[..link_end]) {
            paths.push(path);
        }
        rest = &rest[link_end + 1..];
    }
    paths.extend(
        text.split(|c: char| c.is_whitespace() || "`()[]<>\"'".contains(c))
            .filter_map(reference_path),
    );
    paths
}

fn safe_reference_paths() -> BTreeSet<PathBuf> {
    SAFE_REFERENCE_PATHS.iter().map(PathBuf::from).collect()
}

fn actual_reference_paths() -> BTreeSet<PathBuf> {
    let root = repo_root().join(SKILL).join("references");
    let mut paths = Vec::new();
    paths_under(&root, &mut paths);
    paths
        .into_iter()
        .map(|path| {
            path.strip_prefix(&root)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
                .to_path_buf()
        })
        .collect()
}

fn is_safe_reference_path(path: &Path) -> bool {
    SAFE_REFERENCE_PATHS
        .iter()
        .any(|allowed| Path::new(allowed) == path)
}

#[test]
fn safe_reference_paths_are_explicit_and_link_targets_resolve_within_them() {
    let allowed = safe_reference_paths();
    assert_eq!(actual_reference_paths(), allowed);
    let unlisted = reference_path("../references/not-in-inventory.md").unwrap();
    assert!(!is_safe_reference_path(&unlisted));
    for file in files_under(&repo_root().join(SKILL)) {
        for path in reference_paths(&read(&file)) {
            assert!(
                is_safe_reference_path(&path),
                "{} names an unapproved reference path {}",
                file.display(),
                path.display()
            );
        }
    }
}

/// Add the sentinel as the text of every clause of a fixture clause set.
fn with_clause_text(set: &str) -> String {
    set.lines()
        .flat_map(|line| {
            let mut out = vec![line.to_owned()];
            if line.trim() == "force: mandatory" {
                out.push(format!("    text: \"{SENTINEL}\""));
            }
            out
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[trace("FR-172-AC-7")]
#[test]
fn tc_172_001_no_shipped_skill_or_fixture_file_carries_a_restricted_id() {
    let root = scan_root();
    let mut files = files_under(&root.join(SKILL));
    files.extend(files_under(&root.join(FIXTURES)));
    for file in files {
        let body = read(&file);
        assert_eq!(
            restricted_ids(&format!("{}\n{body}", file.display())),
            Vec::<String>::new(),
            "{} carries a restricted-source identifier",
            file.display()
        );
    }
}

#[trace("FR-172-AC-8")]
#[test]
fn tc_172_002_skill_ships_no_file_or_directory_named_for_a_restricted_reference() {
    assert_eq!(actual_reference_paths(), safe_reference_paths());
}

#[trace("FR-172-AC-5", "FR-172-AC-9", "FR-172-AC-11")]
#[test]
fn tc_172_003_no_skill_file_links_or_names_a_restricted_reference_path() {
    let skill = repo_root().join(SKILL);
    for file in files_under(&skill) {
        for path in reference_paths(&read(&file)) {
            assert!(
                is_safe_reference_path(&path),
                "{} names an unapproved reference path {}",
                file.display(),
                path.display()
            );
        }
    }
    let text = read(&skill.join("SKILL.md"));
    assert!(text.contains("Licensed standards come from an installed pack only"));
    assert!(text.contains("`pack=true`"));
    assert!(text.contains("not mapped, set not installed"));
    assert!(text.contains("source-profile.yaml` lists in `add`"));
    assert!(text.contains("may read an installed pack's clause text locally"));
    assert!(text.contains("never writes any title, text or paraphrase"));
}

#[trace("FR-172-AC-10")]
#[test]
fn tc_172_004_fixture_pack_is_listed_as_a_pack_with_no_clause_text() {
    let fixture = repo_root().join(FIXTURES).join("packs/example-invalid-std");
    let root = scratch();
    let base = root.path().canonicalize().unwrap();
    let home = base.join("ix-home");
    semantic_root(&home);
    let cwd = base.join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();

    // The committed fixture holds ids only; the sentinel text exists only in
    // this temp copy.
    let source = base.join("std-pack");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::copy(fixture.join("manifest.yaml"), source.join("manifest.yaml")).unwrap();
    let set = read(&fixture.join("std.yaml"));
    assert!(
        !set.contains("text: \""),
        "the committed fixture carries no text"
    );
    std::fs::write(source.join("std.yaml"), with_clause_text(&set)).unwrap();
    assert!(read(&source.join("std.yaml")).contains(SENTINEL));

    let before = text(&quoin(&home, &cwd, &["module", "list"], &[]));
    assert!(!before.contains("example-invalid-std"), "{before}");

    let installed = quoin(
        &home,
        &cwd,
        &["module", "install", &format!("path:{}", source.display())],
        &[],
    );
    assert!(installed.status.success(), "{}", text(&installed));
    let after = text(&quoin(&home, &cwd, &["module", "list"], &[]));
    assert!(after.contains("example-invalid-std"), "{after}");
    assert!(after.contains("\"pack\": true"), "{after}");
    assert!(!after.contains(SENTINEL), "{after}");
    assert!(!text(&installed).contains(SENTINEL));
}
