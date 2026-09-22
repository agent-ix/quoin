// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every FR-056 acceptance criterion the deleted `tests/intervention.test.ts`
//! carried, restated against this crate.
//!
//! Trace: FR-056-AC-1, FR-056-AC-2, FR-056-AC-3, FR-056-AC-4, FR-056-AC-5
//! Trace: FR-056-AC-6, FR-056-AC-7, FR-056-AC-8, FR-056-AC-9
//! Provenance: quoin#479
//!
//! # Why this file exists
//!
//! `tests/intervention.test.ts` is deleted in the same commit as
//! `src/measurement/intervention.ts` (FR-101, Stage 6 cutover). It carried the
//! nine FR-056 criteria across six `test` blocks, several of which tagged two
//! criteria at once. `tc_471_corpus.rs` proves this crate reads and reproduces
//! the one retained record, and `tc_471_intake.rs` proves the record-id and
//! path guards, but neither is tagged per criterion: a corpus round trip is not
//! a statement that `producer.environment` may not be empty. This file is one
//! test per criterion, named by the criterion, so a criterion cannot be lost
//! without a named test disappearing with it.
//!
//! # Where the inputs come from
//!
//! `spec/evidence/interventions/quoin-270-cli-eval-sentinel-contract.json` —
//! the committed retained record, not a fixture this file invented. A fixture
//! built here and validated here would agree with itself; the retained record
//! was produced by `agent-eval-intervention.ts` before this crate existed. It
//! is the valid base case, and every refusal case is that record with **one**
//! mutation applied here, in Rust, so each refusal is attributable to the one
//! thing that changed.
//!
//! # Independence, and the floors
//!
//! "Rejected independently" is the criterion in AC-2, AC-4, AC-5 and AC-9: for
//! each field, the otherwise-valid record loses exactly that field and *that*
//! refusal is asserted. Every such list is length-asserted against the literal
//! count the schema declares, so a list that silently shortened would fail
//! rather than pass by measuring less.

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

use std::path::Path;

use quoin_jsonschema::MeasurementSchema;
use quoin_measurement::common::scalar::{EffectValue, ScalarValue};
use quoin_measurement::intervention::record::{
    InterventionConclusionKind, InterventionDisposition, InterventionExperimentRecord,
    InterventionStatus, MeasuredEffect,
};
use quoin_measurement::{
    DiskMeasurement, InterventionIntakeError, InterventionRefusalCode, read_intervention_records,
    validate_intervention_record, write_intervention_record,
};
use serde_json::{Number, Value, json};

// ------------------------------------------------------- the retained record

/// The one retained intervention record, as committed.
///
/// Read from disk on every call so that a mutation made by one test cannot
/// reach another: each test owns its copy.
fn retained() -> Value {
    let path = repo_root()
        .join("spec")
        .join("evidence")
        .join("interventions")
        .join("quoin-270-cli-eval-sentinel-contract.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()));
    serde_json::from_str(&text).expect("the retained record is JSON")
}

// ------------------------------------------------------------- mutation

/// Walk to a member by path, where a numeric step indexes an array.
fn at<'a>(value: &'a mut Value, path: &[&str]) -> &'a mut Value {
    let mut cursor = value;
    for step in path {
        cursor = match cursor {
            Value::Array(items) => {
                let index: usize = step
                    .parse()
                    .unwrap_or_else(|_| panic!("`{step}` is not an array index"));
                items
                    .get_mut(index)
                    .unwrap_or_else(|| panic!("the base case has no element {index}"))
            }
            Value::Object(members) => members
                .get_mut(*step)
                .unwrap_or_else(|| panic!("the base case has no member `{step}`")),
            other => panic!("`{step}` has no container to read: {other}"),
        };
    }
    cursor
}

/// The retained record with the member at `path` deleted.
fn without(path: &[&str]) -> Value {
    let mut value = retained();
    let (last, parents) = path.split_last().expect("a non-empty path");
    let parent = at(&mut value, parents);
    let members = parent
        .as_object_mut()
        .unwrap_or_else(|| panic!("`{last}`'s parent is not an object"));
    assert!(
        members.remove(*last).is_some(),
        "the retained record carries no `{}` to delete",
        path.join(".")
    );
    value
}

/// The retained record with the member at `path` replaced.
fn with(path: &[&str], replacement: Value) -> Value {
    let mut value = retained();
    *at(&mut value, path) = replacement;
    value
}

// ------------------------------------------------------------- verdicts

/// The record a candidate the criterion calls admissible reads as.
fn accepted(candidate: &Value, what: &str) -> InterventionExperimentRecord {
    validate_intervention_record(candidate).unwrap_or_else(|error| {
        panic!("{what}: refused, and the criterion says it is admissible: {error}")
    })
}

/// The refusal a candidate the criterion calls inadmissible carries.
fn refused(candidate: &Value, what: &str) -> InterventionIntakeError {
    match validate_intervention_record(candidate) {
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

/// Assert the refusal carries this exact finding — the literal the port emits,
/// not a re-derivation of it.
fn says(error: &InterventionIntakeError, finding: &str, what: &str) {
    assert!(
        error.findings().iter().any(|found| found == finding),
        "{what}: expected the finding {finding:?}, got {:?}",
        error.findings()
    );
}

/// Assert some finding names `needle` — the pointer or property the schema
/// refused on. The sentence after the colon is the validator's own and is not
/// contractual (quoin#470's `DIVERGENCE.md`), so only the name is asserted.
fn names(error: &InterventionIntakeError, needle: &str, what: &str) {
    assert!(
        error.findings().iter().any(|found| found.contains(needle)),
        "{what}: expected a finding naming {needle:?}, got {:?}",
        error.findings()
    );
}

/// Deleting exactly this member, from an otherwise-valid record, is refused
/// and the refusal names it.
fn required(path: &[&str]) {
    let what = format!("a record without {}", path.join("."));
    let error = refused(&without(path), &what);
    names(&error, path.last().expect("a non-empty path"), &what);
}

// ------------------------------------------------------------- the gates

/// The envelope: schema identity, version, record type, record identity,
/// observation timestamp, subject identity and revision, and the FR-044
/// producer tuple are all required.
///
/// > FR-056-AC-1: The schema requires version, record type and identity,
/// > observation timestamp, subject identity and revision, and the unchanged
/// > FR-044 producer tuple.
///
/// Trace: FR-056-AC-1
/// Provenance: quoin#479
#[test]
fn tc_479_100_the_envelope_and_producer_tuple_are_required() {
    /// Every envelope member `intervention.test.ts:140` deleted, plus the two
    /// discriminants and the two subject members the criterion also names.
    const ENVELOPE: [&[&str]; 8] = [
        &["schema_version"],
        &["record_type"],
        &["record_id"],
        &["observed_at"],
        &["subject"],
        &["subject", "id"],
        &["subject", "revision"],
        &["producer"],
    ];

    // The versioned identity the record family is pinned to. The literal, not
    // whatever the vendored document happens to say today.
    let document = MeasurementSchema::InterventionExperimentV1
        .document()
        .expect("the vendored intervention schema is readable");
    assert_eq!(
        document["$id"],
        json!("https://agent-ix.github.io/quoin/schemas/intervention-experiment-v1.schema.json"),
        "the schema identity is versioned and unchanged"
    );

    let base = accepted(&retained(), "the retained record");
    assert_eq!(base.schema_version, 1);
    assert_eq!(
        base.record_id.as_str(),
        "quoin-270-cli-eval-sentinel-contract"
    );
    assert_eq!(base.observed_at.as_str(), "2026-08-30T17:54:41.378Z");
    assert_eq!(base.subject.id.as_str(), "agent-ix/cli-agent-evals");
    assert_eq!(
        base.subject.revision.as_str(),
        "9e7b6cdde67fccdd9417171f8f116a8c91d400b7"
    );

    assert_eq!(ENVELOPE.len(), 8, "every envelope member is measured");
    for path in ENVELOPE {
        required(path);
    }

    // The timestamp is an instant, not a date and not an impossible one.
    for observed_at in ["2026-08-30", "2026-02-30T12:00:00Z", "2026-08-30T25:00:00Z"] {
        let what = format!("observed_at {observed_at}");
        let error = refused(&with(&["observed_at"], json!(observed_at)), &what);
        says(&error, "/observed_at: must be a valid date-time", &what);
    }
}

/// The producer tuple refuses each missing field independently, a mutable tool
/// version, an empty environment, and a malformed configuration digest.
///
/// > FR-056-AC-2: The producer tuple rejects a missing field, a mutable tool
/// > version, an empty environment, and a malformed configuration digest.
///
/// Trace: FR-056-AC-2
/// Provenance: quoin#479
#[test]
fn tc_479_101_each_producer_defect_is_refused_independently() {
    /// The six producer members `intervention.test.ts:157` deleted one at a
    /// time.
    const PRODUCER: [&str; 6] = [
        "tool_identity",
        "tool_version",
        "configuration_digest",
        "source_revision",
        "environment",
        "definition_version",
    ];

    /// Versions that name something that can move under the record.
    const MUTABLE: [&str; 5] = ["latest", "main", "1.2", "v1", ""];

    /// Digests the `sha256:<64 lowercase hex>` grammar refuses.
    const MALFORMED: [&str; 5] = [
        "sha256:",
        "sha256:abc",
        "3e1a8362b21a05d0962bd9789dae5eebef362f33c9c25665c838372bf20658d2",
        "sha1:3e1a8362b21a05d0962bd9789dae5eebef362f33c9c25665c838372bf20658d2",
        "sha256:3E1A8362B21A05D0962BD9789DAE5EEBEF362F33C9C25665C838372BF20658D2",
    ];

    accepted(&retained(), "the retained record");
    assert_eq!(PRODUCER.len(), 6, "every producer member is measured");
    for member in PRODUCER {
        required(&["producer", member]);
    }

    assert!(!MUTABLE.is_empty() && !MALFORMED.is_empty());
    for version in MUTABLE {
        let what = format!("tool_version {version:?}");
        let error = refused(&with(&["producer", "tool_version"], json!(version)), &what);
        names(&error, "tool_version", &what);
    }
    for digest in MALFORMED {
        let what = format!("configuration_digest {digest:?}");
        let error = refused(
            &with(&["producer", "configuration_digest"], json!(digest)),
            &what,
        );
        names(&error, "configuration_digest", &what);
    }

    let what = "an empty environment";
    let error = refused(&with(&["producer", "environment"], json!({})), what);
    names(&error, "environment", what);
    // Anti-vacuity: the retained environment is not empty, so the refusal above
    // is the emptiness and not the member's absence.
    assert_eq!(
        accepted(&retained(), "the retained record")
            .producer
            .environment
            .len(),
        10,
        "the retained producer declares ten environment entries"
    );
}

/// Every design kind validates with its assignment method, and the repetition
/// count, assignment method, sampling conditions and randomized seed are each
/// enforced.
///
/// > FR-056-AC-3: Repeated, randomized, and factorial designs require a
/// > positive repetition count, assignment method, and non-empty sampling
/// > conditions; randomized assignment requires a reproducible seed.
///
/// Trace: FR-056-AC-3
/// Provenance: quoin#479
#[test]
fn tc_479_102_every_design_and_assignment_boundary_is_enforced() {
    /// The three (kind, method) pairs `intervention.test.ts:192` walked.
    const DESIGNS: [(&str, &str); 3] = [
        ("repeated", "deterministic"),
        ("randomized", "randomized"),
        ("factorial", "blocked_randomized"),
    ];

    assert_eq!(DESIGNS.len(), 3, "every design kind is measured");
    for (kind, method) in DESIGNS {
        let assignment = if method.contains("randomized") {
            json!({ "method": method, "seed": "seed-1" })
        } else {
            json!({ "method": method })
        };
        let design = json!({
            "kind": kind,
            "repetitions": 1,
            "assignment": assignment,
            "sampling_conditions": ["fixed"],
        });
        let valid = with(&["design"], design);
        accepted(&valid, &format!("a {kind} design assigned {method}"));

        // A non-positive repetition count, on each kind.
        let mut zero = valid.clone();
        zero["design"]["repetitions"] = json!(0);
        let what = format!("a {kind} design repeated 0 times");
        names(&refused(&zero, &what), "repetitions", &what);

        // No sampling conditions, on each kind.
        let mut unsampled = valid;
        unsampled["design"]["sampling_conditions"] = json!([]);
        let what = format!("a {kind} design with no sampling conditions");
        names(&refused(&unsampled, &what), "sampling_conditions", &what);
    }

    required(&["design", "assignment", "method"]);
    required(&["design", "repetitions"]);
    required(&["design", "sampling_conditions"]);

    // A randomized assignment with no seed is not reproducible.
    let what = "a randomized assignment with no seed";
    let error = refused(
        &with(&["design", "assignment"], json!({"method": "randomized"})),
        what,
    );
    says(
        &error,
        "/design/assignment/seed: randomized assignment requires a seed",
        what,
    );

    // A randomized design assigned deterministically is not randomized.
    let what = "a randomized design assigned deterministically";
    let error = refused(&with(&["design", "kind"], json!("randomized")), what);
    says(
        &error,
        "/design/assignment/method: randomized design requires randomized assignment",
        what,
    );
}

/// One uniquely identified baseline, one or more uniquely identified
/// treatments, treatment-linked changed variables, and an explicit
/// held-constant list.
///
/// > FR-056-AC-4: Every record carries one uniquely identified baseline, one
/// > or more uniquely identified treatments, treatment-linked changed
/// > variables, and an explicit list of held-constant variables.
///
/// Trace: FR-056-AC-4
/// Provenance: quoin#479
#[test]
fn tc_479_103_arms_variables_and_held_constants_are_linked_and_unique() {
    /// The four arm members, each required on the baseline and on a treatment.
    const ARM: [&str; 4] = ["id", "population", "sample_size", "configuration"];

    /// The four changed-variable members.
    const CHANGED: [&str; 4] = ["name", "treatment_id", "baseline_value", "treatment_value"];

    let base = accepted(&retained(), "the retained record");
    assert_eq!(base.baseline.id.as_str(), "substring-detector");
    assert_eq!(base.treatments.len(), 1);
    assert_eq!(base.treatments[0].id.as_str(), "standalone-detector");
    assert_eq!(
        base.held_constant.len(),
        4,
        "the held-constant list is explicit"
    );
    assert_eq!(
        base.changed_variables[0].treatment_id.as_str(),
        "standalone-detector"
    );

    assert_eq!(ARM.len(), 4, "every arm member is measured");
    for member in ARM {
        required(&["baseline", member]);
        required(&["treatments", "0", member]);
    }
    assert_eq!(
        CHANGED.len(),
        4,
        "every changed-variable member is measured"
    );
    for member in CHANGED {
        required(&["changed_variables", "0", member]);
    }
    required(&["baseline"]);
    required(&["treatments"]);
    required(&["changed_variables"]);
    required(&["held_constant"]);

    // At least one treatment and at least one changed variable.
    for (path, what) in [
        (["treatments"], "a record with no treatments"),
        (["changed_variables"], "a record with no changed variables"),
    ] {
        names(&refused(&with(&path, json!([])), what), path[0], what);
    }

    // A treatment id that repeats the baseline's, and a treatment id that
    // repeats another treatment's.
    let what = "a treatment reusing the baseline id";
    let error = refused(
        &with(&["treatments", "0", "id"], json!("substring-detector")),
        what,
    );
    says(
        &error,
        "/treatments/0/id: duplicate arm id substring-detector",
        what,
    );

    let mut twinned = retained();
    let treatment = twinned["treatments"][0].clone();
    twinned["treatments"]
        .as_array_mut()
        .expect("the treatment list")
        .push(treatment);
    let what = "two treatments sharing one id";
    let error = refused(&twinned, what);
    says(
        &error,
        "/treatments/1/id: duplicate arm id standalone-detector",
        what,
    );

    // A changed variable naming an arm that does not exist.
    let what = "a changed variable naming no treatment";
    let error = refused(
        &with(
            &["changed_variables", "0", "treatment_id"],
            json!("missing"),
        ),
        what,
    );
    says(
        &error,
        "/changed_variables/0/treatment_id: does not resolve to one treatment",
        what,
    );

    // Two changed variables under one (treatment, name) key.
    let mut duplicated = retained();
    let row = duplicated["changed_variables"][0].clone();
    duplicated["changed_variables"]
        .as_array_mut()
        .expect("the changed-variable list")
        .push(row);
    let what = "a duplicated changed-variable key";
    let error = refused(&duplicated, what);
    says(
        &error,
        "/changed_variables/1: duplicate treatment/name key",
        what,
    );
}

/// Effect values of every declared type survive the round trip, `null` is not
/// coerced to zero, every member is required, and a duplicate
/// treatment/metric key is refused.
///
/// > FR-056-AC-5: Measured effects preserve treatment identity, baseline
/// > value, treatment value, effect, metric identity, and unit without
/// > replacing a missing value with zero or permitting duplicate
/// > treatment/metric keys.
///
/// Trace: FR-056-AC-5
/// Provenance: quoin#479
#[test]
fn tc_479_104_measured_effects_keep_their_values_and_reject_duplicates() {
    /// The six measured-effect members, each required.
    const EFFECT: [&str; 6] = [
        "treatment_id",
        "metric",
        "baseline_value",
        "treatment_value",
        "effect",
        "unit",
    ];

    // Identity, metric and unit travel unaltered.
    let base = accepted(&retained(), "the retained record");
    let row = &base.measured_effects[0];
    assert_eq!(row.treatment_id.as_str(), "standalone-detector");
    assert_eq!(row.metric.as_str(), "agent-eval.ea-001.pass-rate");
    assert_eq!(row.unit, "fraction");

    let numeric = effect_case(json!(0.5), json!(1), json!(-0.25));
    assert_eq!(numeric.baseline_value, ScalarValue::Number(half()));
    assert_eq!(
        numeric.treatment_value,
        ScalarValue::Number(Number::from(1))
    );
    assert_eq!(
        numeric.effect,
        EffectValue::Number(Number::from_f64(-0.25).expect("a finite number"))
    );

    let textual = effect_case(json!("before"), json!("after"), json!("increase"));
    assert_eq!(
        textual.baseline_value,
        ScalarValue::String("before".to_owned())
    );
    assert_eq!(
        textual.treatment_value,
        ScalarValue::String("after".to_owned())
    );
    assert_eq!(textual.effect, EffectValue::String("increase".to_owned()));

    let boolean = effect_case(json!(false), json!(true), json!("flipped"));
    assert_eq!(boolean.baseline_value, ScalarValue::Bool(false));
    assert_eq!(boolean.treatment_value, ScalarValue::Bool(true));

    // The whole point of the criterion: unavailable stays unavailable.
    let unavailable = effect_case(Value::Null, Value::Null, Value::Null);
    assert_eq!(unavailable.baseline_value, ScalarValue::Null);
    assert_eq!(unavailable.treatment_value, ScalarValue::Null);
    assert_eq!(unavailable.effect, EffectValue::Null);
    assert_ne!(
        unavailable.baseline_value,
        ScalarValue::Number(Number::from(0))
    );
    assert_ne!(unavailable.effect, EffectValue::Number(Number::from(0)));

    assert_eq!(EFFECT.len(), 6, "every measured-effect member is measured");
    for member in EFFECT {
        required(&["measured_effects", "0", member]);
    }
    required(&["measured_effects"]);

    let mut duplicated = retained();
    let row = duplicated["measured_effects"][0].clone();
    duplicated["measured_effects"]
        .as_array_mut()
        .expect("the measured-effect list")
        .push(row);
    let what = "a duplicated treatment/metric key";
    let error = refused(&duplicated, what);
    says(
        &error,
        "/measured_effects/1: duplicate treatment/metric key",
        what,
    );
}

/// `0.5` as a JSON number.
fn half() -> Number {
    Number::from_f64(0.5).expect("a finite number")
}

/// The retained record's one measured effect, given these three values.
fn effect_case(baseline: Value, treatment: Value, effect: Value) -> MeasuredEffect {
    let mut candidate = retained();
    let row = at(&mut candidate, &["measured_effects", "0"]);
    row["baseline_value"] = baseline;
    row["treatment_value"] = treatment;
    row["effect"] = effect;
    let record = accepted(&candidate, "a measured effect");
    record
        .measured_effects
        .into_iter()
        .next()
        .expect("the measured effect")
}

/// Interactions and confounders stay two collections, and each carries all
/// four dispositions.
///
/// > FR-056-AC-6: Interactions and confounders remain separate collections
/// > whose entries distinguish controlled, uncontrolled, unknown, and
/// > not-applicable dispositions.
///
/// Trace: FR-056-AC-6
/// Provenance: quoin#479
#[test]
fn tc_479_105_qualifier_collections_stay_separate_and_keep_every_disposition() {
    let dispositions = InterventionDisposition::all();
    assert_eq!(
        dispositions.len(),
        4,
        "the disposition union has four members"
    );

    let entries = |prefix: &str| -> Value {
        Value::Array(
            dispositions
                .iter()
                .map(|disposition| {
                    json!({
                        "description": format!("{prefix}-{}", disposition.as_str()),
                        "disposition": disposition.as_str(),
                    })
                })
                .collect(),
        )
    };
    let mut candidate = retained();
    candidate["interactions"] = entries("interaction");
    candidate["confounders"] = entries("confounder");
    let record = accepted(&candidate, "both qualifier collections, fully populated");

    // Each collection keeps its own four entries, in order, with the
    // disposition each declared.
    for (collection, rows) in [
        ("interaction", &record.interactions),
        ("confounder", &record.confounders),
    ] {
        assert_eq!(rows.len(), 4, "{collection}: four entries are retained");
        for (index, disposition) in dispositions.iter().enumerate() {
            assert_eq!(
                rows[index].disposition, *disposition,
                "{collection}[{index}]"
            );
            assert_eq!(
                rows[index].description,
                format!("{collection}-{}", disposition.as_str()),
                "{collection}[{index}]: the description is the one declared"
            );
        }
    }

    // Nothing moved between the two collections: every description carries the
    // name of the collection it was written into.
    assert!(
        record
            .interactions
            .iter()
            .all(|row| row.description.starts_with("interaction-")),
        "a confounder reached the interaction list: {:?}",
        record.interactions
    );
    assert!(
        record
            .confounders
            .iter()
            .all(|row| row.description.starts_with("confounder-")),
        "an interaction reached the confounder list: {:?}",
        record.confounders
    );

    required(&["interactions"]);
    required(&["confounders"]);
    required(&["interactions", "0", "description"]);
    required(&["interactions", "0", "disposition"]);
    required(&["confounders", "0", "description"]);
    required(&["confounders", "0", "disposition"]);

    let what = "a disposition outside the union";
    let error = refused(
        &with(&["interactions", "0", "disposition"], json!("mitigated")),
        what,
    );
    names(&error, "disposition", what);
}

/// All three statuses validate, and a terminal status requires the first-class
/// `cause_not_established` conclusion.
///
/// > FR-056-AC-7: Completed, failed, and inconclusive statuses validate;
/// > failed and inconclusive records require the first-class
/// > `cause_not_established` conclusion.
///
/// Trace: FR-056-AC-7
/// Provenance: quoin#479
#[test]
fn tc_479_106_terminal_statuses_require_cause_not_established() {
    /// The finding a terminal status with any other conclusion carries.
    const FINDING: &str =
        "/conclusion/kind: failed or inconclusive records require cause_not_established";

    let statuses = InterventionStatus::all();
    assert_eq!(statuses.len(), 3, "the status union has three members");

    // The retained record concludes `cause_not_established`, so every status
    // validates against it.
    for status in statuses {
        let what = format!(
            "status {} concluding cause_not_established",
            status.as_str()
        );
        let record = accepted(&with(&["status"], json!(status.as_str())), &what);
        assert_eq!(record.status, *status);
        assert_eq!(
            record.conclusion.kind,
            InterventionConclusionKind::CauseNotEstablished
        );
    }

    // A terminal status with either other conclusion is refused for the
    // conclusion, not for the status.
    for status in ["failed", "inconclusive"] {
        for kind in ["no_effect_observed", "causal_effect_established"] {
            let mut candidate = retained();
            candidate["status"] = json!(status);
            candidate["conclusion"]["kind"] = json!(kind);
            let what = format!("a {status} record concluding {kind}");
            says(&refused(&candidate, &what), FINDING, &what);
        }
    }

    // Anti-vacuity: `completed` with `no_effect_observed` is admissible, so the
    // refusals above are the terminal statuses and not the conclusion alone.
    let mut completed = retained();
    completed["status"] = json!("completed");
    completed["conclusion"]["kind"] = json!("no_effect_observed");
    accepted(&completed, "a completed record observing no effect");

    required(&["status"]);
    required(&["conclusion"]);
    required(&["conclusion", "kind"]);
    required(&["conclusion", "statement"]);
    required(&["conclusion", "attribution_confidence"]);
}

/// A causal conclusion needs positive samples, non-null effects, attributed
/// confidence and no open qualifier; a no-effect conclusion needs positive
/// samples and an observed comparison.
///
/// > FR-056-AC-8: A causal-effect conclusion requires positive baseline and
/// > treatment samples, at least one non-null measured effect, non-`none`
/// > attribution confidence, and no uncontrolled or unknown interaction or
/// > confounder; a no-effect conclusion requires positive samples and an
/// > observed comparison.
///
/// Trace: FR-056-AC-8
/// Provenance: quoin#479
#[test]
fn tc_479_107_causal_conclusions_require_their_support() {
    /// The findings the semantic pass emits for each unsupported causal claim.
    const SAMPLES: &str =
        "/conclusion/kind: observed-effect conclusions require positive arm samples";
    const COMPARISON: &str =
        "/measured_effects: observed-effect conclusions require observed comparisons";
    const NON_NULL: &str = "/measured_effects: causal conclusions require non-null effects";
    const CONFIDENCE: &str =
        "/conclusion/attribution_confidence: causal conclusion requires attributed confidence";
    const OPEN: &str =
        "/conclusion/kind: causal conclusion conflicts with uncontrolled or unknown qualifier";

    // The retained record is `cause_not_established` with open qualifiers. The
    // supported causal base case is that record with its qualifiers controlled
    // and its confidence attributed — the one shape the criterion admits.
    let causal = causal_base();
    accepted(&causal, "a fully supported causal conclusion");

    // Each support the criterion names, withdrawn one at a time from the
    // record that holds all of them.
    let withdrawals: [(&[&str], Value, &str, &str); 10] = [
        (
            &["baseline", "sample_size"],
            json!(0),
            SAMPLES,
            "no baseline sample",
        ),
        (
            &["treatments", "0", "sample_size"],
            json!(0),
            SAMPLES,
            "no treatment sample",
        ),
        (
            &["measured_effects", "0", "baseline_value"],
            Value::Null,
            COMPARISON,
            "no baseline value",
        ),
        (
            &["measured_effects", "0", "treatment_value"],
            Value::Null,
            COMPARISON,
            "no treatment value",
        ),
        (
            &["measured_effects", "0", "effect"],
            Value::Null,
            NON_NULL,
            "no stated effect",
        ),
        (
            &["conclusion", "attribution_confidence"],
            json!("none"),
            CONFIDENCE,
            "no attributed confidence",
        ),
        (
            &["interactions", "0", "disposition"],
            json!("uncontrolled"),
            OPEN,
            "an uncontrolled interaction",
        ),
        (
            &["interactions", "1", "disposition"],
            json!("unknown"),
            OPEN,
            "an unknown interaction",
        ),
        (
            &["confounders", "0", "disposition"],
            json!("uncontrolled"),
            OPEN,
            "an uncontrolled confounder",
        ),
        (
            &["confounders", "1", "disposition"],
            json!("unknown"),
            OPEN,
            "an unknown confounder",
        ),
    ];
    assert_eq!(
        withdrawals.len(),
        10,
        "every named support is withdrawn once"
    );
    for (path, replacement, finding, what) in withdrawals {
        let mut candidate = causal.clone();
        *at(&mut candidate, path) = replacement;
        says(&refused(&candidate, what), finding, what);
    }

    no_effect_needs_samples_and_a_comparison(&causal, SAMPLES, COMPARISON);
}

/// The second half of FR-056-AC-8: a no-effect conclusion needs positive
/// samples and an observed comparison, and is content with a null effect where
/// a causal conclusion is not.
fn no_effect_needs_samples_and_a_comparison(causal: &Value, samples: &str, comparison: &str) {
    let mut no_effect = causal.clone();
    no_effect["conclusion"]["kind"] = json!("no_effect_observed");
    no_effect["measured_effects"][0]["effect"] = Value::Null;
    accepted(&no_effect, "a no-effect conclusion with no stated effect");

    let mut unsampled = no_effect.clone();
    unsampled["baseline"]["sample_size"] = json!(0);
    says(
        &refused(&unsampled, "no-effect with no baseline sample"),
        samples,
        "no-effect samples",
    );

    let mut uncompared = no_effect;
    uncompared["measured_effects"][0]["treatment_value"] = Value::Null;
    says(
        &refused(&uncompared, "no-effect with no treatment value"),
        comparison,
        "no-effect comparison",
    );
}

/// The retained record, made into the causal claim the criterion admits.
fn causal_base() -> Value {
    let mut candidate = retained();
    candidate["conclusion"] = json!({
        "attribution_confidence": "moderate",
        "kind": "causal_effect_established",
        "statement": "The standalone sentinel detector raised the observed pass rate.",
    });
    candidate["interactions"] = json!([
        { "description": "live agent behavior", "disposition": "controlled" },
        { "description": "tool-selection order", "disposition": "not_applicable" },
    ]);
    candidate["confounders"] = json!([
        { "description": "session state", "disposition": "controlled" },
        { "description": "model-side nondeterminism", "disposition": "not_applicable" },
    ]);
    candidate["measured_effects"][0]["effect"] = json!(0.5);
    candidate["measured_effects"][0]["treatment_value"] = json!(1);
    candidate
}

/// Gaps, owner, actions and content-digested raw evidence are required; unsafe
/// paths and undeclared fields are refused, recursively, and nothing is
/// written.
///
/// > FR-056-AC-9: Gaps, owner, actions, and at least one content-digested
/// > raw-evidence reference with media type and byte size are retained; unsafe
/// > paths and undeclared fields are refused.
///
/// Trace: FR-056-AC-9
/// Provenance: quoin#479
#[test]
fn tc_479_108_governance_fields_unsafe_paths_and_undeclared_fields_are_refused() {
    /// Every object the schema closes, by the path an undeclared member is
    /// inserted at. `[]` is the document itself.
    const CLOSED: [&[&str]; 14] = [
        &[],
        &["subject"],
        &["producer"],
        &["design"],
        &["design", "assignment"],
        &["baseline"],
        &["treatments", "0"],
        &["changed_variables", "0"],
        &["held_constant", "0"],
        &["measured_effects", "0"],
        &["interactions", "0"],
        &["confounders", "0"],
        &["conclusion"],
        &["raw_evidence", "0"],
    ];

    /// The four raw-evidence members the criterion names.
    const EVIDENCE: [&str; 4] = ["path", "media_type", "size_bytes", "digest"];

    let base = accepted(&retained(), "the retained record");
    assert_eq!(base.gaps.len(), 5, "the gaps are retained");
    assert_eq!(base.owner, "engineering assurance");
    assert_eq!(base.actions.len(), 3, "the actions are retained");
    assert_eq!(
        base.raw_evidence.len(),
        2,
        "both evidence files are retained"
    );
    assert_eq!(base.raw_evidence[0].media_type.as_str(), "application/json");
    assert_eq!(base.raw_evidence[0].size_bytes, 11305);
    assert_eq!(
        base.raw_evidence[0].digest.as_str(),
        "sha256:741ab150c107d1f5551a9be2092081cc0439f85e804d23f81e3923dfab8fe076"
    );

    for member in ["gaps", "owner", "actions", "raw_evidence"] {
        required(&[member]);
    }
    assert_eq!(EVIDENCE.len(), 4, "every evidence member is measured");
    for member in EVIDENCE {
        required(&["raw_evidence", "0", member]);
    }
    for member in ["actions", "raw_evidence"] {
        let what = format!("an empty {member} list");
        names(&refused(&with(&[member], json!([])), &what), member, &what);
    }

    assert_eq!(CLOSED.len(), 14, "every closed object is measured");
    for path in CLOSED {
        let mut candidate = retained();
        at(&mut candidate, path)["unowned"] = json!(true);
        let what = format!("an undeclared field at /{}", path.join("/"));
        names(&refused(&candidate, &what), "unowned", &what);
    }
    // Anti-vacuity: `configuration` is the one object the schema leaves open,
    // so the refusals above are the closure and not a blanket rule.
    let mut open = retained();
    open["baseline"]["configuration"]["unowned"] = json!(true);
    accepted(&open, "an unmodelled member of an open configuration");

    refuses_unsafe_paths_without_writing();
}

/// Every unsafe raw-evidence path, and the byte mismatch, refused at intake
/// with nothing left in the store.
fn refuses_unsafe_paths_without_writing() {
    /// Paths the schema admits and `resolveRawPath` refuses.
    const UNSAFE: [&str; 6] = [
        "../outside.json",
        "/absolute.json",
        "agent-evals/./baseline.json",
        "agent-evals/../agent-evals/baseline.json",
        "agent-evals/baseline.json/",
        "agent-evals\\baseline.json",
    ];

    let temporary = tempfile::tempdir().expect("a temporary repository");
    let root = temporary.path();
    copy_tree(&repo_root().join("spec"), &root.join("spec"));
    std::fs::remove_file(
        root.join("spec")
            .join("evidence")
            .join("interventions")
            .join("quoin-270-cli-eval-sentinel-contract.json"),
    )
    .expect("the retained record is removable from the copy");
    assert!(
        read_intervention_records(&DiskMeasurement::new(root))
            .expect("the copied store is readable")
            .is_empty(),
        "the copied store starts empty"
    );

    assert_eq!(UNSAFE.len(), 6, "every unsafe spelling is measured");
    for path in UNSAFE {
        refused_write(
            root,
            &with(&["raw_evidence", "0", "path"], json!(path)),
            InterventionRefusalCode::RawEvidenceMismatch,
            &format!("raw evidence at {path:?}"),
        );
    }

    // A reference whose declared bytes are not the retained file's.
    let mut mismatched = retained();
    mismatched["raw_evidence"][0]["size_bytes"] = json!(11306);
    mismatched["raw_evidence"][1]["digest"] = json!(format!("sha256:{}", "f".repeat(64)));
    refused_write(
        root,
        &mismatched,
        InterventionRefusalCode::RawEvidenceMismatch,
        "a raw-evidence byte mismatch",
    );

    // An undeclared field is refused before anything is written, too.
    let mut undeclared = retained();
    undeclared["unowned"] = json!(true);
    refused_write(
        root,
        &undeclared,
        InterventionRefusalCode::InvalidRecord,
        "an undeclared field at intake",
    );

    // Anti-vacuity: the store was writable throughout, so "nothing written"
    // above is the refusals and not a store that refuses everything.
    let bridged =
        quoin_measurement::json_bridge::from_serde(&retained()).expect("the record crosses");
    write_intervention_record(root, &bridged).expect("the retained record is publishable");
    assert_eq!(
        read_intervention_records(&DiskMeasurement::new(root))
            .expect("the store is readable")
            .len(),
        1,
        "the valid record is what the store holds"
    );
}

/// Assert intake refuses this candidate under this code, and writes nothing.
fn refused_write(root: &Path, candidate: &Value, expected: InterventionRefusalCode, what: &str) {
    let bridged =
        quoin_measurement::json_bridge::from_serde(candidate).expect("the candidate crosses");
    let error = write_intervention_record(root, &bridged)
        .err()
        .unwrap_or_else(|| panic!("{what}: published, and the criterion says it is refused"));
    assert_eq!(error.code(), expected, "{what}: {:?}", error.findings());
    assert!(
        read_intervention_records(&DiskMeasurement::new(root))
            .expect("the store is readable")
            .is_empty(),
        "{what}: a refused record reached the store"
    );
}
