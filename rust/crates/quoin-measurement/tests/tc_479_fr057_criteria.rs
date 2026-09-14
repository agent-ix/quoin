// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every FR-057 criterion the deleted `tests/intervention.test.ts` carried,
//! restated against this crate.
//!
//! Trace: FR-057-AC-1, FR-057-AC-2, FR-057-AC-3, FR-057-AC-4, FR-057-AC-5
//! Trace: FR-057-AC-6, FR-057-AC-7, FR-057-AC-8, FR-057-AC-9, FR-057-AC-10
//! Trace: FR-057-AC-11, FR-057-CON-1, FR-057-CON-2, FR-057-CON-3
//! Provenance: quoin#479
//!
//! # Why this file exists
//!
//! `tests/intervention.test.ts` is deleted in the same commit as
//! `src/measurement/intervention.ts`, `intervention-report.ts` and
//! `agent-eval-intervention.ts` (FR-101). Six of its `test` blocks carried
//! FR-057 criteria, several of them four at a time. `tc_471_corpus.rs` proves
//! this crate reproduces the retained store byte for byte and `tc_473_reporting`
//! proves the renderers agree with the captured TypeScript, but both are tagged
//! `FR-100`/`FR-101` — a corpus-wide agreement gate is not a per-criterion one.
//! This file is: one test per criterion, named by the criterion, so a criterion
//! cannot be lost without a named test disappearing with it.
//!
//! # Where the inputs come from
//!
//! `spec/evidence/interventions/quoin-270-cli-eval-sentinel-contract.json` and
//! the two retained `cli-agent-evals` runs under `spec/evidence/agent-evals/` —
//! committed bytes this test did not write. The temporary repository each test
//! builds is those bytes plus one authored `MeasurementPlan` that governs the
//! definition version the retained record already names. Every refusal case is
//! that committed base case, mutated **here**, in Rust; nothing compares the
//! port against bytes the port produced, and no process is spawned —
//! `tc_479_211` asserts over this crate's own sources that none can be.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

#[path = "common/census.rs"]
mod census;
#[path = "common/paths.rs"]
mod paths;

use census::rust_files;
use paths::repo_root;

use std::path::{Path, PathBuf};

use quoin_measurement::intervention::record::InterventionExperimentRecord;
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::report::{
    build_measurement_report, build_measurement_report_from, render_measurement_report,
    render_measurement_report_json,
};
use quoin_measurement::{
    DiskMeasurement, InterventionIntakeError, InterventionRefusalCode, read_intervention_records,
    read_measurement_collections, write_intervention_record,
};
use serde_json::{Value, json};
use tempfile::TempDir;

// ------------------------------------------------------ the committed inputs

/// The definition version the retained record names, and the one the authored
/// plan below governs.
const DEFINITION_VERSION: &str = "cli-agent-evals.sentinel-contract-v1";

/// The file name the writer chooses for the retained record's identity. The
/// `p-` prefix is `intervention.ts:39-45`'s portable namespace; the retained
/// record predates it (quoin#486, held by `tc_471_corpus`).
const PUBLISHED_BASENAME: &str = "p-quoin-270-cli-eval-sentinel-contract.json";

/// The retained record's committed bytes.
fn retained_bytes() -> Vec<u8> {
    let path = repo_root()
        .join("spec")
        .join("evidence")
        .join("interventions")
        .join("quoin-270-cli-eval-sentinel-contract.json");
    std::fs::read(&path).unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()))
}

/// The retained record as JSON.
fn retained_record() -> Value {
    serde_json::from_slice(&retained_bytes()).expect("the retained record is JSON")
}

/// A repository holding the two retained agent-eval runs and one active plan
/// that governs the retained record's definition version.
///
/// Nothing is written into `spec/evidence/interventions/`: every test that
/// needs a retained entry publishes one through the intake under test.
fn governed_repo() -> TempDir {
    let temporary = tempfile::tempdir().expect("a temporary repository");
    let root = temporary.path();
    let assurance = root.join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("the plan directory");
    std::fs::write(
        assurance.join("MP-901.md"),
        format!(
            "---\ntype: MeasurementPlan\nid: MP-901\ntitle: Sentinel contract\nstatus: active\n\
             owner: engineering assurance\nstage: branch-comparison\n\
             metric: intervention.sentinel\ndefinition_version: \"{DEFINITION_VERSION}\"\n\
             ---\n\n# Sentinel contract\n"
        ),
    )
    .expect("the plan is written");

    let evals = root.join("spec").join("evidence").join("agent-evals");
    std::fs::create_dir_all(&evals).expect("the evidence directory");
    let source = repo_root()
        .join("spec")
        .join("evidence")
        .join("agent-evals");
    for name in [
        "quoin-270-sentinel-baseline.json",
        "quoin-270-sentinel-treatment.json",
        "quoin-270-sentinel-definition.json",
    ] {
        std::fs::copy(source.join(name), evals.join(name))
            .unwrap_or_else(|error| panic!("{name} is copied: {error}"));
    }
    temporary
}

/// Submit a candidate through the FR-057 intake.
fn publish(repo: &Path, candidate: &Value) -> Result<PathBuf, InterventionIntakeError> {
    let stored = from_serde(candidate).expect("the candidate crosses the JSON bridge");
    write_intervention_record(repo, &stored)
}

/// Every retained record, as intake reads them back.
fn stored(repo: &Path) -> Vec<InterventionExperimentRecord> {
    read_intervention_records(&DiskMeasurement::new(repo)).expect("the store is readable")
}

/// The refusal a candidate earned, or a panic naming what was accepted instead.
fn refusal(repo: &Path, candidate: &Value, what: &str) -> InterventionIntakeError {
    match publish(repo, candidate) {
        Ok(path) => panic!(
            "{what}: accepted at {}, and the criterion refuses it",
            path.display()
        ),
        Err(error) => error,
    }
}

/// Where the retained record is published in `repo`.
fn published_path(repo: &Path) -> PathBuf {
    repo.join("spec")
        .join("evidence")
        .join("interventions")
        .join(PUBLISHED_BASENAME)
}

/// The temporaries a killed write would have left beside the published record.
fn leftovers(repo: &Path) -> Vec<String> {
    let directory = repo.join("spec").join("evidence").join("interventions");
    let Ok(entries) = std::fs::read_dir(&directory) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.contains(".tmp-"))
        .collect();
    names.sort();
    names
}

// ----------------------------------------------------------- report reading

/// One `#### …` section of a rendered intervention entry, without its heading.
fn section<'a>(text: &'a str, heading: &str) -> &'a str {
    let start = text
        .find(heading)
        .unwrap_or_else(|| panic!("the rendered report has no `{heading}` section"));
    let body = &text[start + heading.len()..];
    let end = [body.find("\n#### "), body.find("\n### ")]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(body.len());
    &body[..end]
}

/// The `trust.score` / `quality.score` / `overall.score` scan
/// `intervention.test.ts:389` performs, as a function rather than a regex: the
/// separator is any single character, and the match is case-insensitive.
fn aggregate_score_mention(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for word in ["trust", "quality", "overall"] {
        let mut from = 0;
        while let Some(offset) = lower.get(from..).and_then(|rest| rest.find(word)) {
            let start = from + offset;
            let after = start + word.len();
            if lower.get(after + 1..after + 6) == Some("score") {
                return lower.get(start..after + 6).map(str::to_owned);
            }
            from = start + 1;
        }
    }
    None
}

// ------------------------------------------------------------- the criteria

/// Valid intake writes one complete canonical record by one atomic
/// same-directory no-replace publication; a concurrent destination cannot be
/// overwritten.
///
/// Trace: FR-057-AC-1
/// Provenance: quoin#479
#[test]
fn tc_479_200_valid_intake_publishes_once_without_replacing_a_concurrent_writer() {
    let temporary = governed_repo();
    let repo = temporary.path();
    let record = retained_record();

    let path = publish(repo, &record).expect("the retained record is admissible");
    assert_eq!(
        path,
        published_path(repo),
        "the record lands under its identity"
    );
    assert_eq!(
        std::fs::read(&path).expect("the published record is readable"),
        retained_bytes(),
        "the published bytes are the committed canonical bytes, not a re-serialisation"
    );
    assert_eq!(stored(repo).len(), 1, "one publication, one retained entry");
    assert_eq!(
        leftovers(repo),
        Vec::<String>::new(),
        "the same-directory publication left no temporary behind"
    );

    // A concurrent writer that reached the destination first. The publication
    // links into place and never replaces, so what is already there survives.
    let second = governed_repo();
    let concurrent = published_path(second.path());
    std::fs::create_dir_all(concurrent.parent().expect("a parent directory"))
        .expect("the interventions directory");
    std::fs::write(&concurrent, b"{\"written\":\"by another writer\"}\n")
        .expect("the concurrent entry is written");
    let error = refusal(second.path(), &record, "a destination another writer holds");
    assert_eq!(error.code(), InterventionRefusalCode::RecordIdCollision);
    assert_eq!(
        std::fs::read(&concurrent).expect("the concurrent entry is readable"),
        b"{\"written\":\"by another writer\"}\n",
        "the concurrent writer's bytes were not overwritten"
    );
    assert_eq!(
        leftovers(second.path()),
        Vec::<String>::new(),
        "the refused publication left no temporary behind"
    );
}

/// Invalid schema or cross-record integrity input writes nothing and returns
/// `invalid_record` with every failing JSON path and reason.
///
/// Trace: FR-057-AC-2
/// Provenance: quoin#479
#[test]
fn tc_479_201_every_invalid_record_is_refused_with_its_paths_and_writes_nothing() {
    /// Below this the census is measuring one refusal and calling it a family.
    const CASE_FLOOR: usize = 8;

    let base = retained_record();

    // Each case is the committed record with one thing wrong: the first four are
    // schema failures, the last four are the cross-record integrity checks
    // `semanticFindings` owns and JSON Schema cannot state.
    let mut cases: Vec<(&str, Value)> = Vec::new();

    let mut undeclared = base.clone();
    undeclared["unowned"] = json!(true);
    cases.push(("an undeclared member", undeclared));

    for member in ["record_id", "observed_at", "subject", "producer"] {
        let mut missing = base.clone();
        missing
            .as_object_mut()
            .expect("a record object")
            .remove(member)
            .expect("the base case carries it");
        cases.push((member, missing));
    }

    let mut mutable_version = base.clone();
    mutable_version["producer"]["tool_version"] = json!("latest");
    cases.push(("a mutable tool_version", mutable_version));

    let mut duplicate_arm = base.clone();
    duplicate_arm["treatments"][0]["id"] = base["baseline"]["id"].clone();
    cases.push(("a treatment reusing the baseline arm id", duplicate_arm));

    let mut unresolved = base.clone();
    unresolved["changed_variables"][0]["treatment_id"] = json!("no-such-arm");
    cases.push(("a changed variable naming no arm", unresolved));

    let mut wrong_conclusion = base.clone();
    wrong_conclusion["status"] = json!("failed");
    wrong_conclusion["conclusion"]["kind"] = json!("no_effect_observed");
    cases.push((
        "a failed record claiming an observed effect",
        wrong_conclusion,
    ));

    assert!(
        cases.len() >= CASE_FLOOR,
        "anti-vacuity floor: at least {CASE_FLOOR} invalid cases expected, saw {}",
        cases.len()
    );

    for (what, candidate) in &cases {
        let temporary = governed_repo();
        let repo = temporary.path();
        let error = refusal(repo, candidate, what);
        assert_eq!(
            error.code(),
            InterventionRefusalCode::InvalidRecord,
            "{what}: refused as an invalid record"
        );
        assert!(
            !error.findings().is_empty(),
            "{what}: a refusal with no finding names no failing path"
        );
        for finding in error.findings() {
            assert!(
                finding.starts_with('/'),
                "{what}: `{finding}` is not a JSON path and a reason"
            );
        }
        assert!(
            stored(repo).is_empty(),
            "{what}: the store was written despite the refusal"
        );
        assert!(
            !published_path(repo).exists(),
            "{what}: a record file was created despite the refusal"
        );
    }
}

/// An absent governing plan returns `governing_plan_absent`; a mismatch returns
/// `definition_mismatch`; both write nothing and name the requested, expected
/// and observed definitions that apply.
///
/// Trace: FR-057-AC-3
/// Provenance: quoin#479
#[test]
fn tc_479_202_an_absent_and_a_mismatched_governing_definition_are_different_refusals() {
    let record = retained_record();

    // No plan at all: the refusal names what was requested and says no active
    // plan exists. A repository with the evidence but no `spec/assurance`.
    let ungoverned = governed_repo();
    std::fs::remove_dir_all(ungoverned.path().join("spec").join("assurance"))
        .expect("the plan directory is removable");
    let absent = refusal(ungoverned.path(), &record, "a repository with no plan");
    assert_eq!(absent.code(), InterventionRefusalCode::GoverningPlanAbsent);
    assert_eq!(
        absent.findings(),
        [format!(
            "requested definition {DEFINITION_VERSION}; no active MeasurementPlan exists"
        )],
        "the refusal names the requested definition"
    );
    assert!(stored(ungoverned.path()).is_empty(), "nothing was written");

    // A plan exists but governs something else: the refusal names expected and
    // observed, which is a different fact from "there is no plan".
    let governed = governed_repo();
    let mut mismatched = record;
    mismatched["producer"]["definition_version"] = json!("different-v1");
    let error = refusal(governed.path(), &mismatched, "a definition no plan governs");
    assert_eq!(error.code(), InterventionRefusalCode::DefinitionMismatch);
    assert_eq!(
        error.findings(),
        [format!(
            "requested definition different-v1; expected one of {DEFINITION_VERSION}; \
             observed different-v1"
        )],
        "the refusal names the expected and the observed definition"
    );
    assert!(stored(governed.path()).is_empty(), "nothing was written");
}

/// Repeating an identical record id and canonical payload is byte-idempotent.
///
/// Trace: FR-057-AC-4
/// Provenance: quoin#479
#[test]
fn tc_479_203_repeating_an_identical_record_is_byte_idempotent() {
    let temporary = governed_repo();
    let repo = temporary.path();
    let record = retained_record();

    let first = publish(repo, &record).expect("the retained record is admissible");
    let before = std::fs::read(&first).expect("the published record is readable");
    let again = publish(repo, &record).expect("republishing identical bytes is idempotent");

    assert_eq!(first, again, "the repeat lands at the same path");
    assert_eq!(
        std::fs::read(&again).expect("the republished record is readable"),
        before,
        "the repeat did not change a byte"
    );
    assert_eq!(
        before,
        retained_bytes(),
        "and the bytes are the committed ones"
    );
    assert_eq!(stored(repo).len(), 1, "the repeat did not add an entry");
    assert_eq!(
        leftovers(repo),
        Vec::<String>::new(),
        "no temporary was left"
    );
}

/// Reusing a record id for different semantic bytes returns
/// `record_id_collision` without replacing the retained entry.
///
/// Trace: FR-057-AC-5
/// Provenance: quoin#479
#[test]
fn tc_479_204_a_reused_record_id_with_different_bytes_does_not_replace_the_entry() {
    let temporary = governed_repo();
    let repo = temporary.path();
    let record = retained_record();

    let path = publish(repo, &record).expect("the retained record is admissible");
    let before = std::fs::read(&path).expect("the published record is readable");

    let mut collision = record;
    collision["owner"] = json!("different-owner");
    let error = refusal(repo, &collision, "a reused record id carrying other bytes");
    assert_eq!(error.code(), InterventionRefusalCode::RecordIdCollision);
    assert_eq!(
        error.findings(),
        [format!(
            "{}: record id already exists with different canonical bytes",
            path.display()
        )],
        "the collision names the entry it refused to replace"
    );

    assert_eq!(
        std::fs::read(&path).expect("the retained record is readable"),
        before,
        "the retained entry was not replaced"
    );
    let retained = stored(repo);
    assert_eq!(retained.len(), 1, "the collision added no second entry");
    assert_eq!(
        retained[0].owner, "engineering assurance",
        "the retained entry still carries its own owner"
    );
}

/// Completed, failed, inconclusive and `cause_not_established` records remain
/// independently queryable with their raw-evidence digests.
///
/// Trace: FR-057-AC-6
/// Provenance: quoin#479
#[test]
fn tc_479_205_every_terminal_result_stays_independently_queryable() {
    /// The three terminal statuses the criterion names.
    const STATUSES: [&str; 3] = ["completed", "failed", "inconclusive"];
    /// The digests the committed record rests on, in path order.
    const DIGESTS: [&str; 2] = [
        "sha256:741ab150c107d1f5551a9be2092081cc0439f85e804d23f81e3923dfab8fe076",
        "sha256:baade4ab1c2e8f3447b4c585c95634884cb2ed1c69a9cf8819d0b495c7f77963",
    ];

    let temporary = governed_repo();
    let repo = temporary.path();
    let base = retained_record();

    for (index, status) in STATUSES.iter().enumerate() {
        let mut record = base.clone();
        record["record_id"] = json!(format!("experiment-{index}"));
        record["status"] = json!(status);
        publish(repo, &record)
            .unwrap_or_else(|error| panic!("a {status} record is admissible: {error}"));
    }

    let retained = stored(repo);
    assert_eq!(
        retained.len(),
        STATUSES.len(),
        "each terminal result is retained as its own entry"
    );
    assert!(
        !STATUSES.is_empty(),
        "a census over no statuses measures nothing"
    );
    for (record, status) in retained.iter().zip(STATUSES) {
        assert_eq!(record.status.as_str(), status, "the status is queryable");
        // The conclusion the committed record carries is the terminal one the
        // criterion names beside the three statuses.
        assert_eq!(record.conclusion.kind.as_str(), "cause_not_established");
        let digests: Vec<&str> = record
            .raw_evidence
            .iter()
            .map(|reference| reference.digest.as_str())
            .collect();
        assert_eq!(
            digests, DIGESTS,
            "the {status} record is queryable with its raw-evidence digests"
        );
    }
}

/// The report renders conclusion statements as claims and measured effects plus
/// raw references as evidence.
///
/// Trace: FR-057-AC-7
/// Provenance: quoin#479
#[test]
fn tc_479_206_conclusions_render_as_claims_and_measurements_as_evidence() {
    /// The conclusion the committed record states, verbatim.
    const STATEMENT: &str = "The retained agent-evaluation runs show no observed pass-rate \
                             difference; this adapter does not establish causality.";
    /// The one measured effect, as `intervention-report.ts:76-80` writes it.
    const EFFECT_LINE: &str = "- standalone-detector/agent-eval.ea-001.pass-rate: baseline 0.5, \
                               treatment 0.5, effect 0 fraction";
    /// The first raw reference, as `intervention-report.ts:82-86` writes it.
    const RAW_LINE: &str = "- agent-evals/quoin-270-sentinel-baseline.json — \
                            sha256:741ab150c107d1f5551a9be2092081cc0439f85e804d23f81e3923dfab8fe076; \
                            11305 bytes; application/json";

    let temporary = governed_repo();
    let repo = temporary.path();
    publish(repo, &retained_record()).expect("the retained record is admissible");

    let report = build_measurement_report(&DiskMeasurement::new(repo), repo)
        .expect("the report builds over the retained record");
    let human = render_measurement_report(&report).expect("the report renders");
    let json: Value = serde_json::from_str(
        &render_measurement_report_json(&report).expect("the JSON view renders"),
    )
    .expect("the JSON view is JSON");

    let claims = section(&human, "#### Claims");
    assert!(
        claims.contains(STATEMENT),
        "the conclusion statement is the claim: {claims:?}"
    );

    let evidence = section(&human, "#### Evidence");
    assert!(
        evidence.contains(EFFECT_LINE),
        "the measured effect is evidence: {evidence:?}"
    );
    assert!(
        evidence.contains(RAW_LINE),
        "the raw reference is evidence: {evidence:?}"
    );
    assert!(
        !evidence.contains(STATEMENT),
        "the conclusion is a claim, not evidence"
    );
    assert!(
        !claims.contains(EFFECT_LINE) && !claims.contains(RAW_LINE),
        "a measured effect and a raw reference are evidence, not claims"
    );

    let entry = &json["interventions"][0];
    assert_eq!(
        entry["claims"],
        json!([STATEMENT]),
        "the JSON view carries the conclusion statement and nothing else as a claim"
    );
    assert_eq!(
        entry["evidence"]["measured_effects"]
            .as_array()
            .map(Vec::len),
        Some(1),
        "the JSON view carries the measured effect under evidence"
    );
    assert_eq!(
        entry["evidence"]["raw_evidence"][0]["digest"],
        json!("sha256:741ab150c107d1f5551a9be2092081cc0439f85e804d23f81e3923dfab8fe076"),
        "the JSON view carries the raw reference under evidence"
    );
}

/// Uncontrolled or unknown interactions and confounders render as
/// counterevidence; declared gaps render separately beside owner and actions.
///
/// Trace: FR-057-AC-8
/// Provenance: quoin#479
#[test]
fn tc_479_207_open_qualifiers_render_as_counterevidence_and_gaps_render_apart() {
    /// The three open qualifiers the committed record declares, in the order
    /// `buildInterventionReport` sorts them: kind, then description.
    const COUNTEREVIDENCE: [&str; 3] = [
        "- confounder: session state and model-side nondeterminism between sequential runs \
         (unknown)",
        "- confounder: the treatment timeout followed omission of the required final sentinel \
         after the task result was written (uncontrolled)",
        "- interaction: live agent behavior and tool-selection order (uncontrolled)",
    ];

    let temporary = governed_repo();
    let repo = temporary.path();
    let base = retained_record();
    publish(repo, &base).expect("the retained record is admissible");

    let report =
        build_measurement_report(&DiskMeasurement::new(repo), repo).expect("the report builds");
    let human = render_measurement_report(&report).expect("the report renders");

    let counterevidence = section(&human, "#### Counterevidence");
    for line in COUNTEREVIDENCE {
        assert!(
            counterevidence.contains(line),
            "an open qualifier is missing from counterevidence: {line}"
        );
    }
    assert!(
        !COUNTEREVIDENCE.is_empty(),
        "a counterevidence census over nothing measures nothing"
    );

    // Gaps, owner and actions are their own sections beside it, and no gap
    // leaked into the counterevidence list.
    let gaps_section = section(&human, "#### Gaps");
    let declared_gaps = base["gaps"].as_array().expect("the record declares gaps");
    assert!(
        declared_gaps.len() >= 5,
        "the committed record declares {} gaps; this assertion needs them",
        declared_gaps.len()
    );
    for gap in declared_gaps {
        let gap = gap.as_str().expect("a gap is text");
        assert!(
            gaps_section.contains(gap),
            "a declared gap is missing: {gap}"
        );
        assert!(
            !counterevidence.contains(gap),
            "a gap rendered as counterevidence: {gap}"
        );
    }
    assert_eq!(
        section(&human, "#### Owner").trim(),
        "engineering assurance",
        "the owner is its own section"
    );
    let actions = section(&human, "#### Actions");
    for action in base["actions"]
        .as_array()
        .expect("the record declares actions")
    {
        let action = action.as_str().expect("an action is text");
        assert!(
            actions.contains(action),
            "a declared action is missing: {action}"
        );
        assert!(
            !gaps_section.contains(action),
            "an action rendered as a gap: {action}"
        );
    }

    // The mirror case: a qualifier that *was* controlled is not counterevidence,
    // so the list above is a disposition test and not a copy of two fields.
    let controlled = governed_repo();
    let mut closed = base;
    closed["interactions"][0]["disposition"] = json!("controlled");
    closed["confounders"][0]["disposition"] = json!("controlled");
    closed["confounders"][1]["disposition"] = json!("not_applicable");
    publish(controlled.path(), &closed).expect("a record with closed qualifiers is admissible");
    let closed_report =
        build_measurement_report(&DiskMeasurement::new(controlled.path()), controlled.path())
            .expect("the report builds");
    assert_eq!(
        section(
            &render_measurement_report(&closed_report).expect("the report renders"),
            "#### Counterevidence"
        )
        .trim(),
        "No declared counterevidence.",
        "a controlled or inapplicable qualifier is not counterevidence"
    );
}

/// Neither human nor JSON output contains an aggregate trust, confidence or
/// quality score derived from experiment records.
///
/// Trace: FR-057-AC-9, FR-057-CON-2
/// Provenance: quoin#479
#[test]
fn tc_479_208_no_view_computes_an_aggregate_trust_or_quality_score() {
    let temporary = governed_repo();
    let repo = temporary.path();
    publish(repo, &retained_record()).expect("the retained record is admissible");

    let report =
        build_measurement_report(&DiskMeasurement::new(repo), repo).expect("the report builds");
    let human = render_measurement_report(&report).expect("the report renders");
    let rendered_json = render_measurement_report_json(&report).expect("the JSON view renders");

    // The scan itself has to be able to fire, or it measures nothing.
    assert_eq!(
        aggregate_score_mention("the overall score was 0.9"),
        Some("overall score".to_owned()),
        "the scan does not detect the thing it is looking for"
    );

    for (view, text) in [("human", &human), ("json", &rendered_json)] {
        assert_eq!(
            aggregate_score_mention(text),
            None,
            "the {view} view states an aggregate score"
        );
    }

    // The JSON view carries the record's own `attribution_confidence` nowhere
    // above the record: there is no report-level confidence member.
    let json: Value = serde_json::from_str(&rendered_json).expect("the JSON view is JSON");
    for member in ["trust", "confidence", "quality", "score"] {
        assert!(
            json.as_object()
                .expect("a report object")
                .get(member)
                .is_none(),
            "the report object states a top-level `{member}`"
        );
    }
}

/// Re-rendering an unchanged store is byte-identical, and the human and JSON
/// views expose the same claims, evidence, counterevidence, gaps, owners and
/// actions.
///
/// Trace: FR-057-AC-10
/// Provenance: quoin#479
#[test]
fn tc_479_209_a_reordered_store_renders_byte_identically_in_both_views() {
    let temporary = governed_repo();
    let repo = temporary.path();
    let base = retained_record();

    // Three retained records, so the projection has an order to get wrong.
    let mut records = Vec::new();
    for (index, observed_at) in [
        "2026-08-30T17:54:41.378Z",
        "2026-08-29T09:00:00.000Z",
        "2026-08-31T23:59:59.999Z",
    ]
    .into_iter()
    .enumerate()
    {
        let mut record = base.clone();
        record["record_id"] = json!(format!("experiment-{index}"));
        record["observed_at"] = json!(observed_at);
        publish(repo, &record).expect("each record is admissible");
        records.push(record);
    }
    let read_back = stored(repo);
    assert_eq!(read_back.len(), 3, "three retained records");

    // Every ordering of the store's input renders the same bytes: the report is
    // a function of what is retained, not of the order it arrived in.
    let expected_human = render_measurement_report(
        &build_measurement_report_from(repo, &[], &[], &read_back, &[]).expect("the report builds"),
    )
    .expect("the report renders");
    let expected_json = render_measurement_report_json(
        &build_measurement_report_from(repo, &[], &[], &read_back, &[]).expect("the report builds"),
    )
    .expect("the JSON view renders");

    // Anti-vacuity: permutation-invariance alone is satisfied by a renderer
    // that emits the same empty string for every ordering. Anchor the expected
    // bytes to literals first, so the loop below compares against a report that
    // is known to say something, and to say it in one deterministic order.
    assert!(
        expected_human.contains("### experiment-0\n")
            && expected_human.contains("### experiment-1\n")
            && expected_human.contains("### experiment-2\n"),
        "the human view must carry a heading per retained record"
    );
    let headings: Vec<&str> = expected_human
        .match_indices("### experiment-")
        .map(|(at, _)| &expected_human[at + 4..at + 16])
        .collect();
    assert_eq!(
        headings,
        vec!["experiment-1", "experiment-0", "experiment-2"],
        "the retained records render in observation order, oldest first"
    );

    let mut orderings = 0_usize;
    for first in 0..3 {
        for second in 0..3 {
            for third in 0..3 {
                let order = [first, second, third];
                if order
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != 3
                {
                    continue;
                }
                let permuted: Vec<InterventionExperimentRecord> = order
                    .iter()
                    .map(|index| read_back[*index].clone())
                    .collect();
                let report = build_measurement_report_from(repo, &[], &[], &permuted, &[])
                    .expect("the report builds");
                assert_eq!(
                    render_measurement_report(&report).expect("the report renders"),
                    expected_human,
                    "ordering {order:?} changed the rendered report"
                );
                assert_eq!(
                    render_measurement_report_json(&report).expect("the JSON view renders"),
                    expected_json,
                    "ordering {order:?} changed the JSON view"
                );
                orderings += 1;
            }
        }
    }
    assert_eq!(orderings, 6, "every ordering of three records is tried");
    views_agree(&expected_human, &expected_json);
}

/// The two views expose the same six things, entry for entry.
fn views_agree(expected_human: &str, expected_json: &str) {
    let json: Value = serde_json::from_str(expected_json).expect("the JSON view is JSON");
    let entries = json["interventions"].as_array().expect("the JSON entries");
    assert_eq!(entries.len(), 3, "the JSON view carries every entry");
    for entry in entries {
        let record_id = entry["record_id"].as_str().expect("a record id");
        let start = expected_human
            .find(&format!("### {record_id}\n"))
            .unwrap_or_else(|| panic!("the human view has no entry for {record_id}"));
        let rendered = &expected_human[start..];
        let texts = |member: &str| -> Vec<String> {
            entry[member]
                .as_array()
                .unwrap_or_else(|| panic!("{record_id}: {member} is a list"))
                .iter()
                .map(|item| item.as_str().expect("text").to_owned())
                .collect()
        };
        for claim in texts("claims") {
            assert!(
                section(rendered, "#### Claims").contains(&claim),
                "{record_id}: {claim}"
            );
        }
        for gap in texts("gaps") {
            assert!(
                section(rendered, "#### Gaps").contains(&gap),
                "{record_id}: {gap}"
            );
        }
        for action in texts("actions") {
            assert!(
                section(rendered, "#### Actions").contains(&action),
                "{record_id}: {action}"
            );
        }
        assert_eq!(
            section(rendered, "#### Owner").trim(),
            entry["owner"].as_str().expect("an owner"),
            "{record_id}: the owner"
        );
        let counterevidence = section(rendered, "#### Counterevidence");
        let declared = entry["counterevidence"]
            .as_array()
            .expect("counterevidence");
        assert_eq!(declared.len(), 3, "{record_id}: the open qualifiers");
        for item in declared {
            assert!(
                counterevidence.contains(item["description"].as_str().expect("a description")),
                "{record_id}: counterevidence"
            );
        }
        let evidence = section(rendered, "#### Evidence");
        for raw in entry["evidence"]["raw_evidence"]
            .as_array()
            .expect("raw evidence")
        {
            assert!(
                evidence.contains(raw["path"].as_str().expect("a path")),
                "{record_id}: raw evidence"
            );
        }
    }
}

/// Unsafe, missing, wrong-sized or digest-mismatched raw evidence returns
/// `raw_evidence_mismatch`, identifies each mismatch, and writes no record.
///
/// Trace: FR-057-AC-11
/// Provenance: quoin#479
#[test]
fn tc_479_210_every_raw_evidence_disagreement_is_named_and_writes_nothing() {
    /// Below this the census is measuring one guard and calling it four.
    const CASE_FLOOR: usize = 5;

    let base = retained_record();
    let mut cases: Vec<(&str, Value, Vec<&str>)> = Vec::new();

    let mut escape = base.clone();
    escape["raw_evidence"][0]["path"] = json!("../outside.json");
    cases.push((
        "a path leaving the store",
        escape,
        vec!["/raw_evidence/0/path"],
    ));

    let mut unnormalised = base.clone();
    unnormalised["raw_evidence"][0]["path"] =
        json!("agent-evals/./quoin-270-sentinel-baseline.json");
    cases.push((
        "a path that is not normalised",
        unnormalised,
        vec!["/raw_evidence/0/path"],
    ));

    let mut absent = base.clone();
    absent["raw_evidence"][1]["path"] = json!("agent-evals/never-retained.json");
    cases.push((
        "a path naming no retained file",
        absent,
        vec!["/raw_evidence/1/path"],
    ));

    let mut wrong_size = base.clone();
    wrong_size["raw_evidence"][0]["size_bytes"] = json!(11306);
    cases.push((
        "a recorded size the file does not have",
        wrong_size,
        vec!["/raw_evidence/0/size_bytes"],
    ));

    let mut wrong_digest = base.clone();
    wrong_digest["raw_evidence"][1]["digest"] = json!(format!("sha256:{}", "f".repeat(64)));
    cases.push((
        "a recorded digest the file does not have",
        wrong_digest,
        vec!["/raw_evidence/1/digest"],
    ));

    // Both at once, on different references: each mismatch is identified, not
    // just the first.
    let mut both = base;
    both["raw_evidence"][0]["size_bytes"] = json!(11306);
    both["raw_evidence"][1]["digest"] = json!(format!("sha256:{}", "f".repeat(64)));
    cases.push((
        "a size mismatch and a digest mismatch together",
        both,
        vec!["/raw_evidence/0/size_bytes", "/raw_evidence/1/digest"],
    ));

    assert!(
        cases.len() >= CASE_FLOOR,
        "anti-vacuity floor: at least {CASE_FLOOR} raw-evidence cases expected, saw {}",
        cases.len()
    );

    for (what, candidate, expected) in &cases {
        let temporary = governed_repo();
        let repo = temporary.path();
        let error = refusal(repo, candidate, what);
        assert_eq!(
            error.code(),
            InterventionRefusalCode::RawEvidenceMismatch,
            "{what}: refused as a raw-evidence mismatch"
        );
        assert_eq!(
            error.findings().len(),
            expected.len(),
            "{what}: each mismatch is identified once: {:?}",
            error.findings()
        );
        for pointer in expected {
            assert!(
                error
                    .findings()
                    .iter()
                    .any(|finding| finding.starts_with(pointer)),
                "{what}: no finding identifies {pointer}: {:?}",
                error.findings()
            );
        }
        assert!(stored(repo).is_empty(), "{what}: the store was written");
        assert!(
            !published_path(repo).exists(),
            "{what}: a record file was created"
        );
    }
}

/// Quoin does not execute an experiment or producer while recording or
/// reporting evidence.
///
/// Trace: FR-057-CON-1
/// Provenance: quoin#479
#[test]
fn tc_479_211_the_intervention_modules_reach_for_no_process_or_network() {
    /// What no intervention module may reach for. `intervention.test.ts:404`
    /// scanned for `node:child_process` and `fetch(`; these are the Rust
    /// spellings of the same two capabilities, plus the filesystem, which the
    /// intervention layer also does not own — reads go through
    /// [`quoin_measurement::source::MeasurementSource`].
    const FORBIDDEN: [&str; 7] = [
        "std::process",
        "std::net",
        "std::fs",
        "Command::new",
        "TcpStream",
        "reqwest",
        "include_str!",
    ];

    /// Below this the census is reading an empty tree and proving nothing.
    const SOURCE_FLOOR: usize = 8;

    let modules = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("intervention");
    let mut measured = 0_usize;
    for path in rust_files(&modules) {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: unreadable: {error}", path.display()));
        for needle in FORBIDDEN {
            assert!(
                !text.contains(needle),
                "{}: an intervention module reaches for `{needle}`",
                path.display()
            );
        }
        measured += 1;
    }
    assert!(
        measured >= SOURCE_FLOOR,
        "the census read {measured} intervention modules, below the floor of {SOURCE_FLOOR}"
    );
}

/// Existing measurement collections and pre-experiment evidence remain readable
/// without migration.
///
/// Trace: FR-057-CON-3
/// Provenance: quoin#479
#[test]
fn tc_479_212_an_empty_intervention_store_preserves_measurement_reporting() {
    // A repository that has never held an intervention: the store reads as
    // empty rather than refusing, and the report still builds.
    let temporary = governed_repo();
    let repo = temporary.path();
    assert!(
        !repo
            .join("spec")
            .join("evidence")
            .join("interventions")
            .exists(),
        "this case needs a repository with no interventions directory"
    );
    let report = build_measurement_report(&DiskMeasurement::new(repo), repo)
        .expect("a report builds over a store with no interventions");
    assert!(
        report.interventions.is_empty(),
        "there is nothing to report"
    );
    assert!(
        render_measurement_report(&report)
            .expect("the report renders")
            .contains("# QA measurement report"),
        "measurement reporting is preserved"
    );

    // Pre-experiment evidence: the committed fixture tree's `delta` repository
    // holds a measurement collection authored before intervention records
    // existed, and no interventions directory at all. It is read as it stands.
    let legacy = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("portfolio-tree")
        .join("delta");
    let source = DiskMeasurement::new(&legacy);
    let collections =
        read_measurement_collections(&source).expect("the pre-experiment collections are readable");
    assert_eq!(
        collections.len(),
        1,
        "the legacy repository's collection is read without migration"
    );
    assert!(
        read_intervention_records(&source)
            .expect("a repository with no interventions directory reads as empty")
            .is_empty(),
        "there is no intervention evidence to read"
    );
    let legacy_report =
        build_measurement_report(&source, &legacy).expect("the legacy repository still reports");
    assert!(
        !legacy_report.current.is_empty(),
        "the legacy measurement rows survive an empty intervention store"
    );
    assert!(
        legacy_report.current[0].observation.is_some(),
        "the legacy observation is still readable"
    );
}
