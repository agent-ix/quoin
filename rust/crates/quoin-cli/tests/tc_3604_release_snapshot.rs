// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! `quoin release snapshot` over fixture repositories (FR-166, FR-167,
//! FR-168, NFR-036).
//!
//! The evidence store is written by the shipped commands, scans through the
//! store's own writer, and every other input by hand. `git` is run only by
//! this file's setup, to give one fixture a history the command must ignore.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use quoin_evidence::adapters::{AdapterSelection, select_finding_adapter};
use quoin_evidence::store::write_scan;
use quoin_evidence::types::FindingRecord;
use quoin_evidence::{DiskEvidence, STORE_SCHEMA_VERSION, SuiteId};
use serde_json::{Value, json};

mod pack_support;

const BASE: &str = "0123456789abcdef0123456789abcdef01234567";
const AS_OF: &str = "2026-08-15T00:00:00Z";
const SCANNED: &str = "2026-07-31T00:00:00.000Z";

const MODULE: &str = r##"name: quoin-cli-release-fixture
version: 0.1.0
description: Fixture module for the quoin release snapshot tests
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

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn audit_document(vulns: &[(&str, &str, &str)]) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/evidence/cargo-audit-real.json");
    let mut document: Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let list: Vec<Value> = vulns
        .iter()
        .map(|(id, name, version)| {
            json!({
                "advisory": { "id": id, "title": format!("{id} title") },
                "package": { "name": name, "version": version }
            })
        })
        .collect();
    document["vulnerabilities"] =
        json!({ "found": !list.is_empty(), "count": list.len(), "list": list });
    document["warnings"] = json!({});
    document.to_string()
}

fn audit_scan(vulns: &[(&str, &str, &str)]) -> FindingRecord {
    let adapter = select_finding_adapter(AdapterSelection {
        adapter: None,
        tool: Some("cargo-audit"),
    })
    .unwrap();
    let parsed = adapter.run(&audit_document(vulns), None).unwrap();
    FindingRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new("audit"),
        tool: "cargo-audit".to_owned(),
        evidence_kind: None,
        timestamp: SCANNED.to_owned(),
        ruleset: parsed.ruleset,
        rules_evaluated: parsed.rules_evaluated,
        findings: parsed.findings,
        evaluated: parsed.evaluated,
    }
}

/// One fixture repository, the IX home it runs under, and a directory beside
/// it for the inputs and the output.
struct Repo {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
    aux: tempfile::TempDir,
}

/// The flags of one `quoin release snapshot` run, replaceable by name.
struct Run {
    flags: Vec<(String, String)>,
}

impl Run {
    fn set(mut self, flag: &str, value: &str) -> Self {
        self.flags.retain(|(name, _)| name != flag);
        self.flags.push((flag.to_owned(), value.to_owned()));
        self
    }
}

impl Repo {
    fn bare() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
            aux: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(repo.path().join("spec/soup")).unwrap();
        repo.write("manifest.yaml", MODULE);
        repo.write("spec/FR-001-example.md", SPEC);
        repo.history(&[], &Value::Null);
        repo.binding(&["C-DISP", "C-OTHER"]);
        repo
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn aux(&self, name: &str) -> PathBuf {
        self.aux.path().join(name)
    }

    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.path().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    fn quoin(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(arguments)
            .env("IX_HOME", self.home.path())
            .output()
            .unwrap()
    }

    fn ok(&self, arguments: &[&str]) -> String {
        let output = self.quoin(arguments);
        assert!(
            output.status.success(),
            "{arguments:?}: {}",
            text(&output.stderr)
        );
        text(&output.stdout)
    }

    /// Two passing-or-missing runs: AC-1 is `bound`, AC-2 `stale` (its run is
    /// gone), AC-3 `undischarged`.
    fn evidence(&self) {
        for (suite, id) in [("unit", "FR-001-AC-1"), ("legacy", "FR-001-AC-2")] {
            let results = self.aux(&format!("{suite}.xml"));
            std::fs::write(
                &results,
                format!(
                    r#"<testsuite name="{suite}"><testcase name="{suite}_case"><properties><property name="trace" value="{id}"/></properties></testcase></testsuite>"#
                ),
            )
            .unwrap();
            self.ok(&[
                "evidence",
                "record",
                "--repo",
                self.path().to_str().unwrap(),
                "--module",
                self.path().to_str().unwrap(),
                "--suite",
                suite,
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
        self.ok(&[
            "evidence",
            "inspect-mocks",
            "--repo",
            self.path().to_str().unwrap(),
            "--suite",
            "unit",
            "--timestamp",
            "2026-01-01T00:00:00Z",
        ]);
        std::fs::remove_dir_all(self.path().join("spec/evidence/runs/legacy")).unwrap();
    }

    fn soup_record(&self, file: &str, purl: &str, extra: &[&str]) {
        let mut front = format!(
            "---\nid: SOUP-{file}\ntitle: \"{file}\"\ntype: soup_record\nobject: soup_record\npurl: \"{purl}\"\n"
        );
        for line in extra {
            front.push_str(line);
            front.push('\n');
        }
        front.push_str("---\n# ");
        front.push_str(file);
        front.push('\n');
        self.write(&format!("spec/soup/{file}.md"), &front);
    }

    /// Four records in the three groups and one unmatched advisory.
    fn soup(&self) {
        self.soup_record(
            "anyhow",
            "pkg:cargo/anyhow@1.0.102",
            &["version: \"1.0.102\"", "disposition_clause: \"C-DISP\""],
        );
        self.soup_record("bytes", "pkg:cargo/bytes@1.0.0", &["version: \"1.0.0\""]);
        self.soup_record(
            "old",
            "pkg:cargo/old@0.1.0",
            &["version: \"0.1.0\"", "end_of_support: \"2026-05-31\""],
        );
        self.soup_record("zed", "pkg:cargo/zed@1.0.0", &["version: \"1.0.0\""]);
        write_scan(
            &mut DiskEvidence::new(self.path()),
            &audit_scan(&[
                ("RUSTSEC-2026-0190", "anyhow", "1.0.102"),
                ("RUSTSEC-2026-0001", "bytes", "1.0.0"),
                ("RUSTSEC-2026-0002", "ghost", "9.9.9"),
            ]),
        )
        .unwrap();
    }

    /// An `Anomaly` document; `lines` are further frontmatter lines and `body`
    /// the sections after the title.
    fn anomaly(&self, id: &str, status: &str, safety_impact: &str, lines: &[&str], body: &str) {
        let mut front = format!(
            "---\nid: {id}\ntitle: \"Title of {id}\"\ntype: Anomaly\nstatus: {status}\nsafety_impact: {safety_impact}\n"
        );
        for line in lines {
            front.push_str(line);
            front.push('\n');
        }
        let document =
            format!("{front}---\n# [{id}] Title\n\n## Description\n\nA defect.\n\n{body}");
        self.write(&format!("spec/anomalies/{id}.md"), &document);
    }

    fn binding(&self, clauses: &[&str]) {
        let clause = |id: &&str| {
            json!({"clauseId": id, "force": "mandatory", "outcome": "binding",
                   "reasons": [], "expectedOutputs": []})
        };
        // A discharge resolves each clause's policy from an installed module.
        pack_support::install_clause_set(
            self.home.path(),
            "example.invalid",
            "soup",
            "1.0.0",
            clauses,
        );
        let binding = self.aux("binding.json");
        std::fs::write(
            &binding,
            json!({
                "schemaVersion": "clause-binding-v1",
                "clauseSet": {"authority": "example.invalid", "id": "soup", "version": "1.0.0"},
                "context": {"product": "widget"},
                "clauses": clauses.iter().map(clause).collect::<Vec<_>>(),
            })
            .to_string(),
        )
        .unwrap();
        let facts = self.aux("facts.json");
        std::fs::write(
            &facts,
            json!([{
                "kind": "disposition", "clauseId": "C-DISP", "decision": "accepted_risk",
                "owner": "risk-owner-1", "rationale": "no reachable path", "approvalRef": "approval://CAB-7",
                "attestation": {"attestedBy": "reviewer-1", "authority": "quality-lead",
                    "attestedAt": "2026-08-01T00:00:00Z", "expiresAt": "2026-09-01T00:00:00Z"}
            }])
            .to_string(),
        )
        .unwrap();
        self.ok(&[
            "evidence",
            "record-discharge",
            "--repo",
            self.path().to_str().unwrap(),
            "--binding",
            binding.to_str().unwrap(),
            "--facts",
            facts.to_str().unwrap(),
        ]);
    }

    fn history(&self, records: &[Value], previous: &Value) {
        self.history_document(&json!({
            "protocol": "engineering-assurance.release-history/v1",
            "baseline_commit": BASE,
            "previous_release": previous,
            "records": records,
        }));
    }

    fn history_document(&self, document: &Value) {
        std::fs::write(self.aux("history.json"), document.to_string()).unwrap();
    }

    fn run(&self) -> Run {
        let flags = [
            ("--repo", self.path().to_str().unwrap().to_owned()),
            ("--baseline-commit", BASE.to_owned()),
            ("--history", self.aux("history.json").display().to_string()),
            ("--binding", self.aux("binding.json").display().to_string()),
            ("--as-of", AS_OF.to_owned()),
            ("--out", self.aux("snapshot.json").display().to_string()),
        ];
        Run {
            flags: flags
                .into_iter()
                .map(|(flag, value)| (flag.to_owned(), value))
                .collect(),
        }
    }

    fn execute(&self, run: &Run) -> Output {
        self.execute_with(run, &[])
    }

    fn execute_with(&self, run: &Run, environment: &[(&str, &str)]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
        command.args(["release", "snapshot"]);
        for (flag, value) in &run.flags {
            command.arg(flag).arg(value);
        }
        command.env("IX_HOME", self.home.path());
        for (name, value) in environment {
            command.env(name, value);
        }
        command.output().unwrap()
    }

    /// Run and read the snapshot it wrote.
    fn snapshot(&self, run: &Run) -> Value {
        let output = self.execute(run);
        assert!(output.status.success(), "{}", text(&output.stderr));
        let out = &run
            .flags
            .iter()
            .find(|(flag, _)| flag == "--out")
            .unwrap()
            .1;
        assert_eq!(text(&output.stdout), format!("{out}\n"));
        serde_json::from_slice(&std::fs::read(out).unwrap()).unwrap()
    }

    /// Run a refusal: exit 2, its code, and that no file was written.
    fn refusal(&self, run: &Run) -> Value {
        let output = self.execute(run);
        assert_eq!(output.status.code(), Some(2), "{}", text(&output.stderr));
        assert_eq!(output.stdout, [] as [u8; 0]);
        let out = &run
            .flags
            .iter()
            .find(|(flag, _)| flag == "--out")
            .unwrap()
            .1;
        assert!(!Path::new(out).exists(), "a refusal writes no file");
        let diagnostics: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(diagnostics.as_array().unwrap().len(), 1);
        diagnostics[0].clone()
    }

    fn tree(&self) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(dir: &Path, found: &mut Vec<(PathBuf, Vec<u8>)>) {
            let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
            entries.sort_by_key(std::fs::DirEntry::path);
            for entry in entries {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, found);
                } else {
                    found.push((path.clone(), std::fs::read(&path).unwrap()));
                }
            }
        }
        let mut found = Vec::new();
        walk(self.path(), &mut found);
        found
    }
}

/// Evidence, SOUP records, anomalies and a two-record history.
fn full() -> Repo {
    let repo = Repo::bare();
    repo.evidence();
    repo.soup();
    repo.anomaly(
        "AN-001",
        "assessed",
        "impacting",
        &["severity: major"],
        "## Safety Impact Assessment\n\n| Kind | Target | Impact | Rationale |\n|------|--------|--------|-----------|\n| Requirement | FR-001 | Affects | Not enforced. |\n\n## Release Rationale\n\n| Author | Rationale |\n|--------|-----------|\n| Dana Lee | Ships behind a flag. |\n| ops-lead | Reviewed with QA. |\n",
    );
    repo.history(
        &[
            json!({"record_id": "CR-2", "revision": 1, "path": "records/CR-2.md", "commit": "c2"}),
            json!({"record_id": "CR-1", "revision": 1, "path": "records/CR-1.md", "commit": "c1"}),
        ],
        &json!({"tag": "v1.0.0", "commit": "f".repeat(40)}),
    );
    repo
}

/// The member names of every object of a snapshot, by position.
enum Shape {
    Leaf,
    Object(Vec<(&'static str, Shape)>),
    List(Box<Shape>),
    /// A member that is `null` or absent, or the shape.
    Optional(Box<Shape>),
}

fn obj(members: Vec<(&'static str, Shape)>) -> Shape {
    Shape::Object(members)
}

fn list(inner: Shape) -> Shape {
    Shape::List(Box::new(inner))
}

fn soup_shape() -> Shape {
    use Shape::{Leaf, Optional};
    let resolved = || {
        obj(vec![
            ("value", Leaf),
            ("origin", Leaf),
            ("source", Optional(Box::new(Leaf))),
            (
                "disagrees_with",
                Optional(Box::new(obj(vec![
                    ("value", Leaf),
                    ("origin", Leaf),
                    ("source", Leaf),
                ]))),
            ),
        ])
    };
    obj(vec![
        (
            "rows",
            list(obj(vec![
                ("purl", Leaf),
                ("version", Leaf),
                ("group", Leaf),
                ("support_status", resolved()),
                ("end_of_support", resolved()),
            ])),
        ),
        (
            "not_assessed",
            list(obj(vec![("kind", Leaf), ("tool", Leaf), ("detail", Leaf)])),
        ),
    ])
}

fn snapshot_shape() -> Shape {
    use Shape::{Leaf, Optional};
    obj(vec![
        ("protocol", Leaf),
        (
            "tool_identity",
            obj(vec![("name", Leaf), ("version", Leaf)]),
        ),
        ("clean_tree_precondition", Leaf),
        ("as_of", Leaf),
        ("baseline_commit", Leaf),
        (
            "previous_release",
            Optional(Box::new(obj(vec![("tag", Leaf), ("commit", Leaf)]))),
        ),
        (
            "vulnerabilities",
            list(obj(vec![
                ("id", Leaf),
                ("cve_id", Optional(Box::new(Leaf))),
                ("severity", Optional(Box::new(Leaf))),
                ("affects", list(Leaf)),
                ("references", list(Leaf)),
                ("status", Leaf),
                ("disposition", Optional(Box::new(Leaf))),
                ("disposition_by", Optional(Box::new(Leaf))),
                ("discovered_at", Optional(Box::new(Leaf))),
                ("disclosed_at", Optional(Box::new(Leaf))),
                ("fixed_at", Optional(Box::new(Leaf))),
            ])),
        ),
        (
            "coverage",
            list(obj(vec![
                ("id", Leaf),
                (
                    "criteria",
                    list(obj(vec![
                        ("id", Leaf),
                        ("static_status", Leaf),
                        ("evidence_status", Leaf),
                    ])),
                ),
            ])),
        ),
        (
            "evidence_freshness",
            obj(vec![
                ("bound", Leaf),
                ("stale", Leaf),
                ("suspect", Leaf),
                ("undischarged", Leaf),
                ("no run evidence", Leaf),
            ]),
        ),
        (
            "open_findings",
            list(obj(vec![
                ("obligation", Leaf),
                ("kind", Leaf),
                ("summary", Leaf),
                ("path", Leaf),
                ("line", Leaf),
                ("symbol", Leaf),
                ("cause", Optional(Box::new(Leaf))),
            ])),
        ),
        ("soup", soup_shape()),
        (
            "anomalies",
            list(obj(vec![
                ("id", Leaf),
                ("title", Leaf),
                ("status", Leaf),
                ("resolution", Leaf),
                ("severity", Leaf),
                ("safety_impact", Leaf),
                ("unassessed", Leaf),
                (
                    "release_rationale",
                    list(obj(vec![("author", Leaf), ("rationale", Leaf)])),
                ),
            ])),
        ),
        (
            "change_records",
            list(obj(vec![
                ("record_id", Leaf),
                ("revision", Leaf),
                ("path", Leaf),
                ("commit", Leaf),
            ])),
        ),
    ])
}

/// Every object of `value` carries the members the shape names and no other.
fn assert_closed(value: &Value, shape: &Shape) {
    match shape {
        Shape::Leaf => {}
        Shape::Optional(inner) => {
            if !value.is_null() {
                assert_closed(value, inner);
            }
        }
        Shape::List(inner) => {
            for item in value.as_array().unwrap() {
                assert_closed(item, inner);
            }
        }
        Shape::Object(members) => {
            let object = value.as_object().unwrap();
            for key in object.keys() {
                assert!(
                    members.iter().any(|(name, _)| name == key),
                    "extra member {key}"
                );
            }
            for (name, inner) in members {
                match object.get(*name) {
                    Some(member) => assert_closed(member, inner),
                    None => assert!(matches!(inner, Shape::Optional(_)), "missing member {name}"),
                }
            }
        }
    }
}

fn keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

#[trace("FR-166-AC-1", "FR-167-AC-1", "FR-167-AC-13", "NFR-036-M-4")]
#[test]
fn one_file_is_written_with_exactly_the_named_members_and_nothing_else_changes() {
    let repo = full();
    let before = repo.tree();
    let snapshot = repo.snapshot(&repo.run());
    assert_eq!(
        repo.tree(),
        before,
        "no file in the repository changes or appears"
    );
    let mut written: Vec<String> = std::fs::read_dir(repo.aux.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    written.sort();
    assert!(written.contains(&"snapshot.json".to_owned()));

    assert_eq!(
        keys(&snapshot),
        [
            "anomalies",
            "as_of",
            "baseline_commit",
            "change_records",
            "clean_tree_precondition",
            "coverage",
            "evidence_freshness",
            "open_findings",
            "previous_release",
            "protocol",
            "soup",
            "tool_identity",
            "vulnerabilities",
        ]
    );
    assert_eq!(snapshot["protocol"], "quoin.release-snapshot/v1");
    assert_eq!(keys(&snapshot["tool_identity"]), ["name", "version"]);
    assert_eq!(snapshot["tool_identity"]["name"], "quoin");
    assert_eq!(
        snapshot["tool_identity"]["version"],
        text(&repo.quoin(&["--version"]).stdout).trim()
    );
    assert_eq!(snapshot["clean_tree_precondition"], "not_verified");
    assert_eq!(snapshot["as_of"], AS_OF);
    assert_eq!(snapshot["baseline_commit"], BASE);
    assert_eq!(keys(&snapshot["soup"]), ["not_assessed", "rows"]);
    assert_eq!(snapshot["vulnerabilities"], json!([]));
    assert_eq!(
        keys(&snapshot["coverage"][0]["criteria"][0]),
        ["evidence_status", "id", "static_status"]
    );
    assert_eq!(
        keys(&snapshot["soup"]["rows"][0]),
        [
            "end_of_support",
            "group",
            "purl",
            "support_status",
            "version"
        ]
    );
    assert_closed(&snapshot, &snapshot_shape());
    let bytes = std::fs::read(repo.aux("snapshot.json")).unwrap();
    assert_eq!(
        bytes.iter().position(|b| *b == b'\n'),
        Some(bytes.len() - 1),
        "one line"
    );
}

#[trace("FR-167-AC-13")]
#[test]
fn the_snapshot_reader_requires_both_members_and_refuses_dirty_or_unordered_values() {
    use quoin_dossier::release_snapshot::ReleaseSnapshot;
    let parse = |document: &Value| ReleaseSnapshot::parse(&serde_json::to_vec(document).unwrap());
    let refusal = |document: &Value| parse(document).unwrap_err().to_string();
    let full = json!({
        "protocol":"quoin.release-snapshot/v1",
        "tool_identity":{"name":"quoin","version":"0.31.1"},
        "clean_tree_precondition":"not_verified",
        "as_of":"2026-11-01T00:00:00Z",
        "baseline_commit":BASE,
        "previous_release":null,
        "coverage":[],
        "evidence_freshness":{"bound":0,"stale":0,"suspect":0,"undischarged":0,"no run evidence":0},
        "open_findings":[],
        "soup":{"rows":[],"not_assessed":[]},
        "vulnerabilities":[],
        "anomalies":[],
        "change_records":[]
    });
    let parsed = parse(&full).unwrap();
    assert_eq!(parsed.tool_identity.name, "quoin");

    let mut terminal = full.clone();
    terminal["vulnerabilities"] = json!([{
        "id":"VULN-001",
        "cve_id":"CVE-2026-0001",
        "severity":"high",
        "affects":["SOUP-001"],
        "references":["AN-001"],
        "status":"remediated",
        "disposition":null,
        "disposition_by":null,
        "discovered_at":null,
        "disclosed_at":null,
        "fixed_at":"2026-01-02"
    }]);
    assert!(refusal(&terminal).contains("vulnerabilities status"));

    let mut healthy_kind = full.clone();
    healthy_kind["soup"]["not_assessed"] =
        json!([{"kind":"scan_not_run","tool":"osv-scanner","detail":"not run"}]);
    assert_eq!(parse(&healthy_kind).unwrap().soup.not_assessed.len(), 1);

    let mut missing_identity = full.clone();
    missing_identity
        .as_object_mut()
        .unwrap()
        .remove("tool_identity");
    assert!(refusal(&missing_identity).contains("tool_identity"));

    let mut missing_clean_tree = full.clone();
    missing_clean_tree
        .as_object_mut()
        .unwrap()
        .remove("clean_tree_precondition");
    assert!(refusal(&missing_clean_tree).contains("clean_tree_precondition"));

    let mut unordered = full.clone();
    unordered["soup"]["not_assessed"] =
        json!([{"kind":"scan_unordered","tool":"osv-scanner","detail":"unordered"}]);
    assert!(refusal(&unordered).contains("soup.not_assessed kind"));

    let mut dirty = full.clone();
    dirty["tool_identity"] = Value::Null;
    assert!(refusal(&dirty).contains("invalid type: null"));

    let mut dirty = full.clone();
    dirty["clean_tree_precondition"] = Value::Null;
    assert!(refusal(&dirty).contains("expected value"));

    let mut dirty = full.clone();
    dirty["tool_identity"]["unexpected"] = json!(true);
    assert!(refusal(&dirty).contains("unknown field `unexpected`"));

    let mut dirty = full.clone();
    dirty["clean_tree_precondition"] = json!("verified");
    assert!(refusal(&dirty).contains("unknown variant `verified`"));

    let mut dirty = full.clone();
    dirty["tool_identity"]["name"] = json!("another-tool");
    assert!(refusal(&dirty).contains("tool_identity must name quoin"));

    let mut dirty = full;
    dirty["tool_identity"]["version"] = json!(" ");
    assert!(refusal(&dirty).contains("tool_identity must name quoin"));
}

fn git(dir: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .args(["-c", "user.name=T", "-c", "user.email=t@example.invalid"])
        .args(arguments)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", text(&output.stderr));
}

#[trace("FR-166-AC-2", "FR-167-AC-9", "FR-168-AC-4")]
#[test]
fn the_bytes_are_the_same_without_git_and_with_anomaly_files_that_have_history() {
    let plain = full();
    let bare = plain.execute_with(&plain.run(), &[("PATH", "")]);
    assert!(bare.status.success(), "{}", text(&bare.stderr));
    let without_git = std::fs::read(plain.aux("snapshot.json")).unwrap();

    let versioned = full();
    git(versioned.path(), &["init", "-q"]);
    git(versioned.path(), &["add", "-A"]);
    git(versioned.path(), &["commit", "-q", "-m", "one"]);
    let file = versioned.path().join("spec/anomalies/AN-001.md");
    let original = std::fs::read_to_string(&file).unwrap();
    std::fs::write(&file, original.replace("Ships behind a flag.", "Revised.")).unwrap();
    git(versioned.path(), &["commit", "-q", "-am", "two"]);
    std::fs::write(&file, &original).unwrap();
    git(versioned.path(), &["commit", "-q", "-am", "three"]);
    let with_git = versioned.execute(&versioned.run());
    assert!(with_git.status.success(), "{}", text(&with_git.stderr));
    let written = std::fs::read(versioned.aux("snapshot.json")).unwrap();

    assert_eq!(without_git, written);
    let snapshot = text(&written);
    assert!(!snapshot.contains("fixingChange") && !snapshot.contains("rationaleChange"));
}

#[trace("FR-166-AC-3")]
#[test]
fn a_branch_name_a_short_hash_and_an_upper_case_hash_are_not_baseline_commits() {
    let repo = Repo::bare();
    for bad in ["main", "0123456", &BASE.to_uppercase()] {
        let refused = repo.refusal(&repo.run().set("--baseline-commit", bad));
        assert_eq!(refused["code"], "release_snapshot_baseline_commit_invalid");
    }
}

#[trace("FR-166-AC-4")]
#[test]
fn an_existing_out_is_refused_and_left_unchanged() {
    let repo = Repo::bare();
    let out = repo.aux("snapshot.json");
    std::fs::write(&out, "kept").unwrap();
    let output = repo.execute(&repo.run());
    assert_eq!(output.status.code(), Some(2));
    let diagnostics: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostics[0]["code"], "release_snapshot_exists");
    assert_eq!(std::fs::read(&out).unwrap(), b"kept");
}

#[trace("FR-166-AC-5")]
#[test]
fn a_missing_history_and_a_malformed_binding_are_refused_naming_the_input() {
    let repo = Repo::bare();
    let missing = repo.refusal(&repo.run().set("--history", "/nonexistent/history.json"));
    assert_eq!(missing["code"], "release_snapshot_input_unreadable");
    assert_eq!(missing["context"]["input"], "--history");

    let malformed = repo.aux("malformed.json");
    std::fs::write(&malformed, "{ not json").unwrap();
    let invalid = repo.refusal(&repo.run().set("--binding", malformed.to_str().unwrap()));
    assert_eq!(invalid["code"], "release_snapshot_input_invalid");
    assert_eq!(invalid["context"]["input"], "--binding");

    let not_a_binding = repo.aux("not-a-binding.json");
    std::fs::write(&not_a_binding, "{}").unwrap();
    let wrong = repo.refusal(&repo.run().set("--binding", not_a_binding.to_str().unwrap()));
    assert_eq!(wrong["code"], "release_snapshot_input_invalid");
    assert_eq!(wrong["context"]["input"], "--binding");
}

#[trace("FR-166-AC-6")]
#[test]
fn coverage_soup_and_anomaly_members_equal_the_views_they_are_built_from() {
    let repo = full();
    let snapshot = repo.snapshot(&repo.run());

    let matrix: Value = serde_json::from_str(&repo.ok(&[
        "matrix",
        "--repo",
        repo.path().to_str().unwrap(),
        "--json",
    ]))
    .unwrap();
    for requirement in matrix["requirements"].as_array().unwrap() {
        let mine = snapshot["coverage"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["id"] == requirement["id"])
            .unwrap();
        for (index, criterion) in requirement["criteria"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            for member in ["id", "static_status", "evidence_status"] {
                assert_eq!(
                    mine["criteria"][index][member], criterion[member],
                    "{member}"
                );
            }
        }
    }

    let report: Value = serde_json::from_str(&repo.ok(&[
        "soup",
        "report",
        "--repo",
        repo.path().to_str().unwrap(),
        "--binding",
        repo.aux("binding.json").to_str().unwrap(),
        "--as-of",
        AS_OF,
        "--json",
    ]))
    .unwrap();
    let rows = report["rows"].as_array().unwrap();
    assert_eq!(
        snapshot["soup"]["rows"].as_array().unwrap().len(),
        rows.len()
    );
    for (mine, theirs) in snapshot["soup"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .zip(rows)
    {
        for member in ["purl", "version", "support_status", "end_of_support"] {
            assert_eq!(mine[member], theirs[member], "{member}");
        }
        let group = if theirs["unresolved_advisory"] == true
            || theirs["unresolved_policy_finding"] == true
        {
            "unresolved_advisory"
        } else if theirs["past_end_of_support"] == true {
            "past_end_of_support"
        } else {
            "other"
        };
        assert_eq!(mine["group"], group);
    }

    // No anomaly report exists to compare with; the authored values are the
    // oracle.
    assert_eq!(
        snapshot["anomalies"],
        json!([{
            "id": "AN-001", "title": "Title of AN-001", "status": "assessed",
            "resolution": null, "severity": "major", "safety_impact": "impacting",
            "unassessed": false,
            "release_rationale": [
                {"author": "Dana Lee", "rationale": "Ships behind a flag."},
                {"author": "ops-lead", "rationale": "Reviewed with QA."},
            ],
        }])
    );
}

#[trace("FR-166-AC-8")]
#[test]
fn support_data_moves_an_unknown_end_of_support_and_a_records_directory_is_read() {
    let repo = Repo::bare();
    let elsewhere = repo.aux("records");
    std::fs::create_dir_all(&elsewhere).unwrap();
    std::fs::write(
        elsewhere.join("old.md"),
        "---\nid: SOUP-old\ntitle: old\ntype: soup_record\nobject: soup_record\npurl: \"pkg:cargo/old@0.1.0\"\nend_of_support: unknown\n---\n# old\n",
    )
    .unwrap();
    let support = repo.aux("support.json");
    std::fs::write(
        &support,
        json!([{"purl": "pkg:cargo/old@0.1.0", "support_status": "unsupported",
                "end_of_support": "2026-05-31", "source": "https://example.invalid/old"}])
        .to_string(),
    )
    .unwrap();
    let records = elsewhere.to_str().unwrap();
    let with = repo.snapshot(
        &repo
            .run()
            .set("--records", records)
            .set("--support", support.to_str().unwrap()),
    );
    assert_eq!(with["soup"]["rows"][0]["group"], "past_end_of_support");
    std::fs::remove_file(repo.aux("snapshot.json")).unwrap();
    let without = repo.snapshot(&repo.run().set("--records", records));
    assert_eq!(without["soup"]["rows"][0]["group"], "other");
    // The default directory holds no record, so it is a different run.
    std::fs::remove_file(repo.aux("snapshot.json")).unwrap();
    let default = repo.snapshot(&repo.run());
    assert_eq!(default["soup"]["rows"], json!([]));
}

#[trace("FR-166-AC-9")]
#[test]
fn an_anomaly_without_a_record_and_a_malformed_history_are_refused_by_their_own_codes() {
    let repo = full();
    repo.write(
        "spec/anomalies/AN-002.md",
        "---\nid: AN-002\ntype: Anomaly\nstatus: identified\n---\n# no safety_impact\n",
    );
    let refused = repo.refusal(&repo.run());
    assert_eq!(refused["code"], "release_snapshot_anomaly_unreadable");
    assert_eq!(refused["context"]["path"], "spec/anomalies/AN-002.md");
    assert_eq!(refused["context"]["code"], "missing");
    std::fs::remove_file(repo.path().join("spec/anomalies/AN-002.md")).unwrap();
    repo.snapshot(&repo.run());

    std::fs::remove_file(repo.aux("snapshot.json")).unwrap();
    std::fs::write(repo.aux("history.json"), "{ not json").unwrap();
    let history = repo.refusal(&repo.run());
    assert_eq!(history["code"], "release_snapshot_history_invalid");
}

#[trace("FR-167-AC-2", "FR-167-AC-3")]
#[test]
fn bound_stale_and_undischarged_criteria_are_counted_and_their_findings_listed() {
    let repo = full();
    let snapshot = repo.snapshot(&repo.run());
    let statuses: Vec<(&str, &str)> = snapshot["coverage"][0]["criteria"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["id"].as_str().unwrap(),
                c["evidence_status"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        statuses,
        [
            ("FR-001-AC-1", "bound"),
            ("FR-001-AC-2", "stale"),
            ("FR-001-AC-3", "undischarged"),
        ]
    );
    assert_eq!(
        snapshot["evidence_freshness"],
        json!({"bound": 1, "stale": 1, "suspect": 0, "undischarged": 1, "no run evidence": 0})
    );
    let obligations: Vec<&str> = snapshot["open_findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["obligation"].as_str().unwrap())
        .collect();
    assert!(obligations.contains(&"FR-001-AC-2") && obligations.contains(&"FR-001-AC-3"));
    assert!(!obligations.contains(&"FR-001-AC-1"));
    assert!(obligations.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[trace("FR-167-AC-4", "FR-142-AC-21")]
#[test]
fn soup_rows_follow_the_group_order_and_unassessed_items_are_listed_after_them() {
    let repo = full();
    let snapshot = repo.snapshot(&repo.run());
    let rows: Vec<(&str, &str)> = snapshot["soup"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["purl"].as_str().unwrap(), r["group"].as_str().unwrap()))
        .collect();
    assert_eq!(
        rows,
        [
            ("pkg:cargo/bytes@1.0.0", "unresolved_advisory"),
            ("pkg:cargo/old@0.1.0", "past_end_of_support"),
            ("pkg:cargo/anyhow@1.0.102", "other"),
            ("pkg:cargo/zed@1.0.0", "other"),
        ]
    );
    let items: Vec<(&str, &str, &str)> = snapshot["soup"]["not_assessed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["kind"].as_str().unwrap(),
                i["tool"].as_str().unwrap(),
                i["detail"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        items,
        [
            ("scan_not_run", "cargo-deny", "not run"),
            ("scan_not_run", "licence-policy", "not run"),
            ("scan_not_run", "osv-scanner", "not run"),
            (
                "unmatched_advisory",
                "cargo-audit",
                "RUSTSEC-2026-0002|pkg:cargo/ghost@9.9.9"
            ),
        ]
    );
}

#[trace("FR-167-AC-5", "FR-167-AC-18", "FR-167-AC-23")]
#[test]
fn snapshot_lists_fr162_carried_and_fixed_anomalies_with_gate_only_on_carried() {
    let repo = Repo::bare();
    let rationale = "## Release Rationale\n\n| Author | Rationale |\n|--------|-----------|\n| Dana LEE | Mixed Case stays. |\n| Sam | Second row. |\n";
    repo.anomaly("AN-003", "identified", "unassessed", &[], "");
    repo.anomaly(
        "AN-001",
        "assessed",
        "impacting",
        &["severity: minor"],
        rationale,
    );
    repo.anomaly(
        "AN-002",
        "closed",
        "impacting",
        &[
            "resolution: wont-fix",
            "closed_by: \"Pat\"",
            "severity: major",
        ],
        "",
    );
    repo.anomaly(
        "AN-004",
        "closed",
        "impacting",
        &["resolution: fixed", "closed_by: \"Pat\"", "severity: major"],
        "",
    );
    repo.anomaly(
        "AN-005",
        "closed",
        "none",
        &[
            "resolution: duplicate",
            "closed_by: \"Pat\"",
            "severity: minor",
        ],
        "",
    );
    repo.anomaly(
        "AN-006",
        "closed",
        "unassessed",
        &[
            "resolution: duplicate",
            "closed_by: \"Pat\"",
            "severity: minor",
        ],
        "",
    );
    repo.anomaly(
        "AN-007",
        "closed",
        "unassessed",
        &["resolution: fixed", "closed_by: \"Pat\"", "severity: minor"],
        "",
    );
    let snapshot = repo.snapshot(&repo.run());
    assert_anomaly_snapshot_projection(&snapshot, &repo);
}

fn assert_anomaly_snapshot_projection(snapshot: &serde_json::Value, repo: &Repo) {
    let anomalies = snapshot["anomalies"].as_array().unwrap();
    let ids: Vec<&str> = anomalies
        .iter()
        .map(|a| a["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        ["AN-001", "AN-002", "AN-003", "AN-004", "AN-006", "AN-007"]
    );
    assert_eq!(
        anomalies[0]["release_rationale"],
        json!([
            {"author": "Dana LEE", "rationale": "Mixed Case stays."},
            {"author": "Sam", "rationale": "Second row."},
        ])
    );
    assert_eq!(anomalies[1]["release_rationale"], json!([]));
    assert_eq!(
        anomalies
            .iter()
            .map(|anomaly| anomaly["unassessed"].clone())
            .collect::<Vec<_>>(),
        [
            json!(false),
            json!(false),
            json!(true),
            json!(false),
            json!(true),
            json!(true)
        ]
    );
    assert!(
        snapshot["open_findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|finding| {
                finding["obligation"] == "AN-007" && finding["kind"] == "anomaly-unassessed"
            })
    );
    assert!(
        snapshot["open_findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|finding| {
                finding["obligation"] != "AN-004"
                    || !finding["kind"]
                        .as_str()
                        .is_some_and(|kind| kind.starts_with("anomaly-"))
            })
    );
    assert!(
        !std::fs::read_to_string(repo.aux("snapshot.json"))
            .unwrap()
            .contains("rationaleChange")
    );
}

#[trace("FR-167-AC-6")]
#[test]
fn an_empty_store_reports_no_run_evidence_and_every_section_is_present() {
    let repo = Repo::bare();
    let snapshot = repo.snapshot(&repo.run());
    let criteria = snapshot["coverage"][0]["criteria"].as_array().unwrap();
    assert_eq!(criteria.len(), 3);
    assert!(
        criteria
            .iter()
            .all(|c| c["evidence_status"] == "no run evidence")
    );
    assert_eq!(snapshot["evidence_freshness"]["no run evidence"], 3);
    let findings = snapshot["open_findings"].as_array().unwrap();
    assert_eq!(findings.len(), 3);
    assert!(
        findings.iter().all(|f| f["kind"] == "undischarged"),
        "{findings:?}"
    );
    assert_eq!(snapshot["soup"]["rows"], json!([]));
    assert_eq!(snapshot["anomalies"], json!([]));
    assert_eq!(snapshot["change_records"], json!([]));
    assert_eq!(snapshot["previous_release"], Value::Null);
}

#[trace("FR-168-AC-1", "FR-168-AC-2", "FR-168-AC-5")]
#[test]
fn change_records_are_copied_in_order_and_an_empty_list_is_a_snapshot() {
    let repo = Repo::bare();
    let entry = |id: &str, revision: u64, commit: &str| json!({"record_id": id, "revision": revision, "path": format!("r/{id}.md"), "commit": commit});
    repo.history(
        &[
            entry("CR-2", 1, "c3"),
            entry("CR-1", 10, "c2"),
            entry("CR-1", 2, "c1"),
        ],
        &json!({"tag": "v1.0.0", "commit": "e".repeat(40)}),
    );
    let snapshot = repo.snapshot(&repo.run());
    assert_eq!(
        snapshot["change_records"],
        json!([
            entry("CR-1", 2, "c1"),
            entry("CR-1", 10, "c2"),
            entry("CR-2", 1, "c3")
        ])
    );
    assert_eq!(
        snapshot["previous_release"],
        json!({"tag": "v1.0.0", "commit": "e".repeat(40)})
    );

    std::fs::remove_file(repo.aux("snapshot.json")).unwrap();
    repo.history(&[], &Value::Null);
    let empty = repo.snapshot(&repo.run());
    assert_eq!(empty["change_records"], json!([]));
    assert_eq!(empty["previous_release"], Value::Null);
}

#[trace("FR-168-AC-3")]
#[test]
fn a_history_file_of_another_shape_is_refused_naming_the_fault_and_writes_no_file() {
    let repo = Repo::bare();
    let entry = json!({"record_id": "CR-1", "revision": 1, "path": "p", "commit": "c1"});
    let document = |edit: &dyn Fn(&mut Value)| {
        let mut document = json!({
            "protocol": "engineering-assurance.release-history/v1",
            "baseline_commit": BASE, "previous_release": null, "records": [entry.clone()],
        });
        edit(&mut document);
        document
    };
    let cases: [(Value, &str); 4] = [
        (
            document(&|d| d["protocol"] = json!("other/v1")),
            "protocol other/v1",
        ),
        (
            document(&|d| {
                d.as_object_mut().unwrap().remove("records");
            }),
            "records",
        ),
        (
            document(&|d| {
                d["records"][0].as_object_mut().unwrap().remove("commit");
            }),
            "records[0].commit",
        ),
        (
            document(&|d| {
                let mut second = entry.clone();
                second["commit"] = json!("c2");
                d["records"] = json!([entry.clone(), second]);
            }),
            "CR-1 revision 1 at commits c1 and c2",
        ),
    ];
    for (history, member) in cases {
        repo.history_document(&history);
        let refused = repo.refusal(&repo.run());
        assert_eq!(refused["code"], "release_snapshot_history_invalid");
        assert_eq!(refused["context"]["member"], member);
    }
}

#[trace("FR-168-AC-6")]
#[test]
fn a_history_for_another_baseline_is_refused_naming_both_values_without_git() {
    let repo = Repo::bare();
    let other = "f".repeat(40);
    repo.history_document(&json!({
        "protocol": "engineering-assurance.release-history/v1",
        "baseline_commit": other, "previous_release": null, "records": [],
    }));
    let output = repo.execute_with(&repo.run(), &[("PATH", "")]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!repo.aux("snapshot.json").exists());
    let diagnostics: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(
        diagnostics[0]["code"],
        "release_snapshot_history_baseline_mismatch"
    );
    assert_eq!(diagnostics[0]["context"]["history_baseline_commit"], other);
    assert_eq!(diagnostics[0]["context"]["baseline_commit"], BASE);
}

fn walk_keys(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                found.push(key.clone());
                walk_keys(inner, found);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| walk_keys(item, found)),
        _ => {}
    }
}

#[trace("NFR-036-M-1", "NFR-036-M-2", "NFR-036-M-3", "FR-167-AC-7")]
#[test]
fn two_runs_in_other_directories_and_environments_write_the_same_bytes_and_no_verdict() {
    let repo = full();
    let first = repo.execute_with(&repo.run(), &[("TZ", "UTC"), ("LC_ALL", "C")]);
    assert!(first.status.success(), "{}", text(&first.stderr));
    let first_bytes = std::fs::read(repo.aux("snapshot.json")).unwrap();
    std::fs::remove_file(repo.aux("snapshot.json")).unwrap();

    let elsewhere = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
    command.args(["release", "snapshot"]);
    for (flag, value) in &repo.run().flags {
        command.arg(flag).arg(value);
    }
    let second = command
        .current_dir(elsewhere.path())
        .env("IX_HOME", repo.home.path())
        .env("TZ", "Pacific/Auckland")
        .env("LC_ALL", "C.UTF-8")
        .env("HOSTNAME", "another-host")
        .output()
        .unwrap();
    assert!(second.status.success(), "{}", text(&second.stderr));
    assert_eq!(
        std::fs::read(repo.aux("snapshot.json")).unwrap(),
        first_bytes
    );

    let snapshot: Value = serde_json::from_slice(&first_bytes).unwrap();
    let mut found = Vec::new();
    walk_keys(&snapshot, &mut found);
    for key in &found {
        for forbidden in [
            "digest", "hash", "manifest", "score", "rank", "verdict", "percent",
        ] {
            assert!(!key.contains(forbidden), "{key}");
        }
    }
    let mut body = text(&first_bytes).to_lowercase();
    for token in [
        "accepted-outside-acceptable-region",
        "benefit-risk-not-accepted",
        "not-accepted-inside-acceptable-region",
    ] {
        body = body.replace(token, "");
    }
    for word in ["accepted", "releasable", "ready"] {
        assert!(!body.contains(word), "{word}");
    }
    assert!(
        !body.contains(repo.path().to_str().unwrap()),
        "no absolute path"
    );
}

/// A malformed persisted scan timestamp must be refused, not written as new evidence.
fn audit_scan_at(timestamp: &str) -> FindingRecord {
    let mut scan = audit_scan(&[("RUSTSEC-2026-0001", "bytes", "1.0.0")]);
    timestamp.clone_into(&mut scan.timestamp);
    scan
}

#[trace("FR-166-AC-10", "FR-167-AC-4", "FR-030-AC-29")]
#[test]
fn a_persisted_scan_with_a_noncanonical_timestamp_refuses_the_snapshot() {
    let repo = Repo::bare();
    repo.soup_record("bytes", "pkg:cargo/bytes@1.0.0", &["version: \"1.0.0\""]);
    let dirty = audit_scan_at("last tuesday");
    let relative = quoin_evidence::paths::record_path("scans", &dirty.suite, &dirty.timestamp);
    let path = repo.path().join("spec/evidence").join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_vec(&dirty).unwrap()).unwrap();
    let refused = repo.refusal(&repo.run());
    assert_eq!(refused["code"], "release_snapshot_input_unreadable");
    assert!(
        refused["message"]
            .as_str()
            .unwrap()
            .contains("last~20tuesday")
    );
}

#[trace("FR-167-AC-4")]
#[test]
fn a_policy_finding_that_matches_no_record_is_listed_and_never_names_a_host_path() {
    let repo = Repo::bare();
    repo.soup_record("bytes", "pkg:cargo/bytes@1.0.0", &["version: \"1.0.0\""]);
    let sbom = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../quoin-evidence/tests/fixtures/licence-npm.syft.cdx.json");
    let policy = repo.aux("policy.yaml");
    std::fs::write(
        &policy,
        "format: licence-policy-v1\nallow: [MIT]\ndeny: [GPL-3.0-only]\nban:\n  - purl: pkg:npm/fx-vale\n",
    )
    .unwrap();
    let root = repo.path().to_str().unwrap();
    repo.ok(&[
        "evidence",
        "record",
        "--repo",
        root,
        "--module",
        root,
        "--suite",
        "SUITE-LIC",
        "--tool",
        "licence-policy",
        "--results",
        sbom.to_str().unwrap(),
        "--policy",
        policy.to_str().unwrap(),
        "--timestamp",
        "2026-01-01T00:00:00Z",
    ]);
    let snapshot = repo.snapshot(&repo.run());
    let details: Vec<&str> = snapshot["soup"]["not_assessed"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "unmatched_policy_finding")
        .map(|i| i["detail"].as_str().unwrap())
        .collect();
    assert!(!details.is_empty(), "{snapshot}");
    assert!(
        details.windows(2).all(|pair| pair[0] <= pair[1]),
        "bytewise order"
    );
    for detail in details {
        let (_, purl) = detail.split_once('|').unwrap();
        assert!(purl.is_empty() || purl.starts_with("pkg:"), "{detail}");
    }
}

#[trace("FR-166-AC-10")]
#[test]
fn a_retained_scan_that_cannot_be_read_refuses_the_snapshot_naming_the_file() {
    let repo = full();
    let suite = repo.path().join("spec/evidence/scans/audit");
    std::fs::write(suite.join("2999-01-01T00-00-00Z.json"), "{ not a scan").unwrap();
    let refused = repo.refusal(&repo.run());
    assert_eq!(refused["code"], "release_snapshot_input_unreadable");
    assert_eq!(refused["context"]["input"], "--repo");
    assert!(
        refused["message"]
            .as_str()
            .unwrap()
            .contains("scans/audit/2999-01-01T00-00-00Z.json"),
        "{refused}"
    );
}

#[trace("FR-167-AC-3")]
#[test]
fn a_failing_test_run_is_not_counted_as_bound() {
    let repo = Repo::bare();
    let results = repo.aux("failing.xml");
    std::fs::write(
        &results,
        r#"<testsuite name="unit"><testcase name="unit_case"><properties><property name="trace" value="FR-001-AC-1"/></properties><failure message="boom"/></testcase></testsuite>"#,
    )
    .unwrap();
    let root = repo.path().to_str().unwrap();
    repo.ok(&[
        "evidence",
        "record",
        "--repo",
        root,
        "--module",
        root,
        "--suite",
        "unit",
        "--tool",
        "fixture",
        "--adapter",
        "junit",
        "--results",
        results.to_str().unwrap(),
        "--timestamp",
        "2026-01-01T00:00:00Z",
    ]);
    let snapshot = repo.snapshot(&repo.run());
    let first = &snapshot["coverage"][0]["criteria"][0];
    assert_eq!(first["id"], "FR-001-AC-1");
    assert_ne!(first["evidence_status"], "bound", "{first}");
    assert_eq!(snapshot["evidence_freshness"]["bound"], 0);
    assert!(
        snapshot["open_findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["obligation"] == "FR-001-AC-1"),
        "the failed criterion has an open finding"
    );
}

#[cfg(unix)]
#[trace("FR-166-AC-5")]
#[test]
fn a_support_file_that_exists_but_cannot_be_read_is_unreadable_not_a_failure() {
    use std::os::unix::fs::PermissionsExt;
    let repo = Repo::bare();
    let support = repo.aux("support.json");
    std::fs::write(&support, "[]").unwrap();
    std::fs::set_permissions(&support, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read(&support).is_ok() {
        return; // a user that ignores file permissions cannot make a file unreadable
    }
    let refused = repo.refusal(&repo.run().set("--support", support.to_str().unwrap()));
    assert_eq!(refused["code"], "release_snapshot_input_unreadable");
    assert_eq!(refused["context"]["input"], "--support");
}

#[trace("FR-144-AC-13")]
#[test]
fn soup_report_with_a_missing_records_dir_and_a_bad_binding_refuses_the_records_first() {
    let repo = Repo::bare();
    let bad = repo.aux("bad.json");
    std::fs::write(&bad, "{}").unwrap();
    let output = repo.quoin(&[
        "soup",
        "report",
        "--repo",
        repo.path().to_str().unwrap(),
        "--records",
        "/nonexistent/records",
        "--binding",
        bad.to_str().unwrap(),
        "--as-of",
        AS_OF,
    ]);
    assert_eq!(output.status.code(), Some(2), "{}", text(&output.stderr));
    let diagnostics: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostics[0]["code"], "soup.path_missing");
}

/// Snapshot `open_findings` carries `missing-run` and `failed-run` causes
/// byte-for-byte, and at least one of each appears (FR-167-AC-10).
#[trace("FR-167-AC-10")]
#[test]
fn tc_1760_snapshot_carries_stale_causes_byte_for_byte() {
    let repo = full();
    record_failed_run(&repo);
    let snapshot = repo.snapshot(&repo.run());

    let stale: Vec<(&str, &str)> = snapshot["open_findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["kind"] == "stale-evidence")
        .map(|finding| {
            (
                finding["obligation"].as_str().unwrap(),
                finding["cause"].as_str().unwrap_or("<absent>"),
            )
        })
        .collect();
    // AC-2's run is gone; AC-3's run failed. Each carries its own cause.
    assert_eq!(
        stale,
        [
            ("FR-001-AC-2", "missing-run"),
            ("FR-001-AC-3", "failed-run"),
        ]
    );
}

/// Bind AC-3 to the `broken` suite, then record a newer failed run for it.
fn record_failed_run(repo: &Repo) {
    let results = repo.aux("broken.xml");
    std::fs::write(
        &results,
        r#"<testsuite name="broken"><testcase name="broken_case"><properties><property name="trace" value="FR-001-AC-3"/></properties></testcase></testsuite>"#,
    )
    .unwrap();
    repo.ok(&[
        "evidence",
        "record",
        "--repo",
        repo.path().to_str().unwrap(),
        "--module",
        repo.path().to_str().unwrap(),
        "--suite",
        "broken",
        "--tool",
        "fixture",
        "--adapter",
        "junit",
        "--results",
        results.to_str().unwrap(),
        "--timestamp",
        "2026-01-01T00:00:00Z",
    ]);
    std::fs::write(
        &results,
        r#"<testsuite name="broken"><testcase name="broken_case"><properties><property name="trace" value="FR-001-AC-3"/></properties><failure/></testcase></testsuite>"#,
    )
    .unwrap();
    repo.ok(&[
        "evidence",
        "record",
        "--repo",
        repo.path().to_str().unwrap(),
        "--module",
        repo.path().to_str().unwrap(),
        "--suite",
        "broken",
        "--tool",
        "fixture",
        "--adapter",
        "junit",
        "--results",
        results.to_str().unwrap(),
        "--timestamp",
        "2026-01-02T00:00:00Z",
    ]);
}

/// A non-stale `open_findings` item has no `cause` key in JSON (FR-167-AC-11).
#[trace("FR-167-AC-11")]
#[test]
fn tc_1760_snapshot_non_stale_has_no_cause_key() {
    let repo = full();
    let snapshot = repo.snapshot(&repo.run());

    let non_stale: Vec<&Value> = snapshot["open_findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["kind"] != "stale-evidence")
        .collect();
    // AC-3 is undischarged, so the fixture does hold a non-stale finding.
    assert!(
        !non_stale.is_empty(),
        "the fixture must hold a non-stale finding"
    );
    for finding in non_stale {
        assert!(
            finding.get("cause").is_none(),
            "non-stale finding must have no cause key: {finding}"
        );
    }
}

fn safety_module() -> String {
    let checkout = std::env::var_os("SPEC_OBJECTS_SAFETY_DIR").map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-objects-safety"),
        PathBuf::from,
    );
    let root = checkout.join("spec_objects_safety");
    assert!(
        root.join("manifest.yaml").is_file(),
        "the spec-objects-safety module is not at {}: check it out next to this repository or set SPEC_OBJECTS_SAFETY_DIR",
        root.display()
    );
    root.display().to_string()
}

/// A risk record under `spec/risk/`.
fn risk_record(repo: &Repo, id: &str, kind: &str, front: &str, body: &str) {
    repo.write(
        &format!("spec/risk/{id}.md"),
        &format!(
            "---\nid: {id}\ntitle: \"{id}\"\ntype: {kind}\nobject: {kind}\n{front}---\n# [{id}] {id}\n\n{body}\
             \n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|---|---|---|---|\n| record_id | UUID | 1..1 | identity |\n"
        ),
    );
}

fn estimate(repo: &Repo, id: &str, situation: &str, control: Option<&str>) {
    let assumes = control.map_or_else(String::new, |control| {
        format!("  - target: {control}\n    type: assumes\n")
    });
    risk_record(
        repo,
        id,
        "risk_estimate",
        &format!(
            "phase: post_control\nrelationships:\n  - target: {situation}\n    type: estimates\n{assumes}"
        ),
        "## Judgement\n\n| Severity | Probability | Rationale |\n| --- | --- | --- |\n| Serious | Likely | why |\n\n\
         ## Provenance\n\n| Asserted by | Asserted at |\n| --- | --- |\n| Pat | 2026-01-01 |\n",
    );
}

/// A hazard, two situations (each with a post-control estimate assuming
/// CTRL-1) and a control that nothing verifies.
fn unverified_control(repo: &Repo) {
    risk_record(repo, "HAZ-1", "hazard", "", "");
    risk_record(repo, "CTRL-1", "control", "", "");
    for n in 1..=2 {
        risk_record(
            repo,
            &format!("SIT-{n}"),
            "hazardous_situation",
            "relationships:\n  - target: HAZ-1\n    type: arises_from\n",
            "",
        );
        estimate(
            repo,
            &format!("EST-{n}"),
            &format!("SIT-{n}"),
            Some("CTRL-1"),
        );
    }
}

fn with_risk(repo: &Repo) -> Run {
    repo.run().set("--module", &safety_module())
}

fn findings_of<'a>(snapshot: &'a Value, kind: &str) -> Vec<&'a Value> {
    snapshot["open_findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["kind"] == kind)
        .collect()
}

/// The FR-170 tokens, which are the kinds of the risk-completeness items.
const RISK_TOKENS: [&str; 13] = [
    "hazard-without-situation",
    "situation-without-hazard",
    "situation-without-harm",
    "control-without-requirement",
    "control-without-verification",
    "post-control-without-control",
    "post-control-estimate-without-decision",
    "residual-risk-undecided",
    "decision-without-region",
    "accepted-outside-acceptable-region",
    "benefit-risk-undecided",
    "benefit-risk-not-accepted",
    "not-accepted-inside-acceptable-region",
];

/// The distinct `(token, names)` of `quoin risk-summary` over the repository.
fn summary_findings(repo: &Repo) -> Vec<(String, Vec<String>)> {
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["risk-summary", "--format", "json"])
        .arg(repo.path())
        .args(["--module", &safety_module()])
        .env("IX_HOME", repo.home.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut found: Vec<(String, Vec<String>)> = report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|row| row["findings"].as_array().unwrap())
        .map(|finding| {
            (
                finding["finding"].as_str().unwrap().to_owned(),
                finding["names"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|name| name.as_str().unwrap().to_owned())
                    .collect(),
            )
        })
        .collect();
    found.sort();
    found.dedup();
    found
}

/// The `(anomaly, path)` failures of `quoin anomaly report --fail-on <mode>`.
fn gate_failures(repo: &Repo, mode: &str) -> Vec<(String, String)> {
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["anomaly", "report", "--fail-on", mode, "--json"])
        .current_dir(repo.path())
        .env("IX_HOME", repo.home.path())
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    report["gate"]["failures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|failure| {
            (
                failure["anomaly"].as_str().unwrap().to_owned(),
                failure["path"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// The snapshot's risk-completeness items are exactly the distinct findings
/// of `quoin risk-summary`, each mapped as FR-167 states.
fn assert_risk_items_equal_summary(repo: &Repo, snapshot: &Value) {
    let mut items: Vec<(String, Vec<String>)> = snapshot["open_findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| RISK_TOKENS.contains(&item["kind"].as_str().unwrap()))
        .map(|item| {
            assert_eq!(item["path"], Value::Null);
            assert_eq!(item["line"], Value::Null);
            assert_eq!(item["symbol"], Value::Null);
            assert!(item.get("cause").is_none());
            let names: Vec<String> = item["summary"]
                .as_str()
                .unwrap()
                .split(", ")
                .map(str::to_owned)
                .collect();
            assert_eq!(item["obligation"], json!(names[0]));
            (item["kind"].as_str().unwrap().to_owned(), names)
        })
        .collect();
    items.sort();
    assert_eq!(items, summary_findings(repo));
}

/// A policy, a hazard, a situation with a harm, a satisfied and verified
/// control, a pre-control estimate, a post-control estimate assuming the
/// control, and a decided residual risk inside the acceptable region.
fn complete_chain(repo: &Repo) {
    risk_record(
        repo,
        "POL-1",
        "acceptability_policy",
        "",
        "## Severity levels\n\n| Severity level |\n| --- |\n| Minor |\n| Serious |\n\n\
         ## Probability levels\n\n| Probability level |\n| --- |\n| Rare |\n| Likely |\n\n\
         ## Regions\n\n| Region | Outside acceptable |\n| --- | --- |\n| Low | false |\n| Mid | false |\n| High | true |\n\n\
         ## Cells\n\n| Severity | Probability | Region |\n| --- | --- | --- |\n\
         | Minor | Rare | Low |\n| Minor | Likely | Low |\n| Serious | Rare | Mid |\n| Serious | Likely | High |\n",
    );
    risk_record(repo, "HAZ-1", "hazard", "status: accepted\n", "");
    risk_record(
        repo,
        "SIT-1",
        "hazardous_situation",
        "relationships:\n  - target: HAZ-1\n    type: arises_from\n",
        "",
    );
    risk_record(
        repo,
        "HARM-1",
        "harm",
        "relationships:\n  - target: SIT-1\n    type: arises_from\n",
        "",
    );
    risk_record(repo, "CTRL-1", "control", "", "");
    risk_record(
        repo,
        "REQ-9",
        "FR",
        "relationships:\n  - target: CTRL-1\n    type: satisfies\n",
        "| ID | Criteria |\n|---|---|\n| REQ-9-AC-1 | synthetic |\n",
    );
    risk_record(
        repo,
        "TC-9",
        "TC",
        "relationships:\n  - target: CTRL-1\n    type: verifies\n",
        "result: failed\n",
    );
    risk_estimate(repo, "EST-A", "pre_control", "Serious", &["SIT-1"]);
    risk_estimate(repo, "EST-B", "post_control", "Minor", &["SIT-1", "CTRL-1"]);
    residual_risk(repo, "RR-1", "EST-B", "accepted");
}

fn risk_estimate(repo: &Repo, id: &str, phase: &str, severity: &str, targets: &[&str]) {
    let probability = if severity == "Minor" {
        "Rare"
    } else {
        "Likely"
    };
    let mut edges = String::new();
    for target in targets {
        let verb = if target.starts_with("SIT") {
            "estimates"
        } else {
            "assumes"
        };
        let _ = writeln!(edges, "  - target: {target}\n    type: {verb}");
    }
    risk_record(
        repo,
        id,
        "risk_estimate",
        &format!("phase: {phase}\nrelationships:\n{edges}"),
        &format!(
            "## Judgement\n\n| Severity | Probability | Rationale |\n| --- | --- | --- |\n| {severity} | {probability} | why |\n\n\
             ## Provenance\n\n| Asserted by | Asserted at |\n| --- | --- |\n| Pat | 2026-01-01 |\n"
        ),
    );
}

fn residual_risk(repo: &Repo, id: &str, estimate: &str, decision: &str) {
    risk_record(
        repo,
        id,
        "residual_risk",
        &format!(
            "relationships:\n  - target: {estimate}\n    type: estimate\n  - target: POL-1\n    type: references\n"
        ),
        &format!(
            "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| {decision} | r | Pat |\n"
        ),
    );
}

#[trace("FR-167-AC-15", "FR-167-AC-17")]
#[test]
fn an_unverified_control_on_two_rows_is_one_open_finding_equal_to_the_risk_summary_token() {
    let repo = Repo::bare();
    unverified_control(&repo);
    let snapshot = repo.snapshot(&with_risk(&repo));
    let items = findings_of(&snapshot, "control-without-verification");
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(
        items[0],
        &json!({
            "obligation": "CTRL-1", "kind": "control-without-verification",
            "summary": "CTRL-1", "path": null, "line": null, "symbol": null,
        })
    );
    assert!(summary_findings(&repo).contains(&(
        "control-without-verification".to_owned(),
        vec!["CTRL-1".to_owned()]
    )));
    assert_risk_items_equal_summary(&repo, &snapshot);
}

#[trace("FR-167-AC-16")]
#[test]
fn a_verified_control_and_a_complete_chain_yield_no_risk_completeness_item() {
    let repo = Repo::bare();
    unverified_control(&repo);
    risk_record(
        &repo,
        "VER-1",
        "FR",
        "relationships:\n  - target: CTRL-1\n    type: verifies\n",
        "| ID | Criteria |\n|---|---|\n| VER-1-AC-1 | synthetic |\n",
    );
    let snapshot = repo.snapshot(&with_risk(&repo));
    assert_eq!(
        findings_of(&snapshot, "control-without-verification"),
        [] as [&serde_json::Value; 0]
    );

    let chain = Repo::bare();
    complete_chain(&chain);
    assert_eq!(summary_findings(&chain), []);
    let snapshot = chain.snapshot(&with_risk(&chain));
    let kinds: Vec<&str> = snapshot["open_findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["kind"].as_str().unwrap())
        .collect();
    assert!(
        kinds.iter().all(|kind| !RISK_TOKENS.contains(kind)),
        "{kinds:?}"
    );
}

#[trace("FR-167-AC-17")]
#[test]
fn a_finding_naming_several_records_is_one_item_with_the_first_name_and_all_names() {
    let repo = Repo::bare();
    complete_chain(&repo);
    risk_estimate(
        &repo,
        "EST-H",
        "post_control",
        "Serious",
        &["SIT-1", "CTRL-1"],
    );
    residual_risk(&repo, "RR-2", "EST-H", "accepted");
    risk_record(
        &repo,
        "BR-1",
        "benefit_risk",
        "relationships:\n  - target: RR-2\n    type: references\n",
        "## Determination\n\n| Benefit | Decision | Rationale | Decided by |\n| --- | --- | --- | --- |\n| b | not_accepted | r | Pat |\n",
    );
    let snapshot = repo.snapshot(&with_risk(&repo));
    let items = findings_of(&snapshot, "benefit-risk-not-accepted");
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(items[0]["obligation"], "RR-2");
    assert_eq!(items[0]["summary"], "RR-2, BR-1");
    assert_risk_items_equal_summary(&repo, &snapshot);
}

#[trace("FR-166-AC-12", "FR-167-AC-18", "FR-167-AC-19")]
#[test]
fn risk_and_anomaly_gate_findings_join_the_auditor_findings_in_the_total_order() {
    let repo = Repo::bare();
    unverified_control(&repo);
    repo.anomaly("AN-001", "identified", "unassessed", &[], "");
    repo.anomaly("AN-002", "assessed", "impacting", &["severity: major"], "");
    repo.anomaly(
        "AN-003",
        "assessed",
        "impacting",
        &["severity: major"],
        "## Release Rationale\n\n| Author | Rationale |\n|--------|-----------|\n| Dana | Ships behind a flag. |\n",
    );
    repo.anomaly(
        "AN-004",
        "closed",
        "impacting",
        &["resolution: fixed", "closed_by: \"Pat\"", "severity: major"],
        "",
    );
    let snapshot = repo.snapshot(&with_risk(&repo));

    // Each gate item is a failure of `quoin anomaly report` for its mode.
    let gate = |kind: &str| -> Vec<(String, String)> {
        findings_of(&snapshot, kind)
            .iter()
            .map(|item| {
                assert_eq!(item["summary"], kind.trim_start_matches("anomaly-"));
                assert_eq!(item["line"], Value::Null);
                assert_eq!(item["symbol"], Value::Null);
                assert!(item.get("cause").is_none());
                (
                    item["obligation"].as_str().unwrap().to_owned(),
                    item["path"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    };
    assert_eq!(
        gate("anomaly-unassessed"),
        gate_failures(&repo, "unassessed")
    );
    assert_eq!(
        gate("anomaly-unjustified-impacting"),
        gate_failures(&repo, "unjustified-impacting")
    );
    assert_eq!(
        gate("anomaly-unassessed"),
        [("AN-001".to_owned(), "spec/anomalies/AN-001.md".to_owned())]
    );
    assert_eq!(
        gate("anomaly-unjustified-impacting"),
        [("AN-002".to_owned(), "spec/anomalies/AN-002.md".to_owned())]
    );

    assert_eq!(
        findings_of(&snapshot, "control-without-verification").len(),
        1
    );
    assert_eq!(findings_of(&snapshot, "situation-without-harm").len(), 2);
    assert_risk_items_equal_summary(&repo, &snapshot);

    // Auditor, risk and gate items sit in one total order, and every item has
    // exactly the closed keys and no severity, score or acceptance word.
    let items = snapshot["open_findings"].as_array().unwrap();
    let order: Vec<(&str, &str)> = items
        .iter()
        .map(|item| {
            (
                item["obligation"].as_str().unwrap(),
                item["kind"].as_str().unwrap(),
            )
        })
        .collect();
    let mut sorted = order.clone();
    sorted.sort_unstable();
    assert_eq!(order, sorted);
    assert!(order.contains(&("FR-001-AC-3", "undischarged")));
    for item in items {
        let mut keys: Vec<&str> = item
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["kind", "line", "obligation", "path", "summary", "symbol"],
            "{item}"
        );
        let lower = item.to_string().to_lowercase();
        for word in ["severity", "score", "accept", "rank", "verdict"] {
            assert!(!lower.contains(word), "{word} in {item}");
        }
    }
}

#[trace("FR-166-AC-13")]
#[test]
fn a_risk_summary_failure_refuses_naming_the_root_and_a_repository_without_risk_records_writes() {
    let repo = Repo::bare();
    let plain = repo.snapshot(&with_risk(&repo));
    assert!(
        plain["open_findings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["obligation"].as_str().unwrap().starts_with("FR-001"))
    );
    std::fs::remove_file(repo.aux("snapshot.json")).unwrap();

    let missing = repo.run().set("--module", "/nonexistent-module-root");
    let refused = repo.refusal(&missing);
    assert_eq!(refused["code"], "release_snapshot_input_invalid");
    assert_eq!(refused["context"]["input"], "--module");
    assert_eq!(refused["context"]["module"], "/nonexistent-module-root");

    estimate_chain_with_bad_residual(&repo);
    let refused = repo.refusal(&with_risk(&repo));
    assert_eq!(refused["code"], "release_snapshot_input_invalid");
    assert_eq!(refused["context"]["input"], "--repo");
    assert!(
        refused["message"].as_str().unwrap().contains("RR-1"),
        "{refused}"
    );
}

/// Run with two `--module` roots, the second of which does not load.
fn run_with_modules(repo: &Repo, modules: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
    command.args(["release", "snapshot"]);
    for (flag, value) in &repo.run().flags {
        command.arg(flag).arg(value);
    }
    for module in modules {
        command.args(["--module", module]);
    }
    command.env("IX_HOME", repo.home.path());
    command.output().unwrap()
}

#[trace("FR-166-AC-13")]
#[test]
fn the_refusal_names_the_module_that_failed_and_not_one_that_loaded() {
    let repo = Repo::bare();
    let broken = tempfile::tempdir().unwrap();
    std::fs::write(broken.path().join("manifest.yaml"), "name: [unclosed\n").unwrap();
    let safety = safety_module();
    let output = run_with_modules(&repo, &[&safety, broken.path().to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(2), "{}", text(&output.stderr));
    let refused: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(refused[0]["code"], "release_snapshot_input_invalid");
    assert_eq!(
        refused[0]["context"]["module"],
        broken.path().to_str().unwrap(),
        "{refused}"
    );

    // A set that declares no safety record type names every root given, and
    // with none given the installed modules, never `--repo`.
    unverified_control(&repo);
    std::fs::remove_file(repo.aux("snapshot.json")).ok();
    let own = repo.path().to_str().unwrap();
    let output = run_with_modules(&repo, &[own]);
    let refused: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(refused[0]["context"]["input"], "--module");
    assert_eq!(refused[0]["context"]["module"], own);
    let output = run_with_modules(&repo, &[]);
    let refused: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(refused[0]["context"]["input"], "--module");
    assert_eq!(refused[0]["context"]["module"], "installed modules");
}

fn estimate_chain_with_bad_residual(repo: &Repo) {
    risk_record(repo, "HAZ-1", "hazard", "", "");
    risk_record(
        repo,
        "SIT-1",
        "hazardous_situation",
        "relationships:\n  - target: HAZ-1\n    type: arises_from\n",
        "",
    );
    estimate(repo, "EST-1", "SIT-1", None);
    risk_record(
        repo,
        "RR-1",
        "residual_risk",
        "relationships:\n  - target: EST-1\n    type: estimate\n",
        "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| bogus-word | r | Pat |\n",
    );
}

#[trace("FR-166-AC-14")]
#[test]
fn only_an_unreadable_or_non_mapping_file_below_repo_refuses_and_a_typeless_one_does_not() {
    let repo = Repo::bare();
    for (path, text) in [
        (
            "docs/SKILL.md",
            "---\nname: tool\ndescription: a skill\n---\n# Skill\n",
        ),
        ("docs/empty.md", "---\n---\n# Empty block\n"),
        (
            "docs/rule.md",
            "---\nA page that opens with a thematic break.\n",
        ),
    ] {
        repo.write(path, text);
    }
    repo.snapshot(&with_risk(&repo));
    std::fs::remove_file(repo.aux("snapshot.json")).unwrap();

    for (path, bytes) in [
        (
            "docs/broken.md",
            b"---\nkey: [unclosed\n---\n# Broken\n".to_vec(),
        ),
        ("docs/list.md", b"---\n- a\n- b\n---\n# List\n".to_vec()),
        ("docs/binary.md", vec![0xff, 0xfe, 0x00, 0x80]),
        (
            "spec/risk/HAZ-9.md",
            b"---\nid: HAZ-9\ntype: hazard\nobject: [unclosed\n---\n# Hazard\n".to_vec(),
        ),
    ] {
        let full = repo.path().join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, bytes).unwrap();
        let refused = repo.refusal(&with_risk(&repo));
        assert_eq!(
            refused["code"], "release_snapshot_input_unreadable",
            "{path}"
        );
        assert_eq!(refused["context"]["input"], "--repo");
        assert_eq!(refused["context"]["path"], path);
        std::fs::remove_file(full).unwrap();
    }
}

#[trace("FR-166-AC-14")]
#[test]
fn the_unusable_file_is_named_relative_to_repo_whatever_the_repo_spelling() {
    let repo = Repo::bare();
    repo.write("docs/broken.md", "---\nkey: [unclosed\n---\n# Broken\n");
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
    command.args(["release", "snapshot"]);
    for (flag, value) in &repo.run().set("--repo", ".").flags {
        command.arg(flag).arg(value);
    }
    let output = command
        .args(["--module", &safety_module()])
        .current_dir(repo.path())
        .env("IX_HOME", repo.home.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{}", text(&output.stderr));
    let refused: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(refused[0]["context"]["path"], "docs/broken.md");
    let message = refused[0]["message"].as_str().unwrap();
    assert!(
        !message.contains(repo.path().to_str().unwrap()),
        "{message}"
    );
}

#[trace("FR-166-AC-14")]
#[test]
fn a_valid_typeless_skill_document_is_not_a_vulnerability_record() {
    let repo = Repo::bare();
    for path in ["docs/SKILL.md", "spec/vulnerabilities/SKILL.md"] {
        repo.write(
            path,
            "---\nname: quoin-test-skill\ndescription: A valid skill document.\n---\n# Skill\n",
        );
    }

    let snapshot = repo.snapshot(&repo.run());
    assert_eq!(snapshot["vulnerabilities"], json!([]));
}

#[trace("FR-166-AC-14")]
#[test]
fn a_vulnerability_without_an_id_is_not_silently_dropped_outside_its_usual_directory() {
    let repo = Repo::bare();
    repo.write(
        "docs/missing-id.md",
        "---\ntitle: MissingId\ntype: vulnerability\nobject: vulnerability\nseverity: high\n---\n# Missing ID\n",
    );
    let security = security_module();
    let safety = safety_module();

    let output = run_with_modules(&repo, &[&safety, &security]);
    assert_eq!(output.status.code(), Some(2), "{}", text(&output.stderr));
    let refused: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(refused[0]["code"], "release_snapshot_input_invalid");
    assert!(
        text(&output.stderr).contains("docs/missing-id.md"),
        "{}",
        text(&output.stderr)
    );
}

fn security_module() -> String {
    let configured = std::env::var_os("SPEC_OBJECTS_SECURITY_DIR").map(PathBuf::from);
    let module = configured
        .and_then(|path| {
            if path.join("manifest.yaml").is_file() {
                Some(path)
            } else {
                let nested = path.join("spec_objects_security");
                nested.join("manifest.yaml").is_file().then_some(nested)
            }
        })
        .or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .map(|ancestor| ancestor.join("spec-objects-security/spec_objects_security"))
                .find(|candidate| candidate.join("manifest.yaml").is_file())
        })
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../../spec-objects-security/spec_objects_security")
        });
    module.display().to_string()
}
