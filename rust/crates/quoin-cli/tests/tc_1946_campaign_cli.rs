// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Native Campaign CLI document and exit behavior over a fictional Git source.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "fixture setup and response assertions must fail the test"
)]

use std::fs;
use std::path::Path;
use std::process::Command;
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

use quoin_measurement::campaign::store::run_path;
use quoin_store::digest_bytes_sha256;
use serde_json::{Value, json};

const PLAN: &str = r"---
type: MeasurementPlan
id: MP-FIXTURE
title: Fictional observation
status: active
stage: gate
metric: fictional.score
definition_version: v1
ground_truth_kind: mechanical
objective:
  direction: higher
statistical_design:
  population: one direct process check
  minimum_population: 1
  sampling: exhaustive
  repetitions: 1
  estimator: count
  error_model: none -- deterministic fixture
  uncertainty: none -- exact count
  decision_rule:
    comparator: ge
    threshold: 1
execution_procedure: campaign/procedure.json
protected_apparatus:
  - spec/assurance/fixture.md
  - campaign/procedure.json
  - campaign/producer.py
negative_controls:
  - kind: apparatus-edit
    description: fixture apparatus is bound to the exact source tree
---

# Fictional observation

## Comparison and Enforcement

Pass when one direct process check is accepted.
";

fn git(repo: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("Git runs");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[allow(
    clippy::too_many_lines,
    reason = "one fictional Git source forms one fixture"
)]
fn fixture() -> tempfile::TempDir {
    let repo = tempfile::tempdir().expect("temporary fictional repository");
    git(repo.path(), &["init", "-q"]);
    fs::create_dir_all(repo.path().join("spec/assurance")).expect("assurance directory");
    fs::create_dir_all(repo.path().join("campaign")).expect("campaign directory");
    fs::write(repo.path().join("spec/assurance/fixture.md"), PLAN).expect("plan");
    fs::write(
        repo.path().join("campaign/procedure.json"),
        serde_json::to_vec(&json!({
            "schemaVersion":"engineering-assurance.measurement-procedure/v1",
            "producerName":"fictional-tool", "producerVersion":"1",
            "sourceRepository":"fictional/source", "responseProtocol":"quoin.process-evidence/v1",
            "responseAdapter":"quoin.process-evidence-adapter", "responseAdapterVersion":"1",
            "arguments":[{"kind":"literal","value":"campaign/producer.py"}],
            "repetitions":1, "timeoutMillis":1000
        }))
        .expect("procedure JSON"),
    )
    .expect("procedure");
    fs::write(repo.path().join("campaign/producer.py"), "print('ok')\n").expect("producer");
    git(
        repo.path(),
        &["add", "spec/assurance/fixture.md", "campaign"],
    );
    git(
        repo.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-q",
            "-m",
            "fictional source",
        ],
    );
    let revision = String::from_utf8(git(repo.path(), &["rev-parse", "HEAD"]))
        .expect("revision")
        .trim()
        .to_owned();
    // BLOCKED-BY QSL-1275 (owner: planner titan:QSL:plan): EA CampaignSource.digest
    let source_digest = digest_bytes_sha256(&git(
        repo.path(),
        &["ls-tree", "-r", "-z", "--full-tree", &revision],
    ))
    .as_hex()
    .to_owned();
    let definition = json!({
        "schemaVersion":"engineering-assurance.campaign-definition/v1",
        "id":"fictional-campaign", "subjectName":"fictional-subject", "subjectVersion":"1",
        "sourceGraph":[{"repository":"fictional/source","revision":revision,"digest":source_digest}],
        "members":[{
            "name":"one", "group":"fixture", "planId":"MP-FIXTURE",
            "definitionVersion":"v1", "required":true
        }],
        "completionRule":"all_required"
    });
    fs::write(
        repo.path().join("definition.json"),
        serde_json::to_vec(&definition).expect("definition JSON"),
    )
    .expect("definition file");
    repo
}

/// Trace: FR-114-AC-2, FR-114-AC-5, TC-1941, TC-1946.
/// SIGINT reaches the exact Event-bound EA request and preserves a
/// non-success verdict document after the bounded process is cancelled.
#[cfg(target_os = "linux")]
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one sleeping producer, its cancellation and its retained verdict form one scenario"
)]
fn tc_1946_campaign_run_sigint_retains_inconclusive_document() {
    let repo = fixture();
    fs::write(
        repo.path().join("campaign/producer.py"),
        format!(
            "import time\nopen({:?}, 'w').close()\ntime.sleep(10)\nprint('ok')\n",
            repo.path().join("started-marker")
        ),
    )
    .expect("sleeping producer");
    let procedure_path = repo.path().join("campaign/procedure.json");
    let mut procedure: Value =
        serde_json::from_slice(&fs::read(&procedure_path).expect("procedure"))
            .expect("procedure JSON");
    procedure["timeoutMillis"] = json!(10000);
    fs::write(
        &procedure_path,
        serde_json::to_vec(&procedure).expect("procedure JSON"),
    )
    .expect("procedure file");
    git(
        repo.path(),
        &["add", "campaign/producer.py", "campaign/procedure.json"],
    );
    git(
        repo.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-q",
            "-m",
            "cancellable source",
        ],
    );
    let revision = String::from_utf8(git(repo.path(), &["rev-parse", "HEAD"]))
        .expect("revision")
        .trim()
        .to_owned();
    // BLOCKED-BY QSL-1275 (owner: planner titan:QSL:plan): EA CampaignSource.digest
    let source_digest = digest_bytes_sha256(&git(
        repo.path(),
        &["ls-tree", "-r", "-z", "--full-tree", &revision],
    ))
    .as_hex()
    .to_owned();
    let definition = json!({
        "schemaVersion":"engineering-assurance.campaign-definition/v1",
        "id":"fictional-campaign", "subjectName":"fictional-subject", "subjectVersion":"1",
        "sourceGraph":[{"repository":"fictional/source","revision":revision,"digest":source_digest}],
        "members":[{"name":"one","planId":"MP-FIXTURE","definitionVersion":"v1","required":true}],
        "completionRule":"all_required"
    });
    let definition_path = repo.path().join("definition.json");
    fs::write(
        &definition_path,
        serde_json::to_vec(&definition).expect("definition JSON"),
    )
    .expect("definition file");
    let python = fs::canonicalize("/usr/bin/python3").expect("system Python");
    // BLOCKED-BY QSL-1275 (owner: planner titan:QSL:plan): EA ProducerDescriptor.executable_digest / ContractBinding.digest
    let python_digest = quoin_store::digest_file_sha256(&python).expect("Python digest");
    let contract = |kind: &str| {
        json!({
            "kind":kind, "version":"1", "revision":revision,
            "digest":digest_bytes_sha256(kind.as_bytes()).as_hex()
        })
    };
    let config = json!({
        "schema":"quoin.campaign-run-config/v1",
        "sources":{"fictional/source":repo.path()},
        "members":{"one":{
            "producer":{
                "producer":{
                    "name":"fictional-tool", "version":"1", "sourceRevision":revision,
                    "executable":python, "executableDigest":python_digest.as_hex()
                },
                "caller":contract("fictional.caller"),
                "environment":{}, "outputs":[], "outputTrees":[],
                "stdin":{"kind":"null"},
                "containment":{"profile":"process-group-v1","contract":contract("fictional.containment")},
                "cancellation":{"kind":"event","authority":"fictional-campaign","event_id":"sigint"},
                "budget":{
                    "timeoutMillis":10000,"maxStdoutBytes":4096,"maxStderrBytes":4096,
                    "maxInputBytes":8_388_608,"maxOutputArtifacts":8,"maxOutputBytes":1_048_576,
                    "maxDescendants":8,"maxConcurrency":1
                },
                "responseProtocol":contract("quoin.process-evidence/v1"),
                "responseAdapter":contract("quoin.process-evidence-adapter"),
                "exitCodes":{"kind":"any"}
            },
            "checker":null,"inputs":[],"timestamp":"2026-09-24T00:00:00Z",
            "toolchains":{"python":"system-python3"},
            "sourceRemotes":{"fictional/source":"https://example.invalid/source.git"}
        }}
    });
    let config_path = repo.path().join("config.json");
    fs::write(
        &config_path,
        serde_json::to_vec(&config).expect("config JSON"),
    )
    .expect("config file");
    let mut child = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(["measurement", "campaign", "run", "--repo"])
        .arg(repo.path())
        .arg("--definition")
        .arg(&definition_path)
        .args(["--run-id", "sigint-run", "--config"])
        .arg(&config_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("native Quoin CLI starts");
    let marker = repo.path().join("started-marker");
    let started = Instant::now();
    while !marker.exists() {
        if child.try_wait().expect("CLI status").is_some() {
            let output = child.wait_with_output().expect("CLI output");
            panic!("CLI exited before the producer started: {output:?}");
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "producer started"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let signal = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("send SIGINT");
    assert!(signal.success(), "SIGINT delivered");
    let output = child.wait_with_output().expect("CLI exits");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).expect("verdict document");
    assert_eq!(document["verdict"], "inconclusive");
    assert_eq!(document["runId"], "sigint-run");
    let run: Value = serde_json::from_slice(
        &fs::read(run_path(repo.path(), "sigint-run").expect("run path")).expect("retained run"),
    )
    .expect("typed run JSON");
    assert_eq!(run["attempts"][0]["status"], "cancelled");
}
