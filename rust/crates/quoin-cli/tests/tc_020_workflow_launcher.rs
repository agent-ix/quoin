// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::expect_used,
    reason = "test setup failures should identify the missing fixture"
)]

use ix_trace_rs::trace;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

struct Fixture {
    _root: TempDir,
    binary: PathBuf,
    home: PathBuf,
    subject: PathBuf,
    skill: PathBuf,
    bin: PathBuf,
    log: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().expect("fixture root");
        let binary = root.path().join("relocated/quoin");
        fs::create_dir_all(binary.parent().expect("binary parent")).expect("binary directory");
        fs::copy(env!("CARGO_BIN_EXE_quoin"), &binary).expect("copy the native CLI");
        let mut permissions = fs::metadata(&binary)
            .expect("binary metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&binary, permissions).expect("make copied CLI executable");

        let home = root.path().join("empty home");
        let subject = root.path().join("subject repository");
        let skill = root.path().join("copied plugin skill");
        let bin = root.path().join("runner bin");
        let log = root.path().join("ix-flow calls.log");
        fs::create_dir_all(&home).expect("empty home");
        fs::create_dir_all(&subject).expect("subject repository");
        fs::create_dir_all(skill.join("workflows/review")).expect("copied skill workflow");
        fs::write(skill.join("SKILL.md"), "# Review skill\n").expect("skill file");
        fs::write(skill.join("workflows/review/def.yaml"), "name: review\n")
            .expect("workflow definition");
        fs::create_dir_all(&bin).expect("runner bin directory");

        Self {
            _root: root,
            binary,
            home,
            subject,
            skill,
            bin,
            log,
        }
    }

    fn install_runner(&self, version: &str) {
        write_executable(
            &self.bin.join("ix-flow"),
            &format!(
                "printf '%s\\n' \"$1\" >> \"$IX_FLOW_LOG\"\ncase \"$1\" in\n  --version) printf '%s\\n' '{version}' ;;\n  run) printf '%s\\n' \"$@\" >> \"$IX_FLOW_LOG\"; printf '%s\\n' '{{\"ok\":true,\"instance_id\":\"fixture-run\"}}' ;;\n  add-item) printf '%s\\n' \"$@\" >> \"$IX_FLOW_LOG\"; printf '%s\\n' '{{\"ok\":true}}' ;;\nesac"
            ),
        );
    }

    fn call(&self, args: &[&str]) -> Output {
        Command::new(&self.binary)
            .args(args)
            .current_dir(&self.subject)
            .env("HOME", &self.home)
            .env("IX_HOME", &self.home)
            .env("IX_FLOW_LOG", &self.log)
            .env("PATH", &self.bin)
            .output()
            .expect("run the relocated Quoin CLI")
    }

    fn log(&self) -> String {
        fs::read_to_string(&self.log).unwrap_or_default()
    }
}

fn write_executable(path: &Path, body: String) {
    fs::write(path, format!("#!/bin/sh\n{body}\n")).expect("write fixture executable");
    let mut permissions = fs::metadata(path)
        .expect("executable metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("set executable mode");
}

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[trace("FR-020-AC-1")]
#[test]
fn copied_skill_launches_from_relocated_cli_and_matrix_does_not_launch_a_workflow() {
    let fixture = Fixture::new();
    fixture.install_runner("0.3.1");

    let output = fixture.call(&[
        "review",
        "--path",
        fixture.skill.to_str().expect("UTF-8 skill path"),
        "--json",
    ]);
    assert!(output.status.success(), "{}", text(&output));
    let calls = fixture.log();
    assert!(
        calls.contains("run\n"),
        "ix-flow run was not called: {calls}"
    );
    assert!(calls.contains(fixture.skill.to_str().expect("UTF-8 skill path")));
    assert!(calls.contains("fixture-run"));

    let before_matrix = calls;
    let matrix = fixture.call(&["matrix"]);
    assert!(matrix.status.success(), "{}", text(&matrix));
    assert_eq!(
        fixture.log(),
        before_matrix,
        "the computed matrix must not resolve a workflow through ix-flow"
    );
}

#[trace("FR-020-AC-3")]
#[test]
fn missing_or_incompatible_ix_flow_reports_the_install_command_before_any_run() {
    for version in [None, Some("0.3.0")] {
        let fixture = Fixture::new();
        if let Some(version) = version {
            fixture.install_runner(version);
        }
        let output = fixture.call(&[
            "review",
            "--path",
            fixture.skill.to_str().expect("UTF-8 skill path"),
        ]);
        let message = text(&output);
        assert!(!output.status.success(), "{message}");
        assert!(
            message.contains("npm install -g @agent-ix/ix-flow@latest"),
            "missing installation guidance: {message}"
        );
        assert!(
            !fixture.log().contains("run\n"),
            "workflow started: {message}"
        );
    }
}

#[trace("FR-020-AC-5")]
#[test]
fn relative_targets_with_spaces_are_resolved_under_the_subject_repository() {
    let fixture = Fixture::new();
    fixture.install_runner("0.3.1");
    let target = fixture.subject.join("evidence file.md");
    fs::write(&target, "subject evidence\n").expect("subject document");

    let output = fixture.call(&[
        "review",
        "--path",
        fixture.skill.to_str().expect("UTF-8 skill path"),
        "--target",
        "evidence file.md",
        "--json",
    ]);
    assert!(output.status.success(), "{}", text(&output));
    assert!(
        fixture
            .log()
            .contains(target.to_str().expect("UTF-8 target path")),
        "canonical subject target was not passed through the run context: {}",
        fixture.log()
    );
}

#[trace("FR-020-AC-7")]
#[test]
fn design_gate_refuses_a_runner_without_the_host_seeding_api_before_creating_a_run() {
    let fixture = Fixture::new();
    fixture.install_runner("0.3.1");
    let package = fixture
        .bin
        .parent()
        .expect("runner root")
        .join("node_modules/@agent-ix/ix-flow");
    fs::create_dir_all(package.join("dist")).expect("runner package directory");
    fs::write(package.join("package.json"), "{\"type\":\"module\"}\n")
        .expect("runner package metadata");
    fs::write(
        package.join("dist/host.js"),
        "export class WorkflowCommandRunner {}\n",
    )
    .expect("incompatible host entry");
    let node = std::env::split_paths(&std::env::var_os("PATH").expect("test PATH"))
        .map(|directory| directory.join("node"))
        .find(|candidate| candidate.is_file())
        .expect("Node is available for the design-gate host preflight");
    std::os::unix::fs::symlink(node, fixture.bin.join("node")).expect("link test Node");

    let output = fixture.call(&[
        "design-gate",
        "--path",
        fixture.skill.to_str().expect("UTF-8 skill path"),
    ]);
    let message = text(&output);
    assert!(!output.status.success(), "{message}");
    assert!(
        message.contains("installed ix-flow host API is incompatible"),
        "{message}"
    );
    assert!(message.contains("update ix-flow"), "{message}");
    assert_eq!(
        fixture.log(),
        "--version\n",
        "run must not start before preflight"
    );
    assert!(!fixture.home.join("flows").exists());
}
