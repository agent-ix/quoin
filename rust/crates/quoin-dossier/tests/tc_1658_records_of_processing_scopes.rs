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
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use ix_trace_rs::trace;
use quire_rs::Registry;
use quire_rs::corpus::{BundlePosture, BundleReport, Spec, validate_bundle};
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
fn processing_records(inputs: &Inputs) -> Result<RecordsOfProcessingSource, SourceFailure> {
    let report = bundle_report(inputs).map_err(|error| SourceFailure {
        source: "records".to_owned(),
        detail: error,
    })?;
    if !report.errors.is_empty() {
        return Err(SourceFailure {
            source: "records".to_owned(),
            detail: report
                .errors
                .iter()
                .map(|finding| format!("{}: {}", finding.reason, finding.message))
                .collect::<Vec<_>>()
                .join("; "),
        });
    }

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

fn security_scope_module(root: &std::path::Path) -> PathBuf {
    fs::create_dir_all(root).unwrap();
    fs::write(
        root.join("manifest.yaml"),
        r#"name: security-scope-fixture
version: 0.1.0
object_types:
  - name: processing_activity
    data_schema: {type: object}
    allowed_links: {scopes: [connection, data_sink, part]}
    body_extraction:
      yield_pattern:
        match:
          id: {from: frontmatter_field, path: [id], required: true}
          title: {from: frontmatter_field, path: [title], required: true}
          type: {from: frontmatter_field, path: [type], required: true}
  - name: connection
    data_schema: {type: object}
  - name: data_sink
    data_schema: {type: object}
  - name: part
    data_schema: {type: object}
  - name: queue
    data_schema: {type: object}
edge_types:
  scopes: {description: model processing scope, category: dependency}
semantic:
  contract_version: 1.0.0
  package: agent-ix/security-scope-fixture
  exports: [processing_activity, connection, data_sink, part, queue]
  targets: [json-schema, markdown]
  mappings: [relationships]
"#,
    )
    .unwrap();
    root.to_path_buf()
}

fn record(id: &str, kind: &str, scope_target: Option<&str>) -> String {
    let relationships = scope_target.map_or_else(String::new, |target| {
        format!("relationships:\n  - target: {target}\n    type: scopes\n")
    });
    format!("---\nid: {id}\ntitle: {id}\ntype: {kind}\n{relationships}---\n")
}

fn input(work: &std::path::Path, module: &std::path::Path) -> Inputs {
    Inputs {
        scope: work.to_path_buf(),
        modules: vec![module.to_path_buf()],
        clause_bindings: Vec::new(),
        pack: None,
        soup_binding: None,
        soup_records: None,
        soup_support: None,
        snapshot: None,
        release_verify: None,
        as_of: None,
        version: String::new(),
    }
}

fn bundle_report(inputs: &Inputs) -> Result<BundleReport, String> {
    let module_paths = inputs
        .modules
        .iter()
        .map(PathBuf::as_path)
        .collect::<Vec<_>>();
    let registry =
        Registry::load_module_set_strict(&module_paths).map_err(|error| error.to_string())?;
    let document_root = inputs.scope.join("spec");
    let spec = Spec::from_path(&document_root);
    Ok(validate_bundle(
        &spec,
        &registry,
        BundlePosture::Strict,
        &document_root,
        &inputs.scope,
    ))
}

#[trace("FR-207-AC-6")]
#[test]
fn generated_records_of_processing_ignores_iso_covers_edges() {
    let work = tempfile::tempdir().unwrap();
    let module = security_scope_module(&work.path().join("module"));
    fs::create_dir(work.path().join("spec")).unwrap();
    fs::write(
        work.path().join("spec/ACT-COVERS.md"),
        record("ACT-COVERS", "processing_activity", None),
    )
    .unwrap();
    let inputs = input(work.path(), &module);

    let report = generate(
        &registry(),
        &["records-of-processing".to_owned()],
        work.path(),
        &inputs,
    )
    .unwrap();

    let document = report
        .documents
        .iter()
        .find(|document| document.id == "DOS-ROP")
        .expect("records-of-processing report exists");
    assert!(document.gaps.is_empty(), "covers-only sink added gaps");
    let generated = std::fs::read_to_string(work.path().join("DOS-ROP.md")).unwrap();
    assert!(!generated.contains("SINK-COVERED"));
    assert!(!generated.contains("Must not be selected"));
    assert!(!generated.contains("transfer region — SINK-COVERED"));
    assert!(!generated.contains("ModelFactsUnavailable"));
}

#[trace("FR-207-AC-6")]
#[test]
fn dangling_and_wrong_kind_scopes_fail_validation_before_generation() {
    for (case, target_doc, expected_reason, target_id, diagnostic_detail) in [
        (
            "dangling",
            None,
            "dangling-reference",
            "MISSING-1",
            "dangling reference",
        ),
        (
            "wrong-kind",
            Some(("QUEUE-1", record("QUEUE-1", "queue", None))),
            "disallowed-edge-target",
            "QUEUE-1",
            "does not satisfy",
        ),
    ] {
        let work = tempfile::tempdir().unwrap();
        let module = security_scope_module(&work.path().join("module"));
        let spec = work.path().join("spec");
        fs::create_dir(&spec).unwrap();
        fs::write(
            spec.join("ACT-1.md"),
            record(
                "ACT-1",
                "processing_activity",
                Some(if case == "dangling" {
                    "MISSING-1"
                } else {
                    "QUEUE-1"
                }),
            ),
        )
        .unwrap();
        if let Some((name, content)) = target_doc {
            fs::write(spec.join(format!("{name}.md")), content).unwrap();
        }
        let inputs = input(work.path(), &module);
        let validation = bundle_report(&inputs).expect("bundle validation runs");
        let finding = validation
            .errors
            .iter()
            .find(|finding| finding.reason == expected_reason)
            .unwrap_or_else(|| panic!("{case}: expected {expected_reason}: {validation:?}"));
        assert!(finding.message.contains(target_id), "{case}: {finding:?}");
        assert!(
            finding
                .message
                .to_ascii_lowercase()
                .contains(diagnostic_detail),
            "{case}: {finding:?}"
        );
        if case == "wrong-kind" {
            assert!(
                finding
                    .message
                    .contains("[\"connection\", \"data_sink\", \"part\"]")
            );
        }
        let output = work.path().join("output");
        let result = generate(
            &registry(),
            &["records-of-processing".to_owned()],
            &output,
            &inputs,
        );

        let error = result.expect_err("invalid scopes must stop dossier generation");
        assert!(error.to_string().contains("scopes"), "{case}: {error}");
        assert!(!output.join("DOS-ROP.md").exists(), "{case}");
    }
}
