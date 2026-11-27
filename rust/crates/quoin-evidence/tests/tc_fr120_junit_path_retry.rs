// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! JUnit path and retry-attempt behavior for FR-120.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test fixtures use expect and indexing as the failure report"
)]

use ix_trace_rs::trace;
use quoin_evidence::Outcome;
use quoin_evidence::adapters::parse_junit;

#[trace("FR-120-AC-3")]
#[test]
fn junit_file_attribute_is_the_entry_path() {
    let result = parse_junit(
        r#"<testsuite><testcase classname="coverage" name="tests::coverage" file="crates/app/tests/coverage.rs"/></testsuite>"#,
    )
    .expect("the JUnit case parses");
    let entry = result.entries.first().expect("one entry");
    assert_eq!(entry.path.as_deref(), Some("crates/app/tests/coverage.rs"));
}

#[trace("FR-120-AC-6")]
#[test]
fn retry_failure_children_do_not_replace_the_final_attempt_outcome() {
    let cases = [
        ("flakyFailure", "<flakyFailure/>"),
        ("rerunFailure", "<rerunFailure/>"),
    ];
    for (name, earlier_attempt) in cases {
        let xml = format!(
            "<testsuite><testcase classname=\"t\" name=\"{name}\">{earlier_attempt}</testcase></testsuite>"
        );
        let entry = parse_junit(&xml)
            .expect("retry result parses")
            .entries
            .into_iter()
            .next()
            .expect("one case");
        assert_eq!(entry.outcome, Outcome::Pass, "{name}");
    }

    let final_failure = parse_junit(
        "<testsuite><testcase classname=\"t\" name=\"final-failure\"><rerunFailure/><failure/></testcase></testsuite>",
    )
    .expect("final failure parses");
    assert_eq!(final_failure.entries[0].outcome, Outcome::Fail);
}
