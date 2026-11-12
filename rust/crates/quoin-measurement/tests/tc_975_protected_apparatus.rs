// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-975: a plan's protected apparatus is resolved (paths only) when a
//! collection is written, and a comparison, a ratchet verdict and the
//! verdict checker each refuse to measure across a change to the recorded
//! set (FR-110).
//!
//! Every repository here is a real temporary directory, every collection is
//! written through `write_measurement_collection` and read back through the
//! store, so what is compared is what intake recorded — never a value the
//! test built for the comparison to agree with.
//!
//! Provenance: PLAT-975

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};
use std::process::Command;

use engineering_assurance::measurement::{Comparator, DecisionRule, NegativeControlKind};
use quoin_jsonschema::SchemaValidator;
use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::report::{InconclusiveReason, RatchetOutcome, StageVerdict};
use quoin_measurement::source::{DiskMeasurement, MemoryMeasurement};
use quoin_measurement::store::{measurement_path, read_measurement_collections};
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::comparison::{ComparisonReasonCode, ComparisonStatus};
use quoin_measurement::types::ids::CollectionId;
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_measurement::{
    MeasurementError, OrderSource, Ranked, Reason, TamperFacts, Verdict, build_measurement_report,
    compare_measurement_collections, stored_measurement_collection, verify,
    write_measurement_collection,
};
use serde_json::{Value, json};

// ------------------------------------------------------------- fixtures

/// A ratchet on a pass rate that protects an answer key and a labels
/// directory. `extra` is spliced into the frontmatter.
fn plan_document(extra: &str) -> String {
    plan_with("ratchet", "baseline: best-seen", extra)
}

/// A plan at `stage` whose decision rule is `ge` against `reference`.
fn plan_with(stage: &str, reference: &str, extra: &str) -> String {
    format!(
        "---\n\
         id: MP-975\n\
         title: Protected answer key\n\
         type: MeasurementPlan\n\
         status: active\n\
         stage: {stage}\n\
         metric: gate.pass_rate\n\
         definition_version: gate.pass-rate-v1\n\
         objective:\n  direction: higher\n\
         statistical_design:\n{DESIGN}  decision_rule:\n    comparator: ge\n    {reference}\n\
         {extra}\
         ---\n\n# Protected answer key\n"
    )
}

/// The statistical-design members engineering-assurance's schema requires
/// besides the decision rule.
const DESIGN: &str = "  population: fixture cases\n  sampling: every case once\n  repetitions: 1\n  \
                      estimator: proportion\n  population_exempt: fixture cases carry no ids\n  error_model: a case is misjudged\n  \
                      uncertainty: none estimated\n";

const PROTECTED: &str = "protected_apparatus:\n  - harness/answers.json\n  - labels/**\n";

/// A repository holding the plan document and the apparatus it protects:
/// the answer key, two label files (one a dotfile, one nested), and an
/// unprotected tool binary.
fn repository(extra: &str) -> tempfile::TempDir {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let root = temporary.path();
    write(root, "spec/assurance/MP-975.md", &plan_document(extra));
    write(root, "harness/answers.json", "{\"answers\":[1,2,3]}");
    write(root, "labels/a.json", "[\"a\"]");
    write(root, "labels/nested/.hidden", "dotfile");
    write(root, "dist/tool", "tool v1");
    temporary
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
}

/// The named files, plus the `config` label every fixture collection
/// carries.
fn artifacts(_root: &Path, names: &[&str]) -> Value {
    let mut list = vec![json!("config")];
    list.extend(names.iter().map(|name| json!(name)));
    Value::Array(list)
}

/// Every file the plan protects, plus the unprotected tool.
const ALL_FILES: [&str; 4] = [
    "harness/answers.json",
    "labels/a.json",
    "labels/nested/.hidden",
    "dist/tool",
];

/// A new collection of `matched` of 10 passing, at minute `minute`.
fn candidate(id: &str, minute: u8, matched: u32, artifacts: &Value) -> Value {
    json!({
        "schemaVersion": 2,
        "collectionId": id,
        "subject": "fixture",
        "scope": { "cases": 10 },
        "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1",
        "timestamp": format!("2026-09-22T00:{minute:02}:00.000Z"),
        "environment": { "runner": "test" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "release",
            "capabilities": ["fixture.capability"],
            "artifacts": artifacts,
        },
        "observations": [{
            "metric": "gate.pass_rate",
            "planId": "MP-975",
            "definitionVersion": "gate.pass-rate-v1",
            "state": "measured",
            "value": f64::from(matched) / 10.0,
            "unit": "fraction",
            "shape": "ratio",
            "population": { "examined": 10, "matched": matched, "complete": true },
        }],
        "rawEvidence": { "payload": [] },
    })
}

fn publish(root: &Path, value: &Value) -> Result<PathBuf, MeasurementError> {
    write_measurement_collection(root, &from_serde(value).unwrap())
}

/// Publish a run whose artifact list names every file.
fn publish_run(root: &Path, id: &str, minute: u8, matched: u32) {
    publish(
        root,
        &candidate(id, minute, matched, &artifacts(root, &ALL_FILES)),
    )
    .unwrap_or_else(|error| panic!("{id} is admitted: {error}"));
}

/// Publish a run whose artifact list also names the new file `labels/b.json`,
/// which this adds under the protected `labels/` directory entry.
fn publish_run_with_extra_label(root: &Path, id: &str, minute: u8, matched: u32) {
    write(root, "labels/b.json", "[\"b\"]");
    let mut names = ALL_FILES.to_vec();
    names.push("labels/b.json");
    publish(
        root,
        &candidate(id, minute, matched, &artifacts(root, &names)),
    )
    .unwrap_or_else(|error| panic!("{id} is admitted: {error}"));
}

fn stored_bytes(root: &Path, id: &str) -> Value {
    let path = measurement_path(root, &CollectionId::parse(id).unwrap());
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn stored(root: &Path) -> Vec<MeasurementCollection> {
    read_measurement_collections(&DiskMeasurement::new(root)).unwrap()
}

fn stored_count(root: &Path) -> usize {
    std::fs::read_dir(root.join("spec/evidence/measurements")).map_or(0, Iterator::count)
}

fn plan_of(root: &Path) -> MeasurementPlan {
    load_measurement_plans(&DiskMeasurement::new(root), PlanLoadOptions::default())
        .unwrap()
        .remove(0)
}

fn reasons(rows: &[quoin_measurement::MeasurementComparison]) -> Vec<ComparisonReasonCode> {
    rows[0].reasons.iter().map(|reason| reason.code).collect()
}

fn checked(plan: &MeasurementPlan, collections: &[MeasurementCollection]) -> Vec<Reason> {
    let ranked: Vec<Ranked<'_>> = (0_u64..)
        .zip(collections)
        .map(|(intake, collection)| Ranked::new(collection, Some(intake)))
        .collect();
    verify(
        plan,
        &ranked,
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    )
    .reasons
}

#[test]
#[trace("TC-1810", "FR-110-AC-1")]
fn tc_975_022_analyze_shaped_gate_matches_locked_ea_schema_and_loads() {
    let fixture = include_str!("fixtures/MP-001-evidence-gate.md");
    let frontmatter = fixture
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map(|(frontmatter, _)| frontmatter)
        .expect("the fixture has delimited frontmatter");
    let authored = quoin_yaml::from_str(frontmatter).expect("the fixture frontmatter is YAML");

    let metadata = Command::new("cargo")
        .args(["metadata", "--locked", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Cargo reports the locked dependency graph");
    assert!(
        metadata.status.success(),
        "{}",
        String::from_utf8_lossy(&metadata.stderr)
    );
    let metadata: Value = serde_json::from_slice(&metadata.stdout).expect("Cargo metadata is JSON");
    let ea = metadata["packages"]
        .as_array()
        .expect("Cargo metadata lists packages")
        .iter()
        .find(|package| package["name"] == "engineering-assurance")
        .expect("the graph carries engineering-assurance");
    let ea_manifest = PathBuf::from(
        ea["manifest_path"]
            .as_str()
            .expect("the EA package has a manifest path"),
    );
    let schema_path = ea_manifest
        .parent()
        .expect("the EA manifest has a parent")
        .join("engineering_assurance/schemas/measurement-plan-frontmatter.schema.json");
    let schema: Value = serde_json::from_slice(
        &std::fs::read(&schema_path).expect("the locked EA package carries its schema"),
    )
    .expect("the locked EA schema is JSON");
    let validator = SchemaValidator::compile_vendored(&schema_path, &schema, &[])
        .expect("the locked EA schema compiles");
    assert!(
        validator.is_valid(&authored),
        "EA schema errors: {:?}",
        validator.errors(&authored)
    );

    let root = tempfile::tempdir().expect("the fixture repository is creatable");
    write(
        root.path(),
        "spec/assurance/MP-001-analyze-measurements.md",
        fixture,
    );
    let plan = plan_of(root.path());
    assert_eq!(plan.id.as_str(), "MP-001");
    assert!(plan.protected_apparatus.is_none());
    assert!(plan.execution_procedure.is_none());
    let missing = verify(
        &plan,
        &[],
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    );
    assert_eq!(missing.verdict, Verdict::Inconclusive);
    assert_eq!(missing.reasons, [Reason::NoCollections]);
}

#[trace("FR-108-AC-4", "FR-110-AC-1", "TC-1810")]
/// Provenance: PLAT-1162
#[test]
fn tc_975_023_active_evidence_gate_uses_retained_conclusive_results() {
    let extra = concat!(
        "ground_truth_kind: mechanical\n",
        "negative_controls:\n  - kind: stale-evidence\n    description: stale evidence refuses a pass\n",
    );
    for (id, matched, complete, expected) in [
        ("gate-pass", 10, true, Verdict::Accept),
        ("gate-fail", 5, true, Verdict::Reject),
        ("gate-incomplete", 10, false, Verdict::Inconclusive),
    ] {
        let root = tempfile::tempdir().expect("the run repository is creatable");
        write(
            root.path(),
            "spec/assurance/MP-975.md",
            &plan_with("gate", "threshold: 0.8", extra),
        );
        let plan = plan_of(root.path());
        let mut collection = candidate(id, 0, matched, &json!(["config"]));
        collection["observations"][0]["population"]["complete"] = json!(complete);
        publish(root.path(), &collection).expect("the retained evidence is admitted");
        let collections = stored(root.path());
        let verdict = verify(
            &plan,
            &[Ranked::new(&collections[0], Some(0))],
            TamperFacts::default(),
            OrderSource::CallerSupplied,
            None,
        );
        assert_eq!(verdict.verdict, expected, "{id}: {verdict:?}");
    }
}

fn verdict_of(plan: &MeasurementPlan, collections: &[MeasurementCollection]) -> Verdict {
    let ranked: Vec<Ranked<'_>> = (0_u64..)
        .zip(collections)
        .map(|(intake, collection)| Ranked::new(collection, Some(intake)))
        .collect();
    verify(
        plan,
        &ranked,
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    )
    .verdict
}

// ------------------------------------------------------------ plan intake

/// Provenance: PLAT-975
#[trace("FR-110-AC-1", "TC-1810")]
#[test]
fn tc_975_001_protected_apparatus_and_negative_controls_load_as_eas_types() {
    let temporary = repository(&format!(
        "{PROTECTED}negative_controls:\n  - kind: apparatus-edit\n    description: the answer key is protected\n"
    ));
    let plan = plan_of(temporary.path());
    let protected = plan.protected_apparatus.expect("the list loads");
    let entries: Vec<&str> = protected
        .iter()
        .map(engineering_assurance::measurement::ApparatusPath::as_str)
        .collect();
    assert_eq!(entries, ["harness/answers.json", "labels/**"]);
    assert!(
        plan.negative_controls
            .expect("the controls load")
            .covers(NegativeControlKind::ApparatusEdit)
    );

    let refused = |extra: &str| {
        let source = MemoryMeasurement::new()
            .with_document("spec/assurance/MP-975.md", plan_document(extra));
        load_measurement_plans(&source, PlanLoadOptions::default())
            .expect_err("the plan load is refused")
    };
    // A gate needs controls; apparatus is required when a control claims it.
    let gate = |extra: &str| {
        let source = MemoryMeasurement::new().with_document(
            "spec/assurance/MP-975.md",
            plan_with(
                "gate",
                "threshold: 0.8",
                &format!("ground_truth_kind: mechanical\n{extra}"),
            ),
        );
        load_measurement_plans(&source, PlanLoadOptions::default())
    };
    let controls =
        "negative_controls:\n  - kind: apparatus-edit\n    description: the key is digested\n";
    for (extra, member) in [
        (String::new(), "negative_controls"),
        (controls.to_owned(), "protected_apparatus"),
        (PROTECTED.to_owned(), "negative_controls"),
    ] {
        let error = gate(&extra).expect_err("a gate plan missing a list is refused");
        assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid, "{extra}");
        assert!(error.to_string().contains(member), "{extra}: {error}");
    }
    gate(&format!("{PROTECTED}{controls}")).expect("a gate plan stating both loads");
    gate("negative_controls:\n  - kind: stale-evidence\n    description: admitted evidence names the subject version\n")
        .expect("an independent evidence gate needs no authored apparatus list");

    for (extra, member) in [
        (
            "protected_apparatus:\n  - ../outside.json\n",
            "protected_apparatus",
        ),
        ("protected_apparatus: []\n", "protected_apparatus"),
        ("protected_apparatus:\n  - '**'\n", "protected_apparatus"),
        (
            "protected_apparatus:\n  - a.json\n  - a.json\n",
            "protected_apparatus",
        ),
        (
            "negative_controls:\n  - kind: wishful-thinking\n    description: x\n",
            "negative_controls",
        ),
        (
            "negative_controls:\n  - kind: apparatus-edit\n    description: guards nothing\n",
            "negative_controls",
        ),
    ] {
        let error = refused(extra);
        assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid, "{extra}");
        assert!(error.to_string().contains(member), "{extra}: {error}");
    }
}

#[trace("FR-110-AC-1", "TC-1810")]
/// Provenance: PLAT-1162
#[test]
fn tc_975_024_non_retired_gate_requires_typed_definition_fields() {
    let document = |truth: &str, design: &str| {
        format!(
            "---\nid: MP-975\ntitle: Evidence gate\ntype: MeasurementPlan\n\
             status: active\nstage: gate\nmetric: gate.pass_rate\n\
             definition_version: gate.pass-rate-v1\n{truth}{design}\
             negative_controls:\n  - kind: stale-evidence\n    description: stale evidence refuses a pass\n---\n\
             # Evidence gate\n"
        )
    };
    let valid_design = format!(
        "statistical_design:\n{DESIGN}  decision_rule:\n    comparator: ge\n    threshold: 0.8\n"
    );
    let mut cases = vec![
        (
            "ground_truth_kind".to_owned(),
            String::new(),
            valid_design.clone(),
        ),
        (
            "statistical_design".to_owned(),
            "ground_truth_kind: mechanical\n".to_owned(),
            String::new(),
        ),
    ];
    for member in [
        "population",
        "sampling",
        "repetitions",
        "estimator",
        "error_model",
        "uncertainty",
    ] {
        let design: String = valid_design
            .lines()
            .filter(|line| !line.trim_start().starts_with(&format!("{member}:")))
            .flat_map(|line| [line, "\n"])
            .collect();
        cases.push((
            format!("statistical_design.{member}"),
            "ground_truth_kind: mechanical\n".to_owned(),
            design,
        ));
    }
    for (member, wrong) in [
        ("population", "population: 5"),
        ("sampling", "sampling: []"),
        ("repetitions", "repetitions: many"),
    ] {
        let design: String = valid_design
            .lines()
            .map(|line| {
                if line.trim_start().starts_with(&format!("{member}:")) {
                    format!("  {wrong}\n")
                } else {
                    format!("{line}\n")
                }
            })
            .collect();
        cases.push((
            format!("statistical_design.{member}"),
            "ground_truth_kind: mechanical\n".to_owned(),
            design,
        ));
    }
    let no_rule: String = valid_design
        .lines()
        .take(8)
        .flat_map(|line| [line, "\n"])
        .collect();
    cases.push((
        "statistical_design.decision_rule".to_owned(),
        "ground_truth_kind: mechanical\n".to_owned(),
        no_rule,
    ));
    for (member, truth, design) in cases {
        let source = MemoryMeasurement::new()
            .with_document("spec/assurance/MP-975.md", document(&truth, &design));
        let error = load_measurement_plans(&source, PlanLoadOptions::default())
            .expect_err("an incomplete non-retired gate is refused");
        assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
        assert!(error.to_string().contains(&member), "{member}: {error}");
    }
}

#[trace("FR-110-AC-1", "TC-1810")]
/// Provenance: PLAT-1162
#[test]
fn tc_975_025_a_gate_with_a_procedure_and_no_apparatus_is_refused() {
    let document = format!(
        "---\nid: MP-975\ntitle: Procedure gate\ntype: MeasurementPlan\n\
         status: active\nstage: gate\nmetric: gate.pass_rate\n\
         definition_version: gate.pass-rate-v1\nground_truth_kind: mechanical\n\
         execution_procedure: campaign/procedure.json\n\
         statistical_design:\n{DESIGN}  decision_rule:\n    comparator: ge\n    threshold: 0.8\n\
         negative_controls:\n  - kind: stale-evidence\n    description: stale evidence refuses a pass\n---\n\
         # Procedure gate\n"
    );
    let source = MemoryMeasurement::new().with_document("spec/assurance/MP-975.md", document);
    let error = load_measurement_plans(&source, PlanLoadOptions::default())
        .expect_err("a procedure with no apparatus is refused at gate stage");
    assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid);
    assert!(error.to_string().contains("execution_procedure"), "{error}");
}

#[trace("FR-110-AC-1", "TC-1810")]
/// Provenance: PLAT-1162
#[test]
fn tc_975_026_a_retired_gate_without_negative_controls_loads() {
    let document = "---\nid: MP-975\ntitle: Retired gate\ntype: MeasurementPlan\n\
         status: retired\nowner: test\nstage: gate\nmetric: gate.pass_rate\n\
         definition_version: gate.pass-rate-v1\nrelationships: []\n---\n# Retired gate\n";
    let source = MemoryMeasurement::new().with_document("spec/assurance/MP-975.md", document);
    let plans = load_measurement_plans(&source, PlanLoadOptions::default())
        .expect("EA accepts a retired gate with no controls");
    assert_eq!(plans.len(), 1);
}

/// MP-001's own `count` / `eq 0` rule decides retained evidence: no adverse
/// outcome accepts, one rejects.
///
#[trace("FR-110-AC-1", "FR-108-AC-4", "TC-1810")]
/// Provenance: PLAT-1162
#[test]
fn tc_975_027_the_analyze_shaped_gate_decides_retained_evidence_by_its_own_rule() {
    let root = tempfile::tempdir().expect("the fixture repository is creatable");
    write(
        root.path(),
        "spec/assurance/MP-001-analyze-measurements.md",
        // Evidence is admitted only under an active plan.
        &include_str!("fixtures/MP-001-evidence-gate.md")
            .replace("status: proposed", "status: active"),
    );
    let plan = plan_of(root.path());
    for (id, adverse, expected) in [
        ("mp001-clean", 0, Verdict::Accept),
        ("mp001-adverse", 1, Verdict::Reject),
    ] {
        let mut collection = candidate(id, 0, 0, &json!(["config"]));
        collection["observations"] = json!([{
            "metric": "analysis_semantic_fault_differential_and_evidence_conformance",
            "planId": "MP-001",
            "definitionVersion": "quire-analyze.measurement-v1",
            "state": "measured",
            "value": adverse,
            "unit": "count",
            "shape": "count",
            "population": { "examined": 12, "matched": adverse, "complete": true, "repetitions": 3 },
        }]);
        publish(root.path(), &collection).expect("the retained evidence is admitted");
        let collections = stored(root.path());
        let stored_one = collections
            .iter()
            .find(|stored| stored.collection_id.as_str() == id)
            .expect("the collection was stored");
        let verdict = verify(
            &plan,
            &[Ranked::new(stored_one, Some(0))],
            TamperFacts::default(),
            OrderSource::CallerSupplied,
            None,
        );
        assert_eq!(verdict.verdict, expected, "{id}: {verdict:?}");
    }
}

/// MP-228 states its 3-of-5 repeated-pass condition as a typed design, and
/// the checker decides by it: 3 of 5 clearing passes accepts, 2 of 5 rejects,
/// and a population under the five passes is refused.
///
#[trace("FR-110-AC-1", "TC-1810")]
/// Provenance: PLAT-1162
#[test]
fn tc_975_028_the_criterion_strength_gate_states_five_repetitions() {
    let document = include_str!("../../../../spec/assurance/MP-228-jev-criterion-strength-gate.md");
    let source = MemoryMeasurement::new().with_document("spec/assurance/MP-228.md", document);
    let plans = load_measurement_plans(&source, PlanLoadOptions::default())
        .expect("the real MP-228 loads as a gate");
    let design = plans[0]
        .statistical_design
        .clone()
        .expect("MP-228 states a design");
    assert_eq!(design.repetitions.map(std::num::NonZeroU32::get), Some(5));
    assert_eq!(
        design.minimum_population.map(std::num::NonZeroU32::get),
        Some(5)
    );
    assert_eq!(
        design.decision_rule,
        Some(DecisionRule::against_threshold(Comparator::Ge, 0.6).unwrap())
    );

    let root = tempfile::tempdir().expect("the repository is creatable");
    write(root.path(), "spec/assurance/MP-228.md", document);
    let apparatus = plans[0]
        .protected_apparatus
        .as_ref()
        .expect("MP-228 protects its corpus");
    let names: Vec<String> = apparatus
        .iter()
        .map(|entry| entry.as_str().to_owned())
        .collect();
    for name in &names {
        write(root.path(), name, "{}");
    }
    let plan = plan_of(root.path());
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    for (id, matched, examined, expected) in [
        ("mp228-three", 3, 5, Some(Verdict::Accept)),
        ("mp228-two", 2, 5, Some(Verdict::Reject)),
        ("mp228-one", 1, 1, None),
    ] {
        let mut collection = candidate(id, 0, 0, &artifacts(root.path(), &names));
        collection["observations"][0]["metric"] = json!("jev.criterion_strength.gate");
        collection["observations"][0]["planId"] = json!("MP-228");
        collection["observations"][0]["definitionVersion"] =
            json!("jev.criterion-strength-gate-v1");
        collection["observations"][0]["value"] = json!(f64::from(matched) / f64::from(examined));
        collection["observations"][0]["population"] =
            json!({ "examined": examined, "matched": matched, "complete": true, "repetitions": 5 });
        let written = publish(root.path(), &collection);
        let Some(expected) = expected else {
            assert!(written.is_err(), "{id}: a 1-of-1 population is refused");
            continue;
        };
        written.unwrap_or_else(|error| panic!("{id} is admitted: {error}"));
        let collections = stored(root.path());
        let stored_one = collections
            .iter()
            .find(|stored| stored.collection_id.as_str() == id)
            .expect("the collection was stored");
        let verdict = verify(
            &plan,
            &[Ranked::new(stored_one, Some(0))],
            TamperFacts::default(),
            OrderSource::CallerSupplied,
            None,
        );
        assert_eq!(verdict.verdict, expected, "{id}: {verdict:?}");
    }
}

// ------------------------------------------------------------ intake

/// A named arrangement of a fixture repository.
type Case = (&'static str, fn(&Path));

/// Provenance: PLAT-975
#[trace("FR-110-AC-2", "TC-1811")]
#[test]
fn tc_975_002_intake_records_the_resolved_set_it_resolved_itself() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);

    let bytes = stored_bytes(root, "run-1");
    assert_eq!(
        bytes["verificationStack"]["protectedApparatus"],
        json!({ "MP-975": [
            "harness/answers.json",
            "labels/a.json",
            "labels/nested/.hidden",
        ]}),
        "every file under `labels/**`, the dotfile included, and nothing unprotected"
    );
    // Every protected file is a local file, never a label.
    assert_eq!(
        bytes["verificationStack"].get("unverifiedArtifacts"),
        Some(&json!(["config"]))
    );
}

/// Provenance: PLAT-975
#[trace("FR-110-AC-3", "TC-1812")]
#[test]
fn tc_975_003_an_entry_naming_no_file_refuses_the_write() {
    let cases: [Case; 5] = [
        ("the protected file is missing", |root| {
            std::fs::remove_file(root.join("harness/answers.json")).unwrap();
        }),
        ("the directory entry's directory is missing", |root| {
            std::fs::remove_dir_all(root.join("labels")).unwrap();
        }),
        ("the directory entry holds no file", |root| {
            std::fs::remove_dir_all(root.join("labels")).unwrap();
            std::fs::create_dir_all(root.join("labels/empty")).unwrap();
        }),
        ("the file entry names a directory", |root| {
            std::fs::remove_file(root.join("harness/answers.json")).unwrap();
            std::fs::create_dir_all(root.join("harness/answers.json")).unwrap();
            std::fs::write(root.join("harness/answers.json/x"), "x").unwrap();
        }),
        ("the name differs only in case", |root| {
            std::fs::rename(
                root.join("harness/answers.json"),
                root.join("harness/Answers.json"),
            )
            .unwrap();
        }),
    ];
    for (case, arrange) in cases {
        let temporary = repository(PROTECTED);
        let root = temporary.path();
        arrange(root);
        let present: Vec<&str> = ALL_FILES
            .into_iter()
            .filter(|name| root.join(name).is_file())
            .collect();
        let error =
            publish(root, &candidate("run-1", 1, 9, &artifacts(root, &present))).expect_err(case);
        assert_eq!(
            error.code(),
            MeasurementErrorCode::ApparatusUnresolved,
            "{case}: {error}"
        );
        assert!(error.to_string().contains("MP-975"), "{case}: {error}");
        assert_eq!(stored_count(root), 0, "{case}: nothing is written");
    }
}

/// Provenance: PLAT-975
#[trace("FR-110-AC-3", "TC-1813")]
#[test]
fn tc_975_004_a_symlink_is_refused_and_never_followed() {
    let cases: [Case; 3] = [
        ("the file entry is a symlink", |root| {
            std::fs::rename(root.join("harness/answers.json"), root.join("real.json")).unwrap();
            std::os::unix::fs::symlink(root.join("real.json"), root.join("harness/answers.json"))
                .unwrap();
        }),
        ("a symlink under the directory entry", |root| {
            std::os::unix::fs::symlink(root.join("dist/tool"), root.join("labels/linked")).unwrap();
        }),
        ("a symlinked ancestor", |root| {
            std::fs::rename(root.join("harness"), root.join("elsewhere")).unwrap();
            std::os::unix::fs::symlink(root.join("elsewhere"), root.join("harness")).unwrap();
        }),
    ];
    for (case, arrange) in cases {
        let temporary = repository(PROTECTED);
        let root = temporary.path();
        let declared = artifacts(root, &ALL_FILES);
        arrange(root);
        let error = publish(root, &candidate("run-1", 1, 9, &declared)).expect_err(case);
        assert_eq!(
            error.code(),
            MeasurementErrorCode::ApparatusSymlink,
            "{case}: {error}"
        );
        assert_eq!(stored_count(root), 0, "{case}: nothing is written");
    }
}

/// A producer-supplied artifact list can omit a file; intake resolves the set
/// itself and refuses a list that leaves a protected file out.
///
/// Provenance: PLAT-975
#[trace("FR-110-AC-3", "TC-1814")]
#[test]
fn tc_975_005_an_artifact_list_that_omits_a_protected_file_is_refused() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    let error = publish(
        root,
        &candidate(
            "run-1",
            1,
            9,
            &artifacts(
                root,
                &["harness/answers.json", "labels/a.json", "dist/tool"],
            ),
        ),
    )
    .expect_err("the dotfile under labels/** is not declared");
    assert_eq!(
        error.code(),
        MeasurementErrorCode::ApparatusUndeclared,
        "{error}"
    );
    assert_eq!(error.findings().len(), 1, "{error}");
    assert!(
        error.findings()[0].contains("labels/nested/.hidden"),
        "{error}"
    );

    assert_eq!(stored_count(root), 0);
}

/// Provenance: PLAT-975
#[trace("FR-110-AC-3", "TC-1815")]
#[test]
fn tc_975_006_an_unreadable_protected_file_is_refused() {
    use std::os::unix::fs::PermissionsExt as _;
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    let declared = artifacts(root, &ALL_FILES);
    std::os::unix::net::UnixListener::bind(root.join("labels/socket")).unwrap();
    let error = publish(root, &candidate("run-1", 1, 9, &declared))
        .expect_err("a socket under a directory entry is not a file");
    assert_eq!(
        error.code(),
        MeasurementErrorCode::ApparatusUnreadable,
        "{error}"
    );
    std::fs::remove_file(root.join("labels/socket")).unwrap();

    let restricted = root.join("labels/nested");
    std::fs::set_permissions(&restricted, std::fs::Permissions::from_mode(0o000)).unwrap();
    // Root reads through mode 000, so the case cannot be exercised there.
    let running_as_root = std::fs::read_dir(&restricted).is_ok();
    let outcome = publish(root, &candidate("run-1", 1, 9, &declared));
    std::fs::set_permissions(&restricted, std::fs::Permissions::from_mode(0o755)).unwrap();
    if !running_as_root {
        let error = outcome.expect_err("an unlistable directory is refused");
        assert_eq!(
            error.code(),
            MeasurementErrorCode::ApparatusUnreadable,
            "{error}"
        );
    }
}

// ------------------------------------------------------------ comparison

/// Provenance: PLAT-975
#[trace("FR-110-AC-4", "TC-1817")]
#[test]
fn tc_975_008_a_file_added_under_a_directory_entry_is_an_apparatus_change() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 8);
    write(root, "labels/b.json", "[\"b\"]");
    publish(
        root,
        &candidate(
            "run-2",
            2,
            8,
            &artifacts(
                root,
                &[
                    "harness/answers.json",
                    "labels/a.json",
                    "labels/b.json",
                    "labels/nested/.hidden",
                    "dist/tool",
                ],
            ),
        ),
    )
    .unwrap();
    let collections = stored(root);
    let rows = compare_measurement_collections(&collections[0], &collections[1], &[]).unwrap();
    assert_eq!(reasons(&rows), [ComparisonReasonCode::ApparatusChanged]);
    assert!(rows[0].reasons[0].message.contains("labels/b.json"));
    assert_eq!(rows[0].delta, None);

    // And removed: the reverse comparison names the same file.
    let rows = compare_measurement_collections(&collections[1], &collections[0], &[]).unwrap();
    assert_eq!(reasons(&rows), [ComparisonReasonCode::ApparatusChanged]);
}

/// Provenance: PLAT-975
#[trace("FR-110-AC-4", "TC-1818")]
#[test]
fn tc_975_009_an_unprotected_artifact_stated_by_one_side_is_reported_and_does_not_block() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 8);
    let without_tool: Vec<&str> = ALL_FILES
        .iter()
        .copied()
        .filter(|name| *name != "dist/tool")
        .collect();
    publish(
        root,
        &candidate("run-2", 2, 8, &artifacts(root, &without_tool)),
    )
    .unwrap();

    let collections = stored(root);
    let rows = compare_measurement_collections(&collections[0], &collections[1], &[]).unwrap();
    assert_eq!(reasons(&rows), [ComparisonReasonCode::ArtifactChanged]);
    assert!(!rows[0].reasons[0].blocking);
    assert!(rows[0].reasons[0].message.contains("dist/tool"));
    assert_eq!(rows[0].status, ComparisonStatus::Comparable);
    assert_eq!(rows[0].delta, Some(0.0));
}

/// A plan that protects nothing is compared and stored exactly as before
/// PLAT-975: no record, and an artifact list is not reported on.
///
/// Provenance: PLAT-975
#[trace("FR-110-AC-2", "FR-110-AC-4", "TC-1819")]
#[test]
fn tc_975_010_no_apparatus_declared_leaves_behaviour_unchanged() {
    let temporary = repository("");
    let root = temporary.path();
    publish_run(root, "run-1", 1, 8);
    publish_run(root, "run-2", 2, 8);

    assert_eq!(
        stored_bytes(root, "run-1")["verificationStack"].get("protectedApparatus"),
        None,
        "no protecting plan, no record"
    );
    let collections = stored(root);
    let rows = compare_measurement_collections(&collections[0], &collections[1], &[]).unwrap();
    assert_eq!(reasons(&rows), []);
    assert_eq!(rows[0].status, ComparisonStatus::Comparable);
}

/// The stored baseline is what a new run is compared against. Regenerating a
/// baseline alongside an edited apparatus cannot replace the stored one, and
/// a regenerated copy under a new id does not change what the stored one
/// recorded.
///
/// Provenance: PLAT-975
#[trace("FR-110-AC-5", "TC-1820")]
#[test]
fn tc_975_011_a_regenerated_baseline_is_still_refused_against_the_stored_one() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "baseline", 1, 8);

    let regenerated = candidate("baseline", 1, 9, &artifacts(root, &ALL_FILES));
    let error = publish(root, &regenerated).expect_err("the stored baseline is write-once");
    assert_eq!(error.code(), MeasurementErrorCode::CollectionIdCollision);
    publish_run(root, "baseline-regenerated", 2, 8);
    publish_run_with_extra_label(root, "run", 3, 8);

    let collections = stored(root);
    let by_id = |id: &str| {
        collections
            .iter()
            .find(|collection| collection.collection_id.as_str() == id)
            .unwrap()
    };
    let rows = compare_measurement_collections(by_id("baseline"), by_id("run"), &[]).unwrap();
    assert_eq!(reasons(&rows), [ComparisonReasonCode::ApparatusChanged]);
    assert_eq!(rows[0].delta, None);
}

/// A collection lacking the record, or listing a protected name as
/// unverified, is not comparable with one that recorded the set.
///
/// Provenance: PLAT-975
#[trace("FR-110-AC-4", "TC-1821")]
#[test]
fn tc_975_012_a_missing_record_or_an_unverified_protected_name_blocks() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 8);
    publish_run(root, "run-2", 2, 8);
    let recorded = &stored(root)[1];

    let mut unrecorded = stored_bytes(root, "run-1");
    unrecorded["verificationStack"]
        .as_object_mut()
        .unwrap()
        .remove("protectedApparatus");
    let unrecorded = stored_measurement_collection(&from_serde(&unrecorded).unwrap()).unwrap();
    let rows = compare_measurement_collections(&unrecorded, recorded, &[]).unwrap();
    assert_eq!(reasons(&rows), [ComparisonReasonCode::ApparatusChanged]);

    let mut unverified = stored_bytes(root, "run-1");
    unverified["verificationStack"]["unverifiedArtifacts"] = json!(["harness/answers.json"]);
    let unverified = stored_measurement_collection(&from_serde(&unverified).unwrap()).unwrap();
    let rows = compare_measurement_collections(&unverified, recorded, &[]).unwrap();
    assert_eq!(reasons(&rows), [ComparisonReasonCode::ApparatusChanged]);

    // The positive control: the same two runs as written compare.
    let collections = stored(root);
    let rows = compare_measurement_collections(&collections[0], &collections[1], &[]).unwrap();
    assert_eq!(reasons(&rows), []);
}

// ------------------------------------------------------------ checker

/// Provenance: PLAT-975
#[trace("FR-110-AC-7", "TC-1822")]
#[test]
fn tc_975_013_a_changed_set_within_one_version_rejects_without_a_control() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);
    publish_run_with_extra_label(root, "run-2", 2, 9);
    let plan = plan_of(root);
    let collections = stored(root);
    assert_eq!(
        checked(&plan, &collections),
        [Reason::NoPrior, Reason::ApparatusEdit],
        "the unversioned change rejects"
    );
    assert_eq!(verdict_of(&plan, &collections), Verdict::Reject);

    // The positive control: the same set on both runs is accepted.
    let same = repository(PROTECTED);
    publish_run(same.path(), "run-1", 1, 9);
    publish_run(same.path(), "run-2", 2, 9);
    assert_eq!(
        verdict_of(&plan_of(same.path()), &stored(same.path())),
        Verdict::Accept
    );
}

/// Provenance: PLAT-975
#[trace("FR-110-AC-7", "TC-1823")]
#[test]
fn tc_975_014_an_apparatus_edit_under_a_declared_control_also_rejects() {
    let temporary = repository(&format!(
        "{PROTECTED}negative_controls:\n  - kind: apparatus-edit\n    description: the answer key is protected\n"
    ));
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);
    publish_run_with_extra_label(root, "run-2", 2, 9);
    let plan = plan_of(root);
    let collections = stored(root);
    assert_eq!(
        checked(&plan, &collections),
        [Reason::NoPrior, Reason::ApparatusEdit]
    );
    assert_eq!(verdict_of(&plan, &collections), Verdict::Reject);
}

/// Provenance: PLAT-975
#[trace("FR-110-AC-7", "TC-1824")]
#[test]
fn tc_975_015_a_candidate_with_no_record_under_a_protecting_plan_is_inconclusive() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);
    let mut bytes = stored_bytes(root, "run-1");
    bytes["verificationStack"]
        .as_object_mut()
        .unwrap()
        .remove("protectedApparatus");
    let unrecorded = stored_measurement_collection(&from_serde(&bytes).unwrap()).unwrap();
    let reasons = checked(&plan_of(root), &[unrecorded]);
    assert!(
        reasons.contains(&Reason::ApparatusUnrecorded),
        "{reasons:?}"
    );
    assert_eq!(Reason::ApparatusUnrecorded.verdict(), Verdict::Inconclusive);
}

// ------------------------------------------------------------ report

fn ratchet_outcome(root: &Path) -> RatchetOutcome {
    let report = build_measurement_report(
        &DiskMeasurement::new(root),
        root,
        &quoin_measurement::IntakeOrder::none(),
    )
    .unwrap();
    match report.current[0].stage_verdict.clone() {
        Some(StageVerdict::Ratchet { outcome, .. }) => outcome,
        other => panic!("a ratchet verdict, got {other:?}"),
    }
}

/// Provenance: PLAT-975
#[trace("FR-110-AC-6", "TC-1825")]
#[test]
fn tc_975_016_a_ratchet_does_not_hold_against_another_apparatus() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);
    publish_run_with_extra_label(root, "run-2", 2, 9);
    assert_eq!(
        ratchet_outcome(root),
        RatchetOutcome::Inconclusive(InconclusiveReason::ApparatusChanged)
    );

    let same = repository(PROTECTED);
    publish_run(same.path(), "run-1", 1, 9);
    write(same.path(), "dist/tool", "tool v2");
    publish_run(same.path(), "run-2", 2, 9);
    assert!(matches!(
        ratchet_outcome(same.path()),
        RatchetOutcome::Held { .. }
    ));
}

/// Provenance: PLAT-975
#[trace("FR-110-AC-8", "TC-1826")]
#[test]
fn tc_975_017_a_malformed_stored_record_is_refused_on_read() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);
    for malformed in [
        json!([]),
        json!({ "MP-975": {} }),
        json!({ "MP-975": { "harness/answers.json": "not a list" } }),
        json!({ "MP-975": [7] }),
    ] {
        let mut bytes = stored_bytes(root, "run-1");
        bytes["verificationStack"]["protectedApparatus"] = malformed.clone();
        let error = stored_measurement_collection(&from_serde(&bytes).unwrap())
            .expect_err("a malformed record is refused");
        assert_eq!(
            error.code(),
            MeasurementErrorCode::CollectionInvalid,
            "{malformed}"
        );
        assert!(error.to_string().contains("protectedApparatus"), "{error}");
    }
}

/// A stored `protectedApparatus` whose plan-id or path key is malformed is
/// refused on read, not carried through as an opaque string (PLAT-985,
/// quoin#600 review): an empty plan-id key can never name a real plan, and a
/// path key that is a `<directory>/**` glob, an absolute path, or carries a
/// `..` segment is not a shape intake's own resolver ever writes.
///
/// Provenance: PLAT-985, quoin#600
#[trace("FR-110-AC-8", "TC-1833")]
#[test]
fn tc_985_001_protected_apparatus_key_shapes_are_validated_on_read() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);
    for malformed in [
        // An empty plan-id key.
        json!({ "": ["harness/answers.json"] }),
        // A `<directory>/**` glob where a resolved record must name one file.
        json!({ "MP-975": ["labels/**"] }),
        // An absolute path.
        json!({ "MP-975": ["/etc/passwd"] }),
        // A `..` segment.
        json!({ "MP-975": ["harness/../../../etc/passwd"] }),
    ] {
        let mut bytes = stored_bytes(root, "run-1");
        bytes["verificationStack"]["protectedApparatus"] = malformed.clone();
        let error = stored_measurement_collection(&from_serde(&bytes).unwrap())
            .expect_err("a malformed key is refused");
        assert_eq!(
            error.code(),
            MeasurementErrorCode::CollectionInvalid,
            "{malformed}"
        );
        assert!(error.to_string().contains("protectedApparatus"), "{error}");
    }
}

/// Provenance: PLAT-975
#[trace("FR-110-AC-2", "TC-1827")]
#[test]
fn tc_975_018_a_candidate_stating_a_computed_member_is_refused() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    for (member, stated) in [
        (
            "protectedApparatus",
            json!({ "MP-975": ["harness/answers.json"] }),
        ),
        ("unverifiedArtifacts", json!(["config"])),
    ] {
        let mut value = candidate("run-1", 1, 9, &artifacts(root, &ALL_FILES));
        value["verificationStack"][member] = stated;
        let error = publish(root, &value).expect_err("a stated computed member is refused");
        assert_eq!(
            error.code(),
            MeasurementErrorCode::CollectionInvalid,
            "{member}"
        );
        assert!(error.to_string().contains(member), "{member}: {error}");
        assert_eq!(stored_count(root), 0, "{member}: nothing is written");
    }
}

/// Deleting the plan's list does not switch protection off: the runs
/// recorded under it still carry their sets.
///
/// Provenance: PLAT-975
#[trace("FR-110-AC-6", "FR-110-AC-7", "TC-1828")]
#[test]
fn tc_975_019_removing_the_list_from_the_plan_keeps_the_series_protected() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);
    publish_run_with_extra_label(root, "run-2", 2, 9);
    write(root, "spec/assurance/MP-975.md", &plan_document(""));
    let plan = plan_of(root);
    assert!(plan.protected_apparatus.is_none());
    let collections = stored(root);
    let verdict = verdict_of(&plan, &collections);
    assert_ne!(verdict, Verdict::Accept);
    assert_eq!(verdict, Verdict::Reject);
    assert!(checked(&plan, &collections).contains(&Reason::ApparatusEdit));
    let outcome = ratchet_outcome(root);
    assert!(
        !matches!(outcome, RatchetOutcome::Held { .. }),
        "{outcome:?}"
    );
    assert_eq!(
        outcome,
        RatchetOutcome::Inconclusive(InconclusiveReason::ApparatusChanged)
    );

    // A run written after the list was removed records nothing, and that is
    // not a pass either.
    publish_run(root, "run-3", 3, 9);
    let collections = stored(root);
    assert!(checked(&plan, &collections).contains(&Reason::ApparatusUnrecorded));
    assert_ne!(verdict_of(&plan, &collections), Verdict::Accept);
    assert_eq!(
        ratchet_outcome(root),
        RatchetOutcome::Inconclusive(InconclusiveReason::ApparatusUnrecorded)
    );
}

/// An answer key edited in the working tree between a failed run and a pass
/// is still a rerun of the same source: the changed apparatus does not
/// excuse it.
///
/// Provenance: PLAT-975
#[trace("FR-110-AC-7", "FR-108-AC-3", "TC-1829")]
#[test]
fn tc_975_020_an_uncommitted_answer_key_edit_then_a_pass_is_a_rerun() {
    let temporary = repository("");
    let root = temporary.path();
    write(
        root,
        "spec/assurance/MP-975.md",
        &plan_with("ratchet", "threshold: 0.8", PROTECTED),
    );
    publish_run(root, "run-1", 1, 5);
    write(root, "harness/answers.json", "{\"answers\":\"easier\"}");
    publish_run(root, "run-2", 2, 9);
    let plan = plan_of(root);
    let collections = stored(root);
    let reasons = checked(&plan, &collections);
    assert!(reasons.contains(&Reason::RerunUntilPass), "{reasons:?}");
    assert_eq!(verdict_of(&plan, &collections), Verdict::Reject);
}

/// Provenance: PLAT-975
#[trace("FR-110-AC-6", "TC-1830")]
#[test]
fn tc_975_021_a_ratchet_whose_newest_collection_recorded_nothing_is_inconclusive() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);
    publish_run(root, "run-2", 2, 9);
    let path = measurement_path(root, &CollectionId::parse("run-2").unwrap());
    let mut bytes = stored_bytes(root, "run-2");
    bytes["verificationStack"]
        .as_object_mut()
        .unwrap()
        .remove("protectedApparatus");
    std::fs::write(path, serde_json::to_vec(&bytes).unwrap()).unwrap();
    assert_eq!(
        ratchet_outcome(root),
        RatchetOutcome::Inconclusive(InconclusiveReason::ApparatusUnrecorded)
    );
}
