// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `JUnit` XML → run entries.
//!
//! The `<testcase>` attributes and child elements this adapter reads are
//! deliberately a hand-written scan rather than a full XML parse. The subset is
//! fixed and small, the input is machine-written, and a dependency-free reader
//! keeps `quoin evidence record` free of a parser whose failure modes would
//! become quoin's. The port keeps that decision and the same patterns.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;

use super::AdapterResult;
use crate::error::EvidenceError;
use crate::types::{Outcome, RunEntry};

/// Compile a pattern that is a source literal.
///
/// Compilation is a build-time property of this file, not a runtime condition:
/// every pattern is exercised by the tests in this crate, so an unparseable one
/// fails the suite rather than reaching a caller.
#[allow(
    clippy::expect_used,
    reason = "patterns are module literals covered by this module's tests and the golden corpus; a failure here is a typo caught by the suite, not a runtime condition"
)]
fn literal_regex(pattern: &'static str) -> Regex {
    Regex::new(pattern).expect("adapter pattern literal must compile")
}

/// `/<testcase\b([^>]*?)(\/>|>([\s\S]*?)<\/testcase\s*>)/g`
static TESTCASE: LazyLock<Regex> =
    LazyLock::new(|| literal_regex(r"(?s)<testcase\b([^>]*?)(?:/>|>(.*?)</testcase\s*>)"));

/// `/([\w:.-]+)\s*=\s*"([^"]*)"/g`
static ATTRIBUTE: LazyLock<Regex> =
    LazyLock::new(|| literal_regex(r#"([\w:.\-]+)\s*=\s*"([^"]*)""#));

/// `/<property\b([^>]*)\/?>/g`
static PROPERTY: LazyLock<Regex> = LazyLock::new(|| literal_regex(r"<property\b([^>]*)/?>"));

static ERROR_CHILD: LazyLock<Regex> = LazyLock::new(|| literal_regex(r"<error\b"));
static FAILURE_CHILD: LazyLock<Regex> = LazyLock::new(|| literal_regex(r"<failure\b"));
static SKIPPED_CHILD: LazyLock<Regex> = LazyLock::new(|| literal_regex(r"<skipped\b"));

/// Split a `<testcase>`'s trace-id property value. `/[,\s]+/` in the retained
/// source.
static ID_SEPARATOR: LazyLock<Regex> = LazyLock::new(|| literal_regex(r"[,\s]+"));

fn attributes(fragment: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for capture in ATTRIBUTE.captures_iter(fragment) {
        let (Some(name), Some(value)) = (capture.get(1), capture.get(2)) else {
            continue;
        };
        // `Map.set` overwrites, so a repeated attribute keeps the LAST value.
        out.insert(name.as_str().to_owned(), decode(value.as_str()));
    }
    out
}

/// The five predefined XML entities. Attribute values carry no others.
///
/// The order is the retained source's and is load-bearing: `&amp;` is replaced
/// last, so `&amp;lt;` decodes to `&lt;` and not to `<`.
fn decode(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// `classname` + `name` → the qualified name the symbol extractor produces.
///
/// This is the join, and it is the whole job. A tool's own test name is not a
/// symbol identity: `quire` records `tests::tc001`, `JUnit` records
/// `classname="tests" name="tc001"`. Anything that skipped this would write a
/// store whose entries match no declared symbol, and every obligation would read
/// as unmatched while looking recorded.
///
/// Separator is `::` because that is what the Rust and Python extractors emit
/// for `container::member`. A `classname` that already contains the name — some
/// runners repeat it — is not doubled.
#[must_use]
pub fn qualified_name(classname: &str, name: &str) -> String {
    let container = classname.trim();
    let member = name.trim();
    if container.is_empty() {
        return member.to_owned();
    }
    if member.is_empty() {
        return container.to_owned();
    }
    if container == member {
        return member.to_owned();
    }
    if container.ends_with(&format!("::{member}")) || container.ends_with(&format!(".{member}")) {
        return container.replace('.', "::");
    }
    format!("{}::{member}", container.replace('.', "::"))
}

/// Trace ids named in a `<testcase>`'s own properties, if any.
fn trace_ids(body: &str) -> Option<Vec<String>> {
    let mut ids: Vec<String> = Vec::new();
    for capture in PROPERTY.captures_iter(body) {
        let Some(fragment) = capture.get(1) else {
            continue;
        };
        let attrs = attributes(fragment.as_str());
        let (Some(key), Some(value)) = (attrs.get("name"), attrs.get("value")) else {
            continue;
        };
        if key != "trace" && key != "traceIds" && key != "trace_ids" {
            continue;
        }
        for id in ID_SEPARATOR.split(value) {
            if !id.is_empty() {
                ids.push(id.to_owned());
            }
        }
    }
    (!ids.is_empty()).then_some(ids)
}

/// A `<testcase>`'s outcome, read from its children.
///
/// Order matters and is not arbitrary: `error` outranks `failure` outranks
/// `skipped`. A case carrying both an error and a failure is reported as the
/// more severe, because the milder reading is the one that would let a broken
/// run look merely red.
fn outcome(body: &str) -> Outcome {
    if ERROR_CHILD.is_match(body) {
        Outcome::Error
    } else if FAILURE_CHILD.is_match(body) {
        Outcome::Fail
    } else if SKIPPED_CHILD.is_match(body) {
        Outcome::Skip
    } else {
        Outcome::Pass
    }
}

/// Parse `JUnit` XML as emitted by most runners.
///
/// # Errors
///
/// Refuses input holding no `<testcase>` element with a non-empty symbol.
pub fn parse_junit(raw: &str) -> Result<AdapterResult, EvidenceError> {
    let mut entries: Vec<RunEntry> = Vec::new();
    for capture in TESTCASE.captures_iter(raw) {
        let attrs = capture
            .get(1)
            .map_or_else(BTreeMap::new, |m| attributes(m.as_str()));
        let body = capture.get(2).map_or("", |m| m.as_str());
        let empty = String::new();
        let symbol = qualified_name(
            attrs.get("classname").unwrap_or(&empty),
            attrs.get("name").unwrap_or(&empty),
        );
        if symbol.is_empty() {
            continue;
        }
        let mut entry = RunEntry::new(symbol, outcome(body));
        entry.trace_ids = trace_ids(body);
        entries.push(entry);
    }
    if entries.is_empty() {
        return Err(EvidenceError::adapter(
            "junit",
            "no <testcase> elements found — is this JUnit XML?",
        ));
    }
    // No evidenceKind: unit, integration and e2e suites all emit JUnit, so the
    // format does not say which this was. The suite registry does.
    Ok(AdapterResult::from_entries(entries))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{decode, parse_junit, qualified_name};
    use crate::types::Outcome;

    #[test]
    fn joins_classname_and_name_the_way_the_symbol_extractor_does() {
        assert_eq!(qualified_name("tests", "tc001"), "tests::tc001");
        assert_eq!(qualified_name("", "tc001"), "tc001");
        assert_eq!(qualified_name("tests", ""), "tests");
        assert_eq!(qualified_name("tc001", "tc001"), "tc001");
        assert_eq!(qualified_name("a.b.tc001", "tc001"), "a::b::tc001");
        assert_eq!(qualified_name("a::tc001", "tc001"), "a::tc001");
        assert_eq!(qualified_name("a.b", "tc001"), "a::b::tc001");
    }

    #[test]
    fn amp_is_decoded_last_so_a_double_escape_stays_escaped() {
        assert_eq!(decode("&amp;lt;"), "&lt;");
        assert_eq!(
            decode("&lt;a&gt; &quot;b&quot; &apos;c&apos;"),
            "<a> \"b\" 'c'"
        );
    }

    #[test]
    fn error_outranks_failure_outranks_skipped() {
        let xml = concat!(
            r#"<testsuite><testcase classname="t" name="a"><error/><failure/></testcase>"#,
            r#"<testcase classname="t" name="b"><failure/><skipped/></testcase>"#,
            r#"<testcase classname="t" name="c"><skipped/></testcase>"#,
            r#"<testcase classname="t" name="d"/></testsuite>"#,
        );
        let result = parse_junit(xml).unwrap();
        let outcomes: Vec<Outcome> = result.entries.iter().map(|e| e.outcome).collect();
        assert_eq!(
            outcomes,
            [Outcome::Error, Outcome::Fail, Outcome::Skip, Outcome::Pass]
        );
    }

    #[test]
    fn reads_trace_ids_from_the_three_accepted_property_names() {
        for key in ["trace", "traceIds", "trace_ids"] {
            let xml = format!(
                r#"<testcase classname="t" name="a"><properties><property name="{key}" value="FR-1-AC-1, FR-1-AC-2"/></properties></testcase>"#
            );
            let result = parse_junit(&xml).unwrap();
            assert_eq!(
                result.entries[0].trace_ids.as_deref(),
                Some(["FR-1-AC-1".to_owned(), "FR-1-AC-2".to_owned()].as_slice()),
                "{key}"
            );
        }
    }

    #[test]
    fn refuses_a_document_with_no_testcase() {
        let error = parse_junit("<testsuite/>").unwrap_err();
        assert_eq!(
            error.to_string(),
            "junit: no <testcase> elements found — is this JUnit XML?"
        );
    }
}
