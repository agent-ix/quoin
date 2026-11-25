// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! SOUP dossier fixtures exercising the shared FR-144 source (FR-197).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use ix_trace_rs::trace;
use quoin_dossier::{GapKind, Inputs, SourceFailure};
use quoin_evidence::store::write_scan;
use quoin_evidence::types::FindingRecord;
use quoin_evidence::{DiskEvidence, STORE_SCHEMA_VERSION, SuiteId};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

const AS_OF: &str = "2026-08-15T00:00:00Z";

fn raw_source(inputs: &Inputs) -> Result<quoin_soup::DossierSource, quoin_soup::SoupError> {
    quoin_soup::dossier_source(
        &inputs.scope,
        inputs.soup_records.as_deref(),
        inputs.soup_binding.as_deref().unwrap(),
        inputs.as_of.as_deref().unwrap(),
        inputs.soup_support.as_deref(),
        &quoin_core::runtime::RuntimeSettings::default(),
    )
}

fn source(inputs: &Inputs) -> Result<quoin_soup::DossierSource, SourceFailure> {
    raw_source(inputs).map_err(|error| SourceFailure {
        source: "quoin soup report".to_owned(),
        detail: error.message().to_owned(),
    })
}

fn registry() -> quoin_dossier::Registry {
    quoin_dossier::builtin_registry(quoin_dossier::Sources {
        matrix: std::sync::Arc::new(NoMatrix),
        rmm: |_| panic!("unexpected discharge source"),
        soup: source,
        policy: |_| panic!("unexpected policy source"),
        threat_model: |_| panic!("unexpected threat-model source"),
        records_of_processing: |_| panic!("unexpected records-of-processing source"),
    })
    .unwrap()
}

struct NoMatrix;

impl quoin_dossier::MatrixSource for NoMatrix {
    fn matrix(&self, _: &Inputs) -> Result<quoin_assurance::matrix::MatrixOutput, SourceFailure> {
        panic!("unexpected matrix source")
    }
}

fn inputs(repo: &Path) -> Inputs {
    Inputs {
        scope: repo.to_path_buf(),
        modules: Vec::new(),
        clause_bindings: Vec::new(),
        pack: None,
        soup_binding: None,
        soup_records: None,
        soup_support: None,
        snapshot: None,
        release_verify: None,
        as_of: None,
        version: "1".to_owned(),
        producer_version: "test-quoin-version".to_owned(),
    }
}

fn record(repo: &Path, name: &str, fields: &str) {
    fs::create_dir_all(repo.join("spec/soup")).unwrap();
    fs::write(repo.join(format!("spec/soup/{name}.md")), format!("---\nid: SOUP-{name}\ntype: soup_record\npurl: pkg:cargo/{name}@1.0.0\nversion: '1.0.0'\n{fields}\n---\n# {name}\n")).unwrap();
}

fn binding(repo: &Path) -> PathBuf {
    let path = repo.join("binding.json");
    fs::write(&path, json!({
        "schemaVersion": "clause-binding-v1", "clauseSet": {"authority":"example.invalid","id":"soup","version":"1"},
        "context": {}, "clauses": []
    }).to_string()).unwrap();
    path
}

fn scan(repo: &Path, tool: &str, raw: &str) {
    let adapter = quoin_evidence::adapters::select_finding_adapter(
        quoin_evidence::adapters::AdapterSelection {
            adapter: None,
            tool: Some(tool),
        },
    )
    .unwrap();
    let parsed = adapter.run(raw, None).unwrap();
    write_scan(
        &mut DiskEvidence::new(repo),
        &FindingRecord {
            schema_version: STORE_SCHEMA_VERSION,
            suite: SuiteId::new(tool),
            tool: tool.to_owned(),
            evidence_kind: None,
            timestamp: "2026-08-01T00:00:00Z".to_owned(),
            ruleset: parsed.ruleset,
            rules_evaluated: parsed.rules_evaluated,
            findings: parsed.findings,
            evaluated: parsed.evaluated,
        },
    )
    .unwrap();
}

fn fixture() -> (tempfile::TempDir, Inputs) {
    let repo = tempfile::tempdir().unwrap();
    record(repo.path(), "unresolved", "known_anomalies: []");
    record(
        repo.path(),
        "old",
        "serves: []\nknown_anomalies: []\nend_of_support: '2026-05-01'",
    );
    record(
        repo.path(),
        "clean",
        "serves: [FR-1]\nknown_anomalies: []\nsupport_status: supported\nend_of_support: '2027-01-01'",
    );
    let audit_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/fixtures/evidence/cargo-audit-real.json");
    let mut audit: Value = serde_json::from_slice(&fs::read(audit_path).unwrap()).unwrap();
    audit["vulnerabilities"] = json!({"found":true,"count":2,"list":[
        {"advisory":{"id":"RUSTSEC-2026-0001","title":"fixture advisory"},"package":{"name":"unresolved","version":"1.0.0"}},
        {"advisory":{"id":"RUSTSEC-2026-0002","title":"unmatched"},"package":{"name":"ghost","version":"1.0.0"}}
    ]});
    audit["warnings"] = json!({});
    scan(repo.path(), "cargo-audit", &audit.to_string());
    let mut input = inputs(repo.path());
    input.soup_binding = Some(binding(repo.path()));
    input.as_of = Some(AS_OF.to_owned());
    (repo, input)
}

fn generate(input: &Inputs) -> (String, quoin_dossier::Report) {
    let out = input.scope.join("dossier");
    let report = quoin_dossier::generate(&registry(), &["soup".to_owned()], &out, input).unwrap();
    (fs::read_to_string(out.join("DOS-SOUP.md")).unwrap(), report)
}

#[trace(
    "FR-197-AC-1",
    "FR-197-AC-2",
    "FR-197-AC-4",
    "FR-197-AC-6",
    "FR-197-AC-7",
    "FR-197-AC-10"
)]
#[test]
fn shared_source_order_words_gaps_and_bytes_are_preserved_without_sbom() {
    let (repo, input) = fixture();
    fs::write(
        repo.path().join("sbom.json"),
        "an SBOM is not a SOUP source",
    )
    .unwrap();
    let (text, report) = generate(&input);
    let rows: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("| pkg:"))
        .collect();
    assert_eq!(rows.len(), 3);
    assert!(rows[0].starts_with("| pkg:cargo/unresolved@1.0.0 | 1.0.0 | not stated | none |"));
    assert!(rows[1].starts_with("| pkg:cargo/old@1.0.0 | 1.0.0 | none | none |"));
    assert!(rows[2].starts_with("| pkg:cargo/clean@1.0.0 | 1.0.0 | FR-1 | none |"));
    assert!(rows[0].ends_with("| unknown | unknown |"));
    assert!(rows[1].ends_with("| unknown | 2026-05-01 |"));
    assert!(text.contains("As of: 2026-08-15T00:00:00Z"));
    assert!(text.contains("as_of:"));
    assert!(text.contains("RUSTSEC-2026-0002"));
    assert!(text.contains("cargo-deny: not run"));
    assert!(text.contains("policy findings not assessed: licence-policy not run"));
    assert!(!text.contains("allowed"));
    let gaps = &report.documents[0].gaps;
    assert!(
        gaps.iter()
            .any(|gap| gap.gap_kind == GapKind::SoupFieldNotStated
                && gap.subject == "pkg:cargo/unresolved@1.0.0"
                && gap.detail.starts_with("serves:"))
    );
    assert!(
        !gaps
            .iter()
            .any(|gap| gap.subject == "pkg:cargo/old@1.0.0" && gap.detail.starts_with("serves:"))
    );
    assert_eq!(
        gaps.iter()
            .filter(|gap| gap.gap_kind == GapKind::SoupPastEndOfSupport)
            .count(),
        1
    );
    fs::remove_file(repo.path().join("sbom.json")).unwrap();
    assert_eq!(text, generate(&input).0);
    assert_eq!(text, generate(&input).0);
    // Adding a record reaches both the source and the dossier exactly once.
    record(repo.path(), "added", "serves: []\nsupport_status: unknown");
    assert_eq!(
        source(&input)
            .unwrap()
            .rows
            .iter()
            .filter(|row| row.cells[0] == "pkg:cargo/added@1.0.0")
            .count(),
        1
    );
    assert_eq!(
        generate(&input)
            .0
            .matches("| pkg:cargo/added@1.0.0 |")
            .count(),
        1
    );
    let (added, report) = generate(&input);
    let row = added
        .lines()
        .find(|line| line.starts_with("| pkg:cargo/added@1.0.0 |"))
        .unwrap();
    assert!(row.starts_with("| pkg:cargo/added@1.0.0 | 1.0.0 | none | not stated |"));
    assert!(row.ends_with("| unknown | unknown |"));
    assert!(
        report.documents[0]
            .gaps
            .iter()
            .any(|gap| gap.subject == "pkg:cargo/added@1.0.0"
                && gap.gap_kind == GapKind::SoupFieldNotStated
                && gap.detail.starts_with("known_anomalies:"))
    );
}

#[trace("FR-197-AC-3")]
#[test]
fn advisory_and_policy_remain_unresolved_and_name_each_finding() {
    let (_repo, input) = fixture();
    let sbom = json!({"bomFormat":"CycloneDX","components":[{"name":"unresolved","version":"1.0.0","purl":"pkg:cargo/unresolved@1.0.0","licenses":[{"license":{"id":"GPL-3.0-only"}}]}]});
    let parsed = quoin_evidence::adapters::parse_licence_policy(
        &sbom.to_string(),
        "format: licence-policy-v1\nallow: [MIT]\ndeny: [GPL-3.0-only]\n",
    )
    .unwrap();
    write_scan(
        &mut DiskEvidence::new(&input.scope),
        &FindingRecord {
            schema_version: STORE_SCHEMA_VERSION,
            suite: SuiteId::new("licence"),
            tool: "licence-policy".to_owned(),
            evidence_kind: None,
            timestamp: "2026-08-01T00:00:00Z".to_owned(),
            ruleset: parsed.ruleset,
            rules_evaluated: parsed.rules_evaluated,
            findings: parsed.findings,
            evaluated: parsed.evaluated,
        },
    )
    .unwrap();
    let (text, report) = generate(&input);
    let row = text
        .lines()
        .find(|line| line.starts_with("| pkg:cargo/unresolved"))
        .unwrap();
    assert_eq!(row.matches("unresolved:").count(), 2);
    assert!(!row.contains("allowed"));
    let gaps: Vec<_> = report.documents[0]
        .gaps
        .iter()
        .filter(|gap| gap.gap_kind == GapKind::SoupUnresolvedAdvisory)
        .collect();
    assert_eq!(gaps.len(), 2);
    assert_eq!(gaps[0].detail, "cargo-audit RUSTSEC-2026-0001");
    assert_eq!(gaps[1].detail, "licence-policy licence-denied");
}

#[trace("FR-197-AC-5", "FR-197-AC-8", "FR-197-AC-9", "FR-197-AC-10")]
#[test]
fn absent_binding_skips_source_but_selected_source_failures_write_nothing() {
    let repo = tempfile::tempdir().unwrap();
    let mut input = inputs(repo.path());
    let (text, report) = generate(&input);
    assert_eq!(report.outcome().code(), 1);
    assert_eq!(
        report.documents[0].gaps[0].gap_kind,
        GapKind::NoClauseBinding
    );
    assert!(!text.contains("as_of:"));
    assert!(!text.contains("As of:"));
    let out = repo.path().join("never-written");
    input.soup_binding = Some(repo.path().join("missing-binding"));
    let error =
        quoin_dossier::generate(&registry(), &["soup".to_owned()], &out, &input).unwrap_err();
    assert_eq!(error.outcome().code(), 2);
    assert!(!out.exists());
    input.as_of = Some(AS_OF.to_owned());
    record(repo.path(), "one", "serves: []");
    assert_eq!(
        raw_source(&input).unwrap_err().code(),
        quoin_soup::SoupErrorCode::Operational
    );
    let error =
        quoin_dossier::generate(&registry(), &["soup".to_owned()], &out, &input).unwrap_err();
    assert_eq!(error.outcome().code(), 3);
    assert_eq!(error.source_failure().unwrap().source, "quoin soup report");
    assert!(
        error
            .source_failure()
            .unwrap()
            .detail
            .contains("missing-binding")
    );
    assert!(!out.exists());
    input.soup_binding = Some(binding(repo.path()));
    fs::write(
        repo.path().join("spec/soup/one.md"),
        "---\ntype: soup_record\npurl: pkg:cargo/one@1\nversion: 42\n---\n",
    )
    .unwrap();
    let error =
        quoin_dossier::generate(&registry(), &["soup".to_owned()], &out, &input).unwrap_err();
    assert_eq!(error.outcome().code(), 3);
    assert!(error.source_failure().unwrap().detail.contains("version"));
    assert!(!out.exists());
    let source_error = raw_source(&input).unwrap_err();
    assert_eq!(
        source_error.code(),
        quoin_soup::SoupErrorCode::SourceRefused
    );
    assert_eq!(
        source_error.source_diagnostics()[0].code,
        "soup.record_field_invalid"
    );
    assert_eq!(
        source_error.source_diagnostics()[0].context["field"],
        "version"
    );
}

fn tool(name: &str, arguments: &[&str], work: &Path) -> String {
    let output = std::process::Command::new(name)
        .args(arguments)
        .current_dir(work)
        .output()
        .unwrap();
    assert!(output.status.success(), "{name} {arguments:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// Read the installed module in place, validate both source-backed and gap
/// documents, then export the committed source-backed list with real quire.
#[trace("FR-197-AC-10", "FR-197-AC-1")]
#[test]
#[ignore = "needs quire, pandoc, typst, pdftotext and SPEC_ARTIFACTS_PROCESS_DIR; run with -- --ignored"]
fn real_quire_validates_both_soup_forms_and_exports_the_list() {
    let module =
        fs::canonicalize(std::env::var_os("SPEC_ARTIFACTS_PROCESS_DIR").expect(
            "set SPEC_ARTIFACTS_PROCESS_DIR to the installed spec_artifacts_process module",
        ))
        .unwrap();
    let (repo, input) = fixture();
    let (expected, _) = generate(&input);
    let source_path = repo.path().join("dossier/DOS-SOUP.md");
    let gap_input = inputs(repo.path());
    let gaps_out = repo.path().join("gaps");
    quoin_dossier::generate(&registry(), &["soup".to_owned()], &gaps_out, &gap_input).unwrap();
    for path in [&source_path, &gaps_out.join("DOS-SOUP.md")] {
        tool(
            "quire",
            &[
                "validate",
                "--module",
                module.to_str().unwrap(),
                path.to_str().unwrap(),
            ],
            repo.path(),
        );
    }
    tool("git", &["init", "-q"], repo.path());
    tool(
        "git",
        &["config", "user.email", "t@example.com"],
        repo.path(),
    );
    tool("git", &["config", "user.name", "t"], repo.path());
    tool("git", &["add", "-A"], repo.path());
    tool(
        "git",
        &["commit", "-q", "-m", "generated SOUP list"],
        repo.path(),
    );
    let pdf = repo.path().join("soup.pdf");
    tool(
        "quire",
        &[
            "export",
            source_path.to_str().unwrap(),
            "--module",
            module.to_str().unwrap(),
            "--to",
            "pdf",
            "--output",
            pdf.to_str().unwrap(),
        ],
        repo.path(),
    );
    let exported = tool("pdftotext", &[pdf.to_str().unwrap(), "-"], repo.path());
    let normalized: String = exported.split_whitespace().collect();
    for component in [
        "pkg:cargo/unresolved@1.0.0",
        "pkg:cargo/old@1.0.0",
        "pkg:cargo/clean@1.0.0",
    ] {
        assert!(
            normalized.contains(component),
            "{component} absent from PDF text: {exported}"
        );
    }
    assert!(exported.contains("unresolved"));
    assert!(exported.contains("unknown"));
    assert!(expected.contains("## Gaps"));
}
