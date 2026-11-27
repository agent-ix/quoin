// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Exercise the retained evidence workflow step and its uploaded artifact path.

#![cfg(target_os = "linux")]
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "workflow contract assertions panic to report the violated release obligation"
)]

use ix_trace_rs::trace;
use serde_yaml_ng::Value;
use std::{
    env, fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::tempdir;

fn workflow() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../.github/workflows/native-release-build.yml");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read workflow {}: {error}", path.display()));
    serde_yaml_ng::from_str(&source)
        .unwrap_or_else(|error| panic!("parse workflow {}: {error}", path.display()))
}

fn git(repo: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git starts")
}

fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("write workflow fixture command");
    let mut permissions = fs::metadata(path)
        .expect("read command metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("make fixture command executable");
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create artifact directory");
    for entry in fs::read_dir(source).expect("read artifact source") {
        let entry = entry.expect("read artifact entry");
        let path = entry.path();
        let target = destination.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            fs::copy(&path, &target).expect("copy uploaded artifact file");
        }
    }
}

#[trace("FR-186-AC-4")]
#[test]
fn tc_fr186_uploaded_artifact_contains_maintained_and_fresh_evidence_without_committing_runs() {
    let definition = workflow();
    let evidence_job = &definition["jobs"]["evidence"]["steps"];
    let steps = evidence_job.as_sequence().expect("evidence job steps");
    let run_step = steps
        .iter()
        .find(|step| step["name"].as_str() == Some("Run and validate retained release evidence"))
        .expect("retained-evidence shell step exists");
    let script = run_step["run"].as_str().expect("shell script");
    let upload = steps
        .iter()
        .find(|step| {
            step["uses"].as_str() == Some("actions/upload-artifact@v7")
                && step["with"]["name"].as_str() == Some("release-evidence")
        })
        .expect("release-evidence upload step exists");
    let upload_path = upload["with"]["path"]
        .as_str()
        .expect("release-evidence upload path");

    let temp = tempdir().expect("temporary test root");
    let repo = temp.path().join("tagged-checkout");
    let evidence = repo.join("spec/evidence");
    fs::create_dir_all(&evidence).expect("create evidence source tree");
    fs::write(
        evidence.join("suites.md"),
        "# SUITE-001\n\nMaintained release-verification suite.\n",
    )
    .expect("write maintained suite declaration");
    let maintained_bindings = br#"{"schemaVersion":1,"bindings":[{"obligation":"FR-186-AC-4","suite":"SUITE-001","symbols":["tests::maintained"]}]}"#;
    fs::write(evidence.join("bindings.json"), maintained_bindings)
        .expect("write maintained binding registry");

    assert!(git(&repo, &["init", "-q", "-b", "main"]).status.success());
    assert!(
        git(&repo, &["config", "user.name", "Workflow Test"])
            .status
            .success()
    );
    assert!(
        git(&repo, &["config", "user.email", "workflow@example.test"])
            .status
            .success()
    );
    assert!(
        git(
            &repo,
            &[
                "add",
                "spec/evidence/suites.md",
                "spec/evidence/bindings.json"
            ]
        )
        .status
        .success()
    );
    assert!(
        git(&repo, &["commit", "-q", "-m", "tagged source"])
            .status
            .success()
    );
    let initial_revision = String::from_utf8(git(&repo, &["rev-parse", "HEAD"]).stdout)
        .expect("git revision is UTF-8")
        .trim()
        .to_owned();
    let initial_files =
        String::from_utf8(git(&repo, &["ls-tree", "-r", "--name-only", "HEAD"]).stdout)
            .expect("tree listing is UTF-8");

    let bin = temp.path().join("bin");
    fs::create_dir_all(&bin).expect("create fixture commands");
    write_executable(
        &bin.join("make"),
        "#!/bin/sh\nset -eu\ntest \"${1-}\" = evidence\nsleep 1\ncat > spec/evidence/bindings.json <<'JSON'\n{\"schemaVersion\":1,\"bindings\":[{\"obligation\":\"FR-186-AC-4\",\"suite\":\"SUITE-001\",\"symbols\":[\"tests::maintained\",\"tests::artifact\"]}]}\nJSON\nmkdir -p spec/evidence/runs/SUITE-001\ncat > spec/evidence/runs/SUITE-001/current.json <<'JSON'\n{\"schemaVersion\":1,\"suite\":\"SUITE-001\",\"evidenceKind\":\"Unit\",\"entries\":[{\"symbol\":\"tests::artifact\",\"outcome\":\"pass\",\"traceIds\":[\"FR-186-AC-4\"]}]}\nJSON\n",
    );
    write_executable(
        &bin.join("quoin"),
        "#!/bin/sh\nset -eu\ntest \"${1-}\" = matrix\nprintf '%s\\n' '{\"requirements\":[{\"document\":\"FR-186\",\"id\":\"FR-186\",\"criteria\":[{\"id\":\"FR-186-AC-4\",\"status\":\"active\"}]}]}'\n",
    );
    let runner_temp = temp.path().join("runner-temp");
    fs::create_dir_all(&runner_temp).expect("create runner temp directory");
    let current_path = env::var_os("PATH").expect("PATH exists");
    let path = env::join_paths(std::iter::once(bin.clone()).chain(env::split_paths(&current_path)))
        .expect("fixture PATH is valid");
    let output = Command::new("bash")
        .args(["-euo", "pipefail", "-c", script])
        .current_dir(&repo)
        .env("PATH", path)
        .env("RUNNER_TEMP", &runner_temp)
        .env("EXPECTED_TAG", "v0.0.0-test")
        .env("TAG_NAME", "v0.0.0-test")
        .env("SOURCE_REVISION", &initial_revision)
        .env("WORKFLOW_RUN_ID", "1234")
        .env("WORKFLOW_RUN_ATTEMPT", "1")
        .env("WORKFLOW_NAME", "native-release-build")
        .output()
        .expect("run the workflow's retained-evidence step");
    assert!(
        output.status.success(),
        "workflow evidence step failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let artifact_root = temp.path().join("uploaded-release-evidence");
    let source = repo.join(upload_path.trim_end_matches('/'));
    copy_tree(&source, &artifact_root);
    assert_eq!(
        fs::read(artifact_root.join("suites.md")).expect("artifact suite declaration"),
        fs::read(evidence.join("suites.md")).expect("source suite declaration")
    );
    assert_eq!(
        fs::read(artifact_root.join("bindings.json")).expect("artifact binding registry"),
        fs::read(evidence.join("bindings.json")).expect("source binding registry")
    );
    let uploaded_bindings = String::from_utf8_lossy(
        &fs::read(artifact_root.join("bindings.json")).expect("artifact bindings are UTF-8"),
    )
    .into_owned();
    assert!(uploaded_bindings.contains("tests::maintained"));
    assert!(uploaded_bindings.contains("tests::artifact"));
    let run_record = artifact_root.join("runs/SUITE-001/current.json");
    let run_record_bytes = fs::read(&run_record).expect("fresh run record is uploaded");
    assert!(String::from_utf8_lossy(&run_record_bytes).contains("FR-186-AC-4"));

    let final_revision = String::from_utf8(git(&repo, &["rev-parse", "HEAD"]).stdout)
        .expect("git revision is UTF-8")
        .trim()
        .to_owned();
    assert_eq!(
        final_revision, initial_revision,
        "workflow does not commit generated runs"
    );
    assert!(
        git(&repo, &["diff", "--cached", "--quiet"])
            .status
            .success()
    );
    let final_files =
        String::from_utf8(git(&repo, &["ls-tree", "-r", "--name-only", "HEAD"]).stdout)
            .expect("tree listing is UTF-8");
    assert_eq!(
        final_files, initial_files,
        "generated run files remain uncommitted"
    );
    assert!(!initial_files.contains("runs/SUITE-001/current.json"));
}
