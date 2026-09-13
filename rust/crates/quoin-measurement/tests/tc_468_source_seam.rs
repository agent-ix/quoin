// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! One analysis, two sources.
//!
//! The measurement analysis — the plan walk, the profile walk, the collection
//! read-back — is written once, over [`MeasurementSource`]. This test states
//! the same repository twice, once as files and once as a map, runs **one**
//! generic function over both, and asserts the two agree. If the port ever
//! grows a second copy of the analysis for the disk case, this test is what
//! notices.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_measurement::{
    DiskMeasurement, MeasurementSource, MemoryMeasurement, PlanLoadOptions,
    load_active_assurance_profiles, load_measurement_plans, read_measurement_collections,
};

const PLAN: &str = "---\ntype: MeasurementPlan\nid: MP-1\ntitle: Coverage\nstatus: \
                    active\nstage: observe\nmetric: coverage\ndefinition_version: v1\n---\n\n# \
                    Coverage\n";

const PROFILE: &str =
    "---\ntype: AssuranceProfile\nid: AP-1\ntitle: Tier 1\nstatus: active\n---\n\n# Tier 1\n";

const COLLECTION: &str = r#"{
  "schemaVersion": 1,
  "collectionId": "c-1",
  "subject": "quoin",
  "scope": {},
  "toolIdentity": "quoin",
  "toolVersion": "0.1.0",
  "configDigest": "d",
  "timestamp": "2026-01-01T00:00:00Z",
  "sourceRevision": "r",
  "environment": {},
  "rawEvidence": [],
  "observations": [
    {
      "metric": "coverage",
      "planId": "MP-1",
      "definitionVersion": "v1",
      "state": "measured",
      "value": 0.5,
      "unit": "ratio",
      "shape": "ratio"
    }
  ]
}
"#;

/// The one analysis. Written over the trait, so it is the same code in both
/// halves of every assertion below.
fn summary<S: MeasurementSource + ?Sized>(source: &S) -> (Vec<String>, Vec<String>, Vec<String>) {
    let plans = load_measurement_plans(source, PlanLoadOptions::default()).expect("plans load");
    let profiles = load_active_assurance_profiles(source).expect("profiles load");
    let collections = read_measurement_collections(source).expect("collections read");
    (
        plans
            .iter()
            .map(|plan| format!("{}@{}", plan.id, plan.metric))
            .collect(),
        profiles.iter().map(|p| p.id.to_string()).collect(),
        collections
            .iter()
            .map(|c| c.collection_id.to_string())
            .collect(),
    )
}

/// Trace: FR-100-AC-2, FR-100-AC-4
/// Provenance: quoin#468
#[test]
fn tc_468_one_analysis_runs_unchanged_on_disk_and_in_memory() {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let repo = temporary.path();
    let assurance = repo.join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("the assurance root");
    std::fs::write(assurance.join("plan.md"), PLAN).expect("the plan document");
    std::fs::write(assurance.join("profile.md"), PROFILE).expect("the profile document");
    let measurements = repo.join("spec").join("evidence").join("measurements");
    std::fs::create_dir_all(&measurements).expect("the measurements directory");
    std::fs::write(measurements.join("c-1.json"), COLLECTION).expect("the collection");

    let memory = MemoryMeasurement::new()
        .with_document("spec/assurance/plan.md", PLAN)
        .with_document("spec/assurance/profile.md", PROFILE)
        .with_collection("c-1.json", COLLECTION);

    let on_disk = summary(&DiskMeasurement::new(repo));
    let in_memory = summary(&memory);
    assert_eq!(on_disk, in_memory);
    assert_eq!(
        on_disk,
        (
            vec!["MP-1@coverage".to_owned()],
            vec!["AP-1".to_owned()],
            vec!["c-1".to_owned()],
        )
    );
}

/// The in-memory host accounts for the bytes it holds, through `quoin-store`.
///
/// Before quoin#484 this asserted the opposite — a refusal, because
/// `quoin-store` exposed sha256 only over a path and this crate will not mint a
/// second implementation. The refusal is now reserved for a path the source was
/// never given, and the digest is asserted against the published NIST vector
/// rather than against a second call to the same function.
///
/// Trace: FR-100-AC-4, FR-100-CON-4
/// Provenance: quoin#468, quoin#484
#[test]
fn tc_484_a_memory_source_digests_the_bytes_it_holds() {
    use quoin_measurement::{MeasurementErrorCode, RawEvidencePath};

    let path = RawEvidencePath::parse("raw/run.json").expect("a safe path");

    let stated = MemoryMeasurement::new().with_retained_evidence("raw/run.json", "abc");
    let accounted = stated.raw_evidence_file(&path).expect("the bytes are held");
    assert_eq!(accounted.size_bytes, 3);
    assert_eq!(
        accounted.digest.to_stored(),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    // A path the source was never given is still a refusal, and still the same
    // code: the host knows nothing about it, which is not the same as it being
    // empty.
    let error = MemoryMeasurement::new()
        .raw_evidence_file(&path)
        .unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::RawEvidenceUnavailable);
}
