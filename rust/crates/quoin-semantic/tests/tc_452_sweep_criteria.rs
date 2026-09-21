// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-071, FR-072 and FR-074, criterion by criterion (quoin#452).
//!
//! The mapping fixtures under `tests/fixtures/semantic-module/mapping` are
//! published by quoin and executed by quire; what quoin owns, and what these
//! restate, is that quoin's own legacy-form classifier agrees with the
//! expectations recorded beside them, that the fence subset stays a subset, and
//! that `legacy_forms: error` is admissible only with a usable sweep report.
//!
//! The half these do NOT take over is the ajv validation of every expected
//! declaration against the vendored semantic-core bundle: that lives in
//! `tests/semantic-mapping.test.ts`, which survives the cutover because the
//! fixtures are a contract with a JavaScript consumer.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{Scratch, codes, mapping_dir, mapping_fixture, mapping_json, validators};
use quoin_semantic::sweep::{CorpusRoot, SweepIdentity};
use quoin_semantic::{PropertiesForm, TYPED_HEADER, classify_artifact, sweep_corpus};
use serde_json::{Value, json};

/// The classifier's own text, read at compile time.
///
/// `include_str!` and not `fs::read_to_string`: a source-reading guard that
/// opens a path at run time is a guard that passes when the path is wrong.
const SWEEP_SOURCE: &str = include_str!("../src/sweep.rs");

/// One `legacy_forms: error` case: a module name, the `sweep_report` path it
/// declares, and the edit that spoils the report it ships.
type Case = (&'static str, Option<&'static str>, Box<dyn Fn(&mut Value)>);

/// The corpus the deleted TypeScript swept: four mapping fixtures and the
/// pinned FR-006 copy, under one root.
fn corpus(scratch: &Scratch) -> PathBuf {
    let root = scratch.path().join("corpus");
    let dir = root.join("spec").join("functional");
    fs::create_dir_all(&dir).unwrap();
    for name in [
        "config-version.table.md",
        "legacy-bullets.md",
        "legacy-mixed.md",
        "config-version.fence.md",
    ] {
        fs::copy(mapping_dir().join(name), dir.join(name)).unwrap();
    }
    fs::copy(
        mapping_dir()
            .join("..")
            .join("corpus")
            .join("config-service")
            .join("spec")
            .join("functional")
            .join("FR-006-config-version-entity.md"),
        dir.join("FR-006.md"),
    )
    .unwrap();
    root
}

/// The typed Properties table fixture is the authored form, and the classifier
/// recognises it by the four-column header the contract names.
///
/// Trace: FR-071-AC-1
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_650_the_typed_table_fixture_classifies_as_the_authored_form() {
    let table = mapping_fixture("config-version.table.md");
    let header = format!("| {} |", TYPED_HEADER.join(" | "));
    assert!(table.contains(&header), "the fixture carries {header}");
    let finding = classify_artifact("table", &table);
    assert_eq!(finding.form, PropertiesForm::TypedTable);
    assert!(finding.diagnostic.is_none(), "the authored form is clean");

    // The expectations recorded beside the fixture are at the semantic-core
    // version this quoin vendors, so the two are talking about one contract.
    let expected = mapping_json("config-version.expected.json");
    assert_eq!(
        expected["semanticCore"],
        quoin_semantic::SEMANTIC_CONTRACT.semantic_core.version
    );
    assert_eq!(
        expected["fields"].as_array().expect("fields").len(),
        7,
        "the FR-006 declaration set"
    );
}

/// The sysml fence fixture is a second authored form over the same
/// declarations, and it classifies as the fence rather than as a legacy form.
///
/// Trace: FR-071-AC-2
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_651_the_fence_fixture_declares_the_tables_rows_and_classifies_as_the_fence() {
    let fence = mapping_fixture("config-version.fence.md");
    let finding = classify_artifact("fence", &fence);
    assert_eq!(finding.form, PropertiesForm::SysmlFence);
    assert!(finding.diagnostic.is_none());

    let expected = mapping_json("config-version.expected.json");
    let authored = &expected["authoredForm"];
    assert_eq!(authored["config-version.table.md"], "table");
    assert_eq!(authored["config-version.fence.md"], "fence");

    // One expected declaration set serves both artifacts, so the fence must
    // declare exactly the table's rows, in order.
    let declared: Vec<String> = fence
        .lines()
        .filter_map(|line| {
            let rest = line
                .strip_prefix("attribute ")
                .or_else(|| line.strip_prefix("ref item "))?;
            rest.split(" :").next().map(str::to_owned)
        })
        .collect();
    let rows: Vec<String> = expected["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|field| field["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(declared, rows);
}

/// The fence subset stays a subset: every declaration line is an `attribute`
/// or `ref item` with a type and a multiplicity, brace content is opaque, and
/// the classifier carries no expression parser or evaluator.
///
/// Trace: FR-071-CON-1
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_652_the_fence_subset_is_declarations_only_and_nothing_evaluates_them() {
    let fence = mapping_fixture("config-version.fence.md");
    let body = fence
        .split("```sysml")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .expect("the fixture carries a sysml fence");
    let lines: Vec<&str> = body.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "the fence carries {} lines; the subset check would be vacuous",
        lines.len()
    );
    for line in lines {
        assert!(
            line.starts_with("attribute ") || line.starts_with("ref item "),
            "{line}"
        );
        let declaration = line.split_once(" {").map_or(line, |(head, _)| head);
        assert!(declaration.contains(" : "), "{line}");
        assert!(
            declaration.ends_with(']'),
            "every declaration carries a multiplicity: {line}"
        );
    }

    let source: &str = SWEEP_SOURCE;
    assert!(
        source.len() > 10_000,
        "the classifier is {} bytes; this guard is reading the wrong file",
        source.len()
    );
    assert!(source.contains("pub fn classify_artifact"));
    for forbidden in ["parse_expression", "evaluate(", "typecheck"] {
        assert!(
            !source.contains(forbidden),
            "the classifier names {forbidden}"
        );
    }
}

/// No source in this crate typechecks or evaluates a clause: quoin records
/// clause text and its locus, and the checking is somebody else's job.
///
/// A census over every module `lib.rs` declares, not a hand-listed subset: the
/// list below is asserted to be that declaration set, so a module added to the
/// crate and not to this guard fails here rather than escaping it.
///
/// Trace: FR-072-CON-1
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_653_no_source_in_the_crate_typechecks_or_evaluates_a_clause() {
    const LIB: &str = include_str!("../src/lib.rs");
    const SOURCES: [(&str, &str); 10] = [
        ("contract", include_str!("../src/contract.rs")),
        ("data_schema", include_str!("../src/data_schema.rs")),
        ("diagnostic", include_str!("../src/diagnostic.rs")),
        ("embedded", include_str!("../src/embedded.rs")),
        ("error", include_str!("../src/error.rs")),
        ("ids", include_str!("../src/ids.rs")),
        ("manifest", include_str!("../src/manifest.rs")),
        (
            "package_manifest",
            include_str!("../src/package_manifest.rs"),
        ),
        ("schema", include_str!("../src/schema.rs")),
        ("sweep", include_str!("../src/sweep.rs")),
    ];

    let declared: Vec<&str> = LIB
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub mod "))
        .filter_map(|rest| rest.strip_suffix(';'))
        .collect();
    assert!(!declared.is_empty(), "lib.rs declares no modules");
    let scanned: Vec<&str> = SOURCES.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        declared, scanned,
        "every module the crate declares must be scanned"
    );

    for (name, source) in SOURCES {
        for forbidden in ["typecheck", "type_check", "eval_clause", "ocl_parse"] {
            assert!(!source.contains(forbidden), "{name}.rs names {forbidden}");
        }
    }
}

/// Every legacy-form expectation recorded beside the fixtures is what quoin's
/// classifier answers — form, line, and the advisory diagnostic in full.
///
/// Trace: FR-074-AC-1, FR-074-AC-2, FR-074-CON-1
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_654_the_classifier_answers_every_recorded_legacy_expectation() {
    let expected = mapping_json("legacy.expected.json");
    let cases = expected["cases"].as_array().expect("recorded cases");
    // A floor over the population, and over its variety: a corpus of one form
    // would satisfy "every case matches" while testing one branch.
    assert!(cases.len() >= 5, "only {} cases recorded", cases.len());
    let mut forms: Vec<&str> = cases
        .iter()
        .map(|case| case["form"].as_str().unwrap())
        .collect();
    forms.sort_unstable();
    forms.dedup();
    assert!(forms.len() >= 3, "the cases cover only {forms:?}");

    for case in cases {
        let file = case["file"].as_str().unwrap();
        let path = mapping_dir().join(file);
        let markdown = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{file}: {e}"));
        let finding = classify_artifact(file, &markdown);
        assert_eq!(finding.form.as_str(), case["form"], "{file}");
        assert_eq!(json!(finding.line), case["line"], "{file}");
        let reported = serde_json::to_value(&finding.diagnostic).unwrap();
        match case.get("diagnostic") {
            Some(diagnostic) => assert_eq!(&reported, diagnostic, "{file}"),
            None => assert_eq!(reported, Value::Null, "{file} is an authored form"),
        }
    }

    // FR-006 is read from the `agent-ix/config-service` git submodule
    // (`tests/fixtures/semantic-module/corpus/config-service`), not a copy: a
    // legacy classification over a document the test wrote would prove
    // nothing about the corpus this contract is about, and the submodule's
    // pinned commit — not a `PROVENANCE.json` beside a copy — is the record of
    // where it came from (`git submodule status` at that path).
    let corpus_dir = mapping_dir()
        .join("..")
        .join("corpus")
        .join("config-service")
        .join("spec")
        .join("functional");
    let pinned = fs::read_to_string(corpus_dir.join("FR-006-config-version-entity.md")).unwrap();
    assert!(
        pinned.contains("| Column | Type | Constraints |"),
        "{pinned:.0}"
    );
}

/// Build a module declaring `legacy_forms: error`, with `report` written to
/// `sweep_path`, and answer the diagnostic codes its manifest reads back.
fn with_report(
    scratch: &Scratch,
    validators: &quoin_semantic::manifest::SemanticValidators,
    name: &str,
    sweep_path: Option<&str>,
    report: Option<&Value>,
) -> Vec<String> {
    let declared = sweep_path.map(str::to_owned);
    let root = scratch.module_copy(name, |manifest, module_root| {
        manifest["semantic"]["legacy_forms"] = json!("error");
        if let Some(path) = &declared {
            manifest["semantic"]["sweep_report"] = json!(path);
        }
        if let Some(report) = report {
            let file = module_root.join("semantic").join("sweep.json");
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, serde_json::to_string(report).unwrap()).unwrap();
        }
    });
    codes(&root, validators)
        .into_iter()
        .map(|code| code.split('@').next().unwrap_or_default().to_owned())
        .collect()
}

/// A real sweep report over the mapping corpus.
fn report(root: &Path) -> Value {
    let swept = sweep_corpus(
        &[CorpusRoot {
            root: root.to_path_buf(),
            repository: "corpus".to_owned(),
            revision: "worktree".to_owned(),
        }],
        &SweepIdentity {
            package: "agent-ix/spec-objects-fixture".into(),
            version: "0.1.0".into(),
        },
        "2026-09-12T00:00:00.000Z",
    )
    .expect("the corpus is readable");
    serde_json::to_value(swept).unwrap()
}

/// `legacy_forms: error` is admissible only with a shipped sweep report for
/// the same package and version, and only one the report schema accepts.
///
/// Six cases, and the first is the control: without it a guard that always
/// refused would satisfy the other five.
///
/// Trace: FR-074-AC-3
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_655_legacy_forms_error_needs_a_matching_schema_valid_sweep_report() {
    let validators = validators();
    let scratch = Scratch::new();
    let corpus_root = corpus(&scratch);
    let good = report(&corpus_root);
    let required = "error:semantic.sweep-report-required";

    let clean = with_report(
        &scratch,
        &validators,
        "guarded-ok",
        Some("semantic/sweep.json"),
        Some(&good),
    );
    assert!(!clean.iter().any(|c| *c == required), "{clean:?}");

    let cases: [Case; 5] = [
        ("no-declaration", None, Box::new(|_: &mut Value| ())),
        (
            "wrong-path",
            Some("semantic/missing.json"),
            Box::new(|_: &mut Value| ()),
        ),
        (
            "wrong-package",
            Some("semantic/sweep.json"),
            Box::new(|report: &mut Value| report["package"] = json!("agent-ix/other")),
        ),
        (
            "wrong-version",
            Some("semantic/sweep.json"),
            Box::new(|report: &mut Value| report["version"] = json!("9.9.9")),
        ),
        (
            "wrong-shape",
            Some("semantic/sweep.json"),
            Box::new(|report: &mut Value| {
                report["counts"].as_object_mut().unwrap().remove("forms");
            }),
        ),
    ];
    for (name, sweep_path, mutate) in cases {
        let mut candidate = good.clone();
        mutate(&mut candidate);
        let reported = with_report(&scratch, &validators, name, sweep_path, Some(&candidate));
        assert!(
            reported.iter().any(|c| *c == required),
            "{name}: {reported:?}"
        );
    }
}
