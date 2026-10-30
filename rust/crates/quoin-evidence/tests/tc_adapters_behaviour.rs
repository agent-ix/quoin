// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Adapter behaviour (FR-033, FR-034, FR-036), asserted against inputs built in
//! each test. Nothing here replays a captured report or compares to a recorded
//! value: each case states what the adapter does with a small document of its own.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use ix_trace_rs::trace;
use quoin_evidence::Outcome;
use quoin_evidence::adapters::{
    ADAPTERS, AdapterSelection, adapter_names, parse_audit_script, parse_cargo_audit,
    parse_cargo_mutants, parse_junit, select_adapter,
};

fn chosen(adapter: Option<&str>, tool: Option<&str>) -> &'static str {
    select_adapter(AdapterSelection { adapter, tool })
        .unwrap()
        .name
}

#[trace("FR-033-AC-1")]
#[test]
fn tc_151_the_adapter_is_the_flag_then_the_tool_then_the_entries_shape() {
    assert_eq!(
        chosen(Some("cargo-mutants"), Some("pytest")),
        "cargo-mutants"
    );
    assert_eq!(chosen(None, Some("pytest 8")), "junit");
    assert_eq!(chosen(None, Some("something nobody claims")), "entries");
    assert_eq!(chosen(None, None), "entries");
}

#[trace("FR-033-AC-2")]
#[test]
fn tc_152_an_unknown_adapter_names_the_available_ones() {
    let error = select_adapter(AdapterSelection {
        adapter: Some("junut"),
        tool: Some("pytest"),
    })
    .unwrap_err()
    .to_string();
    assert!(error.contains("unknown adapter 'junut'"), "{error}");
    for name in adapter_names() {
        assert!(error.contains(name), "{name} missing from {error}");
    }
}

#[trace("FR-033-AC-4")]
#[test]
fn tc_154_classname_and_name_join_without_doubling_the_member() {
    let xml = concat!(
        r#"<testsuite><testcase classname="mod.sub" name="case"/>"#,
        r#"<testcase classname="mod.case" name="case"/>"#,
        r#"<testcase classname="" name="bare"/></testsuite>"#,
    );
    let symbols: Vec<String> = parse_junit(xml)
        .unwrap()
        .entries
        .iter()
        .map(|entry| entry.symbol.to_string())
        .collect();
    assert_eq!(symbols, ["mod::sub::case", "mod::case", "bare"]);
}

#[trace("FR-033-AC-5")]
#[test]
fn tc_155_every_outcome_class_and_declared_trace_ids_are_read() {
    let xml = concat!(
        r#"<testsuite><testcase classname="t" name="a"><error/><failure/></testcase>"#,
        r#"<testcase classname="t" name="b"><failure/><skipped/></testcase>"#,
        r#"<testcase classname="t" name="c"><skipped/></testcase>"#,
        r#"<testcase classname="t" name="d"><properties>"#,
        r#"<property name="trace" value="FR-1-AC-1, FR-1-AC-2"/></properties></testcase></testsuite>"#,
    );
    let entries = parse_junit(xml).unwrap().entries;
    let outcomes: Vec<Outcome> = entries.iter().map(|entry| entry.outcome).collect();
    assert_eq!(
        outcomes,
        [Outcome::Error, Outcome::Fail, Outcome::Skip, Outcome::Pass]
    );
    assert_eq!(
        entries[3].trace_ids.as_deref(),
        Some(["FR-1-AC-1".to_owned(), "FR-1-AC-2".to_owned()].as_slice())
    );
}

#[trace("FR-033-AC-6", "FR-033-CON-3")]
#[test]
fn tc_156_no_shipped_run_adapter_names_an_evidence_kind() {
    let xml = r#"<testsuite><testcase classname="t" name="a"/></testsuite>"#;
    assert_eq!(parse_junit(xml).unwrap().evidence_kind, None);
    let mutants = r#"{"outcomes":[{"scenario":{"Mutant":{"file":"a.rs","function":{"function_name":"f"}}},"summary":"CaughtMutant"}]}"#;
    assert_eq!(parse_cargo_mutants(mutants).unwrap().evidence_kind, None);
    let entries = r#"{"entries":[{"symbol":"a","outcome":"pass"}]}"#;
    let by_name = ADAPTERS.iter().find(|a| a.name == "entries").unwrap();
    assert_eq!((by_name.parse)(entries).unwrap().evidence_kind, None);
}

#[trace("FR-033-AC-7")]
#[test]
fn tc_157_no_testcase_is_an_error_and_an_unnamed_case_is_skipped() {
    assert!(parse_junit("<testsuite/>").is_err());
    assert!(parse_junit(r#"<testsuite><testcase time="1"/></testsuite>"#).is_err());
    let xml = r#"<testsuite><testcase time="1"/><testcase classname="t" name="kept"/></testsuite>"#;
    let entries = parse_junit(xml).unwrap().entries;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].symbol.to_string(), "t::kept");
}

fn mutant(file: &str, function: &str, summary: &str) -> String {
    format!(
        r#"{{"scenario":{{"Mutant":{{"file":"{file}","function":{{"function_name":"{function}"}}}}}},"summary":"{summary}"}}"#
    )
}

fn report(items: &[String]) -> String {
    format!(r#"{{"outcomes":[{}]}}"#, items.join(","))
}

#[trace("FR-033-AC-8")]
#[test]
fn tc_158_the_score_is_per_function_and_leaves_unviable_mutants_out() {
    let raw = report(&[
        mutant("a.rs", "f", "CaughtMutant"),
        mutant("a.rs", "f", "CaughtMutant"),
        mutant("a.rs", "f", "MissedMutant"),
        mutant("a.rs", "f", "Unviable"),
        mutant("b.rs", "g", "CaughtMutant"),
    ]);
    let entries = parse_cargo_mutants(&raw).unwrap().entries;
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].symbol.to_string(), "a.rs::f");
    // 2 caught of 3 viable; the unviable mutant is on neither side.
    assert!((entries[0].score.unwrap() - 2.0 / 3.0).abs() < 1e-12);
    assert!((entries[1].score.unwrap() - 1.0).abs() < f64::EPSILON);
}

#[trace("FR-033-AC-9")]
#[test]
fn tc_159_the_outcome_is_the_tools_classification_not_a_threshold() {
    // 99 caught and 1 missed scores 0.99, far above any plausible threshold,
    // and is still a failure because the tool found a surviving mutant.
    let mut items: Vec<String> = (0..99)
        .map(|_| mutant("a.rs", "f", "CaughtMutant"))
        .collect();
    items.push(mutant("a.rs", "f", "MissedMutant"));
    items.push(mutant("b.rs", "g", "CaughtMutant"));
    let entries = parse_cargo_mutants(&report(&items)).unwrap().entries;
    assert!(entries[0].score.unwrap() > 0.98);
    assert_eq!(entries[0].outcome, Outcome::Fail);
    assert_eq!(entries[1].outcome, Outcome::Pass);
}

#[trace("FR-033-AC-10")]
#[test]
fn tc_160_bad_mutation_reports_are_rejected_and_a_timeout_survives() {
    assert!(parse_cargo_mutants("not json").is_err());
    assert!(parse_cargo_mutants("{}").is_err());
    assert!(parse_cargo_mutants(r#"{"outcomes":[]}"#).is_err());
    // A mutant with no function to attribute it to measures nothing.
    let unattributable =
        r#"{"outcomes":[{"scenario":{"Mutant":{"file":"a.rs"}},"summary":"CaughtMutant"}]}"#;
    assert!(parse_cargo_mutants(unattributable).is_err());
    let timed_out = parse_cargo_mutants(&report(&[mutant("a.rs", "f", "Timeout")]))
        .unwrap()
        .entries;
    assert_eq!(timed_out[0].outcome, Outcome::Fail);
    assert!(timed_out[0].score.unwrap().abs() < f64::EPSILON);
}

#[trace("FR-033-AC-14", "FR-033-CON-4")]
#[test]
fn tc_164_the_normalized_entries_shape_is_accepted_with_no_adapter() {
    let selected = select_adapter(AdapterSelection::default()).unwrap();
    assert_eq!(selected.name, "entries");
    let result = (selected.parse)(
        r#"{"entries":[{"symbol":"a::b","outcome":"fail","traceIds":["FR-1-AC-1"]}]}"#,
    )
    .unwrap();
    assert_eq!(result.entries[0].outcome, Outcome::Fail);
    assert_eq!(
        result.entries[0].trace_ids.as_deref(),
        Some(["FR-1-AC-1".to_owned()].as_slice())
    );
}

#[trace("FR-034-AC-6")]
#[test]
fn tc_170_cargo_audit_reads_the_database_size_as_the_rule_count() {
    let raw = r#"{
        "database": {"advisory-count": 1234},
        "vulnerabilities": {"found": true, "count": 1, "list": [
            {"advisory": {"id": "RUSTSEC-2000-0001", "title": "bad"},
             "package": {"name": "left", "version": "1.0.0"}}]},
        "warnings": {"unmaintained": [
            {"advisory": {"id": "RUSTSEC-2000-0002", "title": "old"},
             "package": {"name": "right", "version": "2.0.0"}}]}
    }"#;
    let result = parse_cargo_audit(raw).unwrap();
    assert_eq!(result.rules_evaluated, Some(1234));
    let ids: Vec<&str> = result.findings.iter().map(|f| f.rule_id.as_str()).collect();
    assert_eq!(ids, ["RUSTSEC-2000-0001", "RUSTSEC-2000-0002"]);
    assert_eq!(result.findings[1].severity.as_deref(), Some("unmaintained"));
}

#[trace("FR-036-AC-1")]
#[test]
fn tc_197_an_all_passing_audit_run_is_rules_that_ran_and_found_nothing() {
    let result = parse_audit_script("check_a: OK\ncheck_b: OK\ncheck_c: OK\n").unwrap();
    assert_eq!(result.rules_evaluated, Some(3));
    assert_eq!(result.findings.len(), 0);
    // An absent scan is an error, not a clean run.
    assert!(parse_audit_script("nothing recognisable\n").is_err());
}
