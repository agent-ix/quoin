// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every FR-059 acceptance criterion the deleted `tests/operational.test.ts`
//! carried, restated against this crate.
//!
//! Trace: FR-059-AC-1, FR-059-AC-2, FR-059-AC-3, FR-059-AC-4, FR-059-AC-5
//! Trace: FR-059-AC-6, FR-059-AC-8, FR-059-AC-9, FR-059-AC-10
//! Provenance: quoin#479
//!
//! # Where the inputs come from
//!
//! `common/release_evidence.rs` builds synthetic GitHub exports and a governing
//! plan into a temporary repository; the producer writes a pair from them, and
//! that pair's capability and exercise are the valid base cases. Every refusal
//! case is that base mutated in Rust, below. Nothing here writes into this
//! repository.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

#[path = "common/paths.rs"]
mod paths;
#[path = "common/release_evidence.rs"]
mod release_evidence;

use paths::repo_root;
use release_evidence::write_repository;

use engineering_assurance::claim_strength::ClaimStrength;
use quoin_jsonschema::MeasurementSchema;

use quoin_measurement::intervention::intake::{InterventionIntakeError, InterventionRefusalCode};
use quoin_measurement::operational::intake::write_operational_record;
use quoin_measurement::operational::read::read_operational_records;
use quoin_measurement::operational::record::{
    ExerciseOutcome, OperationalControlKind, OperationalEvidenceRecord,
};
use quoin_measurement::operational::validate::{
    ValidOperationalRecord, validate_operational_record,
};
use quoin_measurement::source::SystemClock;
use serde_json::{Value, json};

/// The requirement the schema under test is authored in.
const REQUIREMENT: &str = "spec/functional/FR-059-operational-evidence-records.md";

// ------------------------------------------------------------ the base cases

/// The record of one shape, from a pair the producer writes from synthetic
/// exports.
fn retained(shape: &str) -> Value {
    let workspace = tempfile::tempdir().expect("a writable workspace");
    write_repository(workspace.path());
    let definition = serde_json::from_value(release_evidence::definition_document())
        .expect("the definition reads");
    let produced =
        quoin_measurement::operational::github_release::produce_github_release_operational(
            workspace.path(),
            &SystemClock,
            &definition,
        )
        .expect("the synthetic exports produce a pair");
    let text = std::fs::read_to_string(&produced.path).expect("the pair is readable");
    let envelope: Value = serde_json::from_str(&text).expect("the pair is JSON");
    let records = envelope["records"]
        .as_array()
        .expect("the pair carries records");
    assert_eq!(
        records.len(),
        2,
        "a pair is one capability and one exercise"
    );
    records
        .iter()
        .find(|record| record["record_shape"] == json!(shape))
        .unwrap_or_else(|| panic!("the pair carries no `{shape}` record"))
        .clone()
}

/// The standing-capability record.
fn capability() -> Value {
    retained("standing_capability")
}

/// The exercise record.
fn exercise() -> Value {
    retained("exercise")
}

// ------------------------------------------------------------- the mutations

/// The object at a path of member names, panicking if it is not there.
fn object_at<'a>(value: &'a mut Value, path: &[&str]) -> &'a mut serde_json::Map<String, Value> {
    let mut cursor = value;
    for step in path {
        cursor = cursor
            .get_mut(*step)
            .unwrap_or_else(|| panic!("the base case has no `{step}`"));
    }
    cursor
        .as_object_mut()
        .unwrap_or_else(|| panic!("`{}` is not an object", path.join("/")))
}

/// The base case without the member the path names.
fn without(base: &Value, path: &[&str]) -> Value {
    let mut value = base.clone();
    let (last, parents) = path.split_last().expect("a non-empty path");
    let object = object_at(&mut value, parents);
    assert!(
        object.remove(*last).is_some(),
        "the base case carries no `{}` to delete",
        path.join("/")
    );
    value
}

/// The base case with the member the path names replaced.
fn with(base: &Value, path: &[&str], replacement: Value) -> Value {
    let mut value = base.clone();
    let (last, parents) = path.split_last().expect("a non-empty path");
    object_at(&mut value, parents).insert((*last).to_owned(), replacement);
    value
}

// ---------------------------------------------------------------- the verdict

/// The record a candidate reads as, or a panic carrying the refusal.
fn admitted(candidate: &Value, what: &str) -> ValidOperationalRecord {
    validate_operational_record(candidate)
        .unwrap_or_else(|error| panic!("{what}: refused, and it is valid — {:?}", error.findings()))
}

/// The refusal a candidate draws, or a panic naming what was admitted instead.
fn refusal(candidate: &Value, what: &str) -> InterventionIntakeError {
    match validate_operational_record(candidate) {
        Ok(_) => panic!("{what}: admitted, and the criterion says it is refused"),
        Err(error) => {
            assert_eq!(
                error.code(),
                InterventionRefusalCode::InvalidRecord,
                "{what}: refused under the wrong code"
            );
            error
        }
    }
}

/// A candidate is refused, and some finding names the thing that is wrong with
/// it — so a test cannot pass on an unrelated refusal.
fn refused_naming(candidate: &Value, needle: &str, what: &str) {
    let error = refusal(candidate, what);
    assert!(
        error
            .findings()
            .iter()
            .any(|finding| finding.contains(needle)),
        "{what}: no finding names {needle:?}; the findings were {:?}",
        error.findings()
    );
}

/// The exact sentence the schema pass writes for an absent required member,
/// pinned to the object it is absent from — so a deletion test cannot pass on
/// a refusal about some other member of some other object.
fn required(pointer: &str, member: &str) -> String {
    format!("{pointer}: {member:?} is a required property")
}

// -------------------------------------------------------------------- the ACs

/// The envelope requires version, record and subject identities, observation
/// timestamp, deployed scope, shape, control kind, governance fields, raw
/// evidence, and every unchanged FR-044 producer-tuple field.
///
/// Trace: FR-059-AC-1
/// Provenance: quoin#479
#[test]
fn tc_479_300_the_envelope_requires_every_declared_member() {
    /// The thirteen members both shapes carry.
    const ENVELOPE: [&str; 13] = [
        "schema_version",
        "record_type",
        "record_id",
        "observed_at",
        "record_shape",
        "control_kind",
        "subject",
        "producer",
        "scope",
        "owner",
        "gaps",
        "actions",
        "raw_evidence",
    ];
    /// The FR-044 producer tuple, unchanged.
    const PRODUCER: [&str; 4] = [
        "tool_identity",
        "tool_version",
        "environment",
        "definition_version",
    ];

    let base = capability();
    admitted(&base, "the retained capability");

    // Each member independently: a record missing exactly one is refused, and
    // the refusal names the member that is gone.
    assert_eq!(ENVELOPE.len(), 13, "the envelope census is the criterion's");
    for member in ENVELOPE {
        refused_naming(
            &without(&base, &[member]),
            &required("/", member),
            &format!("an envelope missing `{member}`"),
        );
    }
    assert_eq!(
        PRODUCER.len(),
        4,
        "the FR-044 producer tuple is four fields"
    );
    for member in PRODUCER {
        refused_naming(
            &without(&base, &["producer", member]),
            &required("/producer", member),
            &format!("a producer missing `{member}`"),
        );
    }
    refused_naming(
        &without(&base, &["subject", "id"]),
        &required("/subject", "id"),
        "a subject missing `id`",
    );
    for member in ["service", "environment", "population"] {
        refused_naming(
            &without(&base, &["scope", member]),
            &required("/scope", member),
            &format!("a deployed scope missing `{member}`"),
        );
    }

    // The observation timestamp is an instant, not a date and not a day that
    // does not exist.
    for observed_at in ["2026-08-30", "2026-02-30T12:05:00Z"] {
        refused_naming(
            &with(&base, &["observed_at"], json!(observed_at)),
            "/observed_at",
            &format!("an observed_at of {observed_at:?}"),
        );
    }

    // The schema this crate validates against is the one the requirement
    // authors, character for character. `tc_470_vendored_schemas` holds the
    // vendored copy against the TypeScript copy; this holds it against the
    // spec, which is the leg the deleted TypeScript test carried.
    let markdown_path = repo_root().join(REQUIREMENT);
    let markdown = std::fs::read_to_string(&markdown_path)
        .unwrap_or_else(|error| panic!("{}: {error}", markdown_path.display()));
    let authored_text = markdown
        .split_once("```json\n")
        .expect("FR-059 carries a fenced JSON schema")
        .1
        .split_once("\n```")
        .expect("the fence closes")
        .0;
    let authored: Value =
        serde_json::from_str(authored_text).expect("the authored schema block is JSON");
    assert_eq!(
        MeasurementSchema::OperationalEvidenceV1
            .document()
            .expect("the vendored schema parses"),
        authored,
        "the validated schema and the schema FR-059 authors have diverged"
    );
}

/// The control vocabulary admits releases, flags, canary and shadow deployment,
/// rollback and kill, override and appeal, abstention and fallback, and reporting.
///
/// Trace: FR-059-AC-2
/// Provenance: quoin#479
#[test]
fn tc_479_301_every_declared_control_kind_validates_and_no_other_does() {
    /// The eleven kinds FR-059 declares, written out rather than read back
    /// from the enum being measured.
    const KINDS: [&str; 11] = [
        "release",
        "feature_flag",
        "canary_deployment",
        "shadow_deployment",
        "rollback",
        "kill_switch",
        "human_override",
        "appeal",
        "abstention",
        "safe_fallback",
        "reporting",
    ];

    let declared: Vec<&'static str> = OperationalControlKind::all()
        .iter()
        .map(|kind| kind.as_str())
        .collect();
    assert_eq!(
        declared.as_slice(),
        KINDS.as_slice(),
        "the ported vocabulary is not the one FR-059 declares"
    );

    let base = capability();
    for kind in KINDS {
        let candidate = with(&base, &["control_kind"], json!(kind));
        admitted(&candidate, &format!("the control kind `{kind}`"));
    }

    refused_naming(
        &with(&base, &["control_kind"], json!("unknown-control")),
        "/control_kind",
        "a control kind outside the vocabulary",
    );
}

/// A standing-capability record requires its control surface, availability
/// state, authorized roles, coverage, limitations, supported transitions, and
/// an explicit clock-support choice; supported clocks require start event,
/// completion event, and positive deadline, while unsupported clocks exclude
/// them.
///
/// Trace: FR-059-AC-3
/// Provenance: quoin#479
#[test]
fn tc_479_302_a_capability_requires_its_surface_and_a_complete_clock_choice() {
    /// The eight members a standing capability carries.
    const CAPABILITY: [&str; 8] = [
        "control_id",
        "status",
        "surface",
        "authorized_roles",
        "coverage",
        "limitations",
        "supported_transitions",
        "clock_support",
    ];

    let base = capability();
    admitted(&base, "the retained capability");

    assert_eq!(
        CAPABILITY.len(),
        8,
        "the capability census is the criterion's"
    );
    for member in CAPABILITY {
        refused_naming(
            &without(&base, &["capability", member]),
            &required("/capability", member),
            &format!("a capability missing `{member}`"),
        );
    }
    // Authorized roles and supported transitions are each at least one, not
    // merely present.
    for member in ["authorized_roles", "supported_transitions"] {
        refused_naming(
            &with(&base, &["capability", member], json!([])),
            &format!("/capability/{member}"),
            &format!("a capability with no `{member}`"),
        );
    }

    // A supported clock is bounded on all three members, each independently.
    for member in ["start_event", "completion_event", "deadline_seconds"] {
        refused_naming(
            &without(&base, &["capability", "clock_support", member]),
            "supported clock requires events and positive deadline",
            &format!("a supported clock missing `{member}`"),
        );
    }
    refused_naming(
        &with(
            &base,
            &["capability", "clock_support", "deadline_seconds"],
            json!(0),
        ),
        "supported clock requires events and positive deadline",
        "a supported clock with a zero deadline",
    );

    // An unsupported clock is the bare choice and nothing else.
    admitted(
        &with(
            &base,
            &["capability", "clock_support"],
            json!({ "supported": false }),
        ),
        "an unsupported clock",
    );
    refused_naming(
        &with(
            &base,
            &["capability", "clock_support"],
            json!({ "supported": false, "deadline_seconds": 10 }),
        ),
        "unsupported clock excludes event/deadline fields",
        "an unsupported clock still carrying a deadline",
    );
}

/// An exercise record requires actual-or-drill mode, ordered start and
/// completion timestamps, actor, trigger, outcome, before/after state, at least
/// one observation, and clock applicability.
///
/// Trace: FR-059-AC-4
/// Provenance: quoin#479
#[test]
fn tc_479_303_an_exercise_requires_its_whole_shape_and_ordered_timing() {
    /// The eleven members an exercise carries.
    const EXERCISE: [&str; 11] = [
        "control_id",
        "mode",
        "started_at",
        "completed_at",
        "actor",
        "trigger",
        "outcome",
        "state_before",
        "state_after",
        "observations",
        "clock",
    ];

    let base = exercise();
    admitted(&base, "the retained exercise");

    assert_eq!(EXERCISE.len(), 11, "the exercise census is the criterion's");
    for member in EXERCISE {
        refused_naming(
            &without(&base, &["exercise", member]),
            &required("/exercise", member),
            &format!("an exercise missing `{member}`"),
        );
    }

    // Mode is actual or drill, and nothing else.
    for mode in ["actual", "drill"] {
        admitted(
            &with(&base, &["exercise", "mode"], json!(mode)),
            &format!("the exercise mode `{mode}`"),
        );
    }
    refused_naming(
        &with(&base, &["exercise", "mode"], json!("rehearsal")),
        "/exercise/mode",
        "an exercise mode outside the vocabulary",
    );

    // The two instants are ordered, and the observation time is no earlier than
    // the completion it reports.
    refused_naming(
        &with(
            &base,
            &["exercise", "completed_at"],
            json!("2026-08-29T23:10:00Z"),
        ),
        "precedes exercise start",
        "an exercise that completed before it started",
    );
    refused_naming(
        &with(&base, &["observed_at"], json!("2026-08-29T23:11:00Z")),
        "precedes exercise completion",
        "an observation taken before the completion it reports",
    );

    // At least one observation.
    refused_naming(
        &with(&base, &["exercise", "observations"], json!([])),
        "/exercise/observations",
        "an exercise that observed nothing",
    );
}

/// Exactly one shape payload is present: standing capability excludes exercise
/// data, and exercise excludes capability data.
///
/// Trace: FR-059-AC-5
/// Provenance: quoin#479
#[test]
fn tc_479_304_exactly_one_shape_payload_is_present() {
    let capability = capability();
    let exercise = exercise();
    admitted(&capability, "the retained capability alone");
    admitted(&exercise, "the retained exercise alone");

    // Adding the other shape's payload fails.
    refused_naming(
        &with(&capability, &["exercise"], exercise["exercise"].clone()),
        "standing_capability requires only capability payload",
        "a standing capability also carrying exercise data",
    );
    refused_naming(
        &with(&exercise, &["capability"], capability["capability"].clone()),
        "exercise requires only exercise payload",
        "an exercise also carrying capability data",
    );

    // Removing the discriminator-matched payload fails too: the shape names a
    // payload that has to be there.
    refused_naming(
        &without(&capability, &["capability"]),
        "standing_capability requires only capability payload",
        "a standing capability carrying no capability",
    );
    refused_naming(
        &without(&exercise, &["exercise"]),
        "exercise requires only exercise payload",
        "an exercise carrying no exercise",
    );
}

/// An `operational_with_clock` exercise requires ordered start and deadline
/// timestamps and a status consistent with completion and immutable observation
/// time; a not-applicable clock requires `not_applicable` status and no clock
/// timestamps.
///
/// Trace: FR-059-AC-6
/// Provenance: quoin#479
#[test]
fn tc_479_305_a_clocked_status_is_derived_from_the_instants_and_not_believed() {
    let base = exercise();
    // The retained exercise completed inside its deadline and says `met`.
    admitted(&base, "the retained clocked exercise");

    // A status the instants do not support is refused, in both directions.
    refused_naming(
        &with(&base, &["exercise", "clock", "status"], json!("missed")),
        "disagrees with derived met",
        "a clock claiming `missed` on a completion inside the deadline",
    );
    refused_naming(
        &with(&base, &["exercise", "clock", "status"], json!("open")),
        "disagrees with derived met",
        "a clock claiming `open` on a completed exercise",
    );

    // A declared gap is producer uncertainty and does not override a status the
    // timestamps settle deterministically.
    let with_gap = with(
        &with(&base, &["exercise", "clock", "status"], json!("missed")),
        &["gaps"],
        json!(["producer uncertainty must not hide deterministic time"]),
    );
    refused_naming(
        &with_gap,
        "disagrees with derived met",
        "a gap offered in place of the derived status",
    );

    // A completion after the deadline derives `missed`, whatever is recorded.
    refused_naming(
        &with(
            &base,
            &["exercise", "clock", "completed_at"],
            json!("2026-08-29T23:31:00Z"),
        ),
        "disagrees with derived missed",
        "a clock claiming `met` on a completion past the deadline",
    );

    // Clock timestamps are instants.
    refused_naming(
        &with(
            &base,
            &["exercise", "clock", "completed_at"],
            json!("2026-08-29"),
        ),
        "must be an RFC 3339 date-time",
        "a clock completion that is a date and not an instant",
    );

    // Start is no later than the deadline.
    refused_naming(
        &with(
            &base,
            &["exercise", "clock", "started_at"],
            json!("2026-08-29T23:31:00Z"),
        ),
        "precedes clock start",
        "a deadline earlier than the clock start",
    );

    // A running exercise: no completion, observed before the deadline, `open`.
    let open = with(
        &without(&base, &["exercise", "clock", "completed_at"]),
        &["exercise", "clock", "status"],
        json!("open"),
    );
    admitted(&open, "a clock still open inside its deadline");

    // A not-applicable clock is the applicability, the matching status, and no
    // timestamp at all.
    let not_applicable = with(
        &base,
        &["exercise", "clock"],
        json!({ "applicability": "not_applicable", "status": "not_applicable" }),
    );
    admitted(&not_applicable, "a not-applicable clock");
    refused_naming(
        &with(
            &not_applicable,
            &["exercise", "clock", "completed_at"],
            json!("2026-08-29T23:11:37Z"),
        ),
        "not_applicable excludes timestamps",
        "a not-applicable clock still carrying a timestamp",
    );
    refused_naming(
        &with(
            &not_applicable,
            &["exercise", "clock", "status"],
            json!("met"),
        ),
        "/exercise/clock",
        "a not-applicable clock claiming a deadline status",
    );
}

/// Succeeded, failed, partial, and aborted exercises all validate as retained
/// outcomes.
///
/// Trace: FR-059-AC-8
/// Provenance: quoin#479
#[test]
fn tc_479_307_every_declared_outcome_round_trips_without_collapsing() {
    /// The four outcomes FR-059 declares, written out rather than read back
    /// from the enum being measured.
    const OUTCOMES: [&str; 4] = ["succeeded", "failed", "partial", "aborted"];

    let declared: Vec<&'static str> = ExerciseOutcome::all()
        .iter()
        .map(|outcome| outcome.as_str())
        .collect();
    assert_eq!(
        declared.as_slice(),
        OUTCOMES.as_slice(),
        "the ported outcomes are not the ones FR-059 declares"
    );

    let base = exercise();
    for outcome in OUTCOMES {
        let candidate = with(&base, &["exercise", "outcome"], json!(outcome));
        let valid = admitted(&candidate, &format!("the outcome `{outcome}`"));
        // The typed read keeps the outcome apart from the other three: a port
        // that mapped every adverse outcome onto one value would pass a
        // validity check and fail here.
        match valid.record() {
            OperationalEvidenceRecord::Exercise(record) => assert_eq!(
                record.exercise.outcome.as_str(),
                outcome,
                "the outcome was read back as something else"
            ),
            OperationalEvidenceRecord::StandingCapability(_) => {
                panic!("{outcome}: an exercise was read as a standing capability");
            }
        }
        // And the document that reaches the store is still the caller's.
        assert_eq!(
            valid.document()["exercise"]["outcome"],
            json!(outcome),
            "the retained document no longer carries the outcome it was handed"
        );
    }

    refused_naming(
        &with(&base, &["exercise", "outcome"], json!("succeeded_mostly")),
        "/exercise/outcome",
        "an outcome outside the vocabulary",
    );
}

/// Refuses, one defect at a time, every governance or raw-evidence member
/// FR-059-AC-9 requires of a lone document: the three governance fields, a
/// non-empty owner and action list, at least one raw-evidence reference with
/// all three of its members, and no undeclared field.
fn refuse_every_document_defect_ac9(base: &Value) {
    // Governance fields, each independently.
    for member in ["owner", "gaps", "actions"] {
        refused_naming(
            &without(base, &[member]),
            &required("/", member),
            &format!("a record missing `{member}`"),
        );
    }
    refused_naming(
        &with(base, &["owner"], json!("")),
        "/owner",
        "a record owned by nobody",
    );
    refused_naming(
        &with(base, &["actions"], json!([])),
        "/actions",
        "a record asking for nothing next",
    );

    // Raw evidence: at least one reference, and each member of one required.
    refused_naming(
        &without(base, &["raw_evidence"]),
        &required("/", "raw_evidence"),
        "a record resting on no retained file",
    );
    refused_naming(
        &with(base, &["raw_evidence"], json!([])),
        "/raw_evidence",
        "a record with an empty raw-evidence list",
    );
    let reference = base["raw_evidence"][0].clone();
    for member in ["path", "media_type"] {
        refused_naming(
            &with(
                base,
                &["raw_evidence"],
                json!([without(&reference, &[member])]),
            ),
            &required("/raw_evidence/0", member),
            &format!("a raw-evidence reference missing `{member}`"),
        );
    }

    // An undeclared field is refused rather than silently dropped.
    refused_naming(
        &with(base, &["surprise"], json!(1)),
        "/: Additional properties are not allowed ('surprise' was unexpected)",
        "a record carrying a field the schema does not declare",
    );
    refused_naming(
        &with(base, &["capability", "surprise"], json!(1)),
        "/capability: Additional properties are not allowed ('surprise' was unexpected)",
        "a capability carrying a field the schema does not declare",
    );
}

/// Gaps, owner, actions, and at least one safe raw-evidence
/// reference with media type and byte size are retained; invalid capability
/// links and undeclared fields are refused.
///
/// Trace: FR-059-AC-9
/// Provenance: quoin#479
#[test]
fn tc_479_308_governance_and_safe_raw_evidence_are_required_and_bad_links_refused() {
    let base = capability();
    let exercise = exercise();

    refuse_every_document_defect_ac9(&base);
    let reference = base["raw_evidence"][0].clone();

    // Intake, against a store. The link check is about the store and cannot be
    // asked of a lone document, so this half runs in a workspace holding the
    // producer's whole input contract and nothing else.
    let workspace = tempfile::tempdir().expect("a writable workspace");
    let root = workspace.path();
    write_repository(root);

    let retained_path = write_operational_record(root, &SystemClock, &base)
        .expect("the retained capability is admissible against its own evidence");
    assert!(
        retained_path.exists(),
        "the admitted record was not written"
    );
    assert_eq!(
        read_operational_records(root)
            .expect("the store reads back")
            .len(),
        1
    );

    // An exercise naming a capability it does not match is refused, and writes
    // nothing.
    let mislinked = with(
        &exercise,
        &["exercise", "control_id"],
        json!("different-control"),
    );
    let error = write_operational_record(root, &SystemClock, &mislinked)
        .expect_err("a mismatched capability link is refused");
    assert_eq!(error.code(), InterventionRefusalCode::InvalidRecord);
    assert_eq!(
        read_operational_records(root)
            .expect("the store reads back")
            .len(),
        1,
        "a refused record must leave the store as it found it"
    );

    // Raw evidence that points outside the store is refused under the
    // raw-evidence code, and writes nothing.
    let what = "a path that resolves outside the evidence store";
    let raw_evidence = with(&reference, &["path"], json!("../../etc/passwd"));
    let tampered = with(&exercise, &["raw_evidence"], json!([raw_evidence]));
    let Err(error) = write_operational_record(root, &SystemClock, &tampered) else {
        panic!("{what}: admitted, and the criterion says it is refused")
    };
    assert_eq!(
        error.code(),
        InterventionRefusalCode::RawEvidenceMismatch,
        "{what}: refused under the wrong code"
    );
    assert_eq!(
        read_operational_records(root)
            .expect("the store reads back")
            .len(),
        1,
        "{what}: a refused record must leave the store as it found it"
    );
}

/// Every record, of either shape, carries exactly one EA claim strength, and
/// for this family it is `observed`: a record with no `strength` is refused
/// rather than defaulted, and each of the other three wire names, or a label
/// outside the vocabulary, is refused rather than admitted or coerced.
///
/// > FR-059-AC-10: Every record carries `strength`, closed to Engineering
/// > Assurance's `observed` claim strength (FR-022); a record with no
/// > `strength` is refused, not defaulted.
///
/// Trace: FR-059-AC-10
/// Provenance: PLAT-972
#[test]
fn tc_479_309_strength_is_required_and_closed_to_observed() {
    let mut measured = 0_usize;
    for (shape, base) in [("capability", capability()), ("exercise", exercise())] {
        let admitted = admitted(&base, &format!("the retained {shape}"));
        assert_eq!(admitted.record().base().strength, ClaimStrength::Observed);

        refused_naming(
            &without(&base, &["strength"]),
            &required("/", "strength"),
            &format!("a {shape} missing `strength`"),
        );

        for strength in ClaimStrength::ALL {
            if strength == ClaimStrength::Observed {
                continue;
            }
            refused_naming(
                &with(&base, &["strength"], json!(strength.wire_name())),
                "/strength",
                &format!("a {shape} claiming `{strength}`"),
            );
            measured += 1;
        }
        refused_naming(
            &with(&base, &["strength"], json!("verified")),
            "/strength",
            &format!("a {shape} with a strength outside the EA vocabulary"),
        );
    }
    assert_eq!(
        measured, 6,
        "anti-vacuity floor: every strength but `observed`, for both shapes, is measured"
    );
}

// -------------------------------------------------------------- the workspace
