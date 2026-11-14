// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `quoin soup report` command surface (FR-144, FR-143, NFR-030).
//!
//! Records are written by hand, scans are stored through the evidence store's
//! own writer, and discharge facts through `quoin evidence record-discharge`.
//! Nothing here needs the `spec-objects-security` module: the report reads the
//! `soup_record` files it is pointed at.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use quoin_evidence::adapters::{AdapterSelection, select_finding_adapter};
use quoin_evidence::store::write_scan;
use quoin_evidence::types::FindingRecord;
use quoin_evidence::{DiskEvidence, STORE_SCHEMA_VERSION, SuiteId};
use serde_json::{Value, json};

const AS_OF: &str = "2026-08-15T00:00:00Z";
const SCANNED: &str = "2026-07-31T00:00:00.000Z";

/// One repository: records below `spec/soup`, scans in its store, the binding
/// report and the discharge facts.
struct Repo {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
}

fn quoin(home: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(arguments)
        .env("IX_HOME", home)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn audit_document(vulns: &[(&str, &str, &str)]) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/evidence/cargo-audit-real.json");
    let mut document: Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let list: Vec<Value> = vulns
        .iter()
        .map(|(id, name, version)| {
            json!({
                "advisory": { "id": id, "title": format!("{id} title") },
                "package": { "name": name, "version": version }
            })
        })
        .collect();
    document["vulnerabilities"] =
        json!({ "found": !list.is_empty(), "count": list.len(), "list": list });
    document["warnings"] = json!({});
    document.to_string()
}

fn scan_record(tool: &str, suite: &str, raw: &str, vacuous: bool) -> FindingRecord {
    let adapter = select_finding_adapter(AdapterSelection {
        adapter: None,
        tool: Some(tool),
    })
    .unwrap();
    let parsed = adapter.run(raw, None).unwrap();
    FindingRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new(suite),
        tool: tool.to_owned(),
        evidence_kind: None,
        timestamp: SCANNED.to_owned(),
        ruleset: parsed.ruleset,
        rules_evaluated: if vacuous {
            Some(0)
        } else {
            parsed.rules_evaluated
        },
        findings: parsed.findings,
        evaluated: parsed.evaluated,
    }
}

impl Repo {
    fn new() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().unwrap(),
            home: tempfile::tempdir().unwrap(),
        };
        std::fs::create_dir_all(repo.records()).unwrap();
        repo
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn records(&self) -> PathBuf {
        self.path().join("spec/soup")
    }

    /// Write a `soup_record`; `extra` are further frontmatter lines.
    fn record(&self, file: &str, purl: &str, extra: &[&str]) -> PathBuf {
        let mut front = format!(
            "---\nid: SOUP-{file}\ntitle: \"{file}\"\ntype: soup_record\nobject: soup_record\npurl: \"{purl}\"\n"
        );
        for line in extra {
            front.push_str(line);
            front.push('\n');
        }
        front.push_str("---\n# ");
        front.push_str(file);
        front.push('\n');
        let path = self.records().join(format!("{file}.md"));
        std::fs::write(&path, front).unwrap();
        path
    }

    fn store(&self, scan: &FindingRecord) {
        write_scan(&mut DiskEvidence::new(self.path()), scan).unwrap();
    }

    fn audit(&self, vulns: &[(&str, &str, &str)]) {
        self.store(&scan_record(
            "cargo-audit",
            "audit",
            &audit_document(vulns),
            false,
        ));
    }

    fn write(&self, name: &str, body: &Value) -> PathBuf {
        let path = self.path().join(name);
        std::fs::write(&path, serde_json::to_string(body).unwrap()).unwrap();
        path
    }

    /// A binding report with the given binding clauses, and one approved
    /// disposition recorded for `C-DISP`.
    fn binding(&self, clauses: &[&str]) -> PathBuf {
        let clause = |id: &&str| {
            json!({"clauseId": id, "force": "mandatory", "outcome": "binding",
                   "reasons": [], "expectedOutputs": []})
        };
        let binding = self.write(
            "binding.json",
            &json!({
                "schemaVersion": "clause-binding-v1",
                "clauseSet": {"authority": "example.invalid", "id": "soup", "version": "1.0.0"},
                "context": {"product": "widget"},
                "clauses": clauses.iter().map(clause).collect::<Vec<_>>(),
            }),
        );
        let facts = self.write(
            "facts.json",
            &json!([{
                "kind": "disposition", "clauseId": "C-DISP", "decision": "accepted_risk",
                "owner": "risk-owner-1",
                "rationale": "no reachable path", "approvalRef": "approval://CAB-7",
                "attestation": {"attestedBy": "reviewer-1", "authority": "quality-lead",
                    "attestedAt": "2026-08-01T00:00:00Z", "expiresAt": "2026-09-01T00:00:00Z"}
            }]),
        );
        let recorded = quoin(
            self.home.path(),
            &[
                "evidence",
                "record-discharge",
                "--repo",
                self.path().to_str().unwrap(),
                "--binding",
                binding.to_str().unwrap(),
                "--facts",
                facts.to_str().unwrap(),
            ],
        );
        assert!(recorded.status.success(), "{}", text(&recorded.stderr));
        binding
    }

    fn report(&self, binding: &Path, extra: &[&str]) -> Output {
        let mut arguments = vec![
            "soup",
            "report",
            "--repo",
            self.path().to_str().unwrap(),
            "--binding",
            binding.to_str().unwrap(),
            "--as-of",
            AS_OF,
        ];
        arguments.extend_from_slice(extra);
        quoin(self.home.path(), &arguments)
    }

    /// Every file below the repository and its bytes, for before/after.
    fn snapshot(&self) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(dir: &Path, found: &mut Vec<(PathBuf, Vec<u8>)>) {
            let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
            entries.sort_by_key(std::fs::DirEntry::path);
            for entry in entries {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, found);
                } else {
                    found.push((path.clone(), std::fs::read(&path).unwrap()));
                }
            }
        }
        let mut found = Vec::new();
        walk(self.path(), &mut found);
        found
    }
}

/// Four records in the three groups, one resolved advisory, one unresolved,
/// one past end of support and one unmatched advisory.
fn standard() -> (Repo, PathBuf) {
    let repo = Repo::new();
    repo.record(
        "anyhow",
        "pkg:cargo/anyhow@1.0.102",
        &["version: \"1.0.102\"", "disposition_clause: \"C-DISP\""],
    );
    repo.record("bytes", "pkg:cargo/bytes@1.0.0", &["version: \"1.0.0\""]);
    repo.record(
        "old",
        "pkg:cargo/old@0.1.0",
        &["version: \"0.1.0\"", "end_of_support: \"2026-05-31\""],
    );
    repo.record("zed", "pkg:cargo/zed@1.0.0", &["version: \"1.0.0\""]);
    repo.audit(&[
        ("RUSTSEC-2026-0190", "anyhow", "1.0.102"),
        ("RUSTSEC-2026-0001", "bytes", "1.0.0"),
        ("RUSTSEC-2026-0002", "ghost", "9.9.9"),
    ]);
    let binding = repo.binding(&["C-DISP", "C-OTHER"]);
    (repo, binding)
}

fn report_json(repo: &Repo, binding: &Path, extra: &[&str]) -> Value {
    let mut arguments = vec!["--json"];
    arguments.extend_from_slice(extra);
    let output = repo.report(binding, &arguments);
    assert!(output.status.success(), "{}", text(&output.stderr));
    serde_json::from_str(&text(&output.stdout)).unwrap()
}

fn row<'a>(report: &'a Value, purl: &str) -> &'a Value {
    report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["purl"] == purl)
        .unwrap()
}

fn keys(value: &Value, found: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                found.push(key.clone());
                keys(inner, found);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| keys(item, found)),
        _ => {}
    }
}

#[trace("FR-144-AC-1", "FR-144-AC-3", "FR-144-AC-11")]
#[test]
fn rows_carry_every_column_and_sort_unresolved_then_past_support_then_the_rest() {
    let (repo, binding) = standard();
    let report = report_json(&repo, &binding, &[]);
    let purls: Vec<&str> = report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["purl"].as_str().unwrap())
        .collect();
    assert_eq!(
        purls,
        [
            "pkg:cargo/bytes@1.0.0",
            "pkg:cargo/old@0.1.0",
            "pkg:cargo/anyhow@1.0.102",
            "pkg:cargo/zed@1.0.0",
        ]
    );
    for row in report["rows"].as_array().unwrap() {
        for column in [
            "purl",
            "version",
            "serves",
            "known_anomalies",
            "support_status",
            "end_of_support",
            "advisories",
        ] {
            assert!(row.get(column).is_some(), "{column} in {row}");
        }
    }
    let unresolved = &row(&report, "pkg:cargo/bytes@1.0.0")["advisories"][0];
    assert_eq!(unresolved["resolution"]["state"], "unresolved");
    assert_eq!(unresolved["resolution"]["reason"], "no disposition clause");
    let resolved = &row(&report, "pkg:cargo/anyhow@1.0.102")["advisories"][0]["resolution"];
    assert_eq!(resolved["state"], "resolved");
    assert_eq!(resolved["decision"], "accepted_risk");
    assert_eq!(resolved["attested_by"], "reviewer-1");
    assert_eq!(
        row(&report, "pkg:cargo/old@0.1.0")["past_end_of_support"],
        true
    );
    assert_eq!(
        row(&report, "pkg:cargo/zed@1.0.0")["past_end_of_support"],
        false
    );

    let markdown = text(&repo.report(&binding, &[]).stdout);
    assert!(markdown.starts_with("# SOUP report\n"), "{markdown}");
    assert!(
        markdown.contains("- Flags: unresolved advisory"),
        "{markdown}"
    );
    assert!(
        markdown.contains("- Flags: past end of support"),
        "{markdown}"
    );
}

#[trace("FR-144-AC-2")]
#[test]
fn absent_empty_and_stated_lists_render_not_stated_none_and_the_ids() {
    let repo = Repo::new();
    repo.record("a", "pkg:cargo/a@1.0.0", &[]);
    repo.record(
        "b",
        "pkg:cargo/b@1.0.0",
        &["serves: []", "known_anomalies: []"],
    );
    repo.record(
        "c",
        "pkg:cargo/c@1.0.0",
        &[
            "serves: [\"FR-1\", \"FR-2\"]",
            "known_anomalies:",
            "  - statement: \"leaks on close\"",
        ],
    );
    let binding = repo.binding(&["C-DISP"]);
    let report = report_json(&repo, &binding, &[]);
    for (purl, serves, anomalies) in [
        ("pkg:cargo/a@1.0.0", "not stated", "not stated"),
        ("pkg:cargo/b@1.0.0", "none", "none"),
        ("pkg:cargo/c@1.0.0", "stated", "stated"),
    ] {
        let row = row(&report, purl);
        assert_eq!(row["serves"]["state"], serves, "{purl}");
        assert_eq!(row["known_anomalies"]["state"], anomalies, "{purl}");
    }
    assert_eq!(
        row(&report, "pkg:cargo/c@1.0.0")["serves"]["items"],
        json!(["FR-1", "FR-2"])
    );
    let markdown = text(&repo.report(&binding, &[]).stdout);
    assert!(markdown.contains("- Serves: not stated"), "{markdown}");
    assert!(markdown.contains("- Serves: none"), "{markdown}");
    assert!(markdown.contains("- Serves: FR-1; FR-2"), "{markdown}");
    assert!(
        markdown.contains("- Known anomalies: leaks on close"),
        "{markdown}"
    );
    // Nothing is defaulted: the unstated support is `unknown`, not a value.
    assert!(markdown.contains("- Support status: unknown"), "{markdown}");
}

#[trace("FR-144-AC-5", "NFR-030-AC-2")]
#[test]
fn unmatched_advisories_and_unusable_scans_follow_the_rows_and_no_record_is_called_free() {
    let (repo, binding) = standard();
    let report = report_json(&repo, &binding, &[]);
    assert_eq!(report["unmatched"][0]["rule_id"], "RUSTSEC-2026-0002");
    assert_eq!(report["unmatched"][0]["reason"], "no record");
    let state = |tool: &str| {
        report["scans"]
            .as_array()
            .unwrap()
            .iter()
            .find(|scan| scan["tool"] == tool)
            .unwrap()["state"]
            .clone()
    };
    assert_eq!(state("cargo-audit"), "read");
    assert_eq!(state("cargo-deny"), "not run");
    // The deny scan never ran, so a record is not reported free of it.
    for row in report["rows"].as_array().unwrap() {
        let free = row["no_advisory_reported_by"].as_array().unwrap();
        assert!(!free.contains(&json!("cargo-deny")), "{row}");
    }
    let markdown = text(&repo.report(&binding, &[]).stdout);
    assert!(markdown.contains("cargo-deny: not assessed"), "{markdown}");
    assert!(markdown.find("## Unmatched advisories") > markdown.find("pkg:cargo/zed@1.0.0"));

    let vacuous = Repo::new();
    vacuous.record("a", "pkg:cargo/a@1.0.0", &[]);
    vacuous.store(&scan_record(
        "cargo-audit",
        "audit",
        &audit_document(&[]),
        true,
    ));
    let binding = vacuous.binding(&["C-DISP"]);
    let report = report_json(&vacuous, &binding, &[]);
    assert_eq!(report["scans"][0]["state"], "vacuous");
    let free = report["rows"][0]["no_advisory_reported_by"]
        .as_array()
        .unwrap();
    assert!(free.is_empty(), "{report}");
}

#[trace("FR-144-AC-6")]
#[test]
fn two_records_with_one_purl_refuse_the_report_naming_both_files() {
    let repo = Repo::new();
    let first = repo.record("one", "pkg:cargo/a@1.0.0", &[]);
    let second = repo.record("two", "pkg:cargo/a@1.0.0", &[]);
    let binding = repo.binding(&["C-DISP"]);
    let output = repo.report(&binding, &[]);
    assert!(!output.status.success());
    let stderr = text(&output.stderr);
    assert!(stderr.contains(first.to_str().unwrap()), "{stderr}");
    assert!(stderr.contains(second.to_str().unwrap()), "{stderr}");
    assert!(output.stdout.is_empty());
}

#[trace("FR-144-AC-7", "FR-144-AC-8", "FR-144-AC-9")]
#[test]
fn the_same_inputs_give_the_same_bytes_without_the_sbom_and_write_nothing() {
    let (repo, binding) = standard();
    // The SBOM is not an input: one in the repository or none gives one report.
    let sbom = repo.path().join("bom.json");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/fixtures/evidence/cyclonedx-real.json"),
        &sbom,
    )
    .unwrap();
    let before = repo.snapshot();
    let with_sbom = repo.report(&binding, &[]);
    let json_with = repo.report(&binding, &["--json"]);
    assert_eq!(repo.snapshot(), before);
    std::fs::remove_file(&sbom).unwrap();
    let without = repo.report(&binding, &[]);
    let json_without = repo.report(&binding, &["--json"]);
    assert!(with_sbom.status.success() && without.status.success());
    assert_eq!(with_sbom.stdout, without.stdout);
    assert_eq!(json_with.stdout, json_without.stdout);
    let again = repo.report(&binding, &[]);
    assert_eq!(again.stdout, with_sbom.stdout);
}

#[trace("FR-144-AC-10", "NFR-030-AC-3")]
#[test]
fn the_json_has_no_score_rank_class_or_percentage_key() {
    let (repo, binding) = standard();
    let mut found = Vec::new();
    keys(&report_json(&repo, &binding, &[]), &mut found);
    assert!(!found.is_empty());
    for key in found {
        for banned in ["score", "rank", "class", "percent", "rating", "risk"] {
            assert!(!key.to_lowercase().contains(banned), "{key}");
        }
    }
}

#[trace("FR-144-AC-12")]
#[test]
fn a_missing_binding_is_a_usage_error_and_a_foreign_clause_is_unresolved_with_its_reason() {
    let (repo, binding) = standard();
    let output = quoin(
        repo.home.path(),
        &[
            "soup",
            "report",
            "--repo",
            repo.path().to_str().unwrap(),
            "--as-of",
            AS_OF,
        ],
    );
    assert!(!output.status.success());
    assert!(
        text(&output.stderr).contains("binding"),
        "{}",
        text(&output.stderr)
    );

    repo.record(
        "stray",
        "pkg:cargo/stray@1.0.0",
        &["disposition_clause: \"C-NOT-IN-BINDING\""],
    );
    repo.audit(&[("RUSTSEC-2026-0003", "stray", "1.0.0")]);
    let report = report_json(&repo, &binding, &[]);
    let resolution = &row(&report, "pkg:cargo/stray@1.0.0")["advisories"][0]["resolution"];
    assert_eq!(resolution["state"], "unresolved");
    assert_eq!(
        resolution["reason"],
        "the disposition clause is not a clause of the binding report"
    );
}

#[trace("FR-143-AC-1")]
#[test]
fn an_invalid_support_entry_refuses_the_whole_file_and_produces_no_report() {
    let (repo, binding) = standard();
    let entry = |extra: Value| {
        let mut entry = json!({"purl": "pkg:cargo/old", "support_status": "unsupported",
            "end_of_support": "2026-05-31", "source": "https://example.invalid/policy"});
        entry
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        entry
    };
    let valid = entry(json!({}));
    let cases = [
        json!([entry(json!({"source": ""}))]),
        json!([entry(json!({"support_status": "maintained"}))]),
        json!([entry(json!({"end_of_support": "soon"}))]),
        json!([valid.clone(), valid]),
    ];
    for (index, case) in cases.iter().enumerate() {
        let support = repo.write("support.json", case);
        let output = repo.report(&binding, &["--support", support.to_str().unwrap()]);
        assert!(!output.status.success(), "case {index}");
        assert!(output.stdout.is_empty(), "case {index}");
        assert!(
            text(&output.stderr).contains("pkg:cargo/old"),
            "{}",
            text(&output.stderr)
        );
    }
}

#[trace("FR-143-AC-2", "FR-143-AC-3", "FR-143-AC-5", "FR-143-AC-9")]
#[test]
fn supplied_support_fills_unknown_never_writes_the_record_and_dates_compare_by_utc() {
    let repo = Repo::new();
    let unknown = repo.record(
        "u",
        "pkg:cargo/u@1.0.0",
        &["end_of_support: unknown", "support_status: unknown"],
    );
    repo.record(
        "s",
        "pkg:cargo/s@1.0.0",
        &["end_of_support: \"2027-01-01\""],
    );
    let binding = repo.binding(&["C-DISP"]);
    let support = repo.write(
        "support.json",
        &json!([
            {"purl": "pkg:cargo/u", "support_status": "unsupported", "end_of_support": "2026-05-31",
             "source": "https://example.invalid/u"},
            {"purl": "pkg:cargo/s", "support_status": "supported", "end_of_support": "2026-06-01",
             "source": "https://example.invalid/s"},
        ]),
    );
    let record_before = std::fs::read(&unknown).unwrap();
    let report = report_json(&repo, &binding, &["--support", support.to_str().unwrap()]);
    let u = row(&report, "pkg:cargo/u@1.0.0");
    assert_eq!(u["end_of_support"]["value"], "2026-05-31");
    assert_eq!(u["end_of_support"]["origin"], "support file");
    assert_eq!(u["end_of_support"]["source"], "https://example.invalid/u");
    assert_eq!(u["past_end_of_support"], true);
    assert_eq!(std::fs::read(&unknown).unwrap(), record_before);

    let s = row(&report, "pkg:cargo/s@1.0.0");
    assert_eq!(s["end_of_support"]["value"], "2027-01-01");
    assert_eq!(s["end_of_support"]["origin"], "record");
    assert_eq!(s["end_of_support"]["disagrees_with"]["value"], "2026-06-01");
    assert_eq!(s["support_disagreement"], true);
    assert_eq!(s["past_end_of_support"], false);

    // 2026-05-31 ends a component; the as-of date decides, by its UTC date.
    let flagged = |as_of: &str| {
        let output = quoin(
            repo.home.path(),
            &[
                "soup",
                "report",
                "--repo",
                repo.path().to_str().unwrap(),
                "--binding",
                binding.to_str().unwrap(),
                "--as-of",
                as_of,
                "--support",
                support.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(output.status.success(), "{}", text(&output.stderr));
        let report: Value = serde_json::from_str(&text(&output.stdout)).unwrap();
        row(&report, "pkg:cargo/u@1.0.0")["past_end_of_support"].clone()
    };
    assert_eq!(flagged("2026-05-31"), false);
    assert_eq!(flagged("2026-06-01"), true);
    assert_eq!(flagged("2026-05-31T12:00:00Z"), false);
    assert_eq!(flagged("2026-06-01T00:00:00Z"), true);
    assert_eq!(flagged("2026-05-31T23:00:00-02:00"), true);
}

#[trace("FR-143-AC-4")]
#[test]
fn a_record_with_no_support_data_is_unknown_and_not_flagged() {
    let repo = Repo::new();
    repo.record("u", "pkg:cargo/u@1.0.0", &["support_status: unknown"]);
    let binding = repo.binding(&["C-DISP"]);
    let report = report_json(&repo, &binding, &[]);
    let u = row(&report, "pkg:cargo/u@1.0.0");
    assert_eq!(u["support_status"]["value"], "unknown");
    assert_eq!(u["end_of_support"]["value"], "unknown");
    assert_eq!(u["past_end_of_support"], false);
}

#[trace("FR-143-AC-10")]
#[test]
fn a_record_outside_the_support_vocabulary_refuses_the_report_naming_the_record() {
    let repo = Repo::new();
    let bad = repo.record(
        "bad",
        "pkg:cargo/bad@1.0.0",
        &["support_status: maintained"],
    );
    let binding = repo.binding(&["C-DISP"]);
    let output = repo.report(&binding, &[]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = text(&output.stderr);
    assert!(stderr.contains(bad.to_str().unwrap()), "{stderr}");
}

#[trace("FR-143-AC-6")]
#[test]
fn a_versioned_entry_beats_a_version_less_one() {
    let repo = Repo::new();
    repo.record("a", "pkg:cargo/a@1.0.0", &[]);
    let binding = repo.binding(&["C-DISP"]);
    let support = repo.write(
        "support.json",
        &json!([
            {"purl": "pkg:cargo/a", "support_status": "supported", "end_of_support": "unknown",
             "source": "general"},
            {"purl": "pkg:cargo/a@1.0.0", "support_status": "unsupported", "end_of_support": "unknown",
             "source": "specific"},
            {"purl": "pkg:cargo/a@2.0.0", "support_status": "supported", "end_of_support": "unknown",
             "source": "other version"},
        ]),
    );
    let report = report_json(&repo, &binding, &["--support", support.to_str().unwrap()]);
    let status = &row(&report, "pkg:cargo/a@1.0.0")["support_status"];
    assert_eq!(status["value"], "unsupported");
    assert_eq!(status["source"], "specific");
}

#[trace("FR-144-AC-4")]
#[test]
fn a_record_with_both_flags_appears_once_in_the_first_group_showing_both() {
    let repo = Repo::new();
    repo.record(
        "both",
        "pkg:cargo/zboth@1.0.0",
        &["version: \"1.0.0\"", "end_of_support: \"2026-05-31\""],
    );
    repo.record("adv", "pkg:cargo/adv@1.0.0", &["version: \"1.0.0\""]);
    repo.record(
        "old",
        "pkg:cargo/old@1.0.0",
        &["version: \"1.0.0\"", "end_of_support: \"2026-05-31\""],
    );
    repo.audit(&[
        ("RUSTSEC-2026-0001", "zboth", "1.0.0"),
        ("RUSTSEC-2026-0002", "adv", "1.0.0"),
    ]);
    let binding = repo.binding(&["C-DISP"]);
    let report = report_json(&repo, &binding, &[]);
    let purls: Vec<&str> = report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["purl"].as_str().unwrap())
        .collect();
    // Once, and in the first group (ordered by purl), ahead of the record
    // that is only past end of support.
    assert_eq!(
        purls,
        [
            "pkg:cargo/adv@1.0.0",
            "pkg:cargo/zboth@1.0.0",
            "pkg:cargo/old@1.0.0"
        ]
    );
    let both = row(&report, "pkg:cargo/zboth@1.0.0");
    assert_eq!(both["unresolved_advisory"], true);
    assert_eq!(both["past_end_of_support"], true);
    let markdown = text(&repo.report(&binding, &[]).stdout);
    assert_eq!(
        markdown.matches("pkg:cargo/zboth@1.0.0").count(),
        1,
        "{markdown}"
    );
    assert!(
        markdown.contains("- Flags: unresolved advisory, past end of support"),
        "{markdown}"
    );
}

#[trace("FR-143-AC-7")]
#[test]
fn the_report_is_the_same_with_no_network_available() {
    let (repo, binding) = standard();
    let normal = repo.report(&binding, &[]);
    assert!(normal.status.success(), "{}", text(&normal.stderr));
    let mut arguments = vec![
        "-rn".to_owned(),
        env!("CARGO_BIN_EXE_quoin").to_owned(),
        "soup".to_owned(),
        "report".to_owned(),
    ];
    arguments.extend(
        [
            "--repo",
            repo.path().to_str().unwrap(),
            "--binding",
            binding.to_str().unwrap(),
            "--as-of",
            AS_OF,
        ]
        .map(str::to_owned),
    );
    let probe = Command::new("unshare").args(["-rn", "true"]).output();
    if !probe.is_ok_and(|probe| probe.status.success()) {
        eprintln!("skipped: unshare -rn (a network namespace) is unavailable here");
        return;
    }
    let isolated = Command::new("unshare")
        .args(&arguments)
        .env("IX_HOME", repo.home.path())
        .output()
        .unwrap();
    assert!(isolated.status.success(), "{}", text(&isolated.stderr));
    assert_eq!(isolated.stdout, normal.stdout);
}

#[trace("FR-143-AC-8")]
#[test]
fn the_wall_clock_is_not_an_input_only_as_of_is() {
    let repo = Repo::new();
    // Past by today's clock, not yet at the `--as-of` instant of 2026-08-15.
    repo.record(
        "a",
        "pkg:cargo/a@1.0.0",
        &["end_of_support: \"2999-01-01\""],
    );
    repo.record(
        "b",
        "pkg:cargo/b@1.0.0",
        &["end_of_support: \"2026-10-01\""],
    );
    repo.record(
        "c",
        "pkg:cargo/c@1.0.0",
        &["end_of_support: \"2026-08-14\""],
    );
    let binding = repo.binding(&["C-DISP"]);
    let report = report_json(&repo, &binding, &[]);
    assert_eq!(
        row(&report, "pkg:cargo/a@1.0.0")["past_end_of_support"],
        false
    );
    assert_eq!(
        row(&report, "pkg:cargo/b@1.0.0")["past_end_of_support"],
        false
    );
    assert_eq!(
        row(&report, "pkg:cargo/c@1.0.0")["past_end_of_support"],
        true
    );
    let later = repo.report(&binding, &["--json"]);
    assert_eq!(later.stdout, repo.report(&binding, &["--json"]).stdout);
}

#[trace("FR-144-AC-5", "NFR-030-AC-2")]
#[test]
fn a_cargo_deny_log_refused_at_recording_leaves_the_scan_not_run_and_no_record_free() {
    let refused = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../quoin-evidence/tests/fixtures/cargo-deny-failed-real.jsonl"),
    )
    .unwrap();
    let adapter = select_finding_adapter(AdapterSelection {
        adapter: None,
        tool: Some("cargo-deny"),
    })
    .unwrap();
    assert!(
        adapter.run(&refused, None).is_err(),
        "the log must be refused"
    );

    let repo = Repo::new();
    repo.record("a", "pkg:cargo/a@1.0.0", &[]);
    repo.audit(&[]);
    let binding = repo.binding(&["C-DISP"]);
    let report = report_json(&repo, &binding, &[]);
    let deny = report["scans"]
        .as_array()
        .unwrap()
        .iter()
        .find(|scan| scan["tool"] == "cargo-deny")
        .unwrap();
    assert_eq!(deny["state"], "not run");
    let free = report["rows"][0]["no_advisory_reported_by"]
        .as_array()
        .unwrap();
    assert!(!free.contains(&json!("cargo-deny")), "{report}");
    let markdown = text(&repo.report(&binding, &[]).stdout);
    assert!(markdown.contains("cargo-deny: not assessed"), "{markdown}");
}

fn walk(value: &Value, found: &mut Vec<Value>) {
    match value {
        Value::Object(map) => map.values().for_each(|inner| walk(inner, found)),
        Value::Array(items) => items.iter().for_each(|item| walk(item, found)),
        other => found.push(other.clone()),
    }
}

#[trace("NFR-030-AC-1")]
#[test]
fn with_every_optional_input_absent_every_value_is_unknown_not_stated_or_an_input() {
    let repo = Repo::new();
    repo.record("a", "pkg:cargo/a@1.0.0", &[]);
    let binding = repo.binding(&[]);
    let report = report_json(&repo, &binding, &[]);
    let row = row(&report, "pkg:cargo/a@1.0.0");
    let inputs = [
        row["purl"].as_str().unwrap(),
        row["id"].as_str().unwrap(),
        row["path"].as_str().unwrap(),
    ];
    let mut leaves = Vec::new();
    walk(row, &mut leaves);
    for leaf in leaves {
        match leaf {
            Value::String(word) => assert!(
                [
                    "unknown",
                    "not stated",
                    "none",
                    // No licence-policy scan is retained: the one status a row
                    // states, rather than an invented verdict.
                    "policy findings not assessed: licence-policy not run",
                ]
                .contains(&word.as_str())
                    || inputs.contains(&word.as_str()),
                "{word} in {row}"
            ),
            // Absent, or a flag computed from the inputs (nothing is past
            // support or unresolved when no input says so).
            other => assert!(
                other.is_null() || other == Value::Bool(false),
                "{other} in {row}"
            ),
        }
    }
    assert_eq!(row["advisories"], json!([]));
}

#[trace("FR-144-AC-13")]
#[test]
fn a_records_directory_that_does_not_exist_refuses_naming_it() {
    let repo = Repo::new();
    repo.record("a", "pkg:cargo/a@1.0.0", &[]);
    let binding = repo.binding(&["C-DISP"]);
    let output = repo.report(&binding, &["--records", "spec/typo"]);
    assert!(!output.status.success());
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("spec/typo") && stderr.contains("soup.path_missing"),
        "{stderr}"
    );
    assert!(output.stdout.is_empty());
}

#[trace("FR-144-AC-14")]
#[test]
fn a_soup_record_with_no_purl_refuses_naming_its_file() {
    let repo = Repo::new();
    repo.record("a", "pkg:cargo/a@1.0.0", &[]);
    let path = repo.records().join("nopurl.md");
    std::fs::write(
        &path,
        "---\nid: SOUP-x\ntitle: x\ntype: soup_record\n---\n# x\n",
    )
    .unwrap();
    let binding = repo.binding(&["C-DISP"]);
    let output = repo.report(&binding, &[]);
    assert!(!output.status.success());
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("nopurl.md") && stderr.contains("no purl"),
        "{stderr}"
    );
    assert!(output.stdout.is_empty());
}

#[trace("FR-144-AC-15")]
#[test]
fn a_version_or_source_ref_of_the_wrong_type_refuses_instead_of_reading_as_not_stated() {
    for (extra, field) in [
        (vec!["version: 1.0"], "version"),
        (
            vec![
                "known_anomalies:",
                "  - statement: \"leaks\"",
                "    source_ref: 7",
            ],
            "known_anomalies",
        ),
    ] {
        let repo = Repo::new();
        repo.record("a", "pkg:cargo/a@1.0.0", &extra);
        let binding = repo.binding(&["C-DISP"]);
        let output = repo.report(&binding, &[]);
        assert!(!output.status.success(), "{field}");
        let stderr = text(&output.stderr);
        assert!(
            stderr.contains("soup.record_field_invalid") && stderr.contains(field),
            "{stderr}"
        );
        assert!(output.stdout.is_empty());
    }
}

/// A minimal `CycloneDX` inventory written by hand for the report fixtures; the
/// real Syft capture is exercised by the join tests.
fn hand_written_sbom(components: &[(&str, &str, Option<&str>)]) -> String {
    let components: Vec<Value> = components
        .iter()
        .map(|(name, version, licence)| {
            let mut component = json!({
                "name": name, "version": version,
                "purl": format!("pkg:{name}@{version}"),
            });
            if let Some(licence) = licence {
                component["licenses"] = json!([{ "license": { "id": licence } }]);
            }
            component
        })
        .collect();
    json!({ "bomFormat": "CycloneDX", "components": components }).to_string()
}

const LICENCE_POLICY: &str = "format: licence-policy-v1\nallow: [MIT]\ndeny: [GPL-3.0-only]\nban:\n  - purl: pkg:npm/banned-pkg\n    reason: abandoned\n";

impl Repo {
    /// Store a `licence-policy` scan made the way recording makes one.
    fn licence(&self, suite: &str, timestamp: &str, sbom: &str) {
        let parsed = quoin_evidence::adapters::parse_licence_policy(sbom, LICENCE_POLICY).unwrap();
        self.store(&FindingRecord {
            schema_version: STORE_SCHEMA_VERSION,
            suite: SuiteId::new(suite),
            tool: "licence-policy".to_owned(),
            evidence_kind: None,
            timestamp: timestamp.to_owned(),
            ruleset: parsed.ruleset,
            rules_evaluated: parsed.rules_evaluated,
            findings: parsed.findings,
            evaluated: parsed.evaluated,
        });
    }
}

/// An npm record with a denied licence resolved by an attested disposition, a
/// `PyPI` record with an unknown licence unresolved, and an npm record banned:
/// each on its own row with tool, rule id and severity word, apart from the
/// advisories, the unresolved ones in the first group.
#[trace("FR-144-AC-18")]
#[test]
fn policy_findings_render_on_their_rows_apart_from_advisories() {
    let repo = Repo::new();
    repo.record(
        "denied",
        "pkg:npm/denied-pkg@1.0.0",
        &["disposition_clause: \"C-DISP\""],
    );
    repo.record("unknown", "pkg:pypi/unknown-pkg@2.0.0", &[]);
    repo.record("banned", "pkg:npm/banned-pkg@3.0.0", &[]);
    repo.licence(
        "lic",
        SCANNED,
        &hand_written_sbom(&[
            ("npm/denied-pkg", "1.0.0", Some("GPL-3.0-only")),
            ("pypi/unknown-pkg", "2.0.0", None),
            ("npm/banned-pkg", "3.0.0", Some("MIT")),
        ]),
    );
    let binding = repo.binding(&["C-DISP"]);
    let report = report_json(&repo, &binding, &[]);

    let purls: Vec<&str> = report["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["purl"].as_str().unwrap())
        .collect();
    assert_eq!(
        purls,
        [
            "pkg:npm/banned-pkg@3.0.0",
            "pkg:pypi/unknown-pkg@2.0.0",
            "pkg:npm/denied-pkg@1.0.0",
        ]
    );
    let denied = row(&report, "pkg:npm/denied-pkg@1.0.0");
    assert_eq!(denied["advisories"], json!([]));
    let finding = &denied["policy_findings"][0];
    assert_eq!(finding["tool"], "licence-policy");
    assert_eq!(finding["rule_id"], "licence-denied");
    assert_eq!(finding["severity"], "denied");
    assert_eq!(finding["resolution"]["state"], "resolved");
    assert_eq!(finding["resolution"]["attested_by"], "reviewer-1");
    let unknown = &row(&report, "pkg:pypi/unknown-pkg@2.0.0")["policy_findings"][0];
    assert_eq!(unknown["rule_id"], "licence-unknown");
    assert_eq!(unknown["message"], "no licence declared");
    assert_eq!(unknown["resolution"]["state"], "unresolved");
    assert_eq!(unknown["resolution"]["reason"], "no disposition clause");
    let banned = row(&report, "pkg:npm/banned-pkg@3.0.0");
    assert_eq!(banned["policy_findings"][0]["rule_id"], "package-banned");
    assert_eq!(banned["policy_findings"][0]["severity"], "banned");
    assert_eq!(banned["unresolved_policy_finding"], true);
    assert_eq!(banned["unresolved_advisory"], false);

    let markdown = text(&repo.report(&binding, &[]).stdout);
    assert!(
        markdown
            .contains("licence-policy package-banned pkg:npm/banned-pkg@3.0.0 (severity: banned;"),
        "{markdown}"
    );
    assert!(markdown.contains("unresolved policy finding"), "{markdown}");
}

/// Not run, vacuous, unordered, and a record absent from `evaluated` each state
/// their own status; none states `no policy findings`; a finding for a purl with
/// no record is appended as unmatched.
#[trace("FR-144-AC-19", "NFR-030-AC-5")]
#[test]
fn a_row_states_why_it_has_no_policy_verdict_and_never_allowed() {
    let repo = Repo::new();
    repo.record("clean", "pkg:npm/clean-pkg@1.0.0", &[]);
    repo.record("later", "pkg:npm/later-pkg@1.0.0", &[]);
    let binding = repo.binding(&["C-DISP"]);
    let statuses = |report: &Value| -> Vec<String> {
        report["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["policy_status"]["state"].as_str().unwrap().to_owned())
            .collect()
    };

    let not_run = report_json(&repo, &binding, &[]);
    assert_eq!(
        statuses(&not_run),
        ["policy findings not assessed: licence-policy not run"; 2]
    );
    assert!(text(&repo.report(&binding, &[]).stdout).contains("- licence-policy: not run"));

    let vacuous_sbom = r#"{"bomFormat":"CycloneDX","components":[]}"#;
    repo.licence("vacuous", SCANNED, vacuous_sbom);
    assert_eq!(
        statuses(&report_json(&repo, &binding, &[])),
        ["policy findings not assessed: licence-policy scan vacuous"; 2]
    );

    let sbom = hand_written_sbom(&[
        ("npm/clean-pkg", "1.0.0", Some("MIT")),
        ("npm/ghost-pkg", "9.9.9", Some("GPL-3.0-only")),
    ]);
    repo.licence("vacuous", "2026-08-03T00:00:00.000Z", &sbom);
    let read = report_json(&repo, &binding, &[]);
    let clean = row(&read, "pkg:npm/clean-pkg@1.0.0");
    assert_eq!(
        clean["policy_status"]["state"],
        "no policy findings in the scan read"
    );
    assert_eq!(
        clean["policy_status"]["scan_timestamp"],
        "2026-08-03T00:00:00.000Z"
    );
    assert!(
        clean["policy_status"]["ruleset"]
            .as_str()
            .unwrap()
            .contains("GPL-3.0-only")
    );
    let markdown = text(&repo.report(&binding, &[]).stdout);
    assert!(
        markdown.contains("no policy findings in the scan read (licence-policy, 2026-08-03T00:00:00.000Z, ruleset: "),
        "{markdown}"
    );
    // No record is ever stated allowed: no word `allowed` stands alone in either rendering.
    let json_text = serde_json::to_string(&read).unwrap();
    for rendering in [&markdown, &json_text] {
        assert!(
            !rendering
                .split(|c: char| !(c.is_alphanumeric() || c == '-'))
                .any(|word| word == "allowed"),
            "{rendering}"
        );
    }
    assert_eq!(
        row(&read, "pkg:npm/later-pkg@1.0.0")["policy_status"]["state"],
        "not evaluated by the licence-policy scan read"
    );
    assert_eq!(
        read["unmatched_policy_findings"][0]["package"],
        "pkg:npm/ghost-pkg@9.9.9"
    );
    assert_eq!(read["unmatched_policy_findings"][0]["reason"], "no record");
    assert!(text(&repo.report(&binding, &[]).stdout).contains("## Unmatched policy findings"));

    repo.licence("garbled", "not an instant", &sbom);
    assert_eq!(
        statuses(&report_json(&repo, &binding, &[])),
        ["policy findings not assessed: licence-policy scan unordered"; 2]
    );
}

/// One disposition attested between an advisory scan and a later
/// `licence-policy` scan leaves the advisory resolved and the ban finding
/// unresolved, and the row states the scope.
#[trace("FR-144-AC-20")]
#[test]
fn a_disposition_does_not_cover_a_later_policy_scan() {
    let repo = Repo::new();
    repo.record(
        "anyhow",
        "pkg:cargo/anyhow@1.0.102",
        &["disposition_clause: \"C-DISP\""],
    );
    repo.audit(&[("RUSTSEC-2026-0190", "anyhow", "1.0.102")]);
    let ban = "format: licence-policy-v1\nallow: [MIT]\nban:\n  - purl: pkg:cargo/anyhow\n";
    let sbom = hand_written_sbom(&[("cargo/anyhow", "1.0.102", Some("MIT"))]);
    let parsed = quoin_evidence::adapters::parse_licence_policy(&sbom, ban).unwrap();
    repo.store(&FindingRecord {
        schema_version: STORE_SCHEMA_VERSION,
        suite: SuiteId::new("lic"),
        tool: "licence-policy".to_owned(),
        evidence_kind: None,
        timestamp: "2026-08-02T00:00:00.000Z".to_owned(),
        ruleset: parsed.ruleset,
        rules_evaluated: parsed.rules_evaluated,
        findings: parsed.findings,
        evaluated: parsed.evaluated,
    });
    let binding = repo.binding(&["C-DISP"]);
    let report = report_json(&repo, &binding, &[]);
    let anyhow = row(&report, "pkg:cargo/anyhow@1.0.102");
    assert_eq!(anyhow["advisories"][0]["resolution"]["state"], "resolved");
    assert_eq!(anyhow["policy_findings"][0]["rule_id"], "package-banned");
    assert_eq!(
        anyhow["policy_findings"][0]["resolution"]["state"],
        "unresolved"
    );
    assert_eq!(
        anyhow["policy_findings"][0]["resolution"]["reason"],
        "scan newer than attestation"
    );
    assert!(
        anyhow["disposition_scope"]
            .as_str()
            .unwrap()
            .contains("at or before its attestation")
    );
    let markdown = text(&repo.report(&binding, &[]).stdout);
    assert!(markdown.contains("- Disposition scope: "), "{markdown}");
}

/// An unmatched no-purl finding could belong to an evaluated record, so the row
/// states why it cannot be called clean (FR-144's closed status list names it).
#[trace("FR-144-AC-19", "NFR-030-AC-5")]
#[test]
fn an_unmatched_finding_that_could_belong_blocks_a_clean_statement() {
    let repo = Repo::new();
    repo.record("clean", "pkg:npm/clean-pkg@1.0.0", &[]);
    let binding = repo.binding(&["C-DISP"]);
    let sbom = r#"{"bomFormat":"CycloneDX","components":[
        {"name":"clean-pkg","version":"1.0.0","purl":"pkg:npm/clean-pkg@1.0.0","licenses":[{"license":{"id":"MIT"}}]},
        {"name":"mystery","version":"2"}]}"#;
    repo.licence("lic", SCANNED, sbom);
    let report = report_json(&repo, &binding, &[]);
    assert_eq!(
        row(&report, "pkg:npm/clean-pkg@1.0.0")["policy_status"]["state"],
        "policy findings not assessed: an unmatched licence-policy finding could belong to this record"
    );
    assert_eq!(report["unmatched_policy_findings"][0]["reason"], "no purl");
}
