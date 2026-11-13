// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

//! The anomaly report, the derived git values and the `--fail-on` gate
//! (FR-162, FR-163, FR-164, NFR-035, and FR-136's fixing change).
//!
//! The anomaly documents are the fixtures `spec-artifacts-process` ships
//! (FR-014, FR-015, FR-016), read in place and never copied into this
//! repository; where a case needs a variation, the test derives it from a
//! shipped document in memory. Only the commit sequences are authored here, in
//! throwaway repositories.

mod common;

use std::collections::BTreeSet;
use std::path::Path;

use common::{Repo, fixture, fixtures, walk, with_status};
use ix_trace_rs::trace;
use quoin_anomaly::{
    Context, Derived, DiagnosticCode, GateMode, Report, Signature, build_report, check_document,
    parse_record, rationale_change,
};
use serde_json::{Value, json};

/// Replace the frontmatter line `key: ...` with `line`, or drop it for `None`.
fn set_key(text: &str, key: &str, line: Option<&str>) -> String {
    let prefix = format!("{key}:");
    let mut out = Vec::new();
    let mut replaced = false;
    for current in text.lines() {
        if !replaced && current.starts_with(&prefix) {
            replaced = true;
            if let Some(line) = line {
                out.push(line.to_owned());
            }
        } else {
            out.push(current.to_owned());
        }
    }
    out.join("\n") + "\n"
}

/// A shipped fixture under the id `id`.
fn fx(group: &str, name: &str, id: &str) -> String {
    fixture(group, name).replace("id: AN-001", &format!("id: {id}"))
}

fn anomaly_path(id: &str) -> String {
    format!("spec/anomalies/{id}.md")
}

/// A git repository holding the shipped corpus (requirements, hazards, test
/// matrix) in one commit.
fn corpus() -> Repo {
    let mut repo = Repo::new();
    let root = fixtures().join("corpus/spec");
    let mut last = None;
    for file in walk(&root) {
        let rel = Path::new("spec").join(file.strip_prefix(&root).unwrap());
        let rel = rel.to_str().unwrap().to_owned();
        repo.write(&rel, &std::fs::read_to_string(&file).unwrap());
        last = Some(rel);
    }
    let last = last.unwrap();
    repo.commit(&last, &std::fs::read_to_string(repo.path(&last)).unwrap());
    repo
}

/// Commit the anomalies, one commit each, in order.
fn corpus_with(anomalies: &[(&str, &str, &str)]) -> Repo {
    let mut repo = corpus();
    for (id, group, name) in anomalies {
        repo.commit(&anomaly_path(id), &fx(group, name, id));
    }
    repo
}

fn report(repo: &Repo) -> Report {
    build_report(repo.dir.path(), &[]).unwrap()
}

fn report_with(repo: &Repo, modes: &[GateMode]) -> Report {
    build_report(repo.dir.path(), modes).unwrap()
}

fn json_of(report: &Report) -> Value {
    serde_json::from_str(&report.to_json().unwrap()).unwrap()
}

fn ids(report: &Report) -> Vec<&str> {
    report.anomalies.iter().map(|row| row.id.as_str()).collect()
}

fn row<'a>(report: &'a Report, id: &str) -> &'a quoin_anomaly::report::ReportRow {
    report.anomalies.iter().find(|row| row.id == id).unwrap()
}

fn keys(value: &Value) -> BTreeSet<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

const CONTEXT: Context = Context {
    hazard_installed: true,
};

#[trace("FR-162-AC-1", "FR-162-AC-2")]
#[test]
fn every_carried_or_unassessed_anomaly_is_listed_once_in_id_order_and_flagged() {
    let repo = corpus_with(&[
        ("AN-008", "invalid", "closed-unassessed"),
        ("AN-005", "valid", "closed-wont-fix"),
        ("AN-001", "valid", "identified"),
        ("AN-002", "valid", "assessed"),
        ("AN-003", "valid", "resolved-fixed"),
        ("AN-004", "valid", "closed-fixed"),
        ("AN-006", "valid", "closed-duplicate"),
        ("AN-007", "valid", "closed-not-a-defect"),
    ]);
    let report = report(&repo);
    assert_eq!(
        ids(&report),
        ["AN-001", "AN-002", "AN-003", "AN-005", "AN-008"],
        "closed fixed, duplicate and not-a-defect are not listed; a closed unassessed one is"
    );
    let flagged = report
        .anomalies
        .iter()
        .filter(|row| row.unassessed)
        .map(|row| row.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(flagged, ["AN-001", "AN-008"]);
    let json = json_of(&report);
    for entry in json["anomalies"].as_array().unwrap() {
        assert_eq!(
            entry["unassessed"],
            json!(entry["safetyImpact"] == "unassessed")
        );
    }
    let markdown = report.to_markdown();
    let flag_cells = markdown
        .lines()
        .filter(|line| line.starts_with("| AN-"))
        .filter(|line| line.contains("| unassessed | unassessed |"))
        .count();
    assert_eq!(flag_cells, 2, "{markdown}");
}

#[trace("FR-162-AC-3")]
#[test]
fn a_row_carries_every_key_as_authored_and_states_absent_values_as_absent() {
    let repo = corpus_with(&[
        ("AN-001", "valid", "identified"),
        ("AN-002", "valid", "closed-wont-fix-rationale"),
    ]);
    let report = report(&repo);
    let json = json_of(&report);
    let expected = BTreeSet::from([
        "id",
        "title",
        "path",
        "status",
        "resolution",
        "severity",
        "safetyImpact",
        "unassessed",
        "assessment",
        "affected",
        "releaseRationale",
        "fixingChange",
        "rationaleChange",
    ]);
    for entry in json["anomalies"].as_array().unwrap() {
        assert_eq!(keys(entry), expected);
    }
    let identified = &json["anomalies"][0];
    assert_eq!(identified["severity"], Value::Null);
    assert_eq!(identified["resolution"], Value::Null);
    assert_eq!(identified["assessment"], json!([]));
    assert_eq!(identified["affected"], json!([]));
    assert_eq!(identified["title"], "Fixture identified");
    let wont_fix = &json["anomalies"][1];
    assert_eq!(wont_fix["severity"], "major");
    assert_eq!(wont_fix["resolution"], "wont-fix");
    assert_eq!(
        wont_fix["assessment"][0],
        json!({
            "kind": "Requirement",
            "target": "FR-001",
            "impact": "Affects",
            "rationale": "The anomaly shows FR-001 is not enforced."
        })
    );
    assert_eq!(
        wont_fix["affected"][2],
        json!({ "target": "HAZ-001", "kind": "Hazard", "impact": "Introduces" })
    );
    let markdown = report.to_markdown();
    let identified_row = markdown
        .lines()
        .find(|line| line.starts_with("| AN-001 "))
        .unwrap();
    assert!(
        identified_row.contains("| not stated | not stated |"),
        "{identified_row}"
    );
}

#[trace("FR-162-AC-4")]
#[test]
fn an_anomaly_appears_in_the_group_of_each_target_it_names_and_once_in_the_rows() {
    let mut repo = corpus();
    let text = fx("valid", "two-hazards", "AN-001").replace(
        "| Hazard | HAZ-002 | Affects | A second corrupt-artifact path. |",
        "| Hazard | HAZ-002 | Affects | A second corrupt-artifact path. |\n| Requirement | FR-001 | Affects | Not enforced. |",
    );
    repo.commit(&anomaly_path("AN-001"), &text);
    let report = report(&repo);
    assert_eq!(ids(&report), ["AN-001"]);
    let json = json_of(&report);
    assert_eq!(
        json["byHazard"],
        json!([
            { "target": "HAZ-001", "anomalies": [{ "anomaly": "AN-001", "impact": "Introduces" }] },
            { "target": "HAZ-002", "anomalies": [{ "anomaly": "AN-001", "impact": "Affects" }] },
        ])
    );
    assert_eq!(
        json["byRequirement"],
        json!([{ "target": "FR-001", "anomalies": [{ "anomaly": "AN-001", "impact": "Affects" }] }])
    );
}

#[trace("FR-162-AC-5")]
#[test]
fn a_group_lists_the_open_anomaly_only_and_an_unnamed_hazard_has_no_group() {
    let repo = corpus_with(&[
        ("AN-001", "valid", "impacting-all-targets"),
        ("AN-002", "valid", "closed-fixed"),
    ]);
    let json = json_of(&report(&repo));
    assert_eq!(
        json["byHazard"],
        json!([{ "target": "HAZ-001", "anomalies": [{ "anomaly": "AN-001", "impact": "Introduces" }] }])
    );
    let text = json.to_string();
    assert!(!text.contains("HAZ-002") && !text.contains("HAZ-003"));
    assert!(
        !report_markdown_mentions(&repo, "AN-002"),
        "a closed fixed anomaly is nowhere in the report"
    );
}

fn report_markdown_mentions(repo: &Repo, id: &str) -> bool {
    report(repo).to_markdown().contains(id)
}

#[trace("FR-162-AC-6")]
#[test]
fn a_requirement_and_its_criterion_are_separate_groups() {
    let mut repo = corpus();
    let requirement_only = fx("valid", "impacting-all-targets", "AN-001").replace(
        "| Requirement | FR-001-AC-1 | Affects | The criterion is not met. |\n",
        "",
    );
    let criterion_only = fx("valid", "impacting-all-targets", "AN-002").replace(
        "| Requirement | FR-001 | Affects | The anomaly shows FR-001 is not enforced. |\n",
        "",
    );
    repo.commit(&anomaly_path("AN-001"), &requirement_only);
    repo.commit(&anomaly_path("AN-002"), &criterion_only);
    let json = json_of(&report(&repo));
    assert_eq!(
        json["byRequirement"],
        json!([
            { "target": "FR-001", "anomalies": [{ "anomaly": "AN-001", "impact": "Affects" }] },
            { "target": "FR-001-AC-1", "anomalies": [{ "anomaly": "AN-002", "impact": "Affects" }] },
        ])
    );
}

#[trace("FR-162-AC-7")]
#[test]
fn unaffected_is_stated_a_none_row_makes_no_group_and_ungrouped_has_reasons() {
    let mut repo = corpus();
    let unaffected = fx("valid", "impacting-all-targets", "AN-001").replace(
        "| Hazard | HAZ-001 | Introduces | A corrupt artifact is accepted. |",
        "| Hazard | HAZ-001 | Unaffected | The path never reaches the hazard. |",
    );
    repo.commit(&anomaly_path("AN-001"), &unaffected);
    repo.commit(&anomaly_path("AN-002"), &fx("valid", "none", "AN-002"));
    repo.commit(
        &anomaly_path("AN-003"),
        &fx("valid", "identified", "AN-003"),
    );
    let json = json_of(&report(&repo));
    assert_eq!(
        json["byHazard"],
        json!([{ "target": "HAZ-001", "anomalies": [{ "anomaly": "AN-001", "impact": "Unaffected" }] }])
    );
    assert!(
        !json["byHazard"].to_string().contains("AN-002")
            && !json["byRequirement"].to_string().contains("AN-002")
    );
    // The `none` anomaly is not carried only if closed; here it is assessed, so
    // it is listed and, naming nothing, ungrouped.
    assert_eq!(
        json["ungrouped"],
        json!([
            { "anomaly": "AN-002", "reason": "no-affected-id" },
            { "anomaly": "AN-003", "reason": "unassessed" },
        ])
    );
}

#[trace("FR-162-AC-8")]
#[test]
fn a_dangling_target_is_grouped_as_authored() {
    let repo = corpus_with(&[("AN-001", "invalid", "dangling-requirement")]);
    let report = report(&repo);
    let json = json_of(&report);
    assert_eq!(json["byRequirement"][0]["target"], "FR-999");
    assert!(report.unreadable.is_empty());
    assert!(!report.to_markdown().contains("dangling"));
}

#[trace("FR-162-AC-9")]
#[test]
fn an_unreadable_document_is_named_and_fails_while_other_rows_render() {
    let mut repo = corpus_with(&[("AN-001", "valid", "assessed")]);
    let broken = set_key(&fx("valid", "assessed", "AN-002"), "safety_impact", None);
    repo.commit(&anomaly_path("AN-002"), &broken);
    let report = report(&repo);
    assert_eq!(ids(&report), ["AN-001"]);
    assert_eq!(report.unreadable.len(), 1);
    assert!(
        report.unreadable[0]
            .path
            .ends_with("spec/anomalies/AN-002.md")
    );
    assert_eq!(report.unreadable[0].code, "missing");
    assert!(report.failed());
    let markdown = report.to_markdown();
    assert!(markdown.contains("AN-002.md: missing"), "{markdown}");

    let empty = corpus();
    let empty = self::report(&empty);
    assert!(!empty.failed());
    let markdown = empty.to_markdown();
    assert_eq!(markdown.matches("\nnone\n").count(), 5, "{markdown}");
}

#[trace("FR-162-AC-10")]
#[test]
fn the_json_has_exactly_its_keys_and_the_markdown_its_sections_in_order() {
    let repo = corpus_with(&[("AN-001", "valid", "assessed")]);
    let plain = report(&repo);
    assert_eq!(
        keys(&json_of(&plain)),
        BTreeSet::from([
            "anomalies",
            "byHazard",
            "byRequirement",
            "ungrouped",
            "unreadable"
        ])
    );
    let gated = report_with(&repo, &[GateMode::Unassessed]);
    assert!(keys(&json_of(&gated)).contains("gate"));
    let markdown = gated.to_markdown();
    let at = |heading: &str| {
        markdown
            .find(heading)
            .unwrap_or_else(|| panic!("{heading}"))
    };
    assert!(
        at("## Anomalies") < at("## By hazard")
            && at("## By hazard") < at("## By requirement")
            && at("## By requirement") < at("## Ungrouped")
            && at("## Ungrouped") < at("## Unreadable")
            && at("## Unreadable") < at("## Gate")
    );
    assert!(!plain.to_markdown().contains("## Gate"));
}

fn tree(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = walk(dir)
        .into_iter()
        .filter(|path| !path.components().any(|c| c.as_os_str() == ".git"))
        .map(|path| {
            let bytes = std::fs::read(&path).unwrap();
            (path.to_string_lossy().into_owned(), bytes)
        })
        .collect::<Vec<_>>();
    out.sort();
    out
}

#[trace("FR-162-AC-11", "NFR-035-AC-1", "NFR-035-AC-5")]
#[test]
fn two_runs_give_identical_bytes_and_change_no_file() {
    let repo = corpus_with(&[
        ("AN-001", "valid", "rationale-assessed-with"),
        ("AN-002", "valid", "identified"),
        ("AN-003", "valid", "closed-wont-fix-rationale"),
    ]);
    let before = tree(repo.dir.path());
    for modes in [
        &[][..],
        &[GateMode::Unassessed, GateMode::UnjustifiedImpacting],
    ] {
        let first = report_with(&repo, modes);
        let second = report_with(&repo, modes);
        assert_eq!(first.to_markdown(), second.to_markdown());
        assert_eq!(first.to_json().unwrap(), second.to_json().unwrap());
    }
    assert_eq!(tree(repo.dir.path()), before);
}

/// The five value keys of a derived value that is not `present`.
fn assert_all_null(derived: &Value) {
    for key in [
        "commit",
        "author",
        "authoredAt",
        "signature",
        "signatureCode",
    ] {
        assert_eq!(derived[key], Value::Null, "{key} of {derived}");
    }
}

fn state(value: &Value) -> &str {
    value["state"].as_str().unwrap()
}

#[trace("FR-162-AC-14", "FR-136-AC-15", "FR-163-AC-6")]
#[test]
fn the_four_states_are_encoded_in_json_and_markdown() {
    let mut repo = corpus();
    // AN-001: closed fixed after resolved, so both fixingChange and
    // rationaleChange can be present; AN-002: wont-fix, fixingChange absent.
    let resolved = fx("valid", "rationale-resolved-with", "AN-001");
    repo.commit(&anomaly_path("AN-001"), &resolved);
    repo.commit(
        &anomaly_path("AN-002"),
        &fx("valid", "closed-wont-fix-rationale", "AN-002"),
    );
    // AN-003: committed assessed, with a working-tree-only move to resolved.
    repo.commit(
        &anomaly_path("AN-003"),
        &fx("valid", "rationale-assessed-with", "AN-003"),
    );
    repo.write(
        &anomaly_path("AN-003"),
        &fx("valid", "rationale-resolved-with", "AN-003")
            .replace("until the next release", "until release 2"),
    );
    let report = report(&repo);
    let json = json_of(&report);
    let by_id = |id: &str| {
        json["anomalies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == id)
            .unwrap()
    };
    let present = &by_id("AN-001")["fixingChange"];
    assert_eq!(state(present), "present");
    assert_eq!(
        keys(present),
        BTreeSet::from([
            "state",
            "commit",
            "author",
            "authoredAt",
            "signature",
            "signatureCode"
        ])
    );
    assert_eq!(present["author"], "Fixture Author");
    assert_eq!(present["signature"], "unsigned");
    assert_eq!(present["signatureCode"], "N");
    assert_eq!(state(&by_id("AN-001")["rationaleChange"]), "present");
    let absent = &by_id("AN-002")["fixingChange"];
    assert_eq!(state(absent), "absent");
    let pending = &by_id("AN-003")["fixingChange"];
    assert_eq!(state(pending), "pending");
    assert_all_null(absent);
    assert_all_null(pending);
    assert_eq!(state(&by_id("AN-003")["rationaleChange"]), "pending");

    let markdown = report.to_markdown();
    let commit = present["commit"].as_str().unwrap();
    assert!(markdown.contains(&format!(
        "{commit} Fixture Author {} unsigned (N)",
        present["authoredAt"].as_str().unwrap()
    )));
    assert!(markdown.contains("| absent |"), "{markdown}");
    assert!(markdown.contains("| absent (pending) |"), "{markdown}");

    // A file outside git.
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(
        outside.path().join("AN-001.md"),
        fx("valid", "rationale-resolved-with", "AN-001"),
    )
    .unwrap();
    let outside = build_report(outside.path(), &[]).unwrap();
    let json = json_of(&outside);
    assert_eq!(
        state(&json["anomalies"][0]["fixingChange"]),
        "not-applicable"
    );
    assert_eq!(
        state(&json["anomalies"][0]["rationaleChange"]),
        "not-applicable"
    );
    assert_all_null(&json["anomalies"][0]["fixingChange"]);
    assert_all_null(&json["anomalies"][0]["rationaleChange"]);
    assert!(
        outside
            .to_markdown()
            .contains("| not applicable | not applicable |")
    );
}

#[trace("FR-163-AC-1")]
#[test]
fn the_release_rationale_is_stated_as_authored_in_document_order() {
    let mut repo = corpus();
    let text = fx("valid", "rationale-assessed-with", "AN-001")
        + "| Casey TAN | Mixed CASE reason, kept as written. |\n";
    repo.commit(&anomaly_path("AN-001"), &text);
    let report = report(&repo);
    let json = json_of(&report);
    assert_eq!(
        json["anomalies"][0]["releaseRationale"],
        json!([
            {
                "author": "Morgan Reyes",
                "rationale": "The checksum path is not on a safety function and the operator procedure catches the case until the next release."
            },
            { "author": "Casey TAN", "rationale": "Mixed CASE reason, kept as written." },
        ])
    );
    assert!(
        report
            .to_markdown()
            .contains("Casey TAN: Mixed CASE reason, kept as written.")
    );
}

#[trace("FR-163-AC-2")]
#[test]
fn an_anomaly_without_the_section_states_none_and_no_template() {
    let repo = corpus_with(&[("AN-001", "valid", "rationale-assessed-without")]);
    let report = report(&repo);
    let json = json_of(&report);
    assert_eq!(json["anomalies"][0]["releaseRationale"], json!([]));
    assert_eq!(state(&json["anomalies"][0]["rationaleChange"]), "absent");
    let markdown = report.to_markdown();
    let line = markdown
        .lines()
        .find(|l| l.starts_with("| AN-001 "))
        .unwrap();
    assert!(line.contains("| none | absent | absent |"), "{line}");
    for word in ["Author |", "TBD", "tbd", "suggest", "template"] {
        assert!(!markdown.contains(word), "{word}\n{markdown}");
    }
}

#[trace("NFR-035-AC-4")]
#[test]
fn with_every_optional_value_absent_each_printed_value_reads_as_absent_or_is_copied() {
    // `identified` has no severity, resolution, assessment or rationale.
    let repo = corpus_with(&[("AN-001", "valid", "identified")]);
    let report = report(&repo);
    let json = json_of(&report);
    let entry = &json["anomalies"][0];
    for key in ["resolution", "severity"] {
        assert_eq!(entry[key], Value::Null, "{key}");
    }
    for key in ["assessment", "affected", "releaseRationale"] {
        assert_eq!(entry[key], json!([]), "{key}");
    }
    assert_eq!(state(&entry["fixingChange"]), "absent");
    assert_eq!(state(&entry["rationaleChange"]), "absent");
    let markdown = report.to_markdown();
    let line = markdown
        .lines()
        .find(|l| l.starts_with("| AN-001 "))
        .unwrap();
    let cells = line
        .trim_matches('|')
        .split(" | ")
        .map(str::trim)
        .collect::<Vec<_>>();
    // ID, Title, Path, Status, Safety impact are copied from the record or the
    // path; every other cell is an absent marker or a flag word.
    let allowed = ["not stated", "none", "absent", "unassessed"];
    for (index, cell) in cells.iter().enumerate() {
        if matches!(index, 0 | 1 | 2 | 3 | 6) {
            continue;
        }
        assert!(allowed.contains(cell), "cell {index}: {cell} in {line}");
    }
}

#[trace("FR-163-AC-4")]
#[test]
fn the_typed_author_is_stated_not_the_closer_or_the_commit_author() {
    let repo = corpus_with(&[("AN-001", "valid", "closed-wont-fix-rationale")]);
    let report = report(&repo);
    let entry = &row(&report, "AN-001").release_rationale[0];
    assert_eq!(entry.author, "Morgan Reyes");
    let change = match &row(&report, "AN-001").rationale_change {
        Derived::Present(change) => change,
        other => panic!("{other:?}"),
    };
    assert_eq!(change.author, "Fixture Author");
    assert_ne!(change.author, entry.author, "both values are stated");
}

fn change_commit(derived: &Derived) -> &str {
    match derived {
        Derived::Present(change) => &change.commit,
        other => panic!("{other:?}"),
    }
}

fn rationale_of(repo: &Repo, id: &str) -> Derived {
    let path = repo.path(&anomaly_path(id));
    let record = parse_record(&std::fs::read_to_string(&path).unwrap()).unwrap();
    rationale_change(&path, &record).unwrap()
}

#[trace("FR-163-AC-5")]
#[test]
fn the_rationale_change_is_the_latest_commit_that_changed_the_section() {
    let mut repo = corpus();
    let path = anomaly_path("AN-001");
    repo.commit(&path, &fx("valid", "rationale-assessed-without", "AN-001"));
    let added = repo.commit(&path, &fx("valid", "rationale-assessed-with", "AN-001"));
    // Other parts of the file change after the section was added.
    let other = fx("valid", "rationale-assessed-with", "AN-001")
        .replace("The importer accepted", "The importer wrongly accepted");
    repo.commit(&path, &other);
    assert_eq!(change_commit(&rationale_of(&repo, "AN-001")), added);
    // The section text is edited in a later commit.
    let edited = other.replace("until the next release", "until release 2");
    let edit = repo.commit(&path, &edited);
    let derived = rationale_of(&repo, "AN-001");
    assert_eq!(change_commit(&derived), edit);
    let Derived::Present(change) = derived else {
        panic!()
    };
    assert_eq!(change.author, "Fixture Author");
    assert!(change.authored_at.starts_with("2026-01-01T"));
    assert_eq!(change.signature, Signature::Unsigned);
    assert_eq!(change.signature_code, 'N');
}

#[trace("FR-163-AC-6")]
#[test]
fn a_changed_or_re_added_section_is_pending_and_none_yields_a_diagnostic() {
    let mut repo = corpus();
    let path = anomaly_path("AN-001");
    let with = fx("valid", "rationale-assessed-with", "AN-001");
    repo.commit(&path, &with);
    // The working tree differs from the default branch.
    repo.write(
        &path,
        &with.replace("until the next release", "until release 2"),
    );
    assert_eq!(rationale_of(&repo, "AN-001"), Derived::Pending);
    // The default branch removed the section and the working tree re-adds the
    // old text.
    repo.commit(&path, &fx("valid", "rationale-assessed-without", "AN-001"));
    repo.write(&path, &with);
    assert_eq!(rationale_of(&repo, "AN-001"), Derived::Pending);
    // No section in the working tree.
    repo.write(&path, &fx("valid", "rationale-assessed-without", "AN-001"));
    assert_eq!(rationale_of(&repo, "AN-001"), Derived::Absent);
}

#[trace("FR-163-AC-6")]
#[test]
fn pending_absent_and_not_applicable_rationale_changes_yield_no_diagnostic() {
    let mut repo = corpus();
    let path = anomaly_path("AN-001");
    let with = fx("valid", "rationale-assessed-with", "AN-001");
    repo.commit(&path, &with);
    let edited = with.replace("until the next release", "until release 2");
    let without = fx("valid", "rationale-assessed-without", "AN-001");
    for text in [&edited, &without] {
        repo.write(&path, text);
        let checked = check_document(&repo.path(&path), text, CONTEXT).unwrap();
        assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    }
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("AN-001.md");
    std::fs::write(&outside, &with).unwrap();
    let checked = check_document(&outside, &with, CONTEXT).unwrap();
    assert!(checked.diagnostics.is_empty());
}

#[trace("FR-163-AC-7", "FR-136-AC-16")]
#[test]
fn a_typed_commit_author_or_date_never_reaches_the_derived_values() {
    let mut repo = corpus();
    let path = anomaly_path("AN-001");
    let typed = fx("valid", "rationale-resolved-with", "AN-001")
        .replace(
            "severity: major",
            "severity: major\nfixing_commit: deadbeefdeadbeefdeadbeefdeadbeefdeadbeef\nfixed_by: Typed Person\nfixed_on: 1999-01-01",
        )
        .replace(
            "## Description\n",
            "## Description\n\nFixed by Typed Person in deadbeefdeadbeefdeadbeefdeadbeefdeadbeef on 1999-01-01.\n",
        );
    repo.commit(&path, &typed);
    let report = report(&repo);
    let row = row(&report, "AN-001");
    let values = serde_json::to_string(&json!([row.fixing_change, row.rationale_change])).unwrap();
    for leaked in ["deadbeef", "Typed Person", "1999-01-01"] {
        assert!(!values.contains(leaked), "{leaked} in {values}");
    }
    assert_eq!(row.fixing_change.state(), "present");
}

#[trace("FR-163-AC-8")]
#[test]
fn a_differing_typed_author_and_commit_author_are_both_stated() {
    let repo = corpus_with(&[("AN-001", "valid", "rationale-assessed-with")]);
    let report = report(&repo);
    let row = row(&report, "AN-001");
    assert_eq!(row.release_rationale[0].author, "Morgan Reyes");
    let Derived::Present(change) = &row.rationale_change else {
        panic!()
    };
    assert_eq!(change.author, "Fixture Author");
}

/// The fixing change of the document `base` after committing `statuses` and
/// then `working`, with `resolution` set as given.
fn fixing(
    statuses: &[&str],
    working: Option<&str>,
    resolution: Option<&str>,
) -> (Derived, Vec<String>, Vec<DiagnosticCode>) {
    let mut repo = Repo::new();
    let base = set_key(
        &fixture("valid", "closed-fixed"),
        "resolution",
        resolution.map(|r| format!("resolution: {r}")).as_deref(),
    );
    let mut commits = Vec::new();
    for status in statuses {
        commits.push(repo.commit("AN-001.md", &with_status(&base, status)));
    }
    let text = with_status(&base, working.unwrap_or_else(|| statuses.last().unwrap()));
    repo.write("AN-001.md", &text);
    let checked = check_document(&repo.path("AN-001.md"), &text, CONTEXT).unwrap();
    let codes = checked.diagnostics.iter().map(|d| d.code).collect();
    (checked.fixing_change.unwrap(), commits, codes)
}

fn commit_of(derived: &Derived) -> Option<&str> {
    match derived {
        Derived::Present(change) => Some(&change.commit),
        _ => None,
    }
}

#[trace("FR-136-AC-13", "FR-136-AC-14")]
#[test]
fn the_fixing_change_is_the_first_resolved_or_closed_entry_after_the_last_assessed() {
    let cases: [(&[&str], usize); 6] = [
        (&["identified", "assessed", "resolved", "closed"], 2),
        (&["resolved", "assessed", "resolved"], 2),
        (&["resolved"], 0),
        (&["identified", "resolved"], 1),
        (&["assessed", "closed"], 1),
        (&["identified", "closed"], 1),
    ];
    for (statuses, expected) in cases {
        let (derived, commits, _) = fixing(statuses, None, Some("fixed"));
        assert_eq!(
            commit_of(&derived),
            Some(commits[expected].as_str()),
            "{statuses:?}"
        );
    }
}

#[trace("FR-136-AC-15")]
#[test]
fn the_fixing_change_is_absent_without_a_fixed_resolution_pending_or_not_applicable() {
    for resolution in [
        Some("wont-fix"),
        Some("duplicate"),
        Some("not-a-defect"),
        None,
    ] {
        let (derived, _, codes) = fixing(&["identified", "assessed", "closed"], None, resolution);
        assert_eq!(derived, Derived::Absent, "{resolution:?}");
        assert!(
            codes.is_empty(),
            "no diagnostic for an absent fixing change"
        );
    }
    let (derived, _, codes) = fixing(&["assessed"], Some("resolved"), Some("fixed"));
    assert_eq!(derived, Derived::Pending);
    assert!(
        codes.is_empty(),
        "no diagnostic for a pending fixing change"
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("AN-001.md");
    let text = fixture("valid", "closed-fixed");
    std::fs::write(&path, &text).unwrap();
    let checked = check_document(&path, &text, CONTEXT).unwrap();
    assert_eq!(checked.fixing_change, Some(Derived::NotApplicable));
    assert!(checked.diagnostics.is_empty());
}

#[trace("FR-162-AC-14", "FR-163-AC-5", "FR-136-AC-1")]
#[test]
fn similar_files_committed_separately_each_keep_their_own_history() {
    let mut repo = corpus();
    // Three near-identical anomalies added one commit each: git's copy
    // detection would trace the later ones into the earlier ones' history.
    let first = repo.commit(
        &anomaly_path("AN-001"),
        &fx("valid", "rationale-resolved-with", "AN-001"),
    );
    let second = repo.commit(
        &anomaly_path("AN-002"),
        &fx("valid", "closed-wont-fix-rationale", "AN-002"),
    );
    let third = repo.commit(
        &anomaly_path("AN-003"),
        &fx("valid", "rationale-resolved-with", "AN-003"),
    );
    let report = report(&repo);
    for (id, own) in [("AN-001", &first), ("AN-002", &second), ("AN-003", &third)] {
        let row = row(&report, id);
        assert_eq!(change_commit(&row.rationale_change), own, "{id} rationale");
        if id != "AN-002" {
            assert_eq!(change_commit(&row.fixing_change), own, "{id} fixing");
        }
        let path = repo.path(&anomaly_path(id));
        let text = std::fs::read_to_string(&path).unwrap();
        let checked = check_document(&path, &text, CONTEXT).unwrap();
        assert!(
            checked.diagnostics.is_empty(),
            "{id}: {:?}",
            checked.diagnostics
        );
        let (entries, _) = match quoin_anomaly::read_history(&path).unwrap() {
            quoin_anomaly::History::Entries {
                entries,
                assessments,
            } => (entries, assessments),
            quoin_anomaly::History::NotApplicable => panic!("under git"),
        };
        assert_eq!(entries.len(), 1, "{id} has only its own add commit");
    }
}

#[trace("FR-136-AC-1", "FR-136-AC-3")]
#[test]
fn a_commit_that_moves_one_anomaly_and_adds_a_similar_one_keeps_both_histories() {
    let mut repo = corpus();
    let added = repo.commit(
        &anomaly_path("AN-001"),
        &fx("valid", "closed-fixed", "AN-001"),
    );
    std::fs::create_dir_all(repo.path("spec/anomalies/old")).unwrap();
    repo.git(&[
        "mv",
        &anomaly_path("AN-001"),
        "spec/anomalies/old/AN-001.md",
    ]);
    repo.commit(
        &anomaly_path("AN-002"),
        &fx("valid", "rationale-assessed-with", "AN-002"),
    );
    let new = repo.path(&anomaly_path("AN-002"));
    let text = std::fs::read_to_string(&new).unwrap();
    let checked = check_document(&new, &text, CONTEXT).unwrap();
    assert!(checked.diagnostics.is_empty(), "{:?}", checked.diagnostics);
    let entries = |path: &Path| match quoin_anomaly::read_history(path).unwrap() {
        quoin_anomaly::History::Entries { entries, .. } => entries,
        quoin_anomaly::History::NotApplicable => panic!("under git"),
    };
    assert_eq!(entries(&new).len(), 1, "AN-002 has only its own commit");
    let moved = entries(&repo.path("spec/anomalies/old/AN-001.md"));
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].commit.as_deref(), Some(added.as_str()));
}

#[trace("FR-162-AC-3")]
#[test]
fn a_missing_title_reads_as_null_and_the_record_is_readable() {
    let mut repo = corpus();
    repo.commit(
        &anomaly_path("AN-001"),
        &set_key(&fx("valid", "assessed", "AN-001"), "title", None),
    );
    let report = report(&repo);
    assert!(report.unreadable.is_empty());
    assert_eq!(json_of(&report)["anomalies"][0]["title"], Value::Null);
    let markdown = report.to_markdown();
    let line = markdown
        .lines()
        .find(|l| l.starts_with("| AN-001 "))
        .unwrap();
    assert!(line.starts_with("| AN-001 | not stated |"), "{line}");
}

#[trace("FR-162-AC-9")]
#[test]
fn a_malformed_rationale_table_is_a_mapping_error() {
    let cases = [
        ("columns", fx("invalid", "rationale-bad-headers", "AN-001")),
        ("min-rows", fx("invalid", "rationale-no-rows", "AN-001")),
        (
            "min-rows",
            fx("invalid", "rationale-bare-heading", "AN-001"),
        ),
        (
            "min-rows",
            fx("invalid", "rationale-no-delimiter", "AN-001"),
        ),
    ];
    for (code, text) in cases {
        let mut repo = corpus();
        repo.commit(&anomaly_path("AN-001"), &text);
        let report = report(&repo);
        assert!(report.anomalies.is_empty(), "{code}");
        assert_eq!(report.unreadable.len(), 1, "{code}");
        assert_eq!(report.unreadable[0].code, code);
        assert!(report.failed());
        let checked = check_document(&repo.path(&anomaly_path("AN-001")), &text, CONTEXT).unwrap();
        assert!(
            checked
                .diagnostics
                .iter()
                .any(|d| d.code == DiagnosticCode::NoRecord),
            "{code}: {:?}",
            checked.diagnostics
        );
    }
}

fn gate_ids(report: &Report, mode: GateMode) -> Vec<(String, String)> {
    report
        .gate
        .as_ref()
        .unwrap()
        .failures
        .iter()
        .filter(|failure| failure.mode == mode)
        .map(|failure| (failure.anomaly.clone(), failure.path.clone()))
        .collect()
}

#[trace("FR-164-AC-1", "FR-164-AC-2")]
#[test]
fn the_unassessed_gate_names_every_unassessed_anomaly_and_passes_without_one() {
    let repo = corpus_with(&[
        ("AN-001", "valid", "identified"),
        ("AN-002", "invalid", "closed-unassessed"),
        ("AN-003", "valid", "assessed"),
    ]);
    let report = report_with(&repo, &[GateMode::Unassessed]);
    assert!(report.failed());
    let failures = gate_ids(&report, GateMode::Unassessed);
    assert_eq!(failures.len(), 2);
    assert_eq!(failures[0].0, "AN-001");
    assert!(failures[0].1.ends_with("spec/anomalies/AN-001.md"));
    assert_eq!(failures[1].0, "AN-002");
    assert!(failures[1].1.ends_with("spec/anomalies/AN-002.md"));

    let passing = corpus_with(&[("AN-003", "valid", "assessed")]);
    let report = report_with(&passing, &[GateMode::Unassessed]);
    assert!(!report.failed());
    assert!(report.gate.as_ref().unwrap().failures.is_empty());
    assert_eq!(
        ids(&report),
        ["AN-003"],
        "the full report is still rendered"
    );
    assert!(
        report
            .to_markdown()
            .contains("No anomaly fails the evaluated modes: unassessed.")
    );
}

#[trace("FR-164-AC-3", "FR-164-AC-4", "FR-164-AC-7")]
#[test]
fn the_unjustified_gate_fails_without_a_real_rationale_and_passes_with_one() {
    let repo = corpus_with(&[
        ("AN-001", "valid", "rationale-assessed-without"),
        ("AN-002", "valid", "rationale-resolved-without"),
    ]);
    let report = report_with(&repo, &[GateMode::UnjustifiedImpacting]);
    let failures = gate_ids(&report, GateMode::UnjustifiedImpacting);
    assert_eq!(
        failures.iter().map(|f| f.0.as_str()).collect::<Vec<_>>(),
        ["AN-001", "AN-002"]
    );
    assert!(report.failed());

    let with = corpus_with(&[
        ("AN-001", "valid", "rationale-assessed-with"),
        ("AN-002", "valid", "rationale-resolved-with"),
    ]);
    assert!(!report_with(&with, &[GateMode::UnjustifiedImpacting]).failed());

    // A placeholder or empty cell does not count, in any letter case; a bare
    // `x` does.
    let mut cases = Vec::new();
    for placeholder in ["TBD", "n/a", "-", "None", "UNKNOWN", ""] {
        cases.push(format!("| {placeholder} | A reason. |"));
        cases.push(format!("| Morgan Reyes | {placeholder} |"));
    }
    for (index, row) in cases.iter().enumerate() {
        let mut repo = corpus();
        let text = fx("valid", "rationale-assessed-with", "AN-001").replace(
            "| Morgan Reyes | The checksum path is not on a safety function and the operator procedure catches the case until the next release. |",
            row,
        );
        repo.commit(&anomaly_path("AN-001"), &text);
        let report = report_with(&repo, &[GateMode::UnjustifiedImpacting]);
        assert!(report.failed(), "{index}: {row}");
    }
    let mut repo = corpus();
    let text = fx("valid", "rationale-assessed-with", "AN-001").replace(
        "The checksum path is not on a safety function and the operator procedure catches the case until the next release.",
        "x",
    );
    repo.commit(&anomaly_path("AN-001"), &text);
    assert!(!report_with(&repo, &[GateMode::UnjustifiedImpacting]).failed());
}

#[trace("FR-164-AC-5")]
#[test]
fn only_a_carried_impacting_anomaly_without_a_rationale_fails_the_unjustified_gate() {
    let repo = corpus_with(&[
        ("AN-001", "valid", "closed-fixed"),
        ("AN-002", "valid", "closed-duplicate"),
        ("AN-004", "valid", "none"),
        ("AN-005", "valid", "identified"),
        ("AN-006", "valid", "closed-wont-fix"),
    ]);
    // The shipped not-a-defect fixture is `none`; make it `impacting` so the
    // closed-resolution branch, not the impact, is what spares it.
    let impacting = set_key(
        &fx("valid", "closed-not-a-defect", "AN-003"),
        "safety_impact",
        Some("safety_impact: impacting"),
    );
    let mut repo = repo;
    repo.commit(&anomaly_path("AN-003"), &impacting);
    let report = report_with(&repo, &[GateMode::UnjustifiedImpacting]);
    let failed = gate_ids(&report, GateMode::UnjustifiedImpacting)
        .into_iter()
        .map(|f| f.0)
        .collect::<Vec<_>>();
    assert_eq!(
        failed,
        ["AN-006"],
        "wont-fix with no rationale is the only failure"
    );
    let with = corpus_with(&[("AN-006", "valid", "closed-wont-fix-rationale")]);
    assert!(!report_with(&with, &[GateMode::UnjustifiedImpacting]).failed());
}

#[trace("FR-164-AC-6", "FR-164-AC-12")]
#[test]
fn both_modes_list_each_failure_once_by_anomaly_id_then_mode() {
    let mut repo = corpus();
    repo.commit(
        &anomaly_path("AN-002"),
        &fx("valid", "identified", "AN-002"),
    );
    repo.commit(
        &anomaly_path("AN-001"),
        &fx("valid", "rationale-assessed-without", "AN-001"),
    );
    let both = [GateMode::UnjustifiedImpacting, GateMode::Unassessed];
    let report = report_with(&repo, &both);
    let json = json_of(&report);
    assert_eq!(
        json["gate"]["modes"],
        json!(["unassessed", "unjustified-impacting"])
    );
    let failures = json["gate"]["failures"].as_array().unwrap();
    let summary = failures
        .iter()
        .map(|f| (f["anomaly"].as_str().unwrap(), f["mode"].as_str().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(
        summary,
        [
            ("AN-001", "unjustified-impacting"),
            ("AN-002", "unassessed")
        ]
    );
    for failure in failures {
        assert_eq!(keys(failure), BTreeSet::from(["mode", "anomaly", "path"]));
    }
}

#[trace("FR-164-AC-8")]
#[test]
fn an_unreadable_document_fails_the_gate_even_when_every_readable_one_passes() {
    let mut repo = corpus_with(&[("AN-001", "valid", "assessed")]);
    let broken = set_key(&fx("valid", "assessed", "AN-002"), "safety_impact", None);
    repo.commit(&anomaly_path("AN-002"), &broken);
    let report = report_with(&repo, &[GateMode::Unassessed]);
    assert!(report.gate.as_ref().unwrap().failures.is_empty());
    assert!(report.failed());
    assert_eq!(report.unreadable.len(), 1);
    assert!(
        report.unreadable[0]
            .path
            .ends_with("spec/anomalies/AN-002.md")
    );
    let json = json_of(&report);
    assert!(
        json["unreadable"][0]["path"]
            .as_str()
            .unwrap()
            .ends_with("AN-002.md")
    );
}

#[trace("FR-164-AC-9")]
#[test]
fn without_fail_on_there_is_no_gate_and_unassessed_anomalies_do_not_fail() {
    let repo = corpus_with(&[("AN-001", "valid", "identified")]);
    let report = report(&repo);
    assert!(report.gate.is_none());
    assert!(!report.failed());
    assert!(!keys(&json_of(&report)).contains("gate"));
}

/// The stem "accept" may appear only in cells copied from a record: the
/// title, the assessment, the affected targets and the release rationale.
fn assert_accept_only_in_copied_values(markdown: &str) {
    for line in markdown.lines() {
        let lower = line.to_lowercase();
        if !lower.contains("accept") {
            continue;
        }
        assert!(line.starts_with("| AN-"), "tool prose says accept: {line}");
        for (index, cell) in line.trim_matches('|').split(" | ").enumerate() {
            if !matches!(index, 1 | 8 | 9 | 10) {
                assert!(
                    !cell.to_lowercase().contains("accept"),
                    "cell {index} is not copied from a record: {cell}"
                );
            }
        }
    }
}

#[trace("FR-164-AC-11", "NFR-035-AC-2", "NFR-035-AC-3")]
#[test]
fn the_output_never_calls_anything_accepted_and_has_no_verdict_keys() {
    let repo = corpus_with(&[
        ("AN-001", "valid", "closed-wont-fix-rationale"),
        ("AN-002", "valid", "rationale-resolved-with"),
        ("AN-003", "valid", "identified"),
        ("AN-004", "valid", "assessed"),
        ("AN-005", "valid", "two-hazards"),
        ("AN-006", "valid", "none"),
        ("AN-007", "invalid", "closed-unassessed"),
        ("AN-008", "valid", "rationale-assessed-without"),
    ]);
    let modes = [GateMode::Unassessed, GateMode::UnjustifiedImpacting];
    for modes in [&[][..], &modes[..]] {
        let report = report_with(&repo, modes);
        let markdown = report.to_markdown();
        assert!(
            markdown.to_lowercase().contains("accept"),
            "values are copied"
        );
        assert_accept_only_in_copied_values(&markdown);
        let json = report.to_json().unwrap().to_lowercase();
        for key in ["score", "rank", "verdict", "grade", "recommend", "priority"] {
            assert!(!json.contains(&format!("\"{key}")), "{key}");
        }
    }
    let clean = report_with(
        &corpus_with(&[("AN-001", "valid", "closed-wont-fix-rationale")]),
        &modes,
    );
    let markdown = clean.to_markdown();
    assert!(markdown.contains("No anomaly fails the evaluated modes"));
    assert_accept_only_in_copied_values(&markdown);
}
