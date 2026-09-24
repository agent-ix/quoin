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

use std::fmt::Write as _;
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
use eval_v2_support::fixtures::{self, CODE_BODY, corpus_text, row, truth};
use eval_v2_support::keys::{KEYS, Mode};
use eval_v2_support::metrics::{
    Scored, concordance, coverage_curve, graded, render_run, scored, summarize,
};
use eval_v2_support::patch::apply_unified_patch;
use eval_v2_support::preflight;
use eval_v2_support::units::{UnitKind, extract_rust_fn, split_rust_units};
use eval_v2_support::variant::{
    self, Answered, Prediction, Predictions, REGISTRY, Variant, per_unit_asks, rollup_any_yes,
    wording_violations,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn parse(rows: &[Value]) -> CorpusFile {
    fixtures::parse(rows).expect("synthetic corpus parses")
}

/// A four-row corpus, one per mode, with truth each mode supports.
fn four_modes() -> CorpusFile {
    fixtures::four_modes().expect("synthetic corpus parses")
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
/// shared schema, its held-out seal holds, and row ids are unique across
/// sources. PLAT-1025 landed the in-repo corpus, so this runs in the default
/// gate. It fails when it finds no corpus at all.
#[test]
fn tc_1027_the_committed_corpus_validates() {
    let mut sources = Vec::new();
    match corpus::load_in_repo().unwrap_or_else(|error| panic!("{error}")) {
        Some(source) => sources.push(source),
        None => println!(
            "in-repo corpus: {} does not exist (PLAT-1025)",
            corpus::in_repo_corpus_path().display()
        ),
    }
    match corpus::load_external_from_env().unwrap_or_else(|error| panic!("{error}")) {
        Some(source) => sources.push(source),
        None => println!("external corpus: {} is unset", corpus::EXTERNAL_CORPUS_ENV),
    }
    assert!(!sources.is_empty(), "no corpus found: nothing was checked");
    for source in &sources {
        print_and_check(source);
        for excluded in &source.excluded {
            println!("EXCLUDED {}: {}", excluded.id, excluded.reason);
        }
    }
    corpus::combined_rows(&sources, corpus::Split::Dev).unwrap_or_else(|error| panic!("{error}"));
    println!("{} corpus file(s) validated", sources.len());
}

/// A mutant of `source` (an id in the same corpus, or none), in `split`.
fn mutant(id: &str, split: &str, source: Option<&str>) -> Value {
    let mut value = row(
        id,
        Mode::ReqTest,
        split,
        &json!({"test_asserts_intent": truth(&json!("no"), "by_construction", &[])}),
    );
    value["mutation"] = json!({"id": format!("M-{id}"), "target": "test", "kind": "test_weakening",
                               "description": "drops the assertion", "patch": "-assert", "source_id": source});
    value
}

/// Provenance: PLAT-1025. A mutant's `source_id` names an unmutated row of the
/// same file in the same split; a dangling id, a mutant source, or a source in
/// the other split is each refused, and a mutant with no source in the corpus
/// (`null`) is allowed.
#[test]
fn tc_1025_a_mutant_names_an_unmutated_source_in_its_own_split() {
    let source = row(
        "EV2-0001",
        Mode::ReqTest,
        "dev",
        &json!({"test_asserts_intent": truth(&json!("yes"), "agent_dual", &[])}),
    );
    let ok = [
        source.clone(),
        mutant("EV2-0002", "dev", Some("EV2-0001")),
        mutant("EV2-0003", "heldout", None),
    ];
    assert_eq!(validate(&parse(&ok), Origin::InRepo), Vec::<String>::new());
    let bad = [
        source,
        mutant("EV2-0002", "dev", Some("EV2-0999")),
        mutant("EV2-0003", "dev", Some("EV2-0002")),
        mutant("EV2-0004", "heldout", Some("EV2-0001")),
    ];
    let problems = validate(&parse(&bad), Origin::InRepo);
    assert_eq!(
        problems,
        [
            "EV2-0002: mutation.source_id EV2-0999 is not a row of this file",
            "EV2-0003: mutation.source_id EV2-0002 is itself a mutant",
            "EV2-0004: split heldout but its source EV2-0001 is dev",
        ]
    );
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
                rerun_reason: None,
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

fn external_row(id: &str, sha: &str, path: &str, repo: &str) -> Value {
    let mut value = row(
        id,
        Mode::ReqTestCode,
        "dev",
        &json!({"code_implements_intent": truth(&json!(false), "mechanical", &[])}),
    );
    value["test"] = json!({"path": path, "fn_name": "adds"});
    value["code"] = json!({"path": path, "symbol": "Adder::add"});
    value["ref"] = json!({
        "repo": repo, "commit": "deadbeef",
        "paths": {"test": path, "code": path},
        "sha256": {"test": sha, "code": sha},
    });
    value["mutation"] = json!({
        "id": "M1", "target": "code", "kind": "swap_operator",
        "description": "+ becomes *", "patch": EXTERNAL_PATCH,
    });
    value
}

fn write_external(dir: &std::path::Path, rows: &[Value]) -> std::path::PathBuf {
    let corpus = dir.join("external.json");
    std::fs::write(&corpus, corpus_text(rows)).unwrap();
    corpus
}

fn external_corpus(dir: &std::path::Path, sha: &str, path: &str) -> std::path::PathBuf {
    write_external(
        dir,
        &[external_row("EVX-0001", sha, path, "agent-ix/sibling")],
    )
}

/// A checkout root holding `sibling/src/lib.rs` = [`EXTERNAL_SOURCE`], and
/// that file's stored sha256.
fn checkout() -> (tempfile::TempDir, String) {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("sibling/src");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("lib.rs"), EXTERNAL_SOURCE).unwrap();
    let sha = quoin_store::digest_bytes_sha256(EXTERNAL_SOURCE.as_bytes()).to_stored();
    (root, sha)
}

/// The one excluded row's reason, after loading `corpus` against `root`.
fn only_exclusion(corpus: &std::path::Path, root: &std::path::Path) -> String {
    let source = load_external(Some(corpus), Some(root)).unwrap().unwrap();
    assert!(source.file.rows.is_empty(), "{:?}", source.file.rows);
    assert_eq!(source.excluded.len(), 1, "{:?}", source.excluded);
    source.excluded[0].reason.clone()
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
/// digest, or a path that escapes the repo, excludes that row with its
/// reason; a missing root fails the whole load (configuration, not data).
#[test]
fn tc_1027_an_external_row_is_refused_when_its_content_moved() {
    let (root, _) = checkout();
    let stale = external_corpus(root.path(), &"a".repeat(64), "src/lib.rs");
    let reason = only_exclusion(&stale, root.path());
    assert!(reason.contains("does not match the row's"), "{reason}");

    let no_root = load_external(Some(&stale), None).unwrap_err();
    assert!(no_root.contains(corpus::EXTERNAL_ROOT_ENV), "{no_root}");

    let escaping = external_corpus(root.path(), &"a".repeat(64), "../../etc/passwd");
    let reason = only_exclusion(&escaping, root.path());
    assert!(
        reason.contains("not a plain repo-relative path"),
        "{reason}"
    );
}

/// Provenance: PLAT-1027 (review finding 1). `ref.repo` names a directory
/// under the root and nothing else: `agent-ix/..` would make the checkout
/// the root's parent.
#[test]
fn tc_1027_a_repo_name_cannot_escape_the_root() {
    let (root, sha) = checkout();
    for repo in ["agent-ix/..", "agent-ix/.", "..", "agent-ix/", "a\\..\\b"] {
        let corpus = write_external(
            root.path(),
            &[external_row("EVX-0001", &sha, "src/lib.rs", repo)],
        );
        let reason = only_exclusion(&corpus, root.path());
        assert!(reason.contains("plain repository name"), "{repo}: {reason}");
    }
}

/// Provenance: PLAT-1027 (review finding 2). A symlink inside the checkout
/// that points outside the root is refused before it is read.
#[cfg(unix)]
#[test]
fn tc_1027_a_symlink_cannot_escape_the_root() {
    let (root, sha) = checkout();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("lib.rs"), EXTERNAL_SOURCE).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("lib.rs"),
        root.path().join("sibling/src/link.rs"),
    )
    .unwrap();
    let corpus = write_external(
        root.path(),
        &[external_row(
            "EVX-0001",
            &sha,
            "src/link.rs",
            "agent-ix/sibling",
        )],
    );
    let reason = only_exclusion(&corpus, root.path());
    assert!(reason.contains("outside"), "{reason}");
}

/// Provenance: PLAT-1027 (review finding 7). One bad external row is
/// excluded with its reason and the good ones load; the report lists it.
#[test]
fn tc_1027_one_bad_external_row_is_excluded_and_reported() {
    let (root, sha) = checkout();
    let corpus = write_external(
        root.path(),
        &[
            external_row("EVX-0001", &sha, "src/lib.rs", "agent-ix/sibling"),
            external_row("EVX-0002", &sha, "src/missing.rs", "agent-ix/sibling"),
        ],
    );
    let source = load_external(Some(&corpus), Some(root.path()))
        .unwrap()
        .unwrap();
    assert_eq!(source.file.rows.len(), 1);
    assert_eq!(source.excluded.len(), 1);
    assert_eq!(source.excluded[0].id, "EVX-0002");
    let report = render_run(
        &source.file.rows,
        &source.excluded,
        &variant::RunOutput::default(),
        &[],
    );
    assert!(report.contains("1 row(s) EXCLUDED at load"), "{report}");
    assert!(report.contains("- EXCLUDED EVX-0002: "), "{report}");
}

const SPEC_FILE: &str = "# FR-002\n\nThe system shall refuse a request\nlarger than 4096 bytes.\n\n- AC-1: A 5000-byte request is refused.\n";

const SPEC_PATCH: &str = "--- a/spec/FR-002.md\n+++ b/spec/FR-002.md\n@@ -3,2 +3,2 @@\n The system shall refuse a request\n-larger than 4096 bytes.\n+larger than 8192 bytes.\n";

fn requirement_mutation_row(sha: &str, statement: &str, with_path: bool) -> Value {
    let mut value = row(
        "EVX-0002",
        Mode::Req,
        "dev",
        &json!({"criterion_sound": truth(&json!(true), "by_construction", &[])}),
    );
    value["requirement"]["statement"] = json!(statement);
    value["requirement"]["ac_text"] = json!("A 5000-byte request is refused.");
    let paths = if with_path {
        json!({"requirement": "spec/FR-002.md"})
    } else {
        json!({})
    };
    value["ref"] = json!({"repo": "agent-ix/sibling", "commit": "c",
                          "paths": paths, "sha256": {"requirement": sha}});
    value["mutation"] = json!({"id": "M2", "target": "requirement", "kind": "threshold",
                               "description": "4096 becomes 8192", "patch": SPEC_PATCH});
    value
}

/// Provenance: PLAT-1027 (review finding 14). A requirement mutation is
/// applied to the spec file and checked: the patched file must carry the
/// row's statement and AC text. Without a requirement path it is refused.
#[test]
fn tc_1027_a_requirement_mutation_is_checked_against_the_patched_spec() {
    let (root, _) = checkout();
    std::fs::create_dir_all(root.path().join("sibling/spec")).unwrap();
    std::fs::write(root.path().join("sibling/spec/FR-002.md"), SPEC_FILE).unwrap();
    let sha = quoin_store::digest_bytes_sha256(SPEC_FILE.as_bytes()).to_stored();

    let good = "The system shall refuse a request larger than 8192 bytes.";
    let corpus = write_external(root.path(), &[requirement_mutation_row(&sha, good, true)]);
    let source = load_external(Some(&corpus), Some(root.path()))
        .unwrap()
        .unwrap();
    assert_eq!(
        (source.file.rows.len(), source.excluded.len()),
        (1, 0),
        "{:?}",
        source.excluded
    );

    let stale = "The system shall refuse a request larger than 4096 bytes.";
    let corpus = write_external(root.path(), &[requirement_mutation_row(&sha, stale, true)]);
    let reason = only_exclusion(&corpus, root.path());
    assert!(
        reason.contains("does not contain the row's statement"),
        "{reason}"
    );

    let corpus = write_external(root.path(), &[requirement_mutation_row(&sha, good, false)]);
    let reason = only_exclusion(&corpus, root.path());
    assert!(
        reason.contains("targets requirement but ref.paths has no such file"),
        "{reason}"
    );
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
    assert!(rt.iter().any(|v| v.contains("the code")), "{rt:#?}");
    assert!(!rt.iter().any(|v| v.contains("the test")), "{rt:#?}");
    let r = wording_violations(&misapplied, &file.rows[0]);
    assert!(r.iter().any(|v| v.contains("the test")), "{r:#?}");
    assert!(r.iter().any(|v| v.contains("the code")), "{r:#?}");
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
/// alphabetically first label, every `score` with 3.0 (all mass on level 3).
/// Counts requests.
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
                            "legend": {}, "probabilities": {"3": 1.0}}),
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
    // The baselines only: experiment variants add their own requests, and
    // each ticket's own tests count those.
    let variants = variant::resolve("B0,S0,E0,T0,C0").unwrap();
    let output = variant::run(&client, &file.rows, &variants).await.unwrap();

    // One battery request (the RTC row) and one criterion-strength request
    // per row (four distinct requirements). S0, E0 and T0 reuse B0's answer.
    assert_eq!(fake.calls.load(Ordering::SeqCst), 5);
    assert_eq!((output.requests_sent, output.requests_reused), (5, 3));
    assert_eq!(output.models.get("jev-fake"), Some(&5));

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

    let report = render_run(&file.rows, &[], &output, &variants);
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
        references: &[variant::Artifact::Code],
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

// ---------------------------------------------------------------------------
// Review findings (PLAT-1027, PR #617)
// ---------------------------------------------------------------------------

/// A test-only variant whose single question's text is the row's
/// `requirement.context`, so one fn pointer can carry any wording.
fn phrase_asks(row: &eval_v2_support::corpus::Row) -> Vec<variant::Ask> {
    let text = row.requirement.context.clone().unwrap_or_default();
    vec![variant::Ask {
        unit: None,
        request: variant::request(variant::state(row), questions([("q", noul(text))])),
    }]
}

/// As [`phrase_asks`], but its state also smuggles a code field.
fn leaky_asks(row: &eval_v2_support::corpus::Row) -> Vec<variant::Ask> {
    let mut state = variant::state(row);
    state["symbol_body"] = json!("fn f() {}");
    vec![variant::Ask {
        unit: None,
        request: variant::request(state, questions([("q", noul("Is the criterion clear?"))])),
    }]
}

fn no_derive(_row: &eval_v2_support::corpus::Row, _answered: &[Answered]) -> Predictions {
    Predictions::new()
}

const PHRASE: Variant = Variant {
    id: "PHRASE",
    version: 1,
    summary: "wording-rule probe",
    modes: &Mode::ALL,
    references: &[],
    grades: &[],
    asks: phrase_asks,
    derive: no_derive,
};

fn row_with_context(mode: Mode, context: &str) -> eval_v2_support::corpus::Row {
    let mut value = row("EV2-0001", mode, "dev", &json!({}));
    value["requirement"]["context"] = json!(context);
    parse(&[value]).rows.remove(0)
}

/// Provenance: PLAT-1027 (review finding 3). Paraphrases of an absent
/// artifact are caught in question text, and a state field of an absent
/// artifact is caught by name.
#[test]
fn tc_1027_the_wording_rule_catches_paraphrases_and_leaky_state() {
    for (phrase, artifact) in [
        ("Does the covered code do what it says?", "the code"),
        ("Is the implementation under test complete?", "the code"),
        ("Does the function body return early?", "the code"),
        ("Does the source code log?", "the code"),
        ("Would a unit test catch this?", "the test"),
    ] {
        let found = wording_violations(&PHRASE, &row_with_context(Mode::Req, phrase));
        assert!(
            found
                .iter()
                .any(|v| v.contains(artifact) && v.contains("question text")),
            "{phrase}: {found:#?}"
        );
    }
    let leaky = Variant {
        asks: leaky_asks,
        ..PHRASE
    };
    let found = wording_violations(&leaky, &row_with_context(Mode::ReqTest, ""));
    assert!(
        found
            .iter()
            .any(|v| v.contains("state field \"symbol_body\"")),
        "{found:#?}"
    );
    // Declared nothing, asks about code on a full triple: the declaration
    // must not lie even where the mode has the artifact.
    let found = wording_violations(
        &PHRASE,
        &row_with_context(Mode::ReqTestCode, "Does the code log?"),
    );
    assert!(
        found.iter().any(|v| v.contains("does not declare")),
        "{found:#?}"
    );
}

/// Provenance: PLAT-1027 (review finding 4). The rule never reads the row's
/// own text: a requirement that says "the test runner shall report the code
/// of each failure" is not a question about a test or code.
#[test]
fn tc_1027_the_wording_rule_ignores_row_content() {
    let mut value = row(
        "EV2-0001",
        Mode::Req,
        "dev",
        &json!({"criterion_sound": truth(&json!(true), "agent_dual", &[])}),
    );
    value["requirement"]["statement"] =
        json!("The test runner shall report the code of each failure.");
    value["requirement"]["ac_text"] = json!("Each failed test's exit code is printed.");
    let file = parse(&[value]);
    assert_eq!(
        wording_violations(&variant::C0, &file.rows[0]),
        Vec::<String>::new()
    );
}

/// Provenance: PLAT-1027 (review finding 5). Row ids are unique across the
/// in-repo and external corpora combined, and each source has its prefix.
#[test]
fn tc_1027_row_ids_are_unique_across_sources_and_prefixed() {
    let dir = tempfile::tempdir().unwrap();
    let in_repo = sealed_source(dir.path(), None);
    let mut external = in_repo.clone();
    external.origin = Origin::External;
    let error = corpus::combined_rows(&[in_repo.clone(), external.clone()], corpus::Split::Dev)
        .unwrap_err();
    assert!(
        error.contains("row id EV2-0001 appears more than once"),
        "{error}"
    );
    assert_eq!(
        corpus::combined_rows(&[in_repo], corpus::Split::Dev)
            .unwrap()
            .len(),
        1
    );

    let problems = validate(&external.file, Origin::External);
    assert!(
        problems
            .iter()
            .any(|p| p == "EV2-0001: id lacks this source's prefix EVX-"),
        "{problems:#?}"
    );
}

/// Provenance: PLAT-1027 (review finding 5). The runner refuses rows with a
/// duplicate id rather than letting one result overwrite another.
#[tokio::test]
async fn tc_1027_the_runner_refuses_duplicate_row_ids() {
    let mut file = four_modes();
    let twin = file.rows[0].clone();
    file.rows.push(twin);
    let (client, fake) = fake_client(0.9);
    let error = variant::run(&client, &file.rows, &[&variant::C0])
        .await
        .unwrap_err();
    assert!(error.contains("EV2-0001 appears more than once"), "{error}");
    assert_eq!(fake.calls.load(Ordering::SeqCst), 0);
}

/// Provenance: PLAT-1027 (review finding 6). Extraction keeps a multi-line
/// attribute and a plain comment interleaved with doc comments.
#[test]
fn tc_1027_extraction_keeps_multi_line_attributes_and_comments() {
    let source = "use x;\n\n#[allow(\n    clippy::panic,\n    reason = \"a ] in a string\"\n)]\n/// Trace: FR-001\n// a plain note\n#[test]\nfn probe() {\n    panic!();\n}\n";
    assert_eq!(
        extract_rust_fn(source, "probe").unwrap(),
        "#[allow(\n    clippy::panic,\n    reason = \"a ] in a string\"\n)]\n/// Trace: FR-001\n// a plain note\n#[test]\nfn probe() {\n    panic!();\n}"
    );
    let interleaved = "let v = x[0];\n/// doc\n// note\n/// more doc\nfn after() {}\n";
    assert_eq!(
        extract_rust_fn(interleaved, "after").unwrap(),
        "/// doc\n// note\n/// more doc\nfn after() {}"
    );
}

/// Provenance: PLAT-1027 (review finding 7). `Type::method` picks the method
/// in `impl Type`; a `fn` nested in another `fn` is never the top-level
/// match.
#[test]
fn tc_1027_a_qualified_symbol_picks_its_impl() {
    let source =
        "impl A { fn run() -> u8 { 1 } }\nimpl<T> Trait for B<T> { fn run() -> u8 { 2 } }\n";
    assert_eq!(
        extract_rust_fn(source, "run").unwrap_err(),
        "`fn run` is ambiguous: 2 definitions in the source"
    );
    assert!(
        extract_rust_fn(source, "B::run")
            .unwrap()
            .ends_with("{ 2 }")
    );
    assert!(
        extract_rust_fn(source, "crate::a::A::run")
            .unwrap()
            .ends_with("{ 1 }")
    );
    let nested = "fn outer() {\n    fn helper() -> u8 { 1 }\n}\n\nfn helper() -> u8 { 2 }\n";
    assert_eq!(
        extract_rust_fn(nested, "helper").unwrap(),
        "fn helper() -> u8 { 2 }"
    );
}

/// Provenance: PLAT-1027 (review finding 8). C0's `criterion_sound`
/// confidence is P(sound) for `yes` and 1 - P(sound) for `no`, not the
/// confidence in whichever weakness label was chosen.
#[test]
fn tc_1027_c0_confidence_is_the_probability_of_its_answer() {
    let file = four_modes();
    let row = &file.rows[0];
    let answer = |label: &str| {
        let mut answers = variant::RawAnswers::new();
        answers.insert(
            "FR-001-AC-1::weakness_kind".to_owned(),
            variant::RawAnswer::Choice {
                label: label.to_owned(),
                confidence: 0.5,
                probabilities: [("sound".to_owned(), 0.2), ("unfalsifiable".to_owned(), 0.5)]
                    .into_iter()
                    .collect(),
            },
        );
        let answered = [Answered {
            unit: None,
            answers,
        }];
        (variant::C0.derive)(row, &answered)["criterion_sound"].clone()
    };
    let no = answer("unfalsifiable");
    assert_eq!(no.answer, "no");
    assert!((no.confidence.unwrap() - 0.8).abs() < 1e-9, "{no:?}");
    let yes = answer("sound");
    assert_eq!(yes.answer, "yes");
    assert!((yes.confidence.unwrap() - 0.2).abs() < 1e-9, "{yes:?}");
}

/// Provenance: PLAT-1027 (review finding 9). A variant version already on
/// the held-out log is refused unless a rerun reason is given, which is
/// logged.
#[test]
fn tc_1027_a_heldout_rerun_needs_a_reason() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("heldout-runs.jsonl");
    let labels = vec!["S0@v1".to_owned()];
    corpus::check_heldout_rerun(&log, &labels, None).unwrap();
    corpus::record_heldout_run(
        &log,
        &corpus::HeldoutRun {
            unix_seconds: 1,
            variants: labels.clone(),
            seals: Vec::new(),
            rows: 1,
            models: std::collections::BTreeMap::new(),
            rerun_reason: None,
        },
    )
    .unwrap();
    let refused = corpus::check_heldout_rerun(&log, &labels, None).unwrap_err();
    assert!(refused.contains(corpus::HELDOUT_RERUN_ENV), "{refused}");
    corpus::check_heldout_rerun(&log, &["S1@v1".to_owned()], None).unwrap();
    corpus::check_heldout_rerun(&log, &labels, Some("service outage mid-run")).unwrap();
    corpus::record_heldout_run(
        &log,
        &corpus::HeldoutRun {
            unix_seconds: 2,
            variants: labels.clone(),
            seals: Vec::new(),
            rows: 1,
            models: std::collections::BTreeMap::new(),
            rerun_reason: Some("service outage mid-run".to_owned()),
        },
    )
    .unwrap();
    let text = std::fs::read_to_string(&log).unwrap();
    assert!(
        text.contains("\"rerun_reason\":\"service outage mid-run\""),
        "{text}"
    );
}

/// Provenance: PLAT-1027 (review finding 10). A source with no held-out rows
/// needs no seal; one with held-out rows does.
#[test]
fn tc_1027_a_source_without_heldout_rows_needs_no_seal() {
    let dir = tempfile::tempdir().unwrap();
    let text = corpus_text(&[row(
        "EV2-0001",
        Mode::Req,
        "dev",
        &json!({"compound": truth(&json!(false), "agent_dual", &[])}),
    )]);
    let path = dir.path().join("corpus.json");
    std::fs::write(&path, text).unwrap();
    let source = corpus::read_source(&path, Origin::InRepo).unwrap();
    assert_eq!(
        verify_seal(&source).unwrap(),
        "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945"
    );
    let heldout = sealed_source(tempfile::tempdir().unwrap().path(), None);
    assert!(verify_seal(&heldout).unwrap_err().contains("not sealed"));
}

/// Provenance: PLAT-1027 (review finding 11). A zero-context insertion
/// `@@ -1,0 +2 @@` goes after line 1, not before it.
#[test]
fn tc_1027_a_pure_insertion_lands_after_its_line() {
    let patched = apply_unified_patch("a\nb\n", "@@ -1,0 +2 @@\n+x\n").unwrap();
    assert_eq!(patched, "a\nx\nb\n");
    let at_top = apply_unified_patch("a\nb\n", "@@ -0,0 +1 @@\n+x\n").unwrap();
    assert_eq!(at_top, "x\na\nb\n");
}

/// Provenance: PLAT-1027 (review finding 12), MP-226. ECE compares each
/// decile's MEAN stated confidence with its accuracy: two rows at 0.91, one
/// right, give |0.91 - 0.5| = 0.41 (the midpoint rule gave 0.45). The v2
/// summary reports the rows ECE leaves out.
#[test]
fn tc_1027_ece_uses_the_mean_confidence_per_bucket() {
    let spec = eval_v2_support::keys::spec("code_exceeds_requirement").unwrap();
    let rows = [
        scored_row("a", "yes", Some(("yes", 0.91, 0.91)), TruthKind::Mechanical),
        scored_row("b", "no", Some(("yes", 0.91, 0.91)), TruthKind::Mechanical),
        scored_row("c", "no", None, TruthKind::Mechanical),
    ];
    let summary = summarize(&rows, spec);
    assert!(
        (summary.ece.unwrap() - 0.41).abs() < 1e-9,
        "{:?}",
        summary.ece
    );
    assert_eq!(summary.without_confidence, 1);
}

/// Provenance: PLAT-1027 (review finding 15), PLAT-1024 corpus rule 1. At
/// most three natural rows per FR; mutated rows do not count.
#[test]
fn tc_1027_at_most_three_natural_rows_per_fr() {
    let same_fr = |id: &str, mutated: bool| {
        let mut value = row(
            id,
            Mode::Req,
            "dev",
            &json!({"compound": truth(&json!(false), "agent_dual", &[])}),
        );
        value["strata"]["fr_id"] = json!("FR-900");
        value["requirement"]["fr_id"] = json!("FR-900");
        if mutated {
            value["mutation"] = json!({"id": id, "target": "requirement", "kind": "k",
                                       "description": "d", "patch": "p"});
        }
        value
    };
    let three_plus_mutant = parse(&[
        same_fr("EV2-0001", false),
        same_fr("EV2-0002", false),
        same_fr("EV2-0003", false),
        same_fr("EV2-0004", true),
    ]);
    assert_eq!(
        validate(&three_plus_mutant, Origin::InRepo),
        Vec::<String>::new()
    );
    let four = parse(&[
        same_fr("EV2-0001", false),
        same_fr("EV2-0002", false),
        same_fr("EV2-0003", false),
        same_fr("EV2-0004", false),
    ]);
    assert_eq!(
        validate(&four, Origin::InRepo),
        [
            "FR-900 in quoin: 4 natural rows (EV2-0001, EV2-0002, EV2-0003, EV2-0004), \
          at most 3 per FR per repo"
        ]
    );
}

// ---------------------------------------------------------------------------
// Request-digest pins: wording is tied to variant@version (PR #620 review)
// ---------------------------------------------------------------------------

/// Provenance: PR #620 review, PLAT-1024 rule 5. Every (variant version,
/// mode) has a pinned wording digest, and each pin matches: wording cannot
/// change without a version bump.
#[test]
fn tc_1027_request_digest_is_pinned_per_variant_version() {
    let registry: Vec<&Variant> = REGISTRY.iter().collect();
    let actual = preflight::request_digests(&registry).unwrap();
    let pinned: Vec<(String, &str, String)> = preflight::REQUEST_DIGEST_PINS
        .iter()
        .map(|(label, mode, digest)| ((*label).to_owned(), *mode, (*digest).to_owned()))
        .collect();
    let mut table = String::new();
    for (label, mode, digest) in &actual {
        let _ = writeln!(table, "    ({label:?}, {mode:?}, {digest:?}),");
    }
    assert_eq!(
        actual, pinned,
        "request digests differ from REQUEST_DIGEST_PINS. If a variant's wording changed, bump its \
         version first; then pin:\n{table}"
    );
    preflight::check_request_pins(&registry).unwrap();
}

// ---------------------------------------------------------------------------
// The run preflight: pins, the version cap, held-out selection (PR #620
// re-review, findings 3a, 4 and 5)
// ---------------------------------------------------------------------------

fn other_wording(row: &eval_v2_support::corpus::Row) -> Vec<variant::Ask> {
    vec![variant::Ask {
        unit: None,
        request: variant::request(
            variant::state(row),
            questions([("severity", noul("A wording nobody pinned?"))]),
        ),
    }]
}

/// Provenance: PR #620 re-review finding 4, MP-240. The runner's preflight
/// refuses a `variant@version` whose wording differs from its pin, a bumped
/// version with no pin, and a version past the 5-version dev cap; the
/// registry as committed passes on dev.
#[test]
fn tc_1027_the_preflight_refuses_unpinned_wording_and_versions_past_the_cap() {
    let dir = tempfile::tempdir().unwrap();
    let selection = dir.path().join("selection.json");
    let log = dir.path().join("heldout-runs.jsonl");
    let gate = preflight::RunGate {
        split: corpus::Split::Dev,
        heldout_flag: None,
        rerun_reason: None,
        selection: &selection,
        log: &log,
    };
    let registry: Vec<&Variant> = REGISTRY.iter().collect();
    assert_eq!(
        preflight::authorize_run(&gate, &registry, &[]).unwrap(),
        Vec::<String>::new()
    );

    let s1_rt = variant::resolve("S1-RT").unwrap()[0];
    let reworded = Variant {
        asks: other_wording,
        ..*s1_rt
    };
    let error = preflight::authorize_run(&gate, &[&reworded], &[]).unwrap_err();
    assert!(
        error.contains("S1-RT@v1 in RT asks wording sha256:")
            && error.contains("a wording change needs a version bump"),
        "{error}"
    );

    let bumped = Variant {
        version: 2,
        ..*s1_rt
    };
    let error = preflight::authorize_run(&gate, &[&bumped], &[]).unwrap_err();
    assert!(
        error.contains("S1-RT@v2 in RT has no request-digest pin"),
        "{error}"
    );

    assert_eq!(preflight::MAX_DEV_VERSIONS, 5);
    let past_cap = Variant {
        version: 6,
        ..*s1_rt
    };
    let error = preflight::authorize_run(&gate, &[&past_cap], &[]).unwrap_err();
    assert!(error.contains("past the dev cap of 5 versions"), "{error}");
}

/// Provenance: PR #620 re-review finding 5, PLAT-1024 rule 1. On held-out,
/// the preflight the live runner calls refuses a variant the committed
/// selection file does not cover, a missing selection file, a missing flag,
/// and an unexplained rerun; a covered variant over a sealed source passes
/// and returns the seal.
#[test]
fn tc_1027_the_preflight_runs_heldout_only_for_the_committed_selection() {
    let dir = tempfile::tempdir().unwrap();
    let unsealed = sealed_source(dir.path(), None);
    let digest = heldout_digest(&unsealed.text).unwrap();
    let sealed = sealed_source(dir.path(), Some(&format!("sha256:{digest}\n")));
    let selection = dir.path().join("selection.json");
    let log = dir.path().join("heldout-runs.jsonl");
    let gate = preflight::RunGate {
        split: corpus::Split::Heldout,
        heldout_flag: Some("1"),
        rerun_reason: None,
        selection: &selection,
        log: &log,
    };
    let s2_rt = variant::resolve("S2-RT").unwrap();
    let s1 = variant::resolve("S1").unwrap();

    let missing = preflight::authorize_run(&gate, &s2_rt, &[&sealed]).unwrap_err();
    assert!(missing.contains("selection.json"), "{missing}");

    std::fs::write(
        &selection,
        json!({"schema": corpus::SELECTION_SCHEMA, "selections": [
            {"mp": "MP-240", "variant": "S2", "version": 1, "selected_at_commit": "abc1234",
             "dev_evidence": "reviews/dev.md", "baselines": ["S0@v1"]}
        ]})
        .to_string(),
    )
    .unwrap();
    assert_eq!(
        preflight::authorize_run(&gate, &s2_rt, &[&sealed]).unwrap(),
        vec![digest.clone()]
    );
    let refused = preflight::authorize_run(&gate, &s1, &[&sealed]).unwrap_err();
    assert!(
        refused.contains("S1@v1") && refused.contains("not selected for held-out"),
        "{refused}"
    );

    let no_flag = preflight::RunGate {
        heldout_flag: None,
        ..gate
    };
    let refused = preflight::authorize_run(&no_flag, &s2_rt, &[&sealed]).unwrap_err();
    assert!(refused.contains(HELDOUT_ENV), "{refused}");

    corpus::record_heldout_run(
        &log,
        &corpus::HeldoutRun {
            unix_seconds: 1,
            variants: vec!["S2-RT@v1".to_owned()],
            seals: vec![digest],
            rows: 1,
            models: std::collections::BTreeMap::new(),
            rerun_reason: None,
        },
    )
    .unwrap();
    let rerun = preflight::authorize_run(&gate, &s2_rt, &[&sealed]).unwrap_err();
    assert!(rerun.contains(corpus::HELDOUT_RERUN_ENV), "{rerun}");
}
