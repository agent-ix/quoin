// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! A scope whose module lost a traceability declaration is refused, never
//! computed over (FR-029-AC-21, FR-029-AC-23; PLAT-1854), for `soup promote`,
//! `release snapshot`, `assurance`, `dossier generate|check|export` and
//! `formal-rules coverage|status`.
//!
//! The module is the scope's own `manifest.yaml`. Appending a flow rule that
//! names an archetype nothing declares makes the engine drop that one
//! declaration; the same scope without it computes normally.
//!
//! Provenance: PLAT-1854.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod pack_support;

use ix_trace_rs::trace;
use serde_json::{Value, json};

/// The line the refusal names for the dropped flow rule.
const DROP_LINE: &str = "module=quoin-cli-dropped-fixture kind=flow_rule item=bad";

const MANIFEST: &str = r##"name: quoin-cli-dropped-fixture
version: 0.1.0
description: Fixture module for the dropped-declaration refusals
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
title: Three criteria
---

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-001-AC-1 | Every stored payload shall round-trip through the reader unchanged. |  |
| FR-001-AC-2 | The reader shall refuse a payload over its ceiling. |  |
";

const BASE: &str = "0123456789abcdef0123456789abcdef01234567";

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// One scope: a repository root holding the fixture module and one document.
struct Scope {
    dir: tempfile::TempDir,
}

impl Scope {
    fn new() -> Self {
        let scope = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        scope.write("manifest.yaml", MANIFEST);
        scope.write("spec/FR-001-example.md", SPEC);
        scope
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }

    /// The same scope with its whole `traceability:` section unreadable, so
    /// every declaration is dropped and the merged model is empty.
    fn drop_every_declaration(&self) {
        let head = MANIFEST.split("traceability:").next().unwrap();
        self.write(
            "manifest.yaml",
            &format!("{head}traceability:\n  required_relations: 5\n"),
        );
    }

    /// The same scope, now declaring a flow rule that cannot load.
    fn drop_a_declaration(&self) {
        self.write("manifest.yaml", &format!("{MANIFEST}{BAD_FLOW_RULE}"));
    }
}

/// A refusal: exit 2, `QQ-1022` and the dropped declaration on stderr, nothing
/// on stdout.
fn assert_refused(output: &Output) {
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "stderr {stderr}");
    assert!(stderr.contains("QQ-1022"), "{stderr}");
    assert!(stderr.contains(DROP_LINE), "{stderr}");
    assert!(output.stdout.is_empty(), "stdout {}", text(&output.stdout));
}

fn run_git(dir: &Path, args: &[&str]) {
    let result = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "git {args:?}: {}",
        text(&result.stderr)
    );
}

fn git_commit_all(dir: &Path) {
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.name", "Dropped test"]);
    run_git(dir, &["config", "user.email", "dropped@example.invalid"]);
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-qm", "scope"]);
}

fn executable(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

// ---------------------------------------------------------------- soup promote

/// The owning module `soup promote` reads its skeleton from, in place: the
/// `SPEC_OBJECTS_SECURITY_DIR` directory, else the `spec-objects-security`
/// checkout beside the main quoin checkout (`~/dev`), which a worktree under
/// `~/dev/worktrees` reaches through `$HOME`.
fn soup_module() -> PathBuf {
    let beside = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../spec-objects-security/spec_objects_security");
    let from_home = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("dev/spec-objects-security/spec_objects_security"));
    let root = std::env::var_os("SPEC_OBJECTS_SECURITY_DIR")
        .map(PathBuf::from)
        .or_else(|| Some(beside.clone()).filter(|path| path.is_dir()))
        .or(from_home)
        .unwrap_or(beside);
    fs::canonicalize(&root).unwrap_or_else(|error| {
        panic!(
            "the spec-objects-security module is not at {}: {error}; set \
             SPEC_OBJECTS_SECURITY_DIR to its spec_objects_security directory",
            root.display()
        )
    })
}

/// Run `soup promote --discharges FR-001-AC-1` over `scope`; returns the output
/// and the directory it would write records into.
fn promote(scope: &Scope) -> (Output, PathBuf) {
    let sbom_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/evidence/cyclonedx-real.json");
    let out = scope.path().join("soup-out");
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["soup", "promote"])
        .arg(sbom_path)
        .arg("--repo")
        .arg(scope.path())
        .arg("--out")
        .arg(&out)
        .arg("--module")
        .arg(soup_module())
        .args(["--discharges", "FR-001-AC-1"])
        .output()
        .unwrap();
    (output, out)
}

#[trace("FR-029-AC-21")]
#[test]
fn soup_promote_over_a_dropped_declaration_is_refused_and_writes_no_record() {
    let scope = Scope::new();
    scope.drop_a_declaration();
    let (output, out) = promote(&scope);
    assert_refused(&output);
    assert!(!out.exists(), "a refusal writes no record");
}

#[trace("FR-029-AC-23")]
#[test]
fn soup_promote_without_a_dropped_declaration_writes_its_records() {
    let scope = Scope::new();
    let (output, out) = promote(&scope);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let report = text(&output.stdout);
    assert!(report.contains("records created:"), "{report}");
    assert!(!report.contains("unmatched"), "{report}");
    assert!(out.is_dir(), "the records are written");
}

// ------------------------------------------------------------ release snapshot

/// A scope with the evidence-store and input files `release snapshot` reads,
/// written by the shipped commands as `tc_3604_release_snapshot.rs` does.
struct Release {
    scope: Scope,
    home: tempfile::TempDir,
    aux: tempfile::TempDir,
}

impl Release {
    fn new() -> Self {
        let release = Self {
            scope: Scope::new(),
            home: tempfile::tempdir().unwrap(),
            aux: tempfile::tempdir().unwrap(),
        };
        fs::create_dir_all(release.scope.path().join("spec/soup")).unwrap();
        release.write_inputs();
        release
    }

    fn aux(&self, name: &str) -> PathBuf {
        self.aux.path().join(name)
    }

    fn quoin(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(arguments)
            .env("IX_HOME", self.home.path())
            .output()
            .unwrap()
    }

    fn write_inputs(&self) {
        fs::write(
            self.aux("history.json"),
            json!({
                "protocol": "engineering-assurance.release-history/v1",
                "baseline_commit": BASE,
                "previous_release": Value::Null,
                "records": [],
            })
            .to_string(),
        )
        .unwrap();
        // A discharge resolves each clause's policy from an installed module.
        pack_support::install_clause_set(
            self.home.path(),
            "example.invalid",
            "soup",
            "1.0.0",
            &["C-DISP"],
        );
        let binding = self.aux("binding.json");
        fs::write(
            &binding,
            json!({
                "schemaVersion": "clause-binding-v1",
                "clauseSet": {"authority": "example.invalid", "id": "soup", "version": "1.0.0"},
                "context": {"product": "widget"},
                "clauses": [{"clauseId": "C-DISP", "force": "mandatory", "outcome": "binding",
                             "reasons": [], "expectedOutputs": []}],
            })
            .to_string(),
        )
        .unwrap();
        let facts = self.aux("facts.json");
        fs::write(
            &facts,
            json!([{
                "kind": "disposition", "clauseId": "C-DISP", "decision": "accepted_risk",
                "owner": "risk-owner-1", "rationale": "no reachable path",
                "approvalRef": "approval://CAB-7",
                "attestation": {"attestedBy": "reviewer-1", "authority": "quality-lead",
                    "attestedAt": "2026-08-01T00:00:00Z", "expiresAt": "2026-09-01T00:00:00Z"}
            }])
            .to_string(),
        )
        .unwrap();
        let recorded = self.quoin(&[
            "evidence",
            "record-discharge",
            "--repo",
            self.scope.path().to_str().unwrap(),
            "--binding",
            binding.to_str().unwrap(),
            "--facts",
            facts.to_str().unwrap(),
        ]);
        assert_eq!(
            recorded.status.code(),
            Some(0),
            "{}",
            text(&recorded.stderr)
        );
    }

    fn snapshot(&self) -> (Output, PathBuf) {
        let out = self.aux("snapshot.json");
        let output = self.quoin(&[
            "release",
            "snapshot",
            "--repo",
            self.scope.path().to_str().unwrap(),
            "--baseline-commit",
            BASE,
            "--history",
            self.aux("history.json").to_str().unwrap(),
            "--binding",
            self.aux("binding.json").to_str().unwrap(),
            "--as-of",
            "2026-08-15T00:00:00Z",
            "--out",
            out.to_str().unwrap(),
        ]);
        (output, out)
    }
}

#[trace("FR-029-AC-21")]
#[test]
fn release_snapshot_over_a_dropped_declaration_is_refused_and_writes_no_snapshot() {
    let release = Release::new();
    release.scope.drop_a_declaration();
    let (output, out) = release.snapshot();
    assert_refused(&output);
    assert!(!out.exists(), "a refusal writes no snapshot");
}

#[trace("FR-029-AC-23")]
#[test]
fn release_snapshot_without_a_dropped_declaration_writes_its_snapshot() {
    let release = Release::new();
    let (output, out) = release.snapshot();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout), format!("{}\n", out.display()));
    let snapshot: Value = serde_json::from_slice(&fs::read(&out).unwrap()).unwrap();
    assert!(snapshot.is_object(), "{snapshot}");
}

// ------------------------------------------------------------------- assurance

fn assurance(scope: &Scope) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["assurance", "--repo"])
        .arg(scope.path())
        .args([
            "--module",
            scope.path().to_str().unwrap(),
            "--as-of",
            "2026-08-15T00:00:00.000Z",
            "--json",
        ])
        .output()
        .unwrap()
}

#[trace("FR-029-AC-21")]
#[test]
fn assurance_over_a_dropped_declaration_is_refused() {
    let scope = Scope::new();
    scope.drop_a_declaration();
    assert_refused(&assurance(&scope));
}

#[trace("FR-029-AC-23")]
#[test]
fn assurance_without_a_dropped_declaration_answers_its_case() {
    let scope = Scope::new();
    let output = assurance(&scope);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let case: Value = serde_json::from_str(text(&output.stdout).trim()).unwrap();
    assert!(case.is_object(), "{case}");
}

// --------------------------------------------------------------------- dossier

fn dossier(scope: &Scope, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .arg("dossier")
        .args(arguments)
        .current_dir(scope.path())
        .output()
        .unwrap()
}

/// A scope whose stub dossier document is generated and committed.
fn generated_dossier() -> Scope {
    let scope = Scope::new();
    git_commit_all(scope.path());
    let output = dossier(&scope, &["generate", "--version", "1.0", "--doc", "stub-a"]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    run_git(scope.path(), &["add", "dossier"]);
    run_git(scope.path(), &["commit", "-qm", "dossier"]);
    scope
}

/// A `quire` that writes `--output` and succeeds.
fn fake_quire(dir: &Path) -> PathBuf {
    executable(
        dir,
        "fake-quire",
        "#!/bin/sh\nout=\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = --output ]; then out=$2; fi\n  shift\ndone\necho bundle > \"$out\"\n",
    )
}

fn export(scope: &Scope, quire: &Path) -> Output {
    dossier(
        scope,
        &[
            "export",
            "--version",
            "1.0",
            "--doc",
            "stub-a",
            "--to",
            "pdf",
            "--output",
            "out.pdf",
            "--quire",
            quire.to_str().unwrap(),
        ],
    )
}

#[trace("FR-029-AC-21")]
#[test]
fn dossier_generate_over_a_dropped_declaration_is_refused_and_writes_no_document() {
    let scope = Scope::new();
    scope.drop_a_declaration();
    git_commit_all(scope.path());
    let output = dossier(&scope, &["generate", "--version", "1.0", "--doc", "stub-a"]);
    assert_refused(&output);
    assert!(
        !scope.path().join("dossier").exists(),
        "no document written"
    );
}

#[trace("FR-029-AC-21")]
#[test]
fn dossier_check_over_a_dropped_declaration_is_refused() {
    let scope = generated_dossier();
    scope.drop_a_declaration();
    let output = dossier(&scope, &["check", "--version", "1.0", "--doc", "stub-a"]);
    assert_refused(&output);
}

#[trace("FR-029-AC-21")]
#[test]
fn dossier_export_over_a_dropped_declaration_is_refused_and_writes_no_output() {
    let scope = generated_dossier();
    scope.drop_a_declaration();
    let tools = tempfile::tempdir().unwrap();
    let output = export(&scope, &fake_quire(tools.path()));
    assert_refused(&output);
    assert!(!scope.path().join("out.pdf").exists(), "no output written");
}

#[trace("FR-029-AC-23")]
#[test]
fn dossier_generate_check_and_export_without_a_dropped_declaration_succeed() {
    let scope = generated_dossier();
    let document = scope.path().join("dossier/DOS-STUB-A.md");
    assert!(document.is_file(), "generate writes the document");
    let check = dossier(&scope, &["check", "--version", "1.0", "--doc", "stub-a"]);
    assert_eq!(check.status.code(), Some(0), "{}", text(&check.stderr));
    let tools = tempfile::tempdir().unwrap();
    let exported = export(&scope, &fake_quire(tools.path()));
    assert_eq!(
        exported.status.code(),
        Some(0),
        "{}",
        text(&exported.stderr)
    );
    assert!(
        scope.path().join("out.pdf").is_file(),
        "export writes its output"
    );
}

// ---------------------------------------------------------------- formal-rules

/// A scope under git with the trust inputs `formal-rules` reads: a fake
/// engineering-assurance that identifies itself, and an allowed-signers file.
struct Formal {
    scope: Scope,
    tools: tempfile::TempDir,
}

impl Formal {
    fn new() -> Self {
        let formal = Self {
            scope: Scope::new(),
            tools: tempfile::tempdir().unwrap(),
        };
        executable(
            formal.tools.path(),
            "engineering-assurance",
            "#!/bin/sh\necho 'engineering-assurance 0.0.0'\n",
        );
        fs::write(formal.tools.path().join("allowed-signers"), "").unwrap();
        git_commit_all(formal.scope.path());
        formal
    }

    fn run(&self, subcommand: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(["formal-rules", subcommand, "--json", "--repo"])
            .arg(self.scope.path())
            .arg("--engineering-assurance")
            .arg(self.tools.path().join("engineering-assurance"))
            .arg("--allowed-signers")
            .arg(self.tools.path().join("allowed-signers"))
            .output()
            .unwrap()
    }
}

#[trace("FR-029-AC-21")]
#[test]
fn formal_rules_status_over_a_dropped_declaration_is_refused() {
    let formal = Formal::new();
    formal.scope.drop_a_declaration();
    assert_refused(&formal.run("status"));
}

#[trace("FR-029-AC-21")]
#[test]
fn formal_rules_coverage_over_a_dropped_declaration_is_refused() {
    let formal = Formal::new();
    formal.scope.drop_a_declaration();
    assert_refused(&formal.run("coverage"));
}

#[trace("FR-029-AC-23")]
#[test]
fn formal_rules_status_without_a_dropped_declaration_reports() {
    let formal = Formal::new();
    let status = formal.run("status");
    assert_eq!(status.status.code(), Some(0), "{}", text(&status.stderr));
    let report: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(report["protocol"], "quoin.formal-rule-status/v1");
}

/// With every declaration dropped the model is empty and reads as absent; the
/// command still names the drop, not the absent-model code QQ-1020.
#[trace("FR-029-AC-20")]
#[test]
fn formal_rules_status_over_an_all_dropped_model_is_qq_1022_not_qq_1020() {
    let formal = Formal::new();
    formal.scope.drop_every_declaration();
    let output = formal.run("status");
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(output.stdout.is_empty(), "{}", text(&output.stdout));
    assert!(stderr.contains("QQ-1022"), "{stderr}");
    assert!(!stderr.contains("QQ-1020"), "{stderr}");
    assert!(
        stderr.contains(
            "module=quoin-cli-dropped-fixture kind=traceability_section item=traceability"
        ),
        "{stderr}"
    );
}

#[trace("FR-029-AC-23")]
#[test]
fn formal_rules_coverage_without_a_dropped_declaration_reports() {
    let formal = Formal::new();
    let coverage = formal.run("coverage");
    assert_eq!(
        coverage.status.code(),
        Some(0),
        "{}",
        text(&coverage.stderr)
    );
    assert_ne!(coverage.stdout, [] as [u8; 0]);
    let report: Value = serde_json::from_slice(&coverage.stdout).unwrap();
    assert!(report.is_object(), "{report}");
}
