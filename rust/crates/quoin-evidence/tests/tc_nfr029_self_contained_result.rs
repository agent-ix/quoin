// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! A result stays interpretable from its serialized artifact alone (NFR-029).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test fixtures use expect and indexing as the failure report"
)]

use ix_trace_rs::trace;
use quoin_evidence::adapters::parse_junit;
use quoin_evidence::record::{RecordRequest, record_run};
use quoin_evidence::source::EvidenceSource;
use quoin_evidence::{MemoryEvidence, SuiteId};

#[trace("NFR-029-M-1")]
#[test]
fn tagged_result_artifact_contains_symbol_path_outcome_and_trace_ids() {
    let xml = r#"<testsuite><testcase classname="tests::coverage" name="tests::coverage" file="tests/coverage.rs"><properties><property name="trace" value="FR-120-AC-1"/></properties></testcase></testsuite>"#;
    let parsed = parse_junit(xml).expect("JUnit parses from the artifact bytes");
    let artifact = serde_json::to_value(&parsed.entries).expect("entries serialize");
    assert_eq!(artifact[0]["symbol"], "tests::coverage");
    assert_eq!(artifact[0]["path"], "tests/coverage.rs");
    assert_eq!(artifact[0]["outcome"], "pass");
    assert_eq!(artifact[0]["traceIds"][0], "FR-120-AC-1");
}

#[trace("NFR-029-M-2")]
#[test]
fn recording_a_result_uses_only_the_result_artifact() {
    let parsed = parse_junit(
        r#"<testsuite><testcase classname="suite" name="case" file="source/that/is/not/present.rs"><properties><property name="trace" value="FR-120-AC-1"/></properties></testcase></testsuite>"#,
    )
    .expect("the result file is sufficient to parse the case");
    let mut evidence = MemoryEvidence::new();
    let recorded = record_run(
        &mut evidence,
        &RecordRequest {
            suite: SuiteId::new("self-contained-result"),
            tool: "junit".to_owned(),
            evidence_kind: Some("test".to_owned()),
            lineage: None,
            timestamp: "2026-11-27T00:00:00Z".to_owned(),
            entries: parsed.entries,
        },
        &[],
    )
    .expect("recording does not require opening the reported source path");
    let stored = evidence
        .read(&recorded.run_path)
        .expect("record lookup does not need source files")
        .expect("run record is retained");
    let artifact: serde_json::Value =
        serde_json::from_str(&stored).expect("stored artifact parses");
    assert_eq!(artifact["entries"][0]["symbol"], "suite::case");
    assert_eq!(
        artifact["entries"][0]["path"],
        "source/that/is/not/present.rs"
    );
    assert_eq!(artifact["entries"][0]["outcome"], "pass");
    assert_eq!(artifact["entries"][0]["traceIds"][0], "FR-120-AC-1");
}
