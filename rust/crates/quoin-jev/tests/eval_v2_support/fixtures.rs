// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Synthetic corpus rows shared by the offline tests and by the runner's
//! preflight (PR #620 re-review, finding 4): the request-digest pins are
//! computed on [`four_modes`]'s rows, so the live runner needs them too.

use serde_json::{Value, json};

use super::corpus::{self, CorpusFile};
use super::keys::Mode;

/// The synthetic rows' test body.
pub(crate) const TEST_BODY: &str = "#[test]\nfn tc_001_refuses_oversize() {\n    \
    let error = check(\"x\".repeat(5000)).unwrap_err();\n    \
    assert_eq!(error.code, Code::Refused);\n}";

/// The synthetic rows' code body.
pub(crate) const CODE_BODY: &str = "pub fn check(input: String) -> Result<(), Error> {\n    \
    match input.len() {\n        0 => Err(Error::empty()),\n        \
    n if n > 4096 => { log(\"too big }\"); Err(Error::refused()) }\n        \
    _ => Ok(()),\n    }\n}";

/// One synthetic row of `mode`, with `truth` as given.
/// Its FR is `FR-` plus the id's last three digits, so distinct ids are
/// distinct FRs and PLAT-1024's three-natural-rows-per-FR rule holds.
pub(crate) fn row(id: &str, mode: Mode, split: &str, truth: &Value) -> Value {
    let fr = format!("FR-{}", id.get(id.len().saturating_sub(3)..).unwrap_or(id));
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
        "strata": {"fr_id": fr, "req_kind": "functional", "test_kind": null,
                   "crate": "quoin-core", "ears_pattern": null},
        "requirement": {"fr_id": fr, "ac_id": format!("{fr}-AC-1"),
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

/// A truth record.
pub(crate) fn truth(answer: &Value, kind: &str, alternatives: &[&str]) -> Value {
    json!({"answer": answer, "kind": kind, "alternatives": alternatives, "rationale": "stated"})
}

/// A corpus file's text around `rows`.
pub(crate) fn corpus_text(rows: &[Value]) -> String {
    serde_json::to_string_pretty(&json!({
        "schema": corpus::SCHEMA,
        "sampling_rule": "every row, synthetic",
        "seed": 7,
        "source_commit": "0000000",
        "rows": rows,
    }))
    .unwrap_or_default()
}

/// Parses a synthetic corpus of `rows`.
///
/// # Errors
/// When the rows do not make a corpus.
pub(crate) fn parse(rows: &[Value]) -> Result<CorpusFile, String> {
    corpus::parse(&corpus_text(rows))
}

/// A four-row corpus, one per mode, with truth each mode supports. Its row
/// in each mode is that mode's canonical row for the request-digest pins
/// (`preflight::REQUEST_DIGEST_PINS`): changing a row here changes every
/// pin, so do it only with a version bump of every variant.
///
/// # Errors
/// Never, for these rows; the signature is [`parse`]'s.
pub(crate) fn four_modes() -> Result<CorpusFile, String> {
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
