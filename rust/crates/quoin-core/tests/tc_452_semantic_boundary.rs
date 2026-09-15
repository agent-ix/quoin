// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `semantic` operations, reached by their wire names (quoin#452).
//!
//! `tc_447_600_every_operation_is_invoked_by_name_by_some_test` requires every
//! entry in `OPERATIONS` to be handed to the real binary, spelled, by some
//! integration test — because a swapped match arm leaves the drift guard, the
//! unit tests and native fixture replays all green and breaks only for a user. This
//! file is that route coverage for `semantic.read_blocks`,
//! `semantic.sweep_corpus` and `semantic.migration_example`, and it is also the
//! one place the whole crossing is exercised end to end: the real vendored
//! contract tree, the real `QUOIN_SEMANTIC_ROOT` publication, the real
//! subprocess, canonical JSON on stdout.
//!
//! Each test asserts something only its own handler could have written. A
//! payload shape shared with a sibling operation would satisfy the census while
//! leaving the swap undetected.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

/// Invoke the binary with `op` on argv and `request` on stdin.
///
/// `QUOIN_SEMANTIC_ROOT` is published exactly as `src/core/semantic.ts`
/// publishes it, because the vendored contract tree ships inside the npm
/// package and only the caller knows where its own package was installed.
fn run(op: &str, request: &Value, contract: bool) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin-core"));
    command.arg(op);
    // Removed rather than left alone when `contract` is false: the developer's
    // own environment may carry one, and a test of "no contract was supplied"
    // that silently found one would assert nothing.
    command.env_remove("QUOIN_SEMANTIC_ROOT");
    if contract {
        command.env("QUOIN_SEMANTIC_ROOT", semantic_root());
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    Run {
        stdout: String::from_utf8(out.stdout).unwrap(),
        stderr: String::from_utf8(out.stderr).unwrap(),
        status: out.status.code().unwrap(),
    }
}

/// The payload of a run that must have succeeded outright.
fn ok(result: &Run) -> Value {
    assert_eq!(result.status, 0, "{}", result.stderr);
    assert_eq!(result.stderr, "");
    serde_json::from_str(&result.stdout).unwrap()
}

/// The repository root: `rust/crates/quoin-core` up three.
fn repo_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..3 {
        root.pop();
    }
    root
}

/// The vendored semantic contract the npm package ships.
///
/// It stays in `src/semantic/` after the TypeScript beside it is deleted: it is
/// DATA, not code — 35 JSON schemas handed across as `QUOIN_SEMANTIC_ROOT` —
/// and `quoin-semantic`'s golden parity suite reads the same live tree.
fn semantic_root() -> PathBuf {
    repo_root().join("src").join("semantic")
}

/// The `module-ok` fixture, the same one `tc_446` and `tc_449` install.
fn fixture() -> PathBuf {
    repo_root()
        .join("tests")
        .join("fixtures")
        .join("semantic-module")
        .join("module-ok")
}

/// `semantic.read_blocks` reads a module's block against the vendored contract
/// and answers per root, in the order asked.
///
/// Two roots and not one: the payload is a LIST keyed by the root it answers
/// for, and a handler that answered only the first — or answered out of order —
/// would satisfy a single-root assertion.
///
/// Trace: FR-070-AC-1, FR-096
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_601_read_blocks_reads_a_module_block_through_the_boundary() {
    let module = fixture();
    let request = json!({ "roots": [module.to_string_lossy(), module.to_string_lossy()] });
    let payload = ok(&run("semantic.read_blocks", &request, true));

    let modules = payload["modules"].as_array().unwrap();
    assert_eq!(modules.len(), 2, "one answer per root asked");
    assert_eq!(modules[0]["root"], module.to_string_lossy().as_ref());
    // The fixture's own declaration, which only this handler could have read:
    // `package`, `exports` and `targets` come off `module-ok/manifest.yaml`.
    assert_eq!(
        modules[0]["block"]["package"],
        "agent-ix/spec-objects-fixture"
    );
    assert_eq!(modules[0]["block"]["exports"], json!(["entity"]));
    assert_eq!(
        modules[0]["block"]["targets"],
        json!(["json-schema", "markdown"])
    );
    // Defaults the schema supplies rather than the manifest: a handler that
    // forwarded the raw YAML would not have them.
    assert_eq!(modules[0]["block"]["compatibility_posture"], "additive");
    assert_eq!(modules[0]["block"]["legacy_forms"], "warning");
    // The fixture's own defect, reported rather than hidden: `module-ok`
    // declares an inline `data_schema` on its enumeration. A handler that
    // dropped the validator's advice would answer with an empty list here.
    let diagnostics = modules[0]["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1, "payload: {payload}");
    assert_eq!(diagnostics[0]["code"], "semantic.inline-data-schema");
    assert_eq!(diagnostics[0]["severity"], "warning");
    assert_eq!(
        diagnostics[0]["path"],
        "object_types[enumeration].data_schema"
    );
    assert_eq!(modules[1]["block"], modules[0]["block"]);
}

/// A module root with no `manifest.yaml` is refused, and the refusal names the
/// `quoin-semantic` code that declined it.
///
/// Trace: FR-070-AC-1, FR-096
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_602_a_module_root_with_no_manifest_is_refused_not_reported_empty() {
    let temp = tempfile::tempdir().unwrap();
    let request = json!({ "roots": [temp.path().to_string_lossy()] });
    let result = run("semantic.read_blocks", &request, true);
    assert_eq!(result.status, 2, "{}", result.stderr);
    assert_eq!(result.stdout, "", "a refusal carries no payload");
    let diagnostics: Value = serde_json::from_str(&result.stderr).unwrap();
    assert_eq!(diagnostics[0]["context"]["semantic_code"], "QSEM-001");
    assert_eq!(diagnostics[0]["context"]["op"], "semantic.read_blocks");
}

/// With no vendored contract published, nothing is judged and the boundary
/// says so rather than reporting a clean read of an unjudged module.
///
/// Trace: FR-070-AC-1, FR-096
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_603_without_a_contract_root_the_answer_is_a_refusal() {
    let request = json!({ "roots": [fixture().to_string_lossy()] });
    let result = run("semantic.read_blocks", &request, false);
    assert_eq!(result.status, 2, "{}", result.stderr);
    let diagnostics: Value = serde_json::from_str(&result.stderr).unwrap();
    assert_eq!(diagnostics[0]["context"]["semantic_code"], "QSEM-009");
}

/// `semantic.sweep_corpus` walks a real tree and classifies what it finds.
///
/// Trace: FR-074-AC-3, FR-074-AC-4, FR-096
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_604_sweep_corpus_classifies_a_real_tree_through_the_boundary() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("typed.md"),
        "# Entity\n\n## Properties\n\n\
         | Field | Type | Multiplicity | Constraints |\n\
         | --- | --- | --- | --- |\n\
         | id | UUID | 1 | identity |\n",
    )
    .unwrap();
    fs::write(
        root.join("legacy.md"),
        "# Thing\n\n## Properties\n\n- id: UUID, required\n- name: string\n",
    )
    .unwrap();

    let request = json!({
        "roots": [{
            "root": root.to_string_lossy(),
            "repository": "fixture-corpus",
            "revision": "worktree",
        }],
        "package": "agent-ix/spec-objects-business",
        "version": "0.3.0",
        "generated_at": "2026-09-12T00:00:00.000Z",
    });
    // No contract: a sweep reads Markdown as text and validates nothing, so it
    // must not require a vendored tree it does not consult.
    let report = ok(&run("semantic.sweep_corpus", &request, false))["report"].clone();

    assert_eq!(report["package"], "agent-ix/spec-objects-business");
    assert_eq!(report["version"], "0.3.0");
    assert_eq!(report["generatedAt"], "2026-09-12T00:00:00.000Z");
    assert_eq!(
        report["corpus"],
        json!([{ "repository": "fixture-corpus", "revision": "worktree" }])
    );
    assert_eq!(report["counts"]["artifacts"], 2);
    assert_eq!(report["counts"]["forms"]["typed-table"], 1);
    assert_eq!(report["counts"]["forms"]["bullet-list"], 1);
    assert_eq!(report["counts"]["legacy"]["bullet-list"], 1);

    let findings = report["findings"].as_array().unwrap();
    let legacy = findings
        .iter()
        .find(|f| f["form"] == "bullet-list")
        .expect("the bullet-list artifact is reported");
    assert_eq!(legacy["path"], "fixture-corpus:legacy.md");
    assert_eq!(
        legacy["diagnostic"]["code"],
        "semantic.legacy-properties-form"
    );
    assert_eq!(legacy["diagnostic"]["severity"], "warning");
}

/// `semantic.migration_example` serves the FR-074 guidance, and it names the
/// same authored form a legacy finding directs an author to.
///
/// Two halves and not one: `quoin write` prints this text once per authoring
/// pack while every sweep finding names the target form, and before the cutover
/// the guidance lived in TypeScript and the classifier in Rust. Asserting the
/// text alone would pass with a copy that had drifted from what the sweep
/// actually asks for.
///
/// Trace: FR-074-AC-1, FR-096
/// Provenance: agent-ix/quoin#452
#[test]
fn tc_452_605_migration_example_is_the_guidance_a_legacy_finding_directs_to() {
    let payload = ok(&run("semantic.migration_example", &json!({}), false));
    let served = payload["example"].as_str().unwrap();
    assert!(
        served.starts_with("Properties migration (FR-074)"),
        "{served}"
    );
    assert!(served.contains("| Field | Type | Multiplicity | Constraints |"));
    assert!(served.contains("legacy_forms: error"), "{served}");

    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("legacy.md"),
        "# Thing\n\n## Properties\n\n- id: UUID, required\n- name: string\n",
    )
    .unwrap();
    let request = json!({
        "roots": [{
            "root": temp.path().to_string_lossy(),
            "repository": "fixture-corpus",
            "revision": "worktree",
        }],
        "package": "agent-ix/spec-objects-business",
        "version": "0.3.0",
        "generated_at": "2026-09-12T00:00:00.000Z",
    });
    let report = ok(&run("semantic.sweep_corpus", &request, false))["report"].clone();
    let finding = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["form"] == "bullet-list")
        .expect("the bullet-list artifact is reported");
    // The form the finding sends the author to, and the form the served
    // guidance calls the authored one, are the same form.
    assert_eq!(finding["diagnostic"]["migration"], "typed-table");
    assert!(
        served.contains("the typed table is the authored form"),
        "{served}"
    );
}
