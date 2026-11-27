// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Release-evidence suite bindings and handoff order (FR-186).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test assertions use panic as the test failure signal"
)]

use ix_trace_rs::trace;
use serde_json::{Value as JsonValue, json};
use serde_yaml_ng::Value as YamlValue;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repository_file(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../")
        .join(path)
}

fn workflow(name: &str) -> YamlValue {
    let path = repository_file(&format!(".github/workflows/{name}"));
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read workflow {}: {error}", path.display()));
    serde_yaml_ng::from_str(&source)
        .unwrap_or_else(|error| panic!("parse workflow {}: {error}", path.display()))
}

fn evidence_step<'a>(workflow: &'a YamlValue, name: &str) -> &'a YamlValue {
    workflow["jobs"]["evidence"]["steps"]
        .as_sequence()
        .expect("evidence job has steps")
        .iter()
        .find(|step| step["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("missing evidence step {name}"))
}

fn run_status_cases(shell_function: &str) {
    let temp = tempfile::tempdir().expect("temporary status directory");
    for (exit_code, record_count, expected_status) in [
        (0, 1, "complete"),
        (1, 1, "incomplete"),
        (0, 0, "incomplete"),
        (1, 0, "incomplete"),
    ] {
        let status_path = temp
            .path()
            .join(format!("status-{exit_code}-{record_count}.json"));
        let script = format!(
            "{shell_function}\nwrite_release_evidence_status \"$SUITE_EXIT_CODE\" \"$RUN_RECORD_COUNT\" \"$STATUS_PATH\""
        );
        let output = Command::new("bash")
            .arg("-c")
            .arg(script)
            .env("SUITE_EXIT_CODE", exit_code.to_string())
            .env("RUN_RECORD_COUNT", record_count.to_string())
            .env("STATUS_PATH", &status_path)
            .output()
            .expect("bash executes the workflow's status function");
        assert!(
            output.status.success(),
            "status helper failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let status: JsonValue = serde_json::from_slice(
            &fs::read(&status_path).expect("status helper retains its result"),
        )
        .expect("status result is JSON");
        assert_eq!(status["schemaVersion"], 1);
        assert_eq!(status["suite"], "SUITE-001");
        assert_eq!(status["status"], expected_status);
        assert_eq!(status["suiteExitCode"], exit_code);
        assert_eq!(status["runRecordCount"], record_count);
    }
}

fn binding_validator_accepts(
    shell_function: &str,
    bindings_path: &std::path::Path,
    runs_path: &std::path::Path,
    required: &str,
) -> bool {
    let script = format!(
        "{shell_function}\nvalidate_binding_traces \"$BINDINGS_PATH\" \"$RUNS_PATH\" \"$REQUIRED_OBLIGATIONS\""
    );
    Command::new("bash")
        .arg("-c")
        .arg(script)
        .env("BINDINGS_PATH", bindings_path)
        .env("RUNS_PATH", runs_path)
        .env("REQUIRED_OBLIGATIONS", required)
        .status()
        .expect("bash executes the workflow binding validator")
        .success()
}

fn assert_required_suite_bindings_are_declared() {
    let suites_path = repository_file("spec/evidence/suites.md");
    let suites = fs::read_to_string(&suites_path).unwrap_or_else(|error| {
        panic!("read suite declaration {}: {error}", suites_path.display())
    });
    for index in 1..=8 {
        let obligation = format!("FR-186-AC-{index}");
        assert!(
            suites.contains(&obligation),
            "SUITE-001 must declare {obligation} as a required release binding"
        );
    }
}

fn release_evidence_script() -> String {
    let native_release = workflow("native-release-build.yml");
    let steps = native_release["jobs"]["evidence"]["steps"]
        .as_sequence()
        .expect("evidence job has steps");
    let run_position = steps
        .iter()
        .position(|step| {
            step["name"].as_str() == Some("Run and validate retained release evidence")
        })
        .expect("release evidence attempt step exists");
    let upload_position = steps
        .iter()
        .position(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|uses| uses.starts_with("actions/upload-artifact@"))
                && step["with"]["name"].as_str() == Some("release-evidence")
        })
        .expect("release evidence artifact is uploaded");
    assert!(
        run_position < upload_position,
        "attempt completes before upload"
    );

    let run = steps[run_position]["run"]
        .as_str()
        .expect("release evidence attempt is a shell script")
        .to_owned();
    assert!(run.contains("make evidence"));
    let syntax = Command::new("bash")
        .arg("-n")
        .arg("-c")
        .arg(&run)
        .output()
        .expect("bash parses the release evidence script");
    assert!(
        syntax.status.success(),
        "release evidence script has invalid Bash syntax: {}",
        String::from_utf8_lossy(&syntax.stderr)
    );
    for index in 1..=8 {
        assert!(
            run.contains(&format!("FR-186-AC-{index}")),
            "healthy binding validation must require FR-186-AC-{index}"
        );
    }
    assert!(run.contains("release-evidence-status.json"));
    run
}

fn assert_binding_validator_rejects_incomplete_evidence(run: &str) {
    let validator_start = run
        .find("validate_binding_traces() {")
        .expect("binding validator has a stable extraction boundary");
    let validator_end = run[validator_start..]
        .find("\n}")
        .map(|offset| validator_start + offset + 2)
        .expect("binding validator function closes");
    let validator = &run[validator_start..validator_end];
    let temp = tempfile::tempdir().expect("temporary binding fixture directory");
    let required: Vec<_> = (1..=8).map(|index| format!("FR-186-AC-{index}")).collect();
    let required_json = serde_json::to_string(&required).expect("required obligations serialize");
    let bindings_path = temp.path().join("bindings.json");
    let runs_path = temp.path().join("runs.json");
    let bindings = json!({
        "schemaVersion": 1,
        "bindings": required.iter().map(|obligation| json!({
            "obligation": obligation,
            "suite": "SUITE-001",
            "symbols": [format!("tests::{obligation}")],
        })).collect::<Vec<_>>(),
    });
    let runs = json!([{
        "schemaVersion": 1,
        "suite": "SUITE-001",
        "evidenceKind": "Unit",
        "entries": required.iter().map(|obligation| json!({
            "symbol": format!("tests::{obligation}"),
            "outcome": "pass",
            "traceIds": [obligation],
        })).collect::<Vec<_>>(),
    }]);
    fs::write(
        &bindings_path,
        serde_json::to_vec(&bindings).expect("bindings serialize"),
    )
    .expect("bindings fixture writes");
    fs::write(
        &runs_path,
        serde_json::to_vec(&runs).expect("runs serialize"),
    )
    .expect("runs fixture writes");
    assert!(binding_validator_accepts(
        validator,
        &bindings_path,
        &runs_path,
        &required_json
    ));

    let mut missing_binding = bindings.clone();
    missing_binding["bindings"]
        .as_array_mut()
        .expect("bindings are an array")
        .pop();
    fs::write(
        &bindings_path,
        serde_json::to_vec(&missing_binding).expect("incomplete bindings serialize"),
    )
    .expect("incomplete bindings fixture writes");
    assert!(!binding_validator_accepts(
        validator,
        &bindings_path,
        &runs_path,
        &required_json
    ));

    fs::write(
        &bindings_path,
        serde_json::to_vec(&bindings).expect("bindings serialize"),
    )
    .expect("bindings fixture rewrites");
    let mut failed_run = runs;
    failed_run[0]["entries"][0]["outcome"] = json!("fail");
    fs::write(
        &runs_path,
        serde_json::to_vec(&failed_run).expect("failed run serializes"),
    )
    .expect("failed run fixture writes");
    assert!(!binding_validator_accepts(
        validator,
        &bindings_path,
        &runs_path,
        &required_json
    ));
}

fn assert_status_helper_marks_empty_success_incomplete(run: &str) {
    let start = run
        .find("# BEGIN release-evidence status helper")
        .expect("status helper has a stable extraction boundary");
    let end = run
        .find("# END release-evidence status helper")
        .expect("status helper has a stable extraction boundary");
    assert!(start < end, "status helper markers are ordered");
    run_status_cases(&run[start..end]);
}

#[trace("FR-186-AC-1")]
#[test]
fn tc_fr186_required_suite_bindings_and_incomplete_run_are_retained() {
    assert_required_suite_bindings_are_declared();
    let run = release_evidence_script();
    assert_binding_validator_rejects_incomplete_evidence(&run);
    assert_status_helper_marks_empty_success_incomplete(&run);
}

#[trace("FR-186-AC-2")]
#[test]
fn tc_fr186_attestation_waits_for_uploaded_evidence_and_names_its_artifact() {
    let tag_attestation = workflow("tag-attestation.yml");
    let attest = &tag_attestation["jobs"]["attest"];
    let evidence_build = &tag_attestation["jobs"]["build"];
    assert_eq!(
        evidence_build["permissions"]["id-token"].as_str(),
        Some("write")
    );
    let needs = attest["needs"]
        .as_sequence()
        .expect("attest depends on jobs");
    assert!(
        needs.iter().any(|job| job.as_str() == Some("build")),
        "attestation is skipped if the build or evidence artifact upload fails"
    );
    assert!(
        attest["if"].is_null(),
        "attestation has no unconditional override"
    );
    assert_eq!(
        attest["with"]["evidence_artifact"].as_str(),
        Some("release-evidence")
    );
    assert!(
        attest["uses"]
            .as_str()
            .expect("shared Engineering Assurance workflow is called")
            .starts_with(
                "agent-ix/engineering-assurance/.github/workflows/release-attestation.yml@"
            )
    );

    let native_release = workflow("native-release-build.yml");
    assert_eq!(
        native_release["jobs"]["evidence"]["permissions"]["id-token"].as_str(),
        Some("write")
    );
    assert!(native_release["jobs"]["package"]["permissions"]["id-token"].is_null());
    let manual_release = workflow("native-release.yml");
    assert!(manual_release["jobs"]["build"]["permissions"]["id-token"].is_null());
    let run = evidence_step(
        &native_release,
        "Run and validate retained release evidence",
    );
    let upload = native_release["jobs"]["evidence"]["steps"]
        .as_sequence()
        .expect("evidence job has steps")
        .iter()
        .find(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|uses| uses.starts_with("actions/upload-artifact@"))
                && step["with"]["name"].as_str() == Some("release-evidence")
        })
        .expect("evidence artifact upload step exists");
    assert!(run["run"].as_str().is_some());
    assert_eq!(upload["with"]["if-no-files-found"].as_str(), Some("error"));
    assert_eq!(upload["with"]["path"].as_str(), Some("spec/evidence/"));
    assert!(upload["continue-on-error"].is_null());
    assert!(upload["if"].is_null());
}
