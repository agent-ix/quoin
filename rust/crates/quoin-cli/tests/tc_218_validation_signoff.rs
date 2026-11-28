// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Real signature-verification acceptance test for validation report sign-off.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test assertions report sign-off verification failures"
)]

use ix_trace_rs::trace;
use serde_json::Value;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn git(repository: &Path, arguments: &[&str]) -> Output {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository)
        .args(arguments)
        .output()
        .expect("git starts");
    assert!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn find_program(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
}

fn verify(repository: &Path, allowed_signers: &Path, verifier: &Path) -> Value {
    let output = invoke(repository, allowed_signers, verifier);
    assert!(
        output.status.success(),
        "sign-off verifier failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("rendered sign-off facts are JSON")
}

fn invoke(repository: &Path, allowed_signers: &Path, verifier: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["validation", "report", "--verify-signoff", "--repo"])
        .arg(repository)
        .arg("--allowed-signers")
        .arg(allowed_signers)
        .arg("--engineering-assurance")
        .arg(verifier)
        .output()
        .expect("sign-off verification command runs")
}

#[trace("FR-218-AC-4", "FR-218-AC-5")]
#[test]
#[ignore = "real SSH signing lane requires Engineering Assurance, Git and ssh-keygen"]
fn signoff_reports_signed_matching_principal_and_refuses_missing_meaning_or_unsigned_commit() {
    let fixture = SignedReportFixture::new();

    fixture.signed_commit(
        "# Tool validation report\n",
        "quality",
        "Validation report",
        Some("tool-validation-accepted"),
    );
    let accepted = fixture.verify();
    assert_eq!(accepted["status"], "accepted");
    assert_eq!(accepted["principalMatchesRatedBy"], true);
    assert_eq!(accepted["meaningPresent"], true);

    fixture.signed_commit(
        "# Tool validation report\nNo meaning trailer.\n",
        "quality",
        "Update validation report",
        None,
    );
    let missing_meaning = fixture.verify();
    assert_eq!(missing_meaning["status"], "not_accepted");
    assert_eq!(missing_meaning["reason"], "meaning_missing");

    let other_public = generate_ssh_public_key(&fixture.temp, "other-signing-key", "other");
    std::fs::write(&fixture.allowed_signers, format!("other {other_public}"))
        .expect("unrelated signer mapping is written");
    let unmapped = fixture.verify();
    assert_eq!(unmapped["status"], "not_accepted");
    assert_eq!(unmapped["reason"], "signer_unmapped");

    fixture.write_quality_signer();
    fixture.signed_commit(
        "# Tool validation report\n",
        "different-person",
        "Validation report by a different principal",
        Some("tool-validation-accepted"),
    );
    let mismatch = fixture.verify();
    assert_eq!(mismatch["status"], "not_accepted");
    assert_eq!(mismatch["reason"], "signer_mismatch");

    git(&fixture.repository, &["config", "commit.gpgsign", "false"]);
    std::fs::write(
        fixture.repository.join("validation-report.md"),
        "# Unsigned report revision\n",
    )
    .expect("unsigned report written");
    git(&fixture.repository, &["add", "validation-report.md"]);
    git(
        &fixture.repository,
        &["commit", "--no-gpg-sign", "-m", "Unsigned report revision"],
    );
    let unsigned = fixture.verify();
    assert_eq!(unsigned["status"], "not_accepted");
    assert_eq!(unsigned["reason"], "unsigned");
}

struct SignedReportFixture {
    temp: tempfile::TempDir,
    repository: PathBuf,
    allowed_signers: PathBuf,
    verifier: PathBuf,
    quality_public_key: String,
}

impl SignedReportFixture {
    fn new() -> Self {
        let verifier =
            find_program("engineering-assurance").expect("Engineering Assurance executable");
        let temp = tempfile::tempdir().expect("temporary root");
        let repository = temp.path().join("repo");
        std::fs::create_dir_all(&repository).expect("repository directory");
        git(&repository, &["init", "--quiet"]);
        git(&repository, &["config", "user.name", "Quality Person"]);
        git(
            &repository,
            &["config", "user.email", "quality@example.test"],
        );
        git(&repository, &["config", "gpg.format", "ssh"]);

        let key = temp.path().join("signing-key");
        let quality_public_key = generate_ssh_public_key(&temp, "signing-key", "quality");
        let allowed_signers = temp.path().join("allowed_signers");
        std::fs::write(&allowed_signers, format!("quality {quality_public_key}"))
            .expect("trust file written");
        git(
            &repository,
            &[
                "config",
                "gpg.ssh.allowedSignersFile",
                allowed_signers.to_str().expect("trust path is UTF-8"),
            ],
        );
        git(
            &repository,
            &[
                "config",
                "user.signingkey",
                key.to_str().expect("key path is UTF-8"),
            ],
        );
        Self {
            temp,
            repository,
            allowed_signers,
            verifier,
            quality_public_key,
        }
    }

    fn signed_commit(&self, report: &str, rated_by: &str, message: &str, meaning: Option<&str>) {
        std::fs::write(self.repository.join("validation-report.md"), report)
            .expect("report written");
        std::fs::write(
            self.repository.join("validation-report.json"),
            format!(r#"{{"ratedBy":"{rated_by}"}}"#),
        )
        .expect("report JSON written");
        git(
            &self.repository,
            &["add", "validation-report.md", "validation-report.json"],
        );
        let mut args = vec!["commit", "-S", "-m", message];
        let meaning_trailer = meaning.map(|meaning| format!("Signed-off-meaning: {meaning}"));
        if let Some(trailer) = meaning_trailer.as_deref() {
            args.extend(["-m", trailer]);
        }
        git(&self.repository, &args);
    }

    fn write_quality_signer(&self) {
        std::fs::write(
            &self.allowed_signers,
            format!("quality {}", self.quality_public_key),
        )
        .expect("quality signer mapping is restored");
    }

    fn verify(&self) -> Value {
        verify(&self.repository, &self.allowed_signers, &self.verifier)
    }
}

fn generate_ssh_public_key(temp: &tempfile::TempDir, name: &str, comment: &str) -> String {
    let key = temp.path().join(name);
    let generated = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", comment, "-f"])
        .arg(&key)
        .output()
        .expect("ssh-keygen starts");
    assert!(generated.status.success());
    let public = Command::new("ssh-keygen")
        .args(["-y", "-f"])
        .arg(&key)
        .output()
        .expect("ssh-keygen reads public key");
    assert!(public.status.success());
    String::from_utf8(public.stdout).expect("public key is UTF-8")
}

#[trace("FR-218-AC-5")]
#[test]
fn signoff_verification_resolves_relative_repository_and_output_paths() {
    let temp = tempfile::tempdir().expect("temporary root");
    let repository = temp.path().join("repo");
    let report_directory = repository.join("build/validation-report");
    std::fs::create_dir_all(&repository).expect("repository directory exists");
    git(&repository, &["init", "--quiet"]);
    git(&repository, &["config", "user.name", "Validation Tester"]);
    git(
        &repository,
        &["config", "user.email", "validation@example.test"],
    );
    std::fs::write(repository.join("seed.txt"), "seed\n").expect("seed file exists");
    git(&repository, &["add", "seed.txt"]);
    git(&repository, &["commit", "--quiet", "-m", "Seed repository"]);
    std::fs::create_dir_all(&report_directory).expect("report directory exists");
    std::fs::write(
        report_directory.join("validation-report.md"),
        "# Tool validation report\n",
    )
    .expect("report exists");

    let allowed_signers = temp.path().join("allowed_signers");
    std::fs::write(&allowed_signers, "").expect("allowed signers file exists");
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "validation",
            "report",
            "--verify-signoff",
            "--repo",
            ".",
            "--out",
            "build/validation-report",
            "--allowed-signers",
        ])
        .arg(&allowed_signers)
        .args(["--engineering-assurance", "/bin/true"])
        .current_dir(&repository)
        .output()
        .expect("sign-off verification command runs");
    assert!(
        output.status.success(),
        "sign-off verifier failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: Value = serde_json::from_slice(&output.stdout).expect("sign-off output is JSON");
    assert_eq!(result["status"], "not_accepted");
    assert_eq!(result["reason"], "unsigned");

    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            "validation",
            "report",
            "--verify-signoff",
            "--repo",
            "repo",
            "--out",
            "build/validation-report",
            "--allowed-signers",
        ])
        .arg(&allowed_signers)
        .args(["--engineering-assurance", "/bin/true"])
        .current_dir(temp.path())
        .output()
        .expect("sign-off verification command runs from the parent directory");
    assert!(
        output.status.success(),
        "sign-off verifier failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: Value = serde_json::from_slice(&output.stdout).expect("sign-off output is JSON");
    assert_eq!(result["status"], "not_accepted");
    assert_eq!(result["reason"], "unsigned");
}

#[cfg(unix)]
#[trace("FR-218-AC-4", "FR-218-AC-5")]
#[test]
fn signoff_interprets_engineering_assurance_results() {
    let temp = tempfile::tempdir().expect("temporary root");
    let repository = temp.path().join("repo");
    std::fs::create_dir_all(&repository).expect("repository directory");
    git(&repository, &["init", "--quiet"]);
    git(&repository, &["config", "user.name", "Quality Person"]);
    git(
        &repository,
        &["config", "user.email", "quality@example.test"],
    );
    std::fs::write(repository.join("validation-report.md"), "# Report\n").expect("report written");
    std::fs::write(
        repository.join("validation-report.json"),
        r#"{"ratedBy":"quality"}"#,
    )
    .expect("report JSON written");
    git(&repository, &["add", "."]);
    git(&repository, &["commit", "-m", "Validation report"]);

    let allowed_signers = temp.path().join("allowed_signers");
    std::fs::write(&allowed_signers, "quality ssh-ed25519 AAAA\n").expect("signers written");
    let result_path = temp.path().join("verification.json");
    let verifier = temp.path().join("engineering-assurance");
    std::fs::write(
        &verifier,
        format!(
            "#!/bin/sh\ncat >/dev/null\ncat '{}'\n",
            result_path.display()
        ),
    )
    .expect("verifier written");
    std::fs::set_permissions(&verifier, std::fs::Permissions::from_mode(0o755))
        .expect("verifier made executable");

    let set_result = |status: &str, mapping: &str, principal: &str, meaning: bool| {
        let meanings = if meaning {
            r#"["tool-validation-accepted"]"#
        } else {
            "[]"
        };
        std::fs::write(
            &result_path,
            format!(
                r#"{{"protocol":"engineering-assurance.signature-verify-result/v1","kind":"commit","status":"{status}","signer":{{"mapping":"{mapping}","principal":"{principal}"}},"signed_off_meaning":{{"values":{meanings}}}}}"#
            ),
        )
        .expect("verifier response written");
    };

    set_result("signed_valid", "mapped", "quality", true);
    let accepted = verify(&repository, &allowed_signers, &verifier);
    assert_eq!(accepted["status"], "accepted");
    assert_eq!(accepted["principalMatchesRatedBy"], true);
    assert_eq!(accepted["meaningPresent"], true);

    set_result("signed_untrusted", "unmapped", "", true);
    assert_eq!(
        verify(&repository, &allowed_signers, &verifier)["reason"],
        "signer_unmapped"
    );
    set_result("signed_valid", "mapped", "someone-else", true);
    assert_eq!(
        verify(&repository, &allowed_signers, &verifier)["reason"],
        "signer_mismatch"
    );
    set_result("signed_valid", "mapped", "quality", false);
    assert_eq!(
        verify(&repository, &allowed_signers, &verifier)["reason"],
        "meaning_missing"
    );
    set_result("unsigned", "unmapped", "", false);
    assert_eq!(
        verify(&repository, &allowed_signers, &verifier)["reason"],
        "unsigned"
    );
    set_result("signed_bad", "mapped", "quality", true);
    assert_eq!(
        verify(&repository, &allowed_signers, &verifier)["reason"],
        "signature_invalid"
    );

    std::fs::write(
        &result_path,
        r#"{"protocol":"wrong/v1","kind":"commit","status":"signed_valid"}"#,
    )
    .expect("wrong-protocol response is written");
    let wrong_protocol = invoke(&repository, &allowed_signers, &verifier);
    assert!(!wrong_protocol.status.success());
    assert!(
        String::from_utf8_lossy(&wrong_protocol.stderr).contains("unsupported signature result")
    );

    std::fs::write(
        &result_path,
        r#"{"protocol":"engineering-assurance.signature-verify-result/v1","kind":"commit","status":"signed_valid"}"#,
    )
    .expect("incomplete response is written");
    let incomplete = verify(&repository, &allowed_signers, &verifier);
    assert_eq!(incomplete["status"], "not_accepted");
    assert_eq!(incomplete["reason"], "signer_unmapped");

    std::fs::write(&result_path, "not-json\n").expect("malformed response is written");
    let malformed = invoke(&repository, &allowed_signers, &verifier);
    assert!(!malformed.status.success());
    assert!(String::from_utf8_lossy(&malformed.stderr).contains("invalid signature-verify result"));
}

#[cfg(unix)]
#[trace("FR-218-AC-4")]
#[test]
fn signoff_sends_the_exact_versioned_request_to_engineering_assurance() {
    let temp = tempfile::tempdir().expect("temporary root");
    let repository = temp.path().join("repo");
    std::fs::create_dir_all(&repository).expect("repository directory");
    git(&repository, &["init", "--quiet"]);
    git(&repository, &["config", "user.name", "Quality Person"]);
    git(
        &repository,
        &["config", "user.email", "quality@example.test"],
    );
    std::fs::write(repository.join("validation-report.md"), "# Report\n").expect("report written");
    std::fs::write(
        repository.join("validation-report.json"),
        r#"{"ratedBy":"quality"}"#,
    )
    .expect("report JSON written");
    git(&repository, &["add", "."]);
    git(&repository, &["commit", "-m", "Validation report"]);

    let allowed_signers = temp.path().join("allowed_signers");
    std::fs::write(&allowed_signers, "quality ssh-ed25519 AAAA\n")
        .expect("allowed signers written");
    let request_path = temp.path().join("request.json");
    let verifier = temp.path().join("engineering-assurance");
    std::fs::write(
        &verifier,
        format!(
            "#!/bin/sh\ncat > '{}'\nprintf '%s\\n' '{{\"protocol\":\"engineering-assurance.signature-verify-result/v1\",\"kind\":\"commit\",\"status\":\"signed_valid\",\"signer\":{{\"mapping\":\"mapped\",\"principal\":\"quality\"}},\"signed_off_meaning\":{{\"values\":[\"tool-validation-accepted\"]}}}}'\n",
            request_path.display()
        ),
    )
    .expect("verifier shim written");
    std::fs::set_permissions(&verifier, std::fs::Permissions::from_mode(0o755))
        .expect("verifier shim made executable");

    let accepted = verify(&repository, &allowed_signers, &verifier);
    assert_eq!(accepted["status"], "accepted");

    let request: Value = serde_json::from_slice(
        &std::fs::read(&request_path).expect("request sent to Engineering Assurance is retained"),
    )
    .expect("request parses as JSON");
    let revision = String::from_utf8(
        git(
            &repository,
            &["log", "-1", "--format=%H", "--", "validation-report.md"],
        )
        .stdout,
    )
    .expect("report commit id is UTF-8")
    .trim()
    .to_owned();
    assert_eq!(
        request,
        serde_json::json!({
            "protocol": "engineering-assurance.signature-verify-request/v1",
            "repository": repository.canonicalize().expect("repository path resolves"),
            "revision": revision,
            "kind": "commit",
            "allowed_signers": allowed_signers,
        })
    );
}
