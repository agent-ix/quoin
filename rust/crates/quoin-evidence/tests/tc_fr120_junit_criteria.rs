// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! JUnit trace marks, paths, outcomes, and retry history (FR-120).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report"
)]

use ix_trace_rs::trace;
use quoin_evidence::SuiteId;
use quoin_evidence::adapters::{parse_entries, parse_junit};
use quoin_evidence::record::{RecordRequest, record_run};
use quoin_evidence::source::MemoryEvidence;
use quoin_evidence::store::read_bindings;
use quoin_evidence::types::{Outcome, RunEntry};

const TRACE_ID: &str = "FR-001-AC-1";

#[trace("FR-120-AC-1")]
#[test]
fn repeated_trace_properties_accumulate_ids_split_on_whitespace_and_commas() {
    let xml = concat!(
        r#"<testcase classname="tests" name="tc001"><properties>"#,
        r#"<property name="trace" value="FR-001-AC-1,TC-041"/>"#,
        "\n\t",
        r#"<property name="trace" value="FR-002-AC-3  FR-003-AC-2"/>"#,
        r#"</properties></testcase>"#,
    );
    let result = parse_junit(xml).unwrap();
    assert_eq!(
        result.entries[0].trace_ids.as_deref(),
        Some(
            [
                "FR-001-AC-1".to_owned(),
                "TC-041".to_owned(),
                "FR-002-AC-3".to_owned(),
                "FR-003-AC-2".to_owned()
            ]
            .as_slice()
        )
    );
}

#[trace("FR-120-AC-2")]
#[test]
fn equal_classname_and_name_preserves_dots_and_spaces() {
    for name in ["suite.tc dotted", "tests::case with spaces"] {
        let xml = format!(r#"<testcase classname="{name}" name="{name}"/>"#);
        let result = parse_junit(&xml).unwrap();
        assert_eq!(result.entries[0].symbol.as_str(), name);
    }
}

#[trace("FR-120-AC-3")]
#[test]
fn junit_file_and_normalized_path_are_retained_on_the_entry() {
    let junit =
        parse_junit(r#"<testcase classname="tests" name="tc001" file="src/lib.rs"/>"#).unwrap();
    assert_eq!(junit.entries[0].path.as_deref(), Some("src/lib.rs"));

    let normalized = parse_entries(
        r#"{"entries":[{"symbol":"tests::tc001","outcome":"pass","path":"src/lib.rs"}]}"#,
    )
    .unwrap();
    assert_eq!(normalized.entries[0].path.as_deref(), Some("src/lib.rs"));
}

fn criterion() -> quoin_quire_types::Obligation {
    quoin_quire_types::Obligation {
        id: TRACE_ID.to_owned(),
        identity: None,
        document: None,
        statement: "a criterion used only to prove a failed test does not bind".to_owned(),
        target_ids: None,
        method: Some("Test".to_owned()),
        criticality: None,
        parameters: None,
    }
}

fn does_not_bind(entry: RunEntry) {
    let mut source = MemoryEvidence::new();
    let result = record_run(
        &mut source,
        &RecordRequest {
            suite: SuiteId::new("junit-suite"),
            tool: "junit test runner".to_owned(),
            evidence_kind: None,
            lineage: None,
            timestamp: "2026-11-26T12:00:00Z".to_owned(),
            entries: vec![entry],
        },
        &[criterion()],
    )
    .unwrap();
    assert!(result.bound.is_empty());
    assert!(read_bindings(&source).unwrap().bindings.is_empty());
}

#[trace("FR-120-AC-5")]
#[test]
fn nonpassing_tagged_cases_and_cases_without_trace_never_bind() {
    for (body, expected) in [
        ("<failure/>", Outcome::Fail),
        ("<error/>", Outcome::Error),
        ("<skipped/>", Outcome::Skip),
    ] {
        let xml = format!(
            r#"<testcase classname="tests" name="tc001">{body}<properties><property name="trace" value="{TRACE_ID}"/></properties></testcase>"#
        );
        let mut parsed = parse_junit(&xml).unwrap();
        assert_eq!(parsed.entries[0].outcome, expected);
        assert_eq!(
            parsed.entries[0].trace_ids.as_deref(),
            Some([TRACE_ID.to_owned()].as_slice())
        );
        does_not_bind(parsed.entries.remove(0));
    }

    let parsed =
        parse_junit(r#"<testcase classname="tests" name="tc001"><failure/></testcase>"#).unwrap();
    assert_eq!(parsed.entries[0].outcome, Outcome::Fail);
    assert_eq!(parsed.entries[0].trace_ids, None);
    does_not_bind(parsed.entries.into_iter().next().unwrap());
}

#[trace("FR-120-AC-6")]
#[test]
fn retry_history_keeps_the_final_case_outcome() {
    let recovered =
        parse_junit(r#"<testcase classname="tests" name="recovered"><flakyFailure/></testcase>"#)
            .unwrap();
    assert_eq!(recovered.entries[0].outcome, Outcome::Pass);

    let still_failing = parse_junit(
        r#"<testcase classname="tests" name="still_failing"><rerunFailure/><failure/></testcase>"#,
    )
    .unwrap();
    assert_eq!(still_failing.entries[0].outcome, Outcome::Fail);
}
