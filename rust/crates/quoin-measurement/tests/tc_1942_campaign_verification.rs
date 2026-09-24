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

#[cfg(target_os = "linux")]
fn direct_fixture() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    engineering_assurance::campaign::CampaignDefinition,
    BTreeMap<String, std::path::PathBuf>,
    BTreeMap<String, quoin_measurement::campaign::run::RunMemberBindings>,
) {
    use engineering_assurance::campaign::CampaignDefinition;
    use engineering_assurance::producer_execution::{
        CancellationBinding, ContainmentBinding, ContentDigest, ContractBinding, ExecutionBudget,
        ExitCodeBinding, OutputBinding, ProducerDescriptor, StdinBinding,
    };
    use quoin_measurement::campaign::run::RunMemberBindings;

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
    (repo, producer_repo, definition, checkouts, runs)
}

/// Trace: FR-114-AC-1
/// Provenance: PLAT-1043, TC-1940
/// EA's generated wire types and cross-record validator are the campaign
/// intake boundary for malformed definitions and digest links.
#[cfg(target_os = "linux")]
#[test]
fn tc_1940_typed_definition_and_run_refusal_matrix() {
    use engineering_assurance::campaign::{
        CAMPAIGN_RUN_VERSION, CampaignError, CampaignRun, CampaignVerdict, MeasurementProcedure,
        PlanRegistration, canonical_digest, validate_definition, validate_run,
    };

    let (repo, _producer_repo, definition, _checkouts, _runs) = direct_fixture();
    let procedure: MeasurementProcedure = serde_json::from_slice(
        &fs::read(repo.path().join("campaign/procedure.json")).expect("procedure bytes"),
    )
    .expect("typed procedure");
    let registration = [PlanRegistration {
        id: "MP-FIXTURE",
        definition_version: "v1",
        procedure: Some(&procedure),
    }];
    validate_definition(&definition, &registration).expect("valid definition");

    let mut duplicate = definition.clone();
    duplicate.members.push(duplicate.members[0].clone());
    assert!(matches!(
        validate_definition(&duplicate, &registration),
        Err(CampaignError::Duplicate { kind: "member", .. })
    ));
    let mut missing = definition.clone();
    missing.members.clear();
    assert!(matches!(
        validate_definition(&missing, &registration),
        Err(CampaignError::NoEntries { field: "members" })
    ));
    let mut undeclared = definition.clone();
    undeclared.members[0].depends_on = Some(vec!["outside".to_owned()]);
    assert!(matches!(
        validate_definition(&undeclared, &registration),
        Err(CampaignError::Unresolved {
            kind: "dependency",
            ..
        })
    ));
    let mut cycle = definition.clone();
    let mut second = cycle.members[0].clone();
    second.name = "two".to_owned();
    second.depends_on = Some(vec!["one".to_owned()]);
    cycle.members[0].depends_on = Some(vec!["two".to_owned()]);
    cycle.members.push(second);
    assert!(matches!(
        validate_definition(&cycle, &registration),
        Err(CampaignError::DependencyCycle)
    ));
    let mut unknown_rule = serde_json::to_value(&definition).expect("definition JSON");
    unknown_rule["completionRule"] = json!("first-accepted");
    assert!(serde_json::from_value::<engineering_assurance::campaign::CampaignDefinition>(
        unknown_rule,
    ).is_err());

    let mut run = CampaignRun {
        schema_version: CAMPAIGN_RUN_VERSION.to_owned(),
        id: "intake-matrix".to_owned(),
        definition_digest: canonical_digest(&definition)
            .expect("definition identity")
            .as_str()
            .to_owned(),
        source_graph_digest: canonical_digest(&definition.source_graph)
            .expect("source graph identity")
            .as_str()
            .to_owned(),
        attempts: None,
        verdict: CampaignVerdict::Inconclusive,
    };
    validate_run(&run, &definition).expect("well-linked run");
    run.definition_digest = "0".repeat(64);
    assert!(matches!(
        validate_run(&run, &definition),
        Err(CampaignError::Binding {
            field: "definitionDigest"
        })
    ));
    run.definition_digest = canonical_digest(&definition)
        .expect("definition identity")
        .as_str()
        .to_owned();
    run.source_graph_digest = "0".repeat(64);
    assert!(matches!(
        validate_run(&run, &definition),
        Err(CampaignError::Binding {
            field: "sourceGraphDigest"
        })
    ));
}

#[cfg(target_os = "linux")]
fn assert_retained_unchecked_attempts(
    repo: &Path,
    definition: &engineering_assurance::campaign::CampaignDefinition,
    checkouts: &BTreeMap<String, std::path::PathBuf>,
    run_id: &str,
    run: &engineering_assurance::campaign::CampaignRun,
    reason: &str,
    checker_executed: bool,
) {
    use engineering_assurance::campaign::{
        CampaignAttemptStatus, CampaignVerdict, canonical_digest,
    };

    assert_eq!(run.verdict, CampaignVerdict::Inconclusive, "{run:?}");
    let attempts = run.attempts.as_ref().expect("retained attempts");
    assert_eq!(attempts.len(), 2, "one collection per producer repetition");
    let mut collection_ids = std::collections::BTreeSet::new();
    for attempt in attempts {
        assert_eq!(attempt.status, CampaignAttemptStatus::Completed);
        assert_eq!(attempt.reason.as_deref(), Some(reason));
        assert_eq!(attempt.checker_request_digest.is_some(), checker_executed);
        assert_eq!(attempt.checker_result_digest.is_some(), checker_executed);
        assert!(attempt.domain_verdict_digest.is_none());
        assert!(attempt.verdict_digest.is_some());
        let id = attempt.collection_id.as_deref().expect("collection ID");
        assert!(collection_ids.insert(id));
        let parsed_id =
            quoin_measurement::types::ids::CollectionId::parse(id).expect("valid collection ID");
        let path = quoin_measurement::store::measurement_path(repo, &parsed_id);
        let bytes = fs::read(path).expect("retained collection");
        assert_eq!(
            attempt.collection_digest.as_deref(),
            Some(digest_bytes_sha256(&bytes).as_hex())
        );
        let collection: serde_json::Value =
            serde_json::from_slice(&bytes).expect("collection JSON");
        assert_eq!(
            collection["observations"][0]["state"],
            json!("not_computed")
        );
        assert_eq!(
            collection["observations"][0]["value"],
            serde_json::Value::Null
        );
        assert_eq!(collection["observations"][0]["reason"], json!(reason));
        assert_eq!(
            collection["rawEvidence"]["resultDigest"],
            json!(attempt.result_digest)
        );
        assert_eq!(
            collection["rawEvidence"]["checkerResultDigest"].is_string(),
            checker_executed
        );
        assert!(collection["rawEvidence"]["domainVerdictDigest"].is_null());
    }
    let digest = canonical_digest(definition).expect("definition identity");
    let replay = verify_retained_campaign(repo, digest.as_str(), run_id, checkouts)
        .expect("independent retained replay");
    assert_eq!(
        replay.decision.verdict,
        CampaignOutcome::Inconclusive,
        "{replay:?}"
    );
}

/// Trace: FR-114-AC-2, FR-114-AC-3, FR-114-AC-4
/// Provenance: PLAT-1043
#[cfg(target_os = "linux")]
#[test]
fn tc_1942_completed_producer_without_checker_retains_each_collection() {
    use quoin_measurement::campaign::run::run_campaign;

    let (repo, _producer_repo, mut definition, checkouts, runs) = direct_fixture();
    definition.members[0].checker_procedure = None;
    let run_id = "fixture-no-checker";
    let run = run_campaign(repo.path(), &definition, run_id, &checkouts, &runs)
        .expect("completed producer without checker");
    assert_retained_unchecked_attempts(
        repo.path(),
        &definition,
        &checkouts,
        run_id,
        &run,
        "independent_checker_unavailable",
        false,
    );
}

/// Trace: FR-114-AC-2, FR-114-AC-3, FR-114-AC-4
/// Provenance: PLAT-1043
#[cfg(target_os = "linux")]
#[test]
fn tc_1942_unavailable_checker_retains_each_collection() {
    use quoin_measurement::campaign::run::run_campaign;

    let (repo, _producer_repo, definition, checkouts, mut runs) = direct_fixture();
    runs.get_mut("one").expect("fixture member").checker = None;
    let run_id = "fixture-checker-unavailable";
    let run = run_campaign(repo.path(), &definition, run_id, &checkouts, &runs)
        .expect("completed producer with unavailable checker");
    assert_retained_unchecked_attempts(
        repo.path(),
        &definition,
        &checkouts,
        run_id,
        &run,
        "independent_checker_unavailable",
        false,
    );
}

/// Trace: FR-114-AC-2, FR-114-AC-3, FR-114-AC-4
/// Provenance: PLAT-1043
#[cfg(target_os = "linux")]
#[test]
fn tc_1942_malformed_checker_receipt_retains_each_collection() {
    use engineering_assurance::campaign::CampaignDefinition;
    use quoin_measurement::campaign::run::run_campaign;
    use quoin_measurement::campaign::store::digest_path;

    let (repo, _producer_repo, definition, checkouts, runs) = direct_fixture();
    let mut value = serde_json::to_value(definition).expect("definition JSON");
    value["members"][0]["checkerProcedure"]["arguments"] = json!([
        {"kind":"literal","value":"-c"},
        {"kind":"literal","value":"import sys; open(sys.argv[sys.argv.index('--output')+1], 'wb').write(b'not-json')"},
        {"kind":"literal","value":"--output"},
        {"kind":"output_artifact","value":"verdict"}
    ]);
    let definition: CampaignDefinition =
        serde_json::from_value(value).expect("malformed checker definition");
    let run_id = "fixture-checker-malformed";
    let run = run_campaign(repo.path(), &definition, run_id, &checkouts, &runs)
        .expect("completed producer with malformed checker receipt");
    assert_retained_unchecked_attempts(
        repo.path(),
        &definition,
        &checkouts,
        run_id,
        &run,
        "checker_verdict_malformed",
        true,
    );
    let definition_digest =
        engineering_assurance::campaign::canonical_digest(&definition).expect("definition digest");
    let first = &run.attempts.as_ref().expect("retained attempts")[0];
    for (kind, digest) in [
        (
            "requests",
            first
                .checker_request_digest
                .as_deref()
                .expect("checker request digest"),
        ),
        (
            "results",
            first
                .checker_result_digest
                .as_deref()
                .expect("checker result digest"),
        ),
    ] {
        let path = digest_path(repo.path(), kind, digest, "json").expect("checker record path");
        let original = fs::read(&path).expect("checker record bytes");
        fs::write(&path, b"{}" as &[u8]).expect("tamper checker record");
        let replay =
            verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
                .expect("tampered checker replay assessment");
        assert_eq!(replay.decision.verdict, CampaignOutcome::Reject, "{kind}");
        fs::write(&path, original).expect("restore checker record");
    }

    // A coherent new hash chain must not turn a completed checker that
    // produced malformed bytes into an unavailable checker while keeping the
    // old `checker_verdict_malformed` reason.
    let old_result_digest = first
        .checker_result_digest
        .as_deref()
        .expect("checker result digest");
    let old_result_path =
        digest_path(repo.path(), "results", old_result_digest, "json").expect("result path");
    let mut result: serde_json::Value =
        serde_json::from_slice(&fs::read(old_result_path).expect("result bytes"))
            .expect("result JSON");
    result["state"] = json!({"kind":"unavailable"});
    let (new_result_digest, _) =
        quoin_measurement::campaign::store::retain_value(repo.path(), "results", &result)
            .expect("retain substituted result");
    let collection_id = quoin_measurement::types::ids::CollectionId::parse(
        first.collection_id.as_deref().expect("collection ID"),
    )
    .expect("valid collection ID");
    let collection_path = quoin_measurement::store::measurement_path(repo.path(), &collection_id);
    let mut collection: serde_json::Value =
        serde_json::from_slice(&fs::read(&collection_path).expect("collection bytes"))
            .expect("collection JSON");
    collection["rawEvidence"]["checkerResultDigest"] = json!(new_result_digest);
    let collection_bytes = serde_json::to_vec(&collection).expect("collection JSON");
    fs::write(collection_path, &collection_bytes).expect("substituted collection");
    let mut run_value: serde_json::Value = serde_json::from_slice(
        &fs::read(run_path(repo.path(), run_id).expect("run path")).expect("run bytes"),
    )
    .expect("run JSON");
    run_value["attempts"][0]["checkerResultDigest"] = json!(new_result_digest);
    run_value["attempts"][0]["collectionDigest"] =
        json!(digest_bytes_sha256(&collection_bytes).as_hex());
    fs::write(
        run_path(repo.path(), run_id).expect("run path"),
        serde_json::to_vec(&run_value).expect("run JSON"),
    )
    .expect("substituted run");
    let replay =
        verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
            .expect("rehashed checker replay assessment");
    assert_eq!(replay.decision.verdict, CampaignOutcome::Reject);
}

/// Trace: FR-114-AC-1, FR-114-AC-2, FR-114-AC-3, FR-114-AC-4
/// Provenance: PLAT-1043
#[cfg(target_os = "linux")]
#[test]
fn tc_1942_direct_process_and_checker_publish_protected_collection() {
    use engineering_assurance::campaign::{CampaignVerdict, canonical_digest};
    use quoin_measurement::campaign::run::{BindingFailure, CampaignRunError, run_campaign};
    use quoin_measurement::campaign::store::digest_path;

    let (repo, _producer_repo, definition, checkouts, runs) = direct_fixture();
    let mut invalid_runs = runs.clone();
    invalid_runs
        .get_mut("one")
        .expect("fixture member")
        .toolchains
        .clear();
    // TC-1942: a configuration collection intake would refuse must fail
    // before the producer executes or any run is published.
    let invalid_run_id = "fixture-empty-toolchains";
    assert!(matches!(
        run_campaign(
            repo.path(),
            &definition,
            invalid_run_id,
            &checkouts,
            &invalid_runs,
        ),
        Err(CampaignRunError::Binding(BindingFailure::InvalidToolchains { member }))
            if member == "one"
    ));
    assert!(
        !run_path(repo.path(), invalid_run_id)
            .expect("safe invalid run path")
            .exists()
    );
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

/// Trace: FR-114-AC-3, FR-114-AC-4
/// Provenance: PLAT-1043
/// A coherent new hash chain cannot substitute source-authored argv. The
/// retained request is reconstructed from the exact Git procedure on replay.
#[cfg(target_os = "linux")]
#[test]
fn tc_1942_rehashed_producer_arguments_do_not_replay() {
    use engineering_assurance::campaign::{CampaignDefinition, canonical_digest};
    use quoin_measurement::campaign::run::run_campaign;
    use quoin_measurement::campaign::store::digest_path;

    let (repo, _producer_repo, mut definition, checkouts, runs) = direct_fixture();
    definition.members[0].checker_procedure = None;
    let definition: CampaignDefinition = definition;
    let run_id = "fixture-rehashed-argv";
    let run = run_campaign(repo.path(), &definition, run_id, &checkouts, &runs)
        .expect("direct bounded producer");
    let first = &run.attempts.as_ref().expect("attempt inventory")[0];
    let old_request_digest = first.request_digest.as_deref().expect("request digest");
    let old_result_digest = first.result_digest.as_deref().expect("result digest");
    let request_path =
        digest_path(repo.path(), "requests", old_request_digest, "json").expect("request path");
    let result_path =
        digest_path(repo.path(), "results", old_result_digest, "json").expect("result path");
    let mut request: serde_json::Value =
        serde_json::from_slice(&fs::read(request_path).expect("request bytes"))
            .expect("request JSON");
    request["arguments"][0]["value"] = json!("campaign/other-producer.py");
    let (new_request_digest, _) =
        quoin_measurement::campaign::store::retain_value(repo.path(), "requests", &request)
            .expect("rehash substituted request");
    assert_ne!(new_request_digest, old_request_digest);

    let mut result: serde_json::Value =
        serde_json::from_slice(&fs::read(result_path).expect("result bytes")).expect("result JSON");
    result["requestIdentity"]["digest"] = json!(new_request_digest);
    let (new_result_digest, _) =
        quoin_measurement::campaign::store::retain_value(repo.path(), "results", &result)
            .expect("rehash dependent result");

    let collection_id = quoin_measurement::types::ids::CollectionId::parse(
        first.collection_id.as_deref().expect("collection ID"),
    )
    .expect("valid collection ID");
    let collection_path = quoin_measurement::store::measurement_path(repo.path(), &collection_id);
    let mut collection: serde_json::Value =
        serde_json::from_slice(&fs::read(&collection_path).expect("collection bytes"))
            .expect("collection JSON");
    collection["rawEvidence"]["requestDigest"] = json!(new_request_digest);
    collection["rawEvidence"]["resultDigest"] = json!(new_result_digest);
    let artifacts = collection["verificationStack"]["artifacts"]
        .as_object_mut()
        .expect("attestation artifacts");
    let old_result_key = format!("spec/evidence/campaigns/results/{old_result_digest}.json");
    let new_result_key = format!("spec/evidence/campaigns/results/{new_result_digest}.json");
    assert!(artifacts.remove(&old_result_key).is_some());
    artifacts.insert(new_result_key, json!(format!("sha256:{new_result_digest}")));
    let collection_bytes = serde_json::to_vec(&collection).expect("rehashed collection JSON");
    let new_collection_digest = digest_bytes_sha256(&collection_bytes).as_hex().to_owned();
    fs::write(collection_path, collection_bytes).expect("substituted collection");

    let run_file = run_path(repo.path(), run_id).expect("run path");
    let mut run_value: serde_json::Value =
        serde_json::from_slice(&fs::read(&run_file).expect("run bytes")).expect("run JSON");
    run_value["attempts"][0]["requestDigest"] = json!(new_request_digest);
    run_value["attempts"][0]["resultDigest"] = json!(new_result_digest);
    run_value["attempts"][0]["collectionDigest"] = json!(new_collection_digest);
    fs::write(
        run_file,
        serde_json::to_vec(&run_value).expect("substituted run JSON"),
    )
    .expect("substituted run");

    let definition_digest = canonical_digest(&definition).expect("definition digest");
    let replay =
        verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
            .expect("replay returns an assessment");
    assert_eq!(
        replay.decision.verdict,
        CampaignOutcome::Reject,
        "{replay:?}"
    );
}

/// Trace: FR-114-AC-2
/// Provenance: PLAT-1043
/// A source-bound restart reuses completed, write-once attempts under the same
/// run ID. It never reruns or replaces those Collections when a later member
/// failed before its own attempt could be retained.
#[cfg(target_os = "linux")]
#[test]
fn tc_1942_resume_keeps_prior_attempts_and_completes_the_missing_suffix() {
    use engineering_assurance::campaign::{CampaignVerdict, canonical_digest};
    use quoin_measurement::campaign::run::{InputSource, SelectedInput, run_campaign};

    let (repo, _producer_repo, mut definition, checkouts, mut runs) = direct_fixture();
    let mut second = definition.members[0].clone();
    second.name = "two".to_owned();
    second.depends_on = Some(vec!["one".to_owned()]);
    definition.members.push(second);
    let input_dir = tempfile::tempdir().expect("external selected-input directory");
    let input_path = input_dir.path().join("later.bin");
    let mut second_runtime = runs.get("one").expect("first runtime").clone();
    second_runtime.inputs.push(SelectedInput {
        role: "extra".to_owned(),
        path: "selected/later.bin".to_owned(),
        executable: false,
        source: InputSource::File(input_path.clone()),
    });
    runs.insert("two".to_owned(), second_runtime);
    let run_id = "fixture-resume-immutable";
    assert!(run_campaign(repo.path(), &definition, run_id, &checkouts, &runs).is_err());
    assert!(!run_path(repo.path(), run_id).expect("run path").exists());
    let checkpoint_root =
        quoin_measurement::campaign::store::campaigns_root(repo.path()).join("checkpoints");
    let mut original: Vec<(std::path::PathBuf, Vec<u8>)> = fs::read_dir(&checkpoint_root)
        .expect("retained checkpoint directory")
        .map(|entry| {
            let path = entry.expect("checkpoint entry").path();
            let bytes = fs::read(&path).expect("checkpoint bytes");
            (path, bytes)
        })
        .collect();
    original.sort_by_key(|(_, bytes)| {
        serde_json::from_slice::<serde_json::Value>(bytes).expect("checkpoint JSON")["ordinal"]
            .as_u64()
            .expect("checkpoint ordinal")
    });
    assert_eq!(
        original.len(),
        2,
        "two completed producer attempts retained"
    );
    let prior_attempts: Vec<serde_json::Value> = original
        .iter()
        .map(|(_, bytes)| {
            serde_json::from_slice::<serde_json::Value>(bytes).expect("checkpoint JSON")["attempt"]
                .clone()
        })
        .collect();

    fs::write(&input_path, b"selected bytes").expect("make later input available");
    let run = run_campaign(repo.path(), &definition, run_id, &checkouts, &runs)
        .expect("resume missing member without repeating prior attempts");
    assert_eq!(run.verdict, CampaignVerdict::Inconclusive);
    let attempts = run.attempts.as_ref().expect("complete attempt inventory");
    assert_eq!(attempts.len(), 4);
    for (checkpoint, prior) in original.iter().zip(prior_attempts.iter()) {
        assert_eq!(
            fs::read(&checkpoint.0).expect("checkpoint after resume"),
            checkpoint.1
        );
        let retained: serde_json::Value =
            serde_json::from_slice(&checkpoint.1).expect("checkpoint JSON");
        assert_eq!(retained["attempt"], *prior);
    }
    let resumed_first: Vec<_> = attempts
        .iter()
        .filter(|attempt| attempt.member == "one")
        .map(|attempt| serde_json::to_value(attempt).expect("attempt JSON"))
        .collect();
    assert_eq!(resumed_first, prior_attempts);
    let definition_digest = canonical_digest(&definition).expect("definition digest");
    let replay =
        verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
            .expect("independent resumed-run replay");
    assert_eq!(replay.decision.verdict, CampaignOutcome::Inconclusive);

    let published =
        fs::read(run_path(repo.path(), run_id).expect("run path")).expect("published run bytes");
    assert!(run_campaign(repo.path(), &definition, run_id, &checkouts, &runs).is_err());
    assert_eq!(
        fs::read(run_path(repo.path(), run_id).expect("run path")).expect("run after repeat"),
        published,
        "a published run is never replaced"
    );
}

/// Trace: FR-114-AC-2, TC-1942. Provenance: PLAT-1043.
/// A same-run-ID checkpoint claiming InvalidRequest without an EA result has
/// no replayable preflight authority and cannot suppress that invocation.
#[cfg(target_os = "linux")]
#[test]
fn tc_1942_resume_refuses_planted_evidence_free_preflight_checkpoint() {
    use quoin_measurement::campaign::run::{BindingFailure, CampaignRunError, run_campaign};

    let (repo, _producer_repo, definition, checkouts, runs) = direct_fixture();
    let run_id = "fixture-forged-preflight-checkpoint";
    run_campaign(repo.path(), &definition, run_id, &checkouts, &runs).expect("initial direct run");
    let run_file = run_path(repo.path(), run_id).expect("run path");
    fs::remove_file(&run_file).expect("model interrupted final publication");
    let checkpoint_root =
        quoin_measurement::campaign::store::campaigns_root(repo.path()).join("checkpoints");
    let first_path = fs::read_dir(checkpoint_root)
        .expect("checkpoint directory")
        .map(|entry| entry.expect("checkpoint entry").path())
        .find(|path| {
            let record: serde_json::Value =
                serde_json::from_slice(&fs::read(path).expect("checkpoint bytes"))
                    .expect("checkpoint JSON");
            record["ordinal"] == 1
        })
        .expect("first checkpoint");
    let mut checkpoint: serde_json::Value =
        serde_json::from_slice(&fs::read(&first_path).expect("checkpoint bytes"))
            .expect("checkpoint JSON");
    checkpoint["attempt"] = json!({
        "member":"one", "index":1, "status":"invalid_request", "reason":"planted_preflight"
    });
    let planted = serde_json::to_vec(&checkpoint).expect("planted checkpoint JSON");
    fs::write(&first_path, &planted).expect("plant evidence-free checkpoint");

    assert!(matches!(
        run_campaign(repo.path(), &definition, run_id, &checkouts, &runs),
        Err(CampaignRunError::Binding(BindingFailure::IdentityMismatch(
            _
        )))
    ));
    assert!(!run_file.exists(), "no final run may be published");
    assert_eq!(fs::read(first_path).expect("checkpoint remains"), planted);
}

/// Trace: FR-114-AC-2
/// Provenance: PLAT-1043
/// A crash after publishing a Collection but before its checkpoint cannot
/// safely rerun that invocation. The existing bytes must win the collision.
#[cfg(target_os = "linux")]
#[test]
fn tc_1942_orphan_collection_collision_refuses_without_overwrite() {
    use quoin_measurement::campaign::run::run_campaign;

    let (repo, _producer_repo, definition, checkouts, mut runs) = direct_fixture();
    let run_id = "fixture-orphan-collision";
    let run = run_campaign(repo.path(), &definition, run_id, &checkouts, &runs)
        .expect("initial direct run");
    let collection_id = run.attempts.as_ref().expect("attempts")[0]
        .collection_id
        .as_ref()
        .expect("collection ID");
    let collection_path = quoin_measurement::store::measurement_path(
        repo.path(),
        &quoin_measurement::types::ids::CollectionId::parse(collection_id)
            .expect("valid collection ID"),
    );
    let original = fs::read(&collection_path).expect("retained Collection");

    // Model a crash window with an orphan Collection: only the fixture's
    // final-run/checkpoint metadata is removed, never the Collection itself.
    fs::remove_file(run_path(repo.path(), run_id).expect("run path"))
        .expect("remove fixture run metadata");
    let checkpoint_root =
        quoin_measurement::campaign::store::campaigns_root(repo.path()).join("checkpoints");
    for entry in fs::read_dir(checkpoint_root).expect("checkpoint directory") {
        fs::remove_file(entry.expect("checkpoint entry").path())
            .expect("remove fixture checkpoint metadata");
    }
    runs.get_mut("one").expect("member binding").timestamp = "2026-09-24T00:00:00Z".to_owned();
    assert!(run_campaign(repo.path(), &definition, run_id, &checkouts, &runs).is_err());
    assert_eq!(
        fs::read(&collection_path).expect("Collection after refusal"),
        original
    );
    assert!(!run_path(repo.path(), run_id).expect("run path").exists());
}

/// Trace: FR-114-AC-2
/// Provenance: PLAT-1043
/// A missing executable is a retained EA preflight outcome, with only the
/// identities EA actually minted and no fabricated Collection.
#[cfg(target_os = "linux")]
#[test]
fn tc_1941_unavailable_producer_is_retained_and_replayed() {
    use engineering_assurance::campaign::{
        CampaignAttemptStatus, CampaignVerdict, canonical_digest,
    };
    use engineering_assurance::producer_execution::ContentDigest;
    use quoin_measurement::campaign::run::run_campaign;

    let (repo, _producer_repo, definition, checkouts, mut runs) = direct_fixture();
    let producer = &mut runs
        .get_mut("one")
        .expect("member binding")
        .producer
        .producer;
    producer.executable = "/definitely-not-installed/quoin-fixture-producer".to_owned();
    producer.executable_digest = ContentDigest::of_bytes(b"absent executable");
    let run_id = "fixture-unavailable-producer";
    let run = run_campaign(repo.path(), &definition, run_id, &checkouts, &runs)
        .expect("typed unavailable result retained");
    assert_eq!(run.verdict, CampaignVerdict::Inconclusive);
    for attempt in run.attempts.as_ref().expect("attempts") {
        assert_eq!(
            attempt.status,
            CampaignAttemptStatus::Unavailable,
            "{attempt:?}"
        );
        assert!(attempt.request_digest.is_some());
        assert!(attempt.result_digest.is_some());
        assert!(attempt.collection_id.is_none());
    }
    let definition_digest = canonical_digest(&definition).expect("definition identity");
    let replay =
        verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
            .expect("independent terminal replay");
    assert_eq!(replay.decision.verdict, CampaignOutcome::Inconclusive);

    // Rehash the first EA result and update the run's claimed digest. Its
    // changed terminal state still contradicts the retained attempt status.
    let original_result = run.attempts.as_ref().expect("attempts")[0]
        .result_digest
        .as_deref()
        .expect("result identity");
    let result_path = quoin_measurement::campaign::store::digest_path(
        repo.path(),
        "results",
        original_result,
        "json",
    )
    .expect("result path");
    let mut result: serde_json::Value =
        serde_json::from_slice(&fs::read(result_path).expect("EA result")).expect("EA result JSON");
    result["state"]["kind"] = json!("timed_out");
    let (new_result, _) =
        quoin_measurement::campaign::store::retain_value(repo.path(), "results", &result)
            .expect("rehash terminal result");
    let run_file = run_path(repo.path(), run_id).expect("run path");
    let mut altered_run: serde_json::Value =
        serde_json::from_slice(&fs::read(&run_file).expect("run bytes")).expect("run JSON");
    altered_run["attempts"][0]["resultDigest"] = json!(new_result);
    fs::write(
        &run_file,
        serde_json::to_vec(&altered_run).expect("run JSON"),
    )
    .expect("substituted run");
    let rejected =
        verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
            .expect("well-formed substituted terminal attempt");
    assert_eq!(rejected.decision.verdict, CampaignOutcome::Reject);
}

/// Trace: FR-114-AC-2
/// Provenance: PLAT-1043
/// An EA request rejected before identity creation records a typed attempt
/// without inventing request, result or Collection identities.
#[cfg(target_os = "linux")]
#[test]
fn tc_1941_invalid_request_has_no_minted_identity() {
    use engineering_assurance::campaign::{
        CampaignAttemptStatus, CampaignVerdict, canonical_digest,
    };
    use quoin_measurement::campaign::run::run_campaign;

    let (repo, _producer_repo, definition, checkouts, mut runs) = direct_fixture();
    runs.get_mut("one")
        .expect("member binding")
        .producer
        .producer
        .source_revision = "unlisted-revision".to_owned();
    let run_id = "fixture-invalid-request";
    let run = run_campaign(repo.path(), &definition, run_id, &checkouts, &runs)
        .expect("typed invalid-request attempts retained");
    assert_eq!(run.verdict, CampaignVerdict::Inconclusive);
    for attempt in run.attempts.as_ref().expect("attempts") {
        assert_eq!(attempt.status, CampaignAttemptStatus::InvalidRequest);
        assert_eq!(attempt.reason.as_deref(), Some("invalid_request"));
        assert!(attempt.request_digest.is_none());
        assert!(attempt.result_digest.is_none());
        assert!(attempt.collection_id.is_none());
    }
    let definition_digest = canonical_digest(&definition).expect("definition identity");
    let replay =
        verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
            .expect("independent invalid-request replay");
    assert_eq!(replay.decision.verdict, CampaignOutcome::Inconclusive);
}

/// Trace: FR-114-AC-2
/// Provenance: PLAT-1043, TC-1941
/// A selected executable whose actual bytes differ from the request's digest
/// receives EA's identity-bearing typed prelaunch refusal.
#[cfg(target_os = "linux")]
#[test]
fn tc_1941_executable_identity_refusal_is_retained() {
    use engineering_assurance::campaign::{CampaignAttemptStatus, canonical_digest};
    use engineering_assurance::producer_execution::ContentDigest;
    use quoin_measurement::campaign::run::run_campaign;

    let (repo, _producer_repo, definition, checkouts, mut runs) = direct_fixture();
    runs.get_mut("one")
        .expect("member binding")
        .producer
        .producer
        .executable_digest = ContentDigest::of_bytes(b"wrong executable bytes");
    let run_id = "fixture-executable-refused";
    let run = run_campaign(repo.path(), &definition, run_id, &checkouts, &runs)
        .expect("typed EA refusal retained");
    for attempt in run.attempts.as_ref().expect("attempts") {
        assert_eq!(
            attempt.status,
            CampaignAttemptStatus::Refused,
            "{attempt:?}"
        );
        assert!(attempt.request_digest.is_some());
        assert!(attempt.result_digest.is_some());
        assert!(attempt.collection_id.is_none());
    }
    let definition_digest = canonical_digest(&definition).expect("definition identity");
    let replay =
        verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
            .expect("independent refusal replay");
    assert_eq!(replay.decision.verdict, CampaignOutcome::Inconclusive);
}

/// Trace: FR-114-AC-2
/// Provenance: PLAT-1043, TC-1941
/// The native producer exceeds its bound stdout budget and EA records the
/// closed failure category without manufacturing a Collection.
#[cfg(target_os = "linux")]
#[test]
fn tc_1941_bounded_stdout_failure_is_retained() {
    use engineering_assurance::campaign::{CampaignAttemptStatus, canonical_digest};
    use quoin_measurement::campaign::run::run_campaign;

    let (repo, _producer_repo, definition, checkouts, mut runs) = direct_fixture();
    runs.get_mut("one")
        .expect("member binding")
        .producer
        .budget
        .max_stdout_bytes = 1;
    let run_id = "fixture-stdout-bound";
    let run = run_campaign(repo.path(), &definition, run_id, &checkouts, &runs)
        .expect("typed EA output-bound failure retained");
    for attempt in run.attempts.as_ref().expect("attempts") {
        assert_eq!(attempt.status, CampaignAttemptStatus::Failed, "{attempt:?}");
        assert!(attempt.request_digest.is_some());
        assert!(attempt.result_digest.is_some());
        assert!(attempt.collection_id.is_none());
    }
    let definition_digest = canonical_digest(&definition).expect("definition identity");
    let replay =
        verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
            .expect("independent failure replay");
    assert_eq!(replay.decision.verdict, CampaignOutcome::Inconclusive);
}

/// Trace: FR-114-AC-2
/// Provenance: PLAT-1043, TC-1941
/// A source-authored short deadline and a sleeping direct producer exercise
/// the EA timeout path with minted request/result identities.
#[cfg(target_os = "linux")]
#[test]
fn tc_1941_timed_out_producer_is_retained() {
    use engineering_assurance::campaign::{CampaignAttemptStatus, canonical_digest};
    use quoin_measurement::campaign::run::run_campaign;

    let (repo, producer_repo, mut definition, checkouts, mut runs) = direct_fixture();
    let procedure_path = repo.path().join("campaign/procedure.json");
    let mut procedure: serde_json::Value =
        serde_json::from_slice(&fs::read(&procedure_path).expect("procedure bytes"))
            .expect("procedure JSON");
    procedure["timeoutMillis"] = json!(20);
    fs::write(
        &procedure_path,
        serde_json::to_vec(&procedure).expect("procedure JSON"),
    )
    .expect("bounded procedure");
    git(repo.path(), &["add", "campaign/procedure.json"]);
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
            "short deadline",
        ],
    );
    fs::write(
        producer_repo.path().join("campaign/producer.py"),
        "import time\ntime.sleep(2)\nprint('ok')\n",
    )
    .expect("sleeping producer");
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
            "sleeping producer",
        ],
    );
    for (source, checkout) in definition
        .source_graph
        .iter_mut()
        .zip([repo.path(), producer_repo.path()])
    {
        source.revision = String::from_utf8(git(checkout, &["rev-parse", "HEAD"]))
            .expect("revision")
            .trim()
            .to_owned();
        source.digest = digest_bytes_sha256(&git(
            checkout,
            &["ls-tree", "-r", "-z", "--full-tree", &source.revision],
        ))
        .as_hex()
        .to_owned();
    }
    let runtime = runs.get_mut("one").expect("member binding");
    runtime.producer.producer.source_revision = definition.source_graph[1].revision.clone();
    runtime
        .checker
        .as_mut()
        .expect("checker binding")
        .producer
        .source_revision = definition.source_graph[0].revision.clone();
    let run_id = "fixture-producer-timeout";
    let run = run_campaign(repo.path(), &definition, run_id, &checkouts, &runs)
        .expect("typed EA timeout retained");
    for attempt in run.attempts.as_ref().expect("attempts") {
        assert_eq!(
            attempt.status,
            CampaignAttemptStatus::TimedOut,
            "{attempt:?}"
        );
        assert!(attempt.request_digest.is_some());
        assert!(attempt.result_digest.is_some());
        assert!(attempt.collection_id.is_none());
    }
    let definition_digest = canonical_digest(&definition).expect("definition identity");
    let replay =
        verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
            .expect("independent timeout replay");
    assert_eq!(replay.decision.verdict, CampaignOutcome::Inconclusive);
}

/// Trace: FR-114-AC-2
/// Provenance: PLAT-1043, TC-1941
/// A caller cancels the currently registered EA Event-bound direct process;
/// both the terminal outcome and minted request/result identities replay.
#[cfg(target_os = "linux")]
#[test]
fn tc_1941_live_event_cancellation_is_retained() {
    use std::sync::Arc;
    use std::time::Duration;

    use engineering_assurance::campaign::{CampaignAttemptStatus, canonical_digest};
    use engineering_assurance::producer_execution::CancellationBinding;
    use quoin_measurement::campaign::run::{CampaignCancellation, run_campaign_with_cancellation};

    let (repo, producer_repo, mut definition, checkouts, mut runs) = direct_fixture();
    fs::write(
        producer_repo.path().join("campaign/producer.py"),
        "import time\ntime.sleep(2)\nprint('ok')\n",
    )
    .expect("sleeping producer");
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
            "cancellable producer",
        ],
    );
    let source = &mut definition.source_graph[1];
    source.revision = String::from_utf8(git(producer_repo.path(), &["rev-parse", "HEAD"]))
        .expect("producer revision")
        .trim()
        .to_owned();
    source.digest = digest_bytes_sha256(&git(
        producer_repo.path(),
        &["ls-tree", "-r", "-z", "--full-tree", &source.revision],
    ))
    .as_hex()
    .to_owned();
    let producer = &mut runs.get_mut("one").expect("member binding").producer;
    producer.producer.source_revision = source.revision.clone();
    producer.cancellation = CancellationBinding::Event {
        authority: "fictional-campaign".to_owned(),
        event_id: "cancel-current-run".to_owned(),
    };

    let control = Arc::new(CampaignCancellation::new());
    let cancelling = Arc::clone(&control);
    let canceller = std::thread::spawn(move || {
        assert!(
            cancelling.wait_for_active(Duration::from_secs(5)),
            "EA request registered"
        );
        assert!(
            cancelling.cancel(),
            "Event-bound active token accepted cancellation"
        );
    });
    let run_id = "fixture-live-cancellation";
    let run = run_campaign_with_cancellation(
        repo.path(),
        &definition,
        run_id,
        &checkouts,
        &runs,
        &control,
    )
    .expect("cancelled attempts retained");
    canceller.join().expect("canceller completed");
    for attempt in run.attempts.as_ref().expect("attempts") {
        assert_eq!(
            attempt.status,
            CampaignAttemptStatus::Cancelled,
            "{attempt:?}"
        );
        assert!(attempt.request_digest.is_some());
        assert!(attempt.result_digest.is_some());
        assert!(attempt.collection_id.is_none());
    }
    let definition_digest = canonical_digest(&definition).expect("definition identity");
    let replay =
        verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
            .expect("independent cancellation replay");
    assert_eq!(replay.decision.verdict, CampaignOutcome::Inconclusive);
}

/// Trace: FR-114-AC-2. Provenance: PLAT-1043, TC-1941.
/// Disabled request bindings cannot be labelled as cancelled by a caller.
#[cfg(target_os = "linux")]
#[test]
fn tc_1941_disabled_cancellation_never_invents_cancelled_state() {
    use engineering_assurance::campaign::CampaignVerdict;
    use quoin_measurement::campaign::run::{CampaignCancellation, run_campaign_with_cancellation};

    let (repo, _producer_repo, definition, checkouts, runs) = direct_fixture();
    let control = CampaignCancellation::new();
    assert!(!control.cancel(), "no Event-bound token is active");
    let run = run_campaign_with_cancellation(
        repo.path(),
        &definition,
        "fixture-disabled-cancellation",
        &checkouts,
        &runs,
        &control,
    )
    .expect("Disabled binding runs normally");
    assert_eq!(run.verdict, CampaignVerdict::Accepted);
}

/// Trace: FR-114-AC-3
/// Provenance: PLAT-1043, TC-1944
/// Removing a retained receipt downgrades an originally accepted run to
/// inconclusive; changing retained bytes is a contradictory reject.
#[cfg(target_os = "linux")]
#[test]
fn tc_1944_missing_plan_or_domain_receipt_is_inconclusive() {
    use engineering_assurance::campaign::{CampaignVerdict, canonical_digest};
    use quoin_measurement::campaign::run::run_campaign;
    use quoin_measurement::campaign::store::digest_path;

    let (repo, _producer_repo, definition, checkouts, runs) = direct_fixture();
    let run_id = "fixture-missing-receipts";
    let run = run_campaign(repo.path(), &definition, run_id, &checkouts, &runs)
        .expect("accepted direct fixture");
    assert_eq!(run.verdict, CampaignVerdict::Accepted);
    let attempt = &run.attempts.as_ref().expect("attempts")[0];
    let definition_digest = canonical_digest(&definition).expect("definition identity");
    for (kind, digest) in [
        (
            "verdicts",
            attempt.verdict_digest.as_deref().expect("plan verdict"),
        ),
        (
            "domain-verdicts",
            attempt
                .domain_verdict_digest
                .as_deref()
                .expect("domain verdict"),
        ),
    ] {
        let path = digest_path(repo.path(), kind, digest, "json").expect("receipt path");
        let bytes = fs::read(&path).expect("receipt bytes");
        fs::remove_file(&path).expect("remove fixture receipt");
        let missing =
            verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
                .expect("well-formed run with missing receipt");
        assert_eq!(
            missing.decision.verdict,
            CampaignOutcome::Inconclusive,
            "{kind}"
        );
        fs::write(&path, bytes).expect("restore fixture receipt");
    }

    let checker_result_digest = attempt
        .checker_result_digest
        .as_deref()
        .expect("checker result identity");
    let checker_result_path =
        digest_path(repo.path(), "results", checker_result_digest, "json").expect("result path");
    let checker_result: serde_json::Value =
        serde_json::from_slice(&fs::read(checker_result_path).expect("checker result"))
            .expect("checker result JSON");
    let raw_digest = checker_result["artifacts"][0]["digest"]
        .as_str()
        .expect("raw checker verdict digest");
    let raw_path = digest_path(repo.path(), "raw", raw_digest, "bin").expect("raw verdict path");
    let raw_bytes = fs::read(&raw_path).expect("raw checker verdict");
    fs::remove_file(&raw_path).expect("remove raw checker verdict");
    let missing =
        verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
            .expect("well-formed run with missing raw artifact");
    assert_eq!(missing.decision.verdict, CampaignOutcome::Inconclusive);
    fs::write(&raw_path, b"altered raw verdict").expect("alter raw checker verdict");
    let altered =
        verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
            .expect("well-formed run with altered raw artifact");
    assert_eq!(altered.decision.verdict, CampaignOutcome::Reject);
    fs::write(&raw_path, raw_bytes).expect("restore raw checker verdict");
    let restored =
        verify_retained_campaign(repo.path(), definition_digest.as_str(), run_id, &checkouts)
            .expect("restored raw artifact replay");
    assert_eq!(restored.decision.verdict, CampaignOutcome::Accept);
}
