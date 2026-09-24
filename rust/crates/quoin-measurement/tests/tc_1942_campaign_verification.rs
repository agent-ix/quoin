// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Generic campaign verification with a non-TL Git source fixture.

#![cfg(feature = "campaign")]
#![allow(clippy::expect_used, reason = "fixture setup must fail the test")]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use quoin_measurement::campaign::CampaignOutcome;
use quoin_measurement::campaign::checker::DomainVerdictReceipt;
use quoin_measurement::campaign::source::SourceError;
use quoin_measurement::campaign::store::{retain_json_bytes, run_path};
use quoin_measurement::campaign::verify::{CampaignVerificationError, verify_retained_campaign};
use quoin_store::{digest_bytes_sha256, store::write_content_addressed};
use serde_json::json;

fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("Git runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

const CHECKER_SCRIPT: &str = r"import hashlib
import json
import sys

def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()).hexdigest()

input_path = sys.argv[sys.argv.index('--input') + 1]
output_path = sys.argv[sys.argv.index('--output') + 1]
with open(input_path, encoding='utf-8') as stream:
    selected = json.load(stream)
with open(selected['resultPath'], encoding='utf-8') as stream:
    result = json.load(stream)
stdout = bytes(result['process']['stdout']['bytes'])
accepted = stdout == b'ok' and result['process']['terminalStatus'] == {'kind':'exit_code','value':0}
receipt = {
    'schema':'fictional.domain-verdict/v1',
    'member':selected['member'],
    'planId':selected['planId'],
    'definitionVersion':selected['definitionVersion'],
    'definitionDigest':selected['definitionDigest'],
    'sourceGraphDigest':selected['sourceGraphDigest'],
    'requestDigest':selected['requestDigest'],
    'resultDigest':selected['resultDigest'],
    'rawArtifactsDigest':digest(selected['rawArtifacts']),
    'rawBundleDigest':selected['rawBundleDigest'],
    'dependenciesDigest':digest(selected['dependencies']),
    'stdoutDigest':result['process']['stdout']['digest'],
    'stderrDigest':result['process']['stderr']['digest'],
    'verdict':'accept' if accepted else 'reject',
    'reasons':[] if accepted else ['fictional-output-mismatch'],
    'details':{'observation':{'stdout':'ok' if accepted else 'unexpected'},'samples':[1,2]},
}
with open(output_path, 'w', encoding='utf-8') as stream:
    json.dump(receipt, stream, sort_keys=True, separators=(',', ':'))
";

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
  - campaign/checker.py
negative_controls:
  - kind: apparatus-edit
    description: fixture apparatus is bound to the exact source tree
---

# Fictional observation

## Comparison and Enforcement

Pass when one direct process check is accepted.
";

fn fixture() -> (tempfile::TempDir, String, String) {
    let repo = tempfile::tempdir().expect("temporary repository");
    git(repo.path(), &["init", "-q"]);
    fs::write(
        repo.path().join("source.txt"),
        b"generic campaign fixture\n",
    )
    .expect("source file");
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        "/private/tmp/never-stage-this-target",
        repo.path().join("unrelated-link"),
    )
    .expect("unrelated tracked link");
    fs::create_dir_all(repo.path().join("spec/assurance")).expect("assurance directory");
    fs::create_dir_all(repo.path().join("campaign")).expect("campaign directory");
    fs::write(
        repo.path().join("campaign/producer.py"),
        "import sys\nsys.stdout.write('ok')\n",
    )
    .expect("producer script");
    fs::write(repo.path().join("campaign/checker.py"), CHECKER_SCRIPT).expect("checker script");
    fs::write(repo.path().join("spec/assurance/fixture.md"), PLAN).expect("measurement plan");
    fs::write(
        repo.path().join("campaign/procedure.json"),
        serde_json::to_vec(&json!({
            "schemaVersion":"engineering-assurance.measurement-procedure/v1",
            "producerName":"fictional-tool", "producerVersion":"1",
            "sourceRepository":"fictional/source", "responseProtocol":"quoin.process-evidence/v1",
            "responseAdapter":"quoin.process-evidence-adapter", "responseAdapterVersion":"1",
            "arguments":[{"kind":"literal","value":"campaign/producer.py"}],
            "repetitions":2, "timeoutMillis":1000
        }))
        .expect("procedure JSON"),
    )
    .expect("procedure file");
    git(
        repo.path(),
        &[
            "add",
            "source.txt",
            "spec/assurance/fixture.md",
            "campaign/procedure.json",
            "campaign/producer.py",
            "campaign/checker.py",
        ],
    );
    #[cfg(unix)]
    git(repo.path(), &["add", "unrelated-link"]);
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
            "source",
        ],
    );
    let revision = String::from_utf8(git(repo.path(), &["rev-parse", "HEAD"]))
        .expect("revision")
        .trim()
        .to_owned();
    let manifest = git(
        repo.path(),
        &["ls-tree", "-r", "-z", "--full-tree", &revision],
    );
    let source_digest = digest_bytes_sha256(&manifest).as_hex().to_owned();
    (repo, revision, source_digest)
}

/// Trace: FR-114-AC-2
/// Provenance: PLAT-1043
#[test]
fn tc_1942_legacy_domain_receipt_omits_optional_details() {
    let legacy = json!({
        "schema":"fictional.domain-verdict/v1", "member":"one", "planId":"MP-FIXTURE",
        "definitionVersion":"v1", "definitionDigest":"definition",
        "sourceGraphDigest":"source", "requestDigest":"request", "resultDigest":"result",
        "rawArtifactsDigest":"artifacts", "rawBundleDigest":"bundle",
        "dependenciesDigest":"dependencies", "stdoutDigest":null, "stderrDigest":null,
        "verdict":"accept", "reasons":[]
    });
    let receipt: DomainVerdictReceipt =
        serde_json::from_value(legacy.clone()).expect("legacy domain receipt");
    assert!(receipt.details.is_none());
    assert_eq!(
        serde_json::to_value(receipt).expect("encoded legacy domain receipt"),
        legacy
    );
}

/// Trace: FR-114-AC-1, FR-114-AC-4
/// Provenance: PLAT-1043
#[test]
fn tc_1942_missing_member_is_inconclusive_and_source_tampering_is_refused() {
    let (repo, revision, source_digest) = fixture();
    let definition = json!({
        "schemaVersion":"engineering-assurance.campaign-definition/v1",
        "id":"fictional-campaign",
        "subjectName":"fictional-subject",
        "subjectVersion":"1",
        "sourceGraph":[{
            "repository":"fictional/source",
            "revision":revision,
            "digest":source_digest
        }],
        "members":[{
            "name":"one",
            "group":"fixture",
            "planId":"MP-FIXTURE",
            "definitionVersion":"v1",
            "required":true
        }],
        "completionRule":"all_required"
    });
    let definition_bytes = serde_json::to_vec(&definition).expect("definition JSON");
    let (definition_digest, _) = retain_json_bytes(repo.path(), "definitions", &definition_bytes)
        .expect("retained definition");
    let source_graph_digest = engineering_assurance::campaign::canonical_digest(
        definition.get("sourceGraph").expect("source graph"),
    )
    .expect("source digest")
    .as_str()
    .to_owned();
    let run = json!({
        "schemaVersion":"engineering-assurance.campaign-run/v1",
        "id":"fixture-run",
        "definitionDigest":definition_digest,
        "sourceGraphDigest":source_graph_digest,
        "attempts":[],
        "verdict":"inconclusive"
    });
    let path = run_path(repo.path(), "fixture-run").expect("safe run path");
    let bytes = quoin_store::canonical_bytes(
        &quoin_store::parse_strict_json(&serde_json::to_vec(&run).expect("run JSON"))
            .expect("strict JSON"),
    )
    .expect("canonical run");
    write_content_addressed(&path, &bytes).expect("retained run");
    let checkouts = BTreeMap::from([("fictional/source".to_owned(), repo.path().to_path_buf())]);
    let verdict =
        verify_retained_campaign(repo.path(), &definition_digest, "fixture-run", &checkouts)
            .expect("valid run and source");
    assert_eq!(verdict.decision.verdict, CampaignOutcome::Inconclusive);
    fs::write(repo.path().join("source.txt"), b"altered\n").expect("mutated source");
    assert!(matches!(
        verify_retained_campaign(repo.path(), &definition_digest, "fixture-run", &checkouts),
        Err(CampaignVerificationError::Source(SourceError::Dirty(_)))
    ));
}

/// Trace: FR-114-AC-1, FR-114-AC-2, FR-114-AC-3, FR-114-AC-4
/// Provenance: PLAT-1043
#[cfg(target_os = "linux")]
#[test]
fn tc_1942_direct_process_and_checker_publish_protected_collection() {
    use engineering_assurance::campaign::{CampaignDefinition, CampaignVerdict, canonical_digest};
    use engineering_assurance::producer_execution::{
        CancellationBinding, ContainmentBinding, ContentDigest, ContractBinding, ExecutionBudget,
        ExitCodeBinding, OutputBinding, ProducerDescriptor, StdinBinding,
    };
    use quoin_measurement::campaign::run::{RunMemberBindings, run_campaign};
    use quoin_measurement::campaign::store::digest_path;

    let (repo, _, _) = fixture();
    let procedure_path = repo.path().join("campaign/procedure.json");
    let mut procedure: serde_json::Value =
        serde_json::from_slice(&fs::read(&procedure_path).expect("procedure"))
            .expect("procedure JSON");
    procedure["sourceRepository"] = json!("fictional/producer");
    fs::write(
        &procedure_path,
        serde_json::to_vec(&procedure).expect("procedure JSON"),
    )
    .expect("external producer procedure");
    // The campaign selects MP-FIXTURE. An older, unrelated tracked plan may
    // use vocabulary that the current measurement parser no longer accepts.
    fs::write(
        repo.path().join("spec/assurance/MP-LEGACY.md"),
        "---\ntype: MeasurementPlan\nid: MP-LEGACY\ntitle: Legacy\nstatus: active\nstage: observe\nmetric: legacy\ndefinition_version: v1\nstatistical_design:\n  estimator: old prose estimator\n---\n",
    )
    .expect("unrelated tracked legacy plan");
    git(
        repo.path(),
        &[
            "add",
            "campaign/procedure.json",
            "spec/assurance/MP-LEGACY.md",
        ],
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
            "external producer procedure",
        ],
    );
    let revision = String::from_utf8(git(repo.path(), &["rev-parse", "HEAD"]))
        .expect("plan revision")
        .trim()
        .to_owned();
    let source_digest = digest_bytes_sha256(&git(
        repo.path(),
        &["ls-tree", "-r", "-z", "--full-tree", &revision],
    ))
    .as_hex()
    .to_owned();
    let producer_repo = tempfile::tempdir().expect("producer repository");
    git(producer_repo.path(), &["init", "-q"]);
    fs::create_dir_all(producer_repo.path().join("campaign")).expect("producer directory");
    fs::write(
        producer_repo.path().join("campaign/producer.py"),
        "import sys\nsys.stdout.write('ok')\n",
    )
    .expect("producer script");
    git(producer_repo.path(), &["add", "campaign/producer.py"]);
    git(
        producer_repo.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-q",
            "-m",
            "producer",
        ],
    );
    let producer_revision = String::from_utf8(git(producer_repo.path(), &["rev-parse", "HEAD"]))
        .expect("producer revision")
        .trim()
        .to_owned();
    let producer_digest = digest_bytes_sha256(&git(
        producer_repo.path(),
        &["ls-tree", "-r", "-z", "--full-tree", &producer_revision],
    ))
    .as_hex()
    .to_owned();
    let definition: CampaignDefinition = serde_json::from_value(json!({
        "schemaVersion":"engineering-assurance.campaign-definition/v1",
        "id":"fictional-direct-campaign", "subjectName":"fictional-subject", "subjectVersion":"1",
        "sourceGraph":[
            {"repository":"fictional/source","revision":revision,"digest":source_digest},
            {"repository":"fictional/producer","revision":producer_revision,"digest":producer_digest}
        ],
        "members":[{
            "name":"one", "planId":"MP-FIXTURE", "definitionVersion":"v1", "required":true,
            "checkerProcedure":{
                "schemaVersion":"engineering-assurance.measurement-procedure/v1",
                "producerName":"fictional-checker", "producerVersion":"1",
                "sourceRepository":"fictional/source",
                "arguments":[
                    {"kind":"literal","value":"campaign/checker.py"},
                    {"kind":"literal","value":"--input"},
                    {"kind":"input_artifact","value":"bundle"},
                    {"kind":"literal","value":"--output"},
                    {"kind":"output_artifact","value":"verdict"}
                ],
                "inputs":[
                    {"role":"bundle","required":true},
                    {"role":"definition","required":true},
                    {"role":"request","required":true},
                    {"role":"result","required":true},
                    {"role":"rawBundle","required":true}
                ],
                "outputs":[{"role":"verdict","required":true}],
                "responseProtocol":"quoin.process-evidence/v1",
                "responseAdapter":"quoin.process-evidence-adapter",
                "responseAdapterVersion":"1", "repetitions":1, "timeoutMillis":5000
            }
        }],
        "completionRule":"all_required"
    })).expect("typed campaign definition");
    let python = fs::canonicalize("/usr/bin/python3").expect("system Python");
    let executable = python.to_str().expect("UTF-8 Python path").to_owned();
    let executable_digest = ContentDigest::of_file(&python).expect("Python identity");
    let contract = |kind: &str| ContractBinding {
        kind: kind.to_owned(),
        version: "1".to_owned(),
        revision: revision.clone(),
        digest: ContentDigest::of_bytes(kind.as_bytes()),
    };
    let bindings = |name: &str, checker: bool| engineering_assurance::campaign::ProcedureBindings {
        producer: ProducerDescriptor {
            name: name.to_owned(),
            version: "1".to_owned(),
            source_revision: if checker {
                revision.clone()
            } else {
                producer_revision.clone()
            },
            executable: executable.clone(),
            executable_digest: executable_digest.clone(),
        },
        caller: contract("fictional.caller"),
        capability_root: repo.path().to_string_lossy().into_owned(),
        environment: BTreeMap::new(),
        inputs: Vec::new(),
        input_origins: None,
        source_tree: None,
        outputs: if checker {
            vec![OutputBinding {
                role: "verdict".to_owned(),
                path: ".quoin-campaign/domain-verdict.json".to_owned(),
                required: true,
            }]
        } else {
            Vec::new()
        },
        output_trees: Vec::new(),
        stdin: StdinBinding::Null,
        containment: ContainmentBinding::ProcessGroupV1 {
            contract: contract("fictional.containment"),
        },
        cancellation: CancellationBinding::Disabled,
        budget: ExecutionBudget {
            timeout_millis: 5000,
            max_stdout_bytes: 4096,
            max_stderr_bytes: 4096,
            max_input_bytes: 8 * 1024 * 1024,
            max_output_artifacts: 8,
            max_output_bytes: 1024 * 1024,
            max_descendants: 8,
            max_concurrency: 1,
        },
        response_protocol: contract("quoin.process-evidence/v1"),
        response_adapter: contract("quoin.process-evidence-adapter"),
        exit_codes: ExitCodeBinding::Any,
    };
    let runtime = RunMemberBindings {
        producer: bindings("fictional-tool", false),
        checker: Some(bindings("fictional-checker", true)),
        inputs: Vec::new(),
        timestamp: "2026-09-23T00:00:00Z".to_owned(),
        toolchains: BTreeMap::from([("python".to_owned(), "system-python3".to_owned())]),
        source_remotes: BTreeMap::from([
            (
                "fictional/source".to_owned(),
                "https://example.invalid/fictional.git".to_owned(),
            ),
            (
                "fictional/producer".to_owned(),
                "https://example.invalid/producer.git".to_owned(),
            ),
        ]),
        environment_sources: BTreeMap::new(),
    };
    let checkouts = BTreeMap::from([
        ("fictional/source".to_owned(), repo.path().to_path_buf()),
        (
            "fictional/producer".to_owned(),
            producer_repo.path().to_path_buf(),
        ),
    ]);
    let runs = BTreeMap::from([("one".to_owned(), runtime)]);
    let run = run_campaign(
        repo.path(),
        &definition,
        "fixture-direct",
        &checkouts,
        &runs,
    )
    .expect("bounded direct campaign");
    assert_eq!(run.verdict, CampaignVerdict::Accepted, "{run:?}");
    let digest = canonical_digest(&definition).expect("definition identity");
    let receipt =
        verify_retained_campaign(repo.path(), digest.as_str(), "fixture-direct", &checkouts)
            .expect("independent retained replay");
    assert_eq!(
        receipt.decision.verdict,
        CampaignOutcome::Accept,
        "{receipt:?}"
    );

    // Trace: FR-114-AC-2, FR-114-AC-4
    // Provenance: PLAT-1043
    // The domain's nested detail remains in the sole sealed verdict artifact.
    let domain_digest = run.attempts.as_ref().expect("attempts")[0]
        .domain_verdict_digest
        .as_deref()
        .expect("domain verdict digest");
    let domain_path = digest_path(repo.path(), "domain-verdicts", domain_digest, "json")
        .expect("domain verdict path");
    let original_domain = fs::read(&domain_path).expect("retained domain verdict");
    let domain: serde_json::Value =
        serde_json::from_slice(&original_domain).expect("domain verdict JSON");
    assert_eq!(
        domain["details"],
        json!({"observation":{"stdout":"ok"},"samples":[1,2]})
    );
    let mut altered_domain = domain;
    altered_domain["details"]["samples"][0] = json!(9);
    let altered_bytes = serde_json::to_vec(&altered_domain).expect("altered domain verdict JSON");
    assert_ne!(digest_bytes_sha256(&altered_bytes).as_hex(), domain_digest);
    fs::write(&domain_path, altered_bytes).expect("altered retained domain verdict");
    let rejected =
        verify_retained_campaign(repo.path(), digest.as_str(), "fixture-direct", &checkouts)
            .expect("well-formed tampered run");
    assert_eq!(rejected.decision.verdict, CampaignOutcome::Reject);
    fs::write(&domain_path, &original_domain).expect("restore domain verdict");
    let restored =
        verify_retained_campaign(repo.path(), digest.as_str(), "fixture-direct", &checkouts)
            .expect("restored retained replay");
    assert_eq!(restored.decision.verdict, CampaignOutcome::Accept);

    // TC-1943: Rehash both the collection and the run so an
    // identity-only check would accept each altered context claim.
    let first = &run.attempts.as_ref().expect("attempts")[0];
    let collection_id = quoin_measurement::types::ids::CollectionId::parse(
        first.collection_id.as_deref().expect("collection ID"),
    )
    .expect("valid collection ID");
    let collection_path = quoin_measurement::store::measurement_path(repo.path(), &collection_id);
    let original_collection = fs::read(&collection_path).expect("retained collection");
    let run_file = run_path(repo.path(), "fixture-direct").expect("retained run path");
    let original_run = fs::read(&run_file).expect("retained run");
    for (pointer, replacement) in [
        ("/subject", json!("wrong-subject")),
        ("/scope/campaign", json!("wrong-campaign")),
        ("/sourceRevision", json!("wrong-revision")),
        ("/configDigest", json!(format!("sha256:{}", "0".repeat(64)))),
        (
            "/verificationStack/sources/fictional~1source/revision",
            json!("wrong-revision"),
        ),
        (
            "/verificationStack/artifacts/campaign~1procedure.json",
            json!(format!("sha256:{}", "0".repeat(64))),
        ),
    ] {
        let mut collection: serde_json::Value =
            serde_json::from_slice(&original_collection).expect("typed collection");
        *collection
            .pointer_mut(pointer)
            .expect("fixture context field") = replacement;
        let altered = serde_json::to_vec(&collection).expect("encoded collection");
        fs::write(&collection_path, &altered).expect("altered collection");
        let mut run_value: serde_json::Value =
            serde_json::from_slice(&original_run).expect("typed run");
        run_value["attempts"][0]["collectionDigest"] =
            json!(digest_bytes_sha256(&altered).as_hex());
        fs::write(
            &run_file,
            serde_json::to_vec(&run_value).expect("encoded run"),
        )
        .expect("altered run");
        let rejected =
            verify_retained_campaign(repo.path(), digest.as_str(), "fixture-direct", &checkouts)
                .expect("well-formed tampered run");
        assert_eq!(
            rejected.decision.verdict,
            CampaignOutcome::Reject,
            "tamper at {pointer}: {rejected:?}"
        );
        fs::write(&collection_path, &original_collection).expect("restore collection");
        fs::write(&run_file, &original_run).expect("restore run");
    }
}
