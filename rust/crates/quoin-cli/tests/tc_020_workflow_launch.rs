// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![cfg(unix)]
//! End-to-end workflow launcher contracts for the native Quoin CLI.
#![allow(
    clippy::expect_used,
    reason = "integration-test setup failures must be explicit"
)]

use ix_trace_rs::trace;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

struct Harness {
    scratch: TempDir,
    repo: PathBuf,
    bin: PathBuf,
    home: PathBuf,
    log: PathBuf,
    quoin: PathBuf,
}

impl Harness {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;

        let scratch = tempfile::tempdir().expect("scratch directory");
        let repo = scratch.path().join("subject repository with spaces");
        let bin = scratch.path().join("relocated bin");
        let home = scratch.path().join("empty home");
        let log = scratch.path().join("ix-flow.log");
        fs::create_dir_all(&repo).expect("subject repository");
        fs::create_dir_all(&bin).expect("relocated bin");
        fs::create_dir_all(&home).expect("empty home");

        let quoin = bin.join("quoin");
        fs::copy(env!("CARGO_BIN_EXE_quoin"), &quoin).expect("copy native Quoin executable");
        let mut permissions = fs::metadata(&quoin)
            .expect("copied executable metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&quoin, permissions).expect("copied executable is executable");

        let ix_flow = bin.join("ix-flow");
        fs::write(
            &ix_flow,
            r#"#!/bin/sh
operation="$1"
printf 'CALL %s\n' "$operation" >> "$IX_FLOW_LOG"
shift
printf 'ARG %s\n' "$@" >> "$IX_FLOW_LOG"
case "$operation" in
  --version) echo '0.3.1' ;;
  run) echo '{"ok":true,"instance_id":"run-from-stub"}' ;;
  add-item) echo '{}' ;;
  *) echo '{}' ;;
esac
"#,
        )
        .expect("ix-flow stub");
        fs::set_permissions(&ix_flow, fs::Permissions::from_mode(0o755))
            .expect("ix-flow stub is executable");

        Self {
            scratch,
            repo,
            bin,
            home,
            log,
            quoin,
        }
    }

    fn copy_review_skill(&self) -> PathBuf {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../skills/spec-review");
        let destination = self.scratch.path().join("copied plugin skill");
        fs::create_dir_all(destination.join("workflows/review"))
            .expect("copied workflow directory");
        fs::copy(source.join("SKILL.md"), destination.join("SKILL.md")).expect("copy plugin skill");
        fs::copy(
            source.join("workflows/review/def.yaml"),
            destination.join("workflows/review/def.yaml"),
        )
        .expect("copy plugin workflow");
        destination
    }

    fn quoin(&self, args: &[&str]) -> Output {
        Command::new(&self.quoin)
            .args(args)
            .current_dir(&self.repo)
            .env("HOME", self.scratch.path())
            .env("IX_HOME", &self.home)
            .env("IX_FLOW_LOG", &self.log)
            .env("IX_FLOW_MODE", "success")
            .env("PATH", &self.bin)
            .output()
            .expect("run relocated Quoin")
    }

    fn calls(&self) -> String {
        fs::read_to_string(&self.log).unwrap_or_default()
    }
}

#[trace("FR-020-AC-3")]
#[test]
fn missing_or_incompatible_runner_is_actionable_and_does_not_create_a_run() {
    let harness = Harness::new();
    let skill = harness.copy_review_skill();
    let args = [
        "--no-project-config",
        "--config-root",
        harness.home.to_str().expect("UTF-8 home"),
        "review",
        "--path",
        skill.to_str().expect("UTF-8 skill"),
    ];

    fs::remove_file(harness.bin.join("ix-flow")).expect("remove ix-flow for missing case");
    let output = harness.quoin(&args);
    assert!(!output.status.success(), "missing runner is refused");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("npm install -g @agent-ix/ix-flow@latest"),
        "{stderr}"
    );
    assert!(
        !harness.calls().contains("CALL run"),
        "{calls}",
        calls = harness.calls()
    );

    let incompatible = harness.bin.join("ix-flow");
    let source = r#"#!/bin/sh
operation="$1"
printf 'CALL %s\n' "$operation" >> "$IX_FLOW_LOG"
if [ "$operation" = "--version" ]; then echo '0.3.0'; exit 0; fi
echo '{"ok":true,"instance_id":"unexpected-run"}'
"#;
    fs::write(&incompatible, source).expect("incompatible runner stub");
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&incompatible, fs::Permissions::from_mode(0o755))
        .expect("incompatible runner is executable");
    let output = harness.quoin(&args);
    assert!(!output.status.success(), "incompatible runner is refused");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ix-flow >=0.3.1 is required"), "{stderr}");
    assert!(
        stderr.contains("npm install -g @agent-ix/ix-flow@latest"),
        "{stderr}"
    );
    let calls = harness.calls();
    assert!(calls.contains("CALL --version"), "{calls}");
    assert!(
        !calls.contains("CALL run"),
        "no run is created after a failed version check: {calls}"
    );
}

#[trace("FR-020-AC-1")]
#[test]
fn copied_review_skill_launches_from_relocated_cli_and_matrix_stays_a_view() {
    let harness = Harness::new();
    let skill = harness.copy_review_skill();
    assert!(
        !harness.repo.join("Cargo.toml").exists(),
        "the launch directory is not a source checkout"
    );
    assert!(
        !harness.repo.join("skills").exists(),
        "the copied plugin skill is outside the launch directory"
    );
    let skill = skill.to_str().expect("UTF-8 skill path");
    let home = harness.home.to_str().expect("UTF-8 home path");

    let output = harness.quoin(&[
        "--no-project-config",
        "--config-root",
        home,
        "review",
        "--path",
        skill,
        "--json",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = harness.calls();
    assert!(calls.contains("CALL --version"), "{calls}");
    assert!(calls.contains("CALL run"), "{calls}");
    assert!(calls.contains(&format!("ARG {skill}")), "{calls}");
    assert!(calls.contains("CALL add-item"), "{calls}");
    assert!(
        calls.contains(&format!("ARG {}", harness.home.display())),
        "{calls}"
    );
    assert!(
        calls.contains(&format!("ARG {}", harness.home.join("flows").display())),
        "{calls}"
    );

    let before_matrix = calls;
    let module = harness.scratch.path().join("matrix module");
    fs::create_dir_all(&module).expect("matrix module directory");
    fs::write(
        module.join("manifest.yaml"),
        "name: computed-view-fixture\nversion: 0.1.0\nartifact_types:\n  - name: FR\n    defaults:\n      id_pattern: FR-{next:03d}\ntraceability:\n  trace_targets:\n    - name: ac\n      archetype: FR\n      section: Acceptance Criteria\n      id_column: ID\n  obligations:\n    - name: ac\n      target: ac\n      statement_column: Criteria\n      method_column: Verification\n",
    )
    .expect("matrix module manifest");
    fs::create_dir_all(harness.repo.join("spec/functional")).expect("matrix spec directory");
    fs::write(
        harness.repo.join("spec/functional/FR-020.md"),
        "# FR-020: Flow command launch\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n| --- | --- | --- |\n| FR-020-AC-1 | Matrix stays a computed view. | Test |\n",
    )
    .expect("matrix criterion");
    let output = Command::new(&harness.quoin)
        .args([
            "--no-project-config",
            "matrix",
            "--repo",
            harness.repo.to_str().expect("UTF-8 repository path"),
            "--module",
            module.to_str().expect("UTF-8 module path"),
            "--json",
        ])
        .current_dir(&harness.repo)
        .env("HOME", harness.scratch.path())
        .env("IX_HOME", &harness.home)
        .env("IX_FLOW_LOG", &harness.log)
        .env("PATH", &harness.bin)
        .output()
        .expect("run matrix help");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.starts_with(b"{"),
        "matrix emits its computed JSON view"
    );
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("workflow"),
        "the computed matrix contains no workflow resolution: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(
        harness.calls(),
        before_matrix,
        "matrix computes criteria without resolving or launching a workflow"
    );
}

#[trace("FR-020-AC-5")]
#[test]
fn relative_targets_with_spaces_are_resolved_under_the_subject_repository() {
    let harness = Harness::new();
    let skill = harness.copy_review_skill();
    let target = harness.repo.join("spec/Requirement With Space.md");
    fs::create_dir_all(target.parent().expect("target parent")).expect("spec directory");
    fs::write(&target, "# Requirement\n").expect("target document");

    let output = harness.quoin(&[
        "--no-project-config",
        "--config-root",
        harness.home.to_str().expect("UTF-8 home"),
        "review",
        "--path",
        skill.to_str().expect("UTF-8 skill"),
        "--target",
        "spec/Requirement With Space.md",
        "--json",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = harness.calls();
    assert!(
        calls.contains(&format!(
            "ARG {}",
            harness.repo.canonicalize().unwrap().display()
        )),
        "the launch record names the canonical subject repository: {calls}"
    );
    assert!(
        calls.contains(&format!("ARG {}", target.canonicalize().unwrap().display())),
        "the relative target is resolved under that repository, preserving spaces: {calls}"
    );
}

#[trace("FR-020-AC-6")]
#[test]
fn design_gate_seeds_one_host_owned_context_from_canonical_launch_inputs() {
    let harness = Harness::new();
    let node = harness.bin.join("node");
    fs::write(
        &node,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$IX_NODE_LOG\"\necho '{}'\n",
    )
    .expect("node stub");
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&node, fs::Permissions::from_mode(0o755)).expect("node is executable");
    let host = harness
        .bin
        .join("node_modules/@agent-ix/ix-flow/dist/host.js");
    fs::create_dir_all(host.parent().expect("host parent")).expect("host API directory");
    fs::write(
        &host,
        "export class WorkflowCommandRunner { async seedSystemOwnedItem() {} }\n",
    )
    .expect("host API fixture");

    let target = harness.repo.join("design/request.md");
    fs::create_dir_all(target.parent().expect("target parent")).expect("design directory");
    fs::write(&target, "# Request\n").expect("design request");
    let skill = harness.scratch.path().join("design gate skill");
    fs::create_dir_all(skill.join("workflows/design-gate")).expect("design-gate workflow");
    fs::write(skill.join("SKILL.md"), "# Design Gate\n").expect("design-gate skill");
    fs::write(
        skill.join("workflows/design-gate/def.yaml"),
        "name: design-gate\nsystemOwnedItemTypes: [quoin_context]\n",
    )
    .expect("design-gate definition");

    let node_log = harness.scratch.path().join("node.log");
    let output = Command::new(&harness.quoin)
        .args([
            "--no-project-config",
            "--config-root",
            harness.home.to_str().expect("UTF-8 home"),
            "design-gate",
            "--path",
            skill.to_str().expect("UTF-8 skill"),
            "--target",
            "design/request.md",
            "--json",
        ])
        .current_dir(&harness.repo)
        .env("HOME", harness.scratch.path())
        .env("IX_HOME", &harness.home)
        .env("IX_FLOW_LOG", &harness.log)
        .env("IX_NODE_LOG", &node_log)
        .env("PATH", &harness.bin)
        .output()
        .expect("run relocated Quoin");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let flow_calls = harness.calls();
    assert!(flow_calls.contains("CALL --version"), "{flow_calls}");
    assert!(flow_calls.contains("CALL run"), "{flow_calls}");
    assert!(
        !flow_calls.contains("CALL add-item"),
        "design-gate uses host seeding: {flow_calls}"
    );
    let node_calls = fs::read_to_string(node_log).expect("host API preflight and seed calls");
    let seed_input: serde_json::Value = node_calls
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .last()
        .expect("the host seed receives JSON input");
    assert_eq!(
        node_calls.matches("--input-type=module").count(),
        2,
        "{node_calls}"
    );
    assert_eq!(
        node_calls
            .matches("seedSystemOwnedItem({ id: runId, type: 'quoin_context', item })")
            .count(),
        1,
        "exactly one host-owned seed call follows preflight: {node_calls}"
    );
    let repo = harness.repo.canonicalize().expect("canonical repo");
    let home = harness.home.canonicalize().expect("canonical config root");
    let target = target.canonicalize().expect("canonical target");
    assert_eq!(seed_input["runId"], "run-from-stub");
    assert_eq!(seed_input["cwd"], repo.to_string_lossy().as_ref());
    assert_eq!(
        seed_input["stateDir"],
        home.join("flows").to_string_lossy().as_ref()
    );
    assert_eq!(seed_input["item"]["id"], seed_input["runId"]);
    assert_eq!(
        seed_input["item"]["repo_root"],
        repo.to_string_lossy().as_ref()
    );
    assert_eq!(
        seed_input["item"]["config_root"],
        home.to_string_lossy().as_ref()
    );
    assert_eq!(
        seed_input["item"]["targets"],
        serde_json::json!([target.to_string_lossy()])
    );
    assert!(node_calls.contains("type: 'quoin_context'"), "{node_calls}");
}

#[trace("FR-020-AC-7")]
#[test]
fn design_gate_without_host_seeding_api_fails_before_creating_a_run() {
    let harness = Harness::new();
    let node = harness.bin.join("node");
    fs::write(
        &node,
        "#!/bin/sh\necho 'WorkflowCommandRunner.seedSystemOwnedItem is unavailable' >&2\nexit 1\n",
    )
    .expect("incompatible node stub");
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&node, fs::Permissions::from_mode(0o755)).expect("node is executable");
    let host = harness
        .bin
        .join("node_modules/@agent-ix/ix-flow/dist/host.js");
    fs::create_dir_all(host.parent().expect("host parent")).expect("host API directory");
    fs::write(&host, "export class WorkflowCommandRunner {}\n").expect("incompatible host API");
    let skill = harness.scratch.path().join("design gate skill");
    fs::create_dir_all(skill.join("workflows/design-gate")).expect("design-gate workflow");
    fs::write(skill.join("SKILL.md"), "# Design Gate\n").expect("design-gate skill");
    fs::write(
        skill.join("workflows/design-gate/def.yaml"),
        "name: design-gate\n",
    )
    .expect("design-gate definition");

    let node_log = harness.scratch.path().join("node.log");
    let output = Command::new(&harness.quoin)
        .args([
            "--no-project-config",
            "--config-root",
            harness.home.to_str().expect("UTF-8 home"),
            "design-gate",
            "--path",
            skill.to_str().expect("UTF-8 skill"),
        ])
        .current_dir(&harness.repo)
        .env("HOME", harness.scratch.path())
        .env("IX_HOME", &harness.home)
        .env("IX_FLOW_LOG", &harness.log)
        .env("IX_NODE_LOG", &node_log)
        .env("PATH", &harness.bin)
        .output()
        .expect("run relocated Quoin");
    assert!(!output.status.success(), "unsupported API must refuse");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("host API is incompatible"), "{stderr}");
    assert!(stderr.contains("update ix-flow"), "{stderr}");
    let calls = harness.calls();
    assert!(calls.contains("CALL --version"), "{calls}");
    assert!(
        !calls.contains("CALL run"),
        "no run is created before host API preflight: {calls}"
    );
}
