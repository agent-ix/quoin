// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The corpus-v2 experiment harness, offline (PLAT-1027).
//!
//! Everything the live runner (`live_eval_v2.rs`) relies on, checked with no
//! network and no key: the corpus schema and its validator, the external
//! by-reference path, the held-out seal and its gate, the variant registry
//! and its wording rule, the unit splitter, the metrics, and the runner end to
//! end over a fake Jev transport.
//!
//! `tc_1027_the_committed_corpus_validates` checks whatever corpus is present.
//! Until PLAT-1025 commits `fixtures/eval-v2/corpus.json` it prints that
//! there is nothing to check and returns; it never passes over a corpus it
//! did not read.
//!
//! Provenance: PLAT-1027, PLAT-1024.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
#![allow(
    clippy::float_cmp,
    reason = "the asserted rates are exact ratios of small integers (2/4), not rounded aggregates"
)]

mod eval_v2_support;
mod gap_semantic_support;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use serde_json::{Value, json};
use typesafe_sdk_env::Fixed;
use typesafe_sdk_error::Result as SdkResult;
use typesafe_sdk_headers::Headers;
use typesafe_sdk_http::{RawResponse, Request, Transport};
use typesafe_sdk_questions::{Questions, noul, questions};

use eval_v2_support::corpus::{
    self, CorpusFile, HELDOUT_ENV, Origin, Source, authorize_heldout, heldout_digest,
    load_external, validate, verify_seal,
};
use eval_v2_support::corpus::{KindGroup, TruthKind};
use eval_v2_support::keys::{KEYS, Mode};
use eval_v2_support::metrics::{
    Scored, concordance, coverage_curve, graded, render_run, scored, summarize,
};
use eval_v2_support::patch::apply_unified_patch;
use eval_v2_support::units::{UnitKind, extract_rust_fn, split_rust_units};
use eval_v2_support::variant::{
    self, Answered, Prediction, Predictions, REGISTRY, Variant, per_unit_asks, rollup_any_yes,
    wording_violations,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const TEST_BODY: &str = "#[test]\nfn tc_001_refuses_oversize() {\n    \
    let error = check(\"x\".repeat(5000)).unwrap_err();\n    \
    assert_eq!(error.code, Code::Refused);\n}";

const CODE_BODY: &str = "pub fn check(input: String) -> Result<(), Error> {\n    \
    match input.len() {\n        0 => Err(Error::empty()),\n        \
    n if n > 4096 => { log(\"too big }\"); Err(Error::refused()) }\n        \
    _ => Ok(()),\n    }\n}";

/// One synthetic row of `mode`, with `truth` as given.
fn row(id: &str, mode: Mode, split: &str, truth: &Value) -> Value {
    let test = mode.has_test().then(|| {
        json!({"path": "tests/tc_001.rs", "fn_name": "tc_001_refuses_oversize", "body": TEST_BODY})
    });
    let code = mode
        .has_code()
        .then(|| json!({"path": "src/check.rs", "symbol": "check", "body": CODE_BODY}));
    json!({
        "id": id,
        "mode": mode.as_str(),
        "split": split,
        "strata": {"fr_id": "FR-001", "req_kind": "functional", "test_kind": null,
                   "crate": "quoin-core", "ears_pattern": null},
        "requirement": {"fr_id": "FR-001", "ac_id": "FR-001-AC-1",
                        "statement": "The system shall refuse a request larger than 4096 bytes.",
                        "ac_text": "A 5000-byte request is refused with CORE_REFUSED.",
                        "context": null},
        "test": test,
        "code": code,
        "ref": null,
        "mutation": null,
        "truth": truth,
    })
}

fn truth(answer: &Value, kind: &str, alternatives: &[&str]) -> Value {
    json!({"answer": answer, "kind": kind, "alternatives": alternatives, "rationale": "stated"})
}

fn corpus_text(rows: &[Value]) -> String {
    serde_json::to_string_pretty(&json!({
        "schema": corpus::SCHEMA,
        "sampling_rule": "every row, synthetic",
        "seed": 7,
        "source_commit": "0000000",
        "rows": rows,
    }))
    .unwrap()
}

fn parse(rows: &[Value]) -> CorpusFile {
    corpus::parse(&corpus_text(rows)).expect("synthetic corpus parses")
}

/// A four-row corpus, one per mode, with truth each mode supports.
fn four_modes() -> CorpusFile {
    parse(&[
        row(
            "EV2-0001",
            Mode::Req,
            "dev",
            &json!({"criterion_sound": truth(&json!(true), "agent_dual", &[])}),
        ),
        row(
            "EV2-0002",
            Mode::ReqTest,
            "dev",
            &json!({"test_asserts_intent": truth(&json!("yes"), "mechanical", &[])}),
        ),
        row(
            "EV2-0003",
            Mode::ReqCode,
            "dev",
            &json!({"code_exceeds_requirement": truth(&json!(false), "by_construction", &[])}),
        ),
        row(
            "EV2-0004",
            Mode::ReqTestCode,
            "dev",
            &json!({
                "severity": truth(&json!("high"), "agent_contested", &["medium"]),
                "code_exceeds_requirement": truth(&json!(true), "by_construction", &[]),
                "test_asserts_intent": truth(&json!(false), "mechanical", &[]),
                "criterion_sound": truth(&json!(false), "agent_dual", &[]),
            }),
        ),
    ])
}

// ---------------------------------------------------------------------------
// The corpus
// ---------------------------------------------------------------------------

fn print_and_check(source: &Source) {
    println!(
        "{}: {}",
        source.path.display(),
        corpus::census(&source.file)
    );
    let problems = validate(&source.file, source.origin);
    assert!(
        problems.is_empty(),
        "{} has {} problem(s):\n  - {}",
        source.path.display(),
        problems.len(),
        problems.join("\n  - ")
    );
    let seal = verify_seal(source).unwrap_or_else(|error| panic!("{error}"));
    println!("held-out seal holds: {seal}");
}

/// Provenance: PLAT-1027. Whatever corpus is present validates against the
/// shared schema and its held-out seal holds. The in-repo corpus is PLAT-1025's
/// and may not exist yet; the external one is read only when
/// `QUOIN_JEV_EXTERNAL_CORPUS` is set.
#[test]
fn tc_1027_the_committed_corpus_validates() {
    let mut checked = 0usize;
    match corpus::load_in_repo().unwrap_or_else(|error| panic!("{error}")) {
        Some(source) => {
            print_and_check(&source);
            checked += 1;
        }
        None => println!(
            "SKIPPED in-repo corpus: {} does not exist yet (PLAT-1025). Nothing was checked.",
            corpus::in_repo_corpus_path().display()
        ),
    }
    match corpus::load_external_from_env().unwrap_or_else(|error| panic!("{error}")) {
        Some(source) => {
            print_and_check(&source);
            checked += 1;
        }
        None => println!(
            "SKIPPED external corpus: {} is unset.",
            corpus::EXTERNAL_CORPUS_ENV
        ),
    }
    println!("{checked} corpus file(s) validated");
}

/// Provenance: PLAT-1027. A well-formed four-mode corpus has no problems, so
/// the checks below each fail for their own reason.
#[test]
fn tc_1027_a_well_formed_corpus_validates_clean() {
    assert_eq!(
        validate(&four_modes(), Origin::InRepo),
        Vec::<String>::new()
    );
}

/// Provenance: PLAT-1027. The schema is shared with PLAT-1025/PLAT-1026, so
/// a field one side invents is refused rather than dropped.
#[test]
fn tc_1027_an_unknown_field_is_refused() {
    let mut value = row("EV2-0001", Mode::Req, "dev", &json!({}));
    value["requirement"]["priority"] = json!("high");
    let error = corpus::parse(&corpus_text(&[value])).unwrap_err();
    assert!(error.contains("unknown field `priority`"), "{error}");
}

/// Provenance: PLAT-1027. Each rule in `validate` fires on its own defect.
#[test]
fn tc_1027_validation_names_each_defect() {
    let mut mode_mismatch = row(
        "EV2-0001",
        Mode::ReqTest,
        "dev",
        &json!({"test_asserts_intent": truth(&json!(true), "mechanical", &[])}),
    );
    mode_mismatch["test"] = Value::Null;
    let needs_code = row(
        "EV2-0002",
        Mode::ReqTest,
        "dev",
        &json!({"code_exceeds_requirement": truth(&json!(true), "mechanical", &[])}),
    );
    let out_of_space = row(
        "EV2-0003",
        Mode::ReqTestCode,
        "dev",
        &json!({"severity": truth(&json!("catastrophic"), "agent_dual", &[])}),
    );
    let contested_alone = row(
        "EV2-0004",
        Mode::Req,
        "dev",
        &json!({"criterion_sound": truth(&json!(true), "agent_contested", &[])}),
    );
    let mechanical_with_alt = row(
        "EV2-0005",
        Mode::Req,
        "dev",
        &json!({"vague_term": truth(&json!(true), "mechanical", &["no"])}),
    );
    let unknown_key = row(
        "EV2-0006",
        Mode::Req,
        "dev",
        &json!({"elegance": truth(&json!(true), "agent_dual", &[])}),
    );
    let mut referenced_in_repo = row(
        "EV2-0006",
        Mode::Req,
        "dev",
        &json!({"compound": truth(&json!(false), "agent_dual", &[])}),
    );
    referenced_in_repo["ref"] =
        json!({"repo": "agent-ix/quoin", "commit": "abc", "paths": {}, "sha256": {}});
    let file = parse(&[
        mode_mismatch,
        needs_code,
        out_of_space,
        contested_alone,
        mechanical_with_alt,
        unknown_key,
        referenced_in_repo,
    ]);
    let problems = validate(&file, Origin::InRepo);
    let expect = [
        "EV2-0001: mode RT but test is null",
        "EV2-0002: code_exceeds_requirement needs Code, which a mode RT row lacks",
        "EV2-0003: severity: \"catastrophic\" is outside the answer space",
        "EV2-0004: criterion_sound: agent_contested truth records no alternative reading",
        "EV2-0005: vague_term: Mechanical truth carries alternatives",
        "EV2-0006: unknown truth key \"elegance\"",
        "EV2-0006: duplicate id",
        "EV2-0006: an in-repo row embeds its bodies",
    ];
    for needle in expect {
        assert!(
            problems.iter().any(|problem| problem.starts_with(needle)),
            "no problem starts with {needle:?}; got {problems:#?}"
        );
    }
    assert_eq!(problems.len(), expect.len(), "{problems:#?}");
}

// ---------------------------------------------------------------------------
// The held-out seal
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1027. The seal is sha256 over the RFC 8785 bytes of the
/// held-out rows alone. The expected digest is the literal
/// `sha256('[{"id":"H1","split":"heldout"}]')`, not a re-derivation.
#[test]
fn tc_1027_the_seal_covers_heldout_rows_only() {
    let text = r#"{"rows": [ {"split": "heldout", "id": "H1"}, {"split": "dev", "id": "D1"} ]}"#;
    assert_eq!(
        heldout_digest(text).unwrap(),
        "8172870b2749f56bf06605a6a881d30833fd5f1e00e85a4065338bd9a72b9594"
    );
    let dev_changed =
        r#"{"rows": [{"split": "heldout", "id": "H1"}, {"split": "dev", "id": "D2"}]}"#;
    assert_eq!(
        heldout_digest(dev_changed).unwrap(),
        heldout_digest(text).unwrap()
    );
    let heldout_changed = r#"{"rows": [{"split": "heldout", "id": "H2"}]}"#;
    assert_ne!(
        heldout_digest(heldout_changed).unwrap(),
        heldout_digest(text).unwrap()
    );
}

fn sealed_source(dir: &std::path::Path, seal: Option<&str>) -> Source {
    let text = corpus_text(&[
        row(
            "EV2-0001",
            Mode::Req,
            "heldout",
            &json!({"compound": truth(&json!(false), "agent_dual", &[])}),
        ),
        row(
            "EV2-0002",
            Mode::Req,
            "dev",
            &json!({"compound": truth(&json!(true), "agent_dual", &[])}),
        ),
    ]);
    let path = dir.join("corpus.json");
    std::fs::write(&path, &text).unwrap();
    if let Some(seal) = seal {
        std::fs::write(dir.join(corpus::SEAL_FILE), seal).unwrap();
    }
    corpus::read_source(&path, Origin::InRepo).unwrap()
}

/// Provenance: PLAT-1027. A held-out run needs `QUOIN_JEV_HELDOUT=1` AND a
/// seal that matches; each missing piece is refused on its own.
#[test]
fn tc_1027_a_heldout_run_needs_the_flag_and_a_matching_seal() {
    let dir = tempfile::tempdir().unwrap();
    let unsealed = sealed_source(dir.path(), None);
    let digest = heldout_digest(&unsealed.text).unwrap();

    let no_flag = authorize_heldout(None, &[&unsealed]).unwrap_err();
    assert!(no_flag.contains(HELDOUT_ENV), "{no_flag}");
    let wrong_flag = authorize_heldout(Some("yes"), &[&unsealed]).unwrap_err();
    assert!(wrong_flag.contains(HELDOUT_ENV), "{wrong_flag}");
    let no_seal = authorize_heldout(Some("1"), &[&unsealed]).unwrap_err();
    assert!(no_seal.contains("not sealed"), "{no_seal}");

    let mismatched = sealed_source(dir.path(), Some(&"0".repeat(64)));
    let refused = authorize_heldout(Some("1"), &[&mismatched]).unwrap_err();
    assert!(refused.contains("changed after it was sealed"), "{refused}");

    let sealed = sealed_source(dir.path(), Some(&format!("sha256:{digest}\n")));
    assert_eq!(
        authorize_heldout(Some("1"), &[&sealed]).unwrap(),
        vec![digest]
    );
}

/// Provenance: PLAT-1027. The run log appends one line per held-out run.
#[test]
fn tc_1027_a_heldout_run_is_logged_with_its_variants() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("heldout-runs.jsonl");
    for variants in [
        vec!["S0@v1".to_owned()],
        vec!["S1@v2".to_owned(), "E0@v1".to_owned()],
    ] {
        corpus::record_heldout_run(
            &log,
            &corpus::HeldoutRun {
                unix_seconds: 1,
                variants,
                seals: vec!["abc".to_owned()],
                rows: 3,
                models: std::collections::BTreeMap::new(),
            },
        )
        .unwrap();
    }
    let lines: Vec<corpus::HeldoutRun> = std::fs::read_to_string(&log)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1].variants, ["S1@v2", "E0@v1"]);
}

// ---------------------------------------------------------------------------
// External rows, by reference
// ---------------------------------------------------------------------------

const EXTERNAL_SOURCE: &str = "use std::fmt;\n\n/// Adds.\npub fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n\n#[test]\nfn adds() {\n    assert_eq!(add(2, 2), 4);\n}\n";

const EXTERNAL_PATCH: &str = "--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -4,3 +4,3 @@\n pub fn add(a: u32, b: u32) -> u32 {\n-    a + b\n+    a * b\n }\n";

fn external_corpus(dir: &std::path::Path, sha: &str, path: &str) -> std::path::PathBuf {
    let mut value = row(
        "EXT-0001",
        Mode::ReqTestCode,
        "dev",
        &json!({"code_implements_intent": truth(&json!(false), "mechanical", &[])}),
    );
    value["test"] = json!({"path": path, "fn_name": "adds"});
    value["code"] = json!({"path": path, "symbol": "Adder::add"});
    value["ref"] = json!({
        "repo": "agent-ix/sibling", "commit": "deadbeef",
        "paths": {"test": path, "code": path},
        "sha256": {"test": sha, "code": sha},
    });
    value["mutation"] = json!({
        "id": "M1", "target": "code", "kind": "swap_operator",
        "description": "+ becomes *", "patch": EXTERNAL_PATCH,
    });
    let corpus = dir.join("external.json");
    std::fs::write(&corpus, corpus_text(&[value])).unwrap();
    corpus
}

/// Provenance: PLAT-1027. An external row is read from the checkout, its
/// digest checked, its mutation applied, and the named `fn`s cut out — with
/// no body ever stored in quoin.
#[test]
fn tc_1027_an_external_row_materializes_from_its_checkout() {
    let root = tempfile::tempdir().unwrap();
    let checkout = root.path().join("sibling/src");
    std::fs::create_dir_all(&checkout).unwrap();
    std::fs::write(checkout.join("lib.rs"), EXTERNAL_SOURCE).unwrap();
    let sha = quoin_store::digest_bytes_sha256(EXTERNAL_SOURCE.as_bytes()).to_stored();
    let corpus = external_corpus(root.path(), &sha, "src/lib.rs");

    let source = load_external(Some(&corpus), Some(root.path()))
        .unwrap()
        .unwrap();
    let row = &source.file.rows[0];
    assert_eq!(
        row.code.as_ref().unwrap().body,
        "/// Adds.\npub fn add(a: u32, b: u32) -> u32 {\n    a * b\n}"
    );
    assert_eq!(
        row.test.as_ref().unwrap().body,
        "#[test]\nfn adds() {\n    assert_eq!(add(2, 2), 4);\n}"
    );
    assert_eq!(
        validate(&source.file, Origin::External),
        Vec::<String>::new()
    );
}

/// Provenance: PLAT-1027. A checkout whose bytes differ from the row's
/// digest is refused, as are a missing root and a path that escapes the repo.
#[test]
fn tc_1027_an_external_row_is_refused_when_its_content_moved() {
    let root = tempfile::tempdir().unwrap();
    let checkout = root.path().join("sibling/src");
    std::fs::create_dir_all(&checkout).unwrap();
    std::fs::write(checkout.join("lib.rs"), EXTERNAL_SOURCE).unwrap();

    let stale = external_corpus(root.path(), &"a".repeat(64), "src/lib.rs");
    let error = load_external(Some(&stale), Some(root.path())).unwrap_err();
    assert!(error.contains("does not match the row's"), "{error}");

    let no_root = load_external(Some(&stale), None).unwrap_err();
    assert!(no_root.contains(corpus::EXTERNAL_ROOT_ENV), "{no_root}");

    let escaping = external_corpus(root.path(), &"a".repeat(64), "../../etc/passwd");
    let error = load_external(Some(&escaping), Some(root.path())).unwrap_err();
    assert!(error.contains("not a plain repo-relative path"), "{error}");
}

/// Provenance: PLAT-1027. A hunk applies at its stated line; one whose
/// context is gone is refused rather than half-applied.
#[test]
fn tc_1027_a_patch_applies_exactly_or_not_at_all() {
    let patched = apply_unified_patch(EXTERNAL_SOURCE, EXTERNAL_PATCH).unwrap();
    assert!(patched.contains("    a * b\n"));
    assert!(!patched.contains("    a + b\n"));
    assert!(patched.ends_with("}\n"));
    let error = apply_unified_patch("fn other() {}\n", EXTERNAL_PATCH).unwrap_err();
    assert_eq!(error, "hunk 1 does not match the source");
    assert_eq!(
        apply_unified_patch(EXTERNAL_SOURCE, "no hunks here").unwrap_err(),
        "the patch has no hunks"
    );
}

// ---------------------------------------------------------------------------
// Units
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1027. Two `fn`s split into two function units, with
/// their attributes and doc comments.
#[test]
fn tc_1027_several_functions_split_into_function_units() {
    let body = "impl Thing {\n    /// One.\n    fn one(&self) -> u8 { 1 }\n\n    \
                #[inline]\n    fn two(&self) -> u8 { let s = \"}\"; 2 }\n}";
    let units = split_rust_units(body);
    let labels: Vec<(&str, UnitKind)> = units.iter().map(|u| (u.label.as_str(), u.kind)).collect();
    assert_eq!(
        labels,
        [
            ("fn one", UnitKind::Function),
            ("fn two", UnitKind::Function)
        ]
    );
    assert_eq!(
        units[1].text,
        "    #[inline]\n    fn two(&self) -> u8 { let s = \"}\"; 2 }"
    );
}

/// Provenance: PLAT-1027. One `fn` with a `match` splits into one unit per
/// arm; a brace inside a string literal does not move the split.
#[test]
fn tc_1027_a_match_splits_into_arm_units() {
    let units = split_rust_units(CODE_BODY);
    let labels: Vec<&str> = units.iter().map(|u| u.label.as_str()).collect();
    assert_eq!(
        labels,
        ["match arm 0", "match arm n if n > 4096", "match arm _"]
    );
    assert!(units.iter().all(|unit| unit.kind == UnitKind::Branch));
    assert_eq!(
        units[1].text,
        "n if n > 4096 => { log(\"too big }\"); Err(Error::refused()) }"
    );
}

/// Provenance: PLAT-1027. An `if` / `else if` / `else` chain splits into one
/// unit per block; a body with nothing to split is one whole unit.
#[test]
fn tc_1027_an_if_chain_splits_and_a_plain_body_does_not() {
    let body =
        "fn f(x: i32) -> char {\n    if x < 0 { '-' } else if x == 0 { '0' } else { '+' }\n}";
    let labels: Vec<String> = split_rust_units(body)
        .into_iter()
        .map(|u| u.label)
        .collect();
    assert_eq!(labels, ["if x < 0", "else if x == 0", "else"]);

    let plain = "fn g() -> u8 { // if { match\n    let c = '{'; 1 }";
    let units = split_rust_units(plain);
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].kind, UnitKind::Whole);
    assert_eq!(units[0].text, plain);
}

/// Provenance: PLAT-1027. An ambiguous or missing symbol is refused.
#[test]
fn tc_1027_extracting_a_symbol_refuses_ambiguity() {
    let source = "impl A { fn run() {} }\nimpl B { fn run() {} }\n";
    assert_eq!(
        extract_rust_fn(source, "run").unwrap_err(),
        "`fn run` is ambiguous: 2 definitions in the source"
    );
    assert_eq!(
        extract_rust_fn(source, "walk").unwrap_err(),
        "no `fn walk` with a body in the source"
    );
}

// ---------------------------------------------------------------------------
// Variants
// ---------------------------------------------------------------------------

/// Provenance: PLAT-1027. Ids are unique, every graded key is a known key,
/// and every derive only claims keys its variant can be graded on.
#[test]
fn tc_1027_the_registry_is_well_formed() {
    let mut ids: Vec<&str> = REGISTRY.iter().map(|variant| variant.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), REGISTRY.len(), "duplicate variant id");
    for variant in REGISTRY {
        assert!(!variant.modes.is_empty(), "{} runs nowhere", variant.id);
        for key in variant.grades {
            assert!(
                KEYS.iter().any(|spec| spec.key == *key),
                "{} grades unknown key {key}",
                variant.id
            );
        }
    }
    assert_eq!(
        variant::resolve("S0, E0,T0")
            .unwrap()
            .iter()
            .map(|v| v.id)
            .collect::<Vec<_>>(),
        ["S0", "E0", "T0"]
    );
    assert!(variant::resolve("S9").unwrap_err().contains("registered"));
}

/// Provenance: PLAT-1027, PLAT-1024 (Peter's scope ruling). No registered
/// variant's questions or state name an artifact absent from a mode it runs
/// in.
#[test]
fn tc_1027_every_registered_variant_respects_its_modes() {
    let file = four_modes();
    for variant in REGISTRY {
        for row in file.rows.iter().filter(|row| variant.applies_to(row)) {
            assert!(
                !(variant.asks)(row).is_empty(),
                "{} asks nothing",
                variant.id
            );
            let violations = wording_violations(variant, row);
            assert!(violations.is_empty(), "{violations:#?}");
        }
    }
}

/// Provenance: PLAT-1027. The wording rule is not vacuous: the battery sent
/// to a requirement-plus-test row is flagged for naming code, and to a
/// requirement-only row for naming both.
#[test]
fn tc_1027_the_wording_rule_catches_an_absent_artifact() {
    let file = four_modes();
    let misapplied = Variant {
        modes: &Mode::ALL,
        ..variant::B0
    };
    let rt = wording_violations(&misapplied, &file.rows[1]);
    assert!(rt.iter().any(|v| v.contains("refers to code")), "{rt:#?}");
    assert!(
        !rt.iter().any(|v| v.contains("refers to a test")),
        "{rt:#?}"
    );
    let r = wording_violations(&misapplied, &file.rows[0]);
    assert!(r.iter().any(|v| v.contains("refers to a test")), "{r:#?}");
    assert!(r.iter().any(|v| v.contains("refers to code")), "{r:#?}");
}

/// Provenance: PLAT-1027. The runtime state carries only the mode's
/// artifacts.
#[test]
fn tc_1027_state_carries_only_the_modes_artifacts() {
    let file = four_modes();
    // Sorted: whether `serde_json::Map` keeps insertion order depends on
    // workspace feature unification, and this test is about membership.
    let keys = |index: usize| -> Vec<String> {
        let mut keys: Vec<String> = variant::state(&file.rows[index])
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    };
    assert_eq!(keys(0), ["ac_id", "ac_text", "fr_id", "fr_statement"]);
    assert!(
        keys(1).contains(&"test_body".to_owned()) && !keys(1).contains(&"symbol_body".to_owned())
    );
    assert!(
        keys(2).contains(&"symbol_body".to_owned()) && !keys(2).contains(&"test_body".to_owned())
    );
}

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

fn scored_row(
    id: &str,
    expected: &str,
    answer: Option<(&str, f64, f64)>,
    kind: TruthKind,
) -> Scored {
    Scored {
        row_id: id.to_owned(),
        mode: Mode::ReqTestCode,
        kind,
        expected: expected.to_owned(),
        alternatives: Vec::new(),
        prediction: answer.map(|(answer, confidence, ordinal)| Prediction {
            answer: answer.to_owned(),
            confidence: Some(confidence),
            ordinal: Some(ordinal),
        }),
    }
}

/// Provenance: PLAT-1027. Ordering quality counts pairs by hand: truth
/// levels none < low < high, predicted ordinals 0.2, 2.0, 1.0. Pairs
/// (none,low) and (none,high) concordant, (low,high) discordant: C=2, D=1,
/// index 2/3, gamma 1/3. A constant prediction ties every pair: index 0.5.
#[test]
fn tc_1027_ordering_quality_counts_pairs() {
    let severity = eval_v2_support::keys::spec("severity").unwrap();
    let rows = [
        scored_row("a", "none", Some(("none", 0.9, 0.2)), TruthKind::Mechanical),
        scored_row(
            "b",
            "low",
            Some(("medium", 0.9, 2.0)),
            TruthKind::Mechanical,
        ),
        scored_row("c", "high", Some(("low", 0.9, 1.0)), TruthKind::Mechanical),
        scored_row("d", "high", None, TruthKind::Mechanical),
    ];
    let c = concordance(&rows, severity).unwrap();
    assert_eq!(
        (c.concordant, c.discordant, c.tied, c.excluded),
        (2, 1, 0, 1)
    );
    assert!((c.index().unwrap() - 2.0 / 3.0).abs() < 1e-9);
    assert!((c.gamma().unwrap() - 1.0 / 3.0).abs() < 1e-9);

    let constant: Vec<Scored> = ["none", "low", "high"]
        .iter()
        .map(|level| {
            scored_row(
                level,
                level,
                Some(("medium", 0.5, 2.0)),
                TruthKind::Mechanical,
            )
        })
        .collect();
    let c = concordance(&constant, severity).unwrap();
    assert_eq!((c.concordant, c.discordant, c.tied), (0, 0, 3));
    assert!((c.index().unwrap() - 0.5).abs() < 1e-9);
    assert_eq!(c.gamma(), None);
    let binary = eval_v2_support::keys::spec("trace_correct").unwrap();
    assert_eq!(concordance(&constant, binary), None);
}

/// Provenance: PLAT-1027. The coverage/accuracy curve keeps only rows at or
/// above each threshold; an unanswered row is in every total, never kept.
#[test]
fn tc_1027_the_coverage_curve_trades_coverage_for_accuracy() {
    let spec = eval_v2_support::keys::spec("code_exceeds_requirement").unwrap();
    let rows = [
        scored_row("a", "yes", Some(("yes", 0.95, 0.95)), TruthKind::Mechanical),
        scored_row("b", "no", Some(("no", 0.85, 0.15)), TruthKind::Mechanical),
        scored_row("c", "no", Some(("yes", 0.6, 0.6)), TruthKind::Mechanical),
        scored_row("d", "yes", None, TruthKind::Mechanical),
    ];
    let curve = coverage_curve(&graded(&rows, spec), &[0.5, 0.8, 0.9]);
    let points: Vec<(usize, usize, usize)> = curve
        .iter()
        .map(|p| (p.answered, p.correct, p.total))
        .collect();
    assert_eq!(points, [(3, 2, 4), (2, 2, 4), (1, 1, 4)]);

    let summary = summarize(&rows, spec);
    assert_eq!(summary.agreement, Some(50.0));
    assert_eq!(summary.baseline, 50.0);
    assert_eq!(summary.margin(), Some(0.0));
    assert_eq!(summary.defect_recall, Some(50.0));
    assert_eq!(summary.no_defect_recall, Some(50.0));
}

// ---------------------------------------------------------------------------
// The runner, end to end over a fake Jev
// ---------------------------------------------------------------------------

/// A fake Jev: answers every `noul` with `p_yes`, every `choice` with its
/// alphabetically first label, every `score` with 3.0. Counts requests.
struct FakeJev {
    p_yes: f64,
    calls: AtomicUsize,
}

#[async_trait]
impl Transport for FakeJev {
    async fn send(&self, request: Request) -> SdkResult<RawResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let body: Value = serde_json::from_str(request.body.as_deref().unwrap()).unwrap();
        let mut answers = serde_json::Map::new();
        for (key, question) in body["questions"].as_object().unwrap() {
            let answer = match question["type"].as_str().unwrap() {
                "noul" => json!({"type": "noul", "noul": self.p_yes}),
                "choice" => {
                    let label = question["criteria"]
                        .as_object()
                        .unwrap()
                        .keys()
                        .min()
                        .unwrap();
                    json!({"type": "choice", "choice": label, "confidence": 0.7,
                           "probabilities": {label: 0.7}})
                }
                _ => json!({"type": "score", "score": 3.0, "confidence": 0.6,
                            "legend": {}, "probabilities": {}}),
            };
            answers.insert(key.clone(), answer);
        }
        Ok(RawResponse {
            status: 200,
            headers: Headers::new(),
            body: json!({"model": "jev-fake", "answers": answers,
                         "usage": {"input_tokens": 1, "output_tokens": 1}})
            .to_string(),
        })
    }
}

fn fake_client(p_yes: f64) -> (typesafe_sdk_client::Client, Arc<FakeJev>) {
    let config =
        quoin_jev::config::resolve(&Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")])).unwrap();
    let fake = Arc::new(FakeJev {
        p_yes,
        calls: AtomicUsize::new(0),
    });
    (
        quoin_jev::client::with_transport(config, fake.clone()),
        fake,
    )
}

/// Provenance: PLAT-1027. Every baseline runs end to end: requests built,
/// the shared battery request sent once per row for all of B0/S0/E0/T0,
/// answers derived, graded, and rendered with the agent-labelled slice named.
#[tokio::test]
async fn tc_1027_the_runner_grades_every_baseline_end_to_end() {
    let file = four_modes();
    let (client, fake) = fake_client(0.9);
    let variants: Vec<&Variant> = REGISTRY.iter().collect();
    let output = variant::run(&client, &file.rows, &variants).await.unwrap();

    // One battery request (the RTC row) and one criterion-strength request:
    // the four rows share one requirement, so C0's request is identical on
    // all four. S0, E0 and T0 reuse B0's answer; C0 reuses its own three
    // times.
    assert_eq!(fake.calls.load(Ordering::SeqCst), 2);
    assert_eq!((output.requests_sent, output.requests_reused), (2, 6));
    assert_eq!(output.models.get("jev-fake"), Some(&2));

    let severity = scored(&file.rows, &output, "S0@v1", "severity");
    assert_eq!(severity.len(), 1);
    assert_eq!(severity[0].prediction.as_ref().unwrap().answer, "high");
    let exceeds = scored(&file.rows, &output, "E0@v1", "code_exceeds_requirement");
    assert_eq!(
        exceeds.len(),
        1,
        "E0 runs on RTC only; the RC row is not scored"
    );
    assert_eq!(exceeds[0].prediction.as_ref().unwrap().answer, "yes");
    // weakness_kind's alphabetically first label is `happy_path_only`.
    let sound = scored(&file.rows, &output, "C0@v1", "criterion_sound");
    assert_eq!(sound.len(), 2);
    assert!(
        sound
            .iter()
            .all(|row| row.prediction.as_ref().unwrap().answer == "no")
    );

    let report = render_run(&file.rows, &output, &variants);
    println!("{report}");
    assert!(report.contains("| AGENT-LABELLED | 1 |"), "{report}");
    assert!(report.contains("all [AGENT-LABELLED]"), "{report}");
    assert!(report.contains("### S0@v1 — `severity`"), "{report}");
}

fn unit_questions(_unit: &eval_v2_support::units::Unit) -> Questions {
    questions([(
        "unit_exceeds",
        noul("Does this unit do something the requirement does not state?"),
    )])
}

fn unit_asks(row: &eval_v2_support::corpus::Row) -> Vec<variant::Ask> {
    per_unit_asks(row, unit_questions)
}

fn unit_derive(_row: &eval_v2_support::corpus::Row, answered: &[Answered]) -> Predictions {
    rollup_any_yes(answered, "unit_exceeds")
        .map(|prediction| Predictions::from([("code_exceeds_requirement", prediction)]))
        .unwrap_or_default()
}

/// Provenance: PLAT-1027. Per-unit fan-out: a variant defined in five lines
/// asks once per unit of the row's code and rolls the answers up in code.
/// The test-local variant here is a mechanism check, not an experiment.
#[tokio::test]
async fn tc_1027_per_unit_fan_out_asks_each_unit_and_rolls_up() {
    const PER_UNIT: Variant = Variant {
        id: "UNIT-DEMO",
        version: 1,
        summary: "mechanism check only",
        modes: &[Mode::ReqCode],
        grades: &["code_exceeds_requirement"],
        asks: unit_asks,
        derive: unit_derive,
    };
    let file = four_modes();
    let rc = &file.rows[2];
    let asks = (PER_UNIT.asks)(rc);
    assert_eq!(
        asks.iter().map(|a| a.unit).collect::<Vec<_>>(),
        [Some(0), Some(1), Some(2)]
    );
    let state = serde_json::to_value(&asks[2].request.state).unwrap();
    assert_eq!(state["symbol_body"], "_ => Ok(())");
    assert_eq!(state["code_unit"], "match arm _");
    assert!(wording_violations(&PER_UNIT, rc).is_empty());

    let (client, fake) = fake_client(0.2);
    let output = variant::run(&client, &file.rows, &[&PER_UNIT])
        .await
        .unwrap();
    assert_eq!(fake.calls.load(Ordering::SeqCst), 3);
    let rows = scored(
        &file.rows,
        &output,
        "UNIT-DEMO@v1",
        "code_exceeds_requirement",
    );
    let prediction = rows[0].prediction.as_ref().unwrap();
    assert_eq!(prediction.answer, "no");
    assert!((prediction.confidence.unwrap() - 0.8).abs() < 1e-9);
    assert_eq!(rows[0].kind.group(), KindGroup::ByConstruction);
}
