// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Native Campaign CLI document and exit behavior over a fictional Git source.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "fixture setup and response assertions must fail the test"
)]

use std::fs;
use std::path::Path;
use std::process::Command;

use engineering_assurance::campaign::canonical_digest;
use quoin_measurement::campaign::store::{retain_json_bytes, run_path};
use quoin_store::{digest_bytes_sha256, store::write_content_addressed};
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
    reason = "one fictional Git source and its two retained verdict cases form one fixture"
)]
fn fixture() -> (tempfile::TempDir, String) {
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
    let (definition_digest, _) = retain_json_bytes(
        repo.path(),
        "definitions",
        &serde_json::to_vec(&definition).expect("definition JSON"),
    )
    .expect("retained definition");
    let source_graph_digest =
        canonical_digest(definition.get("sourceGraph").expect("source graph"))
            .expect("source graph digest")
            .as_str()
            .to_owned();
    let attempts = json!([{
        "member":"one", "index":1, "status":"invalid_request",
        "reason":"fictional_preflight"
    }]);
    for (run_id, verdict) in [
        ("missing-run", "inconclusive"),
        ("contradictory-run", "rejected"),
    ] {
        let run = json!({
            "schemaVersion":"engineering-assurance.campaign-run/v1",
            "id":run_id,
            "definitionDigest":definition_digest,
            "sourceGraphDigest":source_graph_digest,
            "attempts":attempts,
            "verdict":verdict
        });
        let path = run_path(repo.path(), run_id).expect("safe run path");
        let bytes = quoin_store::canonical_bytes(
            &quoin_store::parse_strict_json(&serde_json::to_vec(&run).expect("run JSON"))
                .expect("strict run JSON"),
        )
        .expect("canonical run");
        write_content_addressed(&path, &bytes).expect("retained run");
    }
    fs::write(
        repo.path().join("sources.json"),
        serde_json::to_vec(&json!({
            "schema":"quoin.campaign-sources/v1",
            "sources":{"fictional/source":repo.path()}
        }))
        .expect("source selection JSON"),
    )
    .expect("source selection");
    (repo, definition_digest)
}

/// Trace: FR-114-AC-5, TC-1946
/// A fictional source exercises the native CLI document and exit.
#[test]
fn tc_1946_campaign_verify_preserves_document_for_inconclusive_and_reject() {
    let (repo, definition_digest) = fixture();
    // Independently fixed SHA-256 of the fixture's canonical attempt array.
    let inventory_digest = "7fe0d5a085ce71d4b46650cff9e6cb242252f6e6615785e888fa74fa1694a286";
    for (run_id, verdict) in [
        ("missing-run", "inconclusive"),
        ("contradictory-run", "reject"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args(["measurement", "campaign", "verify", "--repo"])
            .arg(repo.path())
            .args([
                "--definition-digest",
                &definition_digest,
                "--run-id",
                run_id,
            ])
            .arg("--sources")
            .arg(repo.path().join("sources.json"))
            .output()
            .expect("native Quoin CLI");
        assert_eq!(output.status.code(), Some(1), "{run_id}: {output:?}");
        let document: Value = serde_json::from_slice(&output.stdout)
            .expect("one complete JSON verdict document on stdout");
        assert_eq!(document["schema"], "quoin.campaign-verdict/v1");
        assert_eq!(document["campaignId"], "fictional-campaign");
        assert_eq!(document["runId"], run_id);
        assert_eq!(document["definitionDigest"], definition_digest);
        assert_eq!(document["attemptInventoryDigest"], inventory_digest);
        assert_eq!(document["decision"]["verdict"], verdict);
        assert_eq!(
            document["decision"]["members"]
                .as_array()
                .expect("members")
                .len(),
            1
        );
        assert_eq!(
            document["decision"]["members"][0]["reasons"],
            json!(["execution_incomplete"])
        );
        let expected_reasons = if verdict == "reject" {
            json!(["evidence_contradiction"])
        } else {
            json!([])
        };
        assert_eq!(document["decision"]["reasons"], expected_reasons);
    }
}
