// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Unit tests of the snapshot's pure parts: the history file, the matrix
//! sections and the closed document types. The command itself is exercised end
//! to end in `tests/tc_3604_release_snapshot.rs`.

use ix_trace_rs::trace;
use serde_json::{Value, json};

use super::history::parse;
use super::is_commit_hash;
use super::snapshot::{Snapshot, sections};

fn history(records: &Value, previous: &Value) -> String {
    json!({
        "protocol": "engineering-assurance.release-history/v1",
        "baseline_commit": "a".repeat(40),
        "previous_release": previous,
        "records": records,
    })
    .to_string()
}

fn entry(id: &str, revision: u64, commit: &str) -> Value {
    json!({ "record_id": id, "revision": revision, "path": format!("records/{id}.md"), "commit": commit })
}

#[trace("FR-168-AC-1")]
#[test]
fn records_out_of_order_are_ordered_by_id_then_numeric_revision_then_commit() {
    let parsed = parse(&history(
        &json!([
            entry("CR-2", 1, "c1"),
            entry("CR-1", 10, "c1"),
            entry("CR-1", 2, "c9")
        ]),
        &Value::Null,
    ))
    .unwrap();
    let order: Vec<(&str, u64)> = parsed
        .records
        .iter()
        .map(|record| (record.record_id.as_str(), record.revision))
        .collect();
    assert_eq!(order, [("CR-1", 2), ("CR-1", 10), ("CR-2", 1)]);
    assert_eq!(parsed.records[0].path, "records/CR-1.md");
    assert_eq!(parsed.records[0].commit, "c9");
}

#[trace("FR-168-AC-2", "FR-168-AC-5")]
#[test]
fn previous_release_is_copied_as_given_and_an_empty_list_is_not_a_refusal() {
    let first = parse(&history(&json!([]), &Value::Null)).unwrap();
    assert!(first.previous_release.is_none());
    assert!(first.records.is_empty());
    let later = parse(&history(
        &json!([]),
        &json!({ "tag": "v1.0.0", "commit": "b".repeat(40) }),
    ))
    .unwrap();
    let previous = later.previous_release.unwrap();
    assert_eq!(
        (previous.tag.as_str(), previous.commit.len()),
        ("v1.0.0", 40)
    );
}

#[trace("FR-168-AC-3")]
#[test]
fn a_history_file_that_is_not_the_named_shape_is_invalid_and_names_the_fault() {
    let good = json!({
        "protocol": "engineering-assurance.release-history/v1",
        "baseline_commit": "a".repeat(40),
        "previous_release": null,
        "records": [entry("CR-1", 1, "c1")],
    });
    let with = |edit: &dyn Fn(&mut Value)| {
        let mut document = good.clone();
        edit(&mut document);
        parse(&document.to_string()).unwrap_err().member
    };
    assert_eq!(
        with(&|d| d["protocol"] = json!("engineering-assurance.release-history/v2")),
        "protocol engineering-assurance.release-history/v2"
    );
    assert_eq!(
        with(&|d| {
            d.as_object_mut().unwrap().remove("records");
        }),
        "records"
    );
    assert_eq!(
        with(&|d| {
            d["records"][0].as_object_mut().unwrap().remove("commit");
        }),
        "records[0].commit"
    );
    assert_eq!(
        with(&|d| d["records"] = json!([entry("CR-1", 1, "c1"), entry("CR-1", 1, "c2")])),
        "CR-1 revision 1 at commits c1 and c2"
    );
    assert_eq!(
        with(&|d| {
            d.as_object_mut().unwrap().remove("previous_release");
        }),
        "previous_release"
    );
}

#[trace("FR-166-AC-3")]
#[test]
fn only_a_full_lowercase_hexadecimal_hash_of_40_or_64_digits_is_a_baseline_commit() {
    assert!(is_commit_hash(&"a".repeat(40)));
    assert!(is_commit_hash(&"0".repeat(64)));
    for bad in [
        "main".to_owned(),
        "abc1234".to_owned(),
        "A".repeat(40),
        "a".repeat(41),
        "g".repeat(40),
        String::new(),
    ] {
        assert!(!is_commit_hash(&bad), "{bad}");
    }
}

fn criterion(id: &str, status: &str, findings: &Value) -> Value {
    json!({
        "id": id, "static_status": "tagged", "binders": [], "evidence_status": status,
        "evidence_detail": { "findings": findings, "bindings": [], "unevaluated": [] },
    })
}

fn finding(kind: &str, summary: &str, path: Option<&str>, line: Option<u64>) -> Value {
    json!({ "kind": kind, "summary": summary, "path": path, "line": line })
}

#[trace("FR-167-AC-2", "FR-167-AC-3")]
#[test]
fn freshness_counts_every_criterion_and_findings_follow_the_total_order() {
    let matrix = serde_json::from_value(json!({ "requirements": [
        { "id": "FR-1", "criteria": [
            criterion("FR-1-AC-1", "bound", &json!([])),
            criterion("FR-1-AC-2", "stale", &json!([finding("stale-evidence", "old run", None, None)])),
            criterion("FR-1-AC-3", "suspect", &json!([
                finding("surviving-mutant", "b", Some("src/a.rs"), Some(10)),
                finding("surviving-mutant", "a", Some("src/a.rs"), Some(10)),
                finding("surviving-mutant", "z", Some("src/a.rs"), Some(9)),
                finding("surviving-mutant", "z", None, None),
                finding("surviving-mutant", "z", Some("src/a.rs"), None),
            ])),
        ]},
        { "id": "FR-2", "criteria": [criterion("FR-2-AC-1", "no run evidence", &json!([]))] },
    ]}))
    .unwrap();
    let built = sections(matrix);

    let statuses: Vec<(&str, String)> = built
        .coverage
        .iter()
        .flat_map(|requirement| &requirement.criteria)
        .map(|c| {
            (
                c.id.as_str(),
                serde_json::to_value(c.evidence_status).unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(statuses[1], ("FR-1-AC-2", "\"stale\"".to_owned()));
    assert_eq!(statuses[2], ("FR-1-AC-3", "\"suspect\"".to_owned()));

    let freshness = serde_json::to_value(built.freshness).unwrap();
    assert_eq!(
        freshness,
        json!({ "bound": 1, "stale": 1, "suspect": 1, "undischarged": 0, "no run evidence": 1 })
    );
    let total: u64 = freshness
        .as_object()
        .unwrap()
        .values()
        .map(|v| v.as_u64().unwrap())
        .sum();
    assert_eq!(total, 4);

    let order: Vec<(&str, Option<u64>, &str)> = built
        .findings
        .iter()
        .map(|f| (f.obligation.as_str(), f.line, f.summary.as_str()))
        .collect();
    // Within one obligation and kind: no path first; then path; an absent line
    // before a present one, numerically; then summary.
    assert_eq!(
        order,
        [
            ("FR-1-AC-2", None, "old run"),
            ("FR-1-AC-3", None, "z"),
            ("FR-1-AC-3", None, "z"),
            ("FR-1-AC-3", Some(9), "z"),
            ("FR-1-AC-3", Some(10), "a"),
            ("FR-1-AC-3", Some(10), "b"),
        ]
    );
    assert_eq!(built.findings[1].path, None);
    assert_eq!(built.findings[2].path.as_deref(), Some("src/a.rs"));
}

fn sample() -> Value {
    json!({
        "protocol": "quoin.release-snapshot/v1",
        "as_of": "2026-08-15T00:00:00Z",
        "baseline_commit": "a".repeat(40),
        "previous_release": { "tag": "v1", "commit": "b".repeat(40) },
        "coverage": [{ "id": "FR-1", "criteria": [
            { "id": "FR-1-AC-1", "static_status": "tagged", "evidence_status": "bound" } ] }],
        "evidence_freshness": { "bound": 1, "stale": 0, "suspect": 0, "undischarged": 0, "no run evidence": 0 },
        "open_findings": [{ "obligation": "FR-1-AC-1", "kind": "k", "summary": "s",
            "path": null, "line": null, "symbol": null }],
        "soup": {
            "rows": [{ "purl": "pkg:cargo/a@1", "version": "1", "group": "other",
                "support_status": { "value": "unknown", "origin": "none" },
                "end_of_support": { "value": "unknown", "origin": "none" } }],
            "not_assessed": [{ "kind": "scan_not_run", "tool": "cargo-deny", "detail": "not run" }],
        },
        "anomalies": [{ "id": "AN-1", "title": "t", "status": "identified", "resolution": null,
            "severity": null, "safety_impact": "unassessed", "unassessed": true,
            "release_rationale": [{ "author": "A", "rationale": "R" }] }],
        "change_records": [{ "record_id": "CR-1", "revision": 1, "path": "p", "commit": "c" }],
    })
}

#[trace("FR-167-AC-1")]
#[test]
fn the_closed_types_read_a_snapshot_and_refuse_an_extra_member_at_every_object() {
    serde_json::from_value::<Snapshot>(sample()).expect("the sample is a snapshot");
    let extra_at: [&dyn Fn(&mut Value) -> &mut Value; 8] = [
        &|d| d,
        &|d| &mut d["coverage"][0]["criteria"][0],
        &|d| &mut d["open_findings"][0],
        &|d| &mut d["soup"]["rows"][0],
        &|d| &mut d["anomalies"][0],
        &|d| &mut d["change_records"][0],
        &|d| &mut d["soup"]["not_assessed"][0],
        &|d| &mut d["soup"]["rows"][0]["support_status"],
    ];
    for at in extra_at {
        let mut document = sample();
        at(&mut document)["extra"] = json!(true);
        assert!(serde_json::from_value::<Snapshot>(document).is_err());
    }
}
