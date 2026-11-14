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

#[trace("FR-145-AC-1", "FR-145-AC-4")]
#[test]
fn json_over_an_empty_scope_exits_zero_with_the_four_keys() {
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
            "decision_queue",
            "evidence_gaps",
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
    assert!(String::from_utf8_lossy(&out.stderr).contains("RR-1"));
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
