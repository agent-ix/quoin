// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! `quoin soup vex export` writes only person-authored OpenVEX statements
//! (FR-216).

use std::path::Path;
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const PURL_A: &str = "pkg:cargo/serde@1.0.228";
const PURL_B: &str = "pkg:npm/%40scope/left-pad@1.3.0";

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("spec/soup")).unwrap();
        Self { root }
    }

    fn path(&self) -> &Path {
        self.root.path()
    }

    fn record(&self, id: &str, purl: &str, statements: Option<Value>) {
        let mut front =
            format!("---\nid: {id}\ntitle: {id}\ntype: soup_record\npurl: \"{purl}\"\n");
        if let Some(statements) = statements {
            front.push_str("vex_statements: ");
            front.push_str(&serde_json::to_string(&statements).unwrap());
            front.push('\n');
        }
        front.push_str("---\n# Record\n");
        std::fs::write(self.path().join(format!("spec/soup/{id}.md")), front).unwrap();
    }

    fn command(&self, output: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_quoin"))
            .args(["soup", "vex", "export"])
            .arg("--repo")
            .arg(self.path())
            .arg("--out")
            .arg(output)
            .args([
                "--id",
                "urn:example:vex:document-1",
                "--author",
                "security@example.test",
                "--timestamp",
                "2026-11-24T10:00:00Z",
                "--version",
                "1",
            ])
            .output()
            .unwrap()
    }
}

fn statement(advisory: &str, status: &str) -> Value {
    match status {
        "affected" => json!({
            "advisory": advisory,
            "status": status,
            "assessed_by": "Ada Example",
            "assessed_at": "2026-11-23",
            "action_statement": "Upgrade to the fixed release."
        }),
        "not_affected" => json!({
            "advisory": advisory,
            "status": status,
            "assessed_by": "Ada Example",
            "assessed_at": "2026-11-23",
            "justification": "component_not_present"
        }),
        _ => json!({
            "advisory": advisory,
            "status": status,
            "assessed_by": "Ada Example",
            "assessed_at": "2026-11-23"
        }),
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<_> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

fn assert_matches_openvex_020_schema(document: &Value) {
    let schema_path = std::env::var_os("OPENVEX_SCHEMA_PATH")
        .expect("OPENVEX_SCHEMA_PATH must name the fetched OpenVEX 0.2.0 schema");
    let schema_path = Path::new(&schema_path);
    let schema: Value = serde_json::from_slice(&std::fs::read(schema_path).unwrap())
        .expect("the fetched OpenVEX schema must be valid JSON");
    let validator =
        quoin_jsonschema::validator::SchemaValidator::compile_vendored(schema_path, &schema, &[])
            .expect("the fetched OpenVEX 0.2.0 schema must compile");
    let errors = validator.errors(document);
    assert!(
        errors.is_empty(),
        "generated OpenVEX document does not match the published 0.2.0 schema: {errors:#?}"
    );
}

#[trace(
    "FR-216-AC-1",
    "FR-216-AC-2",
    "FR-216-AC-3",
    "FR-216-AC-5",
    "FR-216-AC-7",
    "FR-216-AC-8",
    "FR-216-AC-10"
)]
#[test]
fn export_copies_authored_values_with_caller_metadata_and_stays_deterministic() {
    let fixture = Fixture::new();
    fixture.record(
        "SOUP-A",
        PURL_A,
        Some(json!([
            {
                "advisory":"CVE-2026-0001",
                "status":"affected",
                "assessed_by":"Ada Example",
                "assessed_at":"2026-11-23",
                "impact_statement":"Impact as authored.",
                "action_statement":"Use release 1.0.229.",
                "evidence_refs":["EVIDENCE-1"]
            },
            {
                "advisory":"CVE-2026-0002",
                "status":"under_investigation",
                "assessed_by":"Ada Example",
                "assessed_at":"2026-11-23"
            }
        ])),
    );
    fixture.record(
        "SOUP-B",
        PURL_B,
        Some(json!([
            statement("CVE-2026-0001", "fixed"),
            {
                "advisory":"CVE-2026-0003",
                "status":"not_affected",
                "assessed_by":"Ada Example",
                "assessed_at":"2026-11-23",
                "justification":"component_not_present"
            }
        ])),
    );
    std::fs::create_dir_all(fixture.path().join("spec/vulnerabilities")).unwrap();
    std::fs::write(
        fixture.path().join("spec/vulnerabilities/VULN-001.md"),
        "---\nid: VULN-001\ntype: vulnerability\nstatus: closed\ncve_id: CVE-2026-9999\n---\n",
    )
    .unwrap();
    std::fs::create_dir_all(fixture.path().join("spec/evidence/scans")).unwrap();
    std::fs::write(
        fixture.path().join("spec/evidence/scans/advisories.json"),
        br#"{"findings":[{"id":"CVE-2026-7777","status":"affected"}]}"#,
    )
    .unwrap();
    let first_path = fixture.path().join("one.json");
    let second_path = fixture.path().join("two.json");

    let first = fixture.command(&first_path);
    assert!(first.status.success(), "{}", stderr(&first));
    let second = fixture.command(&second_path);
    assert!(second.status.success(), "{}", stderr(&second));
    let first_bytes = std::fs::read(&first_path).unwrap();
    let second_bytes = std::fs::read(&second_path).unwrap();
    assert_eq!(first_bytes, second_bytes);
    let document: Value = serde_json::from_slice(&first_bytes).unwrap();
    assert_matches_openvex_020_schema(&document);

    assert_eq!(document["@context"], "https://openvex.dev/ns/v0.2.0");
    assert_eq!(document["@id"], "urn:example:vex:document-1");
    assert_eq!(document["author"], "security@example.test");
    assert_eq!(document["timestamp"], "2026-11-24T10:00:00Z");
    assert_eq!(document["version"], 1);
    assert_eq!(
        keys(&document),
        [
            "@context",
            "@id",
            "author",
            "statements",
            "timestamp",
            "version"
        ]
    );
    assert!(document.get("role").is_none());
    let statements = document["statements"].as_array().unwrap();
    assert_eq!(statements.len(), 4);
    assert_eq!(statements[0]["vulnerability"]["name"], "CVE-2026-0001");
    assert_eq!(statements[0]["status"], "affected");
    assert_eq!(statements[0]["products"][0]["@id"], PURL_A);
    assert_eq!(statements[0]["impact_statement"], "Impact as authored.");
    assert_eq!(statements[0]["action_statement"], "Use release 1.0.229.");
    assert_eq!(
        keys(&statements[0]),
        [
            "action_statement",
            "impact_statement",
            "products",
            "status",
            "vulnerability"
        ]
    );
    assert!(statements[0].get("assessed_by").is_none());
    assert!(statements[0].get("assessed_at").is_none());
    assert!(statements[0].get("evidence_refs").is_none());
    assert_eq!(statements[1]["status"], "under_investigation");
    assert_eq!(
        keys(&statements[1]),
        ["products", "status", "vulnerability"]
    );
    assert!(statements[1].get("justification").is_none());
    assert!(statements[1].get("impact_statement").is_none());
    assert!(statements[1].get("action_statement").is_none());
    assert_eq!(statements[2]["status"], "fixed");
    assert_eq!(statements[2]["products"][0]["@id"], PURL_B);
    assert_eq!(statements[3]["justification"], "component_not_present");
    assert!(statements[3].get("impact_statement").is_none());
    assert!(statements[3].get("action_statement").is_none());
}

#[trace("FR-216-AC-3")]
#[test]
fn supplied_role_is_written_verbatim() {
    let fixture = Fixture::new();
    fixture.record("SOUP-A", PURL_A, Some(json!([statement("CVE-1", "fixed")])));
    let output = fixture.path().join("vex.json");
    let result = Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(["soup", "vex", "export", "--repo"])
        .arg(fixture.path())
        .args(["--out"])
        .arg(&output)
        .args([
            "--id",
            "urn:example:vex:document-2",
            "--author",
            "security@example.test",
            "--role",
            "Product security team",
            "--timestamp",
            "2026-11-24T10:00:00Z",
            "--version",
            "2",
        ])
        .output()
        .unwrap();
    assert!(result.status.success(), "{}", stderr(&result));
    let document: Value = serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap();
    assert_matches_openvex_020_schema(&document);
    assert_eq!(document["role"], "Product security team");
}

#[trace("FR-216-AC-4")]
#[test]
fn no_statements_is_actionable_and_never_creates_or_replaces_output() {
    let fixture = Fixture::new();
    fixture.record("SOUP-A", PURL_A, None);
    fixture.record("SOUP-B", PURL_B, Some(json!([])));
    std::fs::create_dir_all(fixture.path().join("spec/vulnerabilities")).unwrap();
    std::fs::write(
        fixture.path().join("spec/vulnerabilities/VULN-001.md"),
        "---\nid: VULN-001\ntype: vulnerability\nstatus: open\ncve_id: CVE-2026-0001\n---\n",
    )
    .unwrap();
    let new_output = fixture.path().join("new.json");

    let refused = fixture.command(&new_output);
    assert_eq!(refused.status.code(), Some(2), "{}", stderr(&refused));
    let message = format!("{}{}", stdout(&refused), stderr(&refused));
    assert!(message.contains("OpenVEX 0.2.0 requires at least one statement"));
    assert!(message.contains("add an authored vex_statements entry"));
    assert!(!new_output.exists());

    let existing = fixture.path().join("existing.json");
    std::fs::write(&existing, b"keep these bytes\n").unwrap();
    let refused = fixture.command(&existing);
    assert_eq!(refused.status.code(), Some(2), "{}", stderr(&refused));
    assert_eq!(std::fs::read(existing).unwrap(), b"keep these bytes\n");
}

#[trace("FR-216-AC-6")]
#[test]
fn malformed_statement_names_record_and_field_and_preserves_output() {
    let fixture = Fixture::new();
    let mut malformed = statement("CVE-2026-0099", "fixed");
    malformed["status"] = json!("invented_status");
    fixture.record("SOUP-BAD", PURL_A, Some(json!([malformed])));
    let output = fixture.path().join("existing.json");
    std::fs::write(&output, b"preserve\n").unwrap();

    let refused = fixture.command(&output);
    assert_eq!(refused.status.code(), Some(2), "{}", stderr(&refused));
    let diagnostic = format!("{}{}", stdout(&refused), stderr(&refused));
    assert!(diagnostic.contains("SOUP-BAD.md"));
    assert!(diagnostic.contains("status"));
    assert_eq!(std::fs::read(output).unwrap(), b"preserve\n");
}

#[trace("FR-216-AC-9")]
#[test]
fn repeated_advisory_in_one_record_fails_without_writing() {
    let fixture = Fixture::new();
    fixture.record(
        "SOUP-DUP",
        PURL_A,
        Some(json!([
            statement("CVE-2026-0001", "fixed"),
            statement("CVE-2026-0001", "not_affected")
        ])),
    );
    let new_output = fixture.path().join("new.json");

    let refused = fixture.command(&new_output);
    assert_eq!(refused.status.code(), Some(2), "{}", stderr(&refused));
    let diagnostic = format!("{}{}", stdout(&refused), stderr(&refused));
    assert!(diagnostic.contains("SOUP-DUP.md"));
    assert!(diagnostic.contains("vex_statements"));
    assert!(diagnostic.contains("CVE-2026-0001"));
    assert!(!new_output.exists());

    let existing = fixture.path().join("existing.json");
    std::fs::write(&existing, b"unchanged\n").unwrap();
    let refused = fixture.command(&existing);
    assert_eq!(refused.status.code(), Some(2), "{}", stderr(&refused));
    assert_eq!(std::fs::read(existing).unwrap(), b"unchanged\n");
}
