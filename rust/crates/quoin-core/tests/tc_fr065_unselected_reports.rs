// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Legacy audit and assurance report paths ignore unselected change-assurance records.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test fixtures use expect and indexing as the failure report"
)]

use ix_trace_rs::trace;
use quoin_core::protocol::{Diagnostic, Response, canonical_json};
use quoin_core::runtime::{RuntimeSettings, dispatch};
use serde_json::{Value, json};
use std::path::Path;

fn invoke(op: &str, input: &Value) -> String {
    let response =
        dispatch(op, input, &RuntimeSettings::default()).unwrap_or_else(|error| Response {
            payload: Value::Null,
            diagnostics: vec![Diagnostic::from(&error)],
            outcome: error.outcome(),
        });
    assert_eq!(response.outcome.code(), 0, "{:?}", response.diagnostics);
    canonical_json(&response.payload).expect("report payload serializes")
}

#[trace("FR-065-CON-4")]
#[test]
fn existing_audit_and_assurance_reports_remain_readable_with_unselected_records_present() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repository root is above rust/crates/quoin-core");
    let golden_dir = root.join("rust/goldens/quoin-assurance");
    let corpus: Value = serde_json::from_slice(
        &std::fs::read(golden_dir.join("cases.json")).expect("assurance cases are readable"),
    )
    .expect("assurance case corpus parses");
    let case = corpus["cases"]
        .as_array()
        .and_then(|cases| {
            cases
                .iter()
                .find(|case| case["op"] == "build_case" && case["boundary"].is_null())
        })
        .expect("there is a valid legacy assurance report case");
    let input = case["input"].clone();
    let assurance_before = invoke("assurance.build_case", &input);

    let temp = tempfile::tempdir().expect("isolated report repository exists");
    let audit_input = json!({"repo": temp.path()});
    let audit_before = invoke("evidence.audit_inputs", &audit_input);
    std::fs::create_dir_all(
        temp.path()
            .join("spec/evidence/change-assurance/records/unselected"),
    )
    .expect("unselected change-assurance family is creatable");
    std::fs::write(
        temp.path()
            .join("spec/evidence/change-assurance/records/unselected/1.json"),
        br#"{"record_type":"change_assurance","record_id":"unselected"}"#,
    )
    .expect("unselected record fixture writes");
    let audit_with_unselected = invoke("evidence.audit_inputs", &audit_input);
    assert_eq!(
        audit_before, audit_with_unselected,
        "legacy evidence-audit output is unchanged"
    );
    assert_eq!(
        assurance_before,
        invoke("assurance.build_case", &input),
        "legacy assurance output stays readable"
    );
}
