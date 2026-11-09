// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! A clause-binding report the pinned quire engine really produces is read by
//! quoin's reader.
//!
//! The reader is `deny_unknown_fields`, so a member quire no longer emits and
//! the reader still required would refuse every genuine report. This builds a
//! clause set, evaluates it with `quire_rs`, wraps it exactly as
//! `quire clauses evaluate --format json` does, and reads the bytes back.
use quoin_quire::ClauseBindingPayload;
use quoin_quire_types::{ClauseBindingOutcome, ClauseBindingReport};

/// Trace: FR-046-AC-1
/// Provenance: quoin#502
#[test]
fn tc_502_030_an_engine_produced_binding_report_reads_through_quoin()
-> Result<(), Box<dyn std::error::Error>> {
    let set: quire_rs::ClauseSet = serde_json::from_value(serde_json::json!({
        "schemaVersion": "clause-set-v1",
        "authority": "example.invalid",
        "id": "synthetic-widget-rules",
        "title": "Synthetic widget rules",
        "version": "1.0.0",
        "rights": { "structure": "original", "text": "original" },
        "clauses": [
            { "id": "SYN-001", "force": "mandatory", "expectedOutputs": [] }
        ]
    }))?;
    set.validate()?;

    let payload =
        ClauseBindingPayload::from_report(set.evaluate(&std::collections::BTreeMap::new()));
    let bytes = serde_json::to_vec(&payload)?;

    let read: ClauseBindingReport = serde_json::from_slice(&bytes)?;
    assert_eq!(read.clause_set.id, "synthetic-widget-rules");
    assert_eq!(read.clauses.len(), 1);
    assert_eq!(
        read.clauses.first().map(|clause| clause.outcome),
        Some(ClauseBindingOutcome::Binding)
    );
    Ok(())
}
