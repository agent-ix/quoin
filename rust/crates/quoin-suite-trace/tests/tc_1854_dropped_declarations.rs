// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! `quoin-suite-trace` over a scope that lost a traceability declaration is
//! refused with exit 2, `QQ-1022` and the dropped declaration on stderr and no
//! `--out` file; the same scope without it annotates the report (FR-029-AC-21,
//! FR-029-AC-23; PLAT-1854).
//!
//! Provenance: PLAT-1854.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;

/// The line the refusal names for the dropped flow rule.
const DROP_LINE: &str = "module=quoin-suite-trace-dropped-fixture kind=flow_rule item=bad";

const MANIFEST: &str = r##"name: quoin-suite-trace-dropped-fixture
version: 0.1.0
description: Fixture module for the dropped-declaration refusal
artifact_types:
  - name: FR
    grammar_ref: iso-spec-core
    defaults:
      id_pattern: FR-{next:03d}
traceability:
  trace_targets:
    - name: acceptance-criterion
      archetype: FR
      section: Acceptance Criteria
      id_column: ID
  trace_tags:
    markers:
      - name: rust-trace-attribute
        language: rust
        pattern: '#\[trace\(([^)]*)\)\]'
        template: "#[trace({ids})]"
"##;

/// A flow rule the engine cannot load: it names an archetype nothing declares.
const BAD_FLOW_RULE: &str = "  flow_rules:
    - name: bad
      subject: [nowhere]
      match: [{path: [], field: id, present: true}]
      check: bad-check
      rationale: Names an archetype nothing declares.
";

const SPEC: &str = "---
id: FR-001
type: FR
title: One criterion
---

## Acceptance Criteria

| ID | Criteria |
|----|----------|
| FR-001-AC-1 | The reader shall refuse a payload over its ceiling. |
";

const CARGO_TOML: &str = "[package]
name = \"tiny\"
version = \"0.1.0\"
edition = \"2021\"
";

const LIB: &str = "#[cfg(test)]
mod tests {
    #[trace(\"FR-001-AC-1\")]
    #[test]
    fn tc_one() {}
}
";

/// A nextest-style report holding the one tagged test.
const JUNIT: &str = "<testsuites><testsuite name=\"tiny\"><testcase name=\"tests::tc_one\" classname=\"tiny\" time=\"0.001\"/></testsuite></testsuites>";

/// A repository holding the fixture module, one document and a tiny cargo
/// project with one tagged test, plus the report to annotate.
struct Scope {
    repo: tempfile::TempDir,
    aux: tempfile::TempDir,
}

impl Scope {
    fn new() -> Self {
        let scope = Self {
            repo: tempfile::tempdir().unwrap(),
            aux: tempfile::tempdir().unwrap(),
        };
        scope.write("manifest.yaml", MANIFEST);
        scope.write("spec/FR-001-example.md", SPEC);
        scope.write("Cargo.toml", CARGO_TOML);
        scope.write("src/lib.rs", LIB);
        fs::write(scope.junit(), JUNIT).unwrap();
        scope
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.repo.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn junit(&self) -> PathBuf {
        self.aux.path().join("junit.xml")
    }

    fn out(&self) -> PathBuf {
        self.aux.path().join("annotated.xml")
    }

    /// The same scope, now declaring a flow rule that cannot load.
    fn drop_a_declaration(&self) {
        self.write("manifest.yaml", &format!("{MANIFEST}{BAD_FLOW_RULE}"));
    }

    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quoin-suite-trace"))
            .arg("--repo")
            .arg(self.repo.path())
            .arg("--junit")
            .arg(self.junit())
            .arg("--out")
            .arg(self.out())
            .arg("--manifest-path")
            .arg(self.repo.path().join("Cargo.toml"))
            .output()
            .unwrap()
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[trace("FR-029-AC-21")]
#[test]
fn a_dropped_declaration_is_refused_and_writes_no_out_file() {
    let scope = Scope::new();
    scope.drop_a_declaration();
    let output = scope.run();
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "stderr {stderr}");
    assert!(stderr.contains("QQ-1022"), "{stderr}");
    assert!(stderr.contains(DROP_LINE), "{stderr}");
    assert!(output.stdout.is_empty(), "stdout {}", text(&output.stdout));
    assert!(
        !Path::new(&scope.out()).exists(),
        "a refusal writes no --out"
    );
}

#[trace("FR-029-AC-23")]
#[test]
fn a_scope_without_a_dropped_declaration_writes_the_annotated_report() {
    let scope = Scope::new();
    let output = scope.run();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let annotated = fs::read_to_string(scope.out()).unwrap();
    assert!(
        annotated.contains("<property name=\"trace\" value=\"FR-001-AC-1\"/>"),
        "{annotated}"
    );
}
