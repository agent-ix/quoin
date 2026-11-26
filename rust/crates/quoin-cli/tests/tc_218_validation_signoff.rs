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
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["validation", "report", "--verify-signoff", "--repo"])
        .arg(repository)
        .arg("--allowed-signers")
        .arg(allowed_signers)
        .arg("--engineering-assurance")
        .arg(verifier)
        .output()
        .expect("sign-off verification command runs");
    assert!(
        output.status.success(),
        "sign-off verifier failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("rendered sign-off facts are JSON")
}

#[trace("FR-218-AC-4", "FR-218-AC-5")]
#[test]
fn signoff_reports_signed_matching_principal_and_refuses_missing_meaning_or_unsigned_commit() {
    let verifier = find_program("engineering-assurance").expect("Engineering Assurance executable");
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
    let generated = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", "quality", "-f"])
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
    let allowed_signers = temp.path().join("allowed_signers");
    let public_key = String::from_utf8(public.stdout).expect("public key is UTF-8");
    std::fs::write(&allowed_signers, format!("quality {public_key}")).expect("trust file written");
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

    let report_path = repository.join("validation-report.md");
    let report_json_path = repository.join("validation-report.json");
    std::fs::write(&report_path, "# Tool validation report\n").expect("report written");
    std::fs::write(&report_json_path, r#"{"ratedBy":"quality"}"#).expect("report JSON written");
    git(
        &repository,
        &["add", "validation-report.md", "validation-report.json"],
    );
    git(
        &repository,
        &[
            "commit",
            "-S",
            "-m",
            "Validation report",
            "-m",
            "Signed-off-meaning: tool-validation-accepted",
        ],
    );
    let accepted = verify(&repository, &allowed_signers, &verifier);
    assert_eq!(accepted["status"], "accepted");
    assert_eq!(accepted["principalMatchesRatedBy"], true);
    assert_eq!(accepted["meaningPresent"], true);

    std::fs::write(
        &report_path,
        "# Tool validation report\nNo meaning trailer.\n",
    )
    .expect("updated report written");
    git(&repository, &["add", "validation-report.md"]);
    git(
        &repository,
        &["commit", "-S", "-m", "Update validation report"],
    );
    let missing_meaning = verify(&repository, &allowed_signers, &verifier);
    assert_eq!(missing_meaning["status"], "not_accepted");
    assert_eq!(missing_meaning["reason"], "meaning_missing");

    let other_key = temp.path().join("other-signing-key");
    let generated = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", "other", "-f"])
        .arg(&other_key)
        .output()
        .expect("ssh-keygen starts for the unrelated signer");
    assert!(generated.status.success());
    let other_public = Command::new("ssh-keygen")
        .args(["-y", "-f"])
        .arg(&other_key)
        .output()
        .expect("ssh-keygen reads the unrelated public key");
    assert!(other_public.status.success());
    std::fs::write(
        &allowed_signers,
        format!(
            "other {}",
            String::from_utf8(other_public.stdout).expect("public key is UTF-8")
        ),
    )
    .expect("unrelated signer mapping is written");
    let unmapped = verify(&repository, &allowed_signers, &verifier);
    assert_eq!(unmapped["status"], "not_accepted");
    assert_eq!(unmapped["reason"], "signer_unmapped");

    std::fs::write(&allowed_signers, format!("quality {public_key}"))
        .expect("quality signer mapping is restored");
    std::fs::write(&report_json_path, r#"{"ratedBy":"different-person"}"#)
        .expect("different rated-by principal is written");
    std::fs::write(&report_path, "# Tool validation report\n").expect("report is updated");
    git(
        &repository,
        &["add", "validation-report.md", "validation-report.json"],
    );
    git(
        &repository,
        &[
            "commit",
            "-S",
            "-m",
            "Validation report by a different principal",
            "-m",
            "Signed-off-meaning: tool-validation-accepted",
        ],
    );
    let mismatch = verify(&repository, &allowed_signers, &verifier);
    assert_eq!(mismatch["status"], "not_accepted");
    assert_eq!(mismatch["reason"], "signer_mismatch");

    git(&repository, &["config", "commit.gpgsign", "false"]);
    std::fs::write(&report_path, "# Unsigned report revision\n").expect("unsigned report written");
    git(&repository, &["add", "validation-report.md"]);
    git(
        &repository,
        &["commit", "--no-gpg-sign", "-m", "Unsigned report revision"],
    );
    let unsigned = verify(&repository, &allowed_signers, &verifier);
    assert_eq!(unsigned["status"], "not_accepted");
    assert_eq!(unsigned["reason"], "unsigned");
}
