// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The `quoin soup vex export --format cyclonedx` command surface (FR-156).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use quoin_evidence::store::write_scan;
use quoin_evidence::types::{Finding, FindingRecord};
use quoin_evidence::{DiskEvidence, STORE_SCHEMA_VERSION, SuiteId};
use serde_json::{Value, json};

const NPM_PURL: &str = "pkg:npm/%40babel/core@12.3.1";
const NPM_CASE_PURL: &str = "pkg:npm/%40Babel/Core@12.3.1";
const PYPI_PURL: &str = "pkg:pypi/requests@2.31.0";
const PYPI_CASE_PURL: &str = "pkg:pypi/Requests@2.31.0";

struct Fixture {
    root: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("spec/soup")).unwrap();
        Self { root, home }
    }

    fn path(&self) -> &Path {
        self.root.path()
    }

    fn record(&self, name: &str, purl: &str, version: &str, statements: Option<Value>) {
        let mut front = format!(
            "---\nid: SOUP-{name}\ntitle: \"{name}\"\ntype: soup_record\nobject: soup_record\npurl: \"{purl}\"\nversion: \"{version}\"\n"
        );
        if let Some(statements) = statements {
            front.push_str("vex_statements: ");
            front.push_str(&serde_json::to_string(&statements).unwrap());
            front.push('\n');
        }
        front.push_str(
            "---\n# [SOUP-RECORD] Component\n\n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|---|---|---|---|\n| component_id | UUID | 1..1 | identity |\n| purl | String | 1..1 | minLength: 1 |\n",
        );
        std::fs::write(self.path().join(format!("spec/soup/{name}.md")), front).unwrap();
    }

    fn command(&self, output: Option<&Path>, sbom: Option<&Path>) -> Output {
        let security_module = security_module();
        let module_root = security_module
            .parent()
            .expect("the security module has a parent directory");
        let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
        command
            .env("IX_FILAMENT_MODULES_PATH", module_root)
            .env("HOME", self.home.path())
            .env("CYCLONEDX_SCHEMA_PATH", schema_path())
            .args(["soup", "vex", "export", "--format", "cyclonedx", "--repo"])
            .arg(self.path());
        if let Some(output) = output {
            command.arg("--out").arg(output);
        }
        if let Some(sbom) = sbom {
            command.arg("--sbom").arg(sbom);
        }
        command.output().unwrap()
    }

    fn store_scan(&self, suite: &str, findings: Vec<Finding>, rules_evaluated: u64) {
        let scan = FindingRecord {
            schema_version: STORE_SCHEMA_VERSION,
            suite: SuiteId::new(suite),
            tool: "osv-scanner 2.7.1".to_owned(),
            evidence_kind: None,
            timestamp: "2026-11-25T10:00:00.000Z".to_owned(),
            ruleset: Some("osv-db-1".to_owned()),
            rules_evaluated: Some(rules_evaluated),
            findings,
            evaluated: None,
        };
        write_scan(&mut DiskEvidence::new(self.path()), &scan).unwrap();
    }
}

fn security_module() -> PathBuf {
    let root = std::env::var_os("SPEC_OBJECTS_SECURITY_DIR").map_or_else(
        || {
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../../spec-objects-security/spec_objects_security")
        },
        PathBuf::from,
    );
    std::fs::canonicalize(&root).unwrap_or_else(|error| {
        panic!(
            "spec-objects-security module is not available at {}: {error}; set SPEC_OBJECTS_SECURITY_DIR",
            root.display()
        )
    })
}

fn schema_path() -> PathBuf {
    std::env::var_os("CYCLONEDX_SCHEMA_PATH")
        .map(PathBuf::from)
        .expect("CYCLONEDX_SCHEMA_PATH must name the fetched CycloneDX 1.7 schema")
}

fn statement(advisory: &str, status: &str, justification: Option<&str>) -> Value {
    let mut statement = json!({
        "advisory": advisory,
        "status": status,
        "assessed_by": "Ada Example",
        "assessed_at": "2026-11-23",
        "impact_statement": "Impact statement authored by Ada.",
        "action_statement": "Upgrade to the fixed release.",
        "evidence_refs": ["ix://agent-ix/quoin/EVIDENCE-1"]
    });
    if let Some(justification) = justification {
        statement["justification"] = json!(justification);
    }
    statement
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn document(output: &Output, path: &Path) -> Value {
    assert!(output.status.success(), "{}", stderr(output));
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn assert_matches_cyclonedx_17_schema(document: &Value) {
    let path = schema_path();
    let schema: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let cryptography_path = path.with_file_name("cryptography-defs.schema.json");
    let cryptography: Value =
        serde_json::from_slice(&std::fs::read(&cryptography_path).unwrap()).unwrap();
    let jsf_path = path.with_file_name("jsf-0.82.schema.json");
    let jsf: Value = serde_json::from_slice(&std::fs::read(&jsf_path).unwrap()).unwrap();
    let spdx_path = path.with_file_name("spdx.schema.json");
    let spdx: Value = serde_json::from_slice(&std::fs::read(&spdx_path).unwrap()).unwrap();
    let validator = quoin_jsonschema::validator::SchemaValidator::compile_vendored(
        &path,
        &schema,
        &[
            (
                "http://cyclonedx.org/schema/cryptography-defs.schema.json",
                cryptography,
            ),
            ("http://cyclonedx.org/schema/jsf-0.82.schema.json", jsf),
            ("http://cyclonedx.org/schema/spdx.schema.json", spdx),
        ],
    )
    .expect("the fetched CycloneDX 1.7 schema must compile");
    let errors = validator.errors(document);
    assert!(
        errors.is_empty(),
        "generated CycloneDX BOM does not match the published 1.7 schema: {errors:#?}"
    );
}

#[trace("FR-156-AC-1", "FR-156-AC-8", "FR-156-AC-5", "FR-156-CON-1")]
#[test]
fn purl_components_follow_type_definitions_and_bom_matches_schema() {
    let fixture = Fixture::new();
    fixture.record(
        "babel-core",
        NPM_PURL,
        "12.3.1",
        Some(json!([statement("CVE-2026-0001", "affected", None)])),
    );
    fixture.record(
        "requests",
        PYPI_PURL,
        "2.31.0",
        Some(json!([statement(
            "CVE-2026-0002",
            "not_affected",
            Some("component_not_present")
        )])),
    );
    fixture.record(
        "case-sensitive",
        NPM_CASE_PURL,
        "12.3.1",
        Some(json!([statement("CVE-2026-0003", "affected", None)])),
    );
    fixture.record(
        "normalized-name",
        PYPI_CASE_PURL,
        "2.31.0",
        Some(json!([statement("CVE-2026-0004", "affected", None)])),
    );
    let output_path = fixture.path().join("out.json");
    let result = document(&fixture.command(Some(&output_path), None), &output_path);

    assert_eq!(result["bomFormat"], "CycloneDX");
    assert_eq!(result["specVersion"], "1.7");
    let components = result["components"].as_array().unwrap();
    let npm = components
        .iter()
        .find(|component| component["purl"] == NPM_PURL)
        .unwrap();
    assert_eq!(npm["type"], "library");
    assert_eq!(npm["group"], "@babel");
    assert_eq!(npm["name"], "core");
    assert_eq!(npm["bom-ref"], NPM_PURL);
    let pypi = components
        .iter()
        .find(|component| component["purl"] == PYPI_PURL)
        .unwrap();
    assert_eq!(pypi["type"], "library");
    assert_eq!(pypi["name"], "requests");
    assert!(pypi.get("group").is_none());
    assert_eq!(pypi["bom-ref"], PYPI_PURL);
    let case_sensitive = components
        .iter()
        .find(|component| component["purl"] == NPM_CASE_PURL)
        .unwrap();
    assert_eq!(case_sensitive["group"], "@Babel");
    assert_eq!(case_sensitive["name"], "Core");
    assert_eq!(case_sensitive["bom-ref"], NPM_CASE_PURL);
    let normalized = components
        .iter()
        .find(|component| component["purl"] == PYPI_CASE_PURL)
        .unwrap();
    assert_eq!(normalized["name"], "requests");
    assert_matches_cyclonedx_17_schema(&result);
}

#[trace("FR-156-AC-2")]
#[test]
fn four_statuses_map_to_cyclonedx_analysis_and_version_status() {
    let fixture = Fixture::new();
    fixture.record(
        "statuses",
        NPM_PURL,
        "12.3.1",
        Some(json!([
            statement("CVE-AFFECTED", "affected", None),
            statement(
                "CVE-NOT-AFFECTED",
                "not_affected",
                Some("component_not_present")
            ),
            statement(
                "CVE-INVESTIGATING",
                "under_investigation",
                Some("vulnerable_code_not_present")
            ),
            statement(
                "CVE-FIXED",
                "fixed",
                Some("inline_mitigations_already_exist")
            ),
        ])),
    );
    let output_path = fixture.path().join("status.json");
    let result = document(&fixture.command(Some(&output_path), None), &output_path);
    let vulnerabilities = result["vulnerabilities"].as_array().unwrap();
    assert_eq!(vulnerabilities.len(), 4);
    let expected = [
        ("CVE-AFFECTED", "exploitable", "affected"),
        ("CVE-NOT-AFFECTED", "not_affected", "unaffected"),
        ("CVE-INVESTIGATING", "in_triage", "unknown"),
        ("CVE-FIXED", "resolved", "unaffected"),
    ];
    for (id, state, version_status) in expected {
        let entry = vulnerabilities
            .iter()
            .find(|item| item["id"] == id)
            .unwrap();
        assert_eq!(entry["analysis"]["state"], state);
        assert_eq!(entry["affects"][0]["versions"][0]["version"], "12.3.1");
        assert_eq!(entry["affects"][0]["versions"][0]["status"], version_status);
        assert_eq!(
            entry["properties"][0]["value"],
            match id {
                "CVE-AFFECTED" => "affected",
                "CVE-NOT-AFFECTED" => "not_affected",
                "CVE-INVESTIGATING" => "under_investigation",
                _ => "fixed",
            }
        );
    }
}

#[trace("FR-156-AC-3", "FR-156-AC-4", "FR-156-AC-10")]
#[test]
fn authored_fields_and_all_justifications_are_preserved_without_advisory_deduplication_across_records()
 {
    let fixture = Fixture::new();
    fixture.record(
        "a",
        NPM_PURL,
        "12.3.1",
        Some(json!([
            statement(
                "CVE-SHARED",
                "affected",
                Some("vulnerable_code_not_present")
            ),
            statement(
                "CVE-JUST-2",
                "affected",
                Some("vulnerable_code_not_in_execute_path")
            ),
            statement(
                "CVE-JUST-3",
                "affected",
                Some("inline_mitigations_already_exist")
            ),
        ])),
    );
    fixture.record(
        "b",
        PYPI_PURL,
        "2.31.0",
        Some(json!([
            statement("CVE-SHARED", "not_affected", Some("component_not_present")),
            statement(
                "CVE-JUST-5",
                "fixed",
                Some("vulnerable_code_cannot_be_controlled_by_adversary")
            ),
        ])),
    );
    let output_path = fixture.path().join("fields.json");
    let result = document(&fixture.command(Some(&output_path), None), &output_path);
    let entries = result["vulnerabilities"].as_array().unwrap();
    assert_eq!(entries.len(), 5);
    let shared: Vec<_> = entries
        .iter()
        .filter(|item| item["id"] == "CVE-SHARED")
        .collect();
    assert_eq!(shared.len(), 2);
    for entry in &shared {
        assert_eq!(
            entry["analysis"]["detail"],
            "Impact statement authored by Ada."
        );
        assert_eq!(entry["recommendation"], "Upgrade to the fixed release.");
        assert_eq!(entry["properties"][0]["name"], "quoin:vex:source-status");
        assert_eq!(entry["properties"][2]["name"], "quoin:vex:assessed-by");
        assert_eq!(entry["properties"][2]["value"], "Ada Example");
        assert_eq!(entry["properties"][3]["name"], "quoin:vex:assessed-at");
        assert_eq!(entry["properties"][3]["value"], "2026-11-23");
        assert!(
            entry["properties"]
                .as_array()
                .unwrap()
                .iter()
                .any(|property| {
                    property["name"] == "quoin:vex:evidence-ref"
                        && property["value"] == "ix://agent-ix/quoin/EVIDENCE-1"
                })
        );
    }
    for (id, source, mapped) in [
        (
            "CVE-SHARED",
            "vulnerable_code_not_present",
            Some("code_not_present"),
        ),
        (
            "CVE-JUST-2",
            "vulnerable_code_not_in_execute_path",
            Some("code_not_reachable"),
        ),
        (
            "CVE-JUST-3",
            "inline_mitigations_already_exist",
            Some("protected_by_mitigating_control"),
        ),
        ("CVE-SHARED", "component_not_present", None),
        (
            "CVE-JUST-5",
            "vulnerable_code_cannot_be_controlled_by_adversary",
            None,
        ),
    ] {
        let entry = entries
            .iter()
            .find(|entry| {
                entry["id"] == id
                    && entry["properties"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|property| {
                            property["name"] == "quoin:vex:justification"
                                && property["value"] == source
                        })
            })
            .unwrap();
        assert_eq!(
            entry["analysis"]
                .get("justification")
                .and_then(Value::as_str),
            mapped
        );
        assert!(
            entry["properties"]
                .as_array()
                .unwrap()
                .iter()
                .any(|property| {
                    property["name"] == "quoin:vex:justification" && property["value"] == source
                })
        );
    }
}

#[trace("FR-156-AC-6", "NFR-030-AC-6")]
#[test]
fn scanner_advisories_do_not_create_vulnerabilities_without_authored_statements() {
    let fixture = Fixture::new();
    fixture.record("scanned", NPM_PURL, "12.3.1", None);
    let mut finding = Finding::new("CVE-2026-0001");
    finding.path = Some(NPM_PURL.to_owned());
    fixture.store_scan("osv-report", vec![finding], 1);
    let output_path = fixture.path().join("empty.json");
    let result = document(&fixture.command(Some(&output_path), None), &output_path);

    assert!(result.get("vulnerabilities").is_none());
    assert!(result.get("components").is_none());
}

#[trace("FR-156-AC-7")]
#[test]
fn only_a_current_nonvacuous_clean_applicable_scan_marks_a_statement_stale() {
    let fixture = Fixture::new();
    fixture.record(
        "stale",
        NPM_PURL,
        "12.3.1",
        Some(json!([statement("CVE-OLD", "affected", None)])),
    );
    fixture.store_scan("clean-osv", Vec::new(), 1);
    let output_path = fixture.path().join("stale.json");
    let result = document(&fixture.command(Some(&output_path), None), &output_path);
    let stale = result["vulnerabilities"][0]["properties"]
        .as_array()
        .unwrap()
        .iter()
        .any(|property| property["name"] == "quoin:vex:stale" && property["value"] == "true");
    assert!(stale);

    let missing = Fixture::new();
    missing.record(
        "missing",
        NPM_PURL,
        "12.3.1",
        Some(json!([statement("CVE-OLD", "affected", None)])),
    );
    let output_path = missing.path().join("missing.json");
    let result = document(&missing.command(Some(&output_path), None), &output_path);
    assert!(
        result["vulnerabilities"][0]["properties"]
            .as_array()
            .unwrap()
            .iter()
            .all(|property| property["name"] != "quoin:vex:stale")
    );

    let vacuous = Fixture::new();
    vacuous.record(
        "vacuous",
        NPM_PURL,
        "12.3.1",
        Some(json!([statement("CVE-OLD", "affected", None)])),
    );
    vacuous.store_scan("vacuous-osv", Vec::new(), 0);
    let output_path = vacuous.path().join("vacuous.json");
    let result = document(&vacuous.command(Some(&output_path), None), &output_path);
    assert!(
        result["vulnerabilities"][0]["properties"]
            .as_array()
            .unwrap()
            .iter()
            .all(|property| property["name"] != "quoin:vex:stale")
    );
}

#[trace("FR-156-AC-9")]
#[test]
fn identical_inputs_produce_byte_identical_output() {
    let fixture = Fixture::new();
    fixture.record(
        "deterministic",
        NPM_PURL,
        "12.3.1",
        Some(json!([statement("CVE-2026-0001", "affected", None)])),
    );
    let first_path = fixture.path().join("first.json");
    let second_path = fixture.path().join("second.json");
    assert!(fixture.command(Some(&first_path), None).status.success());
    assert!(fixture.command(Some(&second_path), None).status.success());
    assert_eq!(
        std::fs::read(first_path).unwrap(),
        std::fs::read(second_path).unwrap()
    );
}

#[trace("FR-156-AC-5", "FR-156-AC-10")]
#[test]
fn omitted_output_path_uses_the_sbom_directory_and_vex_suffix() {
    let fixture = Fixture::new();
    fixture.record(
        "default-path",
        NPM_PURL,
        "12.3.1",
        Some(json!([statement("CVE-2026-0001", "affected", None)])),
    );
    let sbom = fixture.path().join("probe.cdx.json");
    let result = fixture.command(None, Some(&sbom));
    assert!(result.status.success(), "{}", stderr(&result));
    let output = fixture.path().join("probe.vex.cdx.json");
    assert!(output.is_file());
    let parsed: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert_matches_cyclonedx_17_schema(&parsed);
}

#[trace("FR-156-AC-10")]
#[test]
fn cyclonedx_without_out_requires_an_sbom_path() {
    let fixture = Fixture::new();
    let output = fixture.command(None, None);
    assert!(!output.status.success());
    let diagnostic = stderr(&output);
    assert!(diagnostic.contains("--sbom is required"), "{diagnostic}");
}
