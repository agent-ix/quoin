// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `change_assurance` operations, reached by their wire names (quoin#457).
//!
//! `tc_447_600_every_operation_is_invoked_by_name_by_some_test` requires every
//! entry in `OPERATIONS` to be handed to the REAL BINARY, spelled, by an
//! integration test — because a swapped match arm leaves the source-scanning
//! drift guard, the unit tests and `quoin-difftest` all green and breaks only
//! for a user. Route coverage is not handler coverage (quoin#443, quoin#447),
//! and this file is the route coverage for all six change-assurance
//! operations. Each test asserts a payload member only its own handler writes.
//!
//! It is also where the criteria that `tests/change-assurance-command.test.ts`
//! carried before quoin#457 deleted it are restated against the engine. The
//! criteria that remain properties of the COMMAND rather than of the engine —
//! the 0/1/2 exit grammar, the packaged schema assets, and the static
//! boundaries over `src/commands/change-assurance/` — are restated in
//! `tests/change-assurance-command-surface.test.ts`, which is the side that
//! still owns them.
//!
//! Nothing here recomputes what the Rust decided and compares it with itself.
//! `tests/fixtures/change-assurance-oracle.json` is an extract of the capture
//! taken against the retained TypeScript BEFORE it was deleted, so every digest
//! and verdict asserted below is the TypeScript's answer (FR-101 AC-5).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

struct Run {
    stdout: String,
    stderr: String,
    status: i32,
}

/// Invoke the binary with `op` on argv and `request` on stdin.
fn run(op: &str, request: &Value) -> Run {
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

/// The bytes of an oracle `output` array.
fn bytes_of(value: &Value) -> Vec<u8> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|byte| u8::try_from(byte.as_u64().unwrap()).unwrap())
        .collect()
}

/// One named case of the oracle capture.
fn case(name: &str) -> Value {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/change-assurance-oracle.json")).unwrap();
    oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == name)
        .unwrap_or_else(|| panic!("the fixture has no case named {name}"))
        .clone()
}

/// Every named case of the oracle capture.
fn cases() -> Vec<Value> {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/change-assurance-oracle.json")).unwrap();
    oracle["cases"].as_array().unwrap().clone()
}

/// A repository root that exists only for one test.
fn repo() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// Seal and retain a case's record, answering the digest the store now holds.
fn seal_record(root: &Path, case: &Value) -> String {
    let payload = ok(&run(
        "change_assurance.seal_record",
        &json!({
            "repo": root.to_str().unwrap(),
            "record_hex": hex_json(&case["record_body"]),
        }),
    ));
    payload["record"]["digest"].as_str().unwrap().to_owned()
}

/// Retain a case's sealed attestations and their outputs.
fn intake_all(root: &Path, case: &Value) {
    for pair in case["attestations"].as_array().unwrap() {
        ok(&run(
            "change_assurance.intake",
            &json!({
                "repo": root.to_str().unwrap(),
                "attestation_hex": hex_json(&pair["attestation"]),
                "output_hex": hex(&bytes_of(&pair["output"])),
            }),
        ));
    }
}

/// The receipt request a case describes.
fn receipt_request(root: &Path, case: &Value) -> Value {
    let audits = case["audits"].clone();
    json!({
        "repo": root.to_str().unwrap(),
        "record_digest": case["record_digest"],
        "candidate_revision": case["candidate_revision"],
        "parent_digests": [],
        "selections": case["selections"],
        "decisions_hex": hex_json(&case["decision_history"]),
        "audits_hex": if audits.as_array().unwrap().is_empty() {
            Value::Null
        } else {
            Value::String(hex_json(&audits))
        },
    })
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

/// `change_assurance.seal_record` seals an explicit body, retains it under its
/// digest, and reports that digest and the path it now occupies. A body that
/// supplies `digest`, or one carrying a member the FR-063 schema does not
/// declare, is refused and nothing is retained.
///
/// `path` is the member only this handler writes; a swap onto `intake` would
/// answer `directory` and fail here.
///
/// Trace: FR-068-AC-1
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_600_seal_record_seals_an_explicit_body_and_retains_it() {
    let fixture = case("everything-agrees");
    let root = repo();
    let payload = ok(&run(
        "change_assurance.seal_record",
        &json!({
            "repo": root.path().to_str().unwrap(),
            "record_hex": hex_json(&fixture["record_body"]),
        }),
    ));
    let digest = fixture["record_digest"].as_str().unwrap();
    assert_eq!(payload["record"]["digest"], fixture["record_digest"]);
    assert_eq!(payload["retained"], json!(true));
    assert_eq!(
        payload["path"].as_str().unwrap(),
        root.path()
            .join("spec/evidence/change-assurance/records")
            .join(format!("{digest}.json"))
            .to_str()
            .unwrap()
    );
    assert_eq!(
        retained(root.path()),
        vec!["records".to_owned(), format!("records/{digest}.json")]
    );

    // A supplied digest, and an undeclared member, each refused — over a fresh
    // repository, so "nothing is retained" is a statement about this request
    // rather than about one that had already written the same bytes.
    for body in [
        {
            let mut supplied = fixture["record_body"].clone();
            supplied["digest"] = json!("0".repeat(64));
            supplied
        },
        {
            let mut extra = fixture["record_body"].clone();
            extra["undeclared"] = json!("x");
            extra
        },
    ] {
        let empty = repo();
        let result = run(
            "change_assurance.seal_record",
            &json!({
                "repo": empty.path().to_str().unwrap(),
                "record_hex": hex_json(&body),
            }),
        );
        failed(&result, 3);
        assert_eq!(retained(empty.path()), Vec::<String>::new());
    }
}

/// `change_assurance.seal_attestation` derives the retained output's media
/// type, digest and size from the bytes and the caller's declaration, and
/// NOTHING else. A body supplying `retained_output` or `digest` is refused.
///
/// The proof that nothing else is derived is a comparison of the whole sealed
/// document against the oracle's, member by member: an assertion naming only
/// the three derived members would not notice a fourth.
///
/// Trace: FR-068-AC-2, FR-068-CON-2
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_601_seal_attestation_derives_only_the_retained_output() {
    let fixture = case("everything-agrees");
    let sealed = fixture["attestations"][0]["attestation"].clone();
    let output = bytes_of(&fixture["attestations"][0]["output"]);
    let mut body = sealed.clone();
    let media_type = body["retained_output"]["media_type"].clone();
    let object = body.as_object_mut().unwrap();
    object.remove("retained_output");
    object.remove("digest");

    let payload = ok(&run(
        "change_assurance.seal_attestation",
        &json!({
            "attestation_hex": hex_json(&body),
            "output_hex": hex(&output),
            "media_type": media_type,
        }),
    ));
    assert_eq!(
        payload["attestation"], sealed,
        "the sealed attestation differs from the one the TypeScript sealed"
    );

    for supplied in ["digest", "retained_output"] {
        let mut carried = body.clone();
        carried[supplied] = sealed[supplied].clone();
        let result = run(
            "change_assurance.seal_attestation",
            &json!({
                "attestation_hex": hex_json(&carried),
                "output_hex": hex(&output),
                "media_type": media_type,
            }),
        );
        let diagnostics = failed(&result, 3);
        assert_eq!(diagnostics[0]["context"]["supplied"], supplied);
    }
}

/// `change_assurance.intake` retains the exact attestation and output bytes as
/// one pair, is idempotent for byte-identical inputs, and refuses output whose
/// bytes contradict what the attestation records — retaining nothing on that
/// path.
///
/// `size_bytes` is the member only this handler writes.
///
/// Trace: FR-068-AC-3
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_602_intake_retains_an_exact_pair_atomically_and_idempotently() {
    let fixture = case("everything-agrees");
    let pair = fixture["attestations"][0].clone();
    let output = bytes_of(&pair["output"]);
    let digest = pair["attestation"]["digest"].as_str().unwrap().to_owned();
    let root = repo();
    let request = json!({
        "repo": root.path().to_str().unwrap(),
        "attestation_hex": hex_json(&pair["attestation"]),
        "output_hex": hex(&output),
    });

    let first = ok(&run("change_assurance.intake", &request));
    assert_eq!(first["retained"], json!(true));
    assert_eq!(first["size_bytes"], json!(output.len()));
    assert_eq!(
        first["directory"].as_str().unwrap(),
        root.path()
            .join("spec/evidence/change-assurance/attestations")
            .join(&digest)
            .to_str()
            .unwrap()
    );

    // The retained bytes are the producer's own, byte for byte — which is the
    // whole reason the output crosses the boundary as hex rather than as text.
    let directory = root
        .path()
        .join("spec/evidence/change-assurance/attestations")
        .join(&digest);
    assert_eq!(fs::read(directory.join("output.bin")).unwrap(), output);

    let again = ok(&run("change_assurance.intake", &request));
    assert_eq!(again["retained"], json!(false));

    let contradicting = repo();
    let result = run(
        "change_assurance.intake",
        &json!({
            "repo": contradicting.path().to_str().unwrap(),
            "attestation_hex": hex_json(&pair["attestation"]),
            "output_hex": hex(b"other bytes"),
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
/// record, the named parents, the explicit selections and the supplied
/// decisions and audits. A retained attestation that no selection names has NO
/// effect: a stray upload cannot discharge a proof.
///
/// The comparison is between two receipts over the same store, one of which had
/// an extra attestation retained in it. Anything the second receipt discovered
/// on its own would move its digest.
///
/// Trace: FR-068-AC-4
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_603_receipt_uses_only_what_the_caller_named() {
    let selected = case("everything-agrees");
    let stray = case("attestation-not-selected");
    let root = repo();
    seal_record(root.path(), &selected);
    intake_all(root.path(), &selected);
    let before = ok(&run(
        "change_assurance.receipt",
        &receipt_request(root.path(), &selected),
    ));

    // Retain every attestation the stray case carries, including the one its
    // own selections do not name, and ask the SAME question again.
    intake_all(root.path(), &stray);
    let stored = retained(root.path());
    assert!(
        stored.len() > 3,
        "the store must hold more than the selected pair for this to prove \
         anything; it holds {stored:?}"
    );
    let after = ok(&run(
        "change_assurance.receipt",
        &receipt_request(root.path(), &selected),
    ));
    assert_eq!(after["receipt"], before["receipt"]);
    assert_eq!(before["receipt"]["digest"], selected["receipt_digest"]);
}

/// `unavailable` and `not_computed` producer results, and evidence an audit
/// left unevaluated, survive the crossing as their own receipt outcomes and
/// reasons. None of them is converted into a pass or a failure.
///
/// Each case is asserted against the reasons the TypeScript recorded, so a
/// collapse of `result_unavailable` into `result_failed` is visible here even
/// though both are non-`valid`.
///
/// Trace: FR-068-AC-5
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_604_qualified_evidence_is_neither_a_pass_nor_a_failure() {
    // The bounded domain of qualified evidence, walked exhaustively rather than
    // sampled: a producer that could not run, one that declined to compute, an
    // audit that never evaluated, and evidence that was never selected at all.
    let mut reasons: BTreeSet<String> = BTreeSet::new();
    for name in [
        "producer-unavailable",
        "producer-did-not-compute",
        "audit-left-it-unevaluated",
        "attestation-not-selected",
    ] {
        let fixture = case(name);
        let root = repo();
        seal_record(root.path(), &fixture);
        intake_all(root.path(), &fixture);
        let result = run(
            "change_assurance.receipt",
            &receipt_request(root.path(), &fixture),
        );
        // A verdict is an ANSWER: the engine exits 0 and the caller applies the
        // exit-1 policy to a non-`valid` receipt.
        let payload = ok(&result);
        assert_eq!(
            payload["receipt"]["outcome"], fixture["receipt_outcome"],
            "{name}"
        );
        assert_eq!(
            payload["receipt"]["reasons"], fixture["receipt_reasons"],
            "{name}"
        );
        assert_ne!(payload["receipt"]["outcome"], json!("valid"), "{name}");
        // Nor a failure: `incomplete` is its own outcome, and the per-proof row
        // says which evidence was qualified rather than collapsing to a
        // verdict.
        assert_ne!(payload["receipt"]["outcome"], json!("invalid"), "{name}");
        let proof = &payload["receipt"]["proofs"][0];
        assert_ne!(proof["outcome"], json!("valid"), "{name}");
        assert_eq!(proof["reasons"], fixture["receipt_reasons"], "{name}");
        for reason in proof["reasons"].as_array().expect("reasons is an array") {
            reasons.insert(reason.as_str().expect("a reason is a string").to_owned());
        }
    }
    // Anti-vacuity: four distinct qualified states were actually reached, so a
    // corpus that silently lost one cannot pass this as an empty loop.
    assert_eq!(
        reasons,
        BTreeSet::from([
            "attestation_missing".to_owned(),
            "audit_not_evaluated".to_owned(),
            "result_not_computed".to_owned(),
            "result_unavailable".to_owned(),
        ])
    );

    // Missing evidence stays missing rather than becoming an empty pass: the
    // proof row names no attestation at all.
    let fixture = case("attestation-not-selected");
    let root = repo();
    seal_record(root.path(), &fixture);
    intake_all(root.path(), &fixture);
    let payload = ok(&run(
        "change_assurance.receipt",
        &receipt_request(root.path(), &fixture),
    ));
    assert_eq!(
        payload["receipt"]["proofs"][0]["attestation_digest"],
        Value::Null
    );
}

/// `change_assurance.verify_receipt` accepts a sealed receipt and refuses one
/// whose semantic content or digest was altered.
///
/// Trace: FR-068-AC-7
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_605_verify_receipt_accepts_a_seal_and_refuses_an_alteration() {
    let fixture = case("everything-agrees");
    let root = repo();
    seal_record(root.path(), &fixture);
    intake_all(root.path(), &fixture);
    let sealed = ok(&run(
        "change_assurance.receipt",
        &receipt_request(root.path(), &fixture),
    ))["receipt"]
        .clone();

    let payload = ok(&run(
        "change_assurance.verify_receipt",
        &json!({ "receipt_hex": hex_json(&sealed) }),
    ));
    assert_eq!(payload["receipt"]["digest"], fixture["receipt_digest"]);
    assert_eq!(payload["receipt"]["outcome"], json!("valid"));

    // The semantic field and the digest each altered on their own: an
    // implementation that only re-read the declared digest would accept the
    // first, and one that only recomputed would accept the second.
    for altered in [
        {
            let mut edited = sealed.clone();
            edited["outcome"] = json!("invalid");
            edited
        },
        {
            let mut edited = sealed.clone();
            edited["digest"] = json!("0".repeat(64));
            edited
        },
    ] {
        let result = run(
            "change_assurance.verify_receipt",
            &json!({ "receipt_hex": hex_json(&altered) }),
        );
        failed(&result, 3);
    }
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
    let fixture = case("everything-agrees");
    let root = repo();
    seal_record(root.path(), &fixture);
    intake_all(root.path(), &fixture);
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

/// Every case of the oracle capture reproduces, through the real binary, the
/// receipt the retained TypeScript produced: the same outcome, the same
/// reasons, and the same digest over the same canonical bytes.
///
/// This is the parity statement the cutover rests on. The fixture was captured
/// against `src/change-assurance/` before its deletion, so these are the
/// TypeScript's answers and not the Rust's own.
///
/// Trace: FR-068-AC-10, FR-068-CON-4
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_607_every_oracle_case_reproduces_the_typescripts_receipt() {
    let all = cases();
    // Anti-vacuity: a fixture that failed to load would make the loop below
    // pass over nothing.
    assert!(
        all.len() >= 5,
        "the oracle extract holds {} case(s); the fixture is broken, not the crate",
        all.len()
    );
    let mut verdicts = Vec::new();
    for fixture in &all {
        let name = fixture["name"].as_str().unwrap();
        let root = repo();
        assert_eq!(
            seal_record(root.path(), fixture),
            fixture["record_digest"].as_str().unwrap(),
            "{name}"
        );
        intake_all(root.path(), fixture);
        let payload = ok(&run(
            "change_assurance.receipt",
            &receipt_request(root.path(), fixture),
        ));
        assert_eq!(
            payload["receipt"]["digest"], fixture["receipt_digest"],
            "{name}"
        );
        assert_eq!(
            payload["receipt"]["outcome"], fixture["receipt_outcome"],
            "{name}"
        );
        assert_eq!(
            payload["receipt"]["reasons"], fixture["receipt_reasons"],
            "{name}"
        );
        verdicts.push(payload["receipt"]["outcome"].clone());
    }
    // A corpus in which every case agreed on one verdict would pin the shape
    // and not the decision.
    assert!(
        verdicts.iter().any(|v| v == "valid") && verdicts.iter().any(|v| v != "valid"),
        "the corpus must contain both a valid receipt and a non-valid one: {verdicts:?}"
    );
}

/// DECLARED DIVERGENCE (quoin#457, `DIVERGENCE.md` §4): a malformed
/// `attestation_digest` inside a receipt makes the retained TypeScript THROW
/// while building its own receipt — exit 4, no receipt — where the Rust emits
/// `null` for the member it cannot read and returns a receipt, exit 0.
///
/// It is accepted and it is an exit-code change, so it is asserted here rather
/// than left for a harness to discover. `quoin-difftest` covers only
/// `core.ping`, so there is no live difftest case to declare it in.
///
/// Trace: FR-068-AC-7, FR-101
/// Provenance: agent-ix/quoin#457
#[test]
fn tc_457_608_a_malformed_digest_inside_a_receipt_is_refused_not_thrown() {
    let fixture = case("everything-agrees");
    let root = repo();
    seal_record(root.path(), &fixture);
    intake_all(root.path(), &fixture);
    let sealed = ok(&run(
        "change_assurance.receipt",
        &receipt_request(root.path(), &fixture),
    ))["receipt"]
        .clone();

    let mut malformed = sealed.clone();
    malformed["record_digest"] = json!("not-a-digest");
    let result = run(
        "change_assurance.verify_receipt",
        &json!({ "receipt_hex": hex_json(&malformed) }),
    );
    // The engine answers with a REFUSAL of the caller's document (3), never an
    // internal fault (4). The TypeScript's throw produced a 4 here.
    assert_eq!(
        result.status, 3,
        "a malformed member of a supplied receipt is the caller's mistake, not \
         an engine fault: stdout {} stderr {}",
        result.stdout, result.stderr
    );
}
