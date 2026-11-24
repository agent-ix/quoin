// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `quoin risk advise` command surface (FR-145).
//!
//! A temporary module declares only the type contract needed to exercise the
//! command's record-validation path; it carries no copied spec content.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;

fn module_fixture(object_types: &str) -> tempfile::TempDir {
    let module = tempfile::tempdir().unwrap();
    std::fs::write(
        module.path().join("manifest.yaml"),
        format!(
            "manifest_version: 1.0.0\nname: risk-advice-test\nversion: 0.0.0\n\
             archetypes: []\nobject_types:\n{object_types}"
        ),
    )
    .unwrap();
    module
}

fn risk_module() -> tempfile::TempDir {
    module_fixture(
        r"- name: residual_risk
  body_extraction:
    yield_pattern:
      match:
        fixture_contract_field:
          from: frontmatter_field
          path: [fixture_contract_field]
          required: true
",
    )
}

fn safety_module_root() -> PathBuf {
    let checkout = std::env::var_os("SPEC_OBJECTS_SAFETY_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-objects-safety"),
        PathBuf::from,
    );
    let root = checkout.join("spec_objects_safety");
    assert!(
        root.join("manifest.yaml").is_file(),
        "the spec-objects-safety module is not at {}. Check out agent-ix/spec-objects-safety \
         next to this repository, or set SPEC_OBJECTS_SAFETY_DIR to its checkout",
        root.display()
    );
    root
}

fn advise(scope: &Path, module: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["risk", "advise"])
        .arg(scope)
        .arg("--module")
        .arg(module)
        .args(extra)
        .output()
        .unwrap()
}

fn decision_repo(author: &str, decider: &str) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("RR-1.md"),
        format!(
            "---\nid: RR-1\ntitle: \"RR-1\"\ntype: residual_risk\nobject: residual_risk\n\
             relationships:\n  - target: EST-1\n    type: estimate\n  - target: POL-1\n    type: references\n---\n\
             # [RR-1] RR-1\n\n## Determination\n\n| Decision | Rationale | Decided by |\n\
             | --- | --- | --- |\n| accepted | r | {decider} |\n\n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n\
             | --- | --- | --- |\n| record_id | UUID | 1..1 | identity |\n"
        ),
    )
    .unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir.path())
            .args(args)
            .env("GIT_AUTHOR_NAME", author)
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", author)
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}");
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "commit.gpgsign", "false"]);
    git(&["add", "RR-1.md"]);
    git(&["commit", "-q", "-m", "decision"]);
    let commit = git(&["rev-parse", "HEAD"]);
    (dir, commit)
}

struct SignatureVerifier {
    _tools: tempfile::TempDir,
    ea: PathBuf,
    allowed_signers: PathBuf,
    request_capture: PathBuf,
    result_file: PathBuf,
}

impl SignatureVerifier {
    fn new() -> Self {
        let tools = tempfile::tempdir().unwrap();
        let ea = tools.path().join("engineering-assurance");
        let allowed_signers = tools.path().join("allowed-signers");
        let request_capture = tools.path().join("request.json");
        let result_file = tools.path().join("result.json");
        std::fs::write(
            &allowed_signers,
            "fixture@example.invalid ssh-ed25519 AAAA\n",
        )
        .unwrap();
        std::fs::write(
            &ea,
            "#!/bin/sh\ncat > \"$SIGNATURE_VERIFY_REQUEST\"\ncat \"$SIGNATURE_VERIFY_RESULT\"\nexit \"$SIGNATURE_VERIFY_EXIT_CODE\"\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&ea, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self {
            _tools: tools,
            ea,
            allowed_signers,
            request_capture,
            result_file,
        }
    }

    fn write_result(&self, status: &str, principal: Option<&str>, verified_commit: &str) {
        let signer = principal.map_or_else(
            || serde_json::json!({"source": "allowed_signers", "mapping": "unmapped"}),
            |principal| {
                serde_json::json!({
                    "source": "allowed_signers",
                    "mapping": "mapped",
                    "principal": principal
                })
            },
        );
        let result = serde_json::json!({
            "protocol": "engineering-assurance.signature-verify-result/v1",
            "kind": "commit",
            "status": status,
            "verified_commit_id": verified_commit,
            "signer": signer
        });
        std::fs::write(&self.result_file, serde_json::to_vec(&result).unwrap()).unwrap();
    }

    fn run(&self, scope: &Path, module: &Path, verify_exit_code: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(["risk", "advise"])
            .arg(scope)
            .arg("--module")
            .arg(module)
            .args(["--format", "json", "--engineering-assurance"])
            .arg(&self.ea)
            .arg("--allowed-signers")
            .arg(&self.allowed_signers)
            .env("SIGNATURE_VERIFY_REQUEST", &self.request_capture)
            .env("SIGNATURE_VERIFY_RESULT", &self.result_file)
            .env("SIGNATURE_VERIFY_EXIT_CODE", verify_exit_code)
            .output()
            .unwrap()
    }
}

#[trace("FR-145-AC-1", "FR-145-AC-4", "FR-213-AC-6", "FR-214-AC-7")]
#[test]
fn json_over_an_empty_scope_exits_zero_with_the_six_keys() {
    let dir = tempfile::tempdir().unwrap();
    let module = risk_module();
    let out = advise(dir.path(), module.path(), &["--format", "json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut keys: Vec<&str> = report
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "attribution",
            "decision_attribution",
            "decision_queue",
            "evidence_gaps",
            "overall_evaluation_checks",
            "policy_cells"
        ]
    );
}

#[trace("FR-145-AC-4")]
#[test]
fn markdown_is_the_default() {
    let dir = tempfile::tempdir().unwrap();
    let module = risk_module();
    let out = advise(dir.path(), module.path(), &[]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("policy_cells"));
}

#[trace("FR-145-AC-7")]
#[test]
fn a_missing_scope_and_a_malformed_record_each_exit_2_naming_what_is_wrong() {
    let dir = tempfile::tempdir().unwrap();
    let module = safety_module_root();
    let missing = dir.path().join("absent");
    let out = advise(&missing, module.as_path(), &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("absent"));

    std::fs::write(
        dir.path().join("RR-1.md"),
        "---\nid: RR-1\ntitle: \"RR-1\"\ntype: residual_risk\nobject: residual_risk\n---\n# [RR-1] RR-1\n\n\
         ## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| maybe | r | |\n\n\
         ## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|---|---|---|---|\n| id | UUID | 1..1 | identity |\n",
    )
    .unwrap();
    let out = advise(dir.path(), module.as_path(), &[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("RR-1"), "{stderr}");
    assert!(
        stderr.contains("Decision") && stderr.contains("maybe"),
        "{stderr}"
    );
}

#[trace("FR-145-AC-7")]
#[test]
fn each_failure_exits_2_with_its_own_diagnostic_code_and_an_unreadable_file_is_named_on_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let module = risk_module();
    let code_of = |out: &Output| String::from_utf8_lossy(&out.stderr).into_owned();
    let out = advise(&dir.path().join("absent"), module.path(), &[]);
    assert!(
        code_of(&out).contains("risk.scope_missing"),
        "{}",
        code_of(&out)
    );

    let out = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["risk", "advise"])
        .arg(dir.path())
        .args(["--module", "/nonexistent-module-root"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        code_of(&out).contains("risk.module_missing"),
        "{}",
        code_of(&out)
    );

    std::fs::write(
        dir.path().join("RR-1.md"),
        "---\nid: RR-1\ntitle: \"RR-1\"\ntype: residual_risk\nobject: residual_risk\n---\n# [RR-1] RR-1\n",
    )
    .unwrap();
    let undeclared_module = module_fixture("- name: unrelated_test_type\n");
    let out = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["risk", "advise"])
        .arg(dir.path())
        .arg("--module")
        .arg(undeclared_module.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = code_of(&out);
    assert!(stderr.contains("risk.type_undeclared"), "{stderr}");
    assert!(stderr.contains("--module DIR"), "{stderr}");

    std::fs::write(
        dir.path().join("RR-1.md"),
        "---\nid: RR-1\ntitle: x\ntype: residual_risk\nobject: residual_risk\n---\n# x\n",
    )
    .unwrap();
    let out = advise(dir.path(), module.path(), &[]);
    assert!(
        code_of(&out).contains("risk.record_invalid"),
        "{}",
        code_of(&out)
    );

    std::fs::remove_file(dir.path().join("RR-1.md")).unwrap();
    std::fs::write(dir.path().join("RR-9.md"), [0xff, 0xfe, 0x00, 0x41]).unwrap();
    let out = advise(dir.path(), module.path(), &[]);
    assert!(out.status.success());
    assert!(
        code_of(&out).contains("risk.document_unreadable"),
        "{}",
        code_of(&out)
    );
    assert!(code_of(&out).contains("RR-9.md"));
}

#[trace("FR-214-AC-8", "FR-214-AC-9")]
#[test]
fn risk_advice_uses_mapped_principal_and_passes_signature_verify_inputs() {
    let (repo, commit) = decision_repo("Pat", "Lee");
    let module = safety_module_root();
    let verifier = SignatureVerifier::new();
    verifier.write_result("signed_valid", Some("Lee"), &commit);
    let out = verifier.run(repo.path(), &module, "0");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let entry = &report["decision_attribution"][0];
    assert_eq!(entry["signer_principal"], "Lee");
    let findings: Vec<&str> = entry["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["finding"].as_str().unwrap())
        .collect();
    assert!(!findings.contains(&"decided-by-differs-from-commit-author"));
    assert!(!findings.contains(&"decided-by-differs-from-verified-signer"));
    let request: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&verifier.request_capture).unwrap()).unwrap();
    assert_eq!(request["kind"], "commit");
    assert_eq!(
        request["repository"],
        repo.path()
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .as_ref()
    );
    assert_eq!(request["revision"], commit);
    assert_eq!(
        request["allowed_signers"],
        verifier.allowed_signers.to_string_lossy().as_ref()
    );
}

#[trace("FR-214-AC-8", "FR-214-AC-9")]
#[test]
fn risk_advice_reports_a_mapped_signer_mismatch() {
    let (repo, commit) = decision_repo("Pat", "Lee");
    let module = safety_module_root();
    let verifier = SignatureVerifier::new();
    verifier.write_result("signed_valid", Some("Morgan"), &commit);
    let out = verifier.run(repo.path(), &module, "0");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let findings = report["decision_attribution"][0]["findings"]
        .as_array()
        .unwrap();
    let mismatch = findings
        .iter()
        .find(|finding| finding["finding"] == "decided-by-differs-from-verified-signer")
        .unwrap();
    assert_eq!(mismatch["names"], serde_json::json!(["Lee", "Morgan"]));
    assert!(
        !findings
            .iter()
            .any(|finding| finding["finding"] == "decided-by-differs-from-commit-author")
    );
}

#[trace("FR-214-AC-8", "FR-214-AC-9")]
#[test]
fn risk_advice_does_not_infer_a_principal_from_an_unmapped_signer() {
    let (repo, commit) = decision_repo("Pat", "Lee");
    let module = safety_module_root();
    let verifier = SignatureVerifier::new();
    verifier.write_result("signed_untrusted", None, &commit);
    let out = verifier.run(repo.path(), &module, "0");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let entry = &report["decision_attribution"][0];
    assert!(entry["signer_principal"].is_null());
    assert!(
        !entry["findings"].as_array().unwrap().iter().any(|finding| {
            finding["finding"] == "decided-by-differs-from-commit-author"
                || finding["finding"] == "decided-by-differs-from-verified-signer"
        })
    );
}

#[trace("FR-214-AC-8", "FR-214-AC-9")]
#[test]
fn risk_advice_rejects_a_signature_result_for_another_commit() {
    let (repo, _commit) = decision_repo("Pat", "Lee");
    let module = safety_module_root();
    let verifier = SignatureVerifier::new();
    verifier.write_result("signed_valid", Some("Lee"), "another-commit");
    let out = verifier.run(repo.path(), &module, "0");
    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("risk.signature_verification_result_invalid")
    );
}

#[trace("FR-214-AC-8", "FR-214-AC-9")]
#[test]
fn risk_advice_reports_an_invalid_allowed_signers_input() {
    let (repo, _commit) = decision_repo("Pat", "Lee");
    let module = safety_module_root();
    let verifier = SignatureVerifier::new();
    let refused = serde_json::json!({
        "protocol": "engineering-assurance.error/v1",
        "code": "signature_verify_allowed_signers_invalid",
        "message": "bad allowed-signers file"
    });
    std::fs::write(&verifier.result_file, serde_json::to_vec(&refused).unwrap()).unwrap();
    let out = verifier.run(repo.path(), &module, "2");
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("risk.allowed_signers_invalid"));
}

#[trace("FR-214-AC-8", "FR-214-AC-9")]
#[test]
fn risk_advice_reports_signature_verifier_unavailability() {
    let (repo, _commit) = decision_repo("Pat", "Lee");
    let module = safety_module_root();
    let verifier = SignatureVerifier::new();
    let unavailable = serde_json::json!({
        "protocol": "engineering-assurance.error/v1",
        "code": "signature_verify_unavailable",
        "message": "verification unavailable"
    });
    std::fs::write(
        &verifier.result_file,
        serde_json::to_vec(&unavailable).unwrap(),
    )
    .unwrap();
    let out = verifier.run(repo.path(), &module, "1");
    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("risk.signature_verification_failed"));
}

#[trace("FR-214-AC-9")]
#[test]
fn risk_advice_requires_verification_options_as_a_pair() {
    let dir = tempfile::tempdir().unwrap();
    let module = risk_module();
    let out = advise(
        dir.path(),
        module.path(),
        &["--engineering-assurance", "/tmp/engineering-assurance"],
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("risk.signature_options_incomplete"));
}
