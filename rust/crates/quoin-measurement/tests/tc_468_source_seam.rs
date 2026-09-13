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

/// Trace: FR-100-AC-4
/// Provenance: quoin#468
#[test]
fn tc_468_a_memory_source_refuses_to_digest_rather_than_minting_a_second_sha256() {
    use quoin_measurement::{MeasurementErrorCode, RawEvidencePath};

    let memory = MemoryMeasurement::new();
    let path = RawEvidencePath::parse("raw/run.json").expect("a safe path");
    let error = memory.raw_evidence_file(&path).unwrap_err();
    assert_eq!(error.code(), MeasurementErrorCode::RawEvidenceUnavailable);
}
