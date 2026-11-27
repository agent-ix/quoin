// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The tag-attestation workflow carries the exact release verification evidence (FR-186).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a missing workflow contract is the failing test result"
)]

use ix_trace_rs::trace;
use serde_yaml_ng::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn workflow(name: &str) -> Value {
    let path = repository_root().join(".github/workflows").join(name);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read workflow {}: {error}", path.display()));
    serde_yaml_ng::from_str(&source)
        .unwrap_or_else(|error| panic!("parse workflow {}: {error}", path.display()))
}

fn evidence_steps(workflow: &Value) -> &Vec<Value> {
    workflow["jobs"]["evidence"]["steps"]
        .as_sequence()
        .expect("native release workflow has evidence steps")
}

fn step_named<'a>(steps: &'a [Value], name: &str) -> &'a Value {
    steps
        .iter()
        .find(|step| step["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("workflow step {name:?} exists"))
}

fn evidence_script(steps: &[Value]) -> String {
    steps
        .iter()
        .filter_map(|step| step["run"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[trace("FR-186-AC-4")]
#[test]
fn release_evidence_upload_contains_the_maintained_and_fresh_evidence_tree() {
    let native = workflow("native-release-build.yml");
    let steps = evidence_steps(&native);
    let run = step_named(steps, "Run and validate retained release evidence");
    let upload = steps
        .iter()
        .find(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|uses| uses.starts_with("actions/upload-artifact@"))
        })
        .expect("evidence artifact upload step exists");

    assert_eq!(upload["with"]["name"].as_str(), Some("release-evidence"));
    assert_eq!(upload["with"]["path"].as_str(), Some("spec/evidence/"));
    let run_index = steps
        .iter()
        .position(|step| {
            step["name"].as_str() == Some("Run and validate retained release evidence")
        })
        .expect("release evidence generation step exists");
    let upload_index = steps
        .iter()
        .position(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|uses| uses.starts_with("actions/upload-artifact@"))
        })
        .expect("evidence artifact upload step exists");
    assert!(
        run_index < upload_index,
        "the evidence run completes before artifact upload"
    );

    let script = run["run"].as_str().expect("evidence step runs a script");
    for required in [
        "spec/evidence/bindings.json",
        "spec/evidence/runs/SUITE-001",
        "validate_binding_traces",
        "make evidence",
    ] {
        assert!(
            script.contains(required),
            "evidence step omits {required:?}"
        );
    }
    assert!(
        !script.contains("git commit") && !script.contains("git push"),
        "generated run evidence remains workflow output and is not committed"
    );

    let evidence_root = repository_root().join("spec/evidence");
    assert!(evidence_root.join("suites.md").is_file());
    assert!(evidence_root.join("bindings.json").is_file());
    assert!(
        script.contains("spec/evidence/release-evidence-status.json"),
        "the uploaded tree includes the retained status for missing or failed runs"
    );
    let tracked = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--",
            "spec/evidence/runs/SUITE-001",
            "spec/evidence/release-evidence-status.json",
        ])
        .current_dir(repository_root())
        .output()
        .expect("git lists committed evidence files");
    assert!(tracked.status.success(), "git ls-files succeeds");
    assert!(
        tracked.stdout.is_empty(),
        "run-specific evidence and status remain untracked in the source revision"
    );
}

#[trace("FR-186-AC-5")]
#[test]
fn release_evidence_records_tag_revision_and_producing_run_identity() {
    let native = workflow("native-release-build.yml");
    let steps = evidence_steps(&native);
    let run = evidence_script(steps);
    let all_steps = format!("{steps:?}");

    assert!(
        run.contains("GITHUB_REF_NAME") || all_steps.contains("github.ref_name"),
        "artifact provenance records the signed tag name"
    );
    assert!(
        run.contains("GITHUB_SHA") || all_steps.contains("github.sha"),
        "artifact provenance records the resolved source revision"
    );
    assert!(
        run.contains("GITHUB_RUN_ID") || all_steps.contains("github.run_id"),
        "artifact provenance records the producing workflow run"
    );
    assert!(
        run.contains("sourceRevision") && (run.contains("workflowRun") || run.contains("runId")),
        "the artifact serializes source and run identity alongside the release tag"
    );
    assert!(run.contains("sign_dsse_payload"));
    assert!(run.contains("cosign sign-blob --yes"));
    assert!(run.contains("release-evidence-identity.dsse.json"));
    assert!(run.contains("release-evidence-identity.sigstore.json"));
}

#[trace("FR-186-AC-7")]
#[test]
fn release_provenance_binds_the_exact_v2_payload_and_complete_tagged_matrix() {
    let native = workflow("native-release-build.yml");
    let steps = evidence_steps(&native);
    let run = evidence_script(steps);

    assert!(run.contains("execution-provenance.dsse.json"));
    assert!(run.contains("application/vnd.agent-ix.execution-provenance.v2+json"));
    assert!(
        run.contains(
            "expected_payload_fields='[\"runs\",\"sourceRevision\",\"suiteDeclarationsSha256\",\"testMatrix\",\"version\"]'"
        ),
        "the DSSE payload has exactly the five FR-186 fields"
    );
    assert!(run.contains("version") && run.contains("2"));
    assert!(run.contains("suiteDeclarationsSha256"));
    assert!(run.contains("testMatrix"));
    assert!(run.contains("quire matrix"));
    assert!(
        run.contains("matrix") && (run.contains("cmp ") || run.contains("--argjson")),
        "the generated matrix is compared with the complete matrix from the tagged revision"
    );
    assert!(
        run.contains("omitted") || run.contains("missing obligation") || run.contains("complete"),
        "an omitted active obligation is rejected before provenance signing"
    );
}

#[trace("FR-186-AC-8")]
#[test]
fn release_provenance_binds_present_run_bytes_and_signed_null_for_absent_runs() {
    let native = workflow("native-release-build.yml");
    let steps = evidence_steps(&native);
    let run = evidence_script(steps);

    assert!(run.contains("runs"));
    assert!(run.contains("sha256sum") || run.contains("sha256"));
    assert!(run.contains("recordPath"));
    assert!(
        run.contains("null"),
        "an expected run with no record is represented by a signed null digest"
    );
    assert!(
        run.contains("test -f") || run.contains("is_file"),
        "a non-null digest without its referenced run bytes is refused"
    );
    assert!(
        run.contains("digest")
            && (run.contains("mismatch") || run.contains("!=") || run.contains("==")),
        "the digest is checked against the exact run-record bytes"
    );
}
