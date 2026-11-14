// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

//! Synthetic profile-evaluation fixtures exercise the mapping contract in
//! FR-171 without vendoring clause text from spec-objects-security.

mod pack_support;

use std::fmt::Write as _;
use std::path::PathBuf;

use ix_trace_rs::trace;
use pack_support::{scratch, text};
use serde_json::Value;

struct Fixture {
    root: tempfile::TempDir,
    home: PathBuf,
    repo: PathBuf,
}

fn fixture(profile_name: &str, selected: &[(&str, &str, &str)], context: &str) -> Fixture {
    let root = scratch();
    let base = root.path().canonicalize().unwrap();
    let home = base.join("home");
    let repo = base.join("repo");
    let module = base.join("module");
    std::fs::create_dir_all(module.join("profiles")).unwrap();
    std::fs::create_dir_all(repo.join(".ix")).unwrap();
    std::fs::write(
        module.join("manifest.yaml"),
        "name: synthetic-security\nversion: 0.1.0\nclause_sets:\n  - nist.yaml\n  - owasp.yaml\n",
    )
    .unwrap();
    for (file, authority, id, clauses) in [
        (
            "nist.yaml",
            "nist",
            "sp800-53",
            "  - id: AC-1\n    force: mandatory\n  - id: AC-2\n    force: mandatory\n    applicability: {op: eq, dimension: baseline, value: high}\n  - id: AC-3\n    force: mandatory\n    applicability: {op: eq, dimension: data_class, value: restricted}\n",
        ),
        (
            "owasp.yaml",
            "owasp",
            "asvs",
            "  - id: V1.1.1\n    force: mandatory\n  - id: V1.1.2\n    force: mandatory\n    applicability: {op: eq, dimension: baseline, value: high}\n  - id: V1.1.3\n    force: mandatory\n    applicability: {op: eq, dimension: data_class, value: restricted}\n",
        ),
    ] {
        std::fs::write(
            module.join(file),
            format!(
                "schemaVersion: clause-set-v1\nauthority: {authority}\nid: {id}\ntitle: Synthetic {id}\nversion: \"{}\"\nrights:\n  structure: original\n  text: none\nclassificationDimensions:\n  baseline:\n    values: [low, high]\n  level:\n    values: [\"1\", \"2\"]\n  data_class:\n    values: [public, restricted]\nclauses:\n{clauses}",
                if authority == "nist" { "5.2.0" } else { "5.0.0" }
            ),
        )
        .unwrap();
    }
    let mut profile = format!(
        "schemaVersion: source-profile-v1\nname: {profile_name}\ntitle: Synthetic {profile_name}\ndescription: Fixture profile\nclauseSets:\n"
    );
    for (authority, id, version) in selected {
        writeln!(
            profile,
            "  - {{authority: {authority}, id: {id}, version: \"{version}\"}}"
        )
        .unwrap();
    }
    std::fs::write(
        module.join("profiles").join(format!("{profile_name}.yaml")),
        profile,
    )
    .unwrap();
    std::fs::write(
        repo.join(".ix/source-profile.yaml"),
        format!("profile: {profile_name}\ncontext:\n{context}"),
    )
    .unwrap();
    let semantic = pack_support::semantic_root(&home);
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["module", "install", &format!("path:{}", module.display())])
        .current_dir(&repo)
        .env("IX_HOME", &home)
        .env("QUOIN_SEMANTIC_ROOT", semantic)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output));
    Fixture { root, home, repo }
}

fn evaluate(fixture: &Fixture) -> Value {
    let semantic = pack_support::semantic_root(&fixture.home);
    let out_dir = fixture.root.path().join("evaluation");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["profile", "evaluate", "--out-dir"])
        .arg(&out_dir)
        .current_dir(&fixture.repo)
        .env("IX_HOME", &fixture.home)
        .env("QUOIN_SEMANTIC_ROOT", semantic)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output));
    let report: Value = serde_json::from_slice(&output.stdout).expect("evaluation document");
    assert_eq!(report["schemaVersion"], "source-profile-evaluation-v1");
    report
}

fn binding_ids(set: &Value) -> Vec<String> {
    set["report"]["clauses"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|clause| clause["outcome"] == "binding")
        .map(|clause| clause["clauseId"].as_str().unwrap().to_owned())
        .collect()
}

fn keys(report: &Value) -> Vec<String> {
    report["sets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|set| set["status"] != "unavailable")
        .map(|set| {
            format!(
                "{}/{}@{}",
                set["key"]["authority"].as_str().unwrap(),
                set["key"]["id"].as_str().unwrap(),
                set["key"]["version"].as_str().unwrap()
            )
        })
        .collect()
}

#[trace("FR-171-AC-8", "FR-171-AC-9", "FR-171-AC-12")]
#[test]
fn fixture_a_evaluation_limits_mappings_to_binding_profile_clauses() {
    let fixture = fixture(
        "web-app-baseline",
        &[("nist", "sp800-53", "5.2.0"), ("owasp", "asvs", "5.0.0")],
        "  baseline: low\n  level: \"1\"\n",
    );
    let selection_before = std::fs::read(fixture.repo.join(".ix/source-profile.yaml")).unwrap();
    let report = evaluate(&fixture);
    assert_eq!(report["context"]["baseline"], "low");
    assert_eq!(report["context"]["level"], "1");
    assert_eq!(keys(&report), ["nist/sp800-53@5.2.0", "owasp/asvs@5.0.0"]);

    // These synthetic rows stand for the skill's mapping output. Each is
    // checked against the evaluator's per-set oracle, including negative cases.
    let mappings = [
        ("nist/sp800-53@5.2.0", "AC-1"),
        ("owasp/asvs@5.0.0", "V1.1.1"),
    ];
    for (key, clause) in mappings {
        let set = report["sets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|set| {
                format!(
                    "{}/{}@{}",
                    set["key"]["authority"].as_str().unwrap(),
                    set["key"]["id"].as_str().unwrap(),
                    set["key"]["version"].as_str().unwrap()
                ) == key
            })
            .unwrap();
        assert!(binding_ids(set).iter().any(|id| id == clause));
    }
    assert!(
        !mappings
            .iter()
            .any(|(_, clause)| *clause == "AC-2" || *clause == "V1.1.2")
    );
    let unresolved = report["sets"][0]["report"]["clauses"].as_array().unwrap();
    assert!(
        unresolved
            .iter()
            .any(|c| c["clauseId"] == "AC-3" && c["outcome"] == "unresolved")
    );
    assert_eq!(
        std::fs::read(fixture.repo.join(".ix/source-profile.yaml")).unwrap(),
        selection_before
    );
}

#[trace("FR-171-AC-8", "FR-171-AC-13")]
#[test]
fn fixture_b_owasp_only_profile_has_no_nist_mapping() {
    let fixture = fixture(
        "owasp-only",
        &[("owasp", "asvs", "5.0.0")],
        "  baseline: low\n",
    );
    let report = evaluate(&fixture);
    assert_eq!(keys(&report), ["owasp/asvs@5.0.0"]);
    let mappings = [("owasp/asvs@5.0.0", "V1.1.1")];
    assert!(
        mappings
            .iter()
            .all(|(key, clause)| key.starts_with("owasp/")
                && binding_ids(&report["sets"][0]).contains(&clause.to_string()))
    );
    assert!(!mappings.iter().any(|(key, _)| key.starts_with("nist/")));
}

#[trace("FR-171-AC-10")]
#[test]
fn unavailable_selected_set_does_not_stop_an_evaluated_binding_set() {
    let fixture = fixture(
        "profile-with-unavailable",
        &[
            ("nist", "sp800-53", "5.2.0"),
            ("missing.invalid", "pack", "1"),
        ],
        "  baseline: low\n",
    );
    let report = evaluate(&fixture);
    let sets = report["sets"].as_array().unwrap();
    assert_eq!(sets.len(), 2);
    assert_eq!(sets[0]["status"], "evaluated");
    assert!(binding_ids(&sets[0]).contains(&"AC-1".to_owned()));
    assert_eq!(sets[1]["status"], "unavailable");
    assert!(sets[1]["report"].is_null());
    let skill = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../skills/spec-security-analysis/SKILL.md"
    ))
    .unwrap();
    assert!(skill.contains("report `not mapped, set not installed`"));
    assert!(skill.contains("continue mapping other evaluated sets"));
}

#[trace("FR-171-AC-11")]
#[test]
fn no_profile_instruction_stops_before_mapping_and_does_not_claim_defaults_are_installed() {
    let fixture = fixture("web-app-baseline", &[("nist", "sp800-53", "5.2.0")], "");
    let selection = fixture.repo.join(".ix/source-profile.yaml");
    std::fs::remove_file(&selection).unwrap();
    let out_dir = fixture.root.path().join("missing-profile-evaluation");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["profile", "evaluate", "--out-dir"])
        .arg(&out_dir)
        .current_dir(&fixture.repo)
        .env("IX_HOME", &fixture.home)
        .env(
            "QUOIN_SEMANTIC_ROOT",
            pack_support::semantic_root(&fixture.home),
        )
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output));
    assert!(
        text(&output).contains("no source profile is selected"),
        "{}",
        text(&output)
    );
    assert!(!out_dir.exists(), "no evaluation document is emitted");
    assert!(
        !fixture.repo.join("spec/security").exists(),
        "no mapping artifacts are written"
    );

    let skill = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../skills/spec-security-analysis/SKILL.md"
    ))
    .unwrap();
    for required in [
        "stop before mapping",
        "ask the user to create it",
        "`web-app-baseline`, `data-platform-baseline`, and `minimal`",
        "do not imply they are currently shipped or selectable",
        "never default to all sources",
        "Do not write or change `.ix/source-profile.yaml`",
    ] {
        assert!(skill.contains(required), "skill misses {required}");
    }
}

#[trace("FR-171-AC-9", "FR-171-AC-10", "FR-171-AC-14", "FR-171-AC-15")]
#[test]
fn skill_specifies_unresolved_unavailable_immutability_and_fail_closed_behavior() {
    let skill = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../skills/spec-security-analysis/SKILL.md"
    ))
    .unwrap();
    for required in [
        "name the missing context dimension in a SpecReview",
        "author no control/exclusion for that clause",
        "report `not mapped, set not installed`",
        "continue mapping other evaluated sets",
        "Do not write or change `.ix/source-profile.yaml` or any `context` value",
        "If evaluation exits nonzero, emits no document, or emits a document that fails schema validation",
        "stop without mapping or authoring controls/exclusions",
    ] {
        assert!(skill.contains(required), "skill misses {required}");
    }
}
