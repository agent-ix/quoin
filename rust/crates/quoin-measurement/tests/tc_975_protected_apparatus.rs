// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-975: a plan's protected apparatus is resolved and digested when a
//! collection is written, and a comparison, a ratchet verdict and the
//! verdict checker each refuse to measure across a change to it.
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

use std::path::{Path, PathBuf};

use engineering_assurance::measurement::NegativeControlKind;
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
    MeasurementError, OrderSource, Ranked, Reason, Verdict, build_measurement_report,
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
         statistical_design:\n  estimator: proportion\n  decision_rule:\n    comparator: ge\n    {reference}\n\
         {extra}\
         ---\n\n# Protected answer key\n"
    )
}

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

fn digest(root: &Path, relative: &str) -> String {
    quoin_store::digest_file_sha256(&root.join(relative))
        .unwrap()
        .to_stored()
}

/// The digest of every named file as it reads now, plus the `config` label
/// every fixture collection carries.
fn artifacts(root: &Path, names: &[&str]) -> Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "config".to_owned(),
        json!(format!("sha256:{}", "3".repeat(64))),
    );
    for name in names {
        map.insert((*name).to_owned(), json!(digest(root, name)));
    }
    Value::Object(map)
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
        "configDigest": format!("sha256:{}", "a".repeat(64)),
        "timestamp": format!("2026-09-22T00:{minute:02}:00.000Z"),
        "sourceRevision": "a".repeat(40),
        "environment": { "runner": "test" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "lockDigest": format!("sha256:{}", "1".repeat(64)),
            "executableDigest": format!("sha256:{}", "2".repeat(64)),
            "buildProfile": "release",
            "toolchains": { "rust": "1.98.1" },
            "sources": {
                "fixture": {
                    "revision": "a".repeat(40),
                    "sourceState": "clean",
                    "remote": "https://example.invalid/fixture",
                },
            },
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

/// Publish a run whose artifacts map declares every file as it reads now.
fn publish_run(root: &Path, id: &str, minute: u8, matched: u32) {
    publish(
        root,
        &candidate(id, minute, matched, &artifacts(root, &ALL_FILES)),
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
        .map(|(intake, collection)| Ranked {
            collection,
            intake: Some(intake),
        })
        .collect();
    verify(plan, &ranked, OrderSource::CallerSupplied, None).reasons
}

fn verdict_of(plan: &MeasurementPlan, collections: &[MeasurementCollection]) -> Verdict {
    let ranked: Vec<Ranked<'_>> = (0_u64..)
        .zip(collections)
        .map(|(intake, collection)| Ranked {
            collection,
            intake: Some(intake),
        })
        .collect();
    verify(plan, &ranked, OrderSource::CallerSupplied, None).verdict
}

// ------------------------------------------------------------ plan intake

/// Trace: FR-110-AC-1
/// Provenance: PLAT-975
#[test]
fn tc_975_001_protected_apparatus_and_negative_controls_load_as_eas_types() {
    let temporary = repository(&format!(
        "{PROTECTED}negative_controls:\n  - kind: apparatus-edit\n    description: the answer key is digested\n"
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
    // A gate plan must state both lists (engineering-assurance FR-024-AC-8).
    let gate = |extra: &str| {
        let source = MemoryMeasurement::new().with_document(
            "spec/assurance/MP-975.md",
            plan_with("gate", "threshold: 0.8", extra),
        );
        load_measurement_plans(&source, PlanLoadOptions::default())
    };
    const CONTROLS: &str =
        "negative_controls:\n  - kind: apparatus-edit\n    description: the key is digested\n";
    for (extra, member) in [
        (String::new(), "protected_apparatus"),
        (CONTROLS.to_owned(), "protected_apparatus"),
        (PROTECTED.to_owned(), "negative_controls"),
    ] {
        let error = gate(&extra).expect_err("a gate plan missing a list is refused");
        assert_eq!(error.code(), MeasurementErrorCode::PlanInvalid, "{extra}");
        assert!(error.to_string().contains(member), "{extra}: {error}");
    }
    gate(&format!("{PROTECTED}{CONTROLS}")).expect("a gate plan stating both loads");

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

// ------------------------------------------------------------ intake

/// A named arrangement of a fixture repository.
type Case = (&'static str, fn(&Path));

/// Trace: FR-110-AC-2
/// Provenance: PLAT-975
#[test]
fn tc_975_002_intake_records_the_resolved_set_it_digested_itself() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);

    let bytes = stored_bytes(root, "run-1");
    assert_eq!(
        bytes["verificationStack"]["protectedApparatus"],
        json!({ "MP-975": {
            "harness/answers.json": digest(root, "harness/answers.json"),
            "labels/a.json": digest(root, "labels/a.json"),
            "labels/nested/.hidden": digest(root, "labels/nested/.hidden"),
        }}),
        "every file under `labels/**`, the dotfile included, and nothing unprotected"
    );
    // Every protected file is a digested artifact, never a label.
    assert_eq!(
        bytes["verificationStack"].get("unverifiedArtifacts"),
        Some(&json!(["config"]))
    );
}

/// Trace: FR-110-AC-3
/// Provenance: PLAT-975
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

/// Trace: FR-110-AC-3
/// Provenance: PLAT-975
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

/// A producer-supplied artifacts map can omit a file; intake resolves the set
/// itself and refuses a map that leaves a protected file out or states it at
/// another digest.
///
/// Trace: FR-110-AC-3
/// Provenance: PLAT-975
#[test]
fn tc_975_005_an_artifacts_map_that_omits_a_protected_file_is_refused() {
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

    let mut wrong = artifacts(root, &ALL_FILES);
    wrong["harness/answers.json"] = json!(format!("sha256:{}", "9".repeat(64)));
    let error = publish(root, &candidate("run-1", 1, 9, &wrong))
        .expect_err("a disagreeing digest is refused");
    assert_eq!(
        error.code(),
        MeasurementErrorCode::CollectionInvalid,
        "{error}"
    );
    assert_eq!(stored_count(root), 0);
}

/// Trace: FR-110-AC-3
/// Provenance: PLAT-975
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

/// Trace: FR-110-AC-4
/// Provenance: PLAT-975
#[test]
fn tc_975_007_an_edited_protected_file_refuses_the_delta() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 8);
    write(root, "harness/answers.json", "{\"answers\":[1,2,4]}");
    publish_run(root, "run-2", 2, 8);

    let collections = stored(root);
    let rows = compare_measurement_collections(&collections[0], &collections[1]).unwrap();
    assert_eq!(reasons(&rows), [ComparisonReasonCode::ApparatusChanged]);
    assert_eq!(rows[0].status, ComparisonStatus::Incomparable);
    assert_eq!(rows[0].delta, None);
    assert!(rows[0].reasons[0].blocking);
    assert!(rows[0].reasons[0].message.contains("harness/answers.json"));
}

/// Trace: FR-110-AC-4
/// Provenance: PLAT-975
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
    let rows = compare_measurement_collections(&collections[0], &collections[1]).unwrap();
    assert_eq!(reasons(&rows), [ComparisonReasonCode::ApparatusChanged]);
    assert!(rows[0].reasons[0].message.contains("labels/b.json"));
    assert_eq!(rows[0].delta, None);

    // And removed: the reverse comparison names the same file.
    let rows = compare_measurement_collections(&collections[1], &collections[0]).unwrap();
    assert_eq!(reasons(&rows), [ComparisonReasonCode::ApparatusChanged]);
}

/// Trace: FR-110-AC-4
/// Provenance: PLAT-975
#[test]
fn tc_975_009_an_unprotected_artifact_moving_is_reported_and_does_not_block() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 8);
    write(root, "dist/tool", "tool v2");
    publish_run(root, "run-2", 2, 8);

    let collections = stored(root);
    let rows = compare_measurement_collections(&collections[0], &collections[1]).unwrap();
    assert_eq!(reasons(&rows), [ComparisonReasonCode::ArtifactChanged]);
    assert!(!rows[0].reasons[0].blocking);
    assert!(rows[0].reasons[0].message.contains("dist/tool"));
    assert_eq!(rows[0].status, ComparisonStatus::Comparable);
    assert_eq!(rows[0].delta, Some(0.0));
}

/// A plan that protects nothing is compared and stored exactly as before
/// PLAT-975: no record, and an artifact moving is not reported.
///
/// Trace: FR-110-AC-2, FR-110-AC-4
/// Provenance: PLAT-975
#[test]
fn tc_975_010_no_apparatus_declared_leaves_behaviour_unchanged() {
    let temporary = repository("");
    let root = temporary.path();
    publish_run(root, "run-1", 1, 8);
    write(root, "harness/answers.json", "{\"answers\":[9]}");
    write(root, "dist/tool", "tool v2");
    publish_run(root, "run-2", 2, 8);

    assert_eq!(
        stored_bytes(root, "run-1")["verificationStack"].get("protectedApparatus"),
        None,
        "no protecting plan, no record"
    );
    let collections = stored(root);
    let rows = compare_measurement_collections(&collections[0], &collections[1]).unwrap();
    assert_eq!(reasons(&rows), []);
    assert_eq!(rows[0].status, ComparisonStatus::Comparable);
}

/// The stored baseline is what a new run is compared against. Regenerating a
/// baseline alongside an edited apparatus cannot replace the stored one, and
/// a regenerated copy under a new id does not change what the stored one
/// recorded.
///
/// Trace: FR-110-AC-5
/// Provenance: PLAT-975
#[test]
fn tc_975_011_a_regenerated_baseline_is_still_refused_against_the_stored_one() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "baseline", 1, 8);
    write(root, "harness/answers.json", "{\"answers\":\"easier\"}");

    let regenerated = candidate("baseline", 1, 8, &artifacts(root, &ALL_FILES));
    let error = publish(root, &regenerated).expect_err("the stored baseline is write-once");
    assert_eq!(error.code(), MeasurementErrorCode::CollectionIdCollision);
    publish_run(root, "baseline-regenerated", 2, 8);
    publish_run(root, "run", 3, 8);

    let collections = stored(root);
    let by_id = |id: &str| {
        collections
            .iter()
            .find(|collection| collection.collection_id.as_str() == id)
            .unwrap()
    };
    let rows = compare_measurement_collections(by_id("baseline"), by_id("run")).unwrap();
    assert_eq!(reasons(&rows), [ComparisonReasonCode::ApparatusChanged]);
    assert_eq!(rows[0].delta, None);
}

/// A collection lacking the record, or listing a protected name as
/// unverified, is not comparable with one that recorded the set.
///
/// Trace: FR-110-AC-4
/// Provenance: PLAT-975
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
    let rows = compare_measurement_collections(&unrecorded, recorded).unwrap();
    assert_eq!(reasons(&rows), [ComparisonReasonCode::ApparatusChanged]);

    let mut unverified = stored_bytes(root, "run-1");
    unverified["verificationStack"]["unverifiedArtifacts"] = json!(["harness/answers.json"]);
    let unverified = stored_measurement_collection(&from_serde(&unverified).unwrap()).unwrap();
    let rows = compare_measurement_collections(&unverified, recorded).unwrap();
    assert_eq!(reasons(&rows), [ComparisonReasonCode::ApparatusChanged]);

    // The positive control: the same two runs as written compare.
    let collections = stored(root);
    let rows = compare_measurement_collections(&collections[0], &collections[1]).unwrap();
    assert_eq!(reasons(&rows), []);
}

// ------------------------------------------------------------ checker

/// Trace: FR-110-AC-7
/// Provenance: PLAT-975
#[test]
fn tc_975_013_a_changed_set_within_one_version_rejects_without_a_control() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);
    write(root, "harness/answers.json", "{\"answers\":\"easier\"}");
    publish_run(root, "run-2", 2, 9);
    let plan = plan_of(root);
    let collections = stored(root);
    assert_eq!(
        checked(&plan, &collections),
        [Reason::NoPrior, Reason::ApparatusEdit],
        "the earlier run is not a usable baseline, and the unversioned change rejects"
    );
    assert_eq!(verdict_of(&plan, &collections), Verdict::Reject);

    // The positive control: the same runs over one apparatus are accepted.
    let same = repository(PROTECTED);
    publish_run(same.path(), "run-1", 1, 9);
    publish_run(same.path(), "run-2", 2, 9);
    assert_eq!(
        verdict_of(&plan_of(same.path()), &stored(same.path())),
        Verdict::Accept
    );
}

/// Trace: FR-110-AC-7
/// Provenance: PLAT-975
#[test]
fn tc_975_014_an_apparatus_edit_under_a_declared_control_also_rejects() {
    let temporary = repository(&format!(
        "{PROTECTED}negative_controls:\n  - kind: apparatus-edit\n    description: the answer key is digested\n"
    ));
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);
    write(root, "labels/b.json", "[\"b\"]");
    let mut names = ALL_FILES.to_vec();
    names.push("labels/b.json");
    publish(root, &candidate("run-2", 2, 9, &artifacts(root, &names))).unwrap();
    let plan = plan_of(root);
    let collections = stored(root);
    assert_eq!(
        checked(&plan, &collections),
        [Reason::NoPrior, Reason::ApparatusEdit]
    );
    assert_eq!(verdict_of(&plan, &collections), Verdict::Reject);
}

/// Trace: FR-110-AC-7
/// Provenance: PLAT-975
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
    let report = build_measurement_report(&DiskMeasurement::new(root), root).unwrap();
    match report.current[0].stage_verdict.clone() {
        Some(StageVerdict::Ratchet { outcome, .. }) => outcome,
        other => panic!("a ratchet verdict, got {other:?}"),
    }
}

/// Trace: FR-110-AC-6
/// Provenance: PLAT-975
#[test]
fn tc_975_016_a_ratchet_does_not_hold_against_another_apparatus() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);
    write(root, "harness/answers.json", "{\"answers\":\"easier\"}");
    publish_run(root, "run-2", 2, 9);
    assert_eq!(
        ratchet_outcome(root),
        RatchetOutcome::Inconclusive(InconclusiveReason::ApparatusChanged)
    );

    let same = repository(PROTECTED);
    publish_run(same.path(), "run-1", 1, 9);
    publish_run(same.path(), "run-2", 2, 9);
    assert!(matches!(
        ratchet_outcome(same.path()),
        RatchetOutcome::Held { .. }
    ));
}

/// Trace: FR-110-AC-8
/// Provenance: PLAT-975
#[test]
fn tc_975_017_a_malformed_stored_record_is_refused_on_read() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);
    for malformed in [
        json!([]),
        json!({ "MP-975": {} }),
        json!({ "MP-975": { "harness/answers.json": "not a digest" } }),
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

/// Trace: FR-110-AC-2
/// Provenance: PLAT-975
#[test]
fn tc_975_018_a_candidate_stating_a_computed_member_is_refused() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    for (member, stated) in [
        (
            "protectedApparatus",
            json!({ "MP-975": { "harness/answers.json": format!("sha256:{}", "0".repeat(64)) } }),
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
/// Trace: FR-110-AC-6, FR-110-AC-7
/// Provenance: PLAT-975
#[test]
fn tc_975_019_removing_the_list_from_the_plan_keeps_the_series_protected() {
    let temporary = repository(PROTECTED);
    let root = temporary.path();
    publish_run(root, "run-1", 1, 9);
    write(root, "harness/answers.json", "{\"answers\":\"easier\"}");
    publish_run(root, "run-2", 2, 9);
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
/// Trace: FR-110-AC-7, FR-108-AC-3
/// Provenance: PLAT-975
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

/// Trace: FR-110-AC-6
/// Provenance: PLAT-975
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
