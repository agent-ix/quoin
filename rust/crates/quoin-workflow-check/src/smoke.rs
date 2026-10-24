// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Behavioral acceptance scenarios for source, native-archive and npm installs.

use crate::{Result, require};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const LENSES: [&str; 7] = [
    "failure-domain",
    "integrity",
    "dependency",
    "evidence",
    "risk-complexity",
    "scope-boundary",
    "ears-conformance",
];

struct Harness {
    binary: PathBuf,
    runner: PathBuf,
    runner_bin: PathBuf,
    root: PathBuf,
    home: PathBuf,
    config: PathBuf,
    scratch: tempfile::TempDir,
}

fn write(path: &Path, content: impl AsRef<[u8]>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, content).map_err(|e| e.to_string())
}
fn text(value: &Value, pointer: &str) -> Result<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("missing {pointer}: {value}"))
}
fn passed(output: &Output) -> Result<Value> {
    let value: Value = serde_json::from_slice(&output.stdout).map_err(|e| {
        format!(
            "invalid JSON: {e}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })?;
    require(
        output.status.success() && value.get("ok").and_then(Value::as_bool) == Some(true),
        format!("{}\n{}", value, String::from_utf8_lossy(&output.stderr)),
    )?;
    Ok(value)
}
fn refused(output: &Output, code: &str) -> Result<Value> {
    let value: Value = serde_json::from_slice(&output.stdout).map_err(|e| {
        format!(
            "expected refusal JSON: {e}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })?;
    require(
        value.get("state").and_then(Value::as_str) != Some("ok")
            && value.to_string().contains(code),
        format!("expected {code}: {value}"),
    )?;
    Ok(value)
}
fn selection(set: &str, lenses: &[&str]) -> Value {
    json!({"review_set":set,"selected_analyses":lenses,"assurance_profiles":[],"profile_disposition":"No applicable profile in this synthetic repository."})
}
fn review(analysis: &str, severity: &str) -> String {
    format!(
        "---\nid: SR-001\ntitle: Synthetic review\ntype: SpecReview\nanalysis: {analysis}\n---\n# Review\n\n## Summary\nSynthetic outcome.\n\n## Findings\n\n| ID | Severity | Summary | Refs |\n| --- | --- | --- | --- |\n| FND-001 | {severity} | Synthetic finding | FR-001 |\n"
    )
}
fn simple(kind: &str, id: &str) -> String {
    format!("---\nid: {id}\ntitle: Synthetic {kind}\ntype: {kind}\n---\n# Synthetic {kind}\n")
}
fn schema(kind: &str) -> Value {
    let mut properties = json!({"id":{"type":"string","minLength":1},"title":{"type":"string","minLength":1},"type":{"const":kind}});
    let mut required = vec!["id", "title", "type"];
    if let Some(map) = properties.as_object_mut() {
        if kind == "SpecReview" {
            map.insert(
                "analysis".to_owned(),
                json!({"enum":std::iter::once("base").chain(LENSES).collect::<Vec<_>>()}),
            );
            required.push("analysis");
        }
        if kind == "AssuranceProfile" {
            map.insert(
                "status".to_owned(),
                json!({"enum":["active","proposed","retired"]}),
            );
            map.insert("review_policy".to_owned(), json!({"type":"object","additionalProperties":false,"required":["mode","operations"],"properties":{"mode":{"enum":["recommend","require"]},"operations":{"type":"array","items":{"enum":["spec-review","code-review","gap-analysis"]}}}}));
        }
    }
    json!({"type":"object","additionalProperties":false,"required":required,"properties":properties})
}

impl Harness {
    fn new(binary: &str, runner: &str) -> Result<Self> {
        let scratch = tempfile::tempdir().map_err(|e| e.to_string())?;
        let root = scratch.path().join("unrelated repo with spaces");
        let home = scratch.path().join("fresh home");
        let config = home.join(".ix");
        let binary = fs::canonicalize(binary).map_err(|e| e.to_string())?;
        // npm's launcher remains next to its platform dependency; native artifacts
        // are physically relocated to prove source-checkout independence.
        let binary = if binary.components().any(|c| c.as_os_str() == "node_modules") {
            binary
        } else {
            let relocated = scratch.path().join(if cfg!(windows) {
                "relocated quoin.exe"
            } else {
                "relocated quoin"
            });
            fs::copy(&binary, &relocated).map_err(|e| e.to_string())?;
            relocated
        };
        let runner = fs::canonicalize(runner).map_err(|e| e.to_string())?;
        let runner_bin = runner
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .and_then(Path::parent)
            .ok_or("ix-flow package location invalid")?
            .join(".bin");
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let harness = Self {
            binary,
            runner,
            runner_bin,
            root,
            home,
            config,
            scratch,
        };
        harness.fixture()?;
        Ok(harness)
    }
    fn module(&self) -> PathBuf {
        self.config
            .join("filament/modules/synthetic-workflow-contract")
    }
    fn fixture(&self) -> Result<()> {
        let names = ["SpecReview", "Plan", "Task", "index", "AssuranceProfile"];
        let types = names.iter().map(|name| {
            let mut item = json!({"name":name,"frontmatter_schema_ref":format!("schemas/{name}.json")});
            if *name == "SpecReview" && let Some(map) = item.as_object_mut() { map.insert("body_extraction".to_owned(), json!({"yield_pattern":{"match":{"summary":{"from":"section_body","after_heading":"Summary","required":true},"findings":{"from":"table_row","under_section":"Findings","multiple":true,"required":true,"assert":{"columns":["ID","Severity","Summary","Refs"],"min_rows":1,"id_column":"ID","id_pattern":"^FND-\\d+$","column_choices":{"Severity":["low","medium","high"]}}}}}})); }

            item
        }).collect::<Vec<_>>();
        write(&self.module().join("manifest.yaml"), json!({"manifest_version":"1.0.0","name":"synthetic-workflow-contract","version":"1.0.0","archetypes":names.iter().map(|name|json!({"kind":name,"name":name,"doc_backed":true})).collect::<Vec<_>>(),"artifact_types":types}).to_string())?;
        for name in names {
            write(
                &self.module().join(format!("schemas/{name}.json")),
                schema(name).to_string(),
            )?;
        }
        write(&self.root.join("spec/target.md"), simple("FR", "FR-001"))
    }
    fn command(&self, executable: &Path) -> Result<Command> {
        let mut command = Command::new(executable);
        let mut paths = vec![self.runner_bin.clone()];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        command
            .current_dir(&self.root)
            .env(
                "PATH",
                std::env::join_paths(paths).map_err(|e| e.to_string())?,
            )
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("IX_HOME", &self.config)
            .env(
                "IX_SPEC_WORKFLOWS_ROOT",
                self.scratch.path().join("absent-private-repo"),
            );
        for key in [
            "GH_TOKEN",
            "GITHUB_TOKEN",
            "REGISTRY_TOKEN",
            "NODE_AUTH_TOKEN",
            "NPM_TOKEN",
            "NPM_CONFIG_USERCONFIG",
            "QUOIN_SEMANTIC_ROOT",
            "QUOIN_MODULE_PATHS",
            "IX_FILAMENT_MODULES_PATH",
        ] {
            command.env_remove(key);
        }
        for key in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY"] {
            command.env(key, "http://127.0.0.1:9");
        }
        Ok(command)
    }
    fn flow(&self, args: &[&str]) -> Result<Output> {
        self.command(Path::new(if cfg!(windows) { "node.exe" } else { "node" }))?
            .arg(&self.runner)
            .args(args)
            .arg("--config-root")
            .arg(&self.config)
            .arg("--state-dir")
            .arg(self.config.join("flows"))
            .arg("--json")
            .output()
            .map_err(|e| e.to_string())
    }
    fn start(&self, kind: &str, id: &str) -> Result<Value> {
        passed(
            &self
                .command(&self.binary)?
                .arg("--config-root")
                .arg(&self.config)
                .args([kind, "--target", "spec/target.md", "--id", id, "--json"])
                .output()
                .map_err(|e| e.to_string())?,
        )
    }
    fn answers(&self, id: &str, interview: &str, answers: &Value) -> Result<Value> {
        passed(&self.flow(&[
            "record-answers",
            id,
            interview,
            "--answers",
            &answers.to_string(),
        ])?)
    }
    fn advance(&self, id: &str, phase: &str) -> Result<Value> {
        passed(&self.flow(&["advance", id, phase])?)
    }
    fn add(&self, id: &str, analysis: &str, path: &str) -> Result<Value> {
        passed(&self.flow(&[
            "add-item",
            id,
            "review_doc",
            "--item",
            &json!({"id":format!("{analysis}-record"),"analysis":analysis,"path":path}).to_string(),
        ])?)
    }
}

/// Trace: FR-020-AC-1, FR-020-AC-5, FR-021-AC-6, FR-021-AC-7, FR-095-AC-6, FR-095-AC-8, FR-095-AC-9
pub(super) fn run(binary: &str, runner: &str) -> Result<()> {
    let h = Harness::new(binary, runner)?;
    // Trace: FR-020-AC-1, FR-020-AC-5, FR-021-AC-6, FR-021-AC-7, FR-095-AC-6, FR-095-AC-8, FR-095-AC-9
    let started = h.start("review", "smoke-base")?;
    require(
        text(&started, "/current_phase")? == "intake",
        "run must start at intake",
    )?;
    passed(&h.flow(&["resume", "smoke-base"])?)?;
    h.answers("smoke-base", "request", &selection("base", &[]))?;
    h.advance("smoke-base", "analyses_run")?;
    h.advance("smoke-base", "reviews_rendered")?;
    refused(
        &h.flow(&["advance", "smoke-base", "validated"])?,
        "WORKFLOW_ANALYSES_MISSING",
    )?;
    h.add("smoke-base", "base", "spec/reviews/base.md")?;
    refused(
        &h.flow(&["advance", "smoke-base", "validated"])?,
        "WORKFLOW_DOCUMENT_UNAVAILABLE",
    )?;
    write(
        &h.root.join("spec/reviews/base.md"),
        review("integrity", "low"),
    )?;
    refused(
        &h.flow(&["advance", "smoke-base", "validated"])?,
        "WORKFLOW_REVIEW_MISMATCH",
    )?;
    write(
        &h.root.join("spec/reviews/base.md"),
        review("base", "pretend"),
    )?;
    refused(
        &h.flow(&["advance", "smoke-base", "validated"])?,
        "WORKFLOW_DOCUMENT_INVALID",
    )?;
    write(&h.root.join("spec/reviews/base.md"), review("base", "low"))?;
    h.advance("smoke-base", "validated")?;
    write(
        &h.root.join("spec/reviews/base.md"),
        review("integrity", "low"),
    )?;
    refused(
        &h.flow(&["advance", "smoke-base", "accepted"])?,
        "WORKFLOW_REVIEW_MISMATCH",
    )?;
    write(&h.root.join("spec/reviews/base.md"), review("base", "low"))?;
    let deferred: Value =
        serde_json::from_slice(&h.flow(&["advance", "smoke-base", "accepted"])?.stdout)
            .map_err(|e| e.to_string())?;
    require(
        text(&deferred, "/state")? == "gate_deferred",
        format!("expected human gate: {deferred}"),
    )?;
    let token = text(&deferred, "/open_gates/0/token")?;
    passed(&h.flow(&[
        "ack",
        "smoke-base",
        &token,
        "--reviewer",
        "synthetic-test-human",
    ])?)?;
    h.advance("smoke-base", "accepted")?;
    require(
        text(
            &passed(&h.flow(&["status", "smoke-base"])?)?,
            "/current_phase",
        )? == "accepted",
        "acceptance must persist",
    )?;
    review_scenarios(&h, &started)?;
    plan_scenario(&h)?;
    runner_failures(&h)?;
    println!(
        "owned workflow smoke passed: relocated binary, live catalog, published ix-flow, review/profile/plan validation and explicit human acceptance"
    );
    Ok(())
}

/// Trace: FR-095-AC-3, FR-095-AC-4, FR-095-AC-5, FR-095-AC-7
fn review_scenarios(h: &Harness, started: &Value) -> Result<()> {
    // Trace: FR-095-AC-3, FR-095-AC-4, FR-095-AC-5, FR-095-AC-7
    h.start("review", "smoke-all")?;
    h.answers("smoke-all", "request", &selection("all", &["integrity"]))?;
    refused(
        &h.flow(&["advance", "smoke-all", "analyses_run"])?,
        "WORKFLOW_SELECTION_INVALID",
    )?;
    h.answers("smoke-all", "request", &selection("all", &[]))?;
    h.advance("smoke-all", "analyses_run")?;
    h.advance("smoke-all", "reviews_rendered")?;
    h.add("smoke-all", "base", "spec/reviews/base.md")?;
    refused(
        &h.flow(&["advance", "smoke-all", "validated"])?,
        "WORKFLOW_ANALYSES_MISSING",
    )?;
    for lens in LENSES {
        let file = format!("spec/reviews/{lens}.md");
        write(&h.root.join(&file), review(lens, "low"))?;
        h.add("smoke-all", lens, &file)?;
    }
    h.advance("smoke-all", "validated")?;
    h.start("review", "smoke-profile")?;
    write(
        &h.root.join("spec/profile.md"),
        "---\nid: AP-001\ntitle: Synthetic profile\ntype: AssuranceProfile\nstatus: active\nreview_policy:\n  mode: require\n  operations: [spec-review]\n---\n# Profile\n",
    )?;
    let mut answers = selection("base", &[]);
    if let Some(map) = answers.as_object_mut() {
        map.insert("assurance_profiles".to_owned(), json!(["spec/profile.md"]));
    }
    h.answers("smoke-profile", "request", &answers)?;
    h.advance("smoke-profile", "analyses_run")?;
    let runtime = text(started, "/data/skillPath/absolute")?;
    passed(&h.flow(&[
        "run",
        "review",
        "--path",
        &runtime,
        "--id",
        "smoke-automatic",
        "--gate",
        "validated->accepted=auto",
    ])?)?;
    let context = started
        .pointer("/data/items/quoin_context/0")
        .ok_or("launch context missing")?;
    passed(&h.flow(&[
        "add-item",
        "smoke-automatic",
        "quoin_context",
        "--item",
        &context.to_string(),
    ])?)?;
    h.answers("smoke-automatic", "request", &selection("base", &[]))?;
    refused(
        &h.flow(&["advance", "smoke-automatic", "analyses_run"])?,
        "WORKFLOW_HUMAN_GATE_REQUIRED",
    )?;
    h.start("review", "smoke-unsupported")?;
    h.answers("smoke-unsupported", "request", &selection("all", &[]))?;
    let review_schema = h.module().join("schemas/SpecReview.json");
    let mut unsupported = schema("SpecReview");
    if let Some(map) = unsupported
        .pointer_mut("/properties/analysis")
        .and_then(Value::as_object_mut)
    {
        map.insert("enum".to_owned(), json!(["base"]));
    }
    write(&review_schema, unsupported.to_string())?;
    refused(
        &h.flow(&["advance", "smoke-unsupported", "analyses_run"])?,
        "WORKFLOW_ANALYSIS_UNSUPPORTED",
    )?;
    write(&review_schema, schema("SpecReview").to_string())?;
    h.start("review", "smoke-catalog")?;
    h.answers("smoke-catalog", "request", &selection("base", &[]))?;
    let moved = h.scratch.path().join("missing-module");
    fs::rename(h.module(), &moved).map_err(|e| e.to_string())?;
    let result = refused(
        &h.flow(&["advance", "smoke-catalog", "analyses_run"])?,
        "WORKFLOW_CATALOG_UNAVAILABLE",
    );
    fs::rename(&moved, h.module()).map_err(|e| e.to_string())?;
    result?;
    Ok(())
}

/// Trace: FR-021-AC-6, FR-021-AC-7
fn plan_scenario(h: &Harness) -> Result<()> {
    // Trace: FR-021-AC-6, FR-021-AC-7
    h.start("to-plan", "smoke-plan")?;
    let bundle = "plan/Plan-001-smoke";
    let plan = format!("{bundle}/plan.md");
    let index = format!("{bundle}/index.md");
    let task = format!("{bundle}/tasks/Task-001.md");
    write(&h.root.join(&plan), simple("Plan", "Plan-001"))?;
    write(&h.root.join(&index), simple("index", "index-plan"))?;
    h.answers("smoke-plan","plan",&json!({"plan_path":plan,"index_path":index,"task_paths":[task],"readiness_notes":"Synthetic dependencies and verification resolved."}))?;
    h.advance("smoke-plan", "evidence_recorded")?;
    h.advance("smoke-plan", "plan_drafted")?;
    refused(
        &h.flow(&["advance", "smoke-plan", "validated"])?,
        "WORKFLOW_DOCUMENT_UNAVAILABLE",
    )?;
    write(&h.root.join(&task), simple("Task", "Task-001"))?;
    h.advance("smoke-plan", "validated")?;
    let result: Value =
        serde_json::from_slice(&h.flow(&["advance", "smoke-plan", "accepted"])?.stdout)
            .map_err(|e| e.to_string())?;
    require(
        text(&result, "/state")? == "gate_deferred",
        "plan requires human acceptance",
    )
}

/// Trace: FR-020-AC-3, FR-021-AC-2
fn runner_failures(h: &Harness) -> Result<()> {
    let fake_dir = h.scratch.path().join("fake runner");
    let program = fake_dir.join(if cfg!(windows) {
        "ix-flow.cmd"
    } else {
        "ix-flow"
    });
    let mut paths = vec![fake_dir, h.runner_bin.clone()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let search = std::env::join_paths(paths).map_err(|e| e.to_string())?;
    for version in ["0.2.3", "0.3.0"] {
        let body = if cfg!(windows) {
            format!(
                "@echo off\r\nif \"%~1\"==\"--version\" (echo {version}& exit /b 0)\r\necho synthetic runner failure 1>&2\r\nexit /b 7\r\n"
            )
        } else {
            format!(
                "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '{version}'; exit 0; fi\necho 'synthetic runner failure' >&2\nexit 7\n"
            )
        };
        write(&program, body)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
        }
        let output = h
            .command(&h.binary)?
            .env("PATH", &search)
            .arg("--config-root")
            .arg(&h.config)
            .args(["review", "--id", "must-not-start", "--json"])
            .output()
            .map_err(|e| e.to_string())?;
        if version == "0.2.3" {
            require(
                !output.status.success()
                    && String::from_utf8_lossy(&output.stderr).contains("ix-flow >=0.3.0"),
                "old runner must be refused before starting a run",
            )?;
        } else {
            require(
                output.status.code() == Some(7)
                    && String::from_utf8_lossy(&output.stderr).contains("synthetic runner failure"),
                "child exit 7 must be propagated",
            )?;
        }
    }
    Ok(())
}
