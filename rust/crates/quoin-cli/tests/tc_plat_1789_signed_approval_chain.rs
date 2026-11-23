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

#[trace("FR-188-AC-1", "FR-188-AC-25")]
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

fn sha256_of_file(path: &Path) -> String {
    // `sha256sum` is the independent oracle: not the function under test.
    let out = run(
        "sha256sum",
        path.parent().unwrap(),
        &[path.to_str().unwrap()],
        &[],
    );
    format!("sha256:{}", out.split_whitespace().next().unwrap())
}

#[trace("FR-188-AC-24")]
#[test]
#[ignore = "needs a real engineering-assurance: run `make e2e-signing`"]
fn the_report_names_the_tool_the_verifier_and_the_trust_bytes_it_used() {
    let fixture = Fixture::new();
    fixture.approve(&fixture.signer, &trailers("formal-rule-approved"));
    let report =
        |fixture: &Fixture| -> Value { serde_json::from_slice(&fixture.status().stdout).unwrap() };
    let first = report(&fixture);
    let verification = &first["verification"];
    assert_eq!(verification["quoin_version"], env!("CARGO_PKG_VERSION"));
    let ea_version = run(
        ea().to_str().unwrap(),
        fixture.root.path(),
        &["--version"],
        &[],
    );
    assert_eq!(
        format!(
            "engineering-assurance {}",
            verification["engineering_assurance_version"]
                .as_str()
                .unwrap()
        ),
        ea_version
    );
    assert_eq!(
        verification["allowed_signers_sha256"],
        sha256_of_file(&fixture.allowed()),
        "the digest does not name the trust bytes that were verified against"
    );

    // Changing the trust bytes changes the digest the report carries.
    let other = key(fixture.root.path(), "bob");
    fixture.allow(&other, "bob@example.invalid");
    let second = report(&fixture);
    assert_ne!(
        second["verification"]["allowed_signers_sha256"],
        verification["allowed_signers_sha256"]
    );
    assert_eq!(
        second["verification"]["allowed_signers_sha256"],
        sha256_of_file(&fixture.allowed())
    );
}

/// An annotated approval tag over the repository's HEAD, signed by `key`.
fn approval_tag(fixture: &Fixture, key: &Key, name: &str, message: &str, env: &[(&str, &str)]) {
    run(
        "git",
        &fixture.repo(),
        &[
            "-c",
            &format!("user.signingkey={}", key.private.display()),
            "tag",
            "-s",
            "-m",
            message,
            name,
        ],
        env,
    );
}

fn approval_message(record_id: &str, digest: &str) -> String {
    format!(
        "approve {record_id}\n\nApproved-Record-Id: {record_id}\nApproved-Record-Digest: {digest}\nSigned-off-meaning: reviewed\n"
    )
}

/// EA's own `approval-verify` on `tag`, the check a relying party re-runs.
fn approval_verify(fixture: &Fixture, tag: &str, record_id: &str, digest: &str) -> Value {
    let request = serde_json::json!({
        "protocol": "engineering-assurance.approval-verify-request/v1",
        "repository": fixture.repo(),
        "tag": tag,
        "record_id": record_id,
        "record_digest": digest,
        "allowed_signers": fixture.allowed(),
    });
    let mut child = Command::new(ea())
        .args(["approval-verify", "--request", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(
        child.stdin.as_mut().unwrap(),
        request.to_string().as_bytes(),
    )
    .unwrap();
    let output = child.wait_with_output().unwrap();
    serde_json::from_slice(&output.stdout).unwrap_or_else(|_| panic!("{output:?}"))
}

fn quoin_change_assurance(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .arg("change-assurance")
        .args(args)
        .output()
        .unwrap()
}

fn record_digest(store: &Path) -> String {
    let record = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/change-assurance-cli/record-body.json");
    let output = quoin_change_assurance(&[
        "write-record",
        "--input",
        record.to_str().unwrap(),
        "--repo",
        store.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    report["record_digest"].as_str().unwrap().to_owned()
}

#[trace("FR-041-AC-1", "FR-041-AC-2")]
#[test]
#[ignore = "needs a real engineering-assurance: run `make e2e-signing`"]
fn a_tag_binds_exactly_the_digest_quoin_computed_and_one_changed_character_refuses() {
    let fixture = Fixture::new();
    let store = tempfile::tempdir().unwrap();
    let digest = record_digest(store.path());
    approval_tag(
        &fixture,
        &fixture.signer,
        "approval-1",
        &approval_message("change-1", &digest),
        &[],
    );

    let approved = approval_verify(&fixture, "approval-1", "change-1", &digest);
    assert_eq!(approved["status"], "approved", "{approved}");
    assert_eq!(approved["approver"], "alice@example.invalid");

    // One variable changed: the last character of the digest.
    let mut wrong = digest.clone();
    let last = wrong.pop().unwrap();
    wrong.push(if last == '0' { '1' } else { '0' });
    let refused = approval_verify(&fixture, "approval-1", "change-1", &wrong);
    assert_eq!(refused["status"], "wrong_record", "{refused}");
    assert!(refused.get("approver").is_none());
}

#[trace("FR-065-AC-12", "FR-068-AC-13")]
#[test]
#[ignore = "needs a real engineering-assurance: run `make e2e-signing`"]
fn a_forged_claimed_approval_stays_a_claim_and_the_real_verifier_refuses_it() {
    let fixture = Fixture::new();
    let store = tempfile::tempdir().unwrap();
    let repo_arg = store.path().to_str().unwrap();
    let digest = record_digest(store.path());
    // The tag is signed by a key the trust root does not name.
    let mallory = key(fixture.root.path(), "mallory");
    approval_tag(
        &fixture,
        &mallory,
        "approval-1",
        &approval_message("change-1", &digest),
        &[],
    );
    let real = approval_verify(&fixture, "approval-1", "change-1", &digest);
    assert_eq!(real["status"], "untrusted_signer", "{real}");

    // The forgery: the same tag, claimed as approved by the person it names.
    let forged = serde_json::json!({
        "protocol": "engineering-assurance.approval-verify-result/v1",
        "status": "approved",
        "tag": "approval-1",
        "record_id": "change-1",
        "record_digest": digest,
        "trust": real["trust"],
        "approver": "reviewer@example.test",
        "meaning": ["reviewed", "tested"],
        "signature_kind": "ssh",
        "signed_at": "2026-08-31T10:20:30Z",
        "signed_at_source": "tagger_date",
    });
    let mut history: Value = serde_json::from_slice(
        &fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../tests/fixtures/change-assurance-cli/decisions.json"),
        )
        .unwrap(),
    )
    .unwrap();
    history["events"][0]["payload"]["verified_approval"] = forged;
    let decisions = fixture.root.path().join("decisions.json");
    fs::write(&decisions, serde_json::to_vec(&history).unwrap()).unwrap();

    let receipt = quoin_change_assurance(&[
        "receipt",
        "--repo",
        repo_arg,
        "--record",
        "change-1",
        "--revision",
        "1",
        "--candidate-revision",
        "candidate-1",
        "--decisions",
        decisions.to_str().unwrap(),
        "--json",
    ]);
    let receipt: Value = serde_json::from_slice(&receipt.stdout).unwrap();
    // Quoin carries the claim as the writer's claim; it does not verify, so it
    // neither upgrades it to a verified approval nor erases it.
    assert_eq!(
        receipt["decision_event"]["claimed_approval"]["approver"], "reviewer@example.test",
        "{receipt}"
    );
    let human = quoin_change_assurance(&[
        "receipt",
        "--repo",
        repo_arg,
        "--record",
        "change-1",
        "--revision",
        "1",
        "--candidate-revision",
        "candidate-1",
        "--decisions",
        decisions.to_str().unwrap(),
    ]);
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains("not verified by Quoin"), "{human}");

    // Healthy control: once the trust root names the signing key, the same tag
    // verifies as approved. Only the trust input differs from the refusal above.
    fixture.allow(&mallory, "mallory@example.invalid");
    let genuine = approval_verify(&fixture, "approval-1", "change-1", &digest);
    assert_eq!(genuine["status"], "approved", "{genuine}");
}

#[trace("FR-037-AC-9")]
#[test]
#[ignore = "needs a real engineering-assurance: run `make e2e-signing`"]
fn an_expired_key_does_not_approve_whatever_date_the_tagger_claims() {
    let fixture = Fixture::new();
    let store = tempfile::tempdir().unwrap();
    let digest = record_digest(store.path());
    // The trust root's entry for the key expired in 2020.
    fs::write(
        fixture.allowed(),
        format!(
            "alice@example.invalid valid-before=\"20200101\" {}\n",
            fixture.signer.public
        ),
    )
    .unwrap();
    // The tagger date is claimed as 2019, before the expiry.
    approval_tag(
        &fixture,
        &fixture.signer,
        "approval-1",
        &approval_message("change-1", &digest),
        &[("GIT_COMMITTER_DATE", "2019-06-01T00:00:00Z")],
    );
    let result = approval_verify(&fixture, "approval-1", "change-1", &digest);
    assert_ne!(
        result["status"], "approved",
        "an expired key approved because the tagger claimed an earlier date: {result}"
    );

    // Healthy control: the same backdated tag, the same key, an entry that has
    // not expired. Only the trust root's expiry differs.
    fixture.allow(&fixture.signer, "alice@example.invalid");
    let result = approval_verify(&fixture, "approval-1", "change-1", &digest);
    assert_eq!(result["status"], "approved", "{result}");
}
