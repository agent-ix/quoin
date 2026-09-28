// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin matrix` against a real repository and a real evidence store
//! (FR-115).
//!
//! The store is written by the shipped commands (`quoin evidence record`,
//! `quoin evidence inspect-mocks`), not by hand, so the audit `quoin matrix`
//! renders is the one the real assembly produces over real records.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// The module every fixture repository carries: an FR archetype whose
/// Acceptance Criteria table is an `obligations:` source.
const MODULE: &str = r##"name: quoin-cli-matrix-fixture
version: 0.1.0
description: Fixture module for the quoin matrix command tests
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
  obligations:
    - name: acceptance-criterion
      target: acceptance-criterion
      statement_column: Criteria
      method_column: Verification
  trace_tags:
    markers:
      - name: rust-trace-attribute
        language: rust
        pattern: '#\[trace\(([^)]*)\)\]'
        template: "#[trace({ids})]"
"##;

/// Three criteria and no declared method, so the catalog cannot add an
/// `unknown-method` finding that would blur the three statuses under test.
const SPEC: &str = "---
id: FR-001
type: FR
title: Three criteria
---

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-001-AC-1 | Every stored payload shall round-trip through the reader unchanged. |  |
| FR-001-AC-2 | The reader shall refuse a payload over its ceiling. |  |
| FR-001-AC-3 | The reader shall name the refused field. |  |
";

/// A commit no repository here has, so a run recorded at it is behind HEAD.
const OLD_COMMIT: &str = "0000000000000000000000000000000000000001";

struct Fixture {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Fixture {
    fn repo(&self) -> &Path {
        self.dir.path()
    }

    fn quoin(&self, arguments: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
        command
            .args(arguments)
            .current_dir(self.repo())
            .env("IX_HOME", self.home.path())
            .env("GIT_CEILING_DIRECTORIES", self.repo().parent().unwrap());
        command.output().expect("the native quoin binary runs")
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

    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.repo().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }
}

/// A repository holding the module and `spec`, committed, with no store.
fn repository(module: &str, git: bool) -> Fixture {
    let fixture = Fixture {
        dir: tempfile::tempdir().unwrap(),
        home: tempfile::tempdir().unwrap(),
    };
    fixture.write("manifest.yaml", module);
    fixture.write("spec/FR-001-example.md", SPEC);
    if git {
        for arguments in [
            &["init", "-q"][..],
            &["add", "-A"],
            &[
                "-c",
                "user.email=t@example.invalid",
                "-c",
                "user.name=t",
                "commit",
                "-qm",
                "fixture",
            ],
        ] {
            let status = Command::new("git")
                .args(arguments)
                .current_dir(fixture.repo())
                .status()
                .unwrap();
            assert!(status.success(), "git {arguments:?}");
        }
    }
    fixture
}

fn head(fixture: &Fixture) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(fixture.repo())
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

/// One passing `unit` run at HEAD for AC-1 with its mock inspection at HEAD,
/// and one passing `legacy` run behind HEAD for AC-2. AC-3 is bound nowhere.
fn populated() -> Fixture {
    let fixture = repository(MODULE, true);
    let head = head(&fixture);
    // Outside the repository, so the results file is not part of the tree.
    let junit = |suite: &str, id: &str| {
        let path = fixture.home.path().join(format!("{suite}.xml"));
        std::fs::write(
            &path,
            format!(
                r#"<testsuite name="{suite}"><testcase name="{suite}_case"><properties><property name="trace" value="{id}"/></properties></testcase></testsuite>"#
            ),
        )
        .unwrap();
        path
    };
    for (suite, commit, id) in [
        ("unit", head.as_str(), "FR-001-AC-1"),
        ("legacy", OLD_COMMIT, "FR-001-AC-2"),
    ] {
        let results = junit(suite, id);
        fixture.ok(&[
            "evidence",
            "record",
            "--repo",
            ".",
            "--module",
            ".",
            "--suite",
            suite,
            "--commit",
            commit,
            "--tool",
            "fixture",
            "--adapter",
            "junit",
            "--results",
            results.to_str().unwrap(),
            "--timestamp",
            "2026-01-01T00:00:00Z",
        ]);
    }
    fixture.ok(&[
        "evidence",
        "inspect-mocks",
        "--repo",
        ".",
        "--suite",
        "unit",
        "--commit",
        &head,
        "--timestamp",
        "2026-01-01T00:00:00Z",
    ]);
    fixture
}

fn statuses(matrix: &Value) -> BTreeMap<String, String> {
    matrix["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|requirement| requirement["criteria"].as_array().unwrap())
        .map(|criterion| {
            (
                criterion["id"].as_str().unwrap().to_owned(),
                criterion["evidence_status"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// A passing binding with a mock inspection at HEAD, a binding behind HEAD
/// and no binding at all read as three distinct statuses in one matrix, and
/// agree with what `quoin evidence audit` reports for the same tree.
///
/// Trace: FR-115-AC-6, FR-115-AC-14
#[test]
fn tc_1080_200_a_real_store_yields_bound_stale_and_undischarged() {
    let fixture = populated();
    let matrix: Value =
        serde_json::from_str(&fixture.ok(&["matrix", "--module", ".", "--json"])).unwrap();
    assert_eq!(
        statuses(&matrix),
        BTreeMap::from([
            ("FR-001-AC-1".to_owned(), "bound".to_owned()),
            ("FR-001-AC-2".to_owned(), "stale".to_owned()),
            ("FR-001-AC-3".to_owned(), "undischarged".to_owned()),
        ])
    );

    // The same assembly: the audit command's own healthy set and findings
    // are exactly what the matrix read.
    let audit: Value =
        serde_json::from_str(&fixture.ok(&["evidence", "audit", "--module", ".", "--json"]))
            .unwrap();
    assert_eq!(audit["healthy"], serde_json::json!(["FR-001-AC-1"]));
    let mut from_audit: Vec<(String, String)> = audit["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| {
            (
                finding["obligation"].as_str().unwrap().to_owned(),
                finding["summary"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    from_audit.sort();
    let mut from_matrix: Vec<(String, String)> = matrix["requirements"][0]["criteria"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|criterion| {
            criterion["evidence_detail"]["findings"]
                .as_array()
                .unwrap()
                .iter()
                .map(|finding| {
                    (
                        criterion["id"].as_str().unwrap().to_owned(),
                        finding["summary"].as_str().unwrap().to_owned(),
                    )
                })
        })
        .collect();
    from_matrix.sort();
    assert_eq!(from_matrix, from_audit);
    assert!(
        from_audit
            .iter()
            .any(|(id, summary)| id == "FR-001-AC-2" && summary.contains("not at HEAD")),
        "the stale status must come from the auditor's behind-HEAD check: {from_audit:?}"
    );
}

/// Markdown by default, canonical JSON under `--json`, both byte-identical
/// across two runs.
///
/// Trace: FR-115-AC-10, FR-115-AC-11
#[test]
fn tc_1080_201_renders_markdown_and_canonical_json_deterministically() {
    let fixture = populated();
    let markdown = fixture.ok(&["matrix", "--module", "."]);
    assert_eq!(markdown, fixture.ok(&["matrix", "--module", "."]));
    assert!(
        markdown.starts_with("# Test Matrix\n\n## FR-001\n\n"),
        "{markdown}"
    );
    assert!(markdown.contains(
        "| Requirement | Criterion | Method | Static Status | Evidence Status | Detail |\n"
    ));
    assert!(markdown.contains("| FR-001 | FR-001-AC-3 | — | untagged | undischarged | "));

    let json = fixture.ok(&["matrix", "--module", ".", "--json"]);
    assert_eq!(json, fixture.ok(&["matrix", "--module", ".", "--json"]));
    let parsed: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        json,
        format!("{}\n", serde_json::to_string(&parsed).unwrap()),
        "--json is one line of canonical JSON"
    );
}

/// A module declaring no `obligations:` source renders the reason and exits 0.
///
/// Trace: FR-115-AC-11
#[test]
fn tc_1080_202_no_obligations_source_renders_the_reason() {
    let without = MODULE.replace(
        "  obligations:\n    - name: acceptance-criterion\n      target: acceptance-criterion\n      statement_column: Criteria\n      method_column: Verification\n",
        "",
    );
    assert_ne!(without, MODULE, "the fixture edit must remove the source");
    let fixture = repository(&without, true);
    let markdown = fixture.ok(&["matrix", "--module", "."]);
    assert_eq!(
        markdown,
        format!("# Test Matrix\n\n{}\n", quoin_assurance_reason())
    );
    let json: Value =
        serde_json::from_str(&fixture.ok(&["matrix", "--module", ".", "--json"])).unwrap();
    assert_eq!(json["requirements"], serde_json::json!([]));
    assert_eq!(json["reason"], quoin_assurance_reason());
}

/// The reason text, spelled out: this test asserts the literal rather than
/// importing the constant it is checking.
fn quoin_assurance_reason() -> &'static str {
    "no coverage_matrix criteria: the module declares no `obligations:` source, so there is no criterion to report evidence for"
}

/// Every file under `spec/`, by relative path, with its bytes.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, &root.join("spec"), &mut out);
    out
}

/// `quoin matrix` leaves `spec/` byte-for-byte as it found it.
///
/// Trace: FR-115-AC-12, FR-115-CON-3
#[test]
fn tc_1080_203_writes_nothing_under_spec() {
    let fixture = populated();
    let before = snapshot(fixture.repo());
    assert!(
        before.keys().any(|path| path.starts_with("spec/evidence")),
        "the snapshot must include the store, or it proves nothing about it"
    );
    fixture.ok(&["matrix", "--module", "."]);
    fixture.ok(&["matrix", "--module", ".", "--json"]);
    assert_eq!(snapshot(fixture.repo()), before);
}

/// Without a resolvable HEAD the command refuses with exit 2 and prints no
/// matrix.
///
/// Trace: FR-115-AC-14
#[test]
fn tc_1080_204_an_unresolvable_head_is_refused() {
    let fixture = repository(MODULE, false);
    let output = fixture.quoin(&["matrix", "--module", "."]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "no partial matrix");
    let diagnostics: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostics[0]["code"], "CORE_REFUSED");
    assert_eq!(diagnostics[0]["context"]["op"], "matrix");
}

/// `quoin matrix` never launches `ix-flow`: an `ix-flow` on PATH that would
/// leave a marker is never run.
///
/// Trace: FR-115-AC-13
#[test]
fn tc_1080_205_never_spawns_ix_flow() {
    let fixture = populated();
    let bin = tempfile::tempdir().unwrap();
    let marker = bin.path().join("ran");
    let script = bin.path().join("ix-flow");
    std::fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = std::env::join_paths(
        std::iter::once(bin.path().to_path_buf())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["matrix", "--module", "."])
        .current_dir(fixture.repo())
        .env("IX_HOME", fixture.home.path())
        .env("PATH", path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(!marker.exists(), "quoin matrix ran ix-flow");
}

/// Help describes the computed view, not the retired workflow.
///
/// Trace: FR-115-AC-15
#[test]
fn tc_1080_206_help_describes_the_evidence_backed_render() {
    let fixture = repository(MODULE, false);
    let page = fixture.ok(&["matrix", "--help"]);
    assert!(
        page.starts_with("Render the evidence-backed test matrix, per criterion.\n"),
        "{page}"
    );
    assert!(!page.contains("Build or update a requirements test matrix"));
    assert!(!page.contains("ix-flow"));
    assert!(!page.contains("[--strict]"), "the view adds no gate");
    let root = fixture.ok(&["--help"]);
    assert!(
        root.contains(
            "  matrix            Render the evidence-backed test matrix, per criterion.\n"
        )
    );
    assert!(!root.contains("Build or update a requirements test matrix"));
}
