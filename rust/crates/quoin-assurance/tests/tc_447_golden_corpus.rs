// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The frozen golden corpus, replayed through the library (quoin#447).
//!
//! # Why a checked-in corpus rather than the difftest
//!
//! `quoin-difftest` runs one request through the retained `src/assurance/` and
//! through `quoin-core` and compares the two. That works only while the
//! TypeScript is there to answer, and quoin#447 deletes it. FR-101 AC-5 is
//! explicit that no live Node runtime may remain in the Rust test path after
//! the cutover, so the oracle has to stop being a process and start being
//! bytes.
//!
//! `tests/golden/cases.json` holds hand-authored inputs and
//! `tests/golden/expected.json` holds the verdicts CAPTURED from the retained
//! TypeScript by a generator that was deleted along with its subject.
//! Nothing in this file executes Node: it reads two files and calls the
//! port. `tests/golden/PROVENANCE.md` records what produced the bytes,
//! when, from which revision, and the hashes that let a reader re-derive
//! them at that revision.
//!
//! This test runs the corpus through the crate's own functions.
//! `quoin-core`'s `tc_447_assurance_boundary` runs the same corpus through the
//! real binary over a pipe, so the two together say the seam did not change
//! the answer.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

/// The hand-authored half of the corpus.
#[derive(Debug, Deserialize)]
struct Corpus {
    /// The one complete argument every `parse_argument` case is a variation
    /// of. An authored argument is twelve required keys deep, and forty full
    /// copies would hide the single field each case is actually about.
    argument_base: Map<String, Value>,
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    name: String,
    op: String,
    /// Marks an input the REQUEST SCHEMA refuses before the ported logic runs.
    /// The retained TypeScript's answer to it is still captured — see
    /// `PROVENANCE.md` — so the divergence is recorded, not hidden.
    #[serde(default)]
    boundary: Option<String>,
    #[serde(default)]
    input: Option<Value>,
    #[serde(default)]
    overrides: Option<Map<String, Value>>,
    #[serde(default)]
    remove: Option<Vec<String>>,
    #[serde(default)]
    review_by: Option<String>,
}

/// The captured half. `ok` is the verdict; `error` is recorded for a reader
/// and deliberately never asserted — quoin#373 holds that verdicts are
/// contractual and error prose is not.
#[derive(Debug, Deserialize)]
struct Expectations {
    cases: Vec<Expectation>,
}

#[derive(Debug, Deserialize)]
struct Expectation {
    name: String,
    ok: bool,
    #[serde(default)]
    value: Value,
}

fn golden_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden"))
}

fn read<T: for<'de> Deserialize<'de>>(file: &str) -> T {
    serde_json::from_str(&std::fs::read_to_string(golden_dir().join(file)).unwrap()).unwrap()
}

/// The request a case hands to the port, rebuilt exactly as the capture
/// rebuilt it.
///
/// `remove` DELETES a key where `overrides` could only set one, and the
/// difference is load-bearing: an explicit `null` is itself under test, so
/// "absent" and "null" have to stay distinguishable.
fn request_for(corpus: &Corpus, case: &Case) -> Value {
    if case.op != "parse_argument" || case.input.is_some() {
        return case.input.clone().unwrap_or(Value::Null);
    }
    let mut argument = corpus.argument_base.clone();
    for key in case.remove.iter().flatten() {
        argument.remove(key);
    }
    if let Some(review_by) = &case.review_by {
        argument.insert(
            "assumptions".to_owned(),
            serde_json::json!([{
                "id": "ASM-900",
                "statement": "The reviewed clause set is stable.",
                "owner": "release-owner",
                "status": "accepted",
                "review_by": review_by,
            }]),
        );
    }
    for (key, value) in case.overrides.iter().flatten() {
        argument.insert(key.clone(), value.clone());
    }
    Value::Object(argument)
}

/// Every captured TypeScript verdict, reproduced by the port.
///
/// Trace: FR-040, FR-047, FR-101
/// Provenance: quoin#447
#[test]
fn tc_447_500_the_port_reproduces_every_captured_typescript_verdict() {
    let corpus: Corpus = read("cases.json");
    let expected: Expectations = read("expected.json");
    assert_eq!(corpus.cases.len(), expected.cases.len());
    // A comparison over an empty population is green and says nothing. The
    // corpus is 128 cases; without this an unreadable path would read as a
    // pass.
    assert!(
        corpus.cases.len() >= 120,
        "the golden corpus came back with {} cases; it is not being read",
        corpus.cases.len()
    );

    for (case, expectation) in corpus.cases.iter().zip(&expected.cases) {
        assert_eq!(case.name, expectation.name);
        // Checked before the dispatch rather than in a fallthrough arm: an op
        // the match does not know would otherwise fall through to nothing, and
        // a case that runs no assertion is a case that passes for free.
        assert!(
            matches!(
                case.op.as_str(),
                "requirement_of" | "build_case" | "render_case" | "parse_argument"
            ),
            "{}: unknown op {}",
            case.name,
            case.op
        );
        let request = request_for(&corpus, case);

        // The request schema refuses these before the ported logic is reached,
        // so what this test can assert is the refusal itself. `requirement_of`'s
        // request type lives in `quoin-core`, so its two refusals are asserted
        // by the boundary test and not here.
        if case.boundary.as_deref() == Some("refused") {
            match case.op.as_str() {
                "build_case" => assert!(
                    serde_json::from_value::<quoin_assurance::CaseInput>(request).is_err(),
                    "{}: the request schema accepted an input it refuses",
                    case.name
                ),
                "render_case" => assert!(
                    serde_json::from_value::<quoin_assurance::RenderableCase>(request).is_err(),
                    "{}: the request schema accepted an input it refuses",
                    case.name
                ),
                _ => {}
            }
            continue;
        }

        match case.op.as_str() {
            "requirement_of" => {
                let id = request["obligation_id"].as_str().unwrap();
                assert_eq!(
                    serde_json::json!({ "requirement": quoin_assurance::requirement_of(id) }),
                    expectation.value,
                    "{} diverged from the captured TypeScript verdict",
                    case.name
                );
            }
            "build_case" => {
                let input: quoin_assurance::CaseInput = serde_json::from_value(request).unwrap();
                assert_eq!(
                    serde_json::to_value(quoin_assurance::build_case(&input)).unwrap(),
                    expectation.value,
                    "{} diverged from the captured TypeScript verdict",
                    case.name
                );
            }
            "render_case" => {
                let assurance: quoin_assurance::RenderableCase =
                    serde_json::from_value(request).unwrap();
                assert_eq!(
                    serde_json::json!({ "rendered": quoin_assurance::render_case(&assurance) }),
                    expectation.value,
                    "{} diverged from the captured TypeScript verdict",
                    case.name
                );
            }
            "parse_argument" => {
                let parsed = quoin_assurance::parse_assurance_argument(&request);
                assert_eq!(
                    parsed.is_ok(),
                    expectation.ok,
                    "{} disagreed with the captured TypeScript verdict",
                    case.name
                );
                if let Ok(argument) = parsed {
                    assert_eq!(
                        serde_json::to_value(argument).unwrap(),
                        expectation.value,
                        "{} diverged from the captured TypeScript verdict",
                        case.name
                    );
                }
            }
            _ => {}
        }
    }
}

/// The corpus covers all four retired difftest operations, in numbers.
///
/// The anti-vacuity guard above proves the file was read; this one proves it
/// was authored. A corpus that drifted into 128 `requirement_of` cases would
/// pass every assertion above and test one function.
///
/// Trace: FR-101
/// Provenance: quoin#447
#[test]
fn tc_447_501_the_corpus_exercises_all_four_retired_operations() {
    let corpus: Corpus = read("cases.json");
    for (op, least) in [
        ("requirement_of", 12),
        ("build_case", 30),
        ("render_case", 25),
        ("parse_argument", 50),
    ] {
        let found = corpus.cases.iter().filter(|c| c.op == op).count();
        assert!(
            found >= least,
            "the corpus holds {found} {op} cases; it was authored with at least {least}"
        );
    }
}

/// Every case carries the requirement id it exists for.
///
/// Trace: FR-101
/// Provenance: quoin#447
#[test]
fn tc_447_502_every_case_names_a_requirement_and_a_unique_name() {
    #[derive(Deserialize)]
    struct Covered {
        cases: Vec<CoveredCase>,
    }
    #[derive(Deserialize)]
    struct CoveredCase {
        name: String,
        covers: Vec<String>,
    }

    let corpus: Covered = read("cases.json");
    let mut names: Vec<&str> = corpus.cases.iter().map(|c| c.name.as_str()).collect();
    names.sort_unstable();
    let total = names.len();
    names.dedup();
    assert_eq!(names.len(), total, "the corpus holds a duplicate case name");
    for case in &corpus.cases {
        assert!(
            !case.covers.is_empty(),
            "{} names no requirement it carries",
            case.name
        );
    }
}
