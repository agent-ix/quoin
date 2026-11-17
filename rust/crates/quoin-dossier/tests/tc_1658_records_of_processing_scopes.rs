// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Integration coverage for model-scope selection in records-of-processing output.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test assertions intentionally fail the test"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use ix_trace_rs::trace;
use quoin_dossier::{
    Inputs, MatrixSource, RecordsOfProcessingSource, SourceFailure, Sources, builtin_registry,
    generate,
};
use quoin_quire::model::{FlowFact, FlowFactEdge, FlowReport};
use quoin_quire::records::{Contract, Corpus, Document};
use serde_json::{Map, json};

struct NoMatrix;

impl MatrixSource for NoMatrix {
    fn matrix(
        &self,
        _inputs: &Inputs,
    ) -> Result<quoin_assurance::matrix::MatrixOutput, SourceFailure> {
        Err(SourceFailure {
            source: "matrix".to_owned(),
            detail: "not used by records-of-processing".to_owned(),
        })
    }
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "source adapters share the fallible generator contract"
)]
fn processing_records(_inputs: &Inputs) -> Result<RecordsOfProcessingSource, SourceFailure> {
    let activity = Document {
        path: PathBuf::from("spec/ACT-COVERS.md"),
        id: "ACT-COVERS".to_owned(),
        kind: "processing_activity".to_owned(),
        frontmatter: json!({"title":"Activity", "role":"controller"})
            .as_object()
            .cloned()
            .unwrap_or_else(Map::new),
        edges: Vec::new(),
        tables: BTreeMap::new(),
        sections: BTreeMap::new(),
        raw: String::new(),
        contract: Contract::NotChecked,
    };
    let flow = FlowReport {
        schema: "quire.flow-report/v1",
        rules: Vec::new(),
        vocabularies: BTreeMap::new(),
        facts: BTreeMap::from([
            (
                "processing_activity".to_owned(),
                vec![FlowFact {
                    id: "ACT-COVERS".to_owned(),
                    repo: None,
                    path: "spec/ACT-COVERS.md".to_owned(),
                    fields: BTreeMap::new(),
                    edges: vec![FlowFactEdge {
                        edge: "covers".to_owned(),
                        direction: "outgoing",
                        target: quoin_quire::model::FlowDocumentRef {
                            id: "SINK-COVERED".to_owned(),
                            repo: None,
                        },
                        archetype: "data_sink".to_owned(),
                    }],
                }],
            ),
            (
                "data_sink".to_owned(),
                vec![FlowFact {
                    id: "SINK-COVERED".to_owned(),
                    repo: None,
                    path: "spec/SINK-COVERED.md".to_owned(),
                    fields: BTreeMap::from([
                        ("kind".to_owned(), json!("archive")),
                        ("processor".to_owned(), json!("Must not be selected")),
                    ]),
                    edges: Vec::new(),
                }],
            ),
        ]),
        denials: Vec::new(),
        remediations: Vec::new(),
    };
    Ok(RecordsOfProcessingSource {
        records: Corpus {
            documents: vec![activity],
            unreadable: Vec::new(),
            malformed: Vec::new(),
        },
        flow: Some(flow),
    })
}

fn registry() -> quoin_dossier::Registry {
    builtin_registry(Sources {
        matrix: Arc::new(NoMatrix),
        rmm: |_| panic!("unexpected discharge source"),
        soup: |_| panic!("unexpected SOUP source"),
        policy: |_| panic!("unexpected policy source"),
        threat_model: |_| panic!("unexpected threat-model source"),
        records_of_processing: processing_records,
    })
    .unwrap()
}

#[trace("FR-207-AC-6")]
#[test]
fn generated_records_of_processing_ignores_iso_covers_edges() {
    let work = tempfile::tempdir().unwrap();
    let inputs = Inputs {
        scope: work.path().to_path_buf(),
        modules: Vec::new(),
        clause_bindings: Vec::new(),
        pack: None,
        soup_binding: None,
        soup_records: None,
        soup_support: None,
        snapshot: None,
        release_verify: None,
        as_of: None,
        version: String::new(),
    };

    generate(
        &registry(),
        &["records-of-processing".to_owned()],
        work.path(),
        &inputs,
    )
    .unwrap();

    let generated = std::fs::read_to_string(work.path().join("DOS-ROP.md")).unwrap();
    assert!(!generated.contains("SINK-COVERED"));
    assert!(!generated.contains("Must not be selected"));
    assert!(!generated.contains("transfer region — SINK-COVERED"));
    assert!(!generated.contains("ModelFactsUnavailable"));
}
