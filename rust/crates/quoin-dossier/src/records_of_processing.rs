// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The records-of-processing generator, using only authored activity fields and FR-096 facts.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use quoin_quire::model::{FlowFact, FlowFactEdge, FlowReport};
use quoin_quire::records::{Corpus, Document};
use serde_json::Value;

use crate::RecordsOfProcessingSource;
use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::markdown::{cell, unusable_input};
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};

/// Reads validated activities and the flow report from quire-rs.
pub(crate) struct RecordsOfProcessingGenerator(
    pub(crate) fn(&Inputs) -> Result<RecordsOfProcessingSource, SourceFailure>,
);

impl Generator for RecordsOfProcessingGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "records-of-processing",
            id: "DOS-ROP",
            title: "Records of processing",
            needs_as_of: false,
        }
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        let source = (self.0)(inputs)?;
        if let Some(failure) = unusable_input("records-of-processing records", &source.records) {
            return Err(failure);
        }
        Ok(render(&source.records, source.flow.as_ref()))
    }

    fn requires_version(&self) -> bool {
        false
    }

    fn supports_check(&self) -> bool {
        false
    }

    fn supports_export(&self) -> bool {
        false
    }
}

fn render(corpus: &Corpus, flow: Option<&FlowReport>) -> Generated {
    let activities = corpus
        .documents
        .iter()
        .filter(|document| document.kind == "processing_activity")
        .collect::<Vec<_>>();
    let mut activities = activities;
    activities.sort_by(|left, right| crate::order::dossier_cmp(&left.id, &right.id));
    let mut body = String::from("# Records of processing\n\n");
    let mut gaps = Vec::new();
    if activities.is_empty() {
        return Generated {
            body,
            document_version: None,
            gaps: vec![Gap::new(
                GapKind::NoProcessingActivity,
                "DOS-ROP",
                "no processing activity record exists in the selected scope",
            )],
        };
    }
    let facts = flow.map(facts_by_id).unwrap_or_default();
    let mut model_facts_unavailable = flow
        .is_none_or(|report| report.schema != "quire.flow-report/v1" || report.facts.is_empty());
    for activity in activities {
        model_facts_unavailable |= render_activity(activity, &facts, &mut body, &mut gaps);
    }
    if model_facts_unavailable {
        gaps.push(Gap::new(
            GapKind::ModelFactsUnavailable,
            "DOS-ROP",
            "required model facts are unavailable from quire.flow-report/v1",
        ));
    }
    Generated {
        body,
        document_version: None,
        gaps,
    }
}

fn render_activity(
    activity: &Document,
    facts: &BTreeMap<(Option<&str>, &str), (&str, &FlowFact)>,
    body: &mut String,
    gaps: &mut Vec<Gap>,
) -> bool {
    let activity_fact = facts
        .get(&(None, activity.id.as_str()))
        .map(|(_, fact)| *fact);
    let mut model_facts_unavailable = activity_fact.is_none();
    let title = value_text(field(activity, "title").unwrap_or(&Value::Null))
        .unwrap_or_else(|| activity.id.clone());
    let _ = writeln!(body, "## {} — {}\n", cell(&activity.id), cell(&title));
    body.push_str("| Entry | Content |\n| --- | --- |\n");

    let mut sources = BTreeSet::from([activity.id.clone()]);
    let covered = activity_fact
        .map(|fact| outgoing(fact, "covers"))
        .unwrap_or_default();
    let authored_covers = activity
        .edges
        .iter()
        .filter(|(_, relation)| relation == "covers")
        .map(|(target, _)| target.as_str())
        .collect::<BTreeSet<_>>();
    let reported_covers = covered
        .iter()
        .map(|edge| edge.target.id.as_str())
        .collect::<BTreeSet<_>>();
    if covered.iter().any(|edge| fact_for(edge, facts).is_none())
        || authored_covers != reported_covers
    {
        model_facts_unavailable = true;
    }

    let (connections, sinks, parts) = categorize_targets(&covered, facts, &mut sources);
    let mut expanded_sinks = sinks;
    for edge in parts.iter().flat_map(|part| outgoing(part, "delivers_to")) {
        if let Some(sink) = fact_for(edge, facts) {
            expanded_sinks.insert(sink.id.as_str(), sink);
        } else {
            model_facts_unavailable = true;
        }
    }
    let sinks = ordered_facts(expanded_sinks.into_values().collect());
    let connections = ordered_facts(connections.into_values().collect());
    let covered_elements = connections
        .iter()
        .chain(sinks.iter())
        .copied()
        .collect::<Vec<_>>();
    let (classification_content, missing_classification) =
        classifications(&covered_elements, &connections, &sinks, facts, &mut sources);
    if covered_elements.iter().any(|element| {
        element.edges.iter().any(|edge| {
            matches!(edge.edge.as_str(), "carries" | "delivers_to")
                && fact_for(edge, facts).is_none()
        })
    }) {
        model_facts_unavailable = true;
    }

    {
        let mut context = ActivityContext {
            facts,
            sources: &mut sources,
            body,
            gaps,
        };
        if field(activity, "role").and_then(Value::as_str) == Some("processor") {
            render_processor_activity(activity, &sinks, &covered_elements, &mut context);
        } else {
            render_controller_activity(
                activity,
                &sinks,
                &covered_elements,
                &classification_content,
                &missing_classification,
                &mut context,
            );
        }
    }
    append_sources(body, sources);
    model_facts_unavailable
}

struct ActivityContext<'facts, 'state> {
    facts: &'facts BTreeMap<(Option<&'facts str>, &'facts str), (&'facts str, &'facts FlowFact)>,
    sources: &'state mut BTreeSet<String>,
    body: &'state mut String,
    gaps: &'state mut Vec<Gap>,
}

fn render_processor_activity(
    activity: &Document,
    sinks: &[&FlowFact],
    covered_elements: &[&FlowFact],
    context: &mut ActivityContext<'_, '_>,
) {
    let own = field(activity, "processor")
        .and_then(Value::as_str)
        .unwrap_or("not stated");
    let controllers = field(activity, "controller")
        .map(values)
        .unwrap_or_default();
    let mut identity = vec![format!("Processor: {}", cell(own))];
    identity.extend(
        controllers
            .into_iter()
            .map(|controller| format!("Controller: {}", cell(&controller))),
    );
    append_authored_fields(&mut identity, activity, &["representative", "dpo_contact"]);
    row(
        context.body,
        "(a) Identity and contacts",
        &identity.join("; "),
    );

    let mut categories = Vec::new();
    let controllers = values(field(activity, "controller").unwrap_or(&Value::Null));
    for sink in sinks {
        context.sources.insert(sink.id.clone());
        if let Some(kind) = field_fact(sink, "kind").and_then(Value::as_str) {
            for controller in &controllers {
                categories.push(format!(
                    "{}: {} ({})",
                    cell(controller),
                    cell(kind),
                    cell(&sink.id)
                ));
            }
        } else {
            add_gap(
                context.gaps,
                GapKind::RecipientCategoriesUnstated,
                &sink.id,
                "kind is unstated",
            );
        }
    }
    let purpose_context = sinks
        .iter()
        .filter_map(|sink| {
            field_fact(sink, "purpose")
                .and_then(value_text)
                .map(|purpose| {
                    context.sources.insert(sink.id.clone());
                    format!("{}: {}", cell(&sink.id), cell(&purpose))
                })
        })
        .collect::<Vec<_>>();
    row(
        context.body,
        "(b) Categories of processing",
        &join_or_gap(&categories),
    );
    if !purpose_context.is_empty() {
        row(context.body, "Purpose context", &purpose_context.join("; "));
    }
    let (transfer, transfer_gaps) = transfer_entry(sinks, activity, context.sources);
    context.gaps.extend(transfer_gaps);
    row(
        context.body,
        "(c) Transfers to third countries and safeguards",
        &transfer,
    );
    let (security, security_gaps) =
        security_entry(covered_elements, context.facts, context.sources);
    let security = with_security_gaps(&security, &security_gaps);
    context.gaps.extend(security_gaps);
    row(
        context.body,
        "(d) Technical and organisational security measures",
        &security,
    );
}

fn render_controller_activity(
    activity: &Document,
    sinks: &[&FlowFact],
    covered_elements: &[&FlowFact],
    classification_content: &[String],
    missing_classification: &[(String, String)],
    context: &mut ActivityContext<'_, '_>,
) {
    let controller = field(activity, "controller")
        .and_then(value_text)
        .map_or_else(|| "not stated".to_owned(), |value| cell(&value));
    let mut identity = vec![format!("Controller: {controller}")];
    append_authored_fields(
        &mut identity,
        activity,
        &["joint_controllers", "representative", "dpo_contact"],
    );
    row(
        context.body,
        "(a) Identity and contacts",
        &identity.join("; "),
    );
    row(
        context.body,
        "(b) Purposes",
        &authored(activity, "purposes"),
    );
    row(
        context.body,
        "(c) Categories of data subjects",
        &authored(activity, "data_subject_categories"),
    );
    for (id, detail) in missing_classification {
        add_gap(
            context.gaps,
            GapKind::PersonalDataCategoriesUnstated,
            id,
            detail,
        );
        let _ = writeln!(
            context.body,
            "GAP: (c) categories of personal data — {} {}",
            cell(id),
            cell(detail)
        );
    }
    row(
        context.body,
        "(c) Categories of personal data",
        &join_or_gap(classification_content),
    );

    let mut recipients = Vec::new();
    for sink in sinks {
        context.sources.insert(sink.id.clone());
        let processor = field_fact(sink, "processor").and_then(value_text);
        let kind = field_fact(sink, "kind").and_then(value_text);
        let audience = field_fact(sink, "audience_roles")
            .map(values)
            .unwrap_or_default();
        if processor.is_none() || kind.is_none() || audience.is_empty() {
            add_gap(
                context.gaps,
                GapKind::RecipientCategoriesUnstated,
                &sink.id,
                "processor, kind or audience_roles is unstated",
            );
        }
        recipients.push(format!(
            "{}: processor {}; kind {}; audience {}",
            cell(&sink.id),
            processor.map_or_else(|| "not stated".to_owned(), |value| cell(&value)),
            kind.map_or_else(|| "not stated".to_owned(), |value| cell(&value)),
            join_or_gap(&audience)
        ));
    }
    row(
        context.body,
        "(d) Categories of recipients",
        &join_or_gap(&recipients),
    );
    let (transfer, transfer_gaps) = transfer_entry(sinks, activity, context.sources);
    context.gaps.extend(transfer_gaps);
    row(
        context.body,
        "(e) Transfers to third countries and safeguards",
        &transfer,
    );
    row(
        context.body,
        "(f) Time limits for erasure",
        &authored(activity, "retention"),
    );
    let (security, security_gaps) =
        security_entry(covered_elements, context.facts, context.sources);
    let security = with_security_gaps(&security, &security_gaps);
    context.gaps.extend(security_gaps);
    row(
        context.body,
        "(g) Technical and organisational security measures",
        &security,
    );
}

fn append_sources(body: &mut String, sources: BTreeSet<String>) {
    let mut source_ids = sources.into_iter().collect::<Vec<_>>();
    source_ids.sort_by(|left, right| crate::order::dossier_cmp(left, right));
    let content = if source_ids.is_empty() {
        "none".to_owned()
    } else {
        source_ids
            .iter()
            .map(|id| cell(id))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let _ = writeln!(body, "\nSources: {content}\n");
}

fn facts_by_id(flow: &FlowReport) -> BTreeMap<(Option<&str>, &str), (&str, &FlowFact)> {
    flow.facts
        .iter()
        .flat_map(|(kind, facts)| {
            facts.iter().map(move |fact| {
                (
                    (fact.repo.as_deref(), fact.id.as_str()),
                    (kind.as_str(), fact),
                )
            })
        })
        .collect()
}

fn categorize_targets<'a>(
    targets: &[&'a FlowFactEdge],
    facts: &BTreeMap<(Option<&str>, &str), (&str, &'a FlowFact)>,
    sources: &mut BTreeSet<String>,
) -> (
    BTreeMap<&'a str, &'a FlowFact>,
    BTreeMap<&'a str, &'a FlowFact>,
    Vec<&'a FlowFact>,
) {
    let mut connections = BTreeMap::new();
    let mut sinks = BTreeMap::new();
    let mut parts = Vec::new();
    for edge in targets {
        let Some(fact) = fact_for(edge, facts) else {
            continue;
        };
        sources.insert(fact.id.clone());
        match edge.archetype.as_str() {
            "connection" => {
                connections.insert(fact.id.as_str(), fact);
            }
            "data_sink" => {
                sinks.insert(fact.id.as_str(), fact);
            }
            "part" => parts.push(fact),
            _ => {}
        }
    }
    (connections, sinks, parts)
}

fn ordered_facts(mut facts: Vec<&FlowFact>) -> Vec<&FlowFact> {
    facts.sort_by(|left, right| crate::order::dossier_cmp(&left.id, &right.id));
    facts
}

fn fact_for<'a>(
    edge: &FlowFactEdge,
    facts: &BTreeMap<(Option<&str>, &str), (&str, &'a FlowFact)>,
) -> Option<&'a FlowFact> {
    facts
        .get(&(edge.target.repo.as_deref(), edge.target.id.as_str()))
        .map(|(_, fact)| *fact)
}

fn outgoing<'a>(fact: &'a FlowFact, edge: &str) -> Vec<&'a FlowFactEdge> {
    fact.edges
        .iter()
        .filter(|candidate| candidate.edge == edge && candidate.direction == "outgoing")
        .collect()
}

fn classifications<'a>(
    elements: &[&'a FlowFact],
    connections: &[&'a FlowFact],
    sinks: &[&'a FlowFact],
    facts: &BTreeMap<(Option<&str>, &str), (&str, &'a FlowFact)>,
    sources: &mut BTreeSet<String>,
) -> (Vec<String>, Vec<(String, String)>) {
    let sink_ids = sinks
        .iter()
        .map(|sink| (sink.repo.as_deref(), sink.id.as_str()))
        .collect::<BTreeSet<_>>();
    let mut carriers = elements.to_vec();
    let delivering = facts
        .values()
        .filter(|(kind, _)| *kind == "connection")
        .map(|(_, fact)| *fact)
        .filter(|connection| {
            outgoing(connection, "delivers_to")
                .iter()
                .any(|edge| {
                    fact_for(edge, facts).is_some_and(|target| {
                        sink_ids.contains(&(target.repo.as_deref(), target.id.as_str()))
                    })
                })
        })
        .collect::<Vec<_>>();
    for connection in &delivering {
        sources.insert(connection.id.clone());
    }
    carriers.extend(delivering.iter().copied());
    let mut classes = BTreeMap::new();
    let mut missing = Vec::new();
    for connection in connections {
        if outgoing(connection, "carries").is_empty() {
            missing.push((
                connection.id.clone(),
                "carries no classification".to_owned(),
            ));
        }
    }
    for carrier in carriers {
        let edges = outgoing(carrier, "carries");
        for edge in edges {
            if edge.archetype != "data_classification" {
                continue;
            }
            let Some(classification) = fact_for(edge, facts) else {
                continue;
            };
            sources.insert(classification.id.clone());
            let category = field_fact(classification, "regulated_categories")
                .map(values)
                .unwrap_or_default();
            if category.is_empty() {
                missing.push((
                    carrier.id.clone(),
                    format!(
                        "classification {} has no regulated_categories",
                        classification.id
                    ),
                ));
            }
            let title = field_fact(classification, "title").and_then(value_text);
            let label = if category.is_empty() {
                "not stated".to_owned()
            } else {
                category.join(", ")
            };
            let title = title
                .map(|title| format!(" — {}", cell(&title)))
                .unwrap_or_default();
            classes.insert(
                classification.id.as_str(),
                format!("{} ({}){}", cell(&classification.id), cell(&label), title),
            );
        }
    }
    for sink in sinks {
        let delivered_classification = delivering.iter().any(|connection| {
            let delivers_to_sink = outgoing(connection, "delivers_to")
                .iter()
                .any(|edge| {
                    fact_for(edge, facts).is_some_and(|target| {
                        target.id == sink.id && target.repo == sink.repo
                    })
                });
            delivers_to_sink
                && outgoing(connection, "carries")
                    .iter()
                    .any(|edge| edge.archetype == "data_classification")
        });
        if outgoing(sink, "carries").is_empty() && !delivered_classification {
            missing.push((sink.id.clone(), "carries no classification".to_owned()));
        }
    }
    missing.sort_by(|left, right| crate::order::dossier_cmp(&left.0, &right.0));
    missing.dedup();
    let mut classes = classes.into_iter().collect::<Vec<_>>();
    classes.sort_by(|left, right| crate::order::dossier_cmp(left.0, right.0));
    (
        classes.into_iter().map(|(_, value)| value).collect(),
        missing,
    )
}

fn transfer_entry(
    sinks: &[&FlowFact],
    activity: &Document,
    sources: &mut BTreeSet<String>,
) -> (String, Vec<Gap>) {
    let home = field(activity, "home_regions")
        .map(values)
        .unwrap_or_default();
    let mut facts = Vec::new();
    let mut gaps = Vec::new();
    for sink in sinks {
        sources.insert(sink.id.clone());
        let regions = field_fact(sink, "region").map(values).unwrap_or_default();
        let location = if regions.is_empty() {
            format!("{}: region not stated", cell(&sink.id))
        } else {
            let labels = regions
                .iter()
                .map(|region| {
                    format!(
                        "region {}{}",
                        cell(region),
                        if home.iter().any(|candidate| candidate == region) {
                            " (matches home region)"
                        } else {
                            " (outside exact home-region labels)"
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}: {labels}", cell(&sink.id))
        };
        facts.push(format!(
            "{location}; GAP: transfer region — {} is not classified as a third-country transfer; GAP: transfer safeguards — no transfer safeguard is identified for {}",
            cell(&sink.id),
            cell(&sink.id)
        ));
        gaps.push(
            Gap::new(
                GapKind::TransferRegionUnstated,
                &sink.id,
                "the model does not establish whether this sink is a third-country transfer",
            )
            .for_element(&sink.id),
        );
        gaps.push(
            Gap::new(
                GapKind::TransferSafeguardsUnstated,
                &sink.id,
                "the model does not identify transfer safeguards",
            )
            .for_element(&sink.id),
        );
    }
    (
        if facts.is_empty() {
            "no covered sinks".to_owned()
        } else {
            format!(
                "{}; transfer region and safeguards remain unstated",
                facts.join("; ")
            )
        },
        gaps,
    )
}

fn security_entry<'a>(
    elements: &[&'a FlowFact],
    facts: &BTreeMap<(Option<&str>, &str), (&str, &'a FlowFact)>,
    sources: &mut BTreeSet<String>,
) -> (String, Vec<Gap>) {
    let mut controls = BTreeMap::new();
    let mut gaps = Vec::new();
    for element in elements {
        sources.insert(element.id.clone());
        let mut found = false;
        for control in facts
            .values()
            .filter(|(kind, _)| *kind == "control")
            .map(|(_, fact)| *fact)
            .filter(|fact| {
                fact.edges.iter().any(|edge| {
                    edge.edge == "protects"
                        && edge.direction == "outgoing"
                        && edge.target.id == element.id
                })
            })
        {
            found = true;
            sources.insert(control.id.clone());
            if let Some(safeguards) = field_fact(control, "safeguards").and_then(value_text) {
                controls.insert(
                    control.id.as_str(),
                    format!("{} ({})", cell(&control.id), cell(&safeguards)),
                );
            }
        }
        if !found
            || !facts
                .values()
                .filter(|(kind, _)| *kind == "control")
                .map(|(_, fact)| *fact)
                .any(|fact| {
                    fact.edges.iter().any(|edge| {
                        edge.edge == "protects"
                            && edge.direction == "outgoing"
                            && edge.target.id == element.id
                            && field_fact(fact, "safeguards")
                                .and_then(value_text)
                                .is_some()
                    })
                })
        {
            gaps.push(
                Gap::new(
                    GapKind::SecurityMeasuresUnstated,
                    &element.id,
                    "no protecting control has a stated safeguards value",
                )
                .for_element(&element.id),
            );
        }
    }
    (
        if controls.is_empty() {
            "none stated".to_owned()
        } else {
            let mut controls = controls.into_iter().collect::<Vec<_>>();
            controls.sort_by(|left, right| crate::order::dossier_cmp(left.0, right.0));
            controls
                .into_iter()
                .map(|(_, value)| value)
                .collect::<Vec<_>>()
                .join(", ")
        },
        gaps,
    )
}

fn row(body: &mut String, entry: &str, content: &str) {
    let _ = writeln!(body, "| {} | {} |", cell(entry), content);
}
fn add_gap(gaps: &mut Vec<Gap>, kind: GapKind, subject: &str, detail: &str) {
    gaps.push(Gap::new(kind, subject, detail).for_element(subject));
}
fn join_or_gap(values: &[String]) -> String {
    if values.is_empty() {
        "not stated".to_owned()
    } else {
        values.join(", ")
    }
}
fn field<'a>(document: &'a Document, name: &str) -> Option<&'a Value> {
    document.frontmatter.get(name)
}
fn field_fact<'a>(fact: &'a FlowFact, name: &str) -> Option<&'a Value> {
    fact.fields.get(name)
}
fn values(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items.iter().filter_map(value_text).collect(),
        _ => value_text(value).into_iter().collect(),
    }
}
fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        Value::Array(items) => Some(
            items
                .iter()
                .filter_map(value_text)
                .collect::<Vec<_>>()
                .join(", "),
        ),
        Value::Object(_) | Value::Null => None,
    }
}
fn authored(document: &Document, name: &str) -> String {
    field(document, name)
        .map(values)
        .filter(|values| !values.is_empty())
        .map_or_else(
            || "not stated".to_owned(),
            |values| {
                values
                    .iter()
                    .map(|value| cell(value))
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        )
}

fn append_authored_fields(output: &mut Vec<String>, document: &Document, names: &[&str]) {
    for name in names {
        let values = field(document, name).map(values).unwrap_or_default();
        if !values.is_empty() {
            output.push(format!(
                "{name}: {}",
                values
                    .iter()
                    .map(|value| cell(value))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
}

fn with_security_gaps(content: &str, gaps: &[Gap]) -> String {
    let mut output = content.to_owned();
    for gap in gaps {
        let _ = write!(
            output,
            "; GAP: security measures — {} {}",
            cell(&gap.subject),
            cell(&gap.detail)
        );
    }
    output
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test panic is the failure report"
)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use serde_json::{Map, json};
    use std::path::PathBuf;

    fn document(id: &str, kind: &str, fields: &Value, edges: &[(&str, &str)]) -> Document {
        Document {
            path: PathBuf::from(format!("spec/{id}.md")),
            id: id.to_owned(),
            kind: kind.to_owned(),
            frontmatter: fields.as_object().cloned().unwrap_or_else(Map::new),
            edges: edges
                .iter()
                .map(|(target, relation)| ((*target).to_owned(), (*relation).to_owned()))
                .collect(),
            tables: BTreeMap::new(),
            sections: BTreeMap::new(),
            raw: String::new(),
            contract: quoin_quire::records::Contract::NotChecked,
        }
    }

    fn fact(id: &str, fields: &Value, edges: &[(&str, &str, &str)]) -> FlowFact {
        FlowFact {
            id: id.to_owned(),
            repo: None,
            path: format!("spec/{id}.md"),
            fields: fields
                .as_object()
                .map(|object| {
                    object
                        .iter()
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect()
                })
                .unwrap_or_default(),
            edges: edges
                .iter()
                .map(|(edge, direction, target)| FlowFactEdge {
                    edge: (*edge).to_owned(),
                    direction: match *direction {
                        "incoming" => "incoming",
                        _ => "outgoing",
                    },
                    target: quoin_quire::model::FlowDocumentRef {
                        id: (*target).to_owned(),
                        repo: None,
                    },
                    archetype: if target.starts_with("CONN-") {
                        "connection"
                    } else if target.starts_with("EXT-") || target.starts_with("SINK-") {
                        "data_sink"
                    } else if target.starts_with("CLASS-") {
                        "data_classification"
                    } else if target.starts_with("PART-") {
                        "part"
                    } else {
                        "control"
                    }
                    .to_owned(),
                })
                .collect(),
        }
    }

    fn flow(facts: Vec<(&str, FlowFact)>) -> FlowReport {
        FlowReport {
            schema: "quire.flow-report/v1",
            rules: Vec::new(),
            vocabularies: BTreeMap::new(),
            facts: facts
                .into_iter()
                .fold(BTreeMap::new(), |mut all, (kind, fact)| {
                    all.entry(kind.to_owned())
                        .or_insert_with(Vec::new)
                        .push(fact);
                    all
                }),
            denials: Vec::new(),
            remediations: Vec::new(),
        }
    }

    fn controller_flow() -> FlowReport {
        flow(vec![
            (
                "processing_activity",
                fact(
                    "ACT-1",
                    &json!({}),
                    &[
                        ("covers", "outgoing", "CONN-1"),
                        ("covers", "outgoing", "EXT-LAB"),
                        ("covers", "outgoing", "EXT-EEA"),
                    ],
                ),
            ),
            (
                "connection",
                fact(
                    "CONN-1",
                    &json!({}),
                    &[("carries", "outgoing", "CLASS-PHI")],
                ),
            ),
            (
                "data_sink",
                fact(
                    "EXT-LAB",
                    &json!({"processor":"External Lab", "kind":"laboratory", "audience_roles":["lab"], "region":["us", "eu"]}),
                    &[],
                ),
            ),
            (
                "data_sink",
                fact(
                    "EXT-EEA",
                    &json!({"processor":"EEA Archive", "kind":"archive", "audience_roles":["archive"], "region":["eu", "eea"]}),
                    &[],
                ),
            ),
            (
                "data_classification",
                fact(
                    "CLASS-PHI",
                    &json!({"title":"Health data", "regulated_categories":["health"]}),
                    &[],
                ),
            ),
            (
                "control",
                fact(
                    "CTRL-1",
                    &json!({"safeguards":"encryption"}),
                    &[
                        ("protects", "outgoing", "EXT-LAB"),
                        ("protects", "outgoing", "EXT-EEA"),
                    ],
                ),
            ),
        ])
    }

    fn corpus(documents: Vec<Document>) -> Corpus {
        Corpus {
            documents,
            unreadable: Vec::new(),
            malformed: Vec::new(),
        }
    }

    #[trace("TC-617", "FR-207-AC-1")]
    #[test]
    fn flow_fact_lookup_matches_repository_and_artifact_id() {
        let mut selected = fact("EXT-SHARED", &json!({"processor":"Selected"}), &[]);
        selected.repo = Some("selected/repo".to_owned());
        let mut other = fact("EXT-SHARED", &json!({"processor":"Other"}), &[]);
        other.repo = Some("other/repo".to_owned());
        let flow = flow(vec![("data_sink", selected), ("data_sink", other)]);
        let facts = facts_by_id(&flow);
        let mut edge = fact("ACT-1", &json!({}), &[("covers", "outgoing", "EXT-SHARED")])
            .edges
            .remove(0);
        edge.target.repo = Some("selected/repo".to_owned());

        assert_eq!(
            fact_for(&edge, &facts).and_then(|sink| field_fact(sink, "processor")),
            Some(&json!("Selected")),
            "the target repository must select its matching duplicate artifact id"
        );
    }

    #[allow(
        clippy::unnecessary_wraps,
        reason = "source providers use the fallible generator contract"
    )]
    fn empty_source(_inputs: &Inputs) -> Result<RecordsOfProcessingSource, SourceFailure> {
        Ok(RecordsOfProcessingSource {
            records: corpus(Vec::new()),
            flow: None,
        })
    }

    fn invalid_activity_source(reason: &str) -> RecordsOfProcessingSource {
        RecordsOfProcessingSource {
            records: corpus(vec![Document {
                path: PathBuf::from("spec/ACT-INVALID.md"),
                id: "ACT-INVALID".to_owned(),
                kind: "processing_activity".to_owned(),
                frontmatter: Map::new(),
                edges: Vec::new(),
                tables: BTreeMap::new(),
                sections: BTreeMap::new(),
                raw: String::new(),
                contract: quoin_quire::records::Contract::Violated(reason.to_owned()),
            }]),
            flow: None,
        }
    }

    #[allow(
        clippy::unnecessary_wraps,
        reason = "source providers use the fallible generator contract"
    )]
    fn invalid_retention_source(
        _inputs: &Inputs,
    ) -> Result<RecordsOfProcessingSource, SourceFailure> {
        Ok(invalid_activity_source(
            "retention is required for controller activity",
        ))
    }

    #[allow(
        clippy::unnecessary_wraps,
        reason = "source providers use the fallible generator contract"
    )]
    fn invalid_queue_source(_inputs: &Inputs) -> Result<RecordsOfProcessingSource, SourceFailure> {
        Ok(invalid_activity_source(
            "covers target queue has wrong kind",
        ))
    }

    #[allow(
        clippy::unnecessary_wraps,
        reason = "source providers use the fallible generator contract"
    )]
    fn invalid_dangling_source(
        _inputs: &Inputs,
    ) -> Result<RecordsOfProcessingSource, SourceFailure> {
        Ok(invalid_activity_source("covers target is dangling"))
    }

    #[trace("TC-618", "FR-207-AC-2")]
    #[test]
    fn tc_1658_unusable_activity_fields_fail_before_generation() {
        let generator = RecordsOfProcessingGenerator(invalid_retention_source);
        let failure = generator.generate(&crate::testing::inputs()).unwrap_err();
        assert!(failure.detail.contains("retention is required"));
    }

    #[trace("TC-622", "FR-207-AC-6")]
    #[test]
    fn tc_1658_invalid_covers_edges_fail_before_generation() {
        for source in [invalid_queue_source, invalid_dangling_source] {
            let generator = RecordsOfProcessingGenerator(source);
            let failure = generator.generate(&crate::testing::inputs()).unwrap_err();
            assert!(failure.detail.contains("covers target"));
        }
    }

    #[trace("TC-617", "FR-207-AC-1")]
    #[trace("TC-620", "FR-207-AC-4")]
    #[trace("TC-624", "FR-207-AC-8")]
    #[test]
    fn tc_1658_controller_lists_classification_recipients_transfer_gaps_and_safeguards() {
        let records = corpus(vec![document(
            "ACT-1",
            "processing_activity",
            &json!({"title":"Health processing", "role":"controller", "controller":"Example Controller", "purposes":["care"], "data_subject_categories":["patients"], "retention":"7 years", "home_regions":["eu", "eea"]}),
            &[],
        )]);
        let flow = controller_flow();
        let generated = render(&records, Some(&flow));
        assert!(generated.body.contains("CLASS-PHI (health) — Health data"));
        assert!(
            generated
                .body
                .contains("EXT-LAB: processor External Lab; kind laboratory; audience lab")
        );
        assert!(generated.body.contains(
            "EXT-LAB: region us (outside exact home-region labels), region eu (matches home region)"
        ));
        assert!(generated.body.contains(
            "EXT-EEA: region eu (matches home region), region eea (matches home region)"
        ));
        assert!(generated.body.contains("CTRL-1 (encryption)"));
        assert!(
            generated
                .body
                .contains("(e) Transfers to third countries and safeguards")
        );
        assert_eq!(
            generated
                .gaps
                .iter()
                .filter(|gap| gap.gap_kind == GapKind::TransferRegionUnstated)
                .count(),
            2
        );
        assert_eq!(generated, render(&records, Some(&flow)));
        let repeated = render(&records, Some(&flow));
        let json_output = |generated: &Generated| {
            serde_json::to_vec(&json!({"body": generated.body, "gaps": generated.gaps})).unwrap()
        };
        assert_eq!(json_output(&generated), json_output(&repeated));
        assert_eq!(
            generated
                .gaps
                .iter()
                .filter(|gap| gap.gap_kind == GapKind::TransferSafeguardsUnstated)
                .count(),
            2
        );
    }

    #[trace("TC-619", "FR-207-AC-3")]
    #[test]
    fn tc_1658_unclassified_connection_emits_gap_row_and_gap_record() {
        let records = corpus(vec![document(
            "ACT-1",
            "processing_activity",
            &json!({"title":"Activity", "role":"controller"}),
            &[],
        )]);
        let flow = flow(vec![
            (
                "processing_activity",
                fact("ACT-1", &json!({}), &[("covers", "outgoing", "CONN-7")]),
            ),
            ("connection", fact("CONN-7", &json!({}), &[])),
        ]);
        let generated = render(&records, Some(&flow));
        assert!(
            generated.body.contains(
                "GAP: (c) categories of personal data — CONN-7 carries no classification"
            )
        );
        assert_eq!(
            generated
                .gaps
                .iter()
                .filter(
                    |gap| gap.gap_kind == GapKind::PersonalDataCategoriesUnstated
                        && gap.subject == "CONN-7"
                )
                .count(),
            1
        );
    }

    #[trace("FR-207-AC-3")]
    #[test]
    fn delivered_classification_only_suppresses_gap_for_its_sink() {
        let records = corpus(vec![document(
            "ACT-1",
            "processing_activity",
            &json!({"title":"Activity", "role":"controller"}),
            &[],
        )]);
        let flow = flow(vec![
            (
                "processing_activity",
                fact(
                    "ACT-1",
                    &json!({}),
                    &[
                        ("covers", "outgoing", "SINK-A"),
                        ("covers", "outgoing", "SINK-B"),
                    ],
                ),
            ),
            (
                "connection",
                fact(
                    "CONN-A",
                    &json!({}),
                    &[
                        ("delivers_to", "outgoing", "SINK-A"),
                        ("carries", "outgoing", "CLASS-A"),
                    ],
                ),
            ),
            ("data_sink", fact("SINK-A", &json!({}), &[])),
            ("data_sink", fact("SINK-B", &json!({}), &[])),
            (
                "data_classification",
                fact(
                    "CLASS-A",
                    &json!({"title":"Health data", "regulated_categories":["health"]}),
                    &[],
                ),
            ),
        ]);

        let generated = render(&records, Some(&flow));

        assert!(
            generated.body.contains("GAP: (c) categories of personal data — SINK-B carries no classification"),
            "the unrelated sink must retain its missing-category gap"
        );
        assert!(
            generated.gaps.iter().any(|gap| {
                gap.gap_kind == GapKind::PersonalDataCategoriesUnstated && gap.subject == "SINK-B"
            }),
            "the unrelated sink gap must appear in the structured report"
        );
        assert!(!generated.gaps.iter().any(|gap| {
            gap.gap_kind == GapKind::PersonalDataCategoriesUnstated && gap.subject == "SINK-A"
        }));
    }

    #[trace("TC-618", "FR-207-AC-2")]
    #[trace("TC-621", "FR-207-AC-5")]
    #[test]
    fn tc_1658_processor_uses_article_30_2_rows_without_controller_only_entries() {
        let records = corpus(vec![document(
            "ACT-2",
            "processing_activity",
            &json!({"title":"Processor work", "role":"processor", "processor":"Processor Co; contact", "controller":["Controller One; contact"], "home_regions":["eu"]}),
            &[],
        )]);
        let flow = flow(vec![
            (
                "processing_activity",
                fact("ACT-2", &json!({}), &[("covers", "outgoing", "SINK-2")]),
            ),
            (
                "data_sink",
                fact(
                    "SINK-2",
                    &json!({"kind":"analytics", "purpose":"usage metrics"}),
                    &[],
                ),
            ),
        ]);
        let generated = render(&records, Some(&flow));
        assert!(generated.body.contains("(b) Categories of processing"));
        assert!(generated.body.contains("analytics (SINK-2)"));
        assert!(generated.body.contains("Purpose context"));
        assert!(!generated.body.contains("Categories of data subjects"));
        assert!(!generated.body.contains("Time limits for erasure"));
        assert!(!generated.body.contains("(f)"));
    }

    #[trace("TC-623", "FR-207-AC-7")]
    #[test]
    fn tc_1658_missing_flow_report_is_an_explicit_document_gap() {
        let records = corpus(vec![document(
            "ACT-1",
            "processing_activity",
            &json!({"title":"Activity"}),
            &[],
        )]);
        let generated = render(&records, None);
        assert!(
            generated
                .gaps
                .iter()
                .any(|gap| gap.gap_kind == GapKind::ModelFactsUnavailable
                    && gap.subject == "DOS-ROP")
        );
    }

    #[trace("TC-624", "FR-207-AC-8")]
    #[test]
    fn tc_1658_no_activity_has_a_document_gap_and_no_activity_section() {
        let generated = render(&corpus(Vec::new()), None);
        assert!(generated.gaps.iter().any(|gap| {
            gap.gap_kind == GapKind::NoProcessingActivity && gap.subject == "DOS-ROP"
        }));
        assert!(!generated.body.contains("## "));
    }

    #[trace("FR-191-AC-13", "FR-191-AC-24")]
    #[test]
    fn records_of_processing_generates_without_version_and_is_generate_only() {
        let generator = RecordsOfProcessingGenerator(empty_source);
        let mut registry = crate::Registry::new();
        registry.register(Box::new(generator)).unwrap();
        let mut inputs = crate::testing::inputs();
        inputs.version.clear();
        let work = tempfile::tempdir().unwrap();
        let kinds = ["records-of-processing".to_owned()];

        crate::generate(&registry, &kinds, work.path(), &inputs).unwrap();
        let generated = std::fs::read_to_string(work.path().join("DOS-ROP.md")).unwrap();
        assert!(!generated.lines().any(|line| line.starts_with("version:")));
        assert!(
            crate::check(&registry, &kinds, work.path(), &inputs)
                .unwrap_err()
                .message()
                .contains("does not support check")
        );
        let options = crate::ExportOptions {
            to: "pdf".to_owned(),
            output: work.path().join("out.pdf"),
            release: None,
            approvals: Vec::new(),
            modules: Vec::new(),
            template: None,
            quire: PathBuf::from("quire"),
        };
        assert!(
            crate::export(&registry, &kinds, work.path(), &inputs, &options)
                .unwrap_err()
                .message()
                .contains("does not support export")
        );

        let mut mixed = crate::Registry::new();
        mixed
            .register(Box::new(RecordsOfProcessingGenerator(empty_source)))
            .unwrap();
        mixed
            .register(Box::new(crate::testing::doc("srs", "DOS-SRS", false)))
            .unwrap();
        let inputs = crate::testing::inputs();
        let checked = crate::check(&mixed, &[], work.path(), &inputs).unwrap();
        assert_eq!(
            checked
                .documents
                .iter()
                .map(|document| document.id.as_str())
                .collect::<Vec<_>>(),
            ["DOS-SRS"]
        );
        let exported = crate::export(&mixed, &[], work.path(), &inputs, &options).unwrap();
        assert_eq!(
            exported
                .documents
                .iter()
                .map(|document| document.id.as_str())
                .collect::<Vec<_>>(),
            ["DOS-SRS"]
        );
    }
}
