// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `semantic` operations, reached through the production runtime (quoin#452).
//!
//! `tc_447_600_every_operation_is_invoked_by_name_by_some_test` requires every
//! entry in `OPERATIONS` to be handed to the production runtime, spelled, by some
//! integration test — because a swapped match arm leaves the drift guard, the
//! unit tests and native fixture replays all green and breaks only for a user. This
//! file is that route coverage for `semantic.read_blocks`,
//! `semantic.sweep_corpus` and `semantic.migration_example`, and it is also the
//! one place the whole crossing is exercised end to end: the real vendored
//! contract tree, the real `QUOIN_SEMANTIC_ROOT` publication, the real
//! runtime capability grant, canonical JSON response payloads.
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
use std::path::PathBuf;

use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_core::runtime::{RuntimeSettings, dispatch};
use serde_json::{Value, json};

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

/// Dispatch one request under `settings` and reduce the response to a [`Run`].
///
/// The shared tail of [`run`] and [`run_isolated`]: everything above this line
/// decides which capabilities the runtime is granted, and this is the one
/// place a [`Response`] becomes stdout, stderr and a status.
fn invoke(op: &str, request: &Value, settings: &RuntimeSettings) -> Run {
    let response = dispatch(op, request, settings).unwrap_or_else(|error| Response {
        payload: Value::Null,
        diagnostics: vec![Diagnostic::from(&error)],
        outcome: error.outcome(),
    });
    let stdout = if response.outcome.carries_payload() {
        canonical_json(&response.payload).unwrap()
    } else {
        String::new()
    };
    let stderr = if response.diagnostics.is_empty() {
        String::new()
    } else {
        canonical_json(&response.diagnostics).unwrap()
    };
    Run {
        stdout,
        stderr,
        status: i32::from(response.outcome.code()),
    }
}

/// Invoke the runtime with the explicit vendored-contract capability.
fn run(op: &str, request: &Value, contract: bool) -> Run {
    let settings = RuntimeSettings {
        ix_home: None,
        semantic_root: contract.then(semantic_root),
    };
    invoke(op, request, &settings)
}

/// Invoke the runtime with neither a vendored contract root nor the ambient
/// `IX_HOME`: `ix_home` is an isolated directory this test owns, so the
/// embedded-contract fallback this exercises materialises under a path the
/// test controls rather than whatever the developer's or CI's real `IX_HOME`
/// happens to hold.
///
/// `QUOIN_SEMANTIC_ROOT` cannot be cleared the same way: `std::env::remove_var`
/// is `unsafe fn` on this toolchain, and the workspace lint policy forbids
/// `unsafe_code` outright (see `rust-style`'s lints section) — the same reason
/// `quoin-config`'s `Environment` trait reads ambient state through an
/// injectable seam instead of mutating the process environment in tests. This
/// operation has no such seam, so the precondition is asserted instead of
/// silently overridden: a caller who has actually exported
/// `QUOIN_SEMANTIC_ROOT` gets a named failure, not a test that passed by
/// accident against a root it never meant to exercise.
fn run_isolated(op: &str, request: &Value, ix_home: &std::path::Path) -> Run {
    assert!(
        std::env::var_os("QUOIN_SEMANTIC_ROOT").is_none(),
        "this test exercises the embedded-contract fallback and needs no \
         QUOIN_SEMANTIC_ROOT set in the ambient environment; unset it and rerun"
    );
    let settings = RuntimeSettings {
        ix_home: Some(ix_home.to_path_buf()),
        semantic_root: None,
    };
    invoke(op, request, &settings)
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

/// The real semantic contract `QUOIN_SEMANTIC_ROOT` publishes in production:
/// DATA, not code, handed across as a directory path. Materialized once per
/// process from the embedded bytes `quoin-semantic`'s `build.rs` compiles in
/// (PLAT-887 de-vendoring), not read from a source-tree `src/semantic`,
/// which no longer holds a schema tree.
fn semantic_root() -> PathBuf {
    static ROOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let root = std::env::temp_dir().join(format!(
            "quoin-core-tc-452-boundary-semantic-{}",
            std::process::id()
        ));
        quoin_semantic::materialize_embedded_contract(&root)
            .expect("the embedded semantic contract materializes");
        root
    })
    .clone()
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

/// With no vendored contract root supplied, the boundary falls back to its
/// embedded contract (quoin-semantic's `embedded` module, designed in #527 and
/// wired into this boundary in #539) rather than refusing: a release
/// executable owns its own contract and must not depend on
/// `QUOIN_SEMANTIC_ROOT` being set. The read answers the same as
/// `tc_452_601`'s explicit-root read, and the fallback's own side effect — the
/// contract materialised under this run's `IX_HOME` cache — is asserted
/// directly rather than only inferred from the read succeeding.
///
/// Trace: FR-070-AC-1, FR-096
/// Provenance: agent-ix/quoin#452, agent-ix/quoin#527, agent-ix/quoin#539
#[test]
fn tc_452_603_without_a_contract_root_the_embedded_contract_answers() {
    let home = tempfile::tempdir().unwrap();
    let request = json!({ "roots": [fixture().to_string_lossy()] });
    let payload = ok(&run_isolated("semantic.read_blocks", &request, home.path()));
    let modules = payload["modules"].as_array().unwrap();
    assert_eq!(
        modules[0]["block"]["package"],
        "agent-ix/spec-objects-fixture"
    );

    let materialized = home.path().join("cache/quoin-semantic/v1");
    assert!(
        materialized
            .join("schemas/module-manifest.schema.json")
            .is_file(),
        "the embedded contract is written under the isolated IX_HOME's cache at {}",
        materialized.display()
    );
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
