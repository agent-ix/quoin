// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The four retired `assurance.*` operations across a real pipe, over the
//! captured TypeScript verdicts (quoin#447).
//!
//! # Why this test exists after the difftest already passed
//!
//! `quoin-difftest` compares the binary against the RETAINED TypeScript, and
//! this ticket deletes that TypeScript — so the difftest's `assurance/*` cases
//! are retired in the same commit as the code they oracle. FR-101 is satisfied
//! at the cutover revision by construction, and what remains afterwards must
//! be a gate that does not need Node.
//!
//! That gate is this one. `quoin-assurance`' golden corpus was captured from
//! `src/assurance/` and is checked in; driving the real `quoin-core` binary
//! over it asserts that the operator-visible answer still matches the
//! TypeScript's, at the boundary rather than in the library, for as long as
//! the corpus is kept. `quoin-assurance`'s own `tc_447_500` runs the same
//! corpus through the library; this one runs it through the request shape, so
//! the two together say the seam did not change the answer.
//!
//! Error PROSE is never asserted here. `quoin-difftest` states the rule the
//! corpus inherits: the verdict is contractual and the message text is not, so
//! a refusal asserts its exit class and its diagnostic code and stops there.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Debug, Deserialize)]
struct Corpus {
    /// The one complete argument every `parse_argument` case varies. An
    /// authored argument is twelve required keys deep; forty full copies would
    /// hide the single field each case is about.
    argument_base: Map<String, Value>,
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    name: String,
    op: String,
    /// Marks an input the Rust REQUEST SCHEMA refuses. The retained
    /// TypeScript's answer to it is captured all the same, so the divergence
    /// is recorded rather than hidden; `tests/golden/PROVENANCE.md` counts
    /// them.
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
    Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quoin-assurance/tests/golden"
    ))
}

fn read<T: for<'de> Deserialize<'de>>(file: &str) -> T {
    serde_json::from_str(&std::fs::read_to_string(golden_dir().join(file)).unwrap()).unwrap()
}

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

fn run(op: &str, stdin: &str) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_quoin-core"))
        .arg(op)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    Run {
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
        status: out.status.code().unwrap(),
    }
}

/// The request a case hands to the binary, rebuilt exactly as the capture
/// rebuilt it.
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

/// Every captured TypeScript verdict, reproduced by the real binary over a
/// pipe.
///
/// Trace: FR-040, FR-047, FR-101
/// Provenance: quoin#447
#[test]
fn tc_447_510_the_boundary_reproduces_every_captured_typescript_verdict() {
    let corpus: Corpus = read("cases.json");
    let expected: Expectations = read("expected.json");
    assert_eq!(corpus.cases.len(), expected.cases.len());
    // A comparison over an empty population is green and says nothing. The
    // corpus is 128 cases; without this a path typo would read as a pass.
    assert!(
        corpus.cases.len() >= 120,
        "the golden corpus came back with {} cases; it is not being read",
        corpus.cases.len()
    );

    for (case, expectation) in corpus.cases.iter().zip(&expected.cases) {
        assert_eq!(case.name, expectation.name);
        let op = format!("assurance.{}", case.op);
        let result = run(&op, &request_for(&corpus, case).to_string());

        // Two ways to be refused, and they are not the same finding. A
        // `boundary: refused` case is one the Rust request schema turns away
        // while the retained TypeScript answered it — a recorded divergence
        // class, not a parity failure. A case the TypeScript itself threw on
        // must be refused here too.
        if case.boundary.as_deref() == Some("refused") || !expectation.ok {
            assert_eq!(
                result.status, 3,
                "{} was expected to be refused as invalid: {}",
                case.name, result.stderr
            );
            assert_eq!(result.stdout, "", "{}", case.name);
            let diagnostics: Value = serde_json::from_str(&result.stderr).unwrap();
            // `CORE_BAD_JSON` is the DISPATCHER's refusal and `CORE_BAD_REQUEST`
            // the handler's, and which one answers is itself contractual: a
            // request that is not a JSON object never reaches an operation, so
            // `parse-argument/not-an-object` is turned away one layer earlier
            // than every other refusal in the corpus.
            let code = diagnostics[0]["code"].as_str().unwrap();
            assert!(
                code == "CORE_BAD_REQUEST" || code == "CORE_BAD_JSON",
                "{} was refused as {code}",
                case.name
            );
            continue;
        }

        assert_eq!(result.status, 0, "{}: {}", case.name, result.stderr);
        assert_eq!(result.stderr, "", "{}", case.name);
        let payload: Value = serde_json::from_str(&result.stdout).unwrap();
        assert_eq!(
            payload, expectation.value,
            "{} diverged from the captured TypeScript verdict",
            case.name
        );
    }
}

/// The corpus reaches all four retired operations at the boundary, in numbers.
///
/// The guard above proves the file was read; this one proves it was authored.
/// A corpus that drifted into 128 `requirement_of` cases would pass every
/// assertion above and exercise one handler.
///
/// Trace: FR-101
/// Provenance: quoin#447
#[test]
fn tc_447_511_the_corpus_reaches_all_four_retired_operations() {
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

/// A built case is a renderable case — except where its assessments are
/// untyped, which is a recorded divergence rather than a bug in the loop.
///
/// `assurance.render_case`'s input IS `assurance.build_case`'s output, and the
/// two handlers deserialise it with different types — `CaseInput` against
/// `RenderableCase`. Feeding one straight into the other is the only check
/// that those two agree at the boundary; the corpus alone would still pass if
/// the payload gained a field the renderer refuses.
///
/// **Where they do not agree, this test says so in numbers instead of
/// skipping.** `build_case` treats `producer_trust` and
/// `evidence_independence` as OPAQUE on both sides — the retained
/// implementation copies them unread, and the port types them as
/// `serde_json::Value` — so both carry an assessment missing `useId`, or one
/// whose `triggeredBy` holds objects. The retained RENDERER also accepts those
/// and interpolates `undefined` or `[object Object]`; the port's
/// `TrustAssessment` and `IndependenceAssessment` are typed, so it refuses the
/// request instead. Rust refusing what TypeScript renders as `[object Object]`
/// is the port declining to reproduce a defect, and `quoin-evidence-types`'
/// own `TrustAssessment` docs say as much. Asserting the split pins both
/// halves: a refusal must come from the renderer's request schema and nowhere
/// else, and the divergent population must stay exactly the two cases recorded
/// here — a count, not "fewer than the other one", which 2 against 27 satisfies
/// as comfortably as 13 against 14 would.
///
/// Trace: FR-040, FR-101
/// Provenance: quoin#447
#[test]
fn tc_447_512_a_built_case_is_a_renderable_case_unless_its_assessments_are_untyped() {
    let corpus: Corpus = read("cases.json");
    let mut rendered = 0_usize;
    let mut refused = 0_usize;
    let mut eligible = 0_usize;

    for case in &corpus.cases {
        if case.op != "build_case" || case.boundary.is_some() {
            continue;
        }
        eligible += 1;
        let input = case.input.clone().unwrap();
        let built = run("assurance.build_case", &input.to_string());
        assert_eq!(built.status, 0, "{}: {}", case.name, built.stderr);

        let result = run("assurance.render_case", &built.stdout);
        if result.status == 0 {
            rendered += 1;
            continue;
        }

        // Only the assessments are opaque to `build_case`; every other part of
        // its payload is typed on both sides, so a case carrying neither
        // assessment must round-trip.
        assert!(
            input.get("producer_trust").is_some() || input.get("evidence_independence").is_some(),
            "{}: build_case produced a payload render_case refuses, \
             and the input carries no assessment to explain it: {}",
            case.name,
            result.stderr
        );
        assert_eq!(result.status, 3, "{}", case.name);
        assert_eq!(result.stdout, "", "{}", case.name);
        let diagnostics: Value = serde_json::from_str(&result.stderr).unwrap();
        assert_eq!(diagnostics[0]["code"], "CORE_BAD_REQUEST", "{}", case.name);
        assert_eq!(
            diagnostics[0]["context"]["op"], "assurance.render_case",
            "{}: the refusal came from somewhere other than the renderer's request schema",
            case.name
        );
        refused += 1;
    }

    assert!(
        rendered >= 25,
        "only {rendered} built cases were re-rendered; the loop is not reaching the corpus"
    );
    // Every eligible case ended in one population or the other. Without this a
    // `continue` added to the loop would shrink both counts and still satisfy
    // the thresholds.
    assert_eq!(
        rendered + refused,
        eligible,
        "{eligible} eligible build_case cases, but {rendered} rendered and {refused} \
         refused; some case reached neither outcome"
    );
    // The divergence population is PINNED, not merely bounded below the other
    // one. `refused < rendered` held at 2 against 27 and would have held at 13
    // against 14, so it carried almost nothing beside the threshold above it.
    // Two cases diverge today — the ones whose opaque `producer_trust` /
    // `evidence_independence` the retained renderer interpolates as
    // `[object Object]` and the port's typed request schema refuses. A third
    // is a new divergence class and must be read and decided, not absorbed.
    assert_eq!(
        refused, 2,
        "the recorded renderer divergence is 2 cases; it is now {refused}. \
         Read the new one: either the renderer's request schema has narrowed \
         past the assessments, or the corpus gained a case that needs its own \
         entry in tests/golden/PROVENANCE.md"
    );
}
