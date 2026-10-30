// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `change_assurance` operations, reached through the production runtime
//! (quoin#457).
//!
//! `tc_447_600_every_operation_is_invoked_by_name_by_some_test` requires every
//! entry in `OPERATIONS` to be handed to the production runtime, spelled, by an
//! integration test - because a swapped match arm leaves the source-scanning
//! drift guard and the unit tests green and breaks only for a user. Route
//! coverage is not handler coverage (quoin#443, quoin#447), and this file is the
//! route coverage for all seven change-assurance operations. Each test asserts
//! a payload member only its own handler writes.
//!
//! The 0/1/2 exit grammar lives in `quoin-cli`'s
//! `tc_1322_change_assurance_exit_grammar.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_core::runtime::{RuntimeSettings, dispatch};
use serde_json::{Value, json};

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

/// Invoke the runtime with the operation and request.
fn run(op: &str, request: &Value) -> Run {
    let response =
        dispatch(op, request, &RuntimeSettings::default()).unwrap_or_else(|error| Response {
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

/// The payload of a run that must have succeeded outright.
fn ok(result: &Run) -> Value {
    assert_eq!(result.status, 0, "{}", result.stderr);
    assert_eq!(result.stderr, "");
    serde_json::from_str(&result.stdout).unwrap()
}

/// The diagnostics of a run that must have failed with `status`.
fn failed(result: &Run, status: i32) -> Value {
    assert_eq!(result.status, status, "stdout: {}", result.stdout);
    assert_eq!(result.stdout, "");
    let diagnostics: Value = serde_json::from_str(&result.stderr).unwrap();
    assert!(
        !diagnostics.as_array().unwrap().is_empty(),
        "a failure says why"
    );
    diagnostics
}

/// Lowercase hex of some bytes: the encoding every document crosses in.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

/// Hex of a JSON document's bytes.
fn hex_json(value: &Value) -> String {
    hex(&serde_json::to_vec(value).unwrap())
}

/// The bytes the attestation's retained output holds.
const OUTPUT: &[u8] = b"all tests passed\n";

/// The record body a caller writes.
fn record_body() -> Value {
    json!({
        "schema_version": 1,
        "record_type": "change_assurance",
        "record_id": "change-1",
        "revision": 1,
        "subject": {"repository": "agent-ix/quoin", "base_revision": "abc123", "scope": ["src/evidence"]},
        "source_connections": [{"source_id": "FR-063", "kind": "requirement", "revision": "1"}],
        "impact_snapshot": {"identity": "impact-1", "revision": "1", "completeness": "complete", "truncated": false, "gaps": []},
        "definition": {
            "requirements": [{"id": "FR-063", "statement": "record it", "source_ids": ["FR-063"]}],
            "preservation_constraints": [],
            "proof_obligations": [{
                "proof_id": "proof-1",
                "statement": "tests pass",
                "obligation_ids": ["FR-063-AC-1"],
                "evidence_kind": "Unit",
                "command": {"argv": ["cargo", "test"], "working_directory": "."},
                "tool_identity": "cargo",
                "configuration_ref": "config/test.toml"
            }],
            "unknowns": []
        },
        "review_workflow": {"run_id": "run-1", "decision_event_kind": "change_assurance.review_decided"}
    })
}

/// The attestation body a caller writes, before `retained_output` is derived.
fn attestation_body() -> Value {
    json!({
        "schema_version": 1,
        "record_type": "proof_attestation",
        "attestation_id": "att-1",
        "record_id": "change-1",
        "record_revision": 1,
        "candidate_revision": "candidate-1",
        "proof_id": "proof-1",
        "command": {"argv": ["cargo", "test"], "working_directory": "."},
        "tool": {"identity": "cargo", "version": "1.90.0", "configuration_ref": "config/test.toml"},
        "environment": {"os": "test"},
        "observed_at": "2026-08-31T00:00:00Z",
        "result": "passed",
        "strength": "tested"
    })
}

/// An approving decision history.
fn decisions() -> Value {
    json!({
        "run_id": "run-1",
        "events": [{
            "id": "event-1", "ts": "2026-08-31T00:00:00.000Z", "sequence": 1,
            "actor": {"id": "reviewer-1", "kind": "human"},
            "kind": "change_assurance.review_decided",
            "payload": {"schema_version": 1, "record_id": "change-1", "revision": 1, "decision": "approved"}
        }]
    })
}

/// The audit report that finds the one proof healthy.
fn audits() -> Value {
    json!([{
        "proof_id": "proof-1",
        "report": {"findings": [], "healthy": ["FR-063-AC-1"], "unevaluated": []}
    }])
}

/// Write the record, then prepare and take in the attestation.
fn retain_everything(root: &Path) {
    ok(&run(
        "change_assurance.write_record",
        &json!({ "repo": root.to_str().unwrap(), "record_hex": hex_json(&record_body()) }),
    ));
    let prepared = ok(&run(
        "change_assurance.prepare_attestation",
        &json!({
            "attestation_hex": hex_json(&attestation_body()),
            "output_hex": hex(OUTPUT),
            "media_type": "text/plain",
        }),
    ));
    ok(&run(
        "change_assurance.intake",
        &json!({
            "repo": root.to_str().unwrap(),
            "attestation_hex": hex_json(&prepared["attestation"]),
            "output_hex": hex(OUTPUT),
        }),
    ));
}

/// The receipt request every test varies.
fn receipt_request(root: &Path) -> Value {
    json!({
        "repo": root.to_str().unwrap(),
        "record_id": "change-1",
        "revision": 1,
        "candidate_revision": "candidate-1",
        "selections": [{"proof_id": "proof-1", "attestation_id": "att-1"}],
        "decisions_hex": hex_json(&decisions()),
        "audits_hex": hex_json(&audits()),
    })
}

/// A repository root that exists only for one test.
fn repo() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// The paths under a repository's change-assurance root, sorted.
fn retained(root: &Path) -> Vec<String> {
    fn walk(at: &Path, into: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(at) else {
            return;
        };
        for entry in entries.flatten() {
            into.push(entry.path());
            walk(&entry.path(), into);
        }
    }
    let store = root.join("spec").join("evidence").join("change-assurance");
    let mut found = Vec::new();
    walk(&store, &mut found);
    let mut names: Vec<String> = found
        .iter()
        .map(|path| {
            path.strip_prefix(&store)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// `change_assurance.write_record` retains a record at its `(record_id,
/// revision)` and reports the path it now occupies. A body that supplies a
/// `digest`, or any member the FR-063 schema does not declare, is refused and
/// nothing is retained.
///
/// `path` is the member only this handler writes; a swap onto `intake` would
/// answer `directory` and fail here.
///
/// Trace: FR-068-AC-1
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_600_write_record_retains_a_record_under_its_identity() {
    let root = repo();
    let payload = ok(&run(
        "change_assurance.write_record",
        &json!({ "repo": root.path().to_str().unwrap(), "record_hex": hex_json(&record_body()) }),
    ));
    assert_eq!(payload["retained"], json!(true));
    assert_eq!(
        payload["path"].as_str().unwrap(),
        root.path()
            .join("spec/evidence/change-assurance/records/change-1/1.json")
            .to_str()
            .unwrap()
    );

    let mut with_digest = record_body();
    with_digest["digest"] = json!("a".repeat(64));
    let other = repo();
    failed(
        &run(
            "change_assurance.write_record",
            &json!({ "repo": other.path().to_str().unwrap(), "record_hex": hex_json(&with_digest) }),
        ),
        3,
    );
    assert_eq!(retained(other.path()), Vec::<String>::new());
}

/// `change_assurance.prepare_attestation` derives only the retained output's
/// size; a caller-written `retained_output` is refused.
///
/// Trace: FR-068-AC-2, FR-068-CON-2
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_601_prepare_attestation_derives_only_the_retained_output_size() {
    let prepared = ok(&run(
        "change_assurance.prepare_attestation",
        &json!({
            "attestation_hex": hex_json(&attestation_body()),
            "output_hex": hex(OUTPUT),
            "media_type": "text/plain",
        }),
    ));
    assert_eq!(
        prepared["attestation"]["retained_output"],
        json!({"media_type": "text/plain", "size_bytes": OUTPUT.len()})
    );

    let mut supplied = attestation_body();
    supplied["retained_output"] = json!({"media_type": "text/plain", "size_bytes": 1});
    failed(
        &run(
            "change_assurance.prepare_attestation",
            &json!({
                "attestation_hex": hex_json(&supplied),
                "output_hex": hex(OUTPUT),
                "media_type": "text/plain",
            }),
        ),
        3,
    );
}

/// `change_assurance.intake` retains the exact attestation and output bytes as
/// one pair under the attestation id, is idempotent for byte-identical inputs,
/// and refuses output whose size contradicts what the attestation records -
/// retaining nothing on that path.
///
/// `size_bytes` is the member only this handler writes.
///
/// Trace: FR-068-AC-3
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_602_intake_retains_an_exact_pair_atomically_and_idempotently() {
    let prepared = ok(&run(
        "change_assurance.prepare_attestation",
        &json!({
            "attestation_hex": hex_json(&attestation_body()),
            "output_hex": hex(OUTPUT),
            "media_type": "text/plain",
        }),
    ));
    let root = repo();
    let request = json!({
        "repo": root.path().to_str().unwrap(),
        "attestation_hex": hex_json(&prepared["attestation"]),
        "output_hex": hex(OUTPUT),
    });

    let first = ok(&run("change_assurance.intake", &request));
    assert_eq!(first["retained"], json!(true));
    assert_eq!(first["size_bytes"], json!(OUTPUT.len()));
    let directory = root
        .path()
        .join("spec/evidence/change-assurance/attestations/att-1");
    assert_eq!(
        first["directory"].as_str().unwrap(),
        directory.to_str().unwrap()
    );
    // The retained bytes are the producer's own, byte for byte - which is the
    // whole reason the output crosses the boundary as hex rather than as text.
    assert_eq!(fs::read(directory.join("output.bin")).unwrap(), OUTPUT);

    let again = ok(&run("change_assurance.intake", &request));
    assert_eq!(again["retained"], json!(false));

    let contradicting = repo();
    let result = run(
        "change_assurance.intake",
        &json!({
            "repo": contradicting.path().to_str().unwrap(),
            "attestation_hex": hex_json(&prepared["attestation"]),
            "output_hex": hex(b"other bytes entirely"),
        }),
    );
    let diagnostics = failed(&result, 2);
    assert_eq!(
        diagnostics[0]["context"]["change_assurance_code"],
        "QCA-OUTPUT-INTEGRITY-MISMATCH"
    );
    assert_eq!(retained(contradicting.path()), Vec::<String>::new());
}

/// `change_assurance.receipt` assembles its verification input from the named
/// record and the explicit selections. A retained attestation that no
/// selection names has NO effect: a stray upload cannot discharge a proof.
///
/// Trace: FR-068-AC-4
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_603_receipt_uses_only_what_the_caller_named() {
    let root = repo();
    retain_everything(root.path());
    let named = ok(&run(
        "change_assurance.receipt",
        &receipt_request(root.path()),
    ));
    assert_eq!(named["receipt"]["outcome"], json!("valid"), "{named}");

    let mut none = receipt_request(root.path());
    none["selections"] = json!([]);
    let unselected = ok(&run("change_assurance.receipt", &none));
    assert_eq!(unselected["receipt"]["outcome"], json!("incomplete"));
    assert_eq!(
        unselected["receipt"]["reasons"],
        json!(["attestation_missing"])
    );
}

/// `change_assurance.verify_receipt` re-derives the receipt from the named
/// retained inputs and refuses one that differs, byte for byte.
///
/// Trace: FR-068-AC-5
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_605_verify_receipt_re_derives_and_refuses_an_alteration() {
    let root = repo();
    retain_everything(root.path());
    let stored = ok(&run(
        "change_assurance.receipt",
        &receipt_request(root.path()),
    ))["receipt"]
        .clone();

    let verified = ok(&run(
        "change_assurance.verify_receipt",
        &json!({ "receipt_hex": hex_json(&stored), "inputs": receipt_request(root.path()) }),
    ));
    assert_eq!(verified["receipt"], stored);

    let mut edited = stored;
    edited["outcome"] = json!("invalid");
    let diagnostics = failed(
        &run(
            "change_assurance.verify_receipt",
            &json!({ "receipt_hex": hex_json(&edited), "inputs": receipt_request(root.path()) }),
        ),
        3,
    );
    assert_eq!(
        diagnostics[0]["context"]["change_assurance_code"],
        "QCA-RECEIPT-INVALID"
    );
}

/// `change_assurance.recover` removes the staging directories an interrupted
/// intake left behind, reports how many, and leaves every retained record,
/// attestation and output where it was.
///
/// The count is asserted over a store that actually holds staging, and the
/// survivors over a store that actually holds evidence: a recovery asserted
/// over an empty tree passes whatever the code does.
///
/// Trace: FR-068-AC-9
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_606_recover_removes_only_interrupted_staging() {
    let root = repo();
    retain_everything(root.path());
    let before = retained(root.path());
    assert!(
        before.len() >= 4,
        "the store must hold real evidence for the survivors to mean anything"
    );

    let attestations = root
        .path()
        .join("spec/evidence/change-assurance/attestations");
    for name in [".tmp-attestation-1-1", ".tmp-attestation-2-2"] {
        fs::create_dir_all(attestations.join(name)).unwrap();
        fs::write(attestations.join(name).join("output.bin"), b"half a pair").unwrap();
    }

    let payload = ok(&run(
        "change_assurance.recover",
        &json!({ "repo": root.path().to_str().unwrap() }),
    ));
    assert_eq!(payload["removed"], json!(2));
    assert_eq!(retained(root.path()), before);

    // Nothing staged is not an error, and it is not a count of one.
    let payload = ok(&run(
        "change_assurance.recover",
        &json!({ "repo": root.path().to_str().unwrap() }),
    ));
    assert_eq!(payload["removed"], json!(0));
}

/// The same inputs, run in two independent repositories, yield byte-identical
/// canonical JSON for a record, an attestation and a valid receipt, and the
/// retained layout and the library contracts are those a retained record of
/// either vintage is read from: records at `records/<id>/<revision>.json`,
/// attestations at `attestations/<id>/`. The valid receipt it emits verifies
/// against its own inputs.
///
/// Trace: FR-068-AC-10, FR-068-CON-4
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_607_record_attestation_and_receipt_reproduce_byte_identical_json() {
    let mut runs = Vec::new();
    for _ in 0..2 {
        let root = repo();
        retain_everything(root.path());
        let store = root.path().join("spec/evidence/change-assurance");
        let record = fs::read(store.join("records/change-1/1.json")).unwrap();
        let attestation = fs::read(store.join("attestations/att-1/attestation.json")).unwrap();
        let output = fs::read(store.join("attestations/att-1/output.bin")).unwrap();
        assert_eq!(output, OUTPUT);
        let receipt = ok(&run(
            "change_assurance.receipt",
            &receipt_request(root.path()),
        ));
        assert_eq!(receipt["receipt"]["outcome"], json!("valid"), "{receipt}");
        assert_eq!(receipt["receipt"]["reasons"], json!([]));
        let verified = ok(&run(
            "change_assurance.verify_receipt",
            &json!({
                "receipt_hex": hex_json(&receipt["receipt"]),
                "inputs": receipt_request(root.path()),
            }),
        ));
        assert_eq!(verified["receipt"], receipt["receipt"]);
        runs.push((
            record,
            attestation,
            canonical_json(&receipt["receipt"]).unwrap(),
        ));
    }
    assert_eq!(
        runs[0], runs[1],
        "two runs over the same inputs must agree byte for byte"
    );
    // The retained record is the document that was written, read back through
    // the schema: it names no member the caller did not supply.
    let written: Value = serde_json::from_slice(&runs[0].0).unwrap();
    assert_eq!(written["record_id"], json!("change-1"));
    assert!(written.get("digest").is_none());
}

/// `change_assurance.schema` emits the three normative assets from the binary
/// itself, and refuses a name this build does not ship.
///
/// The assets are compiled into `quoin-change-assurance` (quoin#503), so what
/// this asserts is that the bytes a consumer validates against reach it across
/// the boundary unchanged — parsed back here only to compare against the same
/// constant the writing tests use.
///
/// Trace: FR-068-AC-8
/// Provenance: agent-ix/quoin#503
#[test]
fn tc_503_610_schema_emits_the_shipped_assets_and_refuses_any_other_name() {
    let listed = ok(&run("change_assurance.schema", &json!({})));
    assert_eq!(
        listed["schemas"],
        json!([
            "change-assurance-record-v1.schema.json",
            "proof-attestation-v1.schema.json",
            "verification-receipt-v1.schema.json"
        ])
    );
    assert_eq!(listed["schema"], Value::Null);

    for name in [
        "change-assurance-record-v1.schema.json",
        "proof-attestation-v1.schema.json",
        "verification-receipt-v1.schema.json",
    ] {
        let payload = ok(&run("change_assurance.schema", &json!({ "name": name })));
        let text = payload["schema"].as_str().expect("the asset is a string");
        let document: Value = serde_json::from_str(text).expect("the asset is JSON");
        assert_eq!(
            document["$id"].as_str().unwrap().rsplit('/').next(),
            Some(name),
            "{name} came back declaring another identity",
        );
    }

    let refused = run(
        "change_assurance.schema",
        &json!({ "name": "verification-receipt-v2.schema.json" }),
    );
    failed(&refused, 3);
}

/// Write a minimal `MeasurementPlan` document protecting `entries`, under
/// `root`'s `spec/assurance/`, the root `quoin-measurement`'s plan intake
/// walks (PLAT-975, PLAT-997).
fn write_protecting_plan(root: &Path, id: &str, entries: &[&str]) {
    let mut list = String::new();
    for entry in entries {
        let _ = writeln!(list, "  - {entry}");
    }
    let document = format!(
        "---\nid: {id}\ntitle: PLAT-997 fixture\ntype: MeasurementPlan\nstatus: active\n\
         stage: observe\nmetric: change_assurance.apparatus_touched\n\
         definition_version: v1\nprotected_apparatus:\n{list}---\n\n# PLAT-997 fixture\n"
    );
    let path = root.join("spec/assurance").join(format!("{id}.md"));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, document).unwrap();
}

/// `change_assurance.receipt` resolves a `plan` link through
/// `quoin-measurement`'s own plan intake and folds FR-111's judgment into the
/// receipt: a diff touching the linked plan's `protected_apparatus` is
/// `apparatus_touched` and the receipt is `invalid`, through this operation
/// and not only through the library (closes FR-111-CON-3).
///
/// Trace: FR-111-AC-1, TC-1889
/// Provenance: PLAT-997
#[test]
fn tc_997_611_receipt_reports_apparatus_touched_through_the_operation() {
    let root = repo();
    retain_everything(root.path());
    write_protecting_plan(root.path(), "MP-997-A", &["checker/config.toml"]);

    let mut request = receipt_request(root.path());
    request["diff_paths"] = json!(["src/lib.rs", "checker/config.toml"]);
    request["plan"] = json!("MP-997-A");
    let payload = ok(&run("change_assurance.receipt", &request));
    assert_eq!(payload["receipt"]["outcome"], json!("invalid"));
    assert_eq!(payload["receipt"]["reasons"], json!(["apparatus_touched"]));

    // An unlinked receipt over the SAME store and diff is unaffected: the
    // judgment only fires when `plan` names it.
    let mut unlinked = receipt_request(root.path());
    unlinked["diff_paths"] = json!(["src/lib.rs", "checker/config.toml"]);
    let baseline = ok(&run("change_assurance.receipt", &unlinked));
    assert_eq!(baseline["receipt"]["outcome"], json!("valid"));
    assert_eq!(baseline["receipt"]["reasons"], json!([]));

    // No `MeasurementPlan` names `plan`'s id: refused, not silently ignored.
    let mut unknown_plan = receipt_request(root.path());
    unknown_plan["plan"] = json!("MP-997-DOES-NOT-EXIST");
    let refused = run("change_assurance.receipt", &unknown_plan);
    let diagnostics = failed(&refused, 2);
    assert_eq!(diagnostics[0]["context"]["field"], "plan");
}

/// A plan that protects apparatus, checked with no diff retained, is
/// `diff_missing` and `incomplete` through the operation — never vacuously
/// `valid` because the caller omitted `--diff-path` (FR-111-AC-4).
///
/// Trace: FR-111-AC-4, TC-1890
/// Provenance: PLAT-997
#[test]
fn tc_997_612_receipt_reports_diff_missing_through_the_operation() {
    let root = repo();
    retain_everything(root.path());
    write_protecting_plan(root.path(), "MP-997-B", &["checker/config.toml"]);

    let mut request = receipt_request(root.path());
    request["plan"] = json!("MP-997-B");
    let payload = ok(&run("change_assurance.receipt", &request));
    assert_eq!(payload["receipt"]["outcome"], json!("incomplete"));
    assert_eq!(payload["receipt"]["reasons"], json!(["diff_missing"]));
}

/// `diff_paths` and `plan` are bounded at their ceilings, in the shape every
/// other accumulator on this boundary uses: a list of exactly
/// [`quoin_core::ops::change_assurance::MAX_DIFF_PATHS`] entries, and an
/// entry of exactly [`quoin_core::ops::change_assurance::MAX_SCALAR_BYTES`]
/// bytes, are accepted and verified; one entry or one byte past either is
/// refused naming the field and the ceiling before any store or plan is read,
/// and so is a `plan` id one byte past its ceiling.
///
/// Trace: FR-111-CON-2, TC-1891
/// Provenance: PLAT-997
#[test]
fn tc_997_613_diff_paths_and_plan_are_bounded_at_their_ceilings() {
    use quoin_core::ops::change_assurance::{MAX_DIFF_PATHS, MAX_SCALAR_BYTES};

    // The production ceilings, as literals: a test that read them back
    // would agree with any value.
    assert_eq!(MAX_DIFF_PATHS, 65_536);
    assert_eq!(MAX_SCALAR_BYTES, 4 * 1024);

    let root = repo();
    retain_everything(root.path());
    let paths = |count: usize| json!((0..count).map(|n| format!("p{n}")).collect::<Vec<_>>());

    // At the count ceiling: accepted and verified, not refused.
    let mut at_count = receipt_request(root.path());
    at_count["diff_paths"] = paths(MAX_DIFF_PATHS);
    let payload = ok(&run("change_assurance.receipt", &at_count));
    assert_eq!(payload["receipt"]["outcome"], json!("valid"));

    // One entry past it: refused by count, before any read.
    let mut past_count = receipt_request(root.path());
    past_count["diff_paths"] = paths(MAX_DIFF_PATHS + 1);
    let diagnostics = failed(&run("change_assurance.receipt", &past_count), 2);
    let context = &diagnostics[0]["context"];
    assert_eq!(context["field"], "diff_paths");
    assert_eq!(context["limit_entries"], "65536");
    assert_eq!(context["observed_entries"], "65537");

    // One entry of exactly the scalar ceiling: accepted.
    let mut at_bytes = receipt_request(root.path());
    at_bytes["diff_paths"] = json!(["a".repeat(MAX_SCALAR_BYTES)]);
    let payload = ok(&run("change_assurance.receipt", &at_bytes));
    assert_eq!(payload["receipt"]["outcome"], json!("valid"));

    // One byte past it: refused by size.
    let mut past_bytes = receipt_request(root.path());
    past_bytes["diff_paths"] = json!(["a".repeat(MAX_SCALAR_BYTES + 1)]);
    let diagnostics = failed(&run("change_assurance.receipt", &past_bytes), 2);
    let context = &diagnostics[0]["context"];
    assert_eq!(context["field"], "diff_paths");
    assert_eq!(context["limit_bytes"], "4096");
    assert_eq!(context["observed_bytes"], "4097");

    // A `plan` id one byte past the scalar ceiling: refused by size, not
    // looked up.
    let mut past_plan = receipt_request(root.path());
    past_plan["plan"] = json!("P".repeat(MAX_SCALAR_BYTES + 1));
    let diagnostics = failed(&run("change_assurance.receipt", &past_plan), 2);
    let context = &diagnostics[0]["context"];
    assert_eq!(context["field"], "plan");
    assert_eq!(context["limit_bytes"], "4096");
}

/// `change_assurance.receipt` records `governing_plan_id` and
/// `diff_paths_supplied` on the written receipt, through the operation and not
/// only through the library: a receipt written with `--plan`/`--diff-path`
/// records the plan id and `true`; one written with neither records `null` and
/// `false` explicitly, rather than a caller's omission producing the same
/// silence a genuinely unlinked record would (PLAT-1015).
///
/// Trace: FR-111-AC-5, TC-1915
/// Provenance: PLAT-1015
#[test]
fn tc_1015_614_receipt_records_governing_plan_id_and_diff_paths_supplied() {
    let root = repo();
    retain_everything(root.path());
    write_protecting_plan(root.path(), "MP-1015-CORE", &["checker/config.toml"]);

    let mut linked = receipt_request(root.path());
    linked["diff_paths"] = json!(["src/lib.rs"]);
    linked["plan"] = json!("MP-1015-CORE");
    let payload = ok(&run("change_assurance.receipt", &linked));
    assert_eq!(
        payload["receipt"]["governing_plan_id"],
        json!("MP-1015-CORE")
    );
    assert_eq!(payload["receipt"]["diff_paths_supplied"], json!(true));

    let unlinked = receipt_request(root.path());
    let payload = ok(&run("change_assurance.receipt", &unlinked));
    assert_eq!(payload["receipt"]["governing_plan_id"], json!(null));
    assert_eq!(payload["receipt"]["diff_paths_supplied"], json!(false));
}
