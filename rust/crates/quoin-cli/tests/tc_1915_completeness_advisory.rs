// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The real `quoin completeness` command stays advisory for vocabulary gaps
//! unless strict mode is requested (FR-037).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test fixtures may panic"
)]

use ix_trace_rs::trace;
use serde_json::Value;
use std::fs;
use std::process::Command;

fn json_at<'a>(value: &'a Value, pointer: &str) -> &'a Value {
    value.pointer(pointer).unwrap_or(&Value::Null)
}

const MODULE: &str = "name: iso\n\
    artifact_types:\n\
    \x20 - name: NFR\n\
    \x20   frontmatter_schema_ref: schemas/nfr.json\n\
    traceability:\n\
    \x20 vocabulary_coverage:\n\
    \x20   - name: quality-characteristics\n\
    \x20     from: NFR\n\
    \x20     field: quality_attribute\n\
    \x20     check: vocabulary-coverage\n";

fn setup() -> (tempfile::TempDir, tempfile::TempDir, tempfile::TempDir) {
    let repo = tempfile::tempdir().expect("repository fixture");
    let home = tempfile::tempdir().expect("isolated config root");
    let module = tempfile::tempdir().expect("module fixture");
    fs::create_dir_all(repo.path().join("spec")).expect("bundle directory");
    fs::create_dir_all(module.path().join("schemas")).expect("schema directory");
    let manifest = MODULE.replace(
        "check: vocabulary-coverage",
        "justified_absence_field: excluded\n      check: vocabulary-coverage",
    );
    fs::write(module.path().join("manifest.yaml"), manifest).expect("module manifest");
    fs::write(
        module.path().join("schemas/nfr.json"),
        r#"{"properties":{"quality_attribute":{"enum":["security","reliability"]}}}"#,
    )
    .expect("module schema");
    (repo, home, module)
}

fn run(
    repo: &tempfile::TempDir,
    home: &tempfile::TempDir,
    module: &tempfile::TempDir,
    strict: bool,
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
    command
        .args(["completeness", "--json"])
        .arg("--repo")
        .arg(repo.path())
        .arg("--bundle")
        .arg(repo.path().join("spec"))
        .arg("--module")
        .arg(module.path())
        .current_dir(repo.path())
        .env("IX_HOME", home.path());
    if strict {
        command.arg("--strict");
    }
    command.output().expect("the native quoin binary runs")
}

#[trace("FR-037-AC-9")]
#[test]
fn tc_1915_completeness_reports_a_real_gap_as_advisory() {
    let repo = tempfile::tempdir().expect("repository fixture");
    let home = tempfile::tempdir().expect("isolated config root");
    let module = tempfile::tempdir().expect("module fixture");
    fs::create_dir_all(repo.path().join("spec")).expect("bundle directory");
    fs::create_dir_all(module.path().join("schemas")).expect("schema directory");
    fs::write(module.path().join("manifest.yaml"), MODULE).expect("module manifest");
    fs::write(
        module.path().join("schemas/nfr.json"),
        r#"{"properties":{"quality_attribute":{"enum":["security","reliability"]}}}"#,
    )
    .expect("module schema");
    fs::write(
        repo.path().join("spec/NFR-001.md"),
        "---\ntype: NFR\nquality_attribute: security\n---\nbody\n",
    )
    .expect("bundle document");

    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["completeness", "--json"])
        .arg("--repo")
        .arg(repo.path())
        .arg("--bundle")
        .arg(repo.path().join("spec"))
        .arg("--module")
        .arg(module.path())
        .current_dir(repo.path())
        .env("IX_HOME", home.path())
        .output()
        .expect("the native quoin binary runs");

    assert!(
        output.status.success(),
        "advisory gaps should exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("CONDITIONAL"), "{stdout}");
    assert!(stdout.contains("reliability"), "{stdout}");
}

#[trace("FR-037-AC-10")]
#[test]
fn tc_1915_unjustified_exclusion_fails_without_strict_mode() {
    let (repo, home, module) = setup();
    fs::write(
        repo.path().join("spec/NFR-001.md"),
        "---\ntype: NFR\nquality_attribute: security\nexcluded: reliability\n---\nno reason table\n",
    )
    .expect("bundle document");

    let output = run(&repo, &home, &module, false);
    assert!(
        !output.status.success(),
        "high findings must fail: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("unjustified-exclusion"));
}

#[trace("FR-037-AC-11")]
#[test]
fn tc_1915_strict_mode_fails_on_an_unowned_gap_alone() {
    let (repo, home, module) = setup();
    fs::write(
        repo.path().join("spec/NFR-001.md"),
        "---\ntype: NFR\nquality_attribute: security\n---\nbody\n",
    )
    .expect("bundle document");

    let output = run(&repo, &home, &module, true);
    assert!(
        !output.status.success(),
        "strict gaps must fail: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("unowned"));
}

#[trace("FR-037-AC-13")]
#[test]
fn tc_1915_reported_unowned_values_match_the_bundle_projection() {
    let (repo, home, module) = setup();
    fs::write(
        repo.path().join("spec/NFR-001.md"),
        "---\ntype: NFR\nquality_attribute: security\n---\nbody\n",
    )
    .expect("first bundle document");
    let output = run(&repo, &home, &module, false);
    assert!(output.status.success(), "advisory gaps should exit 0");
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON output");
    let unowned: Vec<&str> = json_at(&payload, "/findings")
        .as_array()
        .expect("findings array")
        .iter()
        .filter(|finding| finding.get("kind").and_then(Value::as_str) == Some("unowned"))
        .filter_map(|finding| finding.get("value").and_then(Value::as_str))
        .collect();
    assert_eq!(unowned, vec!["reliability"]);
    assert_eq!(json_at(&payload, "/rollups/0/owned"), 1);
    assert_eq!(json_at(&payload, "/rollups/0/unowned"), 1);
}
