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
    // Canonicalised: `quire` rejects `..` in `--module`.
    let root = std::fs::canonicalize(&root).unwrap_or_else(|error| {
        panic!(
            "the spec-objects-security module is not at {}: {error}. Check out \
             agent-ix/spec-objects-security next to this repository, or set \
             SPEC_OBJECTS_SECURITY_DIR to its spec_objects_security directory",
            root.display()
        )
    });
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

#[trace("FR-141-AC-1", "FR-141-AC-4")]
#[test]
fn a_cyclonedx_and_an_spdx_inventory_promote_to_valid_records_with_no_fact_reference() {
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
            assert_eq!(field(record, "license_fact_ref"), None);
            assert_eq!(field(record, "disposition_clause"), None);
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
            .all(|n| Path::new(n).extension().is_some_and(|e| e == "md"))
    );
    assert!(names.contains("pkg%3Acargo%2Fserde%401.0.228.md"));
}

#[trace("FR-141-AC-15")]
#[test]
fn an_occupied_destination_stops_the_command_before_any_record_is_written() {
    let work = tempfile::tempdir().unwrap();
    let sbom = write(
        work.path(),
        "bom.json",
        &cyclonedx(&[component("serde", "1.0.228", Some(PURL_B))]),
    );
    let out = work.path().join("soup");
    std::fs::create_dir_all(&out).unwrap();
    let squatter = out.join("pkg%3Acargo%2Fserde%401.0.228.md");
    std::fs::write(&squatter, "# notes, not a record\n").unwrap();
    let before = snapshot(&out);
    let run = with_module(&sbom, &out, &[]);
    assert_eq!(run.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("pkg%3Acargo%2Fserde%401.0.228.md"),
        "{stderr}"
    );
    assert_eq!(snapshot(&out), before);
}

#[trace("FR-141-AC-16")]
#[test]
fn a_component_with_a_control_character_in_its_name_is_skipped_and_listed() {
    let work = tempfile::tempdir().unwrap();
    let sbom = write(
        work.path(),
        "bom.json",
        &cyclonedx(&[component("evil", "1", Some("pkg:npm/ev%0A%23%23il@1"))]),
    );
    let out = work.path().join("soup");
    let run = with_module(&sbom, &out, &[]);
    assert!(run.status.success());
    let report = stdout(&run);
    assert!(
        report.lines().any(|l| l == "components skipped: 1"),
        "{report}"
    );
    assert!(report.contains("pkg:npm/ev%0A%23%23il@1"), "{report}");
    assert_eq!(files(&out).len(), 1);
}

#[trace("FR-141-AC-17")]
#[test]
fn a_qualified_purl_is_named_without_its_qualifiers() {
    let work = tempfile::tempdir().unwrap();
    let sbom = write(work.path(), "bom.json", &cyclonedx(&[]));
    let out = work.path().join("soup");
    assert!(with_module(&sbom, &out, &[]).status.success());
    let written = files(&out);
    assert_eq!(
        written[0].file_name().unwrap().to_str().unwrap(),
        "pkg%3Anpm%2Fsemver%407.6.3.md"
    );
}

#[trace("FR-141-AC-17")]
#[test]
fn a_record_file_that_cannot_be_read_in_full_is_refused_by_name() {
    let work = tempfile::tempdir().unwrap();
    let sbom = write(work.path(), "bom.json", &cyclonedx(&[]));
    let records = work.path().join("records");
    std::fs::create_dir_all(&records).unwrap();
    std::fs::write(records.join("huge.md"), vec![b'a'; 1024 * 1024 + 1]).unwrap();
    let out = work.path().join("out");
    let run = with_module(&sbom, &out, &["--records", records.to_str().unwrap()]);
    assert_eq!(run.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&run.stderr).contains("huge.md"));
    assert!(!out.exists());
}

#[trace("FR-141-AC-5", "FR-141-AC-14")]
#[test]
fn a_record_with_crlf_line_endings_is_still_found_and_ids_continue_after_it() {
    let work = tempfile::tempdir().unwrap();
    let out = work.path().join("soup");
    std::fs::create_dir_all(&out).unwrap();
    let crlf = format!(
        "---\r\nid: SOUP-007\r\ntitle: \"semver\"\r\ntype: soup_record\r\nobject: soup_record\r\npurl: \"{PURL_A}\"\r\n---\r\n# [SOUP-007] semver\r\n"
    );
    let reviewed = out.join("reviewed.md");
    std::fs::write(&reviewed, &crlf).unwrap();
    let sbom = write(work.path(), "bom.json", &cyclonedx(&three()));
    let run = with_module(&sbom, &out, &[]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(stdout(&run).lines().any(|l| l == "records unchanged: 1"));
    assert_eq!(std::fs::read_to_string(&reviewed).unwrap(), crlf);
    let ids: Vec<_> = files(&out)
        .iter()
        .filter(|p| *p != &reviewed)
        .map(|p| field(p, "id").unwrap())
        .collect();
    assert_eq!(ids, ["SOUP-008", "SOUP-009"]);
}

/// A minimal `CycloneDX` document of exactly these components.
fn bom(components: &[Value]) -> Value {
    json!({ "bomFormat": "CycloneDX", "specVersion": "1.6", "components": components })
}

/// A purl-bearing `CycloneDX` component carrying the given extra members.
fn with(purl: &str, name: &str, extra: &Value) -> Value {
    let mut value = component(name, "1", Some(purl));
    for (key, member) in extra.as_object().unwrap() {
        value[key] = member.clone();
    }
    value
}

fn promoted(sbom: &Value) -> (tempfile::TempDir, PathBuf, Output) {
    let work = tempfile::tempdir().unwrap();
    let path = write(work.path(), "bom.json", sbom);
    let out = work.path().join("soup");
    let run = with_module(&path, &out, &[]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    (work, out, run)
}

fn record_of(out: &Path, purl: &str) -> PathBuf {
    out.join(format!(
        "{}.md",
        purl.replace(':', "%3A")
            .replace('/', "%2F")
            .replace('@', "%40")
    ))
}

fn has_line(report: &str, wanted: &str) -> bool {
    report.lines().any(|line| line == wanted)
}

#[trace("FR-141-AC-18")]
#[test]
fn a_stated_supplier_and_one_licence_are_written_as_stated_for_each_licence_kind() {
    let work = tempfile::tempdir().unwrap();
    let cdx = write(
        work.path(),
        "bom.json",
        &bom(&[
            with(
                "pkg:npm/a@1",
                "a",
                &json!({"supplier": {"name": "Acme, Inc."}, "licenses": [{"license": {"id": "MIT"}}]}),
            ),
            with(
                "pkg:npm/b@1",
                "b",
                &json!({"supplier": {"name": "Beta"}, "licenses": [{"license": {"name": "Custom Terms 2"}}]}),
            ),
            with(
                "pkg:npm/c@1",
                "c",
                &json!({"supplier": {"name": "Gamma"}, "licenses": [{"expression": "MIT OR Apache-2.0"}]}),
            ),
        ]),
    );
    let spdx = write(
        work.path(),
        "spdx.json",
        &json!({"spdxVersion": "SPDX-2.3", "packages": [
            {"name": "a", "versionInfo": "1", "supplier": "Organization: Acme, Inc.", "licenseDeclared": "MIT",
             "externalRefs": [{"referenceType": "purl", "referenceLocator": "pkg:npm/a@1"}]},
            {"name": "c", "versionInfo": "1", "supplier": "Person: Gamma", "licenseDeclared": "MIT OR Apache-2.0",
             "externalRefs": [{"referenceType": "purl", "referenceLocator": "pkg:npm/c@1"}]},
        ]}),
    );
    let out = work.path().join("cdx");
    assert!(with_module(&cdx, &out, &[]).status.success());
    let expected = [
        ("pkg:npm/a@1", "Acme, Inc.", "MIT"),
        ("pkg:npm/b@1", "Beta", "Custom Terms 2"),
        ("pkg:npm/c@1", "Gamma", "MIT OR Apache-2.0"),
    ];
    for (purl, supplier, license) in expected {
        let record = record_of(&out, purl);
        validates(&record);
        assert_eq!(field(&record, "supplier").as_deref(), Some(supplier));
        assert_eq!(field(&record, "license").as_deref(), Some(license));
    }
    let out = work.path().join("spdx");
    assert!(with_module(&spdx, &out, &[]).status.success());
    let record = record_of(&out, "pkg:npm/a@1");
    validates(&record);
    assert_eq!(
        field(&record, "supplier").as_deref(),
        Some("Organization: Acme, Inc.")
    );
    assert_eq!(field(&record, "license").as_deref(), Some("MIT"));
}

#[trace("FR-141-AC-19")]
#[test]
fn an_unstated_supplier_or_licence_is_absent_and_never_the_word_unknown() {
    let (_work, out, _) = promoted(&bom(&[
        with("pkg:npm/none@1", "none", &json!({})),
        with(
            "pkg:npm/empty@1",
            "empty",
            &json!({"supplier": {"name": ""}}),
        ),
        with(
            "pkg:npm/stated@1",
            "stated",
            &json!({"supplier": {"name": "unknown"}}),
        ),
    ]));
    for purl in ["pkg:npm/none@1", "pkg:npm/empty@1"] {
        let text = std::fs::read_to_string(record_of(&out, purl)).unwrap();
        assert!(!text.contains("supplier:"), "{text}");
        assert!(!text.contains("license:"), "{text}");
        assert!(!text.contains("unknown"), "{text}");
    }
    assert_eq!(
        field(&record_of(&out, "pkg:npm/stated@1"), "supplier").as_deref(),
        Some("unknown")
    );

    let work = tempfile::tempdir().unwrap();
    let spdx = write(
        work.path(),
        "spdx.json",
        &json!({"spdxVersion": "SPDX-2.3", "packages": [
            {"name": "a", "versionInfo": "1", "supplier": "NOASSERTION", "licenseDeclared": "NOASSERTION",
             "externalRefs": [{"referenceType": "purl", "referenceLocator": "pkg:npm/a@1"}]},
        ]}),
    );
    let out = work.path().join("soup");
    assert!(with_module(&spdx, &out, &[]).status.success());
    let text = std::fs::read_to_string(record_of(&out, "pkg:npm/a@1")).unwrap();
    assert!(
        !text.contains("supplier:") && !text.contains("license:"),
        "{text}"
    );
    assert!(!text.contains("unknown"), "{text}");
}

#[trace("FR-141-AC-20")]
#[test]
fn several_licences_leave_the_licence_absent_and_are_listed_with_their_texts() {
    let (_work, out, run) = promoted(&bom(&[
        with(
            "pkg:npm/two@1",
            "two",
            &json!({"supplier": {"name": "Two Corp"}, "licenses": [{"license": {"id": "MIT"}}, {"license": {"id": "ISC"}}]}),
        ),
        with(
            "pkg:npm/mixed@1",
            "mixed",
            &json!({"supplier": {"name": "Mixed Corp"}, "licenses": [{"license": {"id": "MIT"}}, {"license": {"url": "https://x"}}]}),
        ),
    ]));
    for (purl, supplier) in [
        ("pkg:npm/two@1", "Two Corp"),
        ("pkg:npm/mixed@1", "Mixed Corp"),
    ] {
        let record = record_of(&out, purl);
        assert_eq!(field(&record, "license"), None);
        assert_eq!(field(&record, "supplier").as_deref(), Some(supplier));
    }
    let report = stdout(&run);
    assert!(
        has_line(&report, "multiple or unreadable licences: 2"),
        "{report}"
    );
    assert!(
        has_line(&report, "  pkg:npm/mixed@1 MIT, unreadable"),
        "{report}"
    );
    assert!(has_line(&report, "  pkg:npm/two@1 MIT, ISC"), "{report}");
}

#[trace("FR-141-AC-21", "FR-141-AC-24")]
#[test]
fn control_characters_and_unreadable_licences_omit_the_field_and_are_listed() {
    let (_work, out, run) = promoted(&bom(&[
        with(
            "pkg:npm/sup@1",
            "sup",
            &json!({"supplier": {"name": "Acme\u{7}Corp"}, "licenses": [{"license": {"id": "MIT"}}]}),
        ),
        with(
            "pkg:npm/both@1",
            "both",
            &json!({"supplier": {"name": "Acme\nCorp"}, "licenses": [{"license": {"name": "Own\u{1b}Terms"}}]}),
        ),
        with(
            "pkg:npm/unreadable@1",
            "unreadable",
            &json!({"licenses": [{"license": {"url": "https://example.org/l"}}]}),
        ),
    ]));
    let sup = std::fs::read_to_string(record_of(&out, "pkg:npm/sup@1")).unwrap();
    assert!(!sup.contains("supplier:"), "{sup}");
    assert_eq!(
        field(&record_of(&out, "pkg:npm/sup@1"), "license").as_deref(),
        Some("MIT")
    );
    let both = std::fs::read_to_string(record_of(&out, "pkg:npm/both@1")).unwrap();
    assert!(
        !both.contains("supplier:") && !both.contains("license:"),
        "{both}"
    );
    let unreadable = std::fs::read_to_string(record_of(&out, "pkg:npm/unreadable@1")).unwrap();
    assert!(!unreadable.contains("license:"), "{unreadable}");
    assert!(!unreadable.contains("example.org"), "{unreadable}");
    let report = stdout(&run);
    assert!(has_line(&report, "omitted fields: 3"), "{report}");
    for line in [
        "  pkg:npm/sup@1 supplier",
        "  pkg:npm/both@1 license",
        "  pkg:npm/both@1 supplier",
    ] {
        assert!(has_line(&report, line), "{line}\n{report}");
    }
    assert!(
        has_line(&report, "multiple or unreadable licences: 1"),
        "{report}"
    );
    assert!(
        has_line(&report, "  pkg:npm/unreadable@1 unreadable"),
        "{report}"
    );
}

/// Replace `key` in the frontmatter of `record` with `value`, or drop it.
fn set_field(record: &Path, key: &str, value: Option<&str>) {
    let text = std::fs::read_to_string(record).unwrap();
    let mut lines: Vec<String> = text
        .lines()
        .filter(|line| !line.starts_with(&format!("{key}: ")))
        .map(str::to_owned)
        .collect();
    if let Some(value) = value {
        let at = lines.iter().position(|l| l.starts_with("purl: ")).unwrap() + 1;
        lines.insert(at, format!("{key}: {}", json!(value)));
    }
    std::fs::write(record, lines.join("\n") + "\n").unwrap();
}

fn difference_lines(report: &str) -> Vec<&str> {
    report
        .lines()
        .skip_while(|line| !line.starts_with("field differences:"))
        .skip(1)
        .take_while(|line| line.starts_with("  "))
        .collect()
}

#[trace("FR-141-AC-22", "FR-141-CON-4")]
#[test]
fn an_existing_record_is_never_rewritten_and_its_differing_fields_are_reported() {
    let sbom = bom(&[
        with(
            "pkg:npm/edited@1",
            "edited",
            &json!({"supplier": {"name": "Acme"}, "licenses": [{"license": {"id": "MIT"}}]}),
        ),
        with(
            "pkg:npm/equal@1",
            "equal",
            &json!({"supplier": {"name": "Acme"}, "licenses": [{"license": {"id": "MIT"}}]}),
        ),
        with(
            "pkg:npm/ref@1",
            "ref",
            &json!({"supplier": {"name": "Acme"}, "licenses": [{"license": {"id": "MIT"}}]}),
        ),
    ]);
    let (work, out, _) = promoted(&sbom);
    set_field(
        &record_of(&out, "pkg:npm/edited@1"),
        "supplier",
        Some("Person Edited"),
    );
    set_field(
        &record_of(&out, "pkg:npm/edited@1"),
        "license",
        Some("Apache-2.0"),
    );
    set_field(&record_of(&out, "pkg:npm/ref@1"), "license", None);
    set_field(
        &record_of(&out, "pkg:npm/ref@1"),
        "license_fact_ref",
        Some("LF-1"),
    );
    let before = snapshot(&out);
    let again = with_module(&work.path().join("bom.json"), &out, &[]);
    assert!(again.status.success());
    assert_eq!(snapshot(&out), before);
    let report = stdout(&again);
    let lines = difference_lines(&report);
    assert!(has_line(&report, "field differences: 2"), "{report}");
    assert_eq!(lines.len(), 2, "{report}");
    assert!(
        lines.iter().all(|l| l.contains("pkg:npm/edited@1")),
        "{report}"
    );
    assert!(lines.iter().any(|l| l.ends_with(" supplier")), "{report}");
    assert!(lines.iter().any(|l| l.ends_with(" license")), "{report}");
}

#[trace("FR-141-AC-23")]
#[test]
fn a_field_set_where_the_sbom_states_none_and_the_reverse_are_differences() {
    let sbom = bom(&[
        with("pkg:npm/bare@1", "bare", &json!({})),
        with(
            "pkg:npm/two@1",
            "two",
            &json!({"licenses": [{"license": {"id": "MIT"}}, {"license": {"id": "ISC"}}]}),
        ),
        with(
            "pkg:npm/unreadable@1",
            "unreadable",
            &json!({"licenses": [{"license": {"url": "https://x"}}]}),
        ),
        with(
            "pkg:npm/ctrl@1",
            "ctrl",
            &json!({"licenses": [{"license": {"name": "A\u{7}B"}}]}),
        ),
        with(
            "pkg:npm/stated@1",
            "stated",
            &json!({"supplier": {"name": "Acme"}, "licenses": [{"license": {"id": "MIT"}}]}),
        ),
    ]);
    let (work, out, _) = promoted(&sbom);
    set_field(
        &record_of(&out, "pkg:npm/bare@1"),
        "supplier",
        Some("Set By Person"),
    );
    for purl in [
        "pkg:npm/bare@1",
        "pkg:npm/two@1",
        "pkg:npm/unreadable@1",
        "pkg:npm/ctrl@1",
    ] {
        set_field(&record_of(&out, purl), "license", Some("MIT"));
    }
    set_field(&record_of(&out, "pkg:npm/stated@1"), "supplier", None);
    set_field(&record_of(&out, "pkg:npm/stated@1"), "license", None);
    let before = snapshot(&out);
    let again = with_module(&work.path().join("bom.json"), &out, &[]);
    assert!(again.status.success());
    assert_eq!(snapshot(&out), before);
    let report = stdout(&again);
    assert!(has_line(&report, "field differences: 7"), "{report}");
    let lines = difference_lines(&report);
    let named =
        |purl: &str, field: &str| lines.iter().any(|l| l.contains(&format!("{purl} {field}")));
    assert!(named("pkg:npm/bare@1", "supplier"), "{report}");
    assert!(named("pkg:npm/bare@1", "license"), "{report}");
    assert!(named("pkg:npm/two@1", "license"), "{report}");
    assert!(named("pkg:npm/unreadable@1", "license"), "{report}");
    assert!(named("pkg:npm/ctrl@1", "license"), "{report}");
    assert!(named("pkg:npm/stated@1", "supplier"), "{report}");
    assert!(named("pkg:npm/stated@1", "license"), "{report}");
}

#[trace("FR-141-AC-25")]
#[test]
fn a_repeated_purl_carries_the_first_occurrences_values_and_lists_the_conflict() {
    let (_work, out, run) = promoted(&bom(&[
        with(
            "pkg:npm/dup@1",
            "dup",
            &json!({"supplier": {"name": "First"}, "licenses": [{"license": {"id": "MIT"}}]}),
        ),
        with(
            "pkg:npm/dup@1",
            "dup",
            &json!({"supplier": {"name": "Second"}, "licenses": [{"license": {"id": "ISC"}}]}),
        ),
    ]));
    assert_eq!(files(&out).len(), 1);
    let record = record_of(&out, "pkg:npm/dup@1");
    assert_eq!(field(&record, "supplier").as_deref(), Some("First"));
    assert_eq!(field(&record, "license").as_deref(), Some("MIT"));
    let report = stdout(&run);
    assert!(has_line(&report, "duplicate conflicts: 2"), "{report}");
    assert!(has_line(&report, "  pkg:npm/dup@1 license"), "{report}");
    assert!(has_line(&report, "  pkg:npm/dup@1 supplier"), "{report}");
}
