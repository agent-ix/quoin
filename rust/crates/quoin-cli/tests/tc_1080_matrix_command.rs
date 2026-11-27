// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin matrix` against a real repository and a real evidence store
//! (FR-115).
//!
//! The store is written by the shipped commands (`quoin evidence record`,
//! `quoin evidence inspect-mocks`), not by hand, so the audit `quoin matrix`
//! renders is the one the real assembly produces over real records.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

mod dropped_relation_support;

/// The module every fixture repository carries: an FR archetype whose
/// Acceptance Criteria table is an `obligations:` source.
const MODULE: &str = r##"name: quoin-cli-matrix-fixture
version: 0.1.0
description: Fixture module for the quoin matrix command tests
artifact_types:
  - name: FR
    grammar_ref: iso-spec-core
    defaults:
      id_pattern: FR-{next:03d}
traceability:
  trace_targets:
    - name: acceptance-criterion
      archetype: FR
      section: Acceptance Criteria
      id_column: ID
  obligations:
    - name: acceptance-criterion
      target: acceptance-criterion
      statement_column: Criteria
      method_column: Verification
  trace_tags:
    markers:
      - name: rust-trace-attribute
        language: rust
        pattern: '#\[trace\(([^)]*)\)\]'
        template: "#[trace({ids})]"
"##;

const DESIGN_MODULE: &str = r##"name: quoin-cli-design-matrix-fixture
version: 0.1.0
description: Fixture module for the quoin design matrix command tests
artifact_types:
  - name: FR
    grammar_ref: iso-spec-core
    defaults:
      id_pattern: FR-{next:03d}
  - name: Component
    defaults:
      id_pattern: CMP-{next:03d}
edge_types:
  implements: { description: a component implements an FR, category: traceability }
traceability:
  trace_targets:
    - name: acceptance-criterion
      archetype: FR
      section: Acceptance Criteria
      id_column: ID
  obligations:
    - name: acceptance-criterion
      target: acceptance-criterion
      statement_column: Criteria
      method_column: Verification
  required_relations:
    - name: fr-has-component
      from: FR
      edges: [implements]
      to: [Component]
      direction: outgoing
      check: implementation
  design_relations: [fr-has-component]
  trace_tags:
    markers:
      - name: rust-trace-attribute
        language: rust
        pattern: '#\[trace\(([^)]*)\)\]'
        template: "#[trace({ids})]"
"##;

const RECORD_KEYS_MODULE: &str = r##"name: quoin-cli-record-keys-fixture
version: 0.1.0
description: Fixture module for required-relation record keys
artifact_types:
  - name: FR
    grammar_ref: iso-spec-core
    defaults:
      id_pattern: FR-{next:03d}
  - name: Connection
  - name: Classification
    data_schema:
      schema: schemas/Classification.json
edge_types:
  classified-as: { description: a connection names its classification, category: traceability }
semantic:
  contract_version: 1.0.0
  package: agent-ix/quoin-record-keys-fixture
  exports: [Classification]
  targets: [json-schema, markdown]
traceability:
  trace_targets:
    - name: acceptance-criterion
      archetype: FR
      section: Acceptance Criteria
      id_column: ID
  obligations:
    - name: acceptance-criterion
      target: acceptance-criterion
      statement_column: Criteria
      method_column: Verification
  required_relations:
    - name: connection-carries-classification
      from: Connection
      edges: [classified-as]
      to: [Classification]
      direction: outgoing
      check: connection-carries-classification
      record_keys: [classification, privacy]
  trace_tags:
    markers:
      - name: rust-trace-attribute
        language: rust
        pattern: '#\[trace\(([^)]*)\)\]'
        template: "#[trace({ids})]"
"##;

/// Three criteria and no declared method, so the catalog cannot add an
/// `unknown-method` finding that would blur the three statuses under test.
const SPEC: &str = "---
id: FR-001
type: FR
title: Three criteria
---

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-001-AC-1 | Every stored payload shall round-trip through the reader unchanged. |  |
| FR-001-AC-2 | The reader shall refuse a payload over its ceiling. |  |
| FR-001-AC-3 | The reader shall name the refused field. |  |
";

const ROW_AWARE_MODULE: &str = r#"name: quoin-cli-matrix-claims
version: 0.1.0
description: Fixture module for unbacked matrix claims
artifact_types:
  - name: FR
  - name: TestMatrix
traceability:
  trace_targets:
    - name: acceptance-criterion
      archetype: FR
      section: Acceptance Criteria
      id_column: ID
    - name: test-case
      archetype: TestMatrix
      section: Test Cases
      id_column: ID
  document_references:
    - name: verification
      archetype: FR
      section: Acceptance Criteria
      row_id_column: ID
      column: Verification
      pattern: '\b(TC-\d+)\b'
      targets: [test-case]
    - name: traces-to
      archetype: TestMatrix
      section: Test Cases
      row_id_column: ID
      column: Traces To
      pattern: '\b((?:FR|TC)-\d+(?:-AC-\d+)?)\b'
      targets: [acceptance-criterion, test-case]
  status:
    column: Status
    complete: ["✅"]
    pending: ["🚧"]
  obligations:
    - name: acceptance-criterion
      target: acceptance-criterion
      statement_column: Criteria
      method_column: Verification
  trace_tags:
    markers:
      - name: rust-trace-attribute
        language: rust
        pattern: '#\[trace\(([^)]*)\)\]'
        template: '#[trace({ids})]'
"#;

const ROW_AWARE_SPEC: &str = "---\nid: FR-001\ntype: FR\n---\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n|---|---|---|\n| FR-001-AC-1 | The first behavior shall work. | Test (TC-001) |\n| FR-001-AC-2 | The second behavior shall work. | Test (TC-002) |\n";

const ROW_AWARE_TEST_MATRIX: &str = "---\nid: TM-001\ntype: TestMatrix\n---\n\n## Test Cases\n\n| ID | Traces To |\n|---|---|\n| TC-001 | FR-001-AC-1 |\n| TC-002 | FR-001-AC-2 |\n";

const ROW_AWARE_SOURCE: &str = "#[cfg(test)]\nmod tests {\n    #[trace(\"TC-001\", \"FR-001-AC-1\", \"FR-001-AC-2\")]\n    #[test]\n    fn tc001_first_behavior() {}\n\n    /// FR-001-AC-2\n    #[test]\n    fn tc002_comment_only_claim() {}\n}\n";

struct Fixture {
    dir: tempfile::TempDir,
    home: tempfile::TempDir,
}

impl Fixture {
    fn repo(&self) -> &Path {
        self.dir.path()
    }

    fn command(&self, executable: &str) -> Command {
        let mut command = Command::new(executable);
        command
            .current_dir(self.repo())
            .env("IX_HOME", self.home.path())
            .env("GIT_CEILING_DIRECTORIES", self.repo().parent().unwrap());
        command
    }

    fn quoin(&self, arguments: &[&str]) -> Output {
        self.command(env!("CARGO_BIN_EXE_quoin"))
            .args(arguments)
            .output()
            .expect("the native quoin binary runs")
    }

    fn ok(&self, arguments: &[&str]) -> String {
        let output = self.quoin(arguments);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.repo().join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }
}

/// A directory holding the module and `spec`, with no store.
fn repository(module: &str) -> Fixture {
    let fixture = Fixture {
        dir: tempfile::tempdir().unwrap(),
        home: tempfile::tempdir().unwrap(),
    };
    fixture.write("manifest.yaml", module);
    fixture.write("spec/FR-001-example.md", SPEC);
    fixture
}

/// A matrix module whose relation endpoint declares the two record keys used
/// by the record-key parser regression.
fn record_keys_repository(module: &str) -> Fixture {
    let fixture = repository(module);
    fixture.write(
        "schemas/Classification.json",
        r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"classification":{"type":"string"},"privacy":{"type":"string"}},"additionalProperties":false}"#,
    );
    fixture
}

fn install_defaults(home: &Path) {
    let installed = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["module", "ensure-defaults"])
        .env("IX_HOME", home)
        .output()
        .expect("the native quoin binary installs its published default modules");
    assert_eq!(
        installed.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&installed.stderr)
    );
}

fn install_module_package(home: &Path, package: &str) {
    let source = format!("package:{package}");
    let installed = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["module", "install", &source])
        .env("IX_HOME", home)
        .output()
        .expect("the native quoin binary installs the current published module");
    assert_eq!(
        installed.status.code(),
        Some(0),
        "{package}: {}",
        String::from_utf8_lossy(&installed.stderr)
    );
}

fn clone_current_architecture(repository: &Path) {
    let cloned = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "https://github.com/agent-ix/spec-objects-architecture.git",
        ])
        .arg(repository)
        .output()
        .expect("git is available to fetch the current public architecture repository");
    assert_eq!(
        cloned.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&cloned.stderr)
    );
}

fn assert_architecture_relation_rows(manifest: &Value) {
    let required_relations = manifest["traceability"]["required_relations"]
        .as_array()
        .expect("the architecture module declares required relations");
    let expected_relations = [
        (
            "connection-carries-classification",
            &["regulatedCategories"][..],
        ),
        (
            "classification-carried-by-connection",
            &["regulatedCategories"][..],
        ),
        ("connection-crosses-trust-boundary", &["trust_level"][..]),
        (
            "connection-delivers-to-sink",
            &["kind", "processorAgreement", "processor"][..],
        ),
        (
            "part-delivers-to-sink",
            &["kind", "processorAgreement", "processor"][..],
        ),
    ];
    for (name, keys) in expected_relations {
        assert!(
            required_relations.iter().any(|relation| {
                relation["name"] == name
                    && relation["record_keys"]
                        == serde_json::Value::Array(
                            keys.iter().map(|key| serde_json::json!(key)).collect(),
                        )
            }),
            "the current architecture module retains required relation {name} with its record_keys"
        );
    }
}

fn assert_fr015_matrix_bindings(matrix: &Value) {
    let requirements = matrix["requirements"].as_array().unwrap();
    let fr_015 = requirements
        .iter()
        .find(|requirement| requirement["id"] == "FR-015")
        .expect("the current architecture matrix preserves FR-015");
    let criteria = fr_015["criteria"].as_array().unwrap();
    let criterion_ids: Vec<&str> = criteria
        .iter()
        .map(|criterion| criterion["id"].as_str().unwrap())
        .collect();
    assert!(criterion_ids.contains(&"FR-015-AC-4"), "{criterion_ids:?}");
    assert!(criterion_ids.contains(&"FR-015-AC-5"), "{criterion_ids:?}");
    let ac_4 = criteria
        .iter()
        .find(|criterion| criterion["id"] == "FR-015-AC-4")
        .unwrap();
    let ac_4_binders = ac_4["binders"].as_array().unwrap();
    assert!(
        ac_4_binders.iter().any(|binder| {
            binder["path"] == "tests/test_data_sink.py"
                && binder["qualified_name"]
                    == "test_delivers_to_and_carries_validate_and_refuse_wrong_or_dangling_targets"
        }),
        "FR-015-AC-4 binders: {ac_4_binders:?}"
    );
    let ac_5 = criteria
        .iter()
        .find(|criterion| criterion["id"] == "FR-015-AC-5")
        .unwrap();
    let ac_5_binders = ac_5["binders"].as_array().unwrap();
    for expected in [
        "test_delivers_to_and_carries_validate_and_refuse_wrong_or_dangling_targets",
        "test_data_sink_is_exported_and_declares_only_the_supported_links",
    ] {
        assert!(
            ac_5_binders
                .iter()
                .any(|binder| binder["qualified_name"] == expected),
            "FR-015-AC-5 binders: {ac_5_binders:?}"
        );
    }
}

/// One passing `unit` run for AC-1 with its mock inspection, and one `legacy`
/// binding for AC-2 whose run record is gone. AC-3 is bound nowhere.
fn populated() -> Fixture {
    let fixture = repository(MODULE);
    // Outside the repository, so the results file is not part of the tree.
    let junit = |suite: &str, id: &str| {
        let path = fixture.home.path().join(format!("{suite}.xml"));
        std::fs::write(
            &path,
            format!(
                r#"<testsuite name="{suite}"><testcase name="{suite}_case"><properties><property name="trace" value="{id}"/></properties></testcase></testsuite>"#
            ),
        )
        .unwrap();
        path
    };
    for (suite, id) in [("unit", "FR-001-AC-1"), ("legacy", "FR-001-AC-2")] {
        let results = junit(suite, id);
        fixture.ok(&[
            "evidence",
            "record",
            "--repo",
            ".",
            "--module",
            ".",
            "--suite",
            suite,
            "--tool",
            "fixture",
            "--adapter",
            "junit",
            "--results",
            results.to_str().unwrap(),
            "--timestamp",
            "2026-01-01T00:00:00Z",
        ]);
    }
    fixture.ok(&[
        "evidence",
        "inspect-mocks",
        "--repo",
        ".",
        "--suite",
        "unit",
        "--timestamp",
        "2026-01-01T00:00:00Z",
    ]);
    // The binding stays; the run it names does not.
    std::fs::remove_dir_all(fixture.repo().join("spec/evidence/runs/legacy")).unwrap();
    fixture
}

const DERIVED_ASSURANCE: [&str; 9] = [
    "assurance",
    "--repo",
    ".",
    "--module",
    ".",
    "--claim-type",
    "FR",
    "--as-of",
    "2026-01-01T00:00:00Z",
];

#[trace("FR-234-AC-3", "FR-228-AC-8", "FR-040-AC-21")]
#[test]
fn sacm_derived_command_exports_the_retained_run_selected_by_gsn() {
    let fixture = populated();
    let run_path = "spec/evidence/runs/unit/2026-01-01T00~3A00~3A00Z.json";
    let run = std::fs::read(fixture.repo().join(run_path)).expect("retained run record");
    let digest = quoin_store::attachment::sha256_hex(&run);

    let markdown = fixture.ok(&DERIVED_ASSURANCE);
    assert!(markdown.starts_with("# Assurance case\n"), "{markdown}");
    assert!(
        markdown.contains("```mermaid\nflowchart TD\n"),
        "{markdown}"
    );
    assert!(!markdown.contains("quoin.gsn-case/v1"), "{markdown}");
    assert!(
        !markdown.contains("sacm:AssuranceCasePackage"),
        "{markdown}"
    );

    let mut legacy_json_args = DERIVED_ASSURANCE.to_vec();
    legacy_json_args.push("--json");
    let legacy_json: Value = serde_json::from_str(&fixture.ok(&legacy_json_args))
        .expect("the existing JSON mode remains case JSON");
    assert!(legacy_json["claims"].as_array().is_some(), "{legacy_json}");
    assert!(!legacy_json.get("format").is_some(), "{legacy_json}");

    let mut args = DERIVED_ASSURANCE.to_vec();
    args.push("--gsn");
    let gsn: Value = serde_json::from_str(&fixture.ok(&args)).expect("validated GSN");
    let nodes = gsn["nodes"].as_array().expect("GSN nodes");
    let healthy = nodes
        .iter()
        .find(|node| node["source"] == "FR-001-AC-1")
        .expect("healthy solution");
    let open = nodes
        .iter()
        .find(|node| node["source"] == "FR-001-AC-2")
        .expect("missing run obligation");
    assert_eq!(healthy["kind"], "solution");
    assert_eq!(
        healthy["evidence"]["suites"],
        serde_json::json!([{"suite":"unit","at":"2026-01-01T00:00:00Z"}])
    );
    assert_eq!(open["kind"], "goal");
    assert_eq!(open["status"], "open");
    assert_eq!(open["undeveloped"], true);
    assert_eq!(
        open["evidence"]["suites"],
        serde_json::json!([{"suite":"legacy"}])
    );
    assert_eq!(open["evidence"]["verdict"], "stale");
    assert!(
        open["openReason"]
            .as_str()
            .unwrap()
            .contains("no recorded run"),
        "{open}"
    );
    let open_id = open["id"].as_str().expect("open goal id");
    assert!(
        gsn["edges"]
            .as_array()
            .unwrap()
            .iter()
            .all(|edge| edge["from"] != open_id || edge["kind"] != "supported_by")
    );

    args.pop();
    args.push("--sacm");
    let xmi = fixture.ok(&args);
    assert!(xmi.contains(&format!("gid=\"sha256:{digest}\"")), "{xmi}");
    assert!(
        xmi.contains(&format!("referencedArtifactElement=\"artifact-{digest}\"")),
        "{xmi}"
    );
    assert!(
        xmi.contains(&format!("xmi:id=\"artifact-{digest}\"")),
        "{xmi}"
    );
    assert!(xmi.contains(&format!("content=\"{run_path}\"")), "{xmi}");
    assert!(xmi.contains("content=\"unit\""), "{xmi}");
    assert!(xmi.contains("version=\"2026-01-01T00:00:00Z\""), "{xmi}");
    assert!(xmi.contains("content=\"verdict: bound\""), "{xmi}");
    assert!(xmi.contains("content=\"FR-001-AC-2\""), "{xmi}");
    assert!(!xmi.contains("spec/evidence/runs/legacy/"), "{xmi}");
    let healthy_id = healthy["id"].as_str().expect("healthy solution id");
    assert!(
        xmi.lines()
            .any(|element| element.contains("sacm:AssertedEvidence")
                && element.contains(&format!("source=\"{healthy_id}\""))
                && element.contains("assertionDeclaration=\"asserted\"")),
        "{xmi}"
    );
    assert!(
        xmi.lines().any(|element| element.contains("sacm:Claim")
            && element.contains(&format!("xmi:id=\"{open_id}\""))
            && element.contains("assertionDeclaration=\"needsSupport\"")),
        "{xmi}"
    );
    let open_ref = format!("{open_id}-evidence");
    assert!(
        xmi.lines()
            .any(|element| element.contains("sacm:ArtifactReference")
                && element.contains(&format!("xmi:id=\"{open_ref}\""))
                && !element.contains("referencedArtifactElement")),
        "{xmi}"
    );
    assert!(
        xmi.lines()
            .any(|element| element.contains("sacm:AssertedEvidence")
                && element.contains(&format!("source=\"{open_ref}\""))
                && element.contains(&format!("target=\"{open_id}\""))
                && element.contains("isCounter=\"false\"")
                && element.contains("assertionDeclaration=\"needsSupport\"")),
        "{xmi}"
    );
    assert!(
        xmi.contains("not supporting: stale, missing run for legacy (no observation time)"),
        "{xmi}"
    );
}

#[cfg(target_os = "linux")]
#[trace("NFR-045-M-1")]
#[test]
#[ignore = "Linux offline assurance lane requires strace with network syscall fault injection"]
fn sacm_and_gsn_derived_cli_make_no_network_syscalls() {
    let fixture = populated();
    let trace_path = fixture.home.path().join("network.trace");
    for flag in ["--gsn", "--sacm"] {
        let mut command = fixture.command("strace");
        command
            .args(["-f", "-qq", "-e", "trace=%network", "-e", "signal=none"])
            .args(["-e", "inject=%network:error=ENETDOWN", "-o"])
            .arg(&trace_path)
            .arg("--")
            .arg(env!("CARGO_BIN_EXE_quoin"))
            .args(DERIVED_ASSURANCE)
            .arg(flag);
        if flag == "--sacm" {
            command.args(["--sacm-report", "sacm-report.json"]);
        }
        let output = command.output().expect("strace runs the native exporter");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{flag}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let trace = std::fs::read_to_string(&trace_path).expect("strace creates its syscall log");
        assert!(
            trace.is_empty(),
            "{flag} attempted a network syscall:\n{trace}"
        );
        let rendered = String::from_utf8(output.stdout).expect("UTF-8 export");
        assert!(rendered.contains("FR-001-AC-1"), "{flag}: {rendered}");
        let format = if flag == "--gsn" {
            "quoin.gsn-case/v1"
        } else {
            "sacm:AssuranceCasePackage"
        };
        assert!(rendered.contains(format), "{flag}: {rendered}");
    }
    let report = std::fs::read(fixture.repo().join("sacm-report.json"))
        .expect("SACM report retained offline");
    let report: Value = serde_json::from_slice(&report).expect("JSON report");
    assert!(report["unmapped"].is_array() && report["unreachable"].is_array());
}

fn statuses(matrix: &Value) -> BTreeMap<String, String> {
    matrix["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|requirement| requirement["criteria"].as_array().unwrap())
        .map(|criterion| {
            (
                criterion["id"].as_str().unwrap().to_owned(),
                criterion["evidence_status"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// A passing binding with a mock inspection, a binding whose run is missing
/// and no binding at all read as three distinct statuses in one matrix, and
/// agree with what `quoin evidence audit` reports for the same tree.
#[trace("FR-115-AC-6", "FR-115-AC-14")]
#[test]
fn tc_1080_200_a_real_store_yields_bound_stale_and_undischarged() {
    let fixture = populated();
    let matrix: Value =
        serde_json::from_str(&fixture.ok(&["matrix", "--module", ".", "--json"])).unwrap();
    assert_eq!(
        statuses(&matrix),
        BTreeMap::from([
            ("FR-001-AC-1".to_owned(), "bound".to_owned()),
            ("FR-001-AC-2".to_owned(), "stale".to_owned()),
            ("FR-001-AC-3".to_owned(), "undischarged".to_owned()),
        ])
    );

    // The same assembly: the audit command's own healthy set and findings
    // are exactly what the matrix read.
    let audit: Value =
        serde_json::from_str(&fixture.ok(&["evidence", "audit", "--module", ".", "--json"]))
            .unwrap();
    assert_eq!(audit["healthy"], serde_json::json!(["FR-001-AC-1"]));
    let mut from_audit: Vec<(String, String)> = audit["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| {
            (
                finding["obligation"].as_str().unwrap().to_owned(),
                finding["summary"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    from_audit.sort();
    let mut from_matrix: Vec<(String, String)> = matrix["requirements"][0]["criteria"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|criterion| {
            criterion["evidence_detail"]["findings"]
                .as_array()
                .unwrap()
                .iter()
                .map(|finding| {
                    (
                        criterion["id"].as_str().unwrap().to_owned(),
                        finding["summary"].as_str().unwrap().to_owned(),
                    )
                })
        })
        .collect();
    from_matrix.sort();
    assert_eq!(from_matrix, from_audit);
    assert!(
        from_audit
            .iter()
            .any(|(id, summary)| id == "FR-001-AC-2" && summary.contains("no recorded run")),
        "the stale status must come from the auditor's missing-run check: {from_audit:?}"
    );
}

/// Existing evidence-audit and assurance projections stay byte-identical
/// when the repository gains a retained change-assurance record.
#[trace("FR-065-CON-4")]
#[test]
fn change_assurance_storage_does_not_change_existing_audit_or_assurance_output() {
    let fixture = populated();
    let audit_args = [
        "evidence", "audit", "--repo", ".", "--module", ".", "--json",
    ];
    let before_audit = fixture.ok(&audit_args);
    let mut assurance_args = DERIVED_ASSURANCE.to_vec();
    assurance_args.push("--json");
    let before_assurance = fixture.ok(&assurance_args);

    let record = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/change-assurance-cli/record-body.json")
        .canonicalize()
        .unwrap();
    let write = fixture.quoin(&[
        "change-assurance",
        "write-record",
        "--input",
        record.to_str().unwrap(),
        "--repo",
        ".",
        "--json",
    ]);
    assert_eq!(
        write.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&write.stderr)
    );

    assert_eq!(fixture.ok(&audit_args), before_audit);
    assert_eq!(fixture.ok(&assurance_args), before_assurance);
}

/// The retained runs reach `matrix.build` through the host: the run identity of
/// a bound criterion is read from the real store, bounded to what a binding
/// names, and a criterion whose run is gone has none. The source revision is the
/// checkout SHA an observation file gives for that record (FR-116).
#[trace("FR-196-AC-9", "FR-196-AC-10", "FR-196-AC-11")]
#[test]
fn tc_1080_211_the_matrix_carries_the_run_identity_the_store_retains() {
    let fixture = populated();
    let bindings_of = |matrix: &Value, id: &str| -> Value {
        matrix["requirements"][0]["criteria"]
            .as_array()
            .unwrap()
            .iter()
            .find(|criterion| criterion["id"] == id)
            .unwrap()["evidence_detail"]["bindings"]
            .clone()
    };
    let matrix: Value =
        serde_json::from_str(&fixture.ok(&["matrix", "--module", ".", "--json"])).unwrap();
    let bound = bindings_of(&matrix, "FR-001-AC-1");
    let run = &bound[0]["run"];
    assert_eq!(bound[0]["suite"], "unit");
    assert_eq!(run["tool"], "fixture");
    assert_eq!(run["timestamp"], "2026-01-01T00:00:00Z");
    assert_eq!(
        run["outcomes"],
        serde_json::json!({ "pass": 1, "fail": 0, "skip": 0, "error": 0 })
    );
    assert!(
        run.get("source_revision").is_none_or(Value::is_null),
        "{run}"
    );
    let file = run["file"].as_str().unwrap().to_owned();
    assert!(file.starts_with("runs/unit/"), "{file}");
    // The run of the `legacy` binding is gone: no identity is invented.
    let missing = bindings_of(&matrix, "FR-001-AC-2");
    assert!(
        missing[0].get("run").is_none_or(Value::is_null),
        "{missing}"
    );

    let observation = fixture.home.path().join("observation.json");
    std::fs::write(
        &observation,
        serde_json::json!({
            "protocol": "quoin.analysis-run-observation/v1",
            "record": file,
            "checkout_sha": "9f2c1ab9f2c1ab9f2c1ab9f2c1ab9f2c1ab9f2c1",
        })
        .to_string(),
    )
    .unwrap();
    let observed: Value = serde_json::from_str(&fixture.ok(&[
        "matrix",
        "--module",
        ".",
        "--json",
        "--run-observation",
        observation.to_str().unwrap(),
    ]))
    .unwrap();
    assert_eq!(
        bindings_of(&observed, "FR-001-AC-1")[0]["run"]["source_revision"],
        "9f2c1ab9f2c1ab9f2c1ab9f2c1ab9f2c1ab9f2c1"
    );
}

/// Markdown by default, canonical JSON under `--json`, both byte-identical
/// across two runs.
#[trace("FR-115-AC-11", "FR-235-AC-5")]
#[test]
fn tc_1080_201_renders_markdown_and_canonical_json_deterministically() {
    let fixture = populated();
    let markdown = fixture.ok(&["matrix", "--module", "."]);
    assert_eq!(markdown, fixture.ok(&["matrix", "--module", "."]));
    assert!(
        markdown.starts_with("# Test Matrix\n\n## FR-001\n\n"),
        "{markdown}"
    );
    assert!(markdown.contains(
        "| Requirement | Criterion | Method | Static Status | Evidence Status | Detail |\n"
    ));
    assert!(markdown.contains("| FR-001 | FR-001-AC-3 | — | untagged | undischarged | "));

    let json = fixture.ok(&["matrix", "--module", ".", "--json"]);
    assert_eq!(json, fixture.ok(&["matrix", "--module", ".", "--json"]));
    let parsed: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        json,
        format!("{}\n", serde_json::to_string(&parsed).unwrap()),
        "--json is one line of canonical JSON"
    );
}

/// Formalization records are consumed by assurance and leave the real matrix
/// result byte-identical (VER-105).
#[trace("FR-205-AC-5")]
#[test]
fn formalization_assurance_read_does_not_change_matrix_coverage() {
    let fixture = repository(MODULE);
    let git = Command::new("git")
        .args(["init", "-q"])
        .current_dir(fixture.repo())
        .output()
        .expect("git initializes fixture repository");
    assert!(git.status.success());
    for args in [
        &["config", "user.name", "Fixture"][..],
        &["config", "user.email", "fixture@example.invalid"][..],
        &["add", "."][..],
        &["commit", "-qm", "fixture"][..],
    ] {
        let output = Command::new("git")
            .args(args)
            .current_dir(fixture.repo())
            .output()
            .expect("git fixture operation runs");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let before: Value = serde_json::from_str(&fixture.ok(&["matrix", "--module", ".", "--json"]))
        .expect("matrix is JSON");
    let records = fixture.write(
        "formalization-records.json",
        r#"[
          {"criterion_id":{"obligation_declaration":"acceptance-criterion","scoped_document":"spec/FR-001-example.md","criterion_id":"FR-001-AC-1"},"state":"eligible"},
          {"criterion_id":{"obligation_declaration":"acceptance-criterion","scoped_document":"spec/FR-001-example.md","criterion_id":"FR-001-AC-2"},"state":"eligible"},
          {"criterion_id":{"obligation_declaration":"acceptance-criterion","scoped_document":"spec/FR-001-example.md","criterion_id":"FR-001-AC-3"},"state":"eligible"}
        ]"#,
    );
    let output = fixture.quoin(&[
        "assurance",
        "--repo",
        ".",
        "--module",
        ".",
        "--formalization-records",
        records.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let after: Value = serde_json::from_str(&fixture.ok(&["matrix", "--module", ".", "--json"]))
        .expect("matrix remains JSON");
    assert_eq!(before["requirements"], after["requirements"]);
}

/// The CLI JSON render forwards Quire's unbacked claim diagnostics without
/// changing their static status or binding list.
#[trace("FR-115-AC-1", "FR-115-AC-11")]
#[test]
fn tc_1080_207_json_forwards_unbacked_claims() {
    let fixture = repository(ROW_AWARE_MODULE);
    fixture.write("spec/FR-001-example.md", ROW_AWARE_SPEC);
    fixture.write("spec/TestMatrix.md", ROW_AWARE_TEST_MATRIX);
    fixture.write("tests/matrix_claims.rs", ROW_AWARE_SOURCE);

    let matrix: Value =
        serde_json::from_str(&fixture.ok(&["matrix", "--module", ".", "--json"])).unwrap();
    let criterion = &matrix["requirements"][0]["criteria"][1];
    assert_eq!(criterion["id"], "FR-001-AC-2");
    assert_eq!(criterion["static_status"], "tagged");
    assert_eq!(
        criterion["binders"],
        serde_json::json!([{
            "column": 5,
            "kind": "test_function",
            "line": 5,
            "path": "tests/matrix_claims.rs",
            "qualified_name": "tests::tc001_first_behavior"
        }])
    );
    assert_eq!(
        criterion["unbacked_claims"],
        serde_json::json!([
            {
                "reason": "unmarked-trace-id",
                "id": "FR-001-AC-2",
                "path": "tests/matrix_claims.rs",
                "line": 7,
                "symbol": "tests::tc002_comment_only_claim"
            }
        ])
    );
}

/// A module declaring no `obligations:` source renders the reason and exits 0.
#[trace("FR-115-AC-11")]
#[test]
fn tc_1080_202_no_obligations_source_renders_the_reason() {
    let without = MODULE.replace(
        "  obligations:\n    - name: acceptance-criterion\n      target: acceptance-criterion\n      statement_column: Criteria\n      method_column: Verification\n",
        "",
    );
    assert_ne!(without, MODULE, "the fixture edit must remove the source");
    let fixture = repository(&without);
    let markdown = fixture.ok(&["matrix", "--module", "."]);
    assert_eq!(
        markdown,
        format!("# Test Matrix\n\n{}\n", quoin_assurance_reason())
    );
    let json: Value =
        serde_json::from_str(&fixture.ok(&["matrix", "--module", ".", "--json"])).unwrap();
    assert_eq!(json["requirements"], serde_json::json!([]));
    assert_eq!(json["reason"], quoin_assurance_reason());
}

/// The reason text, spelled out: this test asserts the literal rather than
/// importing the constant it is checking.
fn quoin_assurance_reason() -> &'static str {
    "no coverage_matrix criteria: the module declares no `obligations:` source, so there is no criterion to report evidence for"
}

/// Every file under `spec/`, by relative path, with its bytes.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, &root.join("spec"), &mut out);
    out
}

/// `quoin matrix` leaves `spec/` byte-for-byte as it found it.
#[trace("FR-115-AC-12", "FR-115-CON-3")]
#[test]
fn tc_1080_203_writes_nothing_under_spec() {
    let fixture = populated();
    let before = snapshot(fixture.repo());
    assert!(
        before.keys().any(|path| path.starts_with("spec/evidence")),
        "the snapshot must include the store, or it proves nothing about it"
    );
    fixture.ok(&["matrix", "--module", "."]);
    fixture.ok(&["matrix", "--module", ".", "--json"]);
    assert_eq!(snapshot(fixture.repo()), before);
}

/// A declared design relation is computed without modifying any spec bytes.
///
/// Provenance: PLAT-1590
#[trace("FR-165-AC-10")]
#[test]
fn tc_1080_206_design_matrix_leaves_spec_byte_identical() {
    let fixture = repository(DESIGN_MODULE);
    let before = snapshot(fixture.repo());
    let markdown = fixture.ok(&["matrix", "--module", "."]);
    assert!(
        markdown.contains("| Requirement | Criterion | Design | Method |"),
        "{markdown}"
    );
    assert!(
        markdown.contains("Design gap: no design element (implementation)"),
        "{markdown}"
    );
    assert_eq!(snapshot(fixture.repo()), before);
}

/// An undeclared design relation exits 2 with its name and emits no matrix.
///
/// Provenance: PLAT-1590
#[trace("FR-165-AC-17")]
#[test]
fn tc_1080_207_undeclared_design_relation_does_not_fall_back() {
    let invalid = DESIGN_MODULE.replace(
        "design_relations: [fr-has-component]",
        "design_relations: [unlisted]",
    );
    let fixture = repository(&invalid);
    let output = fixture.quoin(&["matrix", "--module", "."]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unlisted"), "{stderr}");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("# Test Matrix"));
}

/// Quoin matrix has no strict gate of its own.
///
/// Provenance: PLAT-1590
#[trace("FR-165-AC-12")]
#[test]
fn tc_1080_208_matrix_has_no_strict_flag() {
    let fixture = repository(MODULE);
    let output = fixture.quoin(&["matrix", "--module", ".", "--strict"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unexpected argument"));
}

/// `quoin matrix` never launches `ix-flow`: an `ix-flow` on PATH that would
/// leave a marker is never run.
#[trace("FR-115-AC-13")]
#[test]
fn tc_1080_205_never_spawns_ix_flow() {
    let fixture = populated();
    let bin = tempfile::tempdir().unwrap();
    let marker = bin.path().join("ran");
    let script = bin.path().join("ix-flow");
    std::fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = std::env::join_paths(
        std::iter::once(bin.path().to_path_buf())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["matrix", "--module", "."])
        .current_dir(fixture.repo())
        .env("IX_HOME", fixture.home.path())
        .env("PATH", path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(!marker.exists(), "quoin matrix ran ix-flow");
}

/// Help describes the computed view, not the retired workflow.
#[trace("FR-115-AC-15")]
#[test]
fn tc_1080_206_help_describes_the_evidence_backed_render() {
    let fixture = repository(MODULE);
    let page = fixture.ok(&["matrix", "--help"]);
    assert!(
        page.starts_with("Render the evidence-backed test matrix, per criterion.\n"),
        "{page}"
    );
    assert!(!page.contains("Build or update a requirements test matrix"));
    assert!(!page.contains("ix-flow"));
    assert!(!page.contains("[--strict]"), "the view adds no gate");
    assert!(
        page.contains("--class-policy <FILE>")
            && page.contains("floors by stated software safety class"),
        "help explains the class policy input: {page}"
    );
    let root = fixture.ok(&["--help"]);
    assert!(
        root.contains(
            "  matrix            Render the evidence-backed test matrix, per criterion.\n"
        )
    );
    assert!(!root.contains("Build or update a requirements test matrix"));
}

/// The matrix loads required-relation record keys through the released Quire
/// engine, including its relation-specific malformed-value diagnostics.
#[trace(
    "FR-206-AC-1",
    "FR-206-AC-2",
    "FR-206-AC-3",
    "FR-206-AC-4",
    "FR-165-AC-21"
)]
#[test]
fn tc_1080_209_matrix_loads_required_relation_record_keys() {
    let valid = RECORD_KEYS_MODULE.to_owned();
    let fixture = record_keys_repository(&valid);
    let output: Value =
        serde_json::from_str(&fixture.ok(&["matrix", "--module", ".", "--json"])).unwrap();
    assert!(output["requirements"].is_array());

    let omitted_module =
        RECORD_KEYS_MODULE.replace("      record_keys: [classification, privacy]\n", "");
    let omitted = record_keys_repository(&omitted_module);
    let output: Value =
        serde_json::from_str(&omitted.ok(&["matrix", "--module", ".", "--json"])).unwrap();
    assert!(output["requirements"].is_array());

    for invalid in [
        "null",
        "[]",
        "scalar",
        "[classification, 7]",
        "[classification, ' ']",
        "[classification, classification]",
    ] {
        let module = RECORD_KEYS_MODULE.replace(
            "      record_keys: [classification, privacy]\n",
            &format!("      record_keys: {invalid}\n"),
        );
        let fixture = record_keys_repository(&module);
        let output = fixture.quoin(&["matrix", "--module", ".", "--json"]);
        assert_eq!(output.status.code(), Some(2), "{invalid}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("connection-carries-classification"),
            "{invalid}: {stderr}"
        );
        assert!(!String::from_utf8_lossy(&output.stdout).contains("requirements"));
    }

    let undeclared_relation_key = RECORD_KEYS_MODULE.replace(
        "      record_keys: [classification, privacy]\n",
        "      record_keys: [notDeclared]\n",
    );
    let fixture = record_keys_repository(&undeclared_relation_key);
    let output = fixture.quoin(&["matrix", "--module", ".", "--json"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "an undeclared endpoint key remains a partial-matrix condition: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let matrix: Value = serde_json::from_slice(&output.stdout).expect("partial matrix is JSON");
    assert!(
        matrix["requirements"].is_array(),
        "valid rows survive: {matrix}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("connection-carries-classification") && stderr.contains("notDeclared"),
        "the dropped relation and its missing key are reported: {stderr}"
    );

    let conflicting_relation = RECORD_KEYS_MODULE.replace(
        "      record_keys: [classification, privacy]\n",
        "      record_keys: [classification, privacy]\n    - name: connection-carries-classification\n      from: Connection\n      edges: [classified-as]\n      to: [Classification]\n      direction: outgoing\n      check: conflicting-check\n",
    );
    let fixture = record_keys_repository(&conflicting_relation);
    let output = fixture.quoin(&["matrix", "--module", ".", "--json"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "conflicting required relations remain fatal: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("connection-carries-classification"),
        "the conflict names its relation: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let unknown = RECORD_KEYS_MODULE.replace(
        "      record_keys: [classification, privacy]\n",
        "      record_keys: [classification, privacy]\n      undeclared_key: true\n",
    );
    let fixture = repository(&unknown);
    let output = fixture.quoin(&["matrix", "--module", ".", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("undeclared_key"));
}

/// The production matrix loads the released architecture module and the current
/// architecture repository using only the clean Quoin defaults. This covers
/// the real relation declaration whose `record_keys` field previously made
/// Quoin refuse the module (FR-206-AC-5).
#[trace("FR-206-AC-5")]
#[test]
fn tc_1080_210_matrix_computes_current_architecture_with_record_keys() {
    let scratch = tempfile::tempdir().unwrap();
    let home = scratch.path().join("ix-home");
    let repository = scratch.path().join("spec-objects-architecture");
    install_defaults(&home);
    install_module_package(&home, "@agent-ix/spec-objects-architecture");
    install_module_package(&home, "@agent-ix/spec-objects-security");
    clone_current_architecture(&repository);
    let manifest_path = home.join("filament/modules/spec-objects-architecture/manifest.yaml");
    let manifest: Value = quoin_yaml::from_str(&std::fs::read_to_string(&manifest_path).unwrap())
        .expect("the current installed architecture module manifest parses");
    assert_architecture_relation_rows(&manifest);
    let output = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["matrix", "--repo"])
        .arg(&repository)
        .arg("--json")
        .current_dir(&repository)
        .env("IX_HOME", &home)
        .output()
        .expect("the native quoin binary runs against the architecture checkout");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("CORE_REFUSED"), "{stderr}");
    assert!(!stderr.contains("QQ-1020"), "{stderr}");
    let matrix: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_fr015_matrix_bindings(&matrix);
}

/// The real matrix host keeps its valid sibling row and reports a dropped
/// required relation as a diagnostic instead of refusing the whole run.
#[trace("FR-165-AC-21")]
#[test]
fn tc_1080_211_matrix_keeps_valid_rows_when_a_required_relation_drops() {
    let fixture = Fixture {
        dir: tempfile::tempdir().unwrap(),
        home: tempfile::tempdir().unwrap(),
    };
    fixture.write("manifest.yaml", dropped_relation_support::MODULE);
    fixture.write("spec/FR-001-requirement.md", dropped_relation_support::SPEC);
    let output = fixture.quoin(&[
        "matrix",
        "--repo",
        ".",
        "--module",
        ".",
        "--no-project-config",
        "--json",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let matrix: Value = serde_json::from_slice(&output.stdout).expect("matrix is JSON");
    assert!(
        matrix["requirements"]
            .as_array()
            .is_some_and(|requirements| requirements.iter().any(|requirement| {
                requirement["criteria"].as_array().is_some_and(|criteria| {
                    criteria
                        .iter()
                        .any(|criterion| criterion["id"] == "FR-001-AC-1")
                })
            })),
        "the valid requirement row survives: {matrix}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("connection-carries-classification"),
        "the dropped relation is reported beside valid rows: {stderr}"
    );

    let non_relation_drop_module = dropped_relation_support::MODULE.replace(
        "      method_column: Verification\n  required_relations:",
        "      method_column: Verification\n    - name: dropped-obligation\n      target: absent-target\n      statement_column: Criteria\n      method_column: Verification\n  required_relations:",
    );
    assert!(non_relation_drop_module.contains("dropped-obligation"));
    let strict_fixture = Fixture {
        dir: tempfile::tempdir().unwrap(),
        home: tempfile::tempdir().unwrap(),
    };
    strict_fixture.write("manifest.yaml", &non_relation_drop_module);
    strict_fixture.write("spec/FR-001-requirement.md", dropped_relation_support::SPEC);
    let refused = strict_fixture.quoin(&[
        "matrix",
        "--repo",
        ".",
        "--module",
        ".",
        "--no-project-config",
        "--json",
    ]);
    assert_eq!(
        refused.status.code(),
        Some(2),
        "a non-relation drop stays fatal: {}",
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("dropped-obligation"),
        "the fatal drop is named: {}",
        String::from_utf8_lossy(&refused.stderr)
    );
}
