// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The retained run and its analysis projection remain a separate immutable
//! pair keyed by the same suite and timestamp (FR-151, FR-152).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in an integration test, a panic is the failure report"
)]

use ix_trace_rs::trace;
use quoin_evidence::record::{RecordRequest, record_run_with_analysis_projection};
use quoin_evidence::store::read_bindings;
use quoin_evidence::types::{Outcome, RunEntry};
use quoin_evidence::{EvidenceSource, MemoryEvidence, ObligationId, SuiteId, SymbolId};
use quoin_quire_types::Obligation;
use serde_json::{Value, json};

fn request() -> RecordRequest {
    RecordRequest {
        suite: SuiteId::new("ANALYSIS-1"),
        tool: "filament-analyze 0.1.0".to_owned(),
        evidence_kind: None,
        lineage: None,
        timestamp: "2026-08-17T00:00:00Z".to_owned(),
        entries: vec![RunEntry::new("consistency", Outcome::Pass)],
    }
}

fn projection(disposition: &str) -> Value {
    json!({
        "protocol": "quoin.analysis-report-projection/v1",
        "disposition": disposition,
        "agreed_status": "satisfied",
        "producer": {"name":"filament-analyze", "version":"0.1.0"},
        "engines": []
    })
}

#[trace("FR-151-CON-2")]
#[test]
fn recording_keeps_the_run_schema_and_projection_as_one_immutable_pair() {
    let mut source = MemoryEvidence::new();
    let request = request();
    let projection = projection("agreement");

    let first = record_run_with_analysis_projection(&mut source, &request, &[], &projection)
        .unwrap();

    let run_bytes = source.read(&first.run_path).unwrap().unwrap();
    let run: Value = serde_json::from_str(&run_bytes).unwrap();
    let mut run_fields: Vec<_> = run.as_object().unwrap().keys().cloned().collect();
    run_fields.sort();
    assert_eq!(
        run_fields,
        vec![
            "entries".to_owned(),
            "schemaVersion".to_owned(),
            "suite".to_owned(),
            "timestamp".to_owned(),
            "tool".to_owned()
        ]
    );
    let entry = &run["entries"][0];
    let mut entry_fields: Vec<_> = entry.as_object().unwrap().keys().cloned().collect();
    entry_fields.sort();
    assert_eq!(entry_fields, vec!["outcome".to_owned(), "symbol".to_owned()]);

    let suffix = first.run_path.strip_prefix("runs/").unwrap();
    let projection_path = format!("analysis-reports/{suffix}");
    assert_eq!(run["suite"], "ANALYSIS-1");
    assert_eq!(run["timestamp"], request.timestamp);
    let projection_bytes = source.read(&projection_path).unwrap().unwrap();
    assert_eq!(serde_json::from_str::<Value>(&projection_bytes).unwrap(), projection);

    let second =
        record_run_with_analysis_projection(&mut source, &request, &[], &projection).unwrap();
    assert_eq!(first, second);
    assert_eq!(source.read(&first.run_path).unwrap().unwrap(), run_bytes);
    assert_eq!(
        source.read(&projection_path).unwrap().unwrap(),
        projection_bytes
    );

    assert!(
        record_run_with_analysis_projection(
            &mut source,
            &request,
            &[],
            &projection("refuted")
        )
        .is_err()
    );
    assert_eq!(source.read(&first.run_path).unwrap().unwrap(), run_bytes);
    assert_eq!(
        source.read(&projection_path).unwrap().unwrap(),
        projection_bytes
    );
}

#[trace("FR-152-CON-1")]
#[test]
fn obligation_ids_travel_only_through_the_existing_trace_ids_field() {
    let mut source = MemoryEvidence::new();
    let mut request = request();
    request.entries[0].trace_ids = Some(vec!["FR-001-AC-1".to_owned()]);
    let obligations = [Obligation {
        id: "FR-001-AC-1".to_owned(),
        identity: None,
        document: None,
        statement: "stated obligation".to_owned(),
        target_ids: None,
        method: Some("Test".to_owned()),
        criticality: None,
        parameters: None,
    }];
    let projection = projection("agreement");

    let outcome = record_run_with_analysis_projection(
        &mut source,
        &request,
        &obligations,
        &projection,
    )
    .unwrap();
    let run_bytes = source.read(&outcome.run_path).unwrap().unwrap();
    let run: Value = serde_json::from_str(&run_bytes).unwrap();
    let mut run_fields: Vec<_> = run.as_object().unwrap().keys().cloned().collect();
    run_fields.sort();
    assert_eq!(
        run_fields,
        vec![
            "entries".to_owned(),
            "schemaVersion".to_owned(),
            "suite".to_owned(),
            "timestamp".to_owned(),
            "tool".to_owned()
        ]
    );
    assert_eq!(run["entries"][0]["traceIds"], json!(["FR-001-AC-1"]));
    assert!(run.get("obligations").is_none());

    let bindings = read_bindings(&source).unwrap().bindings;
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].obligation, ObligationId::new("FR-001-AC-1"));
    assert_eq!(bindings[0].suite, SuiteId::new("ANALYSIS-1"));
    assert_eq!(bindings[0].symbols, vec![SymbolId::new("consistency")]);
    assert_eq!(outcome.bound, vec![ObligationId::new("FR-001-AC-1")]);
}
