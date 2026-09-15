// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Native command-path coverage for the retained FR-062 graph fixtures.
//!
//! The graph engine has its own exhaustive oracle and model tests.  These
//! tests instead prove that the native command grammar reaches the real
//! filesystem reader, renders the chosen representation on stdout, and keeps
//! a core refusal on stderr with its nonzero exit code.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test assertions deliberately panic to report a failed command contract"
)]

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const REVISION: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

struct Fixture {
    root: PathBuf,
    export: PathBuf,
    premises: PathBuf,
    audit: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    fn new() -> std::io::Result<Self> {
        let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("quoin-cli-graph-{}-{id}", std::process::id()));
        fs::create_dir_all(root.join("inputs"))?;
        let export = root.join("inputs/assurance.json");
        let premises = root.join("inputs/premises.json");
        let audit = root.join("inputs/audit.json");
        let source = serde_json::json!({
            "repository": "agent-ix/example",
            "revision": REVISION,
        });
        let modules = serde_json::json!([
            {
                "name": "example",
                "version": "1.0.0",
                "schemas": [{ "archetype": "FR", "schema_digest": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc" }],
            }
        ]);
        let premises_value = serde_json::json!({
            "format": "quire-assurance",
            "format_version": 1,
            "modules": modules,
        });
        let relation_kinds = [
            "depends_on",
            "derives_from",
            "implements",
            "mitigates",
            "refines",
            "requires",
            "satisfies",
            "traces_to",
        ]
        .into_iter()
        .map(|kind| {
            serde_json::json!({
                "kind": kind,
                "availability": "available",
                "sources": ["module_vocabulary"],
            })
        })
        .collect::<Vec<_>>();
        let export_value = serde_json::json!({
            "format": "quire-assurance",
            "format_version": 1,
            "modules": modules,
            "source": source,
            "artifacts": [{
                "id": "FR-001",
                "artifact_type": "FR",
                "locator": { "path": "spec/FR-001.md", "line": 1, "digest": DIGEST },
            }],
            "obligations": [{
                "source": "acceptance-criterion",
                "id": "FR-001-AC-1",
                "document": "spec/FR-001.md",
                "statement": "The command reports fan-out.",
                "statement_hash": DIGEST,
                "target_ids": ["TC-1249"],
                "locator": { "path": "spec/FR-001.md", "line": 20, "digest": DIGEST },
            }],
            "symbols": [],
            "relation_kinds": relation_kinds,
            "relations": [],
            "relation_observations": [],
        });
        let audit_value = serde_json::json!({
            "format": "quoin-audit-envelope",
            "format_version": 1,
            "source": source,
            "export": premises_value,
            "report": { "findings": [], "healthy": ["FR-001-AC-1"], "unevaluated": [] },
        });
        write_json(&export, &export_value)?;
        write_json(&premises, &premises_value)?;
        write_json(&audit, &audit_value)?;
        write_json(
            &root.join("spec/evidence/bindings.json"),
            &serde_json::json!({
                "schemaVersion": 1,
                "bindings": [{
                    "obligation": "FR-001-AC-1",
                    "statementHashAtBinding": DIGEST,
                    "suite": "unit",
                    "commit": REVISION,
                    "symbols": ["fan-out test"],
                }],
            }),
        )?;
        Ok(Self {
            root,
            export,
            premises,
            audit,
        })
    }

    fn graph(&self, view: &str, json: bool, extra: &[&str]) -> Output {
        let mut command = ProcessCommand::new(env!("CARGO_BIN_EXE_quoin"));
        command
            .arg("graph")
            .arg(view)
            .arg("--repo")
            .arg(&self.root)
            .arg("--export")
            .arg(&self.export)
            .arg("--premises")
            .arg(&self.premises)
            .arg("--audit")
            .arg(&self.audit);
        for argument in extra {
            command.arg(argument);
        }
        if json {
            command.arg("--json");
        }
        command.output().expect("the native quoin binary runs")
    }
}

fn write_json(path: &Path, value: &serde_json::Value) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "fixture path has no parent",
        )
    })?;
    fs::create_dir_all(parent)?;
    fs::write(
        path,
        serde_json::to_string(value).map_err(std::io::Error::other)?,
    )
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("the CLI writes UTF-8 stdout")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("the CLI writes UTF-8 stderr")
}

fn json_stdout(output: &Output) -> serde_json::Value {
    serde_json::from_str(&stdout(output)).expect("the JSON view writes one JSON report")
}

/// Trace: FR-062, FR-062-AC-1, TC-1650
#[test]
fn tc_1650_fan_out_reaches_real_fixture_inputs_in_json_and_human_forms() {
    let fixture = Fixture::new().expect("fixture files");

    let json = fixture.graph("fan-out", true, &[]);
    assert!(json.status.success(), "stderr: {}", stderr(&json));
    let report = json_stdout(&json);
    assert_eq!(report["view"], "fan-out");
    assert_eq!(report["state"], "complete");
    assert_eq!(report["rows"][0]["suite"], "unit");
    assert_eq!(report["rows"][0]["obligationCount"], 1);
    assert!(stderr(&json).is_empty());

    let human = fixture.graph("fan-out", false, &[]);
    assert!(human.status.success(), "stderr: {}", stderr(&human));
    assert!(stdout(&human).contains("| Suite | Live obligations | Count | Unresolved |"));
    assert!(stderr(&human).is_empty());
}

/// Trace: FR-062, FR-062-AC-1, TC-1650
#[test]
fn tc_1650_churn_reaches_real_fixture_inputs_in_json_and_human_forms() {
    let fixture = Fixture::new().expect("fixture files");

    let json = fixture.graph("churn", true, &[]);
    assert!(json.status.success(), "stderr: {}", stderr(&json));
    let report = json_stdout(&json);
    assert_eq!(report["view"], "churn");
    assert_eq!(report["state"], "complete");
    assert_eq!(report["rows"][0]["obligation"], "FR-001-AC-1");
    assert_eq!(report["rows"][0]["eventCount"], 0);
    assert!(stderr(&json).is_empty());

    let human = fixture.graph("churn", false, &[]);
    assert!(human.status.success(), "stderr: {}", stderr(&human));
    assert!(stdout(&human).contains("## Retained reaffirmation history"));
    assert!(stderr(&human).is_empty());
}

/// Trace: FR-062, FR-062-AC-1, TC-1650
#[test]
fn tc_1650_change_impact_reaches_real_fixture_inputs_in_json_and_human_forms() {
    let fixture = Fixture::new().expect("fixture files");

    let json = fixture.graph("change-impact", true, &["--requirement", "FR-001"]);
    assert!(json.status.success(), "stderr: {}", stderr(&json));
    let report = json_stdout(&json);
    assert_eq!(report["view"], "change-impact");
    assert_eq!(report["state"], "complete");
    assert_eq!(report["requested"], serde_json::json!(["FR-001"]));
    assert_eq!(report["rows"][0]["requirement"], "FR-001");
    assert_eq!(report["rows"][0]["depth"], 0);
    assert!(stderr(&json).is_empty());

    let human = fixture.graph("change-impact", false, &["--requirement", "FR-001"]);
    assert!(human.status.success(), "stderr: {}", stderr(&human));
    assert!(stdout(&human).contains("## Change impact"));
    assert!(stderr(&human).is_empty());
}

/// Trace: FR-062, FR-102, TC-1650
#[test]
fn tc_1650_graph_refusal_keeps_stdout_empty_and_diagnostics_on_stderr() {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_quoin"))
        .args([
            OsStr::new("graph"),
            OsStr::new("fan-out"),
            OsStr::new("--export"),
            OsStr::new("missing-export.json"),
            OsStr::new("--premises"),
            OsStr::new("missing-premises.json"),
            OsStr::new("--audit"),
            OsStr::new("missing-audit.json"),
        ])
        .output()
        .expect("the native quoin binary runs");
    assert_eq!(output.status.code(), Some(2));
    assert!(stdout(&output).is_empty());
    let diagnostics: serde_json::Value =
        serde_json::from_str(&stderr(&output)).expect("refusal diagnostics are canonical JSON");
    assert_eq!(diagnostics[0]["code"], "CORE_REFUSED");
}
