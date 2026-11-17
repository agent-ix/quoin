// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The threat-model dossier generator, rendering authored records and engine flow findings.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use quoin_quire::records::{Corpus, Document};

use crate::ThreatModelSource;
use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::markdown::{cell, prose, unusable_input};
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};

/// Reads selected authored records and the flow report from quire-rs.
pub(crate) struct ThreatModelGenerator(
    pub(crate) fn(&Inputs) -> Result<ThreatModelSource, SourceFailure>,
);

impl Generator for ThreatModelGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "threat-model",
            id: "DOS-TM",
            title: "Threat model",
            needs_as_of: false,
        }
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        let source = (self.0)(inputs)?;
        render_threat_model(&source.records, source.flow.as_ref())
    }
}

/// Render a threat-model dossier from the selected records and flow findings.
///
/// This pure entry point is also used by callers that already hold the typed
/// record corpus and relation report; it performs no I/O and does not recompute
/// flow checks.
///
/// # Errors
///
/// Returns the corpus source failure when the selected records include
/// unreadable or malformed inputs.
pub fn render_threat_model(
    corpus: &Corpus,
    flow: Option<&quoin_quire::model::FlowReport>,
) -> Result<Generated, SourceFailure> {
    if let Some(failure) = unusable_input("threat-model-records", corpus) {
        return Err(failure);
    }
    let records = corpus.documents.iter().collect::<Vec<_>>();
    let by_id: BTreeMap<&str, &Document> = records
        .iter()
        .map(|document| (document.id.as_str(), *document))
        .collect();
    let mut models = records
        .iter()
        .copied()
        .filter(|document| document.kind == "threat_model")
        .collect::<Vec<_>>();
    models.sort_by(|left, right| crate::order::dossier_cmp(&left.id, &right.id));

    let mut body = String::from("# Threat model\n\n");
    let mut gaps = Vec::new();
    if models.is_empty() {
        gaps.push(Gap::new(
            GapKind::NoThreatModel,
            "DOS-TM",
            "no threat model record exists in the selected scope",
        ));
    } else {
        let denials = relevant_denials(flow);
        let mut gap_keys = BTreeSet::new();
        for model in models {
            let covered = edge_targets(model, "covers");
            let connections = selected(&covered, "connection", &by_id);
            let sinks = selected(&covered, "data_sink", &by_id);
            let parts = selected(&covered, "part", &by_id);
            let boundaries = selected(&covered, "trust_boundary", &by_id);
            let considered = considered_threats(model, &connections, &sinks, &records);
            let open = findings_for(&denials, &connections, &sinks, &considered);

            let _ = writeln!(body, "## {}\n", cell(&model.id));
            render_scope(&mut body, model, &parts, &boundaries);
            render_crossings(&mut body, &connections, &records, &by_id);
            render_sinks(&mut body, &sinks, &records, &by_id);
            render_findings(&mut body, &mut gaps, &mut gap_keys, &open);
        }
    }
    Ok(Generated {
        body,
        document_version: None,
        gaps,
    })
}

#[derive(Clone, Copy)]
struct Finding<'a> {
    check: &'a str,
    subject: &'a str,
    severity: &'a str,
}

fn relevant_denials(flow: Option<&quoin_quire::model::FlowReport>) -> Vec<Finding<'_>> {
    let mut findings = flow
        .into_iter()
        .flat_map(|report| report.denials.iter())
        .filter_map(|denial| {
            let check = denial
                .check
                .strip_prefix("trace:")
                .unwrap_or(denial.check.as_str());
            matches!(
                check,
                "crossing-without-threat"
                    | "classified-sink-without-threat"
                    | "threat-without-treatment"
            )
            .then_some(Finding {
                check,
                subject: denial.subject.id.as_str(),
                severity: denial.severity,
            })
        })
        .collect::<Vec<_>>();
    findings.sort_by(|left, right| {
        check_order(left.check)
            .cmp(&check_order(right.check))
            .then_with(|| crate::order::dossier_cmp(left.subject, right.subject))
    });
    findings.dedup_by(|left, right| left.check == right.check && left.subject == right.subject);
    findings
}

fn findings_for<'a>(
    denials: &'a [Finding<'a>],
    connections: &[&Document],
    sinks: &[&Document],
    threats: &[&Document],
) -> Vec<Finding<'a>> {
    let connections = connections
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();
    let sinks = sinks
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();
    let threats = threats
        .iter()
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();
    denials
        .iter()
        .copied()
        .filter(|finding| match finding.check {
            "crossing-without-threat" => connections.contains(finding.subject),
            "classified-sink-without-threat" => sinks.contains(finding.subject),
            "threat-without-treatment" => threats.contains(finding.subject),
            _ => false,
        })
        .collect()
}

fn render_scope(
    body: &mut String,
    model: &Document,
    parts: &[&Document],
    boundaries: &[&Document],
) {
    body.push_str("### Scope\n\n");
    body.push_str("Method: ");
    body.push_str(&cell(field(model, "method").unwrap_or("not stated")));
    body.push_str("\n\nReviewed by: ");
    body.push_str(&cell(field(model, "reviewed_by").unwrap_or("not stated")));
    body.push_str("\n\nCovered parts: ");
    body.push_str(&join_ids(parts));
    body.push_str("\n\nCovered trust boundaries: ");
    body.push_str(&join_ids(boundaries));
    body.push_str("\n\n");
    for heading in ["Scope", "Assumptions", "Out of scope"] {
        let _ = writeln!(body, "#### {heading}\n");
        match model.sections.get(heading) {
            Some(text) if !text.trim().is_empty() => {
                body.push_str(&prose(text));
                body.push_str("\n\n");
            }
            _ => body.push_str("Not stated.\n\n"),
        }
    }
}

fn render_crossings(
    body: &mut String,
    connections: &[&Document],
    records: &[&Document],
    by_id: &BTreeMap<&str, &Document>,
) {
    body.push_str("### Covered crossings\n\n| Connection | Carried classifications and categories | Boundary and trust level | Threats (STRIDE class) | Controls and safeguards | Acceptance |\n| --- | --- | --- | --- | --- | --- |\n");
    for connection in connections {
        let threats = threats_for(connection, records);
        let classification = classifications_for(connection, by_id);
        let boundaries = edge_targets(connection, "crosses")
            .iter()
            .filter_map(|id| by_id.get(id.as_str()).copied())
            .filter(|record| record.kind == "trust_boundary")
            .map(boundary_text)
            .collect::<Vec<_>>();
        let boundary = if boundaries.is_empty() {
            "none".to_owned()
        } else {
            boundaries.join(", ")
        };
        let controls = controls_for(connection, &threats, records);
        let acceptance = acceptance_for(&threats, records);
        let _ = writeln!(
            body,
            "| {} | {} | {} | {} | {} | {} |",
            cell(&connection.id),
            cell(&classification),
            cell(&boundary),
            cell(&threat_text(&threats)),
            cell(&control_text(&controls)),
            cell(&acceptance),
        );
    }
    body.push('\n');
}

fn render_sinks(
    body: &mut String,
    sinks: &[&Document],
    records: &[&Document],
    by_id: &BTreeMap<&str, &Document>,
) {
    body.push_str("### Covered sinks\n\n");
    if sinks.is_empty() {
        body.push_str("None.\n\n");
        return;
    }
    for sink in sinks {
        let threats = threats_for(sink, records);
        let classes = sink_classifications(sink, records, by_id);
        let controls = controls_for(sink, &threats, records);
        let _ = writeln!(body, "#### {}\n", cell(&sink.id));
        let _ = writeln!(
            body,
            "Kind: {}\n",
            cell(field(sink, "kind").unwrap_or("not stated"))
        );
        let _ = writeln!(body, "Classifications: {}\n", cell(&classes));
        let _ = writeln!(body, "Threats: {}\n", cell(&threat_text(&threats)));
        let _ = writeln!(
            body,
            "Controls and safeguards: {}\n",
            cell(&control_text(&controls))
        );
    }
}

fn render_findings(
    body: &mut String,
    gaps: &mut Vec<Gap>,
    gap_keys: &mut BTreeSet<(String, String)>,
    findings: &[Finding<'_>],
) {
    body.push_str("### Open findings\n\n");
    if findings.is_empty() {
        body.push_str("none\n\n");
        return;
    }
    for finding in findings {
        let _ = writeln!(
            body,
            "GAP: {} {} ({})",
            finding.check,
            cell(finding.subject),
            finding.severity
        );
        let Some((kind, detail)) = gap_for(finding) else {
            continue;
        };
        let key = (kind.as_str().to_owned(), finding.subject.to_owned());
        if gap_keys.insert(key) {
            let gap = Gap::new(kind, finding.subject, detail);
            gaps.push(if kind == GapKind::NoThreatModel {
                gap
            } else {
                gap.for_element(finding.subject)
            });
        }
    }
    body.push('\n');
}

fn edge_targets(document: &Document, relation: &str) -> Vec<String> {
    let mut targets = document
        .edges
        .iter()
        .filter(|(_, edge)| edge == relation)
        .map(|(target, _)| target.clone())
        .collect::<Vec<_>>();
    targets.sort_by(|left, right| crate::order::dossier_cmp(left, right));
    targets.dedup();
    targets
}

fn selected<'a>(
    targets: &[String],
    kind: &str,
    by_id: &BTreeMap<&'a str, &'a Document>,
) -> Vec<&'a Document> {
    targets
        .iter()
        .filter_map(|id| by_id.get(id.as_str()).copied())
        .filter(|document| document.kind == kind)
        .collect()
}

fn considered_threats<'a>(
    model: &Document,
    connections: &[&Document],
    sinks: &[&Document],
    records: &[&'a Document],
) -> Vec<&'a Document> {
    let subjects = connections
        .iter()
        .chain(sinks)
        .map(|document| document.id.as_str())
        .collect::<BTreeSet<_>>();
    let considered = edge_targets(model, "considers")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut threats = records
        .iter()
        .copied()
        .filter(|document| document.kind == "threat" && considered.contains(&document.id))
        .filter(|document| {
            document.edges.iter().any(|(target, relation)| {
                relation == "threatens" && subjects.contains(target.as_str())
            })
        })
        .collect::<Vec<_>>();
    threats.sort_by(|left, right| crate::order::dossier_cmp(&left.id, &right.id));
    threats.dedup_by(|left, right| left.id == right.id);
    threats
}

fn threats_for<'a>(subject: &Document, records: &[&'a Document]) -> Vec<&'a Document> {
    let mut threats = records
        .iter()
        .copied()
        .filter(|document| {
            document.kind == "threat"
                && document
                    .edges
                    .iter()
                    .any(|(target, relation)| relation == "threatens" && target == &subject.id)
        })
        .collect::<Vec<_>>();
    threats.sort_by(|left, right| crate::order::dossier_cmp(&left.id, &right.id));
    threats
}

fn classifications_for(connection: &Document, by_id: &BTreeMap<&str, &Document>) -> String {
    let ids = edge_targets(connection, "carries");
    classification_text(&ids, by_id)
}

fn sink_classifications(
    sink: &Document,
    records: &[&Document],
    by_id: &BTreeMap<&str, &Document>,
) -> String {
    let mut ids = edge_targets(sink, "carries");
    for connection in records.iter().filter(|record| record.kind == "connection") {
        if edge_targets(connection, "delivers_to").contains(&sink.id) {
            ids.extend(edge_targets(connection, "carries"));
        }
    }
    ids.sort_by(|left, right| crate::order::dossier_cmp(left, right));
    ids.dedup();
    classification_text(&ids, by_id)
}

fn classification_text(ids: &[String], by_id: &BTreeMap<&str, &Document>) -> String {
    let values = ids
        .iter()
        .filter_map(|id| by_id.get(id.as_str()).copied())
        .filter(|document| document.kind == "data_classification")
        .map(|document| {
            let categories = field_text(document, "regulated_categories");
            format!("{} ({categories})", document.id)
        })
        .collect::<Vec<_>>();
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(", ")
    }
}

fn controls_for<'a>(
    subject: &Document,
    threats: &[&Document],
    records: &[&'a Document],
) -> Vec<&'a Document> {
    let threat_ids = threats
        .iter()
        .map(|threat| threat.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut controls = records
        .iter()
        .copied()
        .filter(|document| {
            document.kind == "control"
                && document.edges.iter().any(|(target, relation)| {
                    (relation == "protects" && target == &subject.id)
                        || (relation == "mitigates" && threat_ids.contains(target.as_str()))
                })
        })
        .collect::<Vec<_>>();
    controls.sort_by(|left, right| crate::order::dossier_cmp(&left.id, &right.id));
    controls
}

fn acceptance_for(threats: &[&Document], records: &[&Document]) -> String {
    let threat_ids = threats
        .iter()
        .map(|threat| threat.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut risks = records
        .iter()
        .copied()
        .filter(|document| {
            document.kind == "risk"
                && document.edges.iter().any(|(target, relation)| {
                    relation == "arises_from" && threat_ids.contains(target.as_str())
                })
        })
        .collect::<Vec<_>>();
    risks.sort_by(|left, right| crate::order::dossier_cmp(&left.id, &right.id));
    if risks.is_empty() {
        return "none".to_owned();
    }
    risks
        .iter()
        .map(|risk| {
            format!(
                "{}: {}, {}",
                risk.id,
                field(risk, "decision").unwrap_or("not stated"),
                field(risk, "decided_by").unwrap_or("not stated"),
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn threat_text(threats: &[&Document]) -> String {
    if threats.is_empty() {
        return "none".to_owned();
    }
    threats
        .iter()
        .map(|threat| {
            format!(
                "{} ({})",
                threat.id,
                field(threat, "stride_category").unwrap_or("not stated")
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn control_text(controls: &[&Document]) -> String {
    if controls.is_empty() {
        return "none".to_owned();
    }
    controls
        .iter()
        .map(|control| {
            format!(
                "{} ({})",
                control.id,
                field(control, "safeguards").unwrap_or("not stated")
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn boundary_text(boundary: &Document) -> String {
    format!(
        "{} ({})",
        boundary.id,
        field(boundary, "trust_level").unwrap_or("not stated")
    )
}

fn join_ids(documents: &[&Document]) -> String {
    if documents.is_empty() {
        "none".to_owned()
    } else {
        documents
            .iter()
            .map(|document| cell(&document.id))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn field<'a>(document: &'a Document, name: &str) -> Option<&'a str> {
    document
        .frontmatter
        .get(name)
        .and_then(serde_json::Value::as_str)
}

fn field_text(document: &Document, name: &str) -> String {
    match document.frontmatter.get(name) {
        Some(serde_json::Value::String(value)) => value.clone(),
        Some(serde_json::Value::Array(values)) => {
            let values = values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>();
            if values.is_empty() {
                "not stated".to_owned()
            } else {
                values.join(", ")
            }
        }
        _ => "not stated".to_owned(),
    }
}

fn gap_for(finding: &Finding<'_>) -> Option<(GapKind, &'static str)> {
    match finding.check {
        "crossing-without-threat" => Some((
            GapKind::CrossingWithoutThreat,
            "the flow report found no threat on this covered crossing",
        )),
        "classified-sink-without-threat" => Some((
            GapKind::ClassifiedSinkWithoutThreat,
            "the flow report found no threat on this classified sink",
        )),
        "threat-without-treatment" => Some((
            GapKind::ThreatWithoutTreatment,
            "the flow report found no control or accepted risk for this threat",
        )),
        _ => None,
    }
}

fn check_order(check: &str) -> u8 {
    match check {
        "crossing-without-threat" => 0,
        "classified-sink-without-threat" => 1,
        "threat-without-treatment" => 2,
        _ => u8::MAX,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::*;
    use serde_json::{Map, Value, json};

    fn document(
        id: &str,
        kind: &str,
        fields: Value,
        edges: &[(&str, &str)],
        sections: &[(&str, &str)],
    ) -> Document {
        Document {
            path: format!("spec/{id}.md").into(),
            id: id.to_owned(),
            kind: kind.to_owned(),
            frontmatter: fields.as_object().cloned().unwrap_or_else(Map::new),
            edges: edges
                .iter()
                .map(|(target, relation)| ((*target).to_owned(), (*relation).to_owned()))
                .collect(),
            tables: BTreeMap::new(),
            sections: sections
                .iter()
                .map(|(heading, content)| ((*heading).to_owned(), (*content).to_owned()))
                .collect(),
            raw: String::new(),
            contract: quoin_quire::records::Contract::NotChecked,
        }
    }

    fn corpus(documents: Vec<Document>) -> Corpus {
        Corpus {
            documents,
            unreadable: Vec::new(),
            malformed: Vec::new(),
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

    #[test]
    fn tc_1546_report_preserves_authored_crossing_threat_control_and_acceptance_facts() {
        let records = corpus(vec![
            document(
                "TM-1",
                "threat_model",
                json!({"method":"STRIDE", "reviewed_by":"A. Person"}),
                &[("CONN-1", "covers"), ("THREAT-1", "considers")],
                &[
                    ("Scope", "Selected boundary."),
                    ("Assumptions", "None."),
                    ("Out of scope", "Legacy adapter."),
                ],
            ),
            document(
                "CONN-1",
                "connection",
                json!({}),
                &[("CLASS-PHI", "carries"), ("BOUND-NET", "crosses")],
                &[],
            ),
            document(
                "BOUND-NET",
                "trust_boundary",
                json!({"trust_level":"restricted"}),
                &[],
                &[],
            ),
            document(
                "CLASS-PHI",
                "data_classification",
                json!({"regulated_categories":["health"]}),
                &[],
                &[],
            ),
            document(
                "THREAT-1",
                "threat",
                json!({"stride_category":"tampering"}),
                &[("CONN-1", "threatens")],
                &[],
            ),
            document(
                "CONTROL-1",
                "control",
                json!({"safeguards":"mutual TLS"}),
                &[("THREAT-1", "mitigates")],
                &[],
            ),
            document(
                "RISK-1",
                "risk",
                json!({"decision":"accepted", "decided_by":"A. Person"}),
                &[("THREAT-1", "arises_from")],
                &[],
            ),
            document(
                "RISK-2",
                "risk",
                json!({"decision":"undecided", "decided_by":null}),
                &[("THREAT-1", "arises_from")],
                &[],
            ),
        ]);
        let generated = render_threat_model(&records, Some(&flow(Vec::new()))).unwrap();
        assert!(generated.body.contains("CLASS-PHI (health)"));
        assert!(generated.body.contains("BOUND-NET (restricted)"));
        assert!(generated.body.contains("THREAT-1 (tampering)"));
        assert!(generated.body.contains("CONTROL-1 (mutual TLS)"));
        assert!(generated.body.contains("RISK-1: accepted, A. Person"));
        assert!(generated.body.contains("RISK-2: undecided, not stated"));
        assert!(generated.body.contains("Selected boundary."));
        assert!(generated.gaps.is_empty());
    }

    #[test]
    fn tc_1546_report_renders_only_engine_findings_and_deduplicates_model_gaps() {
        let records = corpus(vec![
            document(
                "TM-1",
                "threat_model",
                json!({}),
                &[("CONN-1", "covers")],
                &[],
            ),
            document(
                "TM-2",
                "threat_model",
                json!({}),
                &[("CONN-1", "covers")],
                &[],
            ),
            document("CONN-1", "connection", json!({}), &[], &[]),
        ]);
        let denial = quoin_quire::model::FlowDenial {
            rule: "crossing".to_owned(),
            check: "trace:crossing-without-threat".to_owned(),
            subject: quoin_quire::model::FlowSubjectRef {
                id: "CONN-1".to_owned(),
                repo: None,
                archetype: "connection".to_owned(),
            },
            matched: BTreeMap::new(),
            remediations: Vec::new(),
            rationale: "Authored threat record required.".to_owned(),
            severity: "warning",
        };
        let report = flow(vec![denial]);
        let generated = render_threat_model(&records, Some(&report)).unwrap();
        assert_eq!(
            generated
                .body
                .matches("GAP: crossing-without-threat CONN-1 (warning)")
                .count(),
            2
        );
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].gap_kind, GapKind::CrossingWithoutThreat);
        assert!(generated.body.contains("(warning)"));
    }

    #[test]
    fn tc_1546_report_without_model_emits_one_document_gap_and_no_findings() {
        let generated = render_threat_model(&corpus(Vec::new()), Some(&flow(Vec::new()))).unwrap();
        assert!(!generated.body.contains("## TM-"));
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].gap_kind, GapKind::NoThreatModel);
    }

    #[test]
    fn tc_1546_sink_collects_direct_and_delivered_classifications_and_deduplicates_controls() {
        let records = corpus(vec![
            document(
                "TM-1",
                "threat_model",
                json!({}),
                &[("SINK-1", "covers")],
                &[],
            ),
            document(
                "SINK-1",
                "data_sink",
                json!({"kind":"database"}),
                &[("CLASS-PHI", "carries")],
                &[],
            ),
            document(
                "CONN-1",
                "connection",
                json!({}),
                &[("CLASS-PII", "carries"), ("SINK-1", "delivers_to")],
                &[],
            ),
            document(
                "CLASS-PHI",
                "data_classification",
                json!({"regulated_categories":["health"]}),
                &[],
                &[],
            ),
            document(
                "CLASS-PII",
                "data_classification",
                json!({"regulated_categories":["identity"]}),
                &[],
                &[],
            ),
            document(
                "THREAT-1",
                "threat",
                json!({"stride_category":"information-disclosure"}),
                &[("SINK-1", "threatens")],
                &[],
            ),
            document(
                "CONTROL-1",
                "control",
                json!({"safeguards":"encryption"}),
                &[("SINK-1", "protects"), ("THREAT-1", "mitigates")],
                &[],
            ),
        ]);
        let generated = render_threat_model(&records, Some(&flow(Vec::new()))).unwrap();
        assert!(
            generated
                .body
                .contains("CLASS-PHI (health), CLASS-PII (identity)")
        );
        assert_eq!(generated.body.matches("CONTROL-1 (encryption)").count(), 1);
    }

    #[test]
    fn tc_1546_threat_without_treatment_is_listed_per_model_but_gapped_once() {
        let records = corpus(vec![
            document(
                "TM-1",
                "threat_model",
                json!({}),
                &[("CONN-1", "covers"), ("THREAT-1", "considers")],
                &[],
            ),
            document(
                "TM-2",
                "threat_model",
                json!({}),
                &[("CONN-1", "covers"), ("THREAT-1", "considers")],
                &[],
            ),
            document(
                "TM-3",
                "threat_model",
                json!({}),
                &[("CONN-1", "covers")],
                &[],
            ),
            document("CONN-1", "connection", json!({}), &[], &[]),
            document(
                "THREAT-1",
                "threat",
                json!({"stride_category":"repudiation"}),
                &[("CONN-1", "threatens")],
                &[],
            ),
            document(
                "RISK-1",
                "risk",
                json!({"decision":"undecided"}),
                &[("THREAT-1", "arises_from")],
                &[],
            ),
        ]);
        let denial = quoin_quire::model::FlowDenial {
            rule: "treatment".to_owned(),
            check: "trace:threat-without-treatment".to_owned(),
            subject: quoin_quire::model::FlowSubjectRef {
                id: "THREAT-1".to_owned(),
                repo: None,
                archetype: "threat".to_owned(),
            },
            matched: BTreeMap::new(),
            remediations: Vec::new(),
            rationale: "Treatment is not recorded.".to_owned(),
            severity: "error",
        };
        let generated = render_threat_model(&records, Some(&flow(vec![denial]))).unwrap();
        assert_eq!(
            generated
                .body
                .matches("GAP: threat-without-treatment THREAT-1 (error)")
                .count(),
            2
        );
        assert!(generated.body.contains("## TM-1"));
        assert!(generated.body.contains("## TM-2"));
        let third_model = generated.body.split("## TM-3").nth(1).unwrap();
        assert!(!third_model.contains("GAP: threat-without-treatment"));
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].gap_kind, GapKind::ThreatWithoutTreatment);
    }

    #[test]
    fn tc_1546_classified_sink_finding_appears_only_when_engine_reports_it() {
        let records = corpus(vec![
            document(
                "TM-1",
                "threat_model",
                json!({}),
                &[("SINK-1", "covers")],
                &[],
            ),
            document(
                "SINK-1",
                "data_sink",
                json!({"kind":"database"}),
                &[("CLASS-PHI", "carries")],
                &[],
            ),
            document(
                "CLASS-PHI",
                "data_classification",
                json!({"regulated_categories":["health"]}),
                &[],
                &[],
            ),
        ]);
        let denial = quoin_quire::model::FlowDenial {
            rule: "classified-sink".to_owned(),
            check: "trace:classified-sink-without-threat".to_owned(),
            subject: quoin_quire::model::FlowSubjectRef {
                id: "SINK-1".to_owned(),
                repo: None,
                archetype: "data_sink".to_owned(),
            },
            matched: BTreeMap::new(),
            remediations: Vec::new(),
            rationale: "Threat coverage is incomplete.".to_owned(),
            severity: "warning",
        };
        let generated = render_threat_model(&records, Some(&flow(vec![denial]))).unwrap();
        assert!(generated.body.contains("#### SINK-1"));
        assert!(
            generated
                .body
                .contains("GAP: classified-sink-without-threat SINK-1 (warning)")
        );
        assert_eq!(
            generated.gaps[0].gap_kind,
            GapKind::ClassifiedSinkWithoutThreat
        );
    }

    #[test]
    fn tc_1546_complete_output_is_deterministic_and_has_no_open_findings() {
        let records = corpus(vec![
            document(
                "TM-1",
                "threat_model",
                json!({}),
                &[("CONN-1", "covers")],
                &[],
            ),
            document("CONN-1", "connection", json!({}), &[], &[]),
        ]);
        let report = flow(Vec::new());
        let first = render_threat_model(&records, Some(&report)).unwrap();
        let second = render_threat_model(&records, Some(&report)).unwrap();
        assert_eq!(first, second);
        assert!(first.body.contains("### Open findings\n\nnone"));
        assert!(first.gaps.is_empty());
    }

    #[test]
    fn tc_1546_scope_and_crossing_rows_follow_dossier_order() {
        let records = corpus(vec![
            document(
                "TM-1",
                "threat_model",
                json!({}),
                &[
                    ("PART-2", "covers"),
                    ("BOUND-2", "covers"),
                    ("CONN-2", "covers"),
                    ("PART-1", "covers"),
                    ("CONN-1", "covers"),
                    ("BOUND-1", "covers"),
                ],
                &[],
            ),
            document("PART-2", "part", json!({}), &[], &[]),
            document("PART-1", "part", json!({}), &[], &[]),
            document(
                "BOUND-2",
                "trust_boundary",
                json!({"trust_level":"internal"}),
                &[],
                &[],
            ),
            document(
                "BOUND-1",
                "trust_boundary",
                json!({"trust_level":"external"}),
                &[],
                &[],
            ),
            document("CONN-2", "connection", json!({}), &[], &[]),
            document("CONN-1", "connection", json!({}), &[], &[]),
        ]);
        let output = render_threat_model(&records, None).unwrap().body;
        assert!(
            output.find("Covered parts: PART-1, PART-2").unwrap()
                < output
                    .find("Covered trust boundaries: BOUND-1, BOUND-2")
                    .unwrap()
        );
        assert!(output.find("Covered crossings").unwrap() < output.find("| CONN-1 |").unwrap());
        assert!(output.find("| CONN-1 |").unwrap() < output.find("| CONN-2 |").unwrap());
    }
}
