// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin evidence audit --class-policy` and `quoin matrix --class-policy`
//! against a real repository and the real Quire engine (FR-160).
//!
//! The class is stated in a requirement's frontmatter and travels to the audit
//! through the engine's `carried` field; the policy is a file the test writes.
//! Nothing in quoin supplies a class or a floor (FR-160-AC-13).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const MODULE: &str = r"name: quoin-cli-class-fixture
version: 0.1.0
description: Fixture module for the class policy command tests
artifact_types:
  - name: FR
    grammar_ref: iso-spec-core
    frontmatter_schema_ref: schemas/fr.schema.json
    defaults:
      id_pattern: FR-{next:03d}
traceability:
  carried_fields:
    - name: class
      from: [FR]
      key: safety_class
  trace_targets:
    - name: acceptance-criterion
      archetype: FR
      section: Acceptance Criteria
      id_column: ID
  obligations:
    - name: acceptance-criterion
      target: acceptance-criterion
      statement_column: Criteria
      method_column: Verification
verification_catalog:
  unit-testing:
    name: Unit testing
    class: Test
    definition: Exercise the unit.
    evidence_kind: test-run
";

const SCHEMA: &str = r#"{
  "type": "object",
  "properties": { "safety_class": { "enum": ["class-1", "class-2", "class-3"] } }
}"#;

const CLASSED: &str = "---
id: FR-001
type: FR
title: Classed
safety_class: class-2
---

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-001-AC-1 | Every stored payload shall round-trip through the reader unchanged. |  |
| FR-001-AC-2 | The reader shall refuse a payload over its ceiling. |  |
";

const CLASSED_WITH_DESIGN: &str = "---
id: FR-001
type: FR
title: Classed
safety_class: class-2
relationships:
  - target: DESIGN-001
    type: designs
---

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-001-AC-1 | Every stored payload shall round-trip through the reader unchanged. |  |
| FR-001-AC-2 | The reader shall refuse a payload over its ceiling. |  |
";

const DESIGN: &str = "---
id: DESIGN-001
type: Design
title: Design for FR-001
---

# Design
";

const UNCLASSED: &str = "---
id: FR-002
type: FR
title: Unclassed
---

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-002-AC-1 | The reader shall name the refused field. |  |
";

const POLICY: &str = "class_field: class
classes:
  class-2:
    required_methods: [unit-testing]
    obligations:
      - id: design
        kind: design-trace
      - id: unit-evidence
        kind: evidence
        method: unit-testing
    rationale: recorded by the fixture's author
";

struct Fixture {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Fixture {
    fn repo(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.repo().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    fn quoin(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(arguments)
            .current_dir(self.repo())
            .env("IX_HOME", self.home.path())
            .env("GIT_CEILING_DIRECTORIES", self.repo().parent().unwrap())
            .output()
            .expect("the native quoin binary runs")
    }

    fn ok(&self, arguments: &[&str]) -> String {
        let output = self.quoin(arguments);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn files(&self) -> Vec<PathBuf> {
        fn walk(directory: &Path, into: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(directory).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, into);
                } else {
                    into.push(path);
                }
            }
        }
        let mut found = Vec::new();
        walk(self.repo(), &mut found);
        found.sort();
        found
    }
}

fn repository() -> Fixture {
    let fixture = Fixture {
        dir: tempfile::tempdir().unwrap(),
        home: tempfile::tempdir().unwrap(),
    };
    fixture.write("manifest.yaml", MODULE);
    fixture.write("schemas/fr.schema.json", SCHEMA);
    fixture.write("spec/FR-001-classed.md", CLASSED);
    fixture.write("spec/FR-002-unclassed.md", UNCLASSED);
    fixture.write("policy.yaml", POLICY);
    fixture
}

fn enable_design_relation(fixture: &Fixture) {
    let module = MODULE
        .replace(
            "      id_pattern: FR-{next:03d}\ntraceability:\n",
            "      id_pattern: FR-{next:03d}\n  - name: Design\n    defaults:\n      id_pattern: DESIGN-{next:03d}\nedge_types:\n  designs: { description: requirement is realized by design, category: traceability, inverse: designed_by }\ntraceability:\n",
        )
        .replace(
            "  carried_fields:\n",
            "  required_relations:\n    - name: fr-has-design\n      from: FR\n      edges: [designs]\n      to: [Design]\n      direction: outgoing\n      check: fr-needs-design\n  design_relations: [fr-has-design]\n  carried_fields:\n",
        );
    fixture.write("manifest.yaml", &module);
}

fn audit_json(fixture: &Fixture, extra: &[&str]) -> Value {
    let mut arguments = vec!["evidence", "audit", "--module", ".", "--json"];
    arguments.extend_from_slice(extra);
    serde_json::from_str(&fixture.ok(&arguments)).unwrap()
}

fn class_findings(audit: &Value) -> Vec<(String, String)> {
    audit["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["kind"].as_str().unwrap().starts_with("class-"))
        .map(|finding| {
            (
                finding["obligation"].as_str().unwrap().to_owned(),
                finding["kind"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// Without `--class-policy` neither command reads a class; with it, the added
/// output is the class column or key, the unclassified count and the findings.
#[trace("FR-160-AC-1")]
#[test]
fn tc_160_300_without_a_policy_nothing_class_appears() {
    let fixture = repository();
    let audit = audit_json(&fixture, &[]);
    for key in ["classReport", "unclassified", "classIndependence"] {
        assert!(audit.get(key).is_none(), "{key} in {audit}");
    }
    assert!(class_findings(&audit).is_empty());
    let matrix: Value =
        serde_json::from_str(&fixture.ok(&["matrix", "--module", ".", "--json"])).unwrap();
    assert!(matrix.get("unclassified").is_none(), "{matrix}");
    assert!(!fixture.ok(&["matrix", "--module", "."]).contains("Class"));
}

/// The class travels from frontmatter through the engine to the audit: the
/// classed requirement's obligations are checked against its floors, and the
/// unclassed one is counted and shown `unassigned`.
#[trace("FR-160-AC-2", "FR-160-AC-6", "FR-160-AC-11")]
#[test]
fn tc_160_301_the_audit_applies_the_stated_class_and_reports_the_unclassified() {
    let fixture = repository();
    let audit = audit_json(&fixture, &["--class-policy", "policy.yaml"]);
    assert_eq!(
        class_findings(&audit),
        [
            ("FR-001-AC-1".to_owned(), "class-method-missing".to_owned()),
            ("FR-001-AC-2".to_owned(), "class-method-missing".to_owned()),
        ],
        "{audit}"
    );
    assert_eq!(audit["unclassified"], 1);
    assert_eq!(audit["classReport"]["classes"]["FR-001-AC-1"], "class-2");
    assert_eq!(audit["classReport"]["unassigned"][0], "FR-002-AC-1");
    assert!(audit["classReport"]["classes"].get("FR-002-AC-1").is_none());

    let text = fixture.ok(&[
        "evidence",
        "audit",
        "--module",
        ".",
        "--class-policy",
        "policy.yaml",
    ]);
    assert!(text.contains("FR-001-AC-1  class-2"), "{text}");
    assert!(text.contains("FR-002-AC-1  unassigned"), "{text}");
    assert!(text.contains("1 obligation(s) unclassified"), "{text}");
}

/// `quoin matrix` reports the same classes as the audit for the same inputs.
#[trace("FR-160-AC-11")]
#[test]
fn tc_160_302_the_matrix_shows_the_class_column_in_both_formats() {
    let fixture = repository();
    let matrix: Value = serde_json::from_str(&fixture.ok(&[
        "matrix",
        "--module",
        ".",
        "--json",
        "--class-policy",
        "policy.yaml",
    ]))
    .unwrap();
    assert_eq!(matrix["unclassified"], 1);
    let classes: Vec<(String, Option<String>)> = matrix["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|requirement| requirement["criteria"].as_array().unwrap())
        .map(|criterion| {
            (
                criterion["id"].as_str().unwrap().to_owned(),
                criterion["class"].as_str().map(ToOwned::to_owned),
            )
        })
        .collect();
    assert_eq!(
        classes,
        [
            ("FR-001-AC-1".to_owned(), Some("class-2".to_owned())),
            ("FR-001-AC-2".to_owned(), Some("class-2".to_owned())),
            ("FR-002-AC-1".to_owned(), None),
        ]
    );
    let markdown = fixture.ok(&["matrix", "--module", ".", "--class-policy", "policy.yaml"]);
    assert!(markdown.contains("| class-2 |"), "{markdown}");
    assert!(markdown.contains("| unassigned |"), "{markdown}");
}

/// A policy that cannot be applied is refused, naming the cause, and no audit
/// output is produced.
#[trace("FR-160-AC-8")]
#[test]
fn tc_160_303_a_policy_naming_an_unknown_class_is_refused_with_no_output() {
    let fixture = repository();
    fixture.write(
        "bad.yaml",
        "class_field: class\nclasses:\n  class-9:\n    rationale: nobody has class-9\n",
    );
    let output = fixture.quoin(&[
        "evidence",
        "audit",
        "--module",
        ".",
        "--json",
        "--class-policy",
        "bad.yaml",
    ]);
    assert_ne!(output.status.code(), Some(0));
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("class-9"), "{combined}");
    assert!(!combined.contains("\"findings\""), "{combined}");
}

/// Two runs print the same bytes and write nothing.
#[trace("FR-160-AC-12", "NFR-034")]
#[test]
fn tc_160_304_two_runs_are_identical_and_write_no_file() {
    let fixture = repository();
    let before = fixture.files();
    let first = fixture.ok(&[
        "evidence",
        "audit",
        "--module",
        ".",
        "--json",
        "--class-policy",
        "policy.yaml",
    ]);
    let second = fixture.ok(&[
        "evidence",
        "audit",
        "--module",
        ".",
        "--json",
        "--class-policy",
        "policy.yaml",
    ]);
    assert_eq!(first, second);
    assert_eq!(fixture.files(), before);
}

/// The unchanged FR-160 policy file drives both commands, and FR-161 preserves
/// its class rows and activity facts across formats and repeated runs.
#[trace(
    "FR-161-AC-1",
    "FR-161-AC-4",
    "FR-161-AC-5",
    "FR-161-AC-6",
    "FR-161-AC-9",
    "FR-161-CON-1",
    "NFR-034-M-1",
    "NFR-034-M-2",
    "NFR-034-M-3"
)]
#[test]
fn tc_161_001_class_obligations_reuses_the_audit_policy_without_writing() {
    let fixture = repository();
    let policy_before = std::fs::read(fixture.repo().join("policy.yaml")).unwrap();
    let files_before = fixture.files();
    let audit = audit_json(&fixture, &["--class-policy", "policy.yaml"]);
    assert_eq!(audit["classReport"]["classes"]["FR-001-AC-1"], "class-2");

    let json = fixture.ok(&[
        "class",
        "obligations",
        ".",
        "--module",
        ".",
        "--class-policy",
        "policy.yaml",
        "--format",
        "json",
    ]);
    let report: Value = serde_json::from_str(&json).unwrap();
    let requirement = report["requirements"].as_array().unwrap().first().unwrap();
    assert_eq!(requirement["class"], "class-2");
    let activities = requirement["activities"].as_array().unwrap();
    assert_eq!(activities[0]["kind"], "design-trace", "{report}");
    assert_eq!(
        activities[0]["reason"], "design join has no relations",
        "{report}"
    );
    assert_eq!(activities[0]["status"], "not-available");
    assert_eq!(activities[1]["kind"], "evidence", "{report}");
    assert_eq!(activities[1]["method"], "unit-testing", "{report}");
    assert_eq!(activities[1]["status"], "unmet");
    assert_eq!(
        activities[1]["missing"],
        serde_json::json!(["FR-001-AC-1", "FR-001-AC-2"])
    );
    assert_eq!(
        report["unassigned"],
        serde_json::json!(["spec/FR-002-unclassed.md"])
    );
    for key in ["score", "rank", "verdict", "approval", "recommendation"] {
        assert!(report.get(key).is_none(), "{report}");
    }
    let markdown = fixture.ok(&[
        "class",
        "obligations",
        ".",
        "--module",
        ".",
        "--class-policy",
        "policy.yaml",
    ]);
    assert!(
        markdown.contains(
            "| unit-evidence | evidence | unit-testing | unmet |  | FR-001-AC-1, FR-001-AC-2 |"
        ),
        "{markdown}"
    );
    assert!(
        markdown.contains(
            "| design | design-trace |  | not-available | design join has no relations |  |"
        ),
        "{markdown}"
    );
    let repeat = fixture.ok(&[
        "class",
        "obligations",
        ".",
        "--module",
        ".",
        "--class-policy",
        "policy.yaml",
        "--format",
        "json",
    ]);
    assert_eq!(json, repeat);
    assert_eq!(
        std::fs::read(fixture.repo().join("policy.yaml")).unwrap(),
        policy_before
    );
    assert_eq!(fixture.files(), files_before);
}

/// The shipped command assembles the real FR-086 design join and FR-160 evidence audit.
#[trace("FR-161-AC-1")]
#[test]
fn tc_161_002_command_joins_covered_design_and_present_evidence() {
    let fixture = repository();
    enable_design_relation(&fixture);
    fixture.write("spec/FR-001-classed.md", CLASSED_WITH_DESIGN);
    fixture.write("spec/DESIGN-001.md", DESIGN);
    fixture.write(
        "spec/evidence/bindings.json",
        r#"{"schemaVersion":1,"bindings":[{"obligation":"FR-001-AC-1","suite":"SUITE-1","symbols":["tests::classed"]},{"obligation":"FR-001-AC-2","suite":"SUITE-1","symbols":["tests::classed"]}]}"#,
    );
    fixture.write(
        "spec/evidence/runs/SUITE-1/run.json",
        r#"{"schemaVersion":1,"suite":"SUITE-1","tool":"fixture","evidenceKind":"test-run","timestamp":"2026-11-13T00:00:00Z","entries":[{"symbol":"tests::classed","outcome":"pass","traceIds":["FR-001-AC-1","FR-001-AC-2"]}]}"#,
    );

    let json = fixture.ok(&[
        "class",
        "obligations",
        ".",
        "--module",
        ".",
        "--class-policy",
        "policy.yaml",
        "--format",
        "json",
    ]);
    let report: Value = serde_json::from_str(&json).unwrap();
    let requirement = report["requirements"].as_array().unwrap().first().unwrap();
    let activities = requirement["activities"].as_array().unwrap();
    assert_eq!(activities[0]["kind"], "design-trace", "{report}");
    assert_eq!(activities[0]["status"], "met", "{report}");
    assert_eq!(activities[1]["kind"], "evidence", "{report}");
    assert_eq!(activities[1]["method"], "unit-testing", "{report}");
    assert_eq!(activities[1]["status"], "met", "{report}");

    let markdown = fixture.ok(&[
        "class",
        "obligations",
        ".",
        "--module",
        ".",
        "--class-policy",
        "policy.yaml",
    ]);
    assert!(
        markdown.contains("| design | design-trace |  | met |  |  |"),
        "{markdown}"
    );
    assert!(
        markdown.contains("| unit-evidence | evidence | unit-testing | met |  |  |"),
        "{markdown}"
    );
}
