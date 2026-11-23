// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! PLAT-1789: `quoin formal-rules status` against the REAL
//! `engineering-assurance` binary, real `git` and `ssh-keygen`, throwaway
//! repositories and throwaway keys (FR-188; engineering-assurance FR-036/037).
//!
//! The unit tests in `formal_rules.rs` stand a shell script in for
//! engineering-assurance and fabricate the answer, so they cannot see whether
//! the two programs agree on the wire. These do not.
//!
//! The tests are `#[ignore]`d from the ordinary run because the ordinary run has
//! no engineering-assurance; `make e2e-signing` runs them with
//! `QUOIN_E2E_EA` set and FAILS, not skips, when it is not.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::Value;

const RULE: &str = "qsl:a.native::P1";
const NATIVE: &str =
    "language qsl\nprofile core\n// formalizes: FR-001-AC-1\nrule P1 { check x }\n";
const MANIFEST: &str = "name: formal-rule-status-fixture\nversion: 0.1.0\ndescription: Status command fixture\nartifact_types:\n  - name: FR\ntraceability:\n  trace_tags:\n    formalizes:\n      - name: formalizes\n        language: qsl\n        syntax: comment\n        pattern: '^// formalizes: (.+)$'\n";

/// The engineering-assurance under test. A missing one is a failure.
fn ea() -> PathBuf {
    let path = std::env::var_os("QUOIN_E2E_EA")
        .map(PathBuf::from)
        .expect("QUOIN_E2E_EA must name an engineering-assurance executable; a missing dependency is not a pass");
    assert!(
        path.is_file(),
        "QUOIN_E2E_EA is not a file: {}",
        path.display()
    );
    path
}

fn run(program: &str, dir: &Path, args: &[&str], env: &[(&str, &str)]) -> String {
    let output = Command::new(program)
        .current_dir(dir)
        .args(args)
        .envs(env.iter().copied())
        .output()
        .unwrap_or_else(|error| panic!("{program} is unavailable: {error}"));
    assert!(
        output.status.success(),
        "{program} {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

struct Key {
    private: PathBuf,
    public: String,
}

fn key(dir: &Path, name: &str) -> Key {
    let private = dir.join(name);
    run(
        "ssh-keygen",
        dir,
        &[
            "-q",
            "-t",
            "ed25519",
            "-N",
            "",
            "-C",
            name,
            "-f",
            private.to_str().unwrap(),
        ],
        &[],
    );
    let public = fs::read_to_string(dir.join(format!("{name}.pub"))).unwrap();
    Key {
        private,
        public: public.trim().to_owned(),
    }
}

/// A throwaway repository, a signer key and an allowed-signers file naming it.
struct Fixture {
    root: tempfile::TempDir,
    signer: Key,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let signer = key(root.path(), "alice");
        let repo = root.path().join("repo");
        fs::create_dir(&repo).unwrap();
        run("git", &repo, &["init", "-q", "-b", "main"], &[]);
        for (k, v) in [
            ("user.name", "Alice"),
            ("user.email", "alice@example.invalid"),
            ("gpg.format", "ssh"),
        ] {
            run("git", &repo, &["config", k, v], &[]);
        }
        fs::write(repo.join("manifest.yaml"), MANIFEST).unwrap();
        fs::write(repo.join("a.native"), NATIVE).unwrap();
        run("git", &repo, &["add", "."], &[]);
        run("git", &repo, &["commit", "-q", "-m", "draft rule"], &[]);
        let fixture = Self { root, signer };
        fixture.allow(&fixture.signer, "alice@example.invalid");
        fixture
    }

    fn repo(&self) -> PathBuf {
        self.root.path().join("repo")
    }

    fn allowed(&self) -> PathBuf {
        self.root.path().join("allowed-signers")
    }

    /// Replace the allowed-signers file with one line.
    fn allow(&self, key: &Key, principal: &str) {
        fs::write(self.allowed(), format!("{principal} {}\n", key.public)).unwrap();
    }

    /// Commit the current tree, signed by `key`, with the given trailers.
    fn approve(&self, key: &Key, trailers: &str) -> String {
        let message = format!("approve\n\n{trailers}");
        run("git", &self.repo(), &["add", "."], &[]);
        run(
            "git",
            &self.repo(),
            &[
                "-c",
                &format!("user.signingkey={}", key.private.display()),
                "commit",
                "-q",
                "--allow-empty",
                "-S",
                "-m",
                &message,
            ],
            &[],
        );
        run("git", &self.repo(), &["rev-parse", "HEAD"], &[])
    }

    fn status(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(["formal-rules", "status", "--json", "--repo"])
            .arg(self.repo())
            .arg("--engineering-assurance")
            .arg(ea())
            .arg("--allowed-signers")
            .arg(self.allowed())
            .output()
            .unwrap()
    }

    /// The report's one rule.
    fn rule(&self) -> Value {
        let output = self.status();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let rules = report["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 1, "{report}");
        rules[0].clone()
    }
}

fn trailers(meaning: &str) -> String {
    format!("Formal-rule: {RULE}\nSigned-off-meaning: {meaning}\n")
}

#[trace("FR-188-AC-1")]
#[test]
#[ignore = "needs a real engineering-assurance: run `make e2e-signing`"]
fn a_signed_approval_with_the_permitted_meaning_is_approved_end_to_end() {
    let fixture = Fixture::new();
    let commit = fixture.approve(&fixture.signer, &trailers("formal-rule-approved"));
    let rule = fixture.rule();
    assert_eq!(rule["state"], "approved", "{rule}");
    assert_eq!(rule["approval"]["commit"], commit);
    assert_eq!(rule["approval"]["principal"], "alice@example.invalid");
}

#[trace("FR-188-AC-5")]
#[test]
#[ignore = "needs a real engineering-assurance: run `make e2e-signing`"]
fn a_valid_signature_by_a_key_the_allowed_signers_do_not_name_is_untrusted() {
    let fixture = Fixture::new();
    let mallory = key(fixture.root.path(), "mallory");
    fixture.approve(&mallory, &trailers("formal-rule-approved"));
    let rule = fixture.rule();
    assert_eq!(rule["state"], "draft", "{rule}");
    assert_eq!(rule["reason"], "approval_untrusted");
}

#[trace("FR-188-AC-5")]
#[test]
#[ignore = "needs a real engineering-assurance: run `make e2e-signing`"]
fn a_signer_removed_from_the_allowed_signers_after_signing_no_longer_approves() {
    let fixture = Fixture::new();
    fixture.approve(&fixture.signer, &trailers("formal-rule-approved"));
    assert_eq!(fixture.rule()["state"], "approved");
    // The same history, with the trust root now naming someone else.
    let other = key(fixture.root.path(), "bob");
    fixture.allow(&other, "bob@example.invalid");
    let rule = fixture.rule();
    assert_eq!(rule["state"], "draft", "{rule}");
    assert_eq!(rule["reason"], "approval_untrusted");
}

#[trace("FR-188-AC-9")]
#[test]
#[ignore = "needs a real engineering-assurance: run `make e2e-signing`"]
fn a_different_meaning_is_a_mismatch_not_an_approval() {
    let fixture = Fixture::new();
    fixture.approve(&fixture.signer, &trailers("release-approved"));
    let rule = fixture.rule();
    assert_eq!(rule["state"], "draft", "{rule}");
    assert_eq!(rule["reason"], "meaning_mismatch");
}

#[trace("FR-188-AC-10")]
#[test]
#[ignore = "needs a real engineering-assurance: run `make e2e-signing`"]
fn a_declaration_edited_after_approval_is_changed_since_approval() {
    let fixture = Fixture::new();
    fixture.approve(&fixture.signer, &trailers("formal-rule-approved"));
    fs::write(
        fixture.repo().join("a.native"),
        NATIVE.replace("check x", "check y"),
    )
    .unwrap();
    run("git", &fixture.repo(), &["add", "."], &[]);
    run(
        "git",
        &fixture.repo(),
        &["commit", "-q", "-m", "edit the rule"],
        &[],
    );
    let rule = fixture.rule();
    assert_eq!(rule["state"], "draft", "{rule}");
    assert_eq!(rule["reason"], "changed_since_approval");
}

#[trace("FR-188-AC-3", "FR-188-AC-19")]
#[test]
#[ignore = "needs a real engineering-assurance: run `make e2e-signing`"]
fn a_signed_tag_over_an_unsigned_commit_is_not_an_approval() {
    let fixture = Fixture::new();
    let message = format!("approve\n\n{}", trailers("formal-rule-approved"));
    run(
        "git",
        &fixture.repo(),
        &["commit", "-q", "--allow-empty", "-m", &message],
        &[],
    );
    run(
        "git",
        &fixture.repo(),
        &[
            "-c",
            &format!("user.signingkey={}", fixture.signer.private.display()),
            "tag",
            "-s",
            "-m",
            &trailers("formal-rule-approved"),
            "approval-1",
        ],
        &[],
    );
    let rule = fixture.rule();
    assert_eq!(rule["state"], "draft", "{rule}");
    assert_eq!(rule["reason"], "approval_unsigned");
}
