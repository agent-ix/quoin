// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Dossier determinism and recorded-values constraints.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "test failures should report the violated assertion"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use ix_trace_rs::trace;

fn quoin(work: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(args)
        .current_dir(work)
        .output()
        .expect("quoin starts")
}

fn quoin_with_safety_fixture(work: &Path, args: &[&str]) -> Output {
    let checkout = safety_checkout();
    Command::new(env!("CARGO_BIN_EXE_quoin"))
        .args(args)
        .env("SPEC_OBJECTS_SAFETY_DIR", checkout)
        .current_dir(work)
        .output()
        .expect("quoin starts")
}

fn repo(work: &Path) -> String {
    work.to_str().unwrap().to_owned()
}

fn write_fixture(root: &Path, order: &[&str]) {
    fs::create_dir_all(root.join("spec")).unwrap();
    let fixture_manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../quoin-core/tests/fixtures/quire-repo/manifest.yaml");
    fs::copy(fixture_manifest, root.join("manifest.yaml")).unwrap();
    for id in order {
        fs::write(
            root.join("spec").join(format!("{id}.md")),
            format!(
                "---\nid: {id}\ntype: FR\ntitle: Requirement {id}\n---\n\n## Acceptance Criteria\n\n| ID | Criteria | Verification |\n| --- | --- | --- |\n| {id}-AC-1 | Authored statement for {id}. | Test |\n"
            ),
        )
        .unwrap();
    }
}

fn omit_verification_methods(root: &Path, ids: &[&str]) {
    for id in ids {
        let path = root.join("spec").join(format!("{id}.md"));
        let text = fs::read_to_string(&path)
            .unwrap()
            .replace("| Test |", "|  |");
        fs::write(path, text).unwrap();
    }
}

fn safety_checkout() -> std::path::PathBuf {
    let module_exists =
        |checkout: &Path| checkout.join("spec_objects_safety/manifest.yaml").is_file();
    if let Some(override_path) = std::env::var_os("SPEC_OBJECTS_SAFETY_DIR") {
        let checkout = std::path::PathBuf::from(override_path);
        if module_exists(&checkout) {
            return checkout;
        }
    }

    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .map(|ancestor| ancestor.join("spec-objects-safety"))
        .find(|checkout| module_exists(checkout))
        .expect("spec-objects-safety checkout with module manifest is available")
}

fn safety_module() -> String {
    safety_checkout()
        .join("spec_objects_safety")
        .to_str()
        .unwrap()
        .to_owned()
}

fn values_after_field<'a>(document: &'a str, field: &str) -> impl Iterator<Item = &'a str> {
    document
        .lines()
        .filter_map(move |line| line.strip_prefix(field))
        .map(str::trim)
}

fn table_cells(document: &str) -> impl Iterator<Item = &str> {
    document
        .lines()
        .filter(|line| line.starts_with('|'))
        .flat_map(|line| line.split('|').map(str::trim))
        .filter(|cell| !cell.is_empty() && !cell.chars().all(|character| character == '-'))
}

fn write_undecided_risk(root: &Path) {
    let spec = root.join("spec");
    fs::write(
        spec.join("HAZ-1.md"),
        "---\nid: HAZ-1\ntitle: Hazard\ntype: hazard\nobject: hazard\n---\n## Condition\n\nA test hazard.\n",
    )
    .unwrap();
    fs::write(
        spec.join("SIT-1.md"),
        "---\nid: SIT-1\ntitle: Situation\ntype: hazardous_situation\nobject: hazardous_situation\nrelationships:\n  - target: HAZ-1\n    type: arises_from\n---\n## Description\n\nA test situation.\n",
    )
    .unwrap();
    fs::write(
        spec.join("EST-1.md"),
        "---\nid: EST-1\ntitle: Estimate\ntype: risk_estimate\nobject: risk_estimate\nphase: post_control\nrelationships:\n  - target: SIT-1\n    type: estimates\n---\n## Judgement\n\n| Severity | Probability | Rationale |\n| --- | --- | --- |\n| unknown | unknown | not assessed |\n\n## Provenance\n\n| Asserted by | Asserted at |\n| --- | --- |\n|  |  |\n\n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n| --- | --- | --- | --- |\n| record_id | UUID | 1..1 | identity |\n",
    )
    .unwrap();
    fs::write(
        spec.join("RR-1.md"),
        "---\nid: RR-1\ntitle: Residual risk\ntype: residual_risk\nobject: residual_risk\nrelationships:\n  - target: EST-1\n    type: estimate\n---\n## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| undecided | not recorded |  |\n\n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n| --- | --- | --- | --- |\n| record_id | UUID | 1..1 | identity |\n",
    )
    .unwrap();
}

fn selected_docs(root: &Path) -> [String; 3] {
    let path = repo(root);
    let output = quoin(
        root,
        &[
            "dossier",
            "generate",
            "--version",
            "1",
            "--doc",
            "srs",
            "--doc",
            "architecture",
            "--doc",
            "trace-matrix",
            "--scope",
            &path,
            "--module",
            &path,
        ],
    );
    assert!(matches!(output.status.code(), Some(0 | 1)), "{output:?}");
    ["DOS-SRS", "DOS-ARCH", "DOS-TRACE"]
        .map(|name| fs::read_to_string(root.join("dossier").join(format!("{name}.md"))).unwrap())
}

#[trace("NFR-040-AC-3", "NFR-040-M-3")]
#[test]
fn outputs_do_not_supply_unrecorded_criticality_or_safety_values() {
    let work = tempfile::tempdir().unwrap();
    write_fixture(work.path(), &["FR-002", "FR-001"]);
    let docs = selected_docs(work.path());
    for (name, document) in ["SRS", "architecture", "trace matrix"]
        .into_iter()
        .zip(docs)
    {
        assert!(
            values_after_field(&document, "Criticality: ").all(|value| value == "unassigned"),
            "{name} rendered an unrecorded criticality: {document}"
        );
        assert!(
            values_after_field(&document, "Software safety class: ")
                .all(|value| value == "unassigned"),
            "{name} rendered an unrecorded software safety class: {document}"
        );
        let cells: Vec<_> = table_cells(&document).collect();
        for unsupported in ["high", "critical", "compliant", "acceptable"] {
            assert!(
                !cells.contains(&unsupported),
                "{name} rendered unsourced value {unsupported:?}: {document}"
            );
        }
        assert!(
            !cells.iter().any(|cell| cell.starts_with("class-")),
            "{name} rendered an unrecorded class: {document}"
        );
        if name == "SRS" || name == "trace matrix" {
            assert!(
                document.contains("Authored statement"),
                "{name} omitted authored requirement text: {document}"
            );
        }
    }
}

#[trace("NFR-040-AC-4")]
#[test]
fn disk_creation_order_does_not_change_generated_bytes() {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    write_fixture(left.path(), &["FR-001", "FR-002"]);
    write_fixture(right.path(), &["FR-002", "FR-001"]);
    assert_eq!(selected_docs(left.path()), selected_docs(right.path()));
}

#[trace("NFR-040-AC-5")]
#[test]
fn untimed_document_selection_writes_no_evaluation_time() {
    let work = tempfile::tempdir().unwrap();
    write_fixture(work.path(), &["FR-001"]);
    let docs = selected_docs(work.path());
    for document in docs {
        for time_field in ["as_of:", "generated_at:", "timestamp:"] {
            assert!(!document.contains(time_field), "{time_field}: {document}");
        }
    }
}

#[trace("NFR-041-AC-3", "NFR-041-M-3")]
#[test]
fn authored_requirement_bytes_are_preserved_and_change_only_the_srs_content() {
    let before = tempfile::tempdir().unwrap();
    let after = tempfile::tempdir().unwrap();
    write_fixture(before.path(), &["FR-001", "FR-002"]);
    write_fixture(after.path(), &["FR-001", "FR-002"]);
    let source = after.path().join("spec/FR-001.md");
    let authored = fs::read_to_string(&source).unwrap().replace(
        "Authored statement for FR-001.",
        "Changed authored meaning — preserve bytes.",
    );
    fs::write(source, authored).unwrap();

    let before_docs = selected_docs(before.path());
    let after_docs = selected_docs(after.path());
    let source_text = "Authored statement for FR-001.";
    let changed_text = "Changed authored meaning — preserve bytes.";
    assert_eq!(
        before_docs[0].replace(source_text, changed_text),
        after_docs[0]
    );
    assert_eq!(before_docs[1], after_docs[1], "architecture output changed");
    assert_eq!(
        before_docs[2].replace(source_text, changed_text),
        after_docs[2],
        "trace-matrix changed beyond the authored criterion bytes"
    );
    assert!(after_docs[0].contains(changed_text));
    assert!(after_docs[2].contains(changed_text));
    assert!(after_docs[0].contains("Authored statement for FR-002."));
    assert!(after_docs[2].contains("Authored statement for FR-002."));
}

fn clause_binding(path: &Path, id: &str, clause: &str) {
    fs::write(
        path,
        format!(
            r#"{{"schemaVersion":"clause-binding-v1","clauseSet":{{"authority":"example.invalid","id":"{id}","version":"1"}},"context":{{}},"clauses":[{{"clauseId":"{clause}","force":"mandatory","outcome":"binding","reasons":[],"expectedOutputs":[]}}]}}"#
        ),
    )
    .unwrap();
}

#[trace("NFR-041-AC-1", "NFR-041-M-1")]
#[test]
fn missing_values_remain_explicit_gaps_in_the_selected_m2_documents() {
    let work = tempfile::tempdir().unwrap();
    write_fixture(work.path(), &["FR-001"]);
    omit_verification_methods(work.path(), &["FR-001"]);
    let scope = repo(work.path());
    let output = quoin(
        work.path(),
        &[
            "dossier",
            "generate",
            "--version",
            "1",
            "--json",
            "--doc",
            "verification-report",
            "--doc",
            "soup",
            "--doc",
            "risk-management",
            "--doc",
            "risk-management-plan",
            "--doc",
            "release-record",
            "--doc",
            "trace-matrix",
            "--scope",
            &scope,
            "--module",
            &scope,
        ],
    );
    assert!(matches!(output.status.code(), Some(0 | 1)), "{output:?}");
    let expected = [
        "DOS-VR",
        "DOS-SOUP",
        "DOS-RISK",
        "DOS-RMP",
        "DOS-REL",
        "DOS-TRACE",
    ];
    for id in expected {
        let markdown = fs::read_to_string(work.path().join("dossier").join(format!("{id}.md")))
            .unwrap_or_else(|error| panic!("{id} was not written: {error}"));
        assert!(
            markdown.contains("not stated")
                || markdown.contains("unknown")
                || markdown.contains("unassigned")
                || markdown.contains("No authored")
                || markdown.contains("no_")
                || markdown.contains("no ")
                || markdown.contains("## Gaps")
                || markdown.contains("none"),
            "{id} has no explicit missing-value representation: {markdown}"
        );
        if id == "DOS-TRACE" {
            assert!(!markdown.contains("| Test |"), "{markdown}");
        }
        for fabricated in [
            "Method: Test",
            "Support status: supported",
            "Safety impact: acceptable",
            "Policy region: acceptable",
            "Decision: accepted",
            "Risk acceptability: acceptable",
            "Release verdict: accepted",
            "Status: released",
        ] {
            assert!(!markdown.contains(fabricated), "{id}: {markdown}");
        }
    }
    for id in expected {
        assert!(
            work.path()
                .join("dossier")
                .join(format!("{id}.md"))
                .is_file()
        );
    }
}

#[trace("NFR-041-AC-4")]
#[test]
fn not_released_stays_not_released_and_undecided_risk_is_not_accepted() {
    let work = tempfile::tempdir().unwrap();
    write_fixture(work.path(), &["FR-001"]);
    write_undecided_risk(work.path());
    let verify = work.path().join("release-verify.json");
    fs::write(
        &verify,
        r#"{"protocol":"engineering-assurance.release-verify-result/v1","status":"not_released","tag":"v1","reasons":["approval_not_found"]}"#,
    )
    .unwrap();
    let verify = verify.to_str().unwrap();
    let scope = repo(work.path());
    let safety = safety_module();
    let output = quoin_with_safety_fixture(
        work.path(),
        &[
            "dossier",
            "generate",
            "--version",
            "1",
            "--json",
            "--doc",
            "release-record",
            "--doc",
            "risk-management",
            "--scope",
            &scope,
            "--module",
            &scope,
            "--module",
            &safety,
            "--release-verify",
            verify,
        ],
    );
    assert!(matches!(output.status.code(), Some(0 | 1)), "{output:?}");
    let json_output = String::from_utf8_lossy(&output.stdout);
    assert!(json_output.contains("incomplete"), "{json_output}");
    let release = fs::read_to_string(work.path().join("dossier/DOS-REL.md")).unwrap();
    assert!(release.contains("Status: not_released"), "{release}");
    assert!(!release.contains("Status: accepted"), "{release}");
    let risk = fs::read_to_string(work.path().join("dossier/DOS-RISK.md")).unwrap();
    let residual = risk.lines().find(|line| line.starts_with("| RR-1 |"));
    assert_eq!(
        residual,
        Some("| RR-1 | undecided | not stated | not recorded |")
    );
    assert!(!risk.contains("| RR-1 | accepted |"), "{risk}");
    assert!(!risk.contains("| RR-1 | acceptable |"), "{risk}");
}

fn rmm_sections(document: &str) -> (Vec<String>, BTreeMap<String, String>) {
    let sections = document
        .split("\n## ")
        .skip(1)
        .map(|section| {
            let (heading, body) = section.split_once('\n').unwrap();
            (heading.to_owned(), body.to_owned())
        })
        .filter(|(heading, _)| heading != "Gaps")
        .collect::<Vec<_>>();
    let order = sections
        .iter()
        .map(|(heading, _)| heading.clone())
        .collect();
    let by_heading = sections.into_iter().collect();
    (order, by_heading)
}

#[trace("NFR-040-AC-4")]
#[test]
fn clause_binding_order_changes_only_the_order_of_rmm_tables() {
    let work = tempfile::tempdir().unwrap();
    write_fixture(work.path(), &["FR-001"]);
    let first = work.path().join("first.json");
    let second = work.path().join("second.json");
    clause_binding(&first, "first-set", "C-1");
    clause_binding(&second, "second-set", "C-2");
    let generate = |output_dir: &str, left: &Path, right: &Path| {
        let scope = repo(work.path());
        let left = left.to_str().unwrap();
        let right = right.to_str().unwrap();
        let output = quoin(
            work.path(),
            &[
                "dossier",
                "generate",
                "--version",
                "1",
                "--doc",
                "rmm",
                "--out",
                output_dir,
                "--scope",
                &scope,
                "--module",
                &scope,
                "--as-of",
                "2026-11-27T00:00:00Z",
                "--clause-binding",
                left,
                "--clause-binding",
                right,
            ],
        );
        assert!(matches!(output.status.code(), Some(0 | 1)), "{output:?}");
        fs::read_to_string(work.path().join(output_dir).join("DOS-RMM.md")).unwrap()
    };
    let forward = generate("forward", &first, &second);
    let reverse = generate("reverse", &second, &first);
    let (forward_order, forward_tables) = rmm_sections(&forward);
    let (reverse_order, reverse_tables) = rmm_sections(&reverse);
    assert_eq!(
        forward_order,
        [
            "example.invalid/first-set@1",
            "example.invalid/second-set@1"
        ]
    );
    assert_eq!(
        reverse_order,
        [
            "example.invalid/second-set@1",
            "example.invalid/first-set@1"
        ]
    );
    assert_eq!(forward_tables, reverse_tables);
    assert_eq!(forward.split("\n## ").next(), reverse.split("\n## ").next());
    assert!(forward.contains("As of: 2026-11-27T00:00:00Z"));
}
