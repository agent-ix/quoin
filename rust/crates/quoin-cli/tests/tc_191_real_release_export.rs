// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-191 AC-27/28 real EA/Quire release-export chain.
//!
//! Named external integration lane. The test is active when the explicit
//! feature is enabled and requires the exact installed tools and local PDF
//! extractor:
//! - `engineering-assurance 0.10.1`
//! - `quire 0.53.3 (engine 0.63.2)`
//! - `ssh-keygen` and `pdftotext` (or an executable path in `PDFTOTEXT`)
//!
//! ```text
//! QUOIN_E2E_EA=/absolute/path/to/engineering-assurance \
//! QUIRE_E2E_QUIRE=/absolute/path/to/quire \
//! PDFTOTEXT=/absolute/path/to/pdftotext \
//! cargo test --locked -j 4 -p quoin-cli --features e2e-release-export \
//!   --test tc_191_real_release_export real_ea_release_and_quire_dossier_export_scenario \
//!   -- --exact --test-threads=4
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the end-to-end test must fail at the exact real-tool contract that diverges"
)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const RELEASE_TAG: &str = "release-v1";
const RECORD_PATH: &str = "release/REL-1.md";
const SNAPSHOT_PATH: &str = "release/snapshot.json";

fn binary(variable: &str) -> PathBuf {
    let path =
        PathBuf::from(std::env::var_os(variable).unwrap_or_else(|| panic!("set {variable}")));
    assert!(path.is_file(), "{} is not a file", path.display());
    path
}

fn run(program: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap_or_else(|error| panic!("{} {args:?}: {error}", program.display()))
}

fn run_with_input(program: &Path, cwd: &Path, args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("{} {args:?}: {error}", program.display()));
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

fn git(repo: &Path, args: &[&str]) -> String {
    let output = run(Path::new("git"), repo, args);
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn commit(repo: &Path, paths: &[&str], message: &str) -> String {
    for path in paths {
        git(repo, &["add", "--", path]);
    }
    git(repo, &["commit", "-q", "-m", message]);
    git(repo, &["rev-parse", "HEAD"])
}

fn quoin(repo: &Path, args: &[&str]) -> Output {
    let mut full_args = vec!["dossier"];
    full_args.extend_from_slice(args);
    run(Path::new(env!("CARGO_BIN_EXE_quoin")), repo, &full_args)
}

#[trace("FR-191-AC-27", "FR-191-AC-28")]
#[test]
fn real_ea_release_and_quire_dossier_export_scenario() {
    let ea = binary("QUOIN_E2E_EA");
    let quire = binary("QUIRE_E2E_QUIRE");
    let pdftotext = std::env::var_os("PDFTOTEXT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("pdftotext"));
    let ea_version = run(&ea, Path::new("."), &["--version"]);
    assert!(ea_version.status.success());
    assert_eq!(
        String::from_utf8_lossy(&ea_version.stdout).trim(),
        "engineering-assurance 0.10.1"
    );
    let quire_version = run(&quire, Path::new("."), &["--version"]);
    assert!(quire_version.status.success());
    assert_eq!(
        String::from_utf8_lossy(&quire_version.stdout).trim(),
        "quire 0.53.3 (engine 0.63.2)"
    );
    assert!(
        Path::new("/usr/bin/ssh-keygen").is_file(),
        "ssh-keygen is required"
    );

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let repo = root.join("repo");
    fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["config", "user.name", "Ada Example"]);
    git(&repo, &["config", "user.email", "ada@example.invalid"]);
    git(&repo, &["config", "gpg.format", "ssh"]);

    let private_key = root.join("release-key");
    let keygen = run(
        Path::new("/usr/bin/ssh-keygen"),
        root,
        &[
            "-q",
            "-t",
            "ed25519",
            "-N",
            "",
            "-C",
            "ada@example.invalid",
            "-f",
            private_key.to_str().unwrap(),
        ],
    );
    assert!(
        keygen.status.success(),
        "{}",
        String::from_utf8_lossy(&keygen.stderr)
    );
    let public_key = fs::read_to_string(root.join("release-key.pub")).unwrap();
    let allowed_signers = root.join("allowed_signers");
    fs::write(
        &allowed_signers,
        format!("ada@example.invalid {}", public_key.trim()),
    )
    .unwrap();

    fs::write(repo.join("README.md"), "baseline README\n").unwrap();
    fs::write(
        repo.join("baseline.md"),
        "---\nid: BL-1\ntitle: Baseline one\ntype: Baseline\nname: Baseline one\nitems:\n  - path: README.md\n    kind: spec_artifact\n---\n\nBaseline.\n",
    )
    .unwrap();
    let baseline = commit(&repo, &["README.md", "baseline.md"], "baseline");

    fs::create_dir_all(repo.join("release")).unwrap();
    let snapshot = repo.join(SNAPSHOT_PATH);
    let snapshot_json = json!({
        "protocol": "quoin.release-snapshot/v1",
        "tool_identity": {"name": "quoin", "version": "0.0.0"},
        "clean_tree_precondition": "not_verified",
        "as_of": "2026-11-26T00:00:00Z",
        "baseline_commit": baseline,
        "previous_release": null,
        "coverage": [],
        "evidence_freshness": {"bound": 0, "stale": 0, "suspect": 0, "undischarged": 0, "no run evidence": 0},
        "open_findings": [],
        "soup": {"rows": [], "not_assessed": []},
        "vulnerabilities": [],
        "anomalies": [],
        "change_records": []
    });
    fs::write(
        &snapshot,
        serde_json::to_vec_pretty(&snapshot_json).unwrap(),
    )
    .unwrap();
    let record = format!(
        "---\nid: REL-1\ntitle: Release one\ntype: Release\nname: Release one\nbaseline:\n  path: baseline.md\n  commit: \"{baseline}\"\nsnapshot_path: {SNAPSHOT_PATH}\ndecision_owner: ada@example.invalid\n---\n\nRelease record.\n"
    );
    fs::write(repo.join(RECORD_PATH), record).unwrap();
    let release = commit(&repo, &[RECORD_PATH, SNAPSHOT_PATH], "release");
    assert_eq!(
        git(&repo, &["show", "-s", "--format=%P", &release]),
        baseline
    );
    let mut release_paths = git(
        &repo,
        &["diff-tree", "-r", "--name-only", &baseline, &release],
    )
    .lines()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    release_paths.sort();
    assert_eq!(release_paths, [RECORD_PATH, SNAPSHOT_PATH]);
    let digest = git(&repo, &["rev-parse", &format!("{release}:{RECORD_PATH}")]);
    let message = format!(
        "Release one\n\nApproved-Record-Id: REL-1\nApproved-Record-Digest: git-blob:{digest}\nSigned-off-meaning: approved for release"
    );
    let signing_key = format!("user.signingkey={}", private_key.display());
    git(
        &repo,
        &[
            "-c",
            "user.name=Ada Example",
            "-c",
            "user.email=ada@example.invalid",
            "-c",
            "gpg.format=ssh",
            "-c",
            &signing_key,
            "tag",
            "-s",
            "-m",
            &message,
            RELEASE_TAG,
            &release,
        ],
    );

    let request = json!({
        "protocol": "engineering-assurance.release-verify-request/v1",
        "repository": repo.display().to_string(),
        "tag": RELEASE_TAG,
        "record_path": RECORD_PATH,
        "allowed_signers": allowed_signers.display().to_string()
    });
    let verified = run_with_input(
        &ea,
        root,
        &["release-verify", "--request", "-"],
        &serde_json::to_vec(&request).unwrap(),
    );
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );
    let verify_json: Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(verify_json["status"], "released", "{verify_json}");
    let verify_path = root.join("release-verify.json");
    fs::write(&verify_path, verified.stdout).unwrap();

    let generated = quoin(
        &repo,
        &[
            "generate",
            "--doc",
            "release-record",
            "--version",
            "1.0",
            "--snapshot",
            snapshot.to_str().unwrap(),
            "--release-verify",
            verify_path.to_str().unwrap(),
        ],
    );
    assert_eq!(
        generated.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    fs::create_dir_all(repo.join("dossier")).unwrap();
    fs::write(repo.join("dossier/NOTES.md"), "not selected\n").unwrap();
    let dossier_commit = commit(
        &repo,
        &["dossier/DOS-REL.md", "dossier/NOTES.md"],
        "dossier",
    );
    let dossier_date = git(&repo, &["show", "-s", "--format=%cs", &dossier_commit]);

    let output = root.join("release.pdf");
    let (snapshot_arg, verify_arg, output_arg, quire_arg) = (
        snapshot.to_str().unwrap(),
        verify_path.to_str().unwrap(),
        output.to_str().unwrap(),
        quire.to_str().unwrap(),
    );
    let args = vec![
        "export",
        "--doc",
        "release-record",
        "--version",
        "1.0",
        "--snapshot",
        snapshot_arg,
        "--release-verify",
        verify_arg,
        "--release",
        RELEASE_TAG,
        "--to",
        "pdf",
        "--output",
        output_arg,
        "--quire",
        quire_arg,
    ];
    let exported = quoin(&repo, &args);
    assert_eq!(
        exported.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&exported.stderr)
    );
    let pdf_text = run(&pdftotext, root, &[output_arg, "-"]);
    assert!(
        pdf_text.status.success(),
        "{}",
        String::from_utf8_lossy(&pdf_text.stderr)
    );
    let text = String::from_utf8_lossy(&pdf_text.stdout)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        text.contains(&format!("{RELEASE_TAG} (tag commit {release})")),
        "{text}"
    );
    assert!(
        text.contains(&dossier_commit),
        "PDF omitted Source commit {dossier_commit}: {text}"
    );
    assert!(text.contains(&dossier_date), "{text}");
    let successful_pdf = fs::read(&output).unwrap();

    for (path, message) in [
        ("README.md", "change README"),
        ("dossier/NOTES.md", "remove NOTES"),
    ] {
        git(&repo, &["reset", "--hard", &dossier_commit]);
        if path == "README.md" {
            fs::write(repo.join(path), "changed outside dossier\n").unwrap();
        } else {
            fs::remove_file(repo.join(path)).unwrap();
        }
        commit(&repo, &[path], message);
        fs::write(&output, &successful_pdf).unwrap();
        let mut failure_args = args.clone();
        failure_args.push("--json");
        let refused = quoin(&repo, &failure_args);
        assert_eq!(
            refused.status.code(),
            Some(4),
            "{}",
            String::from_utf8_lossy(&refused.stderr)
        );
        let report: Value = serde_json::from_slice(&refused.stdout).unwrap();
        assert_eq!(report["quire"]["exit_code"], 3, "{report}");
        assert!(
            report["quire"]["stderr"]
                .as_str()
                .is_some_and(|value| !value.is_empty()),
            "{report}"
        );
        assert_eq!(fs::read(&output).unwrap(), successful_pdf);
    }
}
