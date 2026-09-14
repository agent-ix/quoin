// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every FR-059 acceptance criterion the deleted `tests/operational.test.ts`
//! carried, restated against this crate.
//!
//! Trace: FR-059-AC-1, FR-059-AC-2, FR-059-AC-3, FR-059-AC-4, FR-059-AC-5
//! Trace: FR-059-AC-6, FR-059-AC-7, FR-059-AC-8, FR-059-AC-9
//! Provenance: quoin#479
//!
//! # Why this file exists
//!
//! `tests/operational.test.ts` is deleted in the same commit as
//! `src/measurement/operational.ts` (FR-101). Nine FR-059 criteria were
//! carried by five of its cases, and a criterion that appears on no test after
//! the cutover is a criterion nothing holds. `tc_472_operational_pairs.rs`
//! proves the one retained pair still validates, and
//! `tc_472_github_release.rs` proves the producer rebuilds it byte for byte —
//! but a corpus round trip is not a per-criterion gate: neither would notice
//! if the control vocabulary lost a member or the clock stopped being derived.
//! This file is the per-criterion gate: one `#[test]` per criterion, named by
//! the criterion, so a criterion cannot be lost without a named test
//! disappearing with it.
//!
//! # Where the inputs come from
//!
//! `spec/evidence/operational/pairs/c1b30a…acf8.json` — the one committed pair,
//! produced by the retained TypeScript from real release evidence. Its
//! capability and its exercise are the valid base cases here, and every refusal
//! case is that base mutated **in Rust**, below. The TypeScript fixture the
//! deleted file built by hand is deliberately not respelled: a fixture this
//! file wrote would agree with whatever this file believes, while the committed
//! pair was written by the implementation being retired.
//!
//! The one case the retained evidence cannot supply is intake against a store,
//! which must be able to write. That runs in a `tempfile` workspace holding the
//! producer's whole input contract — the assurance documents the governance
//! check reads and the three retained GitHub exports the raw-evidence
//! accounting digests — copied from this repository, exactly as
//! `tc_472_github_release.rs` does. Nothing here writes into the repository.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

#[path = "common/copy.rs"]
mod copy;
#[path = "common/paths.rs"]
mod paths;

use copy::copy_tree;
use paths::repo_root;

use quoin_jsonschema::VendoredSchema;

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

/// The committed pair both base cases are read from.
const RETAINED_PAIR: &str = "c1b30a188d4d03bbe316e0fdb7582eff3fa314c55268a5f71f81f99f8ea2acf8.json";

/// The requirement the schema under test is authored in.
const REQUIREMENT: &str = "spec/functional/FR-059-operational-evidence-records.md";

// ------------------------------------------------------------ the base cases

/// The retained pair's record of one shape.
fn retained(shape: &str) -> Value {
    let path = repo_root()
        .join("spec")
        .join("evidence")
        .join("operational")
        .join("pairs")
        .join(RETAINED_PAIR);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let envelope: Value = serde_json::from_str(&text).expect("the retained pair is JSON");
    let records = envelope["records"]
        .as_array()
        .expect("the retained pair carries records");
    assert_eq!(
        records.len(),
        2,
        "a pair is one capability and one exercise; the base cases come from it"
    );
    records
        .iter()
        .find(|record| record["record_shape"] == json!(shape))
        .unwrap_or_else(|| panic!("the retained pair carries no `{shape}` record"))
        .clone()
}

/// The retained standing-capability record.
fn capability() -> Value {
    retained("standing_capability")
}

/// The retained exercise record.
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
    /// The fourteen members both shapes carry.
    const ENVELOPE: [&str; 14] = [
        "schema_version",
        "record_type",
        "record_id",
        "observed_at",
        "record_shape",
        "control_kind",
        "subject",
        "producer",
        "scope",
        "configuration",
        "owner",
        "gaps",
        "actions",
        "raw_evidence",
    ];
    /// The FR-044 producer tuple, unchanged.
    const PRODUCER: [&str; 6] = [
        "tool_identity",
        "tool_version",
        "configuration_digest",
        "source_revision",
        "environment",
        "definition_version",
    ];

    let base = capability();
    admitted(&base, "the retained capability");

    // Each member independently: a record missing exactly one is refused, and
    // the refusal names the member that is gone.
    assert_eq!(ENVELOPE.len(), 14, "the envelope census is the criterion's");
    for member in ENVELOPE {
        refused_naming(
            &without(&base, &[member]),
            &required("/", member),
            &format!("an envelope missing `{member}`"),
        );
    }
    assert_eq!(PRODUCER.len(), 6, "the FR-044 producer tuple is six fields");
    for member in PRODUCER {
        refused_naming(
            &without(&base, &["producer", member]),
            &required("/producer", member),
            &format!("a producer missing `{member}`"),
        );
    }
    for member in ["id", "revision"] {
        refused_naming(
            &without(&base, &["subject", member]),
            &required("/subject", member),
            &format!("a subject missing `{member}`"),
        );
    }
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
        VendoredSchema::OperationalEvidenceV1
            .document()
            .expect("the vendored schema parses"),
        authored,
        "the validated schema and the schema FR-059 authors have diverged"
    );
}

/// The control vocabulary admits releases, flags, canary and shadow deployment,
/// rollback and kill, override and appeal, abstention and fallback, reporting,
/// and policy, prompt, model, tool, and data pinning.
///
/// Trace: FR-059-AC-2
/// Provenance: quoin#479
#[test]
fn tc_479_301_every_declared_control_kind_validates_and_no_other_does() {
    /// The sixteen kinds FR-059 declares, written out rather than read back
    /// from the enum being measured.
    const KINDS: [&str; 16] = [
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
        "policy_pin",
        "prompt_pin",
        "model_pin",
        "tool_pin",
        "data_pin",
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
        let mut candidate = with(&base, &["control_kind"], json!(kind));
        // A `*_pin` control is required to carry the pin it is named after;
        // that requirement is FR-059-AC-7's, and here it is only satisfied so
        // that this test measures the vocabulary and nothing else.
        if let Some(pin_kind) = kind.strip_suffix("_pin") {
            candidate = with(
                &candidate,
                &["configuration", "version_pins"],
                json!([{
                    "kind": pin_kind,
                    "identity": "fixture",
                    "revision": "v1",
                    "digest": format!("sha256:{}", "2".repeat(64)),
                }]),
            );
        }
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

/// Each policy, prompt, model, tool, or data pin record carries at least one
/// matching typed identity, revision, and digest, with no duplicate
/// kind/identity key.
///
/// Trace: FR-059-AC-7
/// Provenance: quoin#479
#[test]
fn tc_479_306_a_pin_control_requires_one_unique_matching_typed_pin() {
    let pinned = with(&capability(), &["control_kind"], json!("model_pin"));
    let pin = json!({
        "kind": "model",
        "identity": "fixture-model",
        "revision": "v1",
        "digest": format!("sha256:{}", "3".repeat(64)),
    });
    let pinned_with = |pins: Value| with(&pinned, &["configuration", "version_pins"], pins);

    // An empty pin list, and a list pinning something else, are both refused.
    refused_naming(
        &pinned_with(json!([])),
        "model_pin requires a matching pin kind",
        "a model_pin record pinning nothing",
    );
    refused_naming(
        &pinned_with(json!([with(&pin, &["kind"], json!("policy"))])),
        "model_pin requires a matching pin kind",
        "a model_pin record carrying only a policy pin",
    );

    // One matching pin is enough.
    admitted(
        &pinned_with(json!([pin])),
        "a model_pin record with a model pin",
    );

    // Two pins on one thing are refused at the second.
    refused_naming(
        &pinned_with(json!([pin, pin])),
        "duplicate kind/identity",
        "a pin list naming one thing twice",
    );

    // The pin is typed: every member is required, independently.
    for member in ["kind", "identity", "revision", "digest"] {
        refused_naming(
            &pinned_with(json!([without(&pin, &[member])])),
            &required("/configuration/version_pins/0", member),
            &format!("a pin missing `{member}`"),
        );
    }
    // And the digest is a digest, not any string.
    refused_naming(
        &pinned_with(json!([with(&pin, &["digest"], json!("not-a-digest"))])),
        "/configuration/version_pins/0/digest",
        "a pin digest outside the declared grammar",
    );
    // And the pin kind is the declared vocabulary.
    refused_naming(
        &pinned_with(json!([with(&pin, &["kind"], json!("weights"))])),
        "/configuration/version_pins/0/kind",
        "a pin kind outside the declared vocabulary",
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
/// all four of its members under SHA-256, and no undeclared field.
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
    for member in ["path", "media_type", "size_bytes", "digest"] {
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
    // The content digest is SHA-256, the algorithm quoin recomputes itself.
    refused_naming(
        &with(
            base,
            &["raw_evidence"],
            json!([with(
                &reference,
                &["digest"],
                json!(format!("blake3:{}", "a".repeat(64)))
            )]),
        ),
        "/raw_evidence/0/digest",
        "a raw-evidence digest under another algorithm",
    );

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

/// Gaps, owner, actions, and at least one safe content-digested raw-evidence
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
    copy_tree(
        &repo_root().join("spec").join("assurance"),
        &root.join("spec").join("assurance"),
    );
    copy_tree(
        &repo_root()
            .join("spec")
            .join("evidence")
            .join("github-actions"),
        &root.join("spec").join("evidence").join("github-actions"),
    );

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

    // Raw evidence that no longer digests to what it claims, and raw evidence
    // that points outside the store, are both refused under the raw-evidence
    // code — and both write nothing.
    for (raw_evidence, what) in [
        (
            with(
                &reference,
                &["digest"],
                json!(format!("sha256:{}", "f".repeat(64))),
            ),
            "a digest the retained bytes do not produce",
        ),
        (
            with(&reference, &["path"], json!("../../etc/passwd")),
            "a path that resolves outside the evidence store",
        ),
    ] {
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
}

// -------------------------------------------------------------- the workspace
