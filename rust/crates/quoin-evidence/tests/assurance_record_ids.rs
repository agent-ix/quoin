// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Assurance-record identity: an opaque UUID, not derived from content.
//!
//! `tests/fixtures/assurance-record-inputs.json` is hand-authored input; no
//! recorded output is read.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "integration-test bodies: a panic here is a failing test, which is the intended signal"
)]

use ix_trace_rs::trace;
use quoin_evidence::MemoryEvidence;
use quoin_evidence::assurance_records::{
    write_experiment_record, write_operational_evidence_record,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Corpus {
    cases: Vec<CaseInput>,
}

#[derive(Debug, Deserialize)]
struct CaseInput {
    name: String,
    input: Value,
}

fn load() -> Corpus {
    serde_json::from_str(include_str!("fixtures/assurance-record-inputs.json")).unwrap()
}

/// A record's identity is an opaque UUID that is not derived from its content,
/// its file is named from that UUID, and it is read back by that UUID.
#[trace("FR-100-AC-11")]
#[test]
fn plat_989_assurance_ids_are_opaque_uuids_and_name_the_file() {
    let corpus = load();
    for case in &corpus.cases {
        let name = case.name.as_str();
        let mut store = MemoryEvidence::new();
        let experiment = case.input["record"] == "experiment";
        let publish = |store: &mut MemoryEvidence| {
            if experiment {
                let stored = write_experiment_record(store, &case.input["value"]).unwrap();
                (stored.record.record_id, stored.path)
            } else {
                let stored =
                    write_operational_evidence_record(store, &case.input["value"]).unwrap();
                (stored.record.record_id, stored.path)
            }
        };
        let (id, path) = publish(&mut store);
        let groups: Vec<usize> = id.split('-').map(str::len).collect();
        assert_eq!(groups, [8, 4, 4, 4, 12], "{name}: {id} is not a UUID");
        assert!(path.ends_with(&format!("{id}.json")), "{name}: {path}");

        // Not derived from content: the same input published again, without
        // the id it was given, is a different record.
        let (other, _) = publish(&mut store);
        assert_ne!(
            id, other,
            "{name}: the identity must not follow the content"
        );

        let found = if experiment {
            quoin_evidence::assurance_records::read_experiment_record(&store, &id)
                .unwrap()
                .is_some()
        } else {
            quoin_evidence::assurance_records::read_operational_evidence_record(&store, &id)
                .unwrap()
                .is_some()
        };
        assert!(found, "{name}: published identity cannot be read back");
    }
}
