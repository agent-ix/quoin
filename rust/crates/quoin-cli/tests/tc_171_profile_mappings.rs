// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
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
            "  - id: AC-1\n    force: mandatory\n  - id: AC-2\n    force: mandatory\n  - id: AC-3\n    force: mandatory\n    applicability: {op: eq, dimension: baseline, value: high}\n  - id: PE-3\n    force: mandatory\n  - id: CP-9\n    force: mandatory\n    applicability: {op: eq, dimension: data_class, value: restricted}\n",
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

fn mapping_rows(text: &str) -> Vec<(String, String)> {
    let section = text.split("\n## Mappings\n").nth(1).unwrap_or("");
    let section = section.split("\n## ").next().unwrap_or("");
    section
        .lines()
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| {
            let cells: Vec<_> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            assert_eq!(
                cells.len(),
                2,
                "mapping row has Set and Clause cells: {line}"
            );
            (cells[0].to_owned(), cells[1].to_owned())
        })
        .collect()
}

fn emitted_mapping_rows(profile: &str) -> Vec<(String, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/security-analysis-profile")
        .join(profile)
        .join("output/security");
    let mut files: Vec<_> = std::fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("{}: {error}", root.display()))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .collect();
    files.sort();
    files
        .iter()
        .flat_map(|path| {
            let text = std::fs::read_to_string(path).unwrap();
            let mut rows = mapping_rows(&text);
            if text.contains("\n## Set\n") {
                let set = text
                    .split("\n## Set\n")
                    .nth(1)
                    .unwrap()
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .unwrap();
                let clause = text
                    .split("\n## Clause\n")
                    .nth(1)
                    .unwrap()
                    .lines()
                    .find(|line| !line.trim().is_empty())
                    .unwrap()
                    .trim_matches('`')
                    .to_owned();
                rows.push((set.trim_matches('`').to_owned(), clause));
            }
            rows
        })
        .collect()
}

fn emitted_text(profile: &str, relative: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/fixtures/security-analysis-profile")
            .join(profile)
            .join("output")
            .join(relative),
    )
    .unwrap()
}

fn source_profile_fixture(profile: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/fixtures/security-analysis-profile")
            .join(profile)
            .join("repo/.ix/source-profile.yaml"),
    )
    .unwrap()
}

fn assert_emitted_mappings_are_binding(report: &Value, profile: &str) {
    for (key, clause_id) in emitted_mapping_rows(profile) {
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
            .unwrap_or_else(|| panic!("emitted mapping uses absent set {key}"));
        assert!(
            binding_ids(set).contains(&clause_id),
            "emitted mapping {key} {clause_id} is not binding in evaluation"
        );
    }
}

#[trace(
    "FR-171-AC-8",
    "TC-003",
    "FR-171-AC-9",
    "TC-004",
    "FR-171-AC-12",
    "TC-005",
    "FR-171-AC-14",
    "TC-006"
)]
#[test]
fn fixture_a_skill_output_maps_only_binding_profile_clauses_and_preserves_context() {
    let fixture = fixture(
        "web-app-baseline",
        &[("nist", "sp800-53", "5.2.0"), ("owasp", "asvs", "5.0.0")],
        "  baseline: low\n  level: \"1\"\n",
    );
    let selection_before = std::fs::read(fixture.repo.join(".ix/source-profile.yaml")).unwrap();
    assert_eq!(
        selection_before,
        source_profile_fixture("a").as_bytes(),
        "evaluation input matches the profile used for the skill-emitted Fixture A artifacts"
    );
    let report = evaluate(&fixture);
    assert_eq!(report["context"]["baseline"], "low");
    assert_eq!(report["context"]["level"], "1");
    assert_eq!(keys(&report), ["nist/sp800-53@5.2.0", "owasp/asvs@5.0.0"]);

    let mappings = emitted_mapping_rows("a");
    assert_emitted_mappings_are_binding(&report, "a");
    for clause in ["AC-3", "CP-9", "V1.1.2", "V1.1.3"] {
        assert!(
            !mappings.iter().any(|(_, id)| id == clause),
            "{clause} was mapped"
        );
    }
    let review = emitted_text(
        "a",
        "reviews/2026-11-13-security-control-mapping-gap-analysis.md",
    );
    assert!(review.contains("CP-9") && review.contains("`data_class`"));
    assert!(review.contains("V1.1.3") && review.contains("`chapter`"));
    assert_eq!(
        std::fs::read(fixture.repo.join(".ix/source-profile.yaml")).unwrap(),
        selection_before
    );
}

#[trace("FR-171-AC-8", "TC-003", "FR-171-AC-13", "TC-007")]
#[test]
fn fixture_b_owasp_only_profile_has_no_nist_mapping() {
    let fixture = fixture(
        "owasp-only",
        &[("owasp", "asvs", "5.0.0")],
        "  level: \"1\"\n",
    );
    assert_eq!(
        std::fs::read(fixture.repo.join(".ix/source-profile.yaml")).unwrap(),
        source_profile_fixture("b").as_bytes(),
        "evaluation input matches the profile used for the skill-emitted Fixture B artifacts"
    );
    let report = evaluate(&fixture);
    assert_eq!(keys(&report), ["owasp/asvs@5.0.0"]);
    let mappings = emitted_mapping_rows("b");
    assert_emitted_mappings_are_binding(&report, "b");
    assert!(!mappings.is_empty());
    assert!(mappings.iter().all(|(key, _)| key == "owasp/asvs@5.0.0"));
    assert!(!mappings.iter().any(|(key, _)| key.starts_with("nist/")));
}

#[trace("FR-171-AC-10", "TC-008")]
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
    let mappings: Vec<_> = emitted_mapping_rows("a")
        .into_iter()
        .filter(|(key, _)| key == "nist/sp800-53@5.2.0")
        .collect();
    assert!(!mappings.is_empty(), "evaluated set mappings continue");
    for (_, clause) in mappings {
        assert!(
            binding_ids(&sets[0]).contains(&clause),
            "an emitted mapping from the evaluated set is binding"
        );
    }
    let skill = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../skills/spec-security-analysis/SKILL.md"
    ))
    .unwrap();
    assert!(skill.contains("report `not mapped, set not installed`"));
    assert!(skill.contains("continue mapping other evaluated sets"));
}

#[trace("FR-171-AC-11", "TC-009")]
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

#[trace("FR-171-AC-15", "TC-010")]
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
