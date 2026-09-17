// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Every operation in [`quoin_core::dispatch::OPERATIONS`] is reached by name
//! through the shared native runtime, by at least one test — and a census that
//! fails when one is not.
//!
//! # The hole this closes
//!
//! `dispatch`'s drift guard (#443, #444) checks that `OPERATIONS` and the match
//! agree on the SET of names. It cannot see which handler a name reaches. Swap
//! two arms —
//!
//! ```text
//! "completeness.read_frontmatter" => crate::ops::completeness::schema_refs(request),
//! ```
//!
//! — and the strings are unchanged, so the drift guard is satisfied, the
//! library's own unit tests still pass (they call the handler functions
//! directly and never the wire name), native fixture replay passes, and
//! `make rust-gate` is green. The first thing that breaks is
//! `quoin completeness`, for a user.
//!
//! A unit test therefore cannot cover an operation for this purpose. Coverage
//! means a test that hands the WIRE SPELLING to the process and asserts what
//! came back, which is why the census counts only integration tests in this
//! directory and only where the op is passed to the subprocess runner.
//!
//! `assurance.render_authored_argument` and `completeness.read_frontmatter` had
//! no test of the handler of any kind when this file was written.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::path::{Path, PathBuf};

use quoin_core::protocol::canonical_json;
use quoin_core::runtime::{RuntimeSettings, dispatch};
use serde_json::{Value, json};

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

fn run(op: &str, stdin: &str) -> Run {
    let request: Value = serde_json::from_str(stdin).expect("census request is JSON");
    let semantic_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../src/semantic");
    let response = dispatch(
        op,
        &request,
        &RuntimeSettings {
            ix_home: None,
            semantic_root: Some(semantic_root),
        },
    );
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            return Run {
                stdout: String::new(),
                stderr: error.to_string(),
                status: i32::from(error.outcome().code()),
            };
        }
    };
    let stdout = if response.outcome.carries_payload() {
        canonical_json(&response.payload).expect("payload canonicalizes")
    } else {
        String::new()
    };
    let stderr = if response.diagnostics.is_empty() {
        String::new()
    } else {
        canonical_json(&response.diagnostics).expect("diagnostics canonicalize")
    };
    Run {
        stdout,
        stderr,
        status: i32::from(response.outcome.code()),
    }
}

/// The payload of a run that must have succeeded.
///
/// Takes the finished [`Run`] rather than the operation name, so every test
/// below spells its operation at the `run` call the census scans for.
fn ok(result: &Run) -> Value {
    assert_eq!(result.status, 0, "{}", result.stderr);
    assert_eq!(result.stderr, "");
    serde_json::from_str(&result.stdout).unwrap()
}

/// Every integration test in this directory, as `(file name, source)`.
///
/// This file included. It cannot satisfy the census by accident: the census
/// builds the string it looks for with `format!` from `OPERATIONS`, so the only
/// literal `run("<op>"` in this file is a test below that really does invoke
/// that operation.
fn integration_tests() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("the integration test directory is readable")
        .map(|entry| entry.expect("a directory entry is readable").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&path).expect("an integration test is readable");
            // Whitespace removed, because rustfmt decides where a call breaks
            // across lines and an op name carries no whitespace of its own.
            // Without this the census answers differently after a `cargo fmt`,
            // which would be a guard that depends on formatting.
            (name, text.chars().filter(|c| !c.is_whitespace()).collect())
        })
        .collect()
}

/// Route probes for operations whose former protocol-process tests were moved
/// to the shipped CLI. Their named in-process invocation keeps the dispatch
/// census able to catch a swapped match arm without retaining that executable.
///
/// Trace: FR-101, FR-102
#[test]
fn tc_521_migrated_command_routes_remain_named_in_the_runtime_census() {
    for result in [
        run("core.ping", &json!({}).to_string()),
        run("catalog.load", &json!({}).to_string()),
        run("catalog.methods", &json!({}).to_string()),
        run(
            "modules.remove",
            &json!({ "name": "not-installed" }).to_string(),
        ),
        run("validators.run", &json!({ "files": {} }).to_string()),
        run(
            "assurance.requirement_of",
            &json!({ "obligation_id": "O-1" }).to_string(),
        ),
    ] {
        assert_ne!(
            result.status, 4,
            "a migrated route reached an internal failure instead of its handler: {}",
            result.stderr
        );
    }
}

/// Every operation this build routes is invoked BY NAME by some integration
/// test.
///
/// The list comes from `OPERATIONS` itself rather than a hand-copied one: a
/// census over a transcribed list is a census of the transcription, which is
/// the #443 failure this whole area already has scar tissue from.
///
/// Trace: FR-096, NFR-024
/// Provenance: agent-ix/quoin#445, agent-ix/quoin#447
#[test]
fn tc_447_600_every_operation_is_invoked_by_name_by_some_test() {
    let sources = integration_tests();
    // Anti-vacuity. A scanner that found nothing must fail as a broken scanner
    // rather than report a clean census: with no sources and no operations,
    // every assertion below is over an empty population and passes.
    assert!(
        sources.len() >= 4,
        "the scan found {} integration test file(s); it is broken, not the crate",
        sources.len()
    );
    assert!(
        quoin_core::dispatch::OPERATIONS.len() >= 13,
        "OPERATIONS came back with {} entries; the census is reading the wrong thing",
        quoin_core::dispatch::OPERATIONS.len()
    );

    let uncovered: Vec<&str> = quoin_core::dispatch::OPERATIONS
        .iter()
        .copied()
        .filter(|op| {
            // The op as the subprocess runner is actually called in this
            // directory: `run("<op>", …)` or `run(&["<op>"], …)`. Naming it in
            // a table of constants is not invoking it, and a unit test calling
            // the handler function cannot see a swapped match arm at all.
            let called = format!("run(\"{op}\"");
            let called_argv = format!("run(&[\"{op}\"]");
            !sources
                .iter()
                .any(|(_, text)| text.contains(&called) || text.contains(&called_argv))
        })
        .collect();

    assert_eq!(
        uncovered,
        Vec::<&str>::new(),
        "these operations are dispatched but no test invokes them by name, so a \
         swapped match arm would route them to another handler with every gate \
         green. Add a test that runs the operation and asserts its payload."
    );
}

/// `assurance.parse_argument` returns the validated argument.
///
/// The 128-case golden replay in `tc_447_assurance_boundary.rs` reaches this
/// operation through a `format!("assurance.{}", case.op)`, so no test named it.
/// That is coverage a census cannot see and a reader cannot either.
///
/// Trace: FR-040, FR-096
/// Provenance: agent-ix/quoin#447
#[test]
fn tc_447_601_parse_argument_returns_the_validated_argument() {
    let payload = ok(&run("assurance.parse_argument", &argument().to_string()));
    assert_eq!(payload["id"], "AA-900");
    assert_eq!(payload["top_claim"]["id"], "CLAIM-900");
    assert_eq!(payload["reasoning"][0]["id"], "ARG-900");
}

/// `assurance.build_authored_argument` and `assurance.render_authored_argument`
/// are a pair, and the second had no test of its handler at all.
///
/// Chained rather than tested apart, because the renderer's input IS the
/// builder's output: fed a hand-written view, a renderer that had drifted from
/// the builder would still pass.
///
/// Trace: FR-047, FR-096
/// Provenance: agent-ix/quoin#447
#[test]
fn tc_447_602_an_authored_argument_is_built_and_then_rendered() {
    let request = json!({
        "argument": argument(),
        "decisions": [],
        "asOf": "2026-01-01T00:00:00Z",
    });
    let view = ok(&run(
        "assurance.build_authored_argument",
        &request.to_string(),
    ));
    assert_eq!(view["schemaVersion"], "authored-assurance-view-v1");
    assert_eq!(view["argument"]["id"], "AA-900");
    assert_eq!(view["asOf"], "2026-01-01T00:00:00Z");
    // Open, not supported: no sufficiency decision was supplied, and a view
    // that called an undecided criterion met is the answer this pair exists to
    // not give.
    assert_eq!(view["topClaim"]["status"], "open");
    assert_eq!(view["reasoning"][0]["criteria"][0]["status"], "open");

    let rendered = ok(&run(
        "assurance.render_authored_argument",
        &view.to_string(),
    ));
    let markdown = rendered["rendered"].as_str().unwrap();
    assert!(
        markdown.starts_with("# AA-900: Synthetic widget release decision"),
        "{markdown}"
    );
    assert!(markdown.contains("**◇ OPEN**"), "{markdown}");
    assert!(markdown.contains("### ◇ ARG-900"), "{markdown}");
}

/// `assurance.build_discharge` and `assurance.render_discharge`, the same way.
///
/// The clause set carries one MANDATORY binding clause and no facts, so the
/// partition has to put it in `open`: an empty clause set would exercise the
/// request shape and none of the accounting.
///
/// Trace: FR-046, FR-096
/// Provenance: agent-ix/quoin#447
#[test]
fn tc_447_603_a_discharge_is_built_and_then_rendered() {
    let request = json!({
            "binding": {
                "schemaVersion": "clause-binding-v1",
                "clauseSet": { "authority": "quire", "id": "spec", "version": "1" },
                "clauseSetDigest":
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                "context": {},
                "clauses": [{
                    "clauseId": "C-1",
                    "force": "mandatory",
                    "outcome": "binding",
                    "reasons": [],
                    "expectedOutputs": [],
                }],
            },
            "facts": [],
            "asOf": "2026-01-01T00:00:00Z",
    });
    let report = ok(&run("assurance.build_discharge", &request.to_string()));
    assert_eq!(report["schemaVersion"], "clause-discharge-v1");
    assert_eq!(report["binding"]["direct"], json!([]));
    assert_eq!(report["binding"]["open"][0]["clauseId"], "C-1");
    assert_eq!(report["binding"]["open"][0]["reason"], "no discharge fact");

    let rendered = ok(&run("assurance.render_discharge", &report.to_string()));
    let markdown = rendered["rendered"].as_str().unwrap();
    assert!(
        markdown.starts_with("# Clause discharge: quire/spec@1"),
        "{markdown}"
    );
    assert!(
        markdown.contains("- `C-1` — mandatory; no declared outputs; no discharge fact"),
        "{markdown}"
    );
}

/// `completeness.schema_refs` names the frontmatter schemas a manifest reaches
/// for, so the shell knows what to read.
///
/// Trace: FR-037, FR-096
/// Provenance: agent-ix/quoin#445
#[test]
fn tc_447_604_schema_refs_names_what_the_shell_must_read() {
    let request = json!({ "manifest": MANIFEST });
    let payload = ok(&run("completeness.schema_refs", &request.to_string()));
    assert_eq!(payload["refs"], json!(["schemas/nfr.json"]));
}

/// `completeness.read_frontmatter` — the operation that had no test of its
/// handler anywhere in the tree.
///
/// Three documents, three different answers: one parses, one carries no
/// frontmatter and contributes nothing, and one the caller could not open and
/// which must survive the crossing. Asserting only the first would pass with a
/// handler that dropped the other two.
///
/// Trace: FR-037, FR-040, FR-096
/// Provenance: agent-ix/quoin#445, agent-ix/quoin#447
#[test]
fn tc_447_605_read_frontmatter_splits_documents_and_keeps_the_unreadable() {
    let request = json!({
        "documents": [
            { "path": "a.md", "raw": "---\ntype: NFR\nquality_attribute: security\n---\nthe body\n" },
            { "path": "readme.md", "raw": "no frontmatter here\n" },
        ],
        "unreadable": [{ "path": "locked.md", "reason": "EACCES" }],
    });
    let payload = ok(&run("completeness.read_frontmatter", &request.to_string()));
    assert_eq!(
        payload["documents"],
        json!([{
            "path": "a.md",
            "frontmatter": { "quality_attribute": "security", "type": "NFR" },
            "body": "the body\n",
        }]),
        "a document with no frontmatter declares nothing and must not appear"
    );
    assert_eq!(
        payload["unreadable"],
        json!([{ "path": "locked.md", "reason": "EACCES" }])
    );
}

/// `completeness.assess_bundle` reaches a verdict over content alone.
///
/// Trace: FR-037, FR-096
/// Provenance: agent-ix/quoin#445
#[test]
fn tc_447_606_assess_bundle_returns_a_verdict() {
    let request = json!({
            "bundle_root": "spec",
            "strict": false,
            "documents": [
                { "path": "a.md", "raw": "---\ntype: NFR\nquality_attribute: security\n---\nbody\n" }
            ],
            "modules": [{
                "label": "iso",
                "manifest": MANIFEST,
                "schemas": {
                    "schemas/nfr.json": {
                        "text": "{\"properties\":{\"quality_attribute\":{\"enum\":[\"security\",\"reliability\"]}}}"
                    }
                }
            }]
    });
    let payload = ok(&run("completeness.assess_bundle", &request.to_string()));
    assert_eq!(payload["bundleRoot"], "spec");
    assert_eq!(payload["verdict"], "CONDITIONAL");
    assert_eq!(payload["vocabularies"], json!(["quality-characteristics"]));
    assert_eq!(payload["findings"][0]["value"], "reliability");
}

/// The one complete authored argument the assurance operations above are built
/// from, read from the golden corpus rather than transcribed.
///
/// `tests/tc_447_assurance_boundary.rs` reads the same file for the same
/// reason: a second hand-written copy of twelve required keys would drift from
/// the seventeen predicates that validate it, and would do so silently.
fn argument() -> Value {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../quoin-assurance/tests/golden/cases.json"
    ))
    .to_path_buf();
    let corpus: Value = serde_json::from_str(
        &std::fs::read_to_string(path).expect("the golden corpus is readable"),
    )
    .unwrap();
    corpus["argument_base"].clone()
}

/// A module manifest declaring one vocabulary over one artifact type.
const MANIFEST: &str = "name: iso\n\
    artifact_types:\n\
    \x20 - name: NFR\n\
    \x20   frontmatter_schema_ref: schemas/nfr.json\n\
    traceability:\n\
    \x20 vocabulary_coverage:\n\
    \x20   - name: quality-characteristics\n\
    \x20     from: NFR\n\
    \x20     field: quality_attribute\n\
    \x20     check: vocabulary-coverage\n";
