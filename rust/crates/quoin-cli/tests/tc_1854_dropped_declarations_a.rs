// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Commands computing over the merged traceability model refuse when a
//! module's declaration cannot load (FR-029-AC-21, FR-029-AC-23): exit 2,
//! `QQ-1022` and each dropped declaration on stderr, nothing on stdout, no
//! output document written. The same scope without the dropped declaration
//! runs normally.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MODULE: &str = r"name: quoin-cli-drop-fixture
version: 0.1.0
description: Fixture module for the dropped-declaration tests
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

/// Appended under `traceability:`: names an archetype nothing declares.
const BAD_RULE: &str = "  flow_rules:
    - name: bad
      subject: [nowhere]
      match: [{path: [], field: id, present: true}]
      check: bad-check
      rationale: Names an archetype nothing declares.
";

const DROP_LINE: &str = "module=quoin-cli-drop-fixture kind=flow_rule item=bad";

const SCHEMA: &str = r#"{
  "type": "object",
  "properties": { "safety_class": { "enum": ["class-1", "class-2"] } }
}"#;

const SPEC: &str = "---
id: FR-001
type: FR
title: Classed
safety_class: class-2
---

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-001-AC-1 | Every stored payload shall round-trip through the reader unchanged. |  |
";

const POLICY: &str = "class_field: class
classes:
  class-2:
    required_methods: [unit-testing]
    obligations:
      - id: unit-evidence
        kind: evidence
        method: unit-testing
    rationale: recorded by the fixture's author
";

const RESULTS: &str = r#"{"entries":[{"symbol":"a","outcome":"pass","traceIds":["FR-001-AC-1"]}]}"#;

struct Scope {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
}

/// A directory that is itself the module, with `spec/`, optionally carrying
/// the dropped declaration.
fn scope(dropped: bool) -> Scope {
    let scope = Scope {
        dir: tempfile::tempdir().unwrap(),
        home: tempfile::tempdir().unwrap(),
    };
    let module = if dropped {
        MODULE.replacen("traceability:\n", &format!("traceability:\n{BAD_RULE}"), 1)
    } else {
        MODULE.to_owned()
    };
    write(scope.dir.path(), "manifest.yaml", &module);
    write(scope.dir.path(), "schemas/fr.schema.json", SCHEMA);
    write(scope.dir.path(), "spec/FR-001-classed.md", SPEC);
    write(scope.dir.path(), "policy.yaml", POLICY);
    write(scope.home.path(), "results.json", RESULTS);
    scope
}

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn run(scope: &Scope, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(args)
        .current_dir(scope.dir.path())
        .env("IX_HOME", scope.home.path())
        .env(
            "GIT_CEILING_DIRECTORIES",
            scope.dir.path().parent().unwrap(),
        )
        .output()
        .expect("the native quoin binary runs")
}

fn files(root: &Path) -> Vec<PathBuf> {
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
    walk(root, &mut found);
    found.sort();
    found
}

/// Exit 2, `QQ-1022` and the drop line on stderr, stdout empty, nothing
/// written under the scope.
fn assert_refused(scope: &Scope, args: &[&str]) {
    let before = files(scope.dir.path());
    let output = run(scope, args);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
    assert!(stderr.contains("QQ-1022"), "{args:?}: {stderr}");
    assert!(stderr.contains(DROP_LINE), "{args:?}: {stderr}");
    assert!(
        output.stdout.is_empty(),
        "{args:?}: stdout {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(files(scope.dir.path()), before, "{args:?} wrote a file");
}

/// Exit 0 and non-empty stdout, with no dropped-declaration diagnostic.
fn assert_clean(scope: &Scope, args: &[&str]) -> String {
    let output = run(scope, args);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{args:?}: {stderr}");
    assert!(!stderr.contains("QQ-1022"), "{args:?}: {stderr}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.trim().is_empty(), "{args:?} printed nothing");
    stdout
}

const MATRIX: &[&str] = &["matrix", "--module", ".", "--json"];
const AUDIT: &[&str] = &["evidence", "audit", "--module", ".", "--json"];
const ADVISE: &[&str] = &["advise", "--module", ".", "--json"];
const CLASS: &[&str] = &[
    "class",
    "obligations",
    ".",
    "--module",
    ".",
    "--class-policy",
    "policy.yaml",
    "--format",
    "json",
];

fn record_args(scope: &Scope) -> Vec<String> {
    let results = scope.home.path().join("results.json");
    [
        "evidence",
        "record",
        "--repo",
        ".",
        "--module",
        ".",
        "--suite",
        "SUITE-1",
        "--tool",
        "custom 1.0.0",
        "--results",
        results.to_str().unwrap(),
        "--timestamp",
        "2026-11-23T00:00:00Z",
        "--json",
    ]
    .map(ToOwned::to_owned)
    .to_vec()
}

#[trace("FR-029-AC-21")]
#[test]
fn matrix_refuses_a_dropped_declaration() {
    assert_refused(&scope(true), MATRIX);
}

#[trace("FR-029-AC-23")]
#[test]
fn matrix_runs_without_the_dropped_declaration() {
    assert!(assert_clean(&scope(false), MATRIX).contains("FR-001-AC-1"));
}

#[trace("FR-029-AC-21")]
#[test]
fn evidence_audit_refuses_a_dropped_declaration() {
    assert_refused(&scope(true), AUDIT);
}

#[trace("FR-029-AC-23")]
#[test]
fn evidence_audit_runs_without_the_dropped_declaration() {
    assert!(assert_clean(&scope(false), AUDIT).contains("FR-001-AC-1"));
}

#[trace("FR-029-AC-21")]
#[test]
fn evidence_record_refuses_a_dropped_declaration_and_writes_nothing() {
    let scope = scope(true);
    let args = record_args(&scope);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    assert_refused(&scope, &args);
    assert!(!scope.dir.path().join("spec/evidence").exists());
}

#[trace("FR-029-AC-23")]
#[test]
fn evidence_record_writes_the_record_without_the_dropped_declaration() {
    let scope = scope(false);
    let args = record_args(&scope);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    assert_clean(&scope, &args);
    assert!(
        scope
            .dir
            .path()
            .join("spec/evidence/bindings.json")
            .is_file()
    );
    assert!(scope.dir.path().join("spec/evidence/runs/SUITE-1").is_dir());
}

#[trace("FR-029-AC-21")]
#[test]
fn advise_refuses_a_dropped_declaration() {
    assert_refused(&scope(true), ADVISE);
}

#[trace("FR-029-AC-23")]
#[test]
fn advise_runs_without_the_dropped_declaration() {
    assert_clean(&scope(false), ADVISE);
}

#[trace("FR-029-AC-21")]
#[test]
fn class_obligations_refuses_a_dropped_declaration() {
    assert_refused(&scope(true), CLASS);
}

#[trace("FR-029-AC-23")]
#[test]
fn class_obligations_runs_without_the_dropped_declaration() {
    assert!(assert_clean(&scope(false), CLASS).contains("FR-001"));
}

/// A fixed anomaly with a `Criterion` row makes `anomaly check` read the
/// merged coverage model; the fixture is the document
/// `spec-artifacts-process` ships, copied into the temp scope as the
/// existing anomaly tests do.
fn anomaly_scope(dropped: bool) -> Scope {
    let scope = scope(dropped);
    let root = std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-artifacts-process"),
        PathBuf::from,
    );
    let text = std::fs::read_to_string(
        root.join("tests/fixtures/anomaly/valid/closed-fixed-criterion-only.md"),
    )
    .expect("the spec-artifacts-process checkout holds the anomaly fixtures");
    write(scope.dir.path(), "AN-001.md", &text);
    // The `hazard` object type lives in its own module, as in production; without
    // it `anomaly check` reports a missing-module finding and exits 1.
    write(
        scope.dir.path(),
        "hazard-module/manifest.yaml",
        "name: spec-objects-safety\nversion: 0.0.0\nobject_types:\n- name: hazard\n",
    );
    scope
}

const ANOMALY: &[&str] = &[
    "anomaly",
    "check",
    ".",
    "--module",
    ".",
    "--module",
    "hazard-module",
];

#[trace("FR-029-AC-21")]
#[test]
fn anomaly_check_refuses_a_dropped_declaration() {
    assert_refused(&anomaly_scope(true), ANOMALY);
}

#[trace("FR-029-AC-23")]
#[test]
fn anomaly_check_runs_without_the_dropped_declaration() {
    let scope = anomaly_scope(false);
    let output = run(&scope, ANOMALY);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "anomaly check: {stderr}");
    assert!(!output.stdout.is_empty(), "anomaly check printed nothing");
    assert!(!stderr.contains("QQ-1022"), "{stderr}");
}
