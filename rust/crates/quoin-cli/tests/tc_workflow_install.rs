// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Workflow installation from a complete package into an isolated config root.

#![allow(
    clippy::expect_used,
    reason = "test fixture failures should identify the failed operation"
)]

use std::fs;
use std::path::Path;
use std::process::Command;

fn package(root: &Path, enforced: bool) {
    fs::create_dir_all(root.join("dist")).expect("bundle directory");
    fs::write(
        root.join("dist/index.js"),
        "export const specInvariants = {};",
    )
    .expect("bundle");
    fs::write(
        root.join("package.json"),
        r#"{"name":"@agent-ix/ix-spec-workflows","version":"0.1.4"}"#,
    )
    .expect("manifest");
    for name in ["review", "to-plan"] {
        let skill = root.join("skills").join(name);
        fs::create_dir_all(skill.join("workflows").join(name)).expect("definition directory");
        fs::create_dir_all(skill.join("scripts")).expect("scripts directory");
        fs::write(skill.join("SKILL.md"), "Fixture skill").expect("skill");
        fs::write(
            skill.join("scripts/invariants.js"),
            "export const invariants = {};",
        )
        .expect("shim");
        let gate = if enforced {
            "[selected_analyses_covered]"
        } else {
            "[]"
        };
        fs::write(
            skill.join("workflows").join(name).join("def.yaml"),
            format!("transitions:\n  - to: accepted\n    invariants: {gate}\n"),
        )
        .expect("definition");
    }
}

/// Trace: FR-020-AC-4, FR-020-AC-5, FR-020-AC-6
#[test]
fn tc_workflow_install_emits_json_and_refuses_to_replace_assets_with_a_legacy_review() {
    let source = tempfile::tempdir().expect("source package");
    let config = tempfile::tempdir().expect("configuration root");
    let project = tempfile::tempdir().expect("project without Node dependencies");
    package(source.path(), true);
    let install = || {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(["workflows", "install", "--source"])
            .arg(source.path())
            .arg("--config-root")
            .arg(config.path())
            .arg("--json")
            .current_dir(project.path())
            .output()
            .expect("native installer")
    };
    let output = install();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON output");
    assert_eq!(
        response.get("version").and_then(serde_json::Value::as_str),
        Some("0.1.4")
    );
    let installed = config.path().join("plugins/ix-spec-workflows");
    assert!(installed.join("dist/index.js").is_file());
    let definition = installed.join("skills/review/workflows/review/def.yaml");
    let before = fs::read_to_string(&definition).expect("installed definition");
    package(source.path(), false);
    let refused = install();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("lacks selected_analyses_covered"));
    assert_eq!(
        fs::read_to_string(definition).expect("preserved definition"),
        before
    );
}
