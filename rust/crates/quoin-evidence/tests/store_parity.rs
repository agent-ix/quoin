// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Store, trust, independence, assurance-record and mock-inspection parity with
//! the retained TypeScript (quoin#456, half 2).
//!
//! `tests/golden/store-cases.json` is hand-authored input;
//! `tests/golden/store-expected.json` was captured once from `src/evidence/` and
//! is the only oracle consulted here. No Node process runs in this lane
//! (FR-101-AC-5). See `tests/golden/STORE-PROVENANCE.md`.
//!
//! The `canonical` cases compare the **bytes the writers put on disk**, which is
//! what NFR-025 is a statement about, rather than a re-canonicalisation of a
//! hand-built object.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use quoin_evidence::assurance_records::{
    write_experiment_record, write_operational_evidence_record,
};
use quoin_evidence::independence::assess_independence;
use quoin_evidence::mock_inspection::inspect_mock_injections;
use quoin_evidence::store::{
    affirm, bind, scan_is_vacuous, write_baseline, write_bindings, write_mock_inspection,
    write_run, write_scan, write_trust_decision,
};
use quoin_evidence::trust::assess_trust;
use quoin_evidence::types::{
    Affirmation, BaselineFile, Binding, FindingRecord, IndependenceRequirement,
    MockInspectionRecord, RunRecord, TrustDecision,
};
use quoin_evidence::{
    Commit, DiskEvidence, EvidenceSource, MemoryEvidence, ObligationId, ProfileId, StatementHash,
    SuiteId,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};

/// Markers that a refusal clause was written by `zod`, not by quoin.
///
/// The retained boundary is a zod schema, so a plain type or length failure
/// carries a dependency's prose. Reproducing it would be porting zod. Where one
/// of these appears, only the clause's **path** — quoin's own word for the field
/// — is compared, and the divergence is recorded in `STORE-PROVENANCE.md`.
const ZOD_PROSE: [&str; 3] = ["Too small", "Invalid input", "Invalid string"];

#[derive(Debug, Deserialize)]
struct Corpus {
    cases: Vec<CaseInput>,
}

#[derive(Debug, Clone, Deserialize)]
struct CaseInput {
    name: String,
    kind: String,
    input: Value,
}

#[derive(Debug, Deserialize)]
struct Expectations {
    cases: Vec<CaseExpectation>,
}

#[derive(Debug, Deserialize)]
struct CaseExpectation {
    name: String,
    kind: String,
    /// The value the TypeScript produced, when it accepted the input.
    ok: Option<Value>,
    /// The message it threw, when it refused.
    err: Option<String>,
}

fn golden_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden"))
}

fn load() -> (Corpus, Expectations) {
    let corpus: Corpus =
        serde_json::from_str(&fs::read_to_string(golden_dir().join("store-cases.json")).unwrap())
            .unwrap();
    let expected: Expectations = serde_json::from_str(
        &fs::read_to_string(golden_dir().join("store-expected.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        corpus.cases.len(),
        expected.cases.len(),
        "the corpus and the capture disagree on how many cases exist"
    );
    (corpus, expected)
}

/// Run one case through the Rust implementation.
fn run_case(case: &CaseInput) -> Result<Value, String> {
    match case.kind.as_str() {
        "trust" => {
            let decision: TrustDecision = from_value(&case.input)?;
            assess_trust(&decision)
                .map(|assessment| json!(assessment))
                .map_err(|error| error.to_string())
        }
        "independence" => {
            let profile = ProfileId::parse(string_at(&case.input, "profile"))
                .map_err(|error| error.to_string())?;
            let requirement: IndependenceRequirement = from_value(&case.input["requirement"])?;
            let bindings: Vec<Binding> = from_value(&case.input["bindings"])?;
            Ok(json!(assess_independence(
                &profile,
                &requirement,
                &bindings
            )))
        }
        "bind" => {
            let existing: Vec<Binding> = from_value(&case.input["existing"])?;
            let next: Binding = from_value(&case.input["next"])?;
            let outcome = bind(&existing, &next);
            Ok(json!({
                "bindings": outcome.bindings,
                "created": outcome.created,
                "suspect": outcome.suspect,
            }))
        }
        "affirm" => {
            let existing: Vec<Binding> = from_value(&case.input["existing"])?;
            let obligation = ObligationId::new(string_at(&case.input, "obligation"));
            let suite = case
                .input
                .get("suite")
                .and_then(Value::as_str)
                .map(SuiteId::new);
            let current_hash = StatementHash::new(string_at(&case.input, "currentHash"));
            let affirmation = Affirmation {
                who: string_at(&case.input, "who"),
                commit: Commit::new(string_at(&case.input, "commit")),
                note: case
                    .input
                    .get("note")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            };
            let outcome = affirm(
                &existing,
                &obligation,
                suite.as_ref(),
                &current_hash,
                &affirmation,
            );
            Ok(json!({ "bindings": outcome.bindings, "found": outcome.found }))
        }
        "canonical" => canonical_case(&case.input),
        "vacuity" => {
            let record: FindingRecord = from_value(&case.input)?;
            Ok(json!({ "vacuous": scan_is_vacuous(&record) }))
        }
        "assurance" => assurance_case(&case.input),
        "mock" => mock_case(&case.input),
        other => panic!("case {} names unknown kind {other}", case.name),
    }
}

/// Write one record through the store and return every file it produced.
fn canonical_case(input: &Value) -> Result<Value, String> {
    let record = string_at(input, "record");
    let value = &input["value"];
    let mut store = MemoryEvidence::new();
    match record.as_str() {
        "run" => write_run(&mut store, &from_value::<RunRecord>(value)?)
            .map(|_| ())
            .map_err(|error| error.to_string())?,
        "scan" => write_scan(&mut store, &from_value::<FindingRecord>(value)?)
            .map(|_| ())
            .map_err(|error| error.to_string())?,
        "mockInspection" => {
            write_mock_inspection(&mut store, &from_value::<MockInspectionRecord>(value)?)
                .map(|_| ())
                .map_err(|error| error.to_string())?;
        }
        "bindings" => {
            let bindings: Vec<Binding> = from_value(&value["bindings"])?;
            write_bindings(&mut store, &bindings).map_err(|error| error.to_string())?;
        }
        "baseline" => {
            let file: BaselineFile = from_value(value)?;
            write_baseline(&mut store, &file.commit, &file.accepted)
                .map_err(|error| error.to_string())?;
        }
        "trustDecision" => {
            write_trust_decision(&mut store, &from_value::<TrustDecision>(value)?)
                .map(|_| ())
                .map_err(|error| error.to_string())?;
        }
        other => panic!("unknown canonical record {other}"),
    }
    let files: Vec<Value> = store
        .paths()
        .into_iter()
        .map(|path| {
            json!({
                "path": path,
                "text": store.read(path).unwrap().unwrap(),
            })
        })
        .collect();
    Ok(json!({ "record": record, "files": files }))
}

/// Publish one assurance record and return the validated input it carries.
fn assurance_case(input: &Value) -> Result<Value, String> {
    let mut store = MemoryEvidence::new();
    let value = &input["value"];
    if string_at(input, "record") == "experiment" {
        write_experiment_record(&mut store, value)
            .map(|stored| json!(stored.record.input))
            .map_err(|error| error.to_string())
    } else {
        write_operational_evidence_record(&mut store, value)
            .map(|stored| json!(stored.record.input))
            .map_err(|error| error.to_string())
    }
}

/// Inspect a temporary repository on disk.
///
/// `DiskEvidence` and not `MemoryEvidence`: the excluded-directory and
/// source-extension rules are the disk walk's, and a case that seeds a
/// `node_modules/` file exists to assert the walk skips it.
fn mock_case(input: &Value) -> Result<Value, String> {
    let repo = tempfile::tempdir().unwrap();
    let files = input["files"].as_object().unwrap();
    for (relative, text) in files {
        let target = repo.path().join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, text.as_str().unwrap()).unwrap();
    }
    let suite = SuiteId::new(string_at(input, "suite"));
    inspect_mock_injections(&DiskEvidence::new(repo.path()), &suite)
        .map(|injections| json!(injections))
        .map_err(|error| error.to_string())
}

fn from_value<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|error| error.to_string())
}

fn string_at(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap().to_owned()
}

/// Drop `null`-valued keys and render integral floats as integers.
///
/// `JSON.stringify` omits an `undefined` property where serde writes `null` for
/// a `None` without `skip_serializing_if`; neither is a byte on disk, because
/// every record written by the store goes through `quoin-store`'s canonical
/// serialiser, whose output the `canonical` cases compare verbatim.
fn normalise(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(_, item)| !item.is_null())
                .map(|(key, item)| (key.clone(), normalise(item)))
                .collect::<Map<String, Value>>(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(normalise).collect()),
        Value::Number(number) => number
            .as_f64()
            .filter(|float| float.fract() == 0.0 && float.abs() < 9.007_199_254_740_992e15)
            .map_or_else(
                || value.clone(),
                |float| {
                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "guarded: the value is integral and inside the i64 range"
                    )]
                    Value::Number((float as i64).into())
                },
            ),
        other => other.clone(),
    }
}

/// Assert the two refusals say the same thing, clause by clause.
///
/// Colons are stripped from both sides before comparison: the retained helpers
/// write `"<path> must be …"` where the port writes `"<path>: must be …"`, and
/// the separator is the only difference. A clause carrying zod's own prose is
/// compared on its path alone.
fn assert_refusal_parity(name: &str, expected: &str, actual: &str) {
    let flattened = actual.replace(':', "");
    for clause in expected.split("; ") {
        if ZOD_PROSE.iter().any(|marker| clause.contains(marker)) {
            let path = clause.split(':').next().unwrap_or(clause);
            assert!(
                actual.contains(path),
                "{name}: the port's refusal does not name {path}\n  expected clause: {clause}\n  actual: {actual}"
            );
        } else {
            let wanted = clause.replace(':', "");
            assert!(
                flattened.contains(&wanted),
                "{name}: the port's refusal is missing a clause\n  expected: {clause}\n  actual: {actual}"
            );
        }
    }
}

/// Trace: FR-100-AC-2, FR-030-AC-1, FR-030-AC-2, FR-093-AC-1, FR-094-AC-1
#[test]
fn tc_456_100_the_store_half_reproduces_the_retained_typescript() {
    let (corpus, expected) = load();
    let mut accepted = 0_usize;
    let mut refused = 0_usize;
    for (case, want) in corpus.cases.iter().zip(expected.cases.iter()) {
        assert_eq!(case.name, want.name, "the corpus and the capture drifted");
        assert_eq!(case.kind, want.kind, "the corpus and the capture drifted");
        match (run_case(case), want.ok.as_ref(), want.err.as_ref()) {
            (Ok(got), Some(expect), None) => {
                accepted += 1;
                assert_eq!(
                    normalise(&got),
                    normalise(expect),
                    "{}: the port accepted the input but produced a different result",
                    case.name
                );
            }
            (Err(message), None, Some(expect)) => {
                refused += 1;
                assert_refusal_parity(&case.name, expect, &message);
            }
            (Ok(got), None, Some(expect)) => panic!(
                "{}: the TypeScript refused with {expect}, the port accepted with {got}",
                case.name
            ),
            (Err(message), Some(_), None) => panic!(
                "{}: the TypeScript accepted, the port refused with {message}",
                case.name
            ),
            (_, _, _) => panic!("{}: the capture records neither ok nor err", case.name),
        }
    }
    // Anti-vacuity. A corpus that only accepts carries no refusal criterion, and
    // one that only refuses carries no behaviour at all.
    assert!(accepted > 0, "no case was accepted");
    assert!(refused > 0, "no case was refused");
}

/// Trace: FR-030-AC-1, FR-048-AC-1, FR-093-AC-1, FR-094-AC-1
#[test]
fn tc_456_101_every_ported_behaviour_family_is_represented() {
    let (corpus, _) = load();
    let kinds: BTreeSet<&str> = corpus.cases.iter().map(|case| case.kind.as_str()).collect();
    assert_eq!(
        kinds,
        BTreeSet::from([
            "affirm",
            "assurance",
            "bind",
            "canonical",
            "independence",
            "mock",
            "trust",
            "vacuity",
        ]),
        "a behaviour family lost its cases, or gained one the capture does not cover"
    );
    // An anti-vacuity floor per family: one case cannot carry a family.
    for kind in &kinds {
        let count = corpus
            .cases
            .iter()
            .filter(|case| &case.kind == kind)
            .count();
        assert!(count >= 3, "family {kind} carries only {count} case(s)");
    }
}
