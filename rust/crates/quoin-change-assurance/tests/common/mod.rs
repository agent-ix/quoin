// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Reading the committed oracle capture.
//!
//! `tests/fixtures/oracle.json` was written once by
//! `oracle/capture-change-assurance-oracle.mjs` against the retained
//! TypeScript. Nothing here runs Node, and nothing here recomputes anything the
//! fixture states: these functions only shape what the oracle already said into
//! the types this crate takes (FR-101 AC-5).

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_change_assurance::verify::input::{
    AuditReport, ReportFinding, ReportUnevaluated, RetainedAttestation, RetainedAudit, Selection,
    VerificationInput,
};
use quoin_store::{JsonValue, parse_strict_json_str};

/// The whole capture, parsed.
pub(crate) fn oracle() -> JsonValue {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("oracle.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    parse_strict_json_str(&text).expect("the capture is strict JSON")
}

/// One top-level section of the capture, as a slice of values.
pub(crate) fn section(oracle: &JsonValue, name: &str) -> Vec<JsonValue> {
    let JsonValue::Array(entries) = oracle.as_object().unwrap().get(name).unwrap() else {
        panic!("section {name} is not an array");
    };
    entries.clone()
}

/// A named member of an object.
pub(crate) fn member<'a>(value: &'a JsonValue, name: &str) -> &'a JsonValue {
    value
        .as_object()
        .unwrap_or_else(|_| panic!("expected an object to read {name} from"))
        .get(name)
        .unwrap_or_else(|| panic!("no member {name}"))
}

/// A named string member.
pub(crate) fn text<'a>(value: &'a JsonValue, name: &str) -> &'a str {
    member(value, name).as_str().unwrap()
}

/// Bytes the capture wrote as an array of byte values.
pub(crate) fn bytes(value: &JsonValue) -> Vec<u8> {
    let JsonValue::Array(entries) = value else {
        panic!("expected a byte array");
    };
    entries
        .iter()
        .map(|entry| {
            let byte = entry.as_f64().unwrap();
            u8::try_from(format!("{byte:.0}").parse::<u16>().unwrap()).unwrap()
        })
        .collect()
}

/// Shape one captured verification input into the type this crate takes.
pub(crate) fn verification_input(captured: &JsonValue) -> VerificationInput {
    let array = |name: &str| match member(captured, name) {
        JsonValue::Array(entries) => entries.clone(),
        _ => panic!("{name} is not an array"),
    };
    VerificationInput {
        record: member(captured, "record").clone(),
        parents: array("parents"),
        candidate_revision: text(captured, "candidate_revision").to_owned(),
        selections: array("selections")
            .iter()
            .map(|selection| Selection {
                proof_id: text(selection, "proof_id").to_owned(),
                attestation_digest: text(selection, "attestation_digest").to_owned(),
            })
            .collect(),
        attestations: array("attestations")
            .iter()
            .map(|pair| RetainedAttestation {
                attestation: member(pair, "attestation").clone(),
                output: match member(pair, "output") {
                    JsonValue::Null => None,
                    other => Some(bytes(other)),
                },
            })
            .collect(),
        decision_history: match member(captured, "decision_history") {
            JsonValue::Null => None,
            history => Some(history.clone()),
        },
        audits: array("audits")
            .iter()
            .map(|audit| {
                let report = member(audit, "report");
                let list = |name: &str| match member(report, name) {
                    JsonValue::Array(entries) => entries.clone(),
                    _ => panic!("{name} is not an array"),
                };
                RetainedAudit {
                    proof_id: text(audit, "proof_id").to_owned(),
                    report_digest: text(audit, "report_digest").to_owned(),
                    report: AuditReport {
                        findings: list("findings")
                            .iter()
                            .map(|finding| ReportFinding {
                                obligation: text(finding, "obligation").to_owned(),
                                kind: text(finding, "kind").to_owned(),
                            })
                            .collect(),
                        healthy: list("healthy")
                            .iter()
                            .map(|entry| entry.as_str().unwrap().to_owned())
                            .collect(),
                        unevaluated: list("unevaluated")
                            .iter()
                            .map(|entry| ReportUnevaluated {
                                obligation: text(entry, "obligation").to_owned(),
                            })
                            .collect(),
                    },
                }
            })
            .collect(),
    }
}
