// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Matrix-bound integration coverage for the threat-model dossier renderer.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration-test assertions intentionally fail the test"
)]

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use quoin_dossier::{GapKind, render_threat_model};
use quoin_quire::records::{Contract, Corpus, Document, Unreadable};
use serde_json::{Map, Value, json};

fn doc(id: &str, kind: &str, fields: Value, edges: &[(&str, &str)]) -> Document {
    Document {
        path: format!("spec/{id}.md").into(),
        id: id.to_owned(),
        kind: kind.to_owned(),
        frontmatter: match fields {
            Value::Object(fields) => fields,
            _ => Map::new(),
        },
        edges: edges
            .iter()
            .map(|(target, relation)| ((*target).to_owned(), (*relation).to_owned()))
            .collect(),
        tables: BTreeMap::new(),
        sections: BTreeMap::new(),
        raw: String::new(),
        contract: Contract::NotChecked,
    }
}

fn corpus(documents: Vec<Document>) -> Corpus {
    Corpus {
        documents,
        unreadable: Vec::new(),
        malformed: Vec::new(),
    }
}

fn render(
    records: &Corpus,
    report: Option<&quoin_quire::model::FlowReport>,
) -> quoin_dossier::Generated {
    render_threat_model(records, report).unwrap()
}

fn denial(check: &str, subject: &str, archetype: &str) -> quoin_quire::model::FlowDenial {
    quoin_quire::model::FlowDenial {
        rule: check.to_owned(),
        check: format!("trace:{check}"),
        subject: quoin_quire::model::FlowSubjectRef {
            id: subject.to_owned(),
            repo: None,
            archetype: archetype.to_owned(),
        },
        matched: BTreeMap::new(),
        remediations: Vec::new(),
        rationale: "Fixture finding.".to_owned(),
        severity: "warning",
    }
}

fn flow(denials: Vec<quoin_quire::model::FlowDenial>) -> quoin_quire::model::FlowReport {
    quoin_quire::model::FlowReport {
        schema: "quire.flow-report/v1",
        rules: Vec::new(),
        vocabularies: BTreeMap::new(),
        facts: BTreeMap::new(),
        denials,
        remediations: Vec::new(),
    }
}

fn complete_records() -> Corpus {
    corpus(vec![
        doc(
            "TM-1",
            "threat_model",
            json!({"method":"stride_per_interaction", "reviewed_by":"A. Person"}),
            &[
                ("PART-2", "scopes"),
                ("PART-1", "scopes"),
                ("BOUND-NET", "scopes"),
                ("BOUND-INTERNAL", "scopes"),
                ("CONN-1", "scopes"),
                ("CONN-2", "scopes"),
                ("SINK-1", "scopes"),
                ("SINK-2", "scopes"),
                ("THREAT-1", "considers"),
            ],
        ),
        doc("PART-2", "part", json!({}), &[]),
        doc("PART-1", "part", json!({}), &[]),
        doc(
            "BOUND-NET",
            "trust_boundary",
            json!({"trust_level":"restricted"}),
            &[],
        ),
        doc(
            "BOUND-INTERNAL",
            "trust_boundary",
            json!({"trust_level":"internal"}),
            &[],
        ),
        doc(
            "CONN-1",
            "connection",
            json!({}),
            &[("CLASS-PHI", "carries"), ("BOUND-NET", "crosses")],
        ),
        doc(
            "CONN-2",
            "connection",
            json!({}),
            &[
                ("BOUND-INTERNAL", "crosses"),
                ("CLASS-PII", "carries"),
                ("SINK-2", "delivers_to"),
            ],
        ),
        doc(
            "SINK-1",
            "data_sink",
            json!({"kind":"database"}),
            &[("CLASS-PHI", "carries")],
        ),
        doc("SINK-2", "data_sink", json!({"kind":"archive"}), &[]),
        doc(
            "CLASS-PHI",
            "data_classification",
            json!({"regulated_categories":["health"]}),
            &[],
        ),
        doc(
            "CLASS-PII",
            "data_classification",
            json!({"regulated_categories":["identity"]}),
            &[],
        ),
        doc(
            "THREAT-1",
            "threat",
            json!({"stride_category":"tampering"}),
            &[("CONN-1", "threatens"), ("SINK-1", "threatens")],
        ),
        doc(
            "CONTROL-1",
            "control",
            json!({"safeguards":"mutual TLS"}),
            &[("THREAT-1", "mitigates"), ("SINK-1", "protects")],
        ),
        doc(
            "CONTROL-2",
            "control",
            json!({"safeguards":"restricted access"}),
            &[("SINK-2", "protects")],
        ),
        doc(
            "RISK-1",
            "risk",
            json!({"decision":"accepted", "decided_by":"A. Person"}),
            &[("THREAT-1", "arises_from")],
        ),
        doc(
            "RISK-2",
            "risk",
            json!({"decision":"undecided"}),
            &[("THREAT-1", "arises_from")],
        ),
    ])
}

#[trace(
    "FR-202-AC-1",
    "FR-202-AC-8",
    "FR-202-AC-15",
    "TC-601",
    "TC-608",
    "TC-615"
)]
#[test]
fn tc_1546_report_renders_authored_facts_without_inference() {
    let out = render(&complete_records(), Some(&flow(Vec::new())));
    assert!(out.body.contains("CLASS-PHI (health)"));
    assert!(out.body.contains("BOUND-NET (restricted)"));
    assert!(out.body.contains("THREAT-1 (tampering)"));
    assert!(out.body.contains("CONTROL-1 (mutual TLS)"));
    assert!(out.body.contains("RISK-1: accepted, A. Person"));
    assert!(out.body.contains("RISK-2: undecided, not stated"));
    assert!(out.body.contains("CLASS-PII (identity)"));
    assert!(out.body.contains("CONTROL-2 (restricted access)"));
    assert_eq!(out.body.matches("CONTROL-1 (mutual TLS)").count(), 2);
    assert!(!out.body.to_ascii_lowercase().contains("adequate"));
    assert_eq!(out.gaps, [] as [quoin_dossier::Gap; 0]);
}

#[trace(
    "FR-202-AC-2",
    "FR-202-AC-3",
    "FR-202-AC-4",
    "FR-202-AC-12",
    "TC-602",
    "TC-603",
    "TC-604",
    "TC-612"
)]
#[test]
fn tc_1546_report_renders_only_engine_findings_with_severity() {
    let reports = flow(vec![
        denial("crossing-without-threat", "CONN-2", "connection"),
        denial("classified-sink-without-threat", "SINK-2", "data_sink"),
        denial("threat-without-treatment", "THREAT-1", "threat"),
    ]);
    let out = render(&complete_records(), Some(&reports));
    assert!(
        out.body
            .contains("GAP: crossing-without-threat CONN-2 (warning)")
    );
    assert!(
        out.body
            .contains("GAP: classified-sink-without-threat SINK-2 (warning)")
    );
    assert!(
        out.body
            .contains("GAP: threat-without-treatment THREAT-1 (warning)")
    );
    assert_eq!(out.gaps.len(), 3);
    assert_eq!(out.gaps[0].gap_kind, GapKind::CrossingWithoutThreat);
    assert_eq!(out.gaps[1].gap_kind, GapKind::ClassifiedSinkWithoutThreat);
    assert_eq!(out.gaps[2].gap_kind, GapKind::ThreatWithoutTreatment);
    assert_eq!(
        render(&complete_records(), Some(&flow(Vec::new()))).gaps,
        [] as [quoin_dossier::Gap; 0]
    );
}

#[trace("FR-202-AC-5", "TC-605")]
#[test]
fn tc_1546_report_without_model_has_one_document_gap() {
    let out = render(&corpus(Vec::new()), Some(&flow(Vec::new())));
    assert!(!out.body.contains("## TM-"));
    assert_eq!(out.gaps.len(), 1);
    assert_eq!(out.gaps[0].gap_kind, GapKind::NoThreatModel);
}

#[trace("FR-202-AC-6", "FR-202-AC-7", "TC-606", "TC-607")]
#[test]
fn tc_1546_complete_report_is_deterministic_and_has_no_gaps() {
    let mut records = complete_records();
    records.documents.retain(|document| {
        !matches!(
            document.id.as_str(),
            "PART-1"
                | "BOUND-INTERNAL"
                | "CONN-2"
                | "SINK-2"
                | "CLASS-PII"
                | "CONTROL-2"
                | "RISK-2"
        )
    });
    records.documents[0].edges.retain(|(target, _)| {
        target != "PART-1"
            && target != "BOUND-INTERNAL"
            && target != "CONN-2"
            && target != "SINK-2"
            && target != "THREAT-1"
    });
    records.documents[0]
        .edges
        .push(("THREAT-1".to_owned(), "considers".to_owned()));
    let report = flow(Vec::new());
    let first = render(&records, Some(&report));
    let second = render(&records, Some(&report));
    assert_eq!(first, second);
    assert!(first.body.contains("### Open findings\n\nnone"));
    assert_eq!(first.gaps, [] as [quoin_dossier::Gap; 0]);
}

#[trace("FR-202-AC-9", "TC-609")]
#[test]
fn tc_1546_unreadable_threat_record_is_a_source_failure() {
    let mut records = complete_records();
    records.unreadable.push(Unreadable {
        path: "spec/threats/blocked.md".into(),
        reason: "permission denied".to_owned(),
    });
    let error = render_threat_model(&records, Some(&flow(Vec::new()))).unwrap_err();
    assert_eq!(error.source, "threat-model-records");
    assert!(error.detail.contains("spec/threats/blocked.md"));
}

#[trace("FR-202-AC-10", "FR-202-AC-13", "TC-610", "TC-613")]
#[test]
fn tc_1546_treatment_findings_require_considers_and_scope_edges() {
    let mut records = complete_records();
    records.documents.push(doc(
        "TM-2",
        "threat_model",
        json!({}),
        &[("CONN-2", "scopes")],
    ));
    records.documents.push(doc(
        "TM-3",
        "threat_model",
        json!({}),
        &[("CONN-1", "scopes"), ("THREAT-1", "considers")],
    ));
    let out = render(
        &records,
        Some(&flow(vec![
            denial("crossing-without-threat", "CONN-2", "connection"),
            denial("threat-without-treatment", "THREAT-1", "threat"),
        ])),
    );
    let rows = "GAP: threat-without-treatment THREAT-1 (warning)";
    assert_eq!(out.body.matches(rows).count(), 2);
    let crossing_rows = "GAP: crossing-without-threat CONN-2 (warning)";
    assert_eq!(out.body.matches(crossing_rows).count(), 2);
    assert_eq!(out.gaps.len(), 2);
    let model_without_considers = out
        .body
        .split("## TM-2")
        .nth(1)
        .unwrap()
        .split("## TM-3")
        .next()
        .unwrap();
    assert!(!model_without_considers.contains(rows));
}

#[trace("FR-202-AC-11", "TC-611")]
#[test]
fn tc_1546_scope_lists_parts_then_boundaries_and_only_connection_rows() {
    let mut records = complete_records();
    records.documents[0]
        .edges
        .iter_mut()
        .find(|(target, relation)| target == "CONN-2" && relation == "scopes")
        .unwrap()
        .1 = "covers".to_owned();
    let out = render(&records, None).body;
    assert!(out.contains("Covered parts: PART-1, PART-2"));
    assert!(out.contains("Covered trust boundaries: BOUND-INTERNAL, BOUND-NET"));
    assert!(out.contains("| CONN-1 |"));
    assert!(!out.contains("| CONN-2 |"));
    assert!(out.find("Covered parts:").unwrap() < out.find("Covered trust boundaries:").unwrap());
}

#[trace("FR-202-AC-14", "TC-614")]
#[test]
fn tc_1546_findings_and_gaps_follow_check_order() {
    let out = render(
        &complete_records(),
        Some(&flow(vec![
            denial("threat-without-treatment", "THREAT-1", "threat"),
            denial("classified-sink-without-threat", "SINK-2", "data_sink"),
            denial("crossing-without-threat", "CONN-2", "connection"),
        ])),
    );
    let crossing = out.body.find("GAP: crossing-without-threat").unwrap();
    let sink = out
        .body
        .find("GAP: classified-sink-without-threat")
        .unwrap();
    let threat = out.body.find("GAP: threat-without-treatment").unwrap();
    assert!(crossing < sink && sink < threat);
    assert_eq!(out.gaps.len(), 3);
    assert_eq!(
        out.gaps.iter().map(|gap| gap.gap_kind).collect::<Vec<_>>(),
        [
            GapKind::CrossingWithoutThreat,
            GapKind::ClassifiedSinkWithoutThreat,
            GapKind::ThreatWithoutTreatment,
        ]
    );
    assert!(out.body.find("| CONN-1 |").unwrap() < out.body.find("| CONN-2 |").unwrap());
    assert!(out.body.find("#### SINK-1").unwrap() < out.body.find("#### SINK-2").unwrap());
    assert!(out.body.contains("Covered parts: PART-1, PART-2"));
    let no_model = render(&corpus(Vec::new()), Some(&flow(Vec::new())));
    assert_eq!(no_model.gaps[0].gap_kind, GapKind::NoThreatModel);
    assert_eq!(no_model.gaps.len(), 1);
}

#[trace("FR-202-AC-16", "TC-616")]
#[test]
fn tc_1546_report_uses_only_the_selected_corpus() {
    let out = render(&complete_records(), None);
    assert!(out.body.contains("## TM-1"));
    assert!(!out.body.contains("TM-OUTSIDE"));
    assert!(!out.body.contains("THREAT-OUTSIDE"));
}
