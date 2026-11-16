// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Tests for the support input (FR-143).

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use super::{
    EndOfSupport, RecordSupport, Resolved, SupportErrorCode, SupportFile, SupportStatus,
    as_of_date, parse_support, record_support,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

fn entry(purl: &str, status: &str, end: &str, source: &str) -> Value {
    json!({"purl": purl, "support_status": status, "end_of_support": end, "source": source})
}

fn file(entries: &[Value]) -> SupportFile {
    parse_support(&Value::Array(entries.to_vec()).to_string()).unwrap()
}

fn refusal(entries: &[Value]) -> super::SupportError {
    parse_support(&Value::Array(entries.to_vec()).to_string()).unwrap_err()
}

fn end(text: &str) -> EndOfSupport {
    EndOfSupport::from_text(text).unwrap()
}

fn record(status: Option<&str>, end_text: Option<&str>) -> RecordSupport {
    RecordSupport {
        support_status: status.map(|s| SupportStatus::from_text(s).unwrap()),
        end_of_support: end_text.map(end),
    }
}

fn day(text: &str) -> time::Date {
    as_of_date(text).unwrap()
}

#[trace("FR-143-AC-1")]
#[test]
fn tc_1517_001_each_invalid_entry_refuses_the_whole_file_naming_it() {
    let good = entry(
        "pkg:cargo/ok@1",
        "supported",
        "2030-01-01",
        "https://x.test/p",
    );
    let cases = [
        (
            entry("pkg:cargo/a", "supported", "2030-01-01", ""),
            SupportErrorCode::EmptySource,
        ),
        (
            entry("pkg:cargo/a", "supported", "2030-01-01", "  "),
            SupportErrorCode::EmptySource,
        ),
        (
            json!({"purl": "pkg:cargo/a", "support_status": "supported", "end_of_support": "unknown"}),
            SupportErrorCode::MissingField,
        ),
        (
            entry("pkg:cargo/a", "maintained", "2030-01-01", "s"),
            SupportErrorCode::BadStatus,
        ),
        (
            entry("pkg:cargo/a", "supported", "soon", "s"),
            SupportErrorCode::BadEndOfSupport,
        ),
        (
            entry("pkg:cargo/a", "supported", "2026-02-30", "s"),
            SupportErrorCode::BadEndOfSupport,
        ),
        (
            entry("pkg:cargo/a", "supported", "2026-6-1", "s"),
            SupportErrorCode::BadEndOfSupport,
        ),
        (
            entry("cargo/a", "supported", "2030-01-01", "s"),
            SupportErrorCode::BadPurl,
        ),
    ];
    for (bad, code) in cases {
        let error = refusal(&[good.clone(), bad.clone()]);
        assert_eq!(error.code, code, "{bad}");
        assert_eq!(
            error.context.get("entry").map(String::as_str),
            Some("2"),
            "{bad}"
        );
    }
    let dup = refusal(&[
        entry("pkg:cargo/a@1", "supported", "2030-01-01", "s"),
        good,
        entry("pkg:cargo/a@1?arch=x#sub", "unsupported", "unknown", "t"),
    ]);
    assert_eq!(dup.code, SupportErrorCode::Duplicate);
    assert_eq!(
        dup.context.get("other_entry").map(String::as_str),
        Some("1")
    );
    assert_eq!(dup.context.get("entry").map(String::as_str), Some("3"));
    assert_eq!(
        parse_support("{\"purl\": 1}").unwrap_err().code,
        SupportErrorCode::NotAList
    );
}

#[trace("FR-143-AC-2")]
#[test]
fn tc_1517_002_an_unknown_record_value_takes_the_labelled_entry_value() {
    let f = file(&[entry(
        "pkg:cargo/a",
        "supported",
        "2030-05-01",
        "https://a.test/eol",
    )]);
    let got = f.evaluate(
        "pkg:cargo/a@2.0.0",
        record(Some("unknown"), Some("unknown")),
        day("2026-01-01"),
    );
    assert_eq!(
        got.end_of_support,
        Resolved::Supplied {
            value: end("2030-05-01"),
            source: "https://a.test/eol".to_owned()
        }
    );
    assert_eq!(
        got.support_status,
        Resolved::Supplied {
            value: SupportStatus::Supported,
            source: "https://a.test/eol".to_owned()
        }
    );
    let absent = f.evaluate(
        "pkg:cargo/a@2.0.0",
        RecordSupport::default(),
        day("2026-01-01"),
    );
    assert_eq!(absent, got);
}

#[trace("FR-143-AC-3")]
#[test]
fn tc_1517_003_a_stated_record_value_wins_and_a_difference_shows_both() {
    let f = file(&[entry(
        "pkg:cargo/a",
        "unsupported",
        "2026-01-01",
        "https://a.test/eol",
    )]);
    let got = f.evaluate(
        "pkg:cargo/a@1",
        record(Some("supported"), Some("2030-01-01")),
        day("2027-01-01"),
    );
    assert_eq!(
        got.end_of_support,
        Resolved::Disagree {
            record: end("2030-01-01"),
            supplied: end("2026-01-01"),
            source: "https://a.test/eol".to_owned(),
        }
    );
    assert!(got.end_of_support.disagrees() && got.support_status.disagrees());
    assert_eq!(got.end_of_support.authoritative(), Some(end("2030-01-01")));
    assert!(
        !got.past_end_of_support,
        "the record's date is authoritative"
    );
    let same = f.evaluate(
        "pkg:cargo/a@1",
        record(Some("unsupported"), Some("2026-01-01")),
        day("2025-01-01"),
    );
    assert!(!same.end_of_support.disagrees());
}

#[trace("FR-143-AC-4")]
#[test]
fn tc_1517_004_nothing_stated_stays_unknown_and_unflagged() {
    let f = file(&[entry("pkg:cargo/other", "supported", "2020-01-01", "s")]);
    for stated in [
        RecordSupport::default(),
        record(Some("unknown"), Some("unknown")),
    ] {
        let got = f.evaluate("pkg:cargo/a@1", stated, day("2099-01-01"));
        assert_eq!(got.support_status, Resolved::Unknown);
        assert_eq!(got.end_of_support, Resolved::Unknown);
        assert!(!got.past_end_of_support);
    }
    let unknown_entry = file(&[entry("pkg:cargo/a", "unknown", "unknown", "s")]);
    let got = unknown_entry.evaluate("pkg:cargo/a@1", RecordSupport::default(), day("2099-01-01"));
    assert_eq!(got.end_of_support, Resolved::Unknown);
}

#[trace("FR-143-AC-5")]
#[test]
fn tc_1517_005_past_end_of_support_is_judged_against_the_as_of_date() {
    let f = SupportFile::default();
    let as_of = day("2026-06-01");
    let flagged = |date: &str| {
        f.evaluate("pkg:cargo/a", record(None, Some(date)), as_of)
            .past_end_of_support
    };
    assert!(flagged("2026-05-31"));
    assert!(!flagged("2026-06-01"));
    assert!(!flagged("2026-06-02"));
    assert!(!flagged("unknown"));
    let from_entry = file(&[entry("pkg:cargo/a", "unsupported", "2026-05-31", "s")]);
    assert!(
        from_entry
            .evaluate("pkg:cargo/a", RecordSupport::default(), as_of)
            .past_end_of_support
    );
}

#[trace("FR-143-AC-6")]
#[test]
fn tc_1517_006_matching_ignores_qualifiers_and_prefers_the_versioned_entry() {
    let f = file(&[
        entry("pkg:cargo/a", "supported", "2030-01-01", "general"),
        entry("pkg:cargo/a@1", "unsupported", "2020-01-01", "versioned"),
        entry("pkg:cargo/b@2", "supported", "2030-01-01", "b2"),
    ]);
    let source = |purl: &str| f.matching(purl).map(|e| e.source.as_str());
    assert_eq!(source("pkg:cargo/a@1?arch=x#sub"), Some("versioned"));
    assert_eq!(source("pkg:cargo/a@7"), Some("general"));
    assert_eq!(source("pkg:cargo/a"), Some("general"));
    assert_eq!(
        source("pkg:cargo/b@3"),
        None,
        "another version matches nothing"
    );
    assert_eq!(source("pkg:cargo/b"), None);
    assert_eq!(source("pkg:npm/a@1"), None, "type differs");
    assert_eq!(source("pkg:cargo/ns/a@1"), None, "namespace differs");
    assert_eq!(source("pkg:cargo/c"), None);
    assert_eq!(source("pkg:cargo/b@2"), Some("b2"));
}

#[trace("FR-143-AC-9")]
#[test]
fn tc_1517_007_as_of_is_compared_by_its_utc_date() {
    let f = file(&[entry("pkg:cargo/a", "unsupported", "2026-05-31", "s")]);
    let past = |as_of: &str| {
        f.evaluate(
            "pkg:cargo/a",
            RecordSupport::default(),
            as_of_date(as_of).unwrap(),
        )
        .past_end_of_support
    };
    assert!(!past("2026-05-31T12:00:00Z"));
    assert!(!past("2026-05-31T23:59:59Z"));
    assert!(past("2026-06-01T00:00:00Z"));
    // 00:30 on 06-01 at +02:00 is still 05-31 in UTC.
    assert!(!past("2026-06-01T00:30:00+02:00"));
    // 23:30 on 05-31 at -02:00 is already 06-01 in UTC.
    assert!(past("2026-05-31T23:30:00-02:00"));
    assert_eq!(
        as_of_date("tomorrow").unwrap_err().code,
        SupportErrorCode::BadAsOf
    );
}

#[test]
fn tc_1517_008_a_record_frontmatter_is_read_without_defaults() {
    let front = |value: Value| value.as_object().unwrap().clone();
    let stated = record_support(&front(
        json!({"support_status": "supported", "end_of_support": "2027-03-04"}),
    ))
    .unwrap();
    assert_eq!(stated, record(Some("supported"), Some("2027-03-04")));
    assert_eq!(
        record_support(&front(json!({}))).unwrap(),
        RecordSupport::default()
    );
    let bad = record_support(&front(json!({"support_status": "bogus"}))).unwrap_err();
    assert_eq!(bad.code, SupportErrorCode::BadRecordValue);
    let md = "---\ntype: soup_record\npurl: pkg:cargo/a@1\nend_of_support: 2027-03-04\n---\n# a\n";
    let parsed = quoin_quire::validate::frontmatter(md).unwrap();
    assert_eq!(
        record_support(&parsed).unwrap().end_of_support,
        Some(end("2027-03-04"))
    );
}

#[test]
fn tc_1517_009_an_entry_with_another_key_is_refused_naming_it() {
    let mut stray = entry("pkg:cargo/a", "supported", "2030-01-01", "s");
    stray["sorce"] = json!("typo");
    let error = refusal(&[stray]);
    assert_eq!(error.code, SupportErrorCode::UnknownField);
    assert_eq!(error.context.get("key").map(String::as_str), Some("sorce"));
    assert_eq!(error.context.get("entry").map(String::as_str), Some("1"));
}

#[trace("FR-143-AC-6")]
#[test]
fn tc_1517_010_an_encoded_slash_stays_inside_its_segment() {
    let f = file(&[
        entry("pkg:x/a%2Fb", "supported", "2030-01-01", "name-with-slash"),
        entry("pkg:x/a/b", "unsupported", "2020-01-01", "namespaced"),
    ]);
    let source = |purl: &str| f.matching(purl).map(|e| e.source.as_str());
    assert_eq!(source("pkg:x/a%2Fb@1"), Some("name-with-slash"));
    assert_eq!(source("pkg:x/a/b@1"), Some("namespaced"));
    let same = file(&[entry("pkg:x/%40s/a", "supported", "2030-01-01", "enc")]);
    assert!(
        same.matching("pkg:x/@s/a").is_some(),
        "per-segment decoding still equates spellings"
    );
}

#[test]
fn tc_1517_011_error_codes_are_unique_and_namespaced() {
    let spellings: std::collections::BTreeSet<&str> =
        SupportErrorCode::ALL.iter().map(|c| c.as_str()).collect();
    assert_eq!(spellings.len(), SupportErrorCode::ALL.len());
    assert!(spellings.iter().all(|s| s.starts_with("soup.support_")));
    assert_eq!(SupportErrorCode::ALL.len(), 10);
}
