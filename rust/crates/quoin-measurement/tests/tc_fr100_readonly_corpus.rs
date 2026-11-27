// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! A completed measurement reads a populated, read-only governed corpus
//! without changing any of its bytes (FR-100-AC-5, FR-100-CON-3).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a malformed test fixture is a test failure"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use quoin_measurement::{
    DiskMeasurement, PlanLoadOptions, load_active_assurance_profiles, load_measurement_plans,
    read_measurement_collections,
};

const PLAN: &str = "---\ntype: MeasurementPlan\nid: MP-1\ntitle: Coverage\nstatus: active\nstage: observe\nmetric: coverage\ndefinition_version: v1\n---\n\n# Coverage\n";
const PROFILE: &str =
    "---\ntype: AssuranceProfile\nid: AP-1\ntitle: Tier 1\nstatus: active\n---\n\n# Tier 1\n";
const COLLECTION: &str = r#"{
  "schemaVersion": 1,
  "collectionId": "c-1",
  "subject": "quoin",
  "scope": {},
  "toolIdentity": "quoin",
  "toolVersion": "0.1.0",
  "timestamp": "2026-01-01T00:00:00Z",
  "environment": {},
  "rawEvidence": [],
  "observations": [{
    "metric": "coverage",
    "planId": "MP-1",
    "definitionVersion": "v1",
    "state": "measured",
    "value": 0.5,
    "unit": "ratio",
    "shape": "ratio"
  }]
}
"#;

fn tree_bytes(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(at).expect("the governed corpus directory is readable") {
            let path = entry.expect("the directory entry is readable").path();
            if path.is_dir() {
                visit(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .expect("path stays in corpus")
                        .to_owned(),
                    fs::read(&path).expect("the corpus file is readable"),
                );
            }
        }
    }

    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}

fn set_tree_readonly(root: &Path, readonly: bool) {
    fn visit(at: &Path, paths: &mut Vec<PathBuf>) {
        paths.push(at.to_owned());
        for entry in fs::read_dir(at).expect("the corpus directory is readable") {
            let path = entry.expect("the directory entry is readable").path();
            if path.is_dir() {
                visit(&path, paths);
            } else {
                paths.push(path);
            }
        }
    }

    let mut paths = Vec::new();
    visit(root, &mut paths);
    for path in paths {
        let mut permissions = fs::metadata(&path)
            .expect("the corpus path exists")
            .permissions();
        permissions.set_readonly(readonly);
        fs::set_permissions(path, permissions).expect("corpus permissions can be changed");
    }
}

struct RestoreWritable(PathBuf);

impl Drop for RestoreWritable {
    fn drop(&mut self) {
        set_tree_readonly(&self.0, false);
    }
}

#[trace("FR-100-AC-5", "FR-100-CON-3")]
#[test]
fn a_measurement_completes_over_a_readonly_corpus_without_changing_bytes() {
    let temporary = tempfile::tempdir().expect("a temporary governed corpus");
    let root = temporary.path();
    let assurance = root.join("spec/assurance");
    fs::create_dir_all(&assurance).expect("assurance documents directory");
    fs::write(assurance.join("plan.md"), PLAN).expect("measurement plan");
    fs::write(assurance.join("profile.md"), PROFILE).expect("assurance profile");
    let measurements = root.join("spec/evidence/measurements");
    fs::create_dir_all(&measurements).expect("measurement collection directory");
    fs::write(measurements.join("c-1.json"), COLLECTION).expect("measurement collection");
    fs::write(root.join("accepted-corpus.txt"), "governed bytes\n")
        .expect("a named accepted-corpus byte population");

    let before = tree_bytes(root);
    assert!(!before.is_empty(), "the governed corpus is populated");
    set_tree_readonly(root, true);
    let _restore = RestoreWritable(root.to_owned());
    assert!(
        tree_bytes(root)
            .keys()
            .all(|relative| fs::metadata(root.join(relative))
                .expect("corpus file exists")
                .permissions()
                .readonly()),
        "every governed-corpus file is read-only"
    );

    let source = DiskMeasurement::new(root);
    let plans = load_measurement_plans(&source, PlanLoadOptions::default())
        .expect("measurement completes over the read-only corpus");
    let profiles =
        load_active_assurance_profiles(&source).expect("active assurance profiles are read");
    let collections =
        read_measurement_collections(&source).expect("measurement collections are read");
    assert_eq!(plans.len(), 1);
    assert_eq!(profiles.len(), 1);
    assert_eq!(collections.len(), 1);

    assert_eq!(
        tree_bytes(root),
        before,
        "a completed measurement writes no corpus byte"
    );
}
