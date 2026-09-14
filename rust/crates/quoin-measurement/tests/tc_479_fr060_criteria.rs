// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every FR-060 acceptance criterion the deleted `tests/operational.test.ts`
//! carried, restated against this crate.
//!
//! Trace: FR-060-AC-1, FR-060-AC-2, FR-060-AC-3, FR-060-AC-4, FR-060-AC-5,
//! Trace: FR-060-AC-6, FR-060-AC-7, FR-060-AC-8, FR-060-AC-9, FR-060-AC-10,
//! Trace: FR-060-CON-1, FR-060-CON-2
//! Provenance: quoin#479
//!
//! # Why this file exists
//!
//! FR-101 retires `src/measurement/` after parity, and `tests/operational.test.ts`
//! is deleted with it. Every acceptance criterion that file carried has to be
//! restated against the Rust port before the TypeScript goes, or the criterion
//! is retired rather than ported. One `#[test]` per criterion, each tagged at
//! criterion level so the coverage matrix can bind it.
//!
//! # Where the inputs come from
//!
//! Not from fixtures this file wrote. The base capability and exercise are the
//! two records of the one committed pair,
//! `spec/evidence/operational/pairs/c1b3….json`, produced by the retained
//! TypeScript from the retained GitHub exports; the governing plan is the
//! committed `spec/assurance/MP-221-…md`; the raw-evidence files are the
//! committed `spec/evidence/github-actions/` exports. Every refusal case is a
//! **mutation of that committed base**, so an expected refusal is a statement
//! about real evidence rather than about a fixture written to be refused.
//!
//! Each test runs against a temporary repository holding copies of those
//! inputs. Nothing here writes into this repository.

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

use serde_json::{Value, json};

use quoin_measurement::intervention::intake::InterventionRefusalCode;
use quoin_measurement::operational::discharge::{Discharged, operational_discharge};
use quoin_measurement::operational::intake::{write_operational_pair, write_operational_record};
use quoin_measurement::operational::lock::LOCK_DIRECTORY_NAME;
use quoin_measurement::operational::paths::{operational_pairs_root, operational_root};
use quoin_measurement::operational::read::read_operational_records;
use quoin_measurement::operational::record::{OperationalEvidenceRecord, OperationalObligation};
use quoin_measurement::operational::report::{
    OperationalReportEntry, build_operational_report, render_operational_report,
};
use quoin_measurement::operational::validate::validate_operational_record;
use quoin_measurement::report::build::build_measurement_report_from;
use quoin_measurement::source::{Clock, SystemClock};
use quoin_measurement::{
    DiskMeasurement, build_measurement_report, render_measurement_report,
    render_measurement_report_json,
};
use quoin_store::StoreError;
use quoin_store::store::write_content_addressed;

/// The one pair `spec/evidence/operational/pairs/` retains.
const RETAINED_PAIR: &str = "c1b30a188d4d03bbe316e0fdb7582eff3fa314c55268a5f71f81f99f8ea2acf8.json";

/// A retained measurement collection, for the no-migration criterion.
const RETAINED_COLLECTION: &str = "tier1-20260829134513176-99c26bc4589f.json";

/// A retained intervention record, for the no-migration criterion.
const RETAINED_INTERVENTION: &str = "quoin-270-cli-eval-sentinel-contract.json";

/// The three retained exports both records reference, with their digests.
const RETAINED_DIGESTS: [(&str, &str); 3] = [
    (
        "github-actions/quoin-271-release-v0.22.5-workflow.yml",
        "sha256:5a867277a071c2dd8fe1ab86e22b4b3e580fff0b269756c3911c4dde2ed1cc78",
    ),
    (
        "github-actions/quoin-271-release-v0.22.5-run.json",
        "sha256:a3a3eea43f6f17fc9851a50aa6853aa1d67a77ad91c6b4ddc1922aa03c4acaaf",
    ),
    (
        "github-actions/quoin-271-release-v0.22.5-jobs.json",
        "sha256:adb0e51b5212d9ff4f01238a26040f8ea893c75f6bc02cfbc4b7d98552182e92",
    ),
];

/// A temporary repository carrying the committed governance and raw evidence.
fn temporary_repository() -> tempfile::TempDir {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let root = temporary.path();
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
    temporary
}

/// The committed pair's capability and exercise, as documents.
fn pair_records() -> (Value, Value) {
    let path = repo_root()
        .join("spec")
        .join("evidence")
        .join("operational")
        .join("pairs")
        .join(RETAINED_PAIR);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let envelope: Value = serde_json::from_slice(&bytes).expect("the retained pair is JSON");
    let records = envelope["records"]
        .as_array()
        .expect("the envelope carries records");
    assert_eq!(records.len(), 2, "a pair carries two records");
    (records[0].clone(), records[1].clone())
}

/// Every `.json` file directly inside a directory, sorted.
fn json_files(directory: &Path) -> Vec<PathBuf> {
    let Ok(listing) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = listing
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|end| end == "json"))
        .collect();
    found.sort();
    found
}

/// Replace the value at `pointer`, which must already be present.
fn patch(base: &Value, pointer: &str, value: Value) -> Value {
    let mut out = base.clone();
    let slot = out
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("{pointer} is present in the committed record"));
    *slot = value;
    out
}

/// Remove `key` from the object at `pointer` (`""` for the document root).
fn remove(base: &Value, pointer: &str, key: &str) -> Value {
    let mut out = base.clone();
    let slot = out
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("{pointer} is present in the committed record"));
    slot.as_object_mut()
        .unwrap_or_else(|| panic!("{pointer} is an object"))
        .remove(key)
        .unwrap_or_else(|| panic!("{pointer}/{key} is present"));
    out
}

/// The typed record behind a document the port accepts.
fn record_of(document: &Value) -> OperationalEvidenceRecord {
    validate_operational_record(document)
        .unwrap_or_else(|error| panic!("the document validates: {error}"))
        .record()
        .clone()
}

fn obligation_of(document: &Value) -> OperationalObligation {
    serde_json::from_value(document.clone()).expect("the obligation reads")
}

/// The obligation the committed exercise is against.
fn obligation() -> Value {
    json!({
        "control_kind": "release",
        "subject": {
            "id": "agent-ix/quoin",
            "revision": "a9808be18b61f8e4d44e3b74de27e90f17c5c76b"
        },
        "scope": {
            "service": "agent-ix/quoin",
            "environment": "npm-production",
            "population": "v0.22.5 package release"
        },
        "accepted_modes": ["actual"],
        "clock": {
            "applicability": "operational_with_clock",
            "started_at": "2026-08-29T23:11:00Z",
            "deadline_at": "2026-08-29T23:21:00.000Z"
        }
    })
}

/// A clock that never sleeps: every wait advances its own reading instead.
struct VirtualClock {
    millis: std::cell::Cell<i64>,
}

impl Clock for VirtualClock {
    fn now_millis(&self) -> i64 {
        self.millis.get()
    }

    fn wait(&self, millis: u32) {
        self.millis.set(self.millis.get() + i64::from(millis));
    }
}

/// Valid intake writes one complete canonical record by one atomic
/// same-directory no-replace publication.
///
/// Trace: FR-060-AC-1
/// Provenance: quoin#479
#[test]
fn tc_479_400_valid_intake_publishes_one_complete_canonical_record() {
    let workspace = temporary_repository();
    let root = workspace.path();
    let (capability, _) = pair_records();

    let path = write_operational_record(root, &SystemClock, &capability)
        .expect("the committed capability is admissible against its own evidence");

    // One file, and no temporary left beside it: the publication is atomic in
    // the directory it lands in.
    let listing: Vec<PathBuf> = std::fs::read_dir(operational_root(root))
        .expect("the store directory is readable")
        .map(|entry| entry.expect("a directory entry").path())
        .collect();
    assert_eq!(
        listing,
        vec![path.clone()],
        "the store holds one entry only"
    );

    let retained = std::fs::read(&path).expect("the retained record is readable");
    // Complete: every member of the candidate survived the round trip.
    assert_eq!(
        serde_json::from_slice::<Value>(&retained).expect("the retained record is JSON"),
        capability,
        "the retained document is not the document that was admitted"
    );
    // Canonical: the store's one serialisation, keys sorted — `actions` is the
    // first member of a sorted operational record — and newline-terminated.
    assert!(
        retained.starts_with(b"{\n  \"actions\": ["),
        "the retained bytes are not the store's canonical JSON"
    );
    assert!(
        retained.ends_with(b"}\n"),
        "the retained bytes are not newline-terminated"
    );

    // No-replace: different bytes at the same destination are refused and the
    // retained bytes do not move.
    let refused = write_content_addressed(&path, b"{}").expect_err("a replacement is refused");
    assert!(matches!(refused, StoreError::ContentCollision { .. }));
    assert_eq!(std::fs::read(&path).unwrap(), retained);
    assert!(
        !write_content_addressed(&path, &retained).expect("identical bytes are idempotent"),
        "republishing identical bytes reported a new write"
    );

    let records = read_operational_records(root).expect("the store reads back");
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].record_id().as_str(),
        "quoin-271-release-v0.22.5-capability"
    );
}

/// Invalid schema, cross-record, or temporal input returns `invalid_record`;
/// unsafe, missing, wrong-sized, or digest-mismatched raw evidence returns
/// `raw_evidence_mismatch`; both identify every mismatch and write nothing.
///
/// Trace: FR-060-AC-2
/// Provenance: quoin#479
#[test]
fn tc_479_401_invalid_records_and_mismatched_raw_evidence_write_nothing() {
    /// Below this the case walk is asserting nothing.
    const CASE_FLOOR: usize = 10;

    let cases = intake_refusal_cases();
    assert!(
        cases.len() >= CASE_FLOOR,
        "anti-vacuity floor: {} refusal cases, below {CASE_FLOOR}",
        cases.len()
    );
    for (label, candidate, code, finding) in &cases {
        let workspace = temporary_repository();
        let root = workspace.path();
        let error = write_operational_record(root, &SystemClock, candidate)
            .expect_err(&format!("{label} must be refused"));
        assert_eq!(error.code(), *code, "{label}: {error}");
        assert!(
            error.findings().iter().any(|item| item.contains(finding)),
            "{label}: no finding named {finding}: {error}"
        );
        assert!(
            json_files(&operational_root(root)).is_empty(),
            "{label} wrote a store entry"
        );
    }
}

/// The refused mutations of the committed records, with what each must say.
fn intake_refusal_cases() -> Vec<(&'static str, Value, InterventionRefusalCode, &'static str)> {
    use InterventionRefusalCode::{InvalidRecord, RawEvidenceMismatch};
    let (capability, exercise) = pair_records();
    vec![
        (
            "a record missing a required member",
            remove(&capability, "", "owner"),
            InvalidRecord,
            "owner",
        ),
        (
            "a control kind outside the vocabulary",
            patch(&capability, "/control_kind", json!("unknown-control")),
            InvalidRecord,
            "/control_kind",
        ),
        (
            "a pin control kind with no matching pin",
            patch(&capability, "/control_kind", json!("model_pin")),
            InvalidRecord,
            "/configuration/version_pins: model_pin requires a matching pin kind",
        ),
        (
            "an unsupported clock still carrying its bounds",
            patch(
                &capability,
                "/capability/clock_support",
                json!({ "supported": false, "deadline_seconds": 600 }),
            ),
            InvalidRecord,
            "/capability/clock_support: unsupported clock excludes event/deadline fields",
        ),
        (
            "an exercise observed before it completed",
            patch(&exercise, "/observed_at", json!("2026-08-29T23:11:00Z")),
            InvalidRecord,
            "/observed_at: precedes exercise completion",
        ),
        (
            "an exercise that completed before it started",
            patch(
                &exercise,
                "/exercise/started_at",
                json!("2026-08-29T23:11:38Z"),
            ),
            InvalidRecord,
            "/exercise/completed_at: precedes exercise start",
        ),
        (
            "a recorded clock status the instants disagree with",
            patch(&exercise, "/exercise/clock/status", json!("missed")),
            InvalidRecord,
            "/exercise/clock/status: missed disagrees with derived met",
        ),
        (
            "an exercise naming a capability the store does not hold",
            exercise.clone(),
            InvalidRecord,
            "/exercise/capability_record_id: linked capability does not match",
        ),
        (
            "a raw-evidence digest that is not the file's",
            patch(
                &capability,
                "/raw_evidence/0/digest",
                json!("sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
            ),
            RawEvidenceMismatch,
            "/raw_evidence/0/digest: expected",
        ),
        (
            "a raw-evidence size that is not the file's",
            patch(&capability, "/raw_evidence/1/size_bytes", json!(1)),
            RawEvidenceMismatch,
            "/raw_evidence/1/size_bytes: expected",
        ),
        (
            "a raw-evidence path that escapes the store",
            patch(
                &capability,
                "/raw_evidence/2/path",
                json!("../../../etc/passwd"),
            ),
            RawEvidenceMismatch,
            "/raw_evidence/2/path:",
        ),
        (
            "a raw-evidence path naming no retained file",
            patch(
                &capability,
                "/raw_evidence/0/path",
                json!("github-actions/absent.yml"),
            ),
            RawEvidenceMismatch,
            "/raw_evidence/0/path:",
        ),
    ]
}

/// An absent governing plan returns `governing_plan_absent`; a mismatch returns
/// `definition_mismatch`; both write nothing and name the requested, expected,
/// and observed definitions that apply.
///
/// Trace: FR-060-AC-3
/// Provenance: quoin#479
#[test]
fn tc_479_402_governing_definition_refusals_name_the_versions_and_write_nothing() {
    let (capability, _) = pair_records();

    // No active MeasurementPlan at all: the governance check has nothing to
    // admit the record against.
    let ungoverned = tempfile::tempdir().expect("a temporary directory");
    copy_tree(
        &repo_root()
            .join("spec")
            .join("evidence")
            .join("github-actions"),
        &ungoverned
            .path()
            .join("spec")
            .join("evidence")
            .join("github-actions"),
    );
    let error = write_operational_record(ungoverned.path(), &SystemClock, &capability)
        .expect_err("an ungoverned definition is refused");
    assert_eq!(error.code(), InterventionRefusalCode::GoverningPlanAbsent);
    assert_eq!(
        error.findings(),
        [
            "requested definition github-actions.release-operational-v1; no active \
             MeasurementPlan exists"
        ]
    );
    assert!(json_files(&operational_root(ungoverned.path())).is_empty());

    // Plans exist, but none names this definition.
    let workspace = temporary_repository();
    let root = workspace.path();
    let mismatched = patch(
        &capability,
        "/producer/definition_version",
        json!("github-actions.release-operational-v2"),
    );
    let error = write_operational_record(root, &SystemClock, &mismatched)
        .expect_err("an unnamed definition is refused");
    assert_eq!(error.code(), InterventionRefusalCode::DefinitionMismatch);
    let finding = &error.findings()[0];
    assert!(
        finding.starts_with(
            "requested definition github-actions.release-operational-v2; expected one of "
        ),
        "the refusal must name the requested definition: {finding}"
    );
    assert!(
        finding.contains("github-actions.release-operational-v1"),
        "the refusal must name the expected definitions: {finding}"
    );
    assert!(
        finding.ends_with("; observed github-actions.release-operational-v2"),
        "the refusal must name the observed definition: {finding}"
    );
    assert!(json_files(&operational_root(root)).is_empty());
}

/// Repeating an identical record id and canonical payload is byte-idempotent
/// whether the retained record is standalone or a member of an atomic pair.
///
/// Trace: FR-060-AC-4
/// Provenance: quoin#479
#[test]
fn tc_479_403_identical_bytes_are_idempotent_standalone_and_inside_a_pair() {
    let (capability, exercise) = pair_records();

    // Standalone.
    let standalone = temporary_repository();
    let root = standalone.path();
    let first = write_operational_record(root, &SystemClock, &capability).expect("the first write");
    let bytes = std::fs::read(&first).expect("the retained bytes");
    let second =
        write_operational_record(root, &SystemClock, &capability).expect("the repeated write");
    assert_eq!(second, first, "a repeated write moved the record");
    assert_eq!(std::fs::read(&first).unwrap(), bytes, "the bytes changed");
    assert_eq!(json_files(&operational_root(root)).len(), 1);

    // Inside a pair: the pair itself, and each member republished on its own.
    let paired = temporary_repository();
    let root = paired.path();
    let pair = write_operational_pair(root, &SystemClock, &capability, &exercise)
        .expect("the committed pair is admissible");
    let pair_bytes = std::fs::read(&pair).expect("the retained pair");
    assert_eq!(
        write_operational_pair(root, &SystemClock, &capability, &exercise).expect("repeated"),
        pair,
        "a repeated pair write moved the pair"
    );
    for member in [&capability, &exercise] {
        assert_eq!(
            write_operational_record(root, &SystemClock, member).expect("a retained member"),
            pair,
            "a pair member republished on its own did not resolve to the retained pair"
        );
    }
    assert_eq!(std::fs::read(&pair).unwrap(), pair_bytes);
    assert_eq!(json_files(&operational_pairs_root(root)).len(), 1);
    assert!(json_files(&operational_root(root)).is_empty());
    assert_eq!(read_operational_records(root).expect("read back").len(), 2);
}

/// Reusing a record id for different semantic bytes returns
/// `record_id_collision` without replacement; store-wide serialization prevents
/// standalone/pair writers from retaining that logical id twice, and a stale
/// intake lock fails closed as `intake_busy`.
///
/// Trace: FR-060-AC-5
/// Provenance: quoin#479
#[test]
fn tc_479_404_one_logical_id_cannot_be_retained_twice_and_a_stale_lock_fails_closed() {
    let (capability, exercise) = pair_records();
    let different = patch(&capability, "/owner", json!("someone else"));

    // One destination, two byte-sets.
    let workspace = temporary_repository();
    let root = workspace.path();
    let path = write_operational_record(root, &SystemClock, &capability).expect("the first write");
    let bytes = std::fs::read(&path).expect("the retained bytes");
    let error = write_operational_record(root, &SystemClock, &different)
        .expect_err("a reused identity is refused");
    assert_eq!(error.code(), InterventionRefusalCode::RecordIdCollision);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        bytes,
        "a refused write replaced the retained record"
    );

    // Two containers, two real threads: the standalone writer and the pair
    // writer both claim the capability identity, and the store-wide lock
    // decides. Whichever loses refuses; neither duplicates.
    let raced = temporary_repository();
    let root = raced.path();
    let outcomes = std::thread::scope(|scope| {
        let standalone =
            scope.spawn(|| write_operational_record(root, &SystemClock, &capability).is_ok());
        let paired = scope
            .spawn(|| write_operational_pair(root, &SystemClock, &different, &exercise).is_ok());
        [
            standalone.join().expect("the standalone writer"),
            paired.join().expect("the pair writer"),
        ]
    });
    assert_eq!(
        outcomes.iter().filter(|ok| **ok).count(),
        1,
        "exactly one writer may retain the contested identity, saw {outcomes:?}"
    );
    let mut retained: Vec<String> = read_operational_records(root)
        .expect("the raced store reads back")
        .iter()
        .map(|record| record.record_id().to_string())
        .collect();
    let listed = retained.len();
    retained.sort_unstable();
    retained.dedup();
    assert_eq!(retained.len(), listed, "a logical id was retained twice");
    assert!(retained.contains(&"quoin-271-release-v0.22.5-capability".to_owned()));

    // A lock nobody holds is still a lock: intake fails closed rather than
    // clearing it.
    let stale = temporary_repository();
    let root = stale.path();
    std::fs::create_dir_all(root.join("spec").join("evidence").join(LOCK_DIRECTORY_NAME))
        .expect("a stale lock directory");
    let clock = VirtualClock {
        millis: std::cell::Cell::new(0),
    };
    let error = write_operational_record(root, &clock, &capability)
        .expect_err("a held lock refuses the writer");
    assert_eq!(error.code(), InterventionRefusalCode::IntakeBusy);
    assert!(json_files(&operational_root(root)).is_empty());
}

/// Standing capabilities and succeeded, failed, partial, and aborted exercises
/// remain independently queryable with their raw-evidence digests.
///
/// Trace: FR-060-AC-6
/// Provenance: quoin#479
#[test]
fn tc_479_405_capabilities_and_every_exercise_outcome_stay_queryable() {
    /// Every outcome the vocabulary admits.
    const OUTCOMES: [&str; 4] = ["succeeded", "failed", "partial", "aborted"];

    let workspace = temporary_repository();
    let root = workspace.path();
    let (capability, exercise) = pair_records();
    write_operational_record(root, &SystemClock, &capability).expect("the standing capability");

    for outcome in OUTCOMES {
        let candidate = patch(&exercise, "/exercise/outcome", json!(outcome));
        let candidate = patch(
            &candidate,
            "/record_id",
            json!(format!("quoin-271-release-v0.22.5-exercise-{outcome}")),
        );
        write_operational_record(root, &SystemClock, &candidate)
            .unwrap_or_else(|error| panic!("the {outcome} exercise is admissible: {error}"));
    }

    let records = read_operational_records(root).expect("the store reads back");
    assert_eq!(
        records.len(),
        OUTCOMES.len() + 1,
        "every record must remain independently queryable"
    );
    for record in &records {
        let claimed: Vec<(&str, &str)> = record
            .record()
            .base()
            .raw_evidence
            .iter()
            .map(|item| (item.path.as_str(), item.digest.as_str()))
            .collect();
        assert_eq!(
            claimed,
            RETAINED_DIGESTS.to_vec(),
            "{} lost its raw-evidence digests",
            record.record_id()
        );
    }
}

/// A clocked obligation discharges only from a control-kind, subject, scope,
/// and accepted-mode-matched exercise whose clock start/deadline identify the
/// exact obligation clock condition, whose outcome is `succeeded`, and whose
/// completion falls within that condition; every other case remains a named
/// non-discharge or gap.
///
/// Trace: FR-060-AC-7
/// Provenance: quoin#479
#[test]
fn tc_479_406_discharge_requires_full_identity_mode_success_and_a_met_clock() {
    /// Below this the non-discharge walk is asserting nothing.
    const CASE_FLOOR: usize = 10;

    let (capability, exercise) = pair_records();
    let valid = validate_operational_record(&exercise).expect("the committed exercise");
    assert_eq!(
        operational_discharge(&valid, &obligation_of(&obligation())),
        Ok(Discharged),
        "the committed exercise discharges the obligation it was produced for"
    );
    assert_eq!(
        Discharged::REASON,
        "matched succeeded exercise completed within clock"
    );

    let cases = non_discharge_cases(&capability, &exercise);
    assert!(
        cases.len() >= CASE_FLOOR,
        "anti-vacuity floor: {} non-discharge cases, below {CASE_FLOOR}",
        cases.len()
    );
    for (label, record, against, reason) in &cases {
        let valid = validate_operational_record(record)
            .unwrap_or_else(|error| panic!("{label}: the record validates: {error}"));
        let refused = operational_discharge(&valid, &obligation_of(against))
            .expect_err(&format!("{label} must not discharge"));
        assert_eq!(refused.reason(), *reason, "{label}");
    }

    // A record whose clock claims more than its instants support never reaches
    // discharge at all: it cannot be validated, so no exercise exists to ask.
    let forged = patch(
        &exercise,
        "/exercise/clock/completed_at",
        json!("2026-08-29T23:31:00Z"),
    );
    let error = validate_operational_record(&forged).expect_err("a forged clock is refused");
    assert!(
        error
            .findings()
            .iter()
            .any(|item| item == "/exercise/clock/status: met disagrees with derived missed"),
        "{error}"
    );
}

/// Every case that must remain a named non-discharge.
fn non_discharge_cases(
    capability: &Value,
    exercise: &Value,
) -> Vec<(&'static str, Value, Value, &'static str)> {
    let open = patch(
        &remove(exercise, "/exercise/clock", "completed_at"),
        "/exercise/clock/status",
        json!("open"),
    );
    vec![
        (
            "a standing capability is not an exercise",
            capability.clone(),
            obligation(),
            "invalid operational exercise: record is a standing capability",
        ),
        (
            "a different control kind",
            exercise.clone(),
            patch(&obligation(), "/control_kind", json!("kill_switch")),
            "control_kind mismatch",
        ),
        (
            "a different subject",
            exercise.clone(),
            patch(&obligation(), "/subject/id", json!("agent-ix/quire")),
            "subject mismatch",
        ),
        (
            "a different scope",
            exercise.clone(),
            patch(&obligation(), "/scope/environment", json!("staging")),
            "scope mismatch",
        ),
        (
            "a mode the obligation does not accept",
            exercise.clone(),
            patch(&obligation(), "/accepted_modes", json!(["drill"])),
            "exercise mode mismatch",
        ),
        (
            "a failed exercise",
            patch(exercise, "/exercise/outcome", json!("failed")),
            obligation(),
            "exercise outcome failed",
        ),
        (
            "a partial exercise",
            patch(exercise, "/exercise/outcome", json!("partial")),
            obligation(),
            "exercise outcome partial",
        ),
        (
            "an aborted exercise",
            patch(exercise, "/exercise/outcome", json!("aborted")),
            obligation(),
            "exercise outcome aborted",
        ),
        (
            "an obligation whose clock condition is unreadable",
            exercise.clone(),
            patch(&obligation(), "/clock/started_at", json!("yesterday")),
            "invalid obligation clock condition",
        ),
        (
            "an obligation whose deadline precedes its start",
            exercise.clone(),
            patch(
                &obligation(),
                "/clock/deadline_at",
                json!("2026-08-29T22:00:00Z"),
            ),
            "invalid obligation clock condition",
        ),
        (
            "an exercise no clock applies to",
            patch(
                exercise,
                "/exercise/clock",
                json!({ "applicability": "not_applicable", "status": "not_applicable" }),
            ),
            obligation(),
            "clock status not_applicable",
        ),
        (
            "an exercise against a different clock condition",
            exercise.clone(),
            patch(
                &obligation(),
                "/clock/deadline_at",
                json!("2026-08-29T23:22:00Z"),
            ),
            "obligation clock condition mismatch",
        ),
        (
            "an exercise still running",
            open,
            obligation(),
            "exercise did not complete within obligation clock",
        ),
    ]
}

/// A capability record of one status, under its own identity.
fn capability_with(base: &Value, id: &str, status: &str) -> Value {
    patch(
        &patch(base, "/record_id", json!(id)),
        "/capability/status",
        json!(status),
    )
}

/// An exercise record of one outcome and clock, under its own identity.
fn exercise_with(base: &Value, id: &str, outcome: &str, clock: Value) -> Value {
    let record = patch(
        &patch(base, "/record_id", json!(id)),
        "/exercise/clock",
        clock,
    );
    patch(&record, "/exercise/outcome", json!(outcome))
}

/// The eight records the projection has to separate, and what each must say.
type ReportCase = (&'static str, Value, [Vec<String>; 4]);

fn report_cases() -> Vec<ReportCase> {
    let mut cases = capability_report_cases();
    cases.extend(exercise_report_cases());
    cases
}

/// Turn four literal groups into the four owned line lists a case carries.
fn lines(values: [&[&str]; 4]) -> [Vec<String>; 4] {
    values.map(|group| group.iter().map(|item| (*item).to_owned()).collect())
}

/// The four capability states.
fn capability_report_cases() -> Vec<ReportCase> {
    let (capability, _) = pair_records();
    vec![
        (
            "available",
            capability_with(&capability, "capability-available", "available"),
            lines([
                &["release control quoin-npm-release is available for agent-ix/quoin"],
                &[
                    "surface .github/workflows/release.yml; coverage manual npm publication of \
                     the exact tagged @agent-ix/quoin version",
                ],
                &[],
                &[],
            ]),
        ),
        (
            "unknown",
            capability_with(&capability, "capability-unknown", "unknown"),
            lines([
                &[],
                &[],
                &[],
                &["capability state is unknown for quoin-npm-release"],
            ]),
        ),
        (
            "unavailable",
            capability_with(&capability, "capability-unavailable", "unavailable"),
            lines([
                &[],
                &[],
                &["quoin-npm-release capability is unavailable"],
                &[],
            ]),
        ),
        (
            "not_applicable",
            capability_with(&capability, "capability-not-applicable", "not_applicable"),
            lines([
                &[],
                &[],
                &["quoin-npm-release capability is not_applicable"],
                &[],
            ]),
        ),
    ]
}

/// The four exercise outcomes, each against the clock it ran under.
fn exercise_report_cases() -> Vec<ReportCase> {
    let (_, exercise) = pair_records();
    let met = exercise["exercise"]["clock"].clone();
    let open = json!({
        "applicability": "operational_with_clock",
        "started_at": "2026-08-29T23:11:00Z",
        "deadline_at": "2026-08-29T23:21:00.000Z",
        "status": "open"
    });
    let missed = json!({
        "applicability": "operational_with_clock",
        "started_at": "2026-08-29T23:11:00Z",
        "deadline_at": "2026-08-29T23:21:00.000Z",
        "completed_at": "2026-08-29T23:31:00Z",
        "status": "missed"
    });
    let late = patch(
        &patch(&exercise, "/observed_at", json!("2026-08-29T23:35:00Z")),
        "/exercise/completed_at",
        json!("2026-08-29T23:31:00Z"),
    );
    vec![
        (
            "succeeded and met",
            exercise_with(&exercise, "exercise-succeeded", "succeeded", met.clone()),
            lines([
                &["release control quoin-npm-release was exercised successfully"],
                &["actual exercise completed 2026-08-29T23:11:37Z; clock met"],
                &[],
                &[],
            ]),
        ),
        (
            "failed inside the clock",
            exercise_with(&exercise, "exercise-failed", "failed", met),
            lines([
                &[],
                &[],
                &["quoin-npm-release exercise is failed; clock met"],
                &[],
            ]),
        ),
        (
            "partial with the clock still open",
            exercise_with(&exercise, "exercise-partial", "partial", open),
            lines([
                &[],
                &[],
                &[],
                &["quoin-npm-release exercise is partial; clock open"],
            ]),
        ),
        (
            "aborted past the deadline",
            exercise_with(&late, "exercise-aborted", "aborted", missed),
            lines([
                &[],
                &[],
                &["quoin-npm-release exercise is aborted; clock missed"],
                &[],
            ]),
        ),
    ]
}

fn entry_for<'a>(entries: &'a [OperationalReportEntry], id: &str) -> &'a OperationalReportEntry {
    entries
        .iter()
        .find(|entry| entry.record_id.as_str() == id)
        .unwrap_or_else(|| panic!("{id} is projected"))
}

/// The report renders only available capabilities and succeeded,
/// clock-satisfying exercises as their distinct claims and evidence;
/// unavailable, unknown, not-applicable, adverse-outcome, missed, or open
/// states render as counterevidence or gaps beside owner and actions.
///
/// Trace: FR-060-AC-8
/// Provenance: quoin#479
#[test]
fn tc_479_407_only_available_capabilities_and_met_successes_render_as_claims() {
    let cases = report_cases();
    let records: Vec<OperationalEvidenceRecord> = cases
        .iter()
        .map(|(_, record, _)| record_of(record))
        .collect();
    let entries = build_operational_report(&records);
    assert_eq!(entries.len(), cases.len());

    for (label, record, [claims, evidence, counterevidence, gaps]) in &cases {
        let id = record["record_id"].as_str().expect("an identity");
        let entry = entry_for(&entries, id);
        assert_eq!(&entry.claims, claims, "{label}: claims");
        assert_eq!(&entry.evidence, evidence, "{label}: evidence");
        assert_eq!(
            &entry.counterevidence, counterevidence,
            "{label}: counterevidence"
        );
        // The record's own declared gap travels with the projected ones.
        let mut expected = vec![
            "npm registry receipt is not part of this first producer's retained input contract"
                .to_owned(),
        ];
        expected.extend(gaps.iter().cloned());
        assert_eq!(&entry.gaps, &expected, "{label}: gaps");
        assert_eq!(entry.owner, "engineering assurance", "{label}: owner");
        assert_eq!(
            entry.actions,
            [
                "retain a later adverse release run to exercise the non-success operational path \
              with real evidence"
            ],
            "{label}: actions"
        );
    }

    let rendered = render_operational_report(&entries);
    for heading in [
        "## Operational evidence",
        "#### Claims",
        "#### Evidence",
        "#### Counterevidence",
        "#### Gaps",
        "#### Owner",
        "#### Actions",
    ] {
        assert!(rendered.contains(heading), "the render dropped {heading}");
    }
    assert!(rendered.contains("No affirmative claim."));
}

/// Neither human nor JSON output contains an aggregate trust, confidence, or
/// quality score derived from operational records.
///
/// Trace: FR-060-AC-9
/// Provenance: quoin#479
#[test]
fn tc_479_408_no_derived_trust_confidence_or_quality_score() {
    /// Every member an operational report entry carries, and no other.
    const MEMBERS: [&str; 11] = [
        "actions",
        "claims",
        "control_kind",
        "counterevidence",
        "evidence",
        "gaps",
        "observed_at",
        "owner",
        "raw_evidence",
        "record_id",
        "record_shape",
    ];
    /// No aggregate may be derived under any of these names.
    const FORBIDDEN: [&str; 5] = ["trust", "confidence", "score", "rating", "overall"];

    let workspace = temporary_repository();
    let root = workspace.path();
    let records: Vec<OperationalEvidenceRecord> = report_cases()
        .iter()
        .map(|(_, record, _)| record_of(record))
        .collect();
    let report = build_measurement_report_from(root, &[], &[], &[], &records)
        .expect("the report builds from the snapshot");

    let human = render_operational_report(&report.operational).to_lowercase();
    for needle in FORBIDDEN {
        assert!(!human.contains(needle), "the human view derives a {needle}");
    }

    let json: Value = serde_json::from_str(
        &render_measurement_report_json(&report).expect("the JSON view renders"),
    )
    .expect("the JSON view is JSON");
    let operational = json["operational"]
        .as_array()
        .expect("the operational view");
    assert_eq!(operational.len(), records.len());
    for entry in operational {
        let mut members: Vec<&str> = entry
            .as_object()
            .expect("an entry is an object")
            .keys()
            .map(String::as_str)
            .collect();
        members.sort_unstable();
        assert_eq!(members, MEMBERS, "an entry carries an unexpected member");
    }
    let serialised = serde_json::to_string(operational)
        .expect("the operational view serialises")
        .to_lowercase();
    for needle in FORBIDDEN {
        assert!(
            !serialised.contains(needle),
            "the JSON view derives a {needle}"
        );
    }
}

/// Re-rendering an unchanged store is byte-identical, and human and JSON views
/// expose the same claims, evidence, counterevidence, gaps, owners, and
/// actions.
///
/// Trace: FR-060-AC-10
/// Provenance: quoin#479
#[test]
fn tc_479_409_re_rendering_is_byte_identical_and_the_two_views_agree() {
    let workspace = temporary_repository();
    let root = workspace.path();
    let records: Vec<OperationalEvidenceRecord> = report_cases()
        .iter()
        .map(|(_, record, _)| record_of(record))
        .collect();
    let mut reordered = records.clone();
    reordered.reverse();
    assert_ne!(
        records[0].base().record_id,
        reordered[0].base().record_id,
        "the reordering must actually reorder"
    );

    let first = build_measurement_report_from(root, &[], &[], &[], &records).expect("a report");
    let again = build_measurement_report_from(root, &[], &[], &[], &records).expect("a report");
    let shuffled =
        build_measurement_report_from(root, &[], &[], &[], &reordered).expect("a report");

    let human = render_measurement_report(&first).expect("the human view renders");
    assert_eq!(human, render_measurement_report(&again).unwrap());
    assert_eq!(
        human,
        render_measurement_report(&shuffled).unwrap(),
        "store input order changed the human view"
    );
    let json = render_measurement_report_json(&first).expect("the JSON view renders");
    assert_eq!(json, render_measurement_report_json(&again).unwrap());
    assert_eq!(
        json,
        render_measurement_report_json(&shuffled).unwrap(),
        "store input order changed the JSON view"
    );

    // Both views expose the same claim-centered sections.
    let parsed: Value = serde_json::from_str(&json).expect("the JSON view is JSON");
    let mut measured = 0_usize;
    for entry in parsed["operational"].as_array().expect("the entries") {
        for section in ["claims", "evidence", "counterevidence", "gaps", "actions"] {
            for item in entry[section].as_array().expect("a section is an array") {
                let text = item.as_str().expect("a section item is a string");
                assert!(
                    human.contains(text),
                    "the human view omits the {section} item {text}"
                );
                measured += 1;
            }
        }
        let owner = entry["owner"].as_str().expect("an owner");
        assert!(human.contains(owner), "the human view omits the owner");
    }
    assert!(
        measured >= records.len(),
        "anti-vacuity floor: {measured} section items compared"
    );
}

/// Quoin SHALL NOT invoke, drill, or alter an operational control while
/// recording or reporting evidence.
///
/// Trace: FR-060-CON-1
/// Provenance: quoin#479
#[test]
fn tc_479_410_the_operational_modules_hold_no_control_path() {
    /// Nothing here may reach a process, a network, or a shell: every one of
    /// these is a way to invoke the control the record is only describing.
    const FORBIDDEN: [&str; 10] = [
        "std::process",
        "Command::new",
        "std::net",
        "TcpStream",
        "reqwest",
        "ureq",
        "hyper",
        "curl",
        "libc::",
        "unsafe",
    ];

    /// Below this the census is reading an empty tree and proving nothing.
    const SOURCE_FLOOR: usize = 12;

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("operational");
    let mut measured = 0_usize;
    for path in rust_files(&root) {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: unreadable: {error}", path.display()));
        for needle in FORBIDDEN {
            assert!(
                !text.contains(needle),
                "{}: the operational layer reaches for `{needle}`",
                path.display()
            );
        }
        measured += 1;
    }
    assert!(
        measured >= SOURCE_FLOOR,
        "the census read {measured} operational modules, below the floor of {SOURCE_FLOOR}"
    );
}

/// Existing measurement collections and pre-operational evidence SHALL remain
/// readable without migration.
///
/// Trace: FR-060-CON-2
/// Provenance: quoin#479
#[test]
fn tc_479_411_existing_collections_and_interventions_read_beside_operational_records() {
    let workspace = temporary_repository();
    let root = workspace.path();
    let evidence = repo_root().join("spec").join("evidence");
    for (directory, name) in [
        ("measurements", RETAINED_COLLECTION),
        ("interventions", RETAINED_INTERVENTION),
    ] {
        let target = root.join("spec").join("evidence").join(directory);
        std::fs::create_dir_all(&target).expect("a writable destination");
        std::fs::copy(evidence.join(directory).join(name), target.join(name))
            .expect("a copied retained file");
    }

    let (capability, exercise) = pair_records();
    write_operational_pair(root, &SystemClock, &capability, &exercise)
        .expect("the operational pair lands beside the existing evidence");

    let report = build_measurement_report(&DiskMeasurement::new(root), root)
        .expect("the pre-operational evidence reads without migration");
    assert_eq!(
        report.corpus_gaps,
        Some(0.0),
        "the retained collection's stated bounds did not survive"
    );
    assert_eq!(report.interventions.len(), 1);
    assert_eq!(report.operational.len(), 2);
    assert!(!report.plans.is_empty());

    let human = render_measurement_report(&report).expect("the report renders");
    for heading in [
        "# QA measurement report",
        "## Current evidence",
        "## Intervention experiments",
        "## Operational evidence",
    ] {
        assert!(human.contains(heading), "the render dropped {heading}");
    }
}
