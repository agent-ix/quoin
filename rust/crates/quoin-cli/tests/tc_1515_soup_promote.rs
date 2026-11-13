// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `quoin soup promote` command surface (FR-141).
//!
//! The owning module is `spec-objects-security`, read in place from
//! `SPEC_OBJECTS_SECURITY_DIR` (its `spec_objects_security` package directory)
//! or the checkout next to this repository; nothing of it is copied here. The
//! components come from the real `CycloneDX` fixture, extended in the test with
//! further components of the same shape. Validation needs `quire` on `PATH`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const PURL_A: &str =
    "pkg:npm/semver@7.6.3?download_url=http%3A%2F%2Fnpm.ix%2Fsemver%2F-%2Fsemver-7.6.3.tgz";
const PURL_B: &str = "pkg:cargo/serde@1.0.228";
const PURL_C: &str = "pkg:npm/%40scope/left-pad@1.3.0";

fn module() -> PathBuf {
    let root = std::env::var_os("SPEC_OBJECTS_SECURITY_DIR").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../../spec-objects-security/spec_objects_security")
        },
        PathBuf::from,
    );
    assert!(
        root.join("skeletons/soup_record.md").is_file(),
        "the spec-objects-security soup_record skeleton is not at {}. These tests use the \
         module in place and keep no copy: check out agent-ix/spec-objects-security at or \
         after PLAT-1514 (#45) next to this repository, or set SPEC_OBJECTS_SECURITY_DIR to \
         its spec_objects_security directory",
        root.display()
    );
    root
}

fn fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/evidence/cyclonedx-real.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The real `CycloneDX` document with `extra` appended to
/// the real component list.
fn cyclonedx(extra: &[Value]) -> Value {
    let mut document = fixture();
    let components = document["components"].as_array_mut().unwrap();
    components.extend(extra.iter().cloned());
    document
}

fn component(name: &str, version: &str, purl: Option<&str>) -> Value {
    let mut value = json!({ "type": "library", "name": name, "version": version });
    if let Some(purl) = purl {
        value["purl"] = json!(purl);
    }
    value
}

fn write(dir: &Path, name: &str, document: &Value) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_string(document).unwrap()).unwrap();
    path
}

fn promote(sbom: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["soup", "promote"])
        .arg(sbom)
        .args(extra)
        .output()
        .unwrap()
}

fn with_module(sbom: &Path, out: &Path, extra: &[&str]) -> Output {
    let module = module();
    let mut args = vec![
        "--out",
        out.to_str().unwrap(),
        "--module",
        module.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    promote(sbom, &args)
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn files(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    found.sort();
    found
}

fn snapshot(dir: &Path) -> Vec<(PathBuf, String)> {
    files(dir)
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).unwrap();
            (path, text)
        })
        .collect()
}

fn three() -> Vec<Value> {
    vec![
        component("serde", "1.0.228", Some(PURL_B)),
        component("left-pad", "1.3.0", Some(PURL_C)),
    ]
}

fn validates(record: &Path) {
    let output = Command::new("quire")
        .args(["validate", "--module"])
        .arg(module())
        .arg(record)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}: {}",
        record.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn field(record: &Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(record).unwrap();
    let front = text
        .strip_prefix("---\n")
        .unwrap()
        .split_once("\n---")
        .unwrap()
        .0;
    front
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}: ")))
        .map(|value| serde_json::from_str::<String>(value).unwrap_or_else(|_| value.to_owned()))
}

#[trace("FR-141-AC-1")]
#[test]
fn a_cyclonedx_and_an_spdx_inventory_promote_to_valid_records_without_a_supplier() {
    let work = tempfile::tempdir().unwrap();
    let cdx = write(work.path(), "bom.json", &cyclonedx(&three()));
    let spdx = write(
        work.path(),
        "spdx.json",
        &json!({"spdxVersion": "SPDX-2.3", "packages": [
            {"name": "semver", "versionInfo": "7.6.3", "externalRefs": [{"referenceType": "purl", "referenceLocator": PURL_A}]},
            {"name": "serde", "versionInfo": "1.0.228", "externalRefs": [{"referenceType": "purl", "referenceLocator": PURL_B}]},
            {"name": "left-pad", "versionInfo": "1.3.0", "externalRefs": [{"referenceType": "purl", "referenceLocator": PURL_C}]},
        ]}),
    );
    for (sbom, out) in [(&cdx, "cdx"), (&spdx, "spdx")] {
        let out = work.path().join(out);
        let run = with_module(sbom, &out, &[]);
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
        let written = files(&out);
        assert_eq!(written.len(), 3);
        for record in &written {
            validates(record);
            assert_eq!(field(record, "supplier"), None);
            assert_eq!(field(record, "type").as_deref(), Some("soup_record"));
        }
        let names: Vec<_> = written.iter().map(|p| field(p, "name").unwrap()).collect();
        assert_eq!(names, ["serde", "left-pad", "semver"]);
        let versions: Vec<_> = written
            .iter()
            .map(|p| field(p, "version").unwrap())
            .collect();
        assert_eq!(versions, ["1.0.228", "1.3.0", "7.6.3"]);
    }
}

#[trace("FR-141-AC-2", "FR-141-AC-3")]
#[test]
fn the_report_accounts_for_every_component_and_names_a_skipped_one() {
    let work = tempfile::tempdir().unwrap();
    let sbom = write(
        work.path(),
        "bom.json",
        &cyclonedx(&[
            component("serde", "1.0.228", Some(PURL_B)),
            component("serde", "1.0.228", Some(PURL_B)),
            component("vendored-blob", "2.0", None),
            json!({ "type": "library", "version": "9" }),
        ]),
    );
    let out = work.path().join("soup");
    let run = with_module(&sbom, &out, &[]);
    assert!(run.status.success());
    let report = stdout(&run);
    for line in [
        "components read: 4",
        "records created: 2",
        "records unchanged: 0",
        "merged duplicates: 1",
        "components skipped: 1",
        "  vendored-blob@2.0",
    ] {
        assert!(report.lines().any(|l| l == line), "{line}\n{report}");
    }
    assert_eq!(files(&out).len(), 2);
}

#[trace("FR-141-AC-4", "FR-141-AC-10")]
#[test]
fn a_new_record_carries_no_reviewer_field_and_an_unstated_discharge_is_unmatched() {
    let work = tempfile::tempdir().unwrap();
    let sbom = write(work.path(), "bom.json", &cyclonedx(&[]));
    let out = work.path().join("soup");
    let repo = work.path().join("repo");
    std::fs::create_dir_all(repo.join("spec")).unwrap();
    let run = with_module(
        &sbom,
        &out,
        &["--repo", repo.to_str().unwrap(), "--discharges", "OB-NOPE"],
    );
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(stdout(&run).lines().any(|l| l == "unmatched: OB-NOPE"));
    let record = files(&out).remove(0);
    for key in [
        "serves",
        "known_anomalies",
        "support_status",
        "end_of_support",
        "supplier",
        "disposition_clause",
    ] {
        assert_eq!(field(&record, key), None, "{key}");
    }
    assert_eq!(field(&record, "version").as_deref(), Some("7.6.3"));
}

#[trace("FR-141-AC-4")]
#[test]
fn a_stated_obligation_is_written_verbatim_as_the_disposition_clause() {
    let work = tempfile::tempdir().unwrap();
    let repo = work.path().join("repo");
    let spec = repo.join("spec/functional");
    std::fs::create_dir_all(&spec).unwrap();
    let requirement = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../spec/functional/FR-141-promote-sbom-components-to-soup-records.md");
    std::fs::copy(requirement, spec.join("FR-141.md")).unwrap();
    let sbom = write(work.path(), "bom.json", &cyclonedx(&[]));
    let out = work.path().join("soup");
    let run = with_module(
        &sbom,
        &out,
        &[
            "--repo",
            repo.to_str().unwrap(),
            "--discharges",
            "FR-141-AC-1",
        ],
    );
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(!stdout(&run).contains("unmatched"), "{}", stdout(&run));
    let record = files(&out).remove(0);
    assert_eq!(
        field(&record, "disposition_clause").as_deref(),
        Some("FR-141-AC-1")
    );
    validates(&record);
}

#[trace("FR-141-AC-5", "FR-141-AC-7")]
#[test]
fn promoting_again_changes_no_file_and_never_touches_a_reviewed_record() {
    let work = tempfile::tempdir().unwrap();
    let sbom = write(work.path(), "bom.json", &cyclonedx(&three()));
    let out = work.path().join("soup");
    assert!(with_module(&sbom, &out, &[]).status.success());
    let reviewed = files(&out).remove(0);
    let mut text = std::fs::read_to_string(&reviewed).unwrap();
    text = text.replacen("\n---\n", "\nsupport_status: community_supported\n---\n", 1);
    std::fs::write(&reviewed, &text).unwrap();
    let before = snapshot(&out);
    let again = with_module(&sbom, &out, &[]);
    assert!(again.status.success());
    assert_eq!(snapshot(&out), before);
    let report = stdout(&again);
    assert!(
        report.lines().any(|l| l == "records created: 0"),
        "{report}"
    );
    assert!(
        report.lines().any(|l| l == "records unchanged: 3"),
        "{report}"
    );
}

#[trace("FR-141-AC-6")]
#[test]
fn a_bumped_version_gets_a_new_record_and_reports_the_old_one_as_superseded() {
    let work = tempfile::tempdir().unwrap();
    let out = work.path().join("soup");
    let old = write(
        work.path(),
        "old.json",
        &cyclonedx(&[component("serde", "1.0.228", Some(PURL_B))]),
    );
    assert!(with_module(&old, &out, &[]).status.success());
    let before = snapshot(&out);
    let bumped = write(
        work.path(),
        "new.json",
        &cyclonedx(&[component(
            "serde",
            "1.0.229",
            Some("pkg:cargo/serde@1.0.229"),
        )]),
    );
    let run = with_module(&bumped, &out, &[]);
    assert!(run.status.success());
    let after = snapshot(&out);
    assert_eq!(after.len(), before.len() + 1);
    for entry in &before {
        assert!(after.contains(entry), "{} was modified", entry.0.display());
    }
    let report = stdout(&run);
    assert!(
        report.lines().any(|l| l == "superseded candidates: 1"),
        "{report}"
    );
    assert!(
        report.contains(&format!("{PURL_B} superseded by pkg:cargo/serde@1.0.229")),
        "{report}"
    );
}

#[trace("FR-141-AC-8")]
#[test]
fn without_the_owning_module_the_command_names_it_and_writes_nothing() {
    let work = tempfile::tempdir().unwrap();
    let sbom = write(work.path(), "bom.json", &cyclonedx(&[]));
    let out = work.path().join("soup");
    let empty = tempfile::tempdir().unwrap();
    let run = promote(
        &sbom,
        &[
            "--out",
            out.to_str().unwrap(),
            "--module",
            empty.path().to_str().unwrap(),
        ],
    );
    assert_eq!(run.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&run.stderr).contains("spec-objects-security"));
    assert!(!out.exists());
}

#[trace("FR-141-AC-9")]
#[test]
fn a_document_that_is_not_an_sbom_is_refused_and_writes_nothing() {
    let work = tempfile::tempdir().unwrap();
    let sbom = write(work.path(), "bom.json", &json!({ "hello": "world" }));
    let out = work.path().join("soup");
    let run = with_module(&sbom, &out, &[]);
    assert_eq!(run.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&run.stderr).contains("neither a CycloneDX"));
    assert!(!out.exists());
}

#[trace("FR-141-AC-11")]
#[test]
fn a_record_in_another_records_directory_stops_a_second_one_being_written() {
    let work = tempfile::tempdir().unwrap();
    let sbom = write(work.path(), "bom.json", &cyclonedx(&[]));
    let records = work.path().join("records");
    assert!(with_module(&sbom, &records, &[]).status.success());
    let out = work.path().join("out");
    let run = with_module(&sbom, &out, &["--records", records.to_str().unwrap()]);
    assert!(run.status.success());
    assert!(files(&out).is_empty());
    assert!(stdout(&run).lines().any(|l| l == "records unchanged: 1"));
}

#[trace("FR-141-AC-12")]
#[test]
fn two_records_with_one_purl_stop_the_command_naming_both_files() {
    let work = tempfile::tempdir().unwrap();
    let sbom = write(work.path(), "bom.json", &cyclonedx(&[]));
    let records = work.path().join("records");
    assert!(with_module(&sbom, &records, &[]).status.success());
    let first = files(&records).remove(0);
    let second = records.join("copy.md");
    std::fs::copy(&first, &second).unwrap();
    let out = work.path().join("out");
    let run = with_module(&sbom, &out, &["--records", records.to_str().unwrap()]);
    assert_eq!(run.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(stderr.contains("copy.md"), "{stderr}");
    assert!(
        stderr.contains(first.file_name().unwrap().to_str().unwrap()),
        "{stderr}"
    );
    assert!(!out.exists());
}

#[trace("FR-141-AC-13")]
#[test]
fn with_no_out_records_land_in_spec_soup_under_separator_free_names() {
    let work = tempfile::tempdir().unwrap();
    let repo = work.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let sbom = write(work.path(), "bom.json", &cyclonedx(&three()));
    let module = module();
    let run = promote(
        &sbom,
        &[
            "--repo",
            repo.to_str().unwrap(),
            "--module",
            module.to_str().unwrap(),
        ],
    );
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let written = files(&repo.join("spec/soup"));
    assert_eq!(written.len(), 3);
    let names: std::collections::BTreeSet<String> = written
        .iter()
        .map(|p| p.file_name().unwrap().to_str().unwrap().to_owned())
        .collect();
    assert_eq!(names.len(), 3);
    assert!(
        names
            .iter()
            .all(|n| !n.contains('/') && Path::new(n).extension().is_some_and(|e| e == "md"))
    );
    assert!(names.contains("pkg%3Acargo%2Fserde%401.0.228.md"));
}
