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
    plugin: PathBuf,
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
fn executable(directory: &Path, name: &str, body: &str, windows_body: &str) -> Result<PathBuf> {
    let path = directory.join(if cfg!(windows) {
        format!("{name}.cmd")
    } else {
        name.to_owned()
    });
    let content = if cfg!(windows) {
        format!("@echo off\r\n{windows_body}\r\n")
    } else {
        format!("#!/bin/sh\n{body}\n")
    };
    write(&path, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&path)
            .map_err(|e| e.to_string())?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).map_err(|e| e.to_string())?;
    }
    Ok(path)
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
    review_with_revision(analysis, severity, "v1")
}
fn review_with_revision(analysis: &str, severity: &str, revision: &str) -> String {
    format!(
        "---\nid: SR-001\ntitle: Synthetic review\ntype: SpecReview\nanalysis: {analysis}\npackRevision: {revision}\n---\n# Review\n\n## Summary\nSynthetic outcome.\n\n## Findings\n\n| ID | Severity | Summary | Refs |\n| --- | --- | --- | --- |\n| FND-001 | {severity} | Synthetic finding | FR-001 |\n"
    )
}
fn profile(revision: &str, mode: &str, operations: &[&str]) -> String {
    format!(
        "---\nid: AP-001\ntitle: Synthetic profile\ntype: AssuranceProfile\nstatus: active\nprofileRevision: {revision}\nreview_policy:\n  mode: {mode}\n  operations: [{}]\n---\n# Profile\n",
        operations.join(", ")
    )
}
fn simple(kind: &str, id: &str) -> String {
    format!("---\nid: {id}\ntitle: Synthetic {kind}\ntype: {kind}\n---\n# Synthetic {kind}\n")
}
fn schema(kind: &str) -> Value {
    let mut properties = json!({"id":{"type":"string","minLength":1},"title":{"type":"string","minLength":1},"type":{"const":kind}});
    let mut required = vec!["id", "title", "type"];
    if let Some(map) = properties.as_object_mut() {
        map.insert("packRevision".to_owned(), json!({"const":"v1"}));
        if kind == "SpecReview" {
            map.insert(
                "analysis".to_owned(),
                json!({"enum":std::iter::once("base").chain(LENSES).collect::<Vec<_>>()}),
            );
            required.push("analysis");
        }
        if kind == "AssuranceProfile" {
            map.insert("profileRevision".to_owned(), json!({"const":"v1"}));
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
    fn new(binary: &str, runner: &str, plugin: &str, crlf: bool) -> Result<Self> {
        Self::new_with_plugin_version(binary, runner, plugin, crlf, false)
    }
    #[cfg(all(test, feature = "workflow-test"))]
    fn new_for_workflow(binary: &str, runner: &str, plugin: &str, crlf: bool) -> Result<Self> {
        Self::new_with_plugin_version(binary, runner, plugin, crlf, true)
    }
    fn new_with_plugin_version(
        binary: &str,
        runner: &str,
        plugin: &str,
        crlf: bool,
        normalize_copied_manifest: bool,
    ) -> Result<Self> {
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
            let relocated = scratch
                .path()
                .join("native bin with spaces")
                .join(if cfg!(windows) { "quoin.exe" } else { "quoin" });
            fs::create_dir_all(relocated.parent().ok_or("native bin has no parent")?)
                .map_err(|e| e.to_string())?;
            fs::copy(&binary, &relocated).map_err(|e| e.to_string())?;
            relocated
        };
        let installed_plugin = scratch.path().join("installed plugins/quoin");
        let plugin = fs::canonicalize(plugin).map_err(|e| e.to_string())?;
        copy_tree(&plugin.join("skills"), &installed_plugin.join("skills"))?;
        for skill in ["spec-review", "spec-to-plan"] {
            let path = installed_plugin.join("skills").join(skill).join("SKILL.md");
            let text = fs::read_to_string(&path)
                .map_err(|e| e.to_string())?
                .replace("\r\n", "\n");
            write(
                &path,
                if crlf {
                    text.replace('\n', "\r\n")
                } else {
                    text
                },
            )?;
        }
        for directory in [".codex-plugin", ".claude-plugin"] {
            copy_tree(&plugin.join(directory), &installed_plugin.join(directory))?;
            let manifest_path = installed_plugin.join(directory).join("plugin.json");
            let mut manifest: Value =
                serde_json::from_slice(&fs::read(&manifest_path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            // The FR-189 workflow behavior tests use current skills with the
            // Rust binary built from this workspace. A recent plugin-only
            // release advanced the source manifests independently, so normalize
            // only the temporary copies and preserve the source-version check.
            if normalize_copied_manifest {
                manifest["version"] = json!(env!("CARGO_PKG_VERSION"));
                write(
                    &manifest_path,
                    serde_json::to_vec(&manifest).map_err(|e| e.to_string())?,
                )?;
            }
            require(
                manifest.get("version").and_then(Value::as_str) == Some(env!("CARGO_PKG_VERSION")),
                "plugin manifest must match the release checker version",
            )?;
        }
        let runner_bin = Path::new(runner)
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .and_then(Path::parent)
            .ok_or("ix-flow package location invalid")?
            .join(".bin");
        let runner = fs::canonicalize(runner).map_err(|e| e.to_string())?;
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
        let harness = Self {
            binary,
            runner,
            runner_bin,
            plugin: installed_plugin,
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
        let mut paths = vec![
            self.binary
                .parent()
                .ok_or("binary has no parent")?
                .to_path_buf(),
            self.runner_bin.clone(),
        ];
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
        self.flow_with_tools(args, None)
    }
    fn flow_with_tools(&self, args: &[&str], tools: Option<(&Path, &Path)>) -> Result<Output> {
        self.flow_with_analysis_tools(args, tools, None)
    }
    fn flow_with_analysis_tools(
        &self,
        args: &[&str],
        tools: Option<(&Path, &Path)>,
        analysis_tools: Option<(&Path, &Path, &Path)>,
    ) -> Result<Output> {
        let mut command =
            self.command(Path::new(if cfg!(windows) { "node.exe" } else { "node" }))?;
        command
            .arg(&self.runner)
            .args(args)
            .arg("--config-root")
            .arg(&self.config)
            .arg("--state-dir")
            .arg(self.config.join("flows"))
            .arg("--json");
        if let Some((fsl, quire)) = tools {
            command
                .env("FSL_DRIVER_BIN", fsl)
                .env("QUIRE_DRIVER_BIN", quire);
        }
        if let Some((analyzer, z3, cvc5)) = analysis_tools {
            command
                .env("FILAMENT_ANALYZE_BIN", analyzer)
                .env("Z3_BIN", z3)
                .env("CVC5_BIN", cvc5);
        }
        command.output().map_err(|e| e.to_string())
    }
    fn skill(&self, kind: &str) -> PathBuf {
        self.plugin.join("skills").join(if kind == "review" {
            "spec-review"
        } else if kind == "formalize" {
            "formalize-requirement"
        } else if kind == "design-gate" {
            "design-gate"
        } else {
            "spec-to-plan"
        })
    }
    fn start(&self, kind: &str, id: &str) -> Result<Value> {
        let skill = self.skill(kind);
        let skill = skill.to_str().ok_or("skill path is not UTF-8")?;
        passed(&self.flow(&[
            "run",
            kind,
            "--path",
            skill,
            "--id",
            id,
            "--target",
            "spec/target.md",
        ])?)?;
        passed(&self.flow(&["add-item", id, "quoin_context", "--item", &json!({"id":"quoin-launch-context","repo_root":self.root,"config_root":self.config,"targets":[self.root.join("spec/target.md")]}).to_string()])?)
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

pub(super) fn run(binary: &str, runner: &str, plugin: &str) -> Result<()> {
    for crlf in [false, true] {
        lifecycle(&Harness::new(binary, runner, plugin, crlf)?)?;
        println!(
            "plugin workflow line endings passed: {}",
            if crlf { "CRLF" } else { "LF" }
        );
    }
    Ok(())
}

fn lifecycle(h: &Harness) -> Result<()> {
    let started = review_base_lifecycle(h)?;
    review_scenarios(h, &started)?;
    plan_scenario(h)?;
    formalize_scenarios(h)?;
    runner_failures(h)?;
    println!(
        "owned workflow smoke passed: installed plugin skills, relocated binary, live catalog, host-capable ix-flow package, review/profile/plan validation and explicit human acceptance"
    );
    Ok(())
}

fn review_base_lifecycle(h: &Harness) -> Result<Value> {
    let started = h.start("review", "smoke-base")?;
    require(
        text(&started, "/current_phase")? == "intake",
        "run must start at intake",
    )?;
    passed(&h.flow(&["resume", "smoke-base"])?)?;
    let skill_path = text(&started, "/data/skillPath/absolute")?;
    require(
        Path::new(&skill_path) == h.skill("review"),
        "run must use the installed plugin skill directly",
    )?;
    require(
        !h.config.join("workflows").exists(),
        "launch must not extract embedded workflows",
    )?;
    let convenience = h
        .command(&h.binary)?
        .arg("--config-root")
        .arg(&h.config)
        .args(["review", "--path"])
        .arg(h.skill("review"))
        .args(["--id", "smoke-convenience", "--json"])
        .output()
        .map_err(|e| e.to_string())?;
    passed(&convenience)?;
    h.answers("smoke-base", "request", &selection("base", &[]))?;
    let recorded = passed(&h.flow(&["status", "smoke-base"])?)?;
    let request = recorded
        .pointer("/data/items/operation_request/0")
        .ok_or("review answers were not recorded in the run")?;
    require(
        request.get("assurance_profiles") == Some(&json!([]))
            && request.get("profile_disposition").and_then(Value::as_str)
                == Some("No applicable profile in this synthetic repository."),
        format!("the run records the explicit no-profile disposition: {request}"),
    )?;
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
    fs::create_dir_all(h.root.join("spec/reviews/base.md")).map_err(|error| error.to_string())?;
    refused(
        &h.flow(&["advance", "smoke-base", "validated"])?,
        "WORKFLOW_DOCUMENT_UNAVAILABLE",
    )?;
    fs::remove_dir_all(h.root.join("spec/reviews/base.md")).map_err(|error| error.to_string())?;
    write(
        &h.root.join("spec/reviews/base.md"),
        "---\nid: SR-001\ntitle: Synthetic review\ntype: SpecReview\nanalysis: base\n",
    )?;
    refused(
        &h.flow(&["advance", "smoke-base", "validated"])?,
        "WORKFLOW_DOCUMENT_INVALID",
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
    let review_schema = h.module().join("schemas/SpecReview.json");
    let mut live_review_schema = schema("SpecReview");
    live_review_schema["properties"]["packRevision"]["const"] = json!("v2");
    write(&review_schema, live_review_schema.to_string())?;
    write(&h.root.join("spec/reviews/base.md"), review("base", "low"))?;
    refused(
        &h.flow(&["advance", "smoke-base", "validated"])?,
        "WORKFLOW_DOCUMENT_INVALID",
    )?;
    write(
        &h.root.join("spec/reviews/base.md"),
        review_with_revision("base", "low", "v2"),
    )?;
    h.advance("smoke-base", "validated")?;
    write(&review_schema, schema("SpecReview").to_string())?;
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
    Ok(started)
}

#[cfg(all(test, feature = "workflow-test"))]
fn design_gate_context_seeded(h: &Harness) -> Result<()> {
    let id = "smoke-design-gate-context";
    write(
        &h.root.join("spec/second-target.md"),
        simple("FR", "FR-002"),
    )?;
    let target_args = ["spec/target.md", "spec/second-target.md"];
    let canonical_targets = target_args
        .iter()
        .map(|target| {
            h.root
                .join(target)
                .canonicalize()
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>>>()?
        .iter()
        .map(|target| {
            target
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| "canonical target path is not UTF-8".to_owned())
        })
        .collect::<Result<Vec<_>>>()?;
    launch_design_gate(h, id, &target_args)?;
    let original_context = validate_design_gate_context(h, id, &canonical_targets)?;
    design_gate_proposer_writes_refused(h, id, &original_context)?;
    assert_design_gate_empty_targets(h)
}

#[cfg(all(test, feature = "workflow-test"))]
fn validate_design_gate_context(
    h: &Harness,
    id: &str,
    expected_targets: &[String],
) -> Result<Value> {
    let status = passed(&h.flow(&["status", id])?)?;
    let contexts = status
        .pointer("/data/items/quoin_context")
        .and_then(Value::as_array)
        .ok_or("design-gate host did not seed quoin_context")?;
    require(
        contexts.len() == 1,
        "design-gate must seed one context item",
    )?;
    let context = contexts.first().ok_or("design-gate context is missing")?;
    require(
        context.get("id").and_then(Value::as_str) == Some(id),
        "design-gate context id must be the engine run id",
    )?;
    require(
        context.get("repo_root").and_then(Value::as_str) == h.root.to_str(),
        "design-gate host must derive repo_root from the launch repository",
    )?;
    require(
        context.get("config_root").and_then(Value::as_str) == h.config.to_str(),
        "design-gate host must derive config_root from Quoin configuration",
    )?;
    let expected_targets = Value::Array(
        expected_targets
            .iter()
            .cloned()
            .map(Value::String)
            .collect(),
    );
    require(
        context.get("targets") == Some(&expected_targets),
        "design-gate targets must equal the canonicalized repeatable target arguments",
    )?;
    Ok(context.clone())
}

#[cfg(all(test, feature = "workflow-test"))]
fn design_gate_proposer_writes_refused(
    h: &Harness,
    id: &str,
    original_context: &Value,
) -> Result<()> {
    let unseeded_id = "smoke-design-gate-agent-add";
    let skill = h.skill("design-gate");
    let skill = skill.to_str().ok_or("design-gate path is not UTF-8")?;
    passed(&h.flow(&["run", "design-gate", "--path", skill, "--id", unseeded_id])?)?;
    refused(
        &h.flow(&[
            "add-item",
            unseeded_id,
            "quoin_context",
            "--item",
            &json!({"id":"agent-seed","config_root":"/attacker"}).to_string(),
        ])?,
        "item_system_owned",
    )?;
    refused(
        &h.flow(&[
            "add-item",
            id,
            "quoin_context",
            "--item",
            &json!({"id":"replacement","config_root":"/attacker"}).to_string(),
        ])?,
        "item_system_owned",
    )?;
    refused(
        &h.flow(&[
            "update-item",
            id,
            "quoin_context",
            id,
            "--patch",
            &json!({"config_root":"/attacker"}).to_string(),
        ])?,
        "item_system_owned",
    )?;
    let after = passed(&h.flow(&["status", id])?)?;
    let after_contexts = after
        .pointer("/data/items/quoin_context")
        .and_then(Value::as_array)
        .ok_or("design-gate context disappeared after rejected writes")?;
    require(
        after_contexts.len() == 1 && after_contexts.first() == Some(original_context),
        "rejected proposer writes must leave the seeded context unchanged",
    )
}

#[cfg(all(test, feature = "workflow-test"))]
fn assert_design_gate_empty_targets(h: &Harness) -> Result<()> {
    let empty_id = "smoke-design-gate-empty-targets";
    launch_design_gate(h, empty_id, &[])?;
    let empty_status = passed(&h.flow(&["status", empty_id])?)?;
    let empty_context = empty_status
        .pointer("/data/items/quoin_context/0")
        .ok_or("design-gate did not seed context for an empty target set")?;
    let no_targets = Value::Array(Vec::new());
    require(
        empty_context.get("targets") == Some(&no_targets),
        "omitted --target arguments must seed an empty target list",
    )
}

#[cfg(all(test, feature = "workflow-test"))]
fn launch_design_gate(h: &Harness, id: &str, targets: &[&str]) -> Result<Value> {
    let mut command = h.command(&h.binary)?;
    command
        .arg("--config-root")
        .arg(&h.config)
        .args(["design-gate", "--path"])
        .arg(h.skill("design-gate"))
        .args(["--id", id, "--json"]);
    for target in targets {
        command.arg("--target").arg(target);
    }
    passed(&command.output().map_err(|error| error.to_string())?)
}

#[cfg(all(test, feature = "workflow-test"))]
fn design_gate_missing_host_refused(h: &Harness) -> Result<()> {
    let fake_package = h.scratch.path().join("ix-flow-without-host");
    let fake_bin = fake_package.join("bin");
    let marker = h.scratch.path().join("ix-flow-run-invoked");
    let body = "if [ \"$1\" = \"--version\" ]; then printf '0.3.3\\n'; exit 0; fi\nprintf '%s\\n' \"$*\" > \"$QUOIN_TEST_RUN_MARKER\"\nprintf '{\"ok\":true,\"instance_id\":\"unexpected-run\"}\\n'";
    let windows_body = "@echo off\r\nif \"%~1\"==\"--version\" (echo 0.3.3& exit /b 0)\r\necho %* > \"%QUOIN_TEST_RUN_MARKER%\"\r\necho {\"ok\":true,\"instance_id\":\"unexpected-run\"}\r\n";
    executable(&fake_bin, "ix-flow", body, windows_body)?;
    let mut paths = vec![fake_bin];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let search = std::env::join_paths(paths).map_err(|error| error.to_string())?;
    let id = "smoke-design-gate-missing-host";
    let output = h
        .command(&h.binary)?
        .env("PATH", search)
        .env("QUOIN_TEST_RUN_MARKER", &marker)
        .arg("--config-root")
        .arg(&h.config)
        .args(["design-gate", "--path"])
        .arg(h.skill("design-gate"))
        .args(["--id", id, "--target", "spec/target.md", "--json"])
        .output()
        .map_err(|error| error.to_string())?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    require(
        !output.status.success()
            && stderr.contains("installed ix-flow package has no host API")
            && stderr.contains("update ix-flow before starting design-gate"),
        format!("missing ix-flow host API must fail actionably: {stderr}"),
    )?;
    require(
        !marker.exists() && !h.config.join("flows").join(format!("{id}.json")).exists(),
        "missing host API must be refused before ix-flow creates a run",
    )
}

#[cfg(all(test, feature = "workflow-test"))]
fn design_gate_incompatible_host_refused(h: &Harness) -> Result<()> {
    let fake_package = h.scratch.path().join("ix-flow-incompatible-host");
    let fake_bin = fake_package.join("bin");
    let host_entry = fake_package.join("dist/host.js");
    let marker = h.scratch.path().join("ix-flow-run-invoked");
    write(&host_entry, "export class WorkflowCommandRunner {}\n")?;
    let body = "if [ \"$1\" = \"--version\" ]; then printf '0.3.3\\n'; exit 0; fi\nprintf '%s\\n' \"$*\" > \"$QUOIN_TEST_RUN_MARKER\"\nprintf '{\"ok\":true,\"instance_id\":\"unexpected-run\"}\\n'";
    let windows_body = "@echo off\r\nif \"%~1\"==\"--version\" (echo 0.3.3& exit /b 0)\r\necho %* > \"%QUOIN_TEST_RUN_MARKER%\"\r\necho {\"ok\":true,\"instance_id\":\"unexpected-run\"}\r\n";
    executable(&fake_bin, "ix-flow", body, windows_body)?;
    let mut paths = vec![fake_bin];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    let search = std::env::join_paths(paths).map_err(|error| error.to_string())?;
    let id = "smoke-design-gate-incompatible-host";
    let output = h
        .command(&h.binary)?
        .env("PATH", search)
        .env("QUOIN_TEST_RUN_MARKER", &marker)
        .arg("--config-root")
        .arg(&h.config)
        .args(["design-gate", "--path"])
        .arg(h.skill("design-gate"))
        .args(["--id", id, "--target", "spec/target.md", "--json"])
        .output()
        .map_err(|error| error.to_string())?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    require(
        !output.status.success()
            && stderr.contains("WorkflowCommandRunner.seedSystemOwnedItem is unavailable")
            && stderr.contains("update ix-flow before starting design-gate"),
        format!("incompatible ix-flow host API must fail actionably: {stderr}"),
    )?;
    require(
        !marker.exists() && !h.config.join("flows").join(format!("{id}.json")).exists(),
        "incompatible host API must be refused before ix-flow creates a run",
    )
}

fn formalize_scenarios(h: &Harness) -> Result<()> {
    let tools = h.scratch.path().join("formalize tools");
    fs::create_dir_all(&tools).map_err(|e| e.to_string())?;
    let fsl_ok = executable(
        &tools,
        "fsl-ok",
        "printf '%s\\n' '{\"format\":\"filament-outcome/1\",\"operation\":\"check\",\"category\":\"success\",\"diagnostics\":[]}'; exit 0",
        "echo {\"format\":\"filament-outcome/1\",\"operation\":\"check\",\"category\":\"success\",\"diagnostics\":[]}\r\nexit /b 0",
    )?;
    let quire_ok = executable(
        &tools,
        "quire-ok",
        "printf '%s\\n' '{\"query\":{\"kind\":\"file\",\"path\":\"qsl/rule.native\"},\"claims\":{\"formalizes\":[{\"trace_id\":\"FR-001\",\"path\":\"qsl/rule.native\",\"symbol\":\"Rule\"}]}}'; exit 0",
        "echo {\"query\":{\"kind\":\"file\",\"path\":\"qsl/rule.native\"},\"claims\":{\"formalizes\":[{\"trace_id\":\"FR-001\",\"path\":\"qsl/rule.native\",\"symbol\":\"Rule\"}]}}\r\nexit /b 0",
    )?;
    // The request interview also requires these supporting paths. This smoke
    // only advances to `checked`, so the tools are not invoked and no approval
    // is signed.
    let analysis_package = h.scratch.path().join("analysis-package");
    let engineering_assurance = executable(&tools, "engineering-assurance", "exit 0", "exit /b 0")?;
    let allowed_signers = h.scratch.path().join("allowed-signers");
    write(&allowed_signers, "")?;
    write(
        &h.root.join("qsl/rule.native"),
        "// formalizes: FR-001\nRule: Bool = true;\n",
    )?;
    formalize_success_scenario(
        h,
        &fsl_ok,
        &quire_ok,
        &analysis_package,
        &engineering_assurance,
        &allowed_signers,
    )?;
    let fsl_bad = executable(
        &tools,
        "fsl-bad",
        "printf '%s\\n' '{\"format\":\"filament-outcome/1\",\"operation\":\"check\",\"category\":\"refusal\",\"diagnostics\":[{\"locus\":{\"kind\":\"region\",\"start\":1},\"message\":\"expected Boolean\"}]}' ; exit 20",
        "echo {\"format\":\"filament-outcome/1\",\"operation\":\"check\",\"category\":\"refusal\",\"diagnostics\":[{\"locus\":{\"kind\":\"region\",\"start\":1},\"message\":\"expected Boolean\"}]}\r\nexit /b 20",
    )?;
    formalize_retry_scenario(
        h,
        &fsl_bad,
        &quire_ok,
        &analysis_package,
        &engineering_assurance,
        &allowed_signers,
    )
}

fn formalize_success_scenario(
    h: &Harness,
    fsl: &Path,
    quire: &Path,
    analysis_package: &Path,
    engineering_assurance: &Path,
    allowed_signers: &Path,
) -> Result<()> {
    h.start("formalize", "smoke-formalize-success")?;
    h.answers(
        "smoke-formalize-success",
        "request",
        &json!({
            "requirement_id":"FR-001",
            "draft_path":"qsl/rule.native",
            "analysis_package":analysis_package,
            "engineering_assurance":engineering_assurance,
            "allowed_signers":allowed_signers
        }),
    )?;
    let success = h.flow_with_tools(
        &["advance", "smoke-formalize-success", "checked"],
        Some((fsl, quire)),
    )?;
    let success = passed(&success)?;
    require(
        text(&success, "/current_phase")? == "checked",
        "a clean FSL result with the exact formalizes relation reaches checked",
    )?;

    Ok(())
}

fn formalize_retry_scenario(
    h: &Harness,
    fsl: &Path,
    quire: &Path,
    analysis_package: &Path,
    engineering_assurance: &Path,
    allowed_signers: &Path,
) -> Result<()> {
    h.start("formalize", "smoke-formalize-stopped")?;
    h.answers(
        "smoke-formalize-stopped",
        "request",
        &json!({
            "requirement_id":"FR-001",
            "draft_path":"qsl/rule.native",
            "analysis_package":analysis_package,
            "engineering_assurance":engineering_assurance,
            "allowed_signers":allowed_signers
        }),
    )?;
    let mut diagnostics = Vec::new();
    for attempt in 0..4 {
        let failed = h.flow_with_tools(
            &["advance", "smoke-formalize-stopped", "checked"],
            Some((fsl, quire)),
        )?;
        let failure: Value = serde_json::from_slice(&failed.stdout).map_err(|e| e.to_string())?;
        require(
            failure.to_string().contains("qsl_check_failed"),
            format!("attempt {attempt} must retain the FSL diagnostic: {failure}"),
        )?;
        let last_diagnostic = failure.to_string();
        diagnostics.push(last_diagnostic.clone());
        let before_recording = passed(&h.flow(&["status", "smoke-formalize-stopped"])?)?;
        require(
            text(&before_recording, "/current_phase")? == "drafting"
                && before_recording
                    .pointer("/data/items/formalization_failure")
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len)
                    == attempt,
            format!("failed attempt {attempt} must return to drafting before its retry record"),
        )?;
        passed(
            &h.flow(&[
                "add-item",
                "smoke-formalize-stopped",
                "formalization_failure",
                "--item",
                &json!({"id":format!("fsl-failure-{attempt}"),"diagnostic":last_diagnostic})
                    .to_string(),
            ])?,
        )?;
        let after_recording = passed(&h.flow(&["status", "smoke-formalize-stopped"])?)?;
        let failures = after_recording
            .pointer("/data/items/formalization_failure")
            .and_then(Value::as_array)
            .ok_or("retry diagnostic items are absent")?;
        require(
            failures.len() == attempt + 1
                && failures
                    .last()
                    .and_then(|item| item.get("diagnostic"))
                    .and_then(Value::as_str)
                    == Some(last_diagnostic.as_str()),
            format!("failed attempt {attempt} must increment the recorded retry count"),
        )?;
    }
    let stopped = passed(&h.flow(&["advance", "smoke-formalize-stopped", "stopped"])?)?;
    require(
        text(&stopped, "/current_phase")? == "stopped",
        "four consecutive failed drafts reach stopped",
    )?;
    let final_diagnostic = diagnostics.last().ok_or("no checker failures recorded")?;
    let report = passed(&h.flow(&["status", "smoke-formalize-stopped"])?)?;
    require(
        report
            .pointer("/data/items/formalization_failure/3/diagnostic")
            .and_then(Value::as_str)
            == Some(final_diagnostic.as_str())
            && final_diagnostic.contains("expected Boolean")
            && final_diagnostic.contains("qsl_check_failed"),
        format!("the stopped run retains the final checker diagnostics: {report}"),
    )?;
    Ok(())
}

#[cfg(all(test, feature = "workflow-test"))]
fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|error| error.to_string())?;
    require(
        output.status.success(),
        format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
    )?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(all(test, feature = "workflow-test"))]
fn formalize_analysis_fixture(h: &Harness) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf)> {
    let module = json!({
        "name": "formalize-workflow-fixture",
        "version": "0.1.0",
        "description": "workflow smoke fixture",
        "artifact_types": [{"name":"FR"}],
        "traceability": {
            "trace_tags": {
                "formalizes": [{
                    "name": "formalizes",
                    "language": "qsl",
                    "syntax": "comment",
                    "pattern": "^// formalizes: (.+)$"
                }]
            }
        }
    });
    write(
        &h.root.join("manifest.yaml"),
        serde_yaml_ng::to_string(&module).map_err(|e| e.to_string())?,
    )?;
    let draft = "language qsl\nprofile core\n// formalizes: FR-001\nrule P1 { check x }\n";
    write(&h.root.join("qsl/rule.native"), draft)?;
    write(
        &h.root.join("spec/FR-001.md"),
        "---\nid: FR-001\ntype: FR\n---\n\n## Description\n\nSynthetic requirement.\n",
    )?;
    git(&h.root, &["init", "-q"])?;
    git(&h.root, &["config", "user.name", "workflow smoke"])?;
    git(
        &h.root,
        &["config", "user.email", "workflow-smoke@example.invalid"],
    )?;
    git(
        &h.root,
        &["add", "manifest.yaml", "qsl/rule.native", "spec/FR-001.md"],
    )?;
    git(
        &h.root,
        &["commit", "-q", "-m", "formalize workflow fixture"],
    )?;

    let mut package: Value = serde_json::from_slice(include_bytes!(
        "../../quoin-cli/tests/fixtures/formal-rules/checked-package-v2-success.json"
    ))
    .map_err(|error| error.to_string())?;
    let node_count = {
        let nodes = package
            .pointer_mut("/semantic_graph/nodes")
            .and_then(Value::as_array_mut)
            .ok_or("checked fixture has no semantic nodes")?;
        nodes.retain(|node| {
            node.pointer("/node_id/path/0").and_then(Value::as_str) != Some("claim/approved")
        });
        nodes.len()
    };
    let operand_rows = package
        .pointer_mut("/semantic_graph/operand_rows")
        .and_then(Value::as_array_mut)
        .ok_or("checked fixture has no operand rows")?;
    if operand_rows.len() != node_count + 1 {
        return Err("checked fixture node and operand row counts did not align".to_owned());
    }
    operand_rows.remove(4);
    let declaration_start = draft.find("rule P1").ok_or("draft declaration missing")?;
    let declaration_end = draft.trim_end().len();
    let source_map = package
        .get_mut("source_map")
        .and_then(Value::as_array_mut)
        .ok_or("checked fixture has no source map")?;
    source_map.retain(|entry| {
        entry.pointer("/node_id/path/0").and_then(Value::as_str) != Some("claim/approved")
    });
    for entry in source_map {
        if entry.pointer("/node_id/path/0").and_then(Value::as_str) == Some("claim/draft") {
            let region = entry
                .pointer_mut("/regions/0")
                .ok_or("draft claim region missing")?;
            region["source"]["identity"] = json!("qsl/rule.native");
            region["start"] = json!(declaration_start);
            region["end"] = json!(declaration_end);
        }
    }
    let sources = package
        .pointer_mut("/lock/sources")
        .and_then(Value::as_array_mut)
        .ok_or("checked fixture has no source list")?;
    sources.retain(|source| {
        source.get("identity").and_then(Value::as_str) != Some("approved/rule.native")
    });
    for source in sources {
        if source.get("identity").and_then(Value::as_str) == Some("draft/rule.native") {
            source["identity"] = json!("qsl/rule.native");
        }
    }
    let package_path = h.scratch.path().join("formalize-analysis-package.json");
    write(
        &package_path,
        serde_json::to_vec(&package).map_err(|e| e.to_string())?,
    )?;
    let allowed_signers = h.scratch.path().join("allowed-signers");
    write(&allowed_signers, "fixture\n")?;
    let tools = h.scratch.path().join("formalize analysis tools");
    fs::create_dir_all(&tools).map_err(|e| e.to_string())?;
    let ea = executable(
        &tools,
        "engineering-assurance",
        "if [ \"$1\" = \"--version\" ]; then printf 'engineering-assurance 0.8.2\\n'; exit 0; fi\nexit 99",
        "if \"%~1\"==\"--version\" (echo engineering-assurance 0.8.2& exit /b 0)\r\nexit /b 99",
    )?;
    Ok((package_path, ea, allowed_signers, tools))
}

#[cfg(all(test, feature = "workflow-test"))]
fn analyzer_for_status(directory: &Path, status: &str) -> Result<PathBuf> {
    let body = format!(
        r#"while [ "$#" -gt 0 ]; do case "$1" in --output) output="$2"; shift 2 ;; --request) mode="$2"; shift 2 ;; *) shift ;; esac; done
printf '{{"analysisKind":"%s","differential":{{"disposition":"agreement","agreedStatus":"{status}"}},"selection":{{"selected":["claim/draft"],"excluded":[]}}}}' "$mode" > "$output"
exit {exit_code}"#,
        exit_code = if status == "satisfied" { 0 } else { 1 }
    );
    executable(directory, "filament-analyze", &body, "exit /b 99")
}

#[cfg(all(test, feature = "workflow-test"))]
fn formalize_analysis_scenario(
    h: &Harness,
    id: &str,
    analyzer_status: &str,
    expected_phase: &str,
) -> Result<()> {
    let (package, ea, allowed_signers, tools) = formalize_analysis_fixture(h)?;
    let fsl = executable(
        &tools,
        "fsl-check",
        "printf '%s\\n' '{\"format\":\"filament-outcome/1\",\"operation\":\"check\",\"category\":\"success\",\"diagnostics\":[]}'; exit 0",
        "exit /b 0",
    )?;
    let quire = executable(
        &tools,
        "quire-trace",
        "printf '%s\\n' '{\"query\":{\"kind\":\"file\",\"path\":\"qsl/rule.native\"},\"claims\":{\"formalizes\":[{\"trace_id\":\"FR-001\",\"path\":\"qsl/rule.native\",\"symbol\":\"P1\"}]}}'; exit 0",
        "exit /b 0",
    )?;
    let analyzer = analyzer_for_status(&tools, analyzer_status)?;
    let z3 = executable(&tools, "z3", "exit 0", "exit /b 0")?;
    let cvc5 = executable(&tools, "cvc5", "exit 0", "exit /b 0")?;
    h.start("formalize", id)?;
    h.answers(
        id,
        "request",
        &json!({
            "requirement_id": "FR-001",
            "draft_path": "qsl/rule.native",
            "analysis_package": package,
            "engineering_assurance": ea,
            "allowed_signers": allowed_signers
        }),
    )?;
    let checked = passed(&h.flow_with_tools(&["advance", id, "checked"], Some((&fsl, &quire)))?)?;
    require(
        text(&checked, "/current_phase")? == "checked",
        "clean formalization reaches checked",
    )?;
    let status = passed(&h.flow(&["status", id])?)?;
    let request_item = status
        .pointer("/data/items/formalization_request/0")
        .ok_or("formalization request is missing")?;
    let item_id = request_item
        .get("id")
        .and_then(Value::as_str)
        .ok_or("formalization request has no item id")?;
    passed(&h.flow(&[
        "update-item",
        id,
        "formalization_request",
        item_id,
        "--patch",
        &json!({"drafted_symbols":["qsl:qsl/rule.native::P1"]}).to_string(),
    ])?)?;
    h.advance(id, "analysed")?;
    let outcome = h.flow_with_analysis_tools(
        &["advance", id, "awaiting_approval"],
        None,
        Some((&analyzer, &z3, &cvc5)),
    )?;
    if expected_phase == "awaiting_approval" {
        let result = passed(&outcome)?;
        require(
            text(&result, "/current_phase")? == expected_phase,
            "a clean analysis reaches awaiting_approval",
        )?;
    } else {
        let failure: Value = serde_json::from_slice(&outcome.stdout).map_err(|e| e.to_string())?;
        require(
            failure.to_string().contains("draft_claims_inconsistent"),
            format!("refuted analysis reports the failing claim: {failure}"),
        )?;
        let diagnostic = failure.to_string();
        passed(&h.flow(&[
            "add-item",
            id,
            "formalization_failure",
            "--item",
            &json!({"id":"analysis-failure-1","diagnostic":diagnostic}).to_string(),
        ])?)?;
        h.advance(id, expected_phase)?;
        let status = passed(&h.flow(&["status", id])?)?;
        require(
            text(&status, "/current_phase")? == expected_phase,
            format!("failed analysis returns to {expected_phase}: {status}"),
        )?;
        let after_recording = passed(&h.flow(&["status", id])?)?;
        require(
            after_recording
                .pointer("/data/items/formalization_failure")
                .and_then(Value::as_array)
                .is_some_and(|items| items.len() == 1),
            "failed analysis increments the recorded retry count",
        )?;
    }
    Ok(())
}

fn review_scenarios(h: &Harness, started: &Value) -> Result<()> {
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
    let profile_schema = h.module().join("schemas/AssuranceProfile.json");
    write(
        &h.root.join("spec/profile.md"),
        profile("v1", "require", &["spec-review", "code-review"]),
    )?;
    let mut answers = selection("subset", &["integrity"]);
    if let Some(map) = answers.as_object_mut() {
        map.insert("assurance_profiles".to_owned(), json!(["spec/profile.md"]));
        map.insert(
            "profile_disposition".to_owned(),
            json!("The active profile applies to spec-review; code-review remains a separate operation."),
        );
    }
    h.answers("smoke-profile", "request", &answers)?;
    let profile_run = passed(&h.flow(&["status", "smoke-profile"])?)?;
    let recorded = profile_run
        .pointer("/data/items/operation_request/0")
        .ok_or("review profile answers were not recorded in the run")?;
    require(
        recorded.get("assurance_profiles") == Some(&json!(["spec/profile.md"]))
            && recorded.get("profile_disposition").and_then(Value::as_str)
                == Some(
                    "The active profile applies to spec-review; code-review remains a separate operation.",
                ),
        format!("the run records profile paths and applicability: {recorded}"),
    )?;
    let mut live_profile_schema = schema("AssuranceProfile");
    live_profile_schema["properties"]["profileRevision"]["const"] = json!("v2");
    write(&profile_schema, live_profile_schema.to_string())?;
    refused(
        &h.flow(&["advance", "smoke-profile", "analyses_run"])?,
        "WORKFLOW_DOCUMENT_INVALID",
    )?;
    write(
        &h.root.join("spec/profile.md"),
        profile("v2", "require", &["spec-review", "code-review"]),
    )?;
    h.advance("smoke-profile", "analyses_run")?;
    h.advance("smoke-profile", "reviews_rendered")?;
    write(&h.root.join("spec/reviews/base.md"), review("base", "low"))?;
    write(
        &h.root.join("spec/reviews/integrity.md"),
        review("integrity", "low"),
    )?;
    h.add("smoke-profile", "base", "spec/reviews/base.md")?;
    h.add("smoke-profile", "integrity", "spec/reviews/integrity.md")?;
    h.advance("smoke-profile", "validated")?;
    write(&profile_schema, schema("AssuranceProfile").to_string())?;

    h.start("review", "smoke-obsolete-profile")?;
    write(
        &h.root.join("spec/obsolete-profile.md"),
        profile("v1", "require", &["spec-review"]).replace(
            "---\n# Profile\n",
            "review_selection: [all]\n---\n# Profile\n",
        ),
    )?;
    let mut obsolete = selection("base", &[]);
    obsolete["assurance_profiles"] = json!(["spec/obsolete-profile.md"]);
    h.answers("smoke-obsolete-profile", "request", &obsolete)?;
    refused(
        &h.flow(&["advance", "smoke-obsolete-profile", "analyses_run"])?,
        "WORKFLOW_DOCUMENT_INVALID",
    )?;

    h.start("review", "smoke-invalid-profile-policy")?;
    write(
        &h.root.join("spec/invalid-profile.md"),
        profile("v1", "automatic", &["spec-review"]),
    )?;
    let mut invalid_policy = selection("base", &[]);
    invalid_policy["assurance_profiles"] = json!(["spec/invalid-profile.md"]);
    h.answers("smoke-invalid-profile-policy", "request", &invalid_policy)?;
    refused(
        &h.flow(&["advance", "smoke-invalid-profile-policy", "analyses_run"])?,
        "WORKFLOW_DOCUMENT_INVALID",
    )?;

    h.start("review", "smoke-profile-invents-severity")?;
    let mut invented = profile("v1", "recommend", &["spec-review"]);
    invented = invented.replace(
        "  operations: [spec-review]",
        "  operations: [spec-review]\n  severity: high",
    );
    write(&h.root.join("spec/invented-profile.md"), invented)?;
    let mut invented_policy = selection("base", &[]);
    invented_policy["assurance_profiles"] = json!(["spec/invented-profile.md"]);
    h.answers(
        "smoke-profile-invents-severity",
        "request",
        &invented_policy,
    )?;
    refused(
        &h.flow(&["advance", "smoke-profile-invents-severity", "analyses_run"])?,
        "WORKFLOW_DOCUMENT_INVALID",
    )?;

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

fn plan_scenario(h: &Harness) -> Result<()> {
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
    for version in ["0.3.0", "0.3.1"] {
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
            .args(["review", "--path"])
            .arg(h.skill("review"))
            .args(["--id", "must-not-start", "--json"])
            .output()
            .map_err(|e| e.to_string())?;
        if version == "0.3.0" {
            require(
                !output.status.success()
                    && String::from_utf8_lossy(&output.stderr).contains("ix-flow >=0.3.1"),
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

#[cfg(all(test, feature = "workflow-test"))]
fn runner_unavailable_or_incompatible_refused(h: &Harness) -> Result<()> {
    let empty_bin = h.scratch.path().join("empty runner search path");
    fs::create_dir_all(&empty_bin).map_err(|error| error.to_string())?;
    let id = "smoke-missing-ix-flow";
    let missing = h
        .command(&h.binary)?
        .env("PATH", &empty_bin)
        .arg("--config-root")
        .arg(&h.config)
        .args(["review", "--path"])
        .arg(h.skill("review"))
        .args(["--id", id, "--json"])
        .output()
        .map_err(|error| error.to_string())?;
    let stderr = String::from_utf8_lossy(&missing.stderr);
    require(
        !missing.status.success() && stderr.contains("npm install -g @agent-ix/ix-flow@latest"),
        format!("missing ix-flow must report its install command: {stderr}"),
    )?;
    require(
        !h.config.join("flows").join(format!("{id}.json")).exists(),
        "a missing ix-flow must not create a run",
    )?;

    let fake_bin = h.scratch.path().join("incompatible runner path");
    let marker = h.scratch.path().join("incompatible runner invoked");
    let body = "if [ \"$1\" = \"--version\" ]; then printf '0.3.0\\n'; exit 0; fi\nprintf 'invoked\\n' > \"$QUOIN_TEST_RUN_MARKER\"\nexit 7";
    let windows_body = "@echo off\r\nif \"%~1\"==\"--version\" (echo 0.3.0& exit /b 0)\r\necho invoked > \"%QUOIN_TEST_RUN_MARKER%\"\r\nexit /b 7";
    executable(&fake_bin, "ix-flow", body, windows_body)?;
    let id = "smoke-incompatible-ix-flow";
    let incompatible = h
        .command(&h.binary)?
        .env("PATH", &fake_bin)
        .env("QUOIN_TEST_RUN_MARKER", &marker)
        .arg("--config-root")
        .arg(&h.config)
        .args(["review", "--path"])
        .arg(h.skill("review"))
        .args(["--id", id, "--json"])
        .output()
        .map_err(|error| error.to_string())?;
    let stderr = String::from_utf8_lossy(&incompatible.stderr);
    require(
        !incompatible.status.success()
            && stderr.contains("npm install -g @agent-ix/ix-flow@latest"),
        format!("incompatible ix-flow must report its install command: {stderr}"),
    )?;
    require(
        !marker.exists() && !h.config.join("flows").join(format!("{id}.json")).exists(),
        "an incompatible ix-flow must be refused before creating a run",
    )
}

fn copy_tree(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(source).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &target.join(entry.file_name()))?;
        } else if kind.is_file() {
            fs::copy(entry.path(), target.join(entry.file_name())).map_err(|e| e.to_string())?;
        } else {
            return Err("plugin install must contain regular files and directories".to_owned());
        }
    }
    Ok(())
}

#[cfg(all(test, feature = "workflow-test"))]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test assertions may panic"
)]
mod tests {
    use super::{
        Harness, design_gate_context_seeded, design_gate_incompatible_host_refused,
        design_gate_missing_host_refused, formalize_analysis_scenario, formalize_scenarios,
        runner_unavailable_or_incompatible_refused,
    };
    use ix_trace_rs::trace;

    #[cfg(feature = "workflow-test")]
    #[cfg(feature = "workflow-test")]
    mod formalize_criterion_tests;

    fn harness() -> Harness {
        let binary = std::env::var("QUOIN_BINARY").expect("QUOIN_BINARY from workflow-test");
        let runner = std::env::var("IX_FLOW_RUNNER").expect("IX_FLOW_RUNNER from workflow-test");
        let plugin =
            std::env::var("QUOIN_PLUGIN_ROOT").expect("QUOIN_PLUGIN_ROOT from workflow-test");
        Harness::new_for_workflow(&binary, &runner, &plugin, false).expect("workflow smoke harness")
    }

    #[cfg(feature = "workflow-test")]
    #[trace("FR-020-AC-1")]
    #[test]
    fn tc_fr020_relocated_plugin_launch_resolves_no_workflow() {
        let h = harness();
        let started = h
            .start("review", "fr020-relocated-plugin")
            .expect("review launches from copied plugin skill");
        let expected = h.skill("review");
        assert_eq!(
            started
                .pointer("/data/skillPath/absolute")
                .and_then(serde_json::Value::as_str),
            expected.to_str(),
            "ix-flow must use the copied plugin skill directly"
        );
        assert!(
            !h.config.join("workflows").exists(),
            "plugin installation must not extract a private workflow tree"
        );
    }

    #[cfg(feature = "workflow-test")]
    #[trace("FR-020-AC-3")]
    #[test]
    fn tc_fr020_missing_ix_flow_reports_supported_install_without_a_run() {
        runner_unavailable_or_incompatible_refused(&harness())
            .expect("missing and incompatible ix-flow are actionable and leave no run");
    }

    #[cfg(feature = "workflow-test")]
    use super::{review_base_lifecycle, review_scenarios};

    #[cfg(feature = "workflow-test")]
    #[trace(
        "FR-095-AC-1",
        "FR-095-AC-3",
        "FR-095-AC-4",
        "FR-095-AC-5",
        "FR-095-AC-6",
        "FR-095-AC-7",
        "FR-095-AC-8",
        "FR-095-AC-9",
        "FR-095-CON-2",
        "FR-095-CON-3"
    )]
    #[test]
    fn fr095_review_workflow_enforces_live_contracts() {
        let harness = harness();
        let started = review_base_lifecycle(&harness).expect("base review lifecycle");
        review_scenarios(&harness, &started).expect("review contract scenarios");
    }

    #[cfg(feature = "workflow-test")]
    #[trace("FR-020-AC-5", "FR-020-AC-6")]
    #[test]
    fn tc_fr020_relative_documents_resolve_under_subject_repo_with_spaces() {
        let harness = harness();
        design_gate_context_seeded(&harness).expect("design-gate host seeding and protection");
    }

    #[cfg(feature = "workflow-test")]
    #[trace("FR-020-AC-7")]
    #[test]
    fn design_gate_refuses_runner_without_host_api_before_run_creation() {
        let harness = harness();
        design_gate_missing_host_refused(&harness).expect("missing host API is refused");
    }

    #[cfg(feature = "workflow-test")]
    #[trace("FR-020-AC-7")]
    #[test]
    fn design_gate_refuses_incompatible_host_api_before_run_creation() {
        let harness = harness();
        design_gate_incompatible_host_refused(&harness).expect("incompatible host API is refused");
    }

    #[cfg(feature = "workflow-test")]
    #[trace("FR-189-AC-2", "FR-189-AC-3", "FR-189-AC-9")]
    #[test]
    fn formalize_failures_return_to_drafting_with_diagnostics() {
        let binary = std::env::var("QUOIN_BINARY").expect("QUOIN_BINARY from workflow-test");
        let runner = std::env::var("IX_FLOW_RUNNER").expect("IX_FLOW_RUNNER from workflow-test");
        let plugin =
            std::env::var("QUOIN_PLUGIN_ROOT").expect("QUOIN_PLUGIN_ROOT from workflow-test");
        let harness =
            Harness::new_for_workflow(&binary, &runner, &plugin, false).expect("smoke harness");
        formalize_scenarios(&harness).expect("formalize ix-flow scenarios");
    }

    #[cfg(feature = "workflow-test")]
    #[trace("FR-189-AC-11")]
    #[test]
    fn formalize_refuted_analysis_returns_to_drafting_with_retry_record() {
        let binary = std::env::var("QUOIN_BINARY").expect("QUOIN_BINARY from workflow-test");
        let runner = std::env::var("IX_FLOW_RUNNER").expect("IX_FLOW_RUNNER from workflow-test");
        let plugin =
            std::env::var("QUOIN_PLUGIN_ROOT").expect("QUOIN_PLUGIN_ROOT from workflow-test");
        let harness = Harness::new_for_workflow(&binary, &runner, &plugin, false)
            .expect("formalize smoke harness");
        formalize_analysis_scenario(
            &harness,
            "smoke-formalize-refuted-analysis",
            "refuted",
            "drafting",
        )
        .expect("refuted analysis returns to drafting and records a retry");
    }
}
