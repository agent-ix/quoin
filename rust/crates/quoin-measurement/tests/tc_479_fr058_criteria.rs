// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every FR-058 criterion the deleted `tests/intervention.test.ts` carried,
//! restated against this crate.
//!
//! Trace: FR-058-AC-1, FR-058-AC-2, FR-058-AC-3, FR-058-AC-4, FR-058-AC-5
//! Provenance: quoin#479
//!
//! # Why this file exists
//!
//! `src/measurement/agent-eval-intervention.ts` and the tests that held it to
//! FR-058 are deleted in the same commit (FR-101). `tc_471_corpus.rs` proves
//! the port reproduces the retained record byte for byte, but it is one test
//! tagged `FR-100`: it does not say which of FR-058's five criteria it carries,
//! and four of them are about what the producer *refuses* or *ignores*, which a
//! reproduction test cannot see. This file is one test per criterion.
//!
//! # Where the inputs come from
//!
//! The two committed `cli-agent-evals` runs under `spec/evidence/agent-evals/`
//! — real two-run output, retained, not written here — the committed producer
//! definition beside them, and `spec/assurance/MP-220-cli-eval-sentinel-contract.md`,
//! the authored plan that governs the definition version. Every test copies the
//! committed `spec/` tree into a temporary repository and deletes the retained
//! record, so the producer has to make it again; every refusal case is that
//! committed base, mutated here in Rust. Nothing is compared against bytes the
//! producer wrote.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

#[path = "common/census.rs"]
mod census;
#[path = "common/copy.rs"]
mod copy;
#[path = "common/paths.rs"]
mod paths;

use census::rust_files;
use copy::copy_tree;
use paths::repo_root;

use std::path::{Path, PathBuf};

use quoin_measurement::intervention::agent_eval::{
    AgentEvalInterventionDefinition, ProducedIntervention, produce_agent_eval_intervention,
};
use quoin_measurement::{
    DiskMeasurement, InterventionIntakeError, InterventionRefusalCode, read_intervention_records,
};
use serde_json::{Value, json};
use tempfile::TempDir;

// ------------------------------------------------------ the committed inputs

/// The treatment report's `generatedAt`, which the record's `observed_at` is
/// derived from. The baseline's is `2026-08-30T17:34:20.678Z`; the two differ,
/// which is what makes `tc_479_252` able to tell them apart.
const TREATMENT_GENERATED_AT: &str = "2026-08-30T17:54:41.378Z";

/// The three gap sentences the producer states about its own power.
const SELF_DECLARED_GAPS: [&str; 3] = [
    "agent-eval reports do not contain repeated samples for every scenario",
    "one or more declared interactions or confounders are uncontrolled or unknown",
    "no justified attribution method was supplied",
];

/// The committed `spec/` tree, copied, with the retained intervention record
/// removed so the producer has to publish it again.
///
/// The retained record's bytes are returned beside the repository: they are the
/// oracle, and they were committed long before this test.
fn producing_repo() -> (TempDir, Vec<u8>) {
    let temporary = tempfile::tempdir().expect("a temporary repository");
    copy_tree(&repo_root().join("spec"), &temporary.path().join("spec"));
    let retained = temporary
        .path()
        .join("spec")
        .join("evidence")
        .join("interventions")
        .join("quoin-270-cli-eval-sentinel-contract.json");
    let bytes = std::fs::read(&retained).expect("the retained record was copied");
    std::fs::remove_file(&retained).expect("the retained record is removable");
    (temporary, bytes)
}

/// The committed producer definition, as JSON.
fn definition_json(repo: &Path) -> Value {
    let raw = std::fs::read_to_string(
        repo.join("spec")
            .join("evidence")
            .join("agent-evals")
            .join("quoin-270-sentinel-definition.json"),
    )
    .expect("the committed definition is readable");
    serde_json::from_str(&raw).expect("the committed definition is JSON")
}

/// A definition value, as the producer's argument type.
fn definition(value: &Value) -> AgentEvalInterventionDefinition {
    serde_json::from_value(value.clone()).expect("the definition deserialises")
}

/// One of the two retained reports, as JSON.
fn report_json(repo: &Path, arm: &str) -> Value {
    let raw = std::fs::read_to_string(report_path(repo, arm))
        .unwrap_or_else(|error| panic!("the {arm} report is readable: {error}"));
    serde_json::from_str(&raw).expect("the report is JSON")
}

/// Where a retained report lives in `repo`.
fn report_path(repo: &Path, arm: &str) -> PathBuf {
    repo.join("spec")
        .join("evidence")
        .join("agent-evals")
        .join(format!("quoin-270-sentinel-{arm}.json"))
}

/// Run the producer.
fn produce(repo: &Path, value: &Value) -> Result<ProducedIntervention, InterventionIntakeError> {
    produce_agent_eval_intervention(repo, &definition(value))
}

/// The refusal a production earned, or a panic naming what it produced instead.
fn refusal(repo: &Path, value: &Value, what: &str) -> InterventionIntakeError {
    match produce(repo, value) {
        Ok(produced) => panic!(
            "{what}: produced {}, and the criterion refuses it",
            produced.path.display()
        ),
        Err(error) => error,
    }
}

// ------------------------------------------------------------- the criteria

/// Two real agent-eval reports governed by a supported input-schema declaration
/// and carrying the same non-empty scenario set produce treatment-linked
/// observations whose sample counts, pass rates, computed effects, and
/// observation timestamp match the reports.
///
/// Trace: FR-058-AC-1
/// Provenance: quoin#479
#[test]
fn tc_479_250_two_retained_runs_produce_the_observations_the_reports_state() {
    let (temporary, retained) = producing_repo();
    let repo = temporary.path();

    let produced = produce(repo, &definition_json(repo)).expect("the retained runs produce");

    // Each arm's sample count is the report's, not the definition's: the
    // definition states no sample size at all.
    assert_eq!(
        produced.record.baseline.sample_size, 2,
        "the baseline ran twice"
    );
    assert_eq!(produced.record.treatments.len(), 1, "one treatment arm");
    assert_eq!(
        produced.record.treatments[0].sample_size, 2,
        "the treatment ran twice"
    );
    assert_eq!(
        produced.record.treatments[0].id.as_str(),
        "standalone-detector"
    );

    // One scenario, `1/2` on both arms, so the effect is exactly zero and the
    // observation is treatment-linked.
    assert_eq!(
        serde_json::to_value(&produced.record.measured_effects).expect("the effects serialise"),
        json!([{
            "treatment_id": "standalone-detector",
            "metric": "agent-eval.ea-001.pass-rate",
            "baseline_value": 0.5,
            "treatment_value": 0.5,
            "effect": 0.0,
            "unit": "fraction",
        }]),
        "the effects are the reports' own pass rates and their difference"
    );

    assert_eq!(
        produced.record.observed_at.as_str(),
        TREATMENT_GENERATED_AT,
        "the observation timestamp is the treatment report's clock"
    );

    // And the whole record is the one the repository already retains — an
    // oracle committed before this test, and not one it wrote.
    assert_eq!(
        std::fs::read(&produced.path).expect("the produced record is readable"),
        retained,
        "the producer did not reproduce the retained record"
    );
}

/// An absent or unsupported input-schema declaration and empty, malformed,
/// structurally incompatible, duplicate-scenario, or baseline/treatment
/// scenario-mismatch reports are refused without an intervention store entry.
///
/// Trace: FR-058-AC-2
/// Provenance: quoin#479
#[test]
fn tc_479_251_every_unreadable_declaration_or_report_is_refused_without_a_store_entry() {
    /// Below this the census is measuring one refusal and calling it six.
    const CASE_FLOOR: usize = 7;

    /// What a case does to the committed base before the producer runs.
    enum Mutation {
        /// Replace the definition's `report_schema_version`.
        Schema(&'static str),
        /// Replace the treatment report's bytes.
        Treatment(&'static str),
        /// Rewrite the treatment report's JSON.
        TreatmentJson(fn(&mut Value)),
    }

    let cases: [(&str, Mutation, InterventionRefusalCode); 7] = [
        (
            "an absent input-schema declaration",
            Mutation::Schema(""),
            InterventionRefusalCode::DefinitionMismatch,
        ),
        (
            "an unsupported input-schema declaration",
            Mutation::Schema("cli-agent-evals-report-v2"),
            InterventionRefusalCode::DefinitionMismatch,
        ),
        (
            "a malformed report",
            Mutation::Treatment("{ this is not JSON"),
            InterventionRefusalCode::InvalidRecord,
        ),
        (
            "an empty report",
            Mutation::TreatmentJson(|report| {
                report["results"] = json!([]);
            }),
            InterventionRefusalCode::InvalidRecord,
        ),
        (
            "a structurally incompatible report",
            Mutation::TreatmentJson(|report| {
                report
                    .as_object_mut()
                    .expect("a report object")
                    .remove("repeats");
            }),
            InterventionRefusalCode::InvalidRecord,
        ),
        (
            "a report repeating a scenario id",
            Mutation::TreatmentJson(|report| {
                let first = report["results"][0].clone();
                report["results"]
                    .as_array_mut()
                    .expect("a results array")
                    .push(first);
            }),
            InterventionRefusalCode::InvalidRecord,
        ),
        (
            "reports that do not name the same scenarios",
            Mutation::TreatmentJson(|report| {
                report["results"][0]["id"] = json!("EA-002");
            }),
            InterventionRefusalCode::InvalidRecord,
        ),
    ];

    assert!(
        cases.len() >= CASE_FLOOR,
        "anti-vacuity floor: at least {CASE_FLOOR} cases expected, saw {}",
        cases.len()
    );

    for (what, mutation, expected) in cases {
        let (temporary, _) = producing_repo();
        let repo = temporary.path();
        let mut value = definition_json(repo);
        match mutation {
            Mutation::Schema(version) => value["report_schema_version"] = json!(version),
            Mutation::Treatment(raw) => {
                std::fs::write(report_path(repo, "treatment"), raw)
                    .expect("the treatment report is writable");
            }
            Mutation::TreatmentJson(edit) => {
                let mut report = report_json(repo, "treatment");
                edit(&mut report);
                std::fs::write(
                    report_path(repo, "treatment"),
                    serde_json::to_vec(&report).expect("the report serialises"),
                )
                .expect("the treatment report is writable");
            }
        }

        let error = refusal(repo, &value, what);
        assert_eq!(error.code(), expected, "{what}: the refusal code");
        assert!(
            !error.findings().is_empty(),
            "{what}: the refusal says nothing"
        );
        assert!(
            read_intervention_records(&DiskMeasurement::new(repo))
                .expect("the store is readable")
                .is_empty(),
            "{what}: a store entry was written despite the refusal"
        );
    }
}

/// The observation timestamp and raw-evidence media type, byte size, and digest
/// are computed from the exact unmodified reports; caller-supplied substitutes
/// cannot change them.
///
/// Trace: FR-058-AC-3
/// Provenance: quoin#479
#[test]
fn tc_479_252_caller_supplied_substitutes_do_not_reach_the_record() {
    let (temporary, _) = producing_repo();
    let repo = temporary.path();

    // A definition document carrying exactly the substitutes the criterion
    // names. The producer's definition type declares none of them, so they
    // deserialise away — this test is what holds that shape in place.
    let mut value = definition_json(repo);
    value["observed_at"] = json!("2000-01-01T00:00:00.000Z");
    value["measured_effects"] = json!([{
        "treatment_id": "standalone-detector",
        "metric": "agent-eval.ea-001.pass-rate",
        "baseline_value": 0.1,
        "treatment_value": 0.9,
        "effect": 0.8,
        "unit": "fraction",
    }]);
    value["raw_evidence"] = json!([{
        "path": "agent-evals/quoin-270-sentinel-baseline.json",
        "media_type": "text/plain",
        "size_bytes": 1,
        "digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
    }]);

    let produced = produce(repo, &value).expect("the substitutes are ignored, not refused");

    assert_eq!(
        produced.record.observed_at.as_str(),
        TREATMENT_GENERATED_AT,
        "the caller's timestamp replaced the treatment report's clock"
    );
    assert_ne!(
        produced.record.observed_at.as_str(),
        "2026-08-30T17:34:20.678Z",
        "the baseline report's clock is not the observation timestamp"
    );
    assert_eq!(
        serde_json::to_value(&produced.record.raw_evidence).expect("the references serialise"),
        json!([
            {
                "path": "agent-evals/quoin-270-sentinel-baseline.json",
                "media_type": "application/json",
                "size_bytes": 11305,
                "digest": "sha256:741ab150c107d1f5551a9be2092081cc0439f85e804d23f81e3923dfab8fe076",
            },
            {
                "path": "agent-evals/quoin-270-sentinel-treatment.json",
                "media_type": "application/json",
                "size_bytes": 14261,
                "digest": "sha256:baade4ab1c2e8f3447b4c585c95634884cb2ed1c69a9cf8819d0b495c7f77963",
            },
        ]),
        "the raw-evidence metadata is the retained files', not the caller's"
    );
    assert_eq!(
        serde_json::to_value(&produced.record.measured_effects).expect("the effects serialise"),
        json!([{
            "treatment_id": "standalone-detector",
            "metric": "agent-eval.ea-001.pass-rate",
            "baseline_value": 0.5,
            "treatment_value": 0.5,
            "effect": 0.0,
            "unit": "fraction",
        }]),
        "the caller's measured effect replaced the computed one"
    );
}

/// Inadequate repetition, an uncontrolled/unknown qualifier, or absent
/// attribution method produces `cause_not_established` with `none` confidence
/// and explicit gaps rather than a causal conclusion.
///
/// Trace: FR-058-AC-4
/// Provenance: quoin#479
#[test]
fn tc_479_253_an_underpowered_production_states_its_gaps_and_claims_no_cause() {
    let (temporary, _) = producing_repo();
    let repo = temporary.path();

    // One sample per arm: inadequate repetition, stated by the reports
    // themselves rather than by the definition.
    for arm in ["baseline", "treatment"] {
        let mut report = report_json(repo, arm);
        report["repeats"] = json!(1);
        report["results"][0]["passRate"] = json!("1/1");
        std::fs::write(
            report_path(repo, arm),
            serde_json::to_vec(&report).expect("the report serialises"),
        )
        .expect("the report is writable");
    }

    // The committed definition already declares one uncontrolled interaction,
    // two open confounders, and no attribution method; the record id is changed
    // so this under-powered production cannot be mistaken for the retained one.
    let mut value = definition_json(repo);
    value["record_id"] = json!("quoin-479-underpowered");
    assert!(
        value.get("attribution_method").is_none(),
        "this case needs a definition that supplies no attribution method"
    );

    let produced = produce(repo, &value).expect("an under-powered production is still produced");

    assert_eq!(
        produced.record.conclusion.kind.as_str(),
        "cause_not_established",
        "an under-powered production claims no cause"
    );
    assert_eq!(
        produced.record.conclusion.attribution_confidence.as_str(),
        "none",
        "and attributes nothing"
    );
    assert_eq!(
        produced.record.conclusion.statement,
        "The retained agent-evaluation runs show no observed pass-rate difference; this adapter \
         does not establish causality.",
        "the statement says what the runs showed and what it does not establish"
    );

    for gap in SELF_DECLARED_GAPS {
        assert!(
            produced.record.gaps.iter().any(|stated| stated == gap),
            "the producer did not state the gap `{gap}`: {:?}",
            produced.record.gaps
        );
    }
    assert!(
        !SELF_DECLARED_GAPS.is_empty(),
        "a gap census over nothing measures nothing"
    );
    // The definition's own three gaps survive beside the producer's three.
    assert_eq!(
        produced.record.gaps.len(),
        value["gaps"].as_array().expect("declared gaps").len() + SELF_DECLARED_GAPS.len(),
        "the declared and the self-declared gaps are both stated, once each: {:?}",
        produced.record.gaps
    );
}

/// A real two-run integration invokes `cli-agent-evals` outside Quoin, then the
/// Quoin producer consumes the retained reports without spawning any process
/// and persists the record through FR-057.
///
/// Trace: FR-058-AC-5
/// Provenance: quoin#479
#[test]
fn tc_479_254_the_producer_spawns_nothing_and_persists_through_the_intake() {
    /// What the producer may not reach for. A process spawn or a socket is how
    /// an adapter would become an experiment runner.
    const FORBIDDEN: [&str; 6] = [
        "std::process",
        "std::net",
        "Command::new",
        "TcpStream",
        "reqwest",
        "tokio",
    ];

    /// Below this the census is reading an empty tree and proving nothing.
    const SOURCE_FLOOR: usize = 4;

    let producer_modules = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("intervention")
        .join("agent_eval");
    let mut measured = 0_usize;
    for path in rust_files(&producer_modules) {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: unreadable: {error}", path.display()));
        for needle in FORBIDDEN {
            assert!(
                !text.contains(needle),
                "{}: the producer reaches for `{needle}`",
                path.display()
            );
        }
        measured += 1;
    }
    assert!(
        measured >= SOURCE_FLOOR,
        "the census read {measured} producer modules, below the floor of {SOURCE_FLOOR}"
    );

    // The reports are what a real two-run evaluation left behind: they are
    // committed, and the producer only reads them.
    let (temporary, retained) = producing_repo();
    let repo = temporary.path();
    let before = std::fs::read(report_path(repo, "treatment")).expect("the report is readable");

    let produced = produce(repo, &definition_json(repo)).expect("the retained runs produce");
    assert_eq!(
        std::fs::read(report_path(repo, "treatment")).expect("the report is readable"),
        before,
        "the producer modified the run it consumed"
    );

    // It persists through FR-057: the record is where the intake publishes, and
    // the intake reads it back.
    assert_eq!(
        produced.path,
        repo.join("spec")
            .join("evidence")
            .join("interventions")
            .join("p-quoin-270-cli-eval-sentinel-contract.json"),
        "the record was published by the FR-057 writer, under its namespaced name"
    );
    let stored =
        read_intervention_records(&DiskMeasurement::new(repo)).expect("the store is readable");
    assert_eq!(stored.len(), 1, "one production, one retained record");
    // Field by field rather than whole-record: the canonical writer emits the
    // zero effect as `0`, so the read-back carries a JSON integer where the
    // producer held an IEEE-754 zero. That is the store's number grammar, not a
    // difference in what was retained, and the bytes are compared below.
    assert_eq!(
        stored[0].record_id, produced.record.record_id,
        "the intake retained the produced identity"
    );
    assert_eq!(
        stored[0].observed_at, produced.record.observed_at,
        "and its observation timestamp"
    );
    assert_eq!(
        stored[0].raw_evidence, produced.record.raw_evidence,
        "and its raw-evidence references"
    );
    assert_eq!(
        stored[0].conclusion, produced.record.conclusion,
        "and its conclusion"
    );
    assert_eq!(stored[0].gaps, produced.record.gaps, "and its gaps");
    assert_eq!(
        std::fs::read(&produced.path).expect("the record is readable"),
        retained,
        "and the retained bytes are the committed ones"
    );

    // The FR-057 boundary is not bypassed: without the governing plan the
    // producer cannot publish, and it refuses with the intake's own code.
    let (ungoverned, _) = producing_repo();
    std::fs::remove_dir_all(ungoverned.path().join("spec").join("assurance"))
        .expect("the authored plans are removable");
    let error = refusal(
        ungoverned.path(),
        &definition_json(ungoverned.path()),
        "a production with no governing plan",
    );
    assert_eq!(
        error.code(),
        InterventionRefusalCode::GoverningPlanAbsent,
        "the producer submits through the FR-057 intake, which owns this refusal"
    );
    assert!(
        read_intervention_records(&DiskMeasurement::new(ungoverned.path()))
            .expect("the store is readable")
            .is_empty(),
        "nothing was written"
    );
}
