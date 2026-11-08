// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Writing each test case's trace ids into the `JUnit` report.
//!
//! The report is rewritten by inserting a `<properties>` element right after
//! each matched `<testcase>` start tag and changing nothing else, so the
//! runner's own timings, outcomes and output survive byte for byte. The
//! `quoin evidence record` `JUnit` adapter reads the `trace` property and
//! nothing here judges a result.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use quoin_quire::coverage::TestTrace;
use regex::Regex;

use crate::targets::Target;

/// The start tag of a `<testcase>`, self-closing or not.
static TESTCASE: LazyLock<Regex> = LazyLock::new(|| compiled(r"<testcase\b([^>]*?)(/?)>"));

/// One `name="value"` attribute.
static ATTRIBUTE: LazyLock<Regex> = LazyLock::new(|| compiled(r#"([\w:.\-]+)\s*=\s*"([^"]*)""#));

#[allow(
    clippy::expect_used,
    reason = "patterns are module literals exercised by this module's tests; a failure is a typo caught by the suite, not a runtime condition"
)]
fn compiled(pattern: &'static str) -> Regex {
    Regex::new(pattern).expect("pattern literal must compile")
}

/// Trace ids by the `(binary id, test name)` nextest reports the test under.
#[derive(Debug, Default)]
pub(crate) struct TraceIndex {
    ids: BTreeMap<(String, String), BTreeSet<String>>,
}

impl TraceIndex {
    /// Join the engine's per-file relations to the targets the files compile into.
    pub(crate) fn build(traces: &[TestTrace], targets: &[Target]) -> Self {
        let mut ids: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
        for trace in traces {
            // A package's library and binary roots share `src/`, so a file
            // under it matches both. The library owns it: a binary only owns a
            // file no library does.
            let in_lib = targets
                .iter()
                .any(|t| t.is_library() && t.module_path(&trace.path).is_some());
            for target in targets {
                if in_lib && !target.is_library() {
                    continue;
                }
                let Some(mut path) = target.module_path(&trace.path) else {
                    continue;
                };
                path.push(trace.symbol.clone());
                ids.entry((target.binary_id().to_owned(), path.join("::")))
                    .or_default()
                    .insert(trace.trace_id.clone());
            }
        }
        Self { ids }
    }

    /// Tagged tests the index holds.
    pub(crate) fn len(&self) -> usize {
        self.ids.len()
    }
}

/// The annotated report and what the pass did.
#[derive(Debug)]
pub(crate) struct Annotated {
    /// The report with trace properties inserted.
    pub(crate) xml: String,
    /// Test cases that received a `trace` property.
    pub(crate) tagged: usize,
    /// Test cases in the report.
    pub(crate) cases: usize,
    /// Tagged tests the report holds no case for (not run, ignored or filtered).
    pub(crate) unrun: Vec<String>,
}

/// Insert a `trace` property into every test case the index knows.
pub(crate) fn annotate(xml: &str, index: &TraceIndex) -> Annotated {
    let mut out = String::with_capacity(xml.len() + 4096);
    let mut last = 0;
    let mut tagged = 0;
    let mut cases = 0;
    let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
    for start in TESTCASE.captures_iter(xml) {
        cases += 1;
        let (Some(whole), Some(attrs), Some(closer)) = (start.get(0), start.get(1), start.get(2))
        else {
            continue;
        };
        let key = case_key(attrs.as_str());
        let Some(ids) = index.ids.get(&key) else {
            continue;
        };
        tagged += 1;
        seen.insert(key);
        out.push_str(xml.get(last..whole.start()).unwrap_or_default());
        let joined = ids.iter().cloned().collect::<Vec<_>>().join(",");
        let property = format!(
            "<properties><property name=\"trace\" value=\"{}\"/></properties>",
            escape(&joined)
        );
        if closer.as_str().is_empty() {
            out.push_str(whole.as_str());
            out.push_str(&property);
        } else {
            out.push_str("<testcase");
            out.push_str(attrs.as_str());
            out.push('>');
            out.push_str(&property);
            out.push_str("</testcase>");
        }
        last = whole.end();
    }
    out.push_str(xml.get(last..).unwrap_or_default());
    let unrun = index
        .ids
        .keys()
        .filter(|key| !seen.contains(*key))
        .map(|(binary, name)| format!("{binary} {name}"))
        .collect();
    Annotated {
        xml: out,
        tagged,
        cases,
        unrun,
    }
}

/// `(classname, name)` of a test case start tag's attributes.
fn case_key(attrs: &str) -> (String, String) {
    let mut classname = String::new();
    let mut name = String::new();
    for capture in ATTRIBUTE.captures_iter(attrs) {
        let (Some(key), Some(value)) = (capture.get(1), capture.get(2)) else {
            continue;
        };
        match key.as_str() {
            "classname" => classname = unescape(value.as_str()),
            "name" => name = unescape(value.as_str()),
            _ => {}
        }
    }
    (classname, name)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;

    fn index(entries: &[(&str, &str, &[&str])]) -> TraceIndex {
        let mut ids = BTreeMap::new();
        for (binary, name, tags) in entries {
            ids.insert(
                ((*binary).to_owned(), (*name).to_owned()),
                tags.iter().map(|t| (*t).to_owned()).collect(),
            );
        }
        TraceIndex { ids }
    }

    /// Trace: FR-033-AC-15
    #[test]
    fn a_tagged_case_gains_a_trace_property_and_keeps_its_attributes() {
        let xml = r#"<testsuite><testcase name="a::t" classname="pkg" time="0.1"/></testsuite>"#;
        let done = annotate(xml, &index(&[("pkg", "a::t", &["FR-1-AC-1", "FR-1-AC-2"])]));
        assert_eq!(done.tagged, 1);
        assert_eq!(
            done.xml,
            r#"<testsuite><testcase name="a::t" classname="pkg" time="0.1"><properties><property name="trace" value="FR-1-AC-1,FR-1-AC-2"/></properties></testcase></testsuite>"#
        );
    }

    /// Trace: FR-033-AC-15
    #[test]
    fn a_failing_case_keeps_its_failure_child_after_the_property() {
        let xml = r#"<testcase name="t" classname="pkg"><failure message="x"/></testcase>"#;
        let done = annotate(xml, &index(&[("pkg", "t", &["FR-1-AC-1"])]));
        assert_eq!(
            done.xml,
            r#"<testcase name="t" classname="pkg"><properties><property name="trace" value="FR-1-AC-1"/></properties><failure message="x"/></testcase>"#
        );
    }

    /// Trace: FR-033-AC-15
    #[test]
    fn untagged_cases_are_left_alone_and_unrun_tagged_tests_are_named() {
        let xml = r#"<testcase name="other" classname="pkg"/>"#;
        let done = annotate(xml, &index(&[("pkg", "gone", &["FR-1-AC-1"])]));
        assert_eq!(done.xml, xml);
        assert_eq!((done.tagged, done.cases), (0, 1));
        assert_eq!(done.unrun, ["pkg gone"]);
    }

    /// Trace: FR-033-AC-15
    #[test]
    fn a_file_under_a_shared_src_belongs_to_the_library_not_the_binary() {
        let lib = crate::targets::from_metadata(
            r#"{"packages":[{"name":"p","targets":[
              {"name":"p","kind":["lib"],"src_path":"/r/p/src/lib.rs"},
              {"name":"t","kind":["bin"],"src_path":"/r/p/src/main.rs"}]}]}"#,
            std::path::Path::new("/r"),
        )
        .unwrap();
        let traces = [TestTrace {
            path: "p/src/ops.rs".to_owned(),
            symbol: "tests::x".to_owned(),
            trace_id: "FR-1-AC-1".to_owned(),
        }];
        let built = TraceIndex::build(&traces, &lib);
        assert_eq!(built.len(), 1);
        assert!(
            built
                .ids
                .contains_key(&("p".to_owned(), "ops::tests::x".to_owned()))
        );
    }

    /// Trace: FR-033-AC-15
    #[test]
    fn the_annotated_report_binds_through_the_evidence_junit_adapter_shape() {
        // The adapter reads `<property name="trace" value="…">` inside the case.
        let xml = r#"<testcase name="t" classname="pkg"/>"#;
        let done = annotate(xml, &index(&[("pkg", "t", &["FR-9-AC-3"])]));
        assert!(
            done.xml
                .contains(r#"<property name="trace" value="FR-9-AC-3"/>"#)
        );
    }
}
