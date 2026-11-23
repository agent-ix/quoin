// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin dossier generate` over a repository with and without a risk
//! management plan record (FR-200, FR-191-AC-22), reading the safety module's
//! own types. Point `SPEC_OBJECTS_SAFETY_DIR` at a spec-objects-safety checkout.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::Value;

const POLICY: &str = "---\nid: POL-1\ntitle: Policy\ntype: acceptability_policy\nobject: acceptability_policy\n---\n\
    # Policy\n\n\
    ## Severity levels\n\n| Severity level |\n| --- |\n| serious |\n\n\
    ## Probability levels\n\n| Probability level |\n| --- |\n| likely |\n\n\
    ## Regions\n\n| Region | Outside acceptable |\n| --- | --- |\n| unacceptable | true |\n\n\
    ## Cells\n\n| Severity | Probability | Region |\n| --- | --- | --- |\n| serious | likely | unacceptable |\n\n\
    ## Properties\n\n| Field | Type | Multiplicity | Constraints |\n| --- | --- | --- | --- |\n| record_id | UUID | 1..1 | identity |\n";

fn module_root() -> PathBuf {
    std::env::var_os("SPEC_OBJECTS_SAFETY_DIR")
        .map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-objects-safety"),
            PathBuf::from,
        )
        .join("spec_objects_safety")
}

fn plan_document() -> String {
    let skeleton =
        fs::read_to_string(module_root().join("skeletons/risk_management_plan.md")).unwrap();
    [
        ("<what the plan covers>", "Brake controller."),
        ("<responsibility>", "Keep the risk file current."),
        ("<person or role>", "Safety lead"),
        ("<planned verification method>", "Test the controls."),
        ("<the controls it verifies>", "Every risk control."),
        (
            "<production or post-production activity>",
            "Review field reports.",
        ),
        ("<how it is carried out>", "Quarterly review."),
    ]
    .iter()
    .fold(skeleton, |text, (placeholder, value)| {
        text.replace(placeholder, value)
    })
    .replacen(
        "\n---\n<!--",
        "\nrelationships:\n  - target: POL-1\n    type: references\n---\n<!--",
        1,
    )
}

/// Run `quoin dossier generate --doc risk-management-plan` over a scope holding
/// the policy and, when given, the plan text.
fn generate(plan: Option<&str>) -> (tempfile::TempDir, Output) {
    let work = tempfile::tempdir().unwrap();
    let records = work.path().join("spec");
    fs::create_dir_all(&records).unwrap();
    fs::write(records.join("policy.md"), POLICY).unwrap();
    if let Some(plan) = plan {
        fs::write(records.join("plan.md"), plan).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["dossier", "generate", "--version", "1.0", "--json"])
        .args(["--doc", "risk-management-plan", "--scope", "spec"])
        .arg("--module")
        .arg(module_root())
        .current_dir(work.path())
        .output()
        .expect("quoin starts");
    (work, output)
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("quoin exits with a status")
}

#[trace("FR-191-AC-22")]
#[test]
fn the_plan_document_makes_generate_exit_1_only_while_no_plan_record_exists() {
    let (without, output) = generate(None);
    assert_eq!(code(&output), 1);
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["documents"][0]["gaps"][0]["gap_kind"],
        "no_plan_record"
    );
    assert!(without.path().join("dossier/DOS-RMP.md").is_file());

    let (with, output) = generate(Some(&plan_document()));
    assert_eq!(
        code(&output),
        0,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let markdown = fs::read_to_string(with.path().join("dossier/DOS-RMP.md")).unwrap();
    assert!(markdown.contains("### RMP-001"));
    assert!(!markdown.contains("no_plan_record"));
}

#[trace("FR-200-AC-14")]
#[test]
fn an_unusable_plan_makes_generate_exit_3_name_the_plan_and_write_nothing() {
    let good = plan_document();
    let blank_cell = good.replace(
        "| Keep the risk file current. | Safety lead |",
        "| Keep the risk file current. |  |",
    );
    let missing_table = good.replace("## Scope\n", "## Not scope\n");
    let empty_table = good.replace("| Brake controller. |\n", "");
    for broken in [blank_cell, missing_table, empty_table] {
        assert_ne!(broken, good);
        let (work, output) = generate(Some(&broken));
        assert_eq!(
            code(&output),
            3,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let source = report["source_failure"]["source"].as_str().unwrap();
        assert!(source.contains("risk management plan"), "{source}");
        assert!(!work.path().join("dossier").exists());
    }
}
