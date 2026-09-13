// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Adapter parity with the retained TypeScript (quoin#456).
//!
//! `tests/golden/cases.json` is hand-authored input; `tests/golden/expected.json`
//! was captured once from `src/evidence/adapters/` and is the only oracle
//! consulted here. No Node process runs in this lane (FR-101-AC-5). See
//! `tests/golden/PROVENANCE.md`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;
use std::path::{Path, PathBuf};

use quoin_evidence::adapters::{
    ADAPTERS, AdapterResult, FINDING_ADAPTERS, FindingResult, UnrepresentedResult,
};
use quoin_evidence::{EvidenceError, Finding, RunEntry};
use serde::Deserialize;
use serde_json::{Map, Value, json};

#[derive(Debug, Deserialize)]
struct Corpus {
    cases: Vec<CaseInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct CaseInput {
    name: String,
    adapter: String,
    shape: Shape,
    /// The input, held in the corpus file itself.
    inline: Option<String>,
    /// The input, held in a checked-in real tool output, repository-relative.
    file: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Shape {
    Run,
    Finding,
}

#[derive(Debug, Deserialize)]
struct Expectations {
    cases: Vec<CaseExpectation>,
}

#[derive(Debug, Deserialize)]
struct CaseExpectation {
    name: String,
    /// The parse the TypeScript produced, when it accepted the input.
    ok: Option<Value>,
    /// The `AdapterError` message it threw, when it refused.
    err: Option<String>,
}

fn golden_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden"))
}

/// crates/quoin-evidence -> crates -> rust -> repository root.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

fn load() -> (Corpus, Expectations) {
    let corpus: Corpus =
        serde_json::from_str(&fs::read_to_string(golden_dir().join("cases.json")).unwrap())
            .unwrap();
    let expected: Expectations =
        serde_json::from_str(&fs::read_to_string(golden_dir().join("expected.json")).unwrap())
            .unwrap();
    assert_eq!(
        corpus.cases.len(),
        expected.cases.len(),
        "the corpus and the capture disagree on how many cases exist"
    );
    (corpus, expected)
}

fn input_of(case: &CaseInput) -> String {
    match (&case.inline, &case.file) {
        (Some(inline), _) => inline.clone(),
        (None, Some(file)) => fs::read_to_string(repo_root().join(file))
            .unwrap_or_else(|why| panic!("case {}: cannot read {file}: {why}", case.name)),
        (None, None) => panic!("case {} names neither inline nor file", case.name),
    }
}

fn insert_opt(map: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        map.insert(key.to_owned(), value);
    }
}

/// The TypeScript `RunEntry` object literal, with absent optionals omitted —
/// the `...(x === undefined ? {} : {x})` idiom the adapters use.
fn entry_json(entry: &RunEntry) -> Value {
    let mut map = Map::new();
    map.insert("symbol".to_owned(), json!(entry.symbol));
    map.insert("outcome".to_owned(), json!(entry.outcome.as_str()));
    insert_opt(&mut map, "score", entry.score.map(|s| json!(s)));
    insert_opt(&mut map, "metric", entry.metric.clone().map(Value::String));
    insert_opt(
        &mut map,
        "traceIds",
        entry.trace_ids.clone().map(|ids| json!(ids)),
    );
    insert_opt(&mut map, "config", entry.config.clone().map(|c| json!(c)));
    Value::Object(map)
}

fn unrepresented_json(item: &UnrepresentedResult) -> Value {
    json!({ "symbol": item.symbol, "state": item.state, "reason": item.reason })
}

fn finding_json(finding: &Finding) -> Value {
    let mut map = Map::new();
    map.insert("ruleId".to_owned(), json!(finding.rule_id));
    insert_opt(
        &mut map,
        "severity",
        finding.severity.clone().map(Value::String),
    );
    insert_opt(
        &mut map,
        "message",
        finding.message.clone().map(Value::String),
    );
    insert_opt(&mut map, "path", finding.path.clone().map(Value::String));
    insert_opt(&mut map, "line", finding.line.map(|l| json!(l)));
    insert_opt(
        &mut map,
        "traceIds",
        finding.trace_ids.clone().map(|ids| json!(ids)),
    );
    Value::Object(map)
}

fn run_result_json(result: &AdapterResult) -> Value {
    let mut map = Map::new();
    map.insert(
        "entries".to_owned(),
        Value::Array(result.entries.iter().map(entry_json).collect()),
    );
    insert_opt(
        &mut map,
        "unrepresented",
        result
            .unrepresented
            .as_ref()
            .map(|items| Value::Array(items.iter().map(unrepresented_json).collect())),
    );
    insert_opt(
        &mut map,
        "evidenceKind",
        result.evidence_kind.clone().map(Value::String),
    );
    Value::Object(map)
}

fn finding_result_json(result: &FindingResult) -> Value {
    let mut map = Map::new();
    map.insert(
        "findings".to_owned(),
        Value::Array(result.findings.iter().map(finding_json).collect()),
    );
    insert_opt(&mut map, "tool", result.tool.clone().map(Value::String));
    insert_opt(
        &mut map,
        "ruleset",
        result.ruleset.clone().map(Value::String),
    );
    insert_opt(
        &mut map,
        "rulesEvaluated",
        result.rules_evaluated.map(|n| json!(n)),
    );
    Value::Object(map)
}

fn parse_case(case: &CaseInput, raw: &str) -> Result<Value, EvidenceError> {
    match case.shape {
        Shape::Run => {
            let adapter = ADAPTERS
                .iter()
                .find(|a| a.name == case.adapter)
                .unwrap_or_else(|| panic!("case {} names unknown adapter", case.name));
            (adapter.parse)(raw).map(|r| run_result_json(&r))
        }
        Shape::Finding => {
            let adapter = FINDING_ADAPTERS
                .iter()
                .find(|a| a.name == case.adapter)
                .unwrap_or_else(|| panic!("case {} names unknown adapter", case.name));
            (adapter.parse)(raw).map(|r| finding_result_json(&r))
        }
    }
}

/// Rewrites every integral JSON number as an integer, in place.
///
/// `JSON.stringify(1)` writes `1` where `serde_json` writes `1.0` for the same
/// `f64`. That is a *representation* difference and it never reaches a record:
/// `quoin-store` renders numbers with the `ECMAScript` algorithm (`ryu-js`, see
/// its `tc_380_integral_doubles_print_without_a_fraction`), so the store writes
/// `1` for this score exactly as the TypeScript did. Normalising both sides
/// keeps this parity test about the adapters' VALUES and leaves number
/// formatting to the crate that owns it (FR-100-CON-4).
fn normalise_numbers(value: &mut Value) {
    match value {
        Value::Number(number) => {
            if let Some(float) = number.as_f64()
                && float.fract() == 0.0
                && float.abs() < 9.007_199_254_740_992e15
            {
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "guarded: the value is integral and inside the i64 range"
                )]
                let integral = float as i64;
                *value = Value::Number(integral.into());
            }
        }
        Value::Array(items) => items.iter_mut().for_each(normalise_numbers),
        Value::Object(map) => map.values_mut().for_each(normalise_numbers),
        _ => {}
    }
}

/// The comparable prefix of a refusal message.
///
/// Where the TypeScript embeds a Node `JSON.parse` detail — "Expected property
/// name or '}' in JSON at position 1" — the text after the marker is V8's, not
/// quoin's, and no Rust parser reproduces it. Everything up to and including
/// the marker is quoin's own and IS compared; the tail is not. Every other
/// message is compared whole.
fn comparable(message: &str) -> &str {
    for marker in ["is not JSON: ", "not JSON: "] {
        if let Some(at) = message.find(marker) {
            return &message[..at + marker.len()];
        }
    }
    message
}

/// Runs every case whose adapter is in `adapters` and asserts parity with the
/// capture, returning (accepted, refused) counts so the caller can floor them.
fn parity_over(adapters: &[&str]) -> (usize, usize) {
    let (corpus, expected) = load();
    let mut accepted = 0;
    let mut refused = 0;
    for (case, expectation) in corpus.cases.iter().zip(expected.cases.iter()) {
        assert_eq!(
            case.name, expectation.name,
            "the corpus and the capture are out of order"
        );
        if !adapters.contains(&case.adapter.as_str()) {
            continue;
        }
        let raw = input_of(case);
        match (parse_case(case, &raw), &expectation.ok, &expectation.err) {
            (Ok(mut actual), Some(want), None) => {
                normalise_numbers(&mut actual);
                let mut want = want.clone();
                normalise_numbers(&mut want);
                assert_eq!(actual, want, "case {}: parse differs", case.name);
                accepted += 1;
            }
            (Err(actual), None, Some(want)) => {
                assert_eq!(
                    comparable(&actual.to_string()),
                    comparable(want),
                    "case {}: refusal text differs",
                    case.name
                );
                refused += 1;
            }
            (Ok(actual), None, Some(want)) => panic!(
                "case {}: Rust accepted {actual} where TypeScript refused with {want}",
                case.name
            ),
            (Err(actual), Some(want), None) => panic!(
                "case {}: Rust refused with {actual} where TypeScript returned {want}",
                case.name
            ),
            _ => panic!("case {}: the capture records neither ok nor err", case.name),
        }
    }
    (accepted, refused)
}

/// Both halves of a criterion's population are non-empty: a parity run over
/// only-accepting or only-refusing cases carries no refusal behaviour at all.
fn assert_both_halves(counts: (usize, usize), what: &str) {
    let (accepted, refused) = counts;
    assert!(accepted > 0, "{what}: no accepting case ran — vacuous");
    assert!(refused > 0, "{what}: no refusing case ran — vacuous");
}

/// The normalized `entries` shape still parses with no adapter selected, and a
/// malformed normalized payload is reported against the `entries` adapter.
///
/// Trace: FR-033-AC-2, FR-033-AC-14
/// Provenance: quoin#456
#[test]
fn tc_456_001_entries_adapter_parity() {
    assert_both_halves(parity_over(&["entries"]), "entries");
}

/// `classname` + `name` become the qualified name the extractor emits, every
/// outcome class and declared trace id is read, XML entities are decoded,
/// separator debris is dropped, and input carrying no testcase is refused
/// rather than recorded as an empty run.
///
/// Trace: FR-033-AC-4, FR-033-AC-5, FR-033-AC-6, FR-033-AC-7, FR-033-CON-3, FR-033-CON-4
/// Provenance: quoin#456
#[test]
fn tc_456_002_junit_adapter_parity() {
    assert_both_halves(parity_over(&["junit"]), "junit");
}

/// The native mutation score, the tool's own classification rather than a
/// threshold, a timed-out mutant counted as survived, and the refusals for a
/// malformed or empty report.
///
/// Trace: FR-033-AC-8, FR-033-AC-9, FR-033-AC-10
/// Provenance: quoin#456
#[test]
fn tc_456_003_cargo_mutants_adapter_parity() {
    assert_both_halves(parity_over(&["cargo-mutants"]), "cargo-mutants");
}

/// The remaining run-shaped adapters over real checked-in tool output: SBOM
/// identity precedence, agent-eval pass rates, contract-conformance rows and
/// the differential report's unrepresented `unsupported` state.
///
/// Trace: FR-030-AC-17, FR-033-AC-3
/// Provenance: quoin#456
#[test]
fn tc_456_004_remaining_run_adapters_parity() {
    assert_both_halves(
        parity_over(&[
            "sbom",
            "agent-eval",
            "contract-conformance",
            "differential-report",
        ]),
        "run adapters over real tool output",
    );
}

/// The finding-shaped adapters: SARIF, the architecture-conformance audit
/// script, and `cargo audit --json`, including the `None`-versus-`Some(0)`
/// `rulesEvaluated` distinction FR-034 turns on.
///
/// The real captured outputs matter to two criteria by name: FR-034-AC-6 asks
/// for `cargo audit --json` as the tool actually emits it, and FR-036-AC-1 for
/// a real all-passing architecture-conformance run read as rules that ran and
/// found nothing.
///
/// Trace: FR-033-AC-1, FR-033-AC-3, FR-034-AC-6, FR-036-AC-1
/// Provenance: quoin#456, quoin#458
#[test]
fn tc_456_005_finding_adapters_parity() {
    assert_both_halves(
        parity_over(&["sarif", "audit-script", "cargo-audit"]),
        "finding adapters",
    );
}

/// Every case in the corpus is exercised by exactly one of the tests above,
/// and every registered adapter appears in the corpus.
///
/// Without this, adding an adapter or a case silently narrows the population
/// the parity tests above measure, and every one of them keeps passing.
///
/// Trace: FR-033-AC-3
/// Provenance: quoin#456
#[test]
fn tc_456_006_corpus_covers_every_registered_adapter() {
    let (corpus, _) = load();
    assert!(!corpus.cases.is_empty(), "the corpus is empty");
    for adapter in ADAPTERS.iter().map(|a| a.name) {
        assert!(
            corpus.cases.iter().any(|c| c.adapter == adapter),
            "no golden case exercises the {adapter} adapter"
        );
    }
    for adapter in FINDING_ADAPTERS.iter().map(|a| a.name) {
        assert!(
            corpus.cases.iter().any(|c| c.adapter == adapter),
            "no golden case exercises the {adapter} finding adapter"
        );
    }
    let covered: usize = [
        &["entries"][..],
        &["junit"],
        &["cargo-mutants"],
        &[
            "sbom",
            "agent-eval",
            "contract-conformance",
            "differential-report",
        ],
        &["sarif", "audit-script", "cargo-audit"],
    ]
    .iter()
    .map(|group| {
        corpus
            .cases
            .iter()
            .filter(|c| group.contains(&c.adapter.as_str()))
            .count()
    })
    .sum();
    assert_eq!(
        covered,
        corpus.cases.len(),
        "some golden case belongs to no parity test"
    );
}

/// At least one golden case reads a real, checked-in tool output rather than a
/// string authored beside the assertion.
///
/// A corpus made only of inputs the porter wrote is a corpus the porter can
/// make pass; the fixtures under `tests/fixtures/evidence/` are unedited
/// producer output and carry their provenance in that directory's README.
///
/// Trace: FR-030-AC-17
/// Provenance: quoin#456
#[test]
fn tc_456_007_corpus_holds_real_producer_output() {
    let (corpus, _) = load();
    let from_file = corpus.cases.iter().filter(|c| c.file.is_some()).count();
    assert!(
        from_file >= 5,
        "only {from_file} golden cases read a checked-in real tool output"
    );
}
