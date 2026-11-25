// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Deterministic GSN case export and its well-formedness checks.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{AssuranceCase, CaseNode, NodeKind, NodeStatus};

/// GSN export wire format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GsnCase {
    /// Format discriminator.
    pub format: String,
    /// Nodes in deterministic depth-first order.
    pub nodes: Vec<GsnNode>,
    /// Typed relationships.
    pub edges: Vec<GsnEdge>,
    /// Root node ids.
    pub roots: Vec<String>,
    /// Unreachable requirement ids.
    pub unreachable: Vec<String>,
    /// Source facts that this format does not represent.
    pub unmapped: Vec<Unmapped>,
}

/// A typed GSN node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GsnNode {
    /// Generated deterministic id.
    pub id: String,
    /// GSN dialect node kind.
    pub kind: String,
    /// Source Quoin id.
    pub source: String,
    /// Source statement.
    pub statement: String,
    /// Supported or open.
    pub status: String,
    /// Reason the node remains open.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub open_reason: Option<String>,
    /// True for an undeveloped goal.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub undeveloped: Option<bool>,
    /// Evidence for a solution node.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub evidence: Option<serde_json::Value>,
    /// Additional source-authored fields retained by the dialect.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_data: Option<serde_json::Value>,
}

/// A directed GSN relationship.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct GsnEdge {
    /// Relationship kind.
    pub kind: String,
    /// Source node id.
    pub from: String,
    /// Target node id.
    pub to: String,
}

/// A source item that cannot be represented.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unmapped {
    /// Source identifier.
    pub source: String,
    /// Why it is unmapped.
    pub reason: String,
}

/// Convert a derived assurance case to deterministic GSN nodes and edges.
pub fn from_case(case: &AssuranceCase) -> Result<GsnCase, String> {
    let mut builder = Builder {
        nodes: Vec::new(),
        edges: Vec::new(),
        ids: BTreeMap::new(),
        counters: BTreeMap::new(),
    };
    let roots = case
        .claims
        .iter()
        .map(|node| builder.add(node))
        .collect::<Result<Vec<_>, _>>()?;
    let trust = serde_json::to_value(&case.producer_trust).map_err(|error| error.to_string())?;
    append_global_context(&mut builder, &roots, trust, "producer trust")?;
    let independence =
        serde_json::to_value(&case.evidence_independence).map_err(|error| error.to_string())?;
    append_global_context(&mut builder, &roots, independence, "evidence independence")?;
    let model = GsnCase {
        format: "quoin.gsn-case/v1".to_owned(),
        nodes: builder.nodes,
        edges: builder.edges,
        roots,
        unreachable: case.unreachable.clone(),
        unmapped: case
            .unreadable
            .iter()
            .map(|item| Unmapped {
                source: item.path.clone(),
                reason: format!("source document could not be read: {}", item.reason),
            })
            .chain(case.formalization_records.iter().map(|item| Unmapped {
                source: item.obligation_id.clone(),
                reason: "formalization record is outside the declared GSN mapping".to_owned(),
            }))
            .collect(),
    };
    validate(&model)?;
    Ok(model)
}

/// Convert a derived case and attach source-owned TVK facts for exact retained
/// run matches. A missing report never causes tool identity to be guessed.
pub fn from_case_with_validation_report(
    case: &AssuranceCase,
    run_records: &[serde_json::Value],
    report: Option<&serde_json::Value>,
) -> Result<GsnCase, String> {
    let mut model = from_case(case)?;
    let mut strategy_index = next_index(&model.nodes, "S");
    let mut goal_index = next_index(&model.nodes, "G");
    let mut context_index = next_index(&model.nodes, "C");
    let solution_sources = model
        .nodes
        .iter()
        .filter(|node| node.kind == "solution")
        .map(|node| node.source.clone())
        .collect::<Vec<_>>();
    for source in solution_sources {
        let Some(case_node) = find_case_node(&case.claims, &source) else {
            continue;
        };
        let Some(evidence) = case_node.evidence.as_ref() else {
            continue;
        };
        for suite in &evidence.suites {
            let Some(at) = suite.at.as_deref() else {
                continue;
            };
            let matching_run = run_records.iter().find(|record| {
                record.get("suite").and_then(serde_json::Value::as_str) == Some(&suite.suite)
                    && record.get("timestamp").and_then(serde_json::Value::as_str) == Some(at)
            });
            let Some(run) = matching_run else {
                continue;
            };
            let Some(tool) = run.get("tool").and_then(serde_json::Value::as_str) else {
                continue;
            };
            let tool_row = report
                .and_then(|value| value.get("tools"))
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .find(|row| row.get("tool").and_then(serde_json::Value::as_str) == Some(tool));
            let package_identity = tool_row
                .and_then(|row| row.get("packageIdentity"))
                .and_then(serde_json::Value::as_str);
            let displayed_identity = package_identity.unwrap_or(tool);
            let parent = model
                .edges
                .iter()
                .find(|edge| {
                    edge.kind == "supported_by"
                        && edge.to
                            == model
                                .nodes
                                .iter()
                                .find(|node| node.source == source)
                                .map(|node| node.id.clone())
                                .unwrap_or_default()
                })
                .map(|edge| edge.from.clone());
            let Some(parent) = parent else {
                continue;
            };
            let strategy_id = format!("S{strategy_index}");
            strategy_index += 1;
            model.nodes.push(GsnNode {
                id: strategy_id.clone(),
                kind: "strategy".to_owned(),
                source: format!("tool-confidence:{source}:{displayed_identity}"),
                statement: format!("Tool evidence for {displayed_identity}"),
                status: "open".to_owned(),
                open_reason: None,
                undeveloped: None,
                evidence: None,
                source_data: Some(serde_json::json!({"producerRun":run})),
            });
            model.edges.push(GsnEdge {
                kind: "supported_by".to_owned(),
                from: parent,
                to: strategy_id.clone(),
            });
            let mut gaps = Vec::new();
            let mut add_fact = |id: &str, statement: String, fact: serde_json::Value| {
                let context_id = format!("C{context_index}");
                context_index += 1;
                model.nodes.push(GsnNode {
                    id: context_id.clone(),
                    kind: "context".to_owned(),
                    source: id.to_owned(),
                    statement,
                    status: "supported".to_owned(),
                    open_reason: None,
                    undeveloped: None,
                    evidence: None,
                    source_data: Some(fact),
                });
                model.edges.push(GsnEdge {
                    kind: "in_context_of".to_owned(),
                    from: strategy_id.clone(),
                    to: context_id,
                });
            };
            match tool_row {
                None => gaps.push("validation-report tool entry not provided".to_owned()),
                Some(row) => {
                    if package_identity.is_none() {
                        gaps.push("tool package identity not provided".to_owned());
                    }
                    if let Some(intended_use) = row.get("intendedUse") {
                        let id = intended_use
                            .get("id")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("IntendedUse");
                        let statement = serde_json::to_string(intended_use)
                            .map_err(|error| error.to_string())?;
                        add_fact(id, statement, intended_use.clone());
                    } else {
                        gaps.push("IntendedUse record not provided".to_owned());
                    }
                    if let Some(validation_id) = row
                        .get("validationReportId")
                        .and_then(serde_json::Value::as_str)
                    {
                        add_fact(
                            validation_id,
                            "retained validation report".to_owned(),
                            row.clone(),
                        );
                    } else {
                        gaps.push("validation report reference not provided".to_owned());
                    }
                    let self_tests = row
                        .get("selfTestRuns")
                        .and_then(serde_json::Value::as_array);
                    if let Some(self_tests) = self_tests.filter(|items| !items.is_empty()) {
                        for self_test in self_tests {
                            let id = self_test
                                .get("id")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("self-test run");
                            let statement = serde_json::to_string(self_test)
                                .map_err(|error| error.to_string())?;
                            add_fact(id, statement, self_test.clone());
                        }
                    } else {
                        gaps.push("retained self-test run not provided".to_owned());
                    }
                    if let Some(anomalies) =
                        row.get("anomalies").and_then(serde_json::Value::as_array)
                    {
                        if anomalies.is_empty() {
                            add_fact(
                                "anomaly-inventory",
                                "none known".to_owned(),
                                serde_json::json!({"anomalies": []}),
                            );
                        }
                        for anomaly in anomalies {
                            let id = anomaly
                                .get("id")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("Anomaly");
                            let statement = serde_json::to_string(anomaly)
                                .map_err(|error| error.to_string())?;
                            add_fact(id, statement, anomaly.clone());
                        }
                    } else {
                        gaps.push("Anomaly inventory not provided".to_owned());
                    }
                }
            }
            if !gaps.is_empty() {
                let gap_id = format!("G{goal_index}");
                goal_index += 1;
                let reason = gaps.join("; ");
                model.nodes.push(GsnNode {
                    id: gap_id.clone(),
                    kind: "goal".to_owned(),
                    source: format!("tool-confidence-gap:{source}"),
                    statement: "Required tool-confidence source facts are incomplete".to_owned(),
                    status: "open".to_owned(),
                    open_reason: Some(reason),
                    undeveloped: Some(true),
                    evidence: None,
                    source_data: None,
                });
                model.edges.push(GsnEdge {
                    kind: "supported_by".to_owned(),
                    from: strategy_id,
                    to: gap_id,
                });
            } else {
                // Source facts describe the tool; they never become a confidence verdict.
                if let Some(node) = model.nodes.iter_mut().find(|node| node.id == strategy_id) {
                    node.undeveloped = Some(true);
                }
            }
        }
    }
    validate(&model)?;
    Ok(model)
}

fn next_index(nodes: &[GsnNode], prefix: &str) -> usize {
    nodes
        .iter()
        .filter_map(|node| node.id.strip_prefix(prefix)?.parse::<usize>().ok())
        .max()
        .unwrap_or(0)
        + 1
}

fn find_case_node<'a>(nodes: &'a [CaseNode], source: &str) -> Option<&'a CaseNode> {
    for node in nodes {
        if node.id == source {
            return Some(node);
        }
        if let Some(found) =
            find_case_node(&node.children, source).or_else(|| find_case_node(&node.context, source))
        {
            return Some(found);
        }
    }
    None
}

fn append_global_context(
    builder: &mut Builder,
    roots: &[String],
    source: serde_json::Value,
    label: &str,
) -> Result<(), String> {
    let records = match source {
        serde_json::Value::Array(values) => values,
        serde_json::Value::Null => Vec::new(),
        value => vec![value],
    };
    for (index, record) in records.into_iter().enumerate() {
        let id = record
            .get("id")
            .or_else(|| record.get("obligation"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{label} {}", index + 1));
        let statement = serde_json::to_string(&record).map_err(|error| error.to_string())?;
        let node = builder.push(
            "context",
            "C",
            &id,
            statement,
            "supported",
            None,
            None,
            Some(record),
        );
        for root in roots {
            builder.edges.push(GsnEdge {
                kind: "in_context_of".to_owned(),
                from: root.clone(),
                to: node.clone(),
            });
        }
    }
    Ok(())
}

struct Builder {
    nodes: Vec<GsnNode>,
    edges: Vec<GsnEdge>,
    ids: BTreeMap<String, String>,
    counters: BTreeMap<String, usize>,
}

impl Builder {
    fn add(&mut self, node: &CaseNode) -> Result<String, String> {
        if matches!(
            node.kind,
            NodeKind::Context
                | NodeKind::Assumption
                | NodeKind::Justification
                | NodeKind::CounterClaim
        ) && (!node.children.is_empty()
            || node.evidence.is_some()
            || node.analysis_evidence.is_some()
            || !node.context.is_empty())
        {
            return Err(format!(
                "terminal GSN node {} contains children, evidence, or nested context",
                node.id
            ));
        }
        if let Some(existing) = self.ids.get(&node.id) {
            return Ok(existing.clone());
        }
        let (kind, prefix) = match node.kind {
            NodeKind::Goal => ("goal", "G"),
            NodeKind::Strategy => ("strategy", "S"),
            NodeKind::Solution if node.status == NodeStatus::Open => ("goal", "G"),
            NodeKind::Solution => ("solution", "Sn"),
            NodeKind::Context => ("context", "C"),
            NodeKind::Assumption => ("assumption", "A"),
            NodeKind::Justification => ("justification", "J"),
            NodeKind::CounterClaim => ("counter_claim", "CC"),
        };
        let counter = self.counters.entry(prefix.to_owned()).or_default();
        *counter += 1;
        let generated = format!("{prefix}{counter}");
        self.ids.insert(node.id.clone(), generated.clone());
        let undeveloped = (matches!(node.kind, NodeKind::Goal | NodeKind::Strategy)
            || (node.kind == NodeKind::Solution && node.status == NodeStatus::Open))
            && node.children.is_empty();
        self.nodes.push(GsnNode {
            id: generated.clone(),
            kind: kind.to_owned(),
            source: node.id.clone(),
            statement: node.statement.clone(),
            status: if node.status == NodeStatus::Open {
                "open"
            } else {
                "supported"
            }
            .to_owned(),
            open_reason: node.because.clone(),
            undeveloped: undeveloped.then_some(true),
            evidence: if node.kind == NodeKind::Solution && node.status == NodeStatus::Supported {
                node.evidence
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|error| error.to_string())?
            } else {
                None
            },
            source_data: None,
        });
        for child in &node.children {
            let edge_position = self.edges.len();
            let child_id = self.add(child)?;
            self.edges.insert(
                edge_position,
                GsnEdge {
                    kind: "supported_by".to_owned(),
                    from: generated.clone(),
                    to: child_id,
                },
            );
        }
        for context in &node.context {
            let edge_position = self.edges.len();
            let context_id = self.add(context)?;
            self.edges.insert(
                edge_position,
                GsnEdge {
                    kind: "in_context_of".to_owned(),
                    from: generated.clone(),
                    to: context_id,
                },
            );
        }
        Ok(generated)
    }
}

/// Convert the authored view returned by FR-047 to GSN.
pub fn from_authored(value: &serde_json::Value) -> Result<GsnCase, String> {
    let mut b = Builder {
        nodes: Vec::new(),
        edges: Vec::new(),
        ids: BTreeMap::new(),
        counters: BTreeMap::new(),
    };
    let top = value
        .get("topClaim")
        .ok_or("authored view is missing topClaim")?;
    let root_source = str_field(top, "id")?;
    let reasoning_rows = value
        .get("reasoning")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let root = b.push(
        "goal",
        "G",
        root_source,
        str_field(top, "statement")?,
        status(top),
        top.get("reasons")
            .and_then(serde_json::Value::as_array)
            .and_then(|v| v.first())
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        reasoning_rows.is_empty().then_some(true),
        Some(top.clone()),
    );
    for evidence_ref in top
        .get("evidenceRefs")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let reference = str_field(evidence_ref, "reference")?;
        let state = evidence_ref
            .get("state")
            .and_then(serde_json::Value::as_str);
        let reason = evidence_ref
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let supported = state == Some("resolved");
        let solution = b.push(
            "solution",
            "Sn",
            reference,
            reference,
            if supported { "supported" } else { "open" },
            reason,
            None,
            Some(evidence_ref.clone()),
        );
        if let Some(node) = b.nodes.iter_mut().find(|node| node.id == solution) {
            node.evidence = Some(serde_json::json!({"references": [reference]}));
        }
        b.edges.push(GsnEdge {
            kind: "supported_by".to_owned(),
            from: root.clone(),
            to: solution,
        });
    }
    let mut strategy_ids = BTreeMap::new();
    let mut unmapped_extra = Vec::new();
    for reasoning in &reasoning_rows {
        let source = str_field(reasoning, "id")?;
        let criteria_rows = reasoning
            .get("criteria")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let strategy = b.push(
            "strategy",
            "S",
            source,
            str_field(reasoning, "statement")?,
            status(reasoning),
            criteria_rows
                .iter()
                .find_map(|c| c.get("reason").and_then(serde_json::Value::as_str))
                .map(str::to_owned),
            criteria_rows.is_empty().then_some(true),
            Some(reasoning.clone()),
        );
        strategy_ids.insert(source.to_owned(), strategy.clone());
    }
    for reasoning in &reasoning_rows {
        let source = str_field(reasoning, "id")?;
        let strategy = strategy_ids.get(source).ok_or("missing strategy node")?;
        let target = str_field(reasoning, "supports")?;
        let target_id = if target == root_source {
            root.clone()
        } else if let Some(id) = strategy_ids.get(target) {
            id.clone()
        } else {
            unmapped_extra.push(Unmapped {
                source: source.to_owned(),
                reason: format!("supports target {target} has no GSN node"),
            });
            continue;
        };
        b.edges.push(GsnEdge {
            kind: "supported_by".to_owned(),
            from: target_id,
            to: strategy.clone(),
        });
        let criteria_rows = reasoning
            .get("criteria")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        for criterion in &criteria_rows {
            let criterion_text = str_field(criterion, "criterion")?;
            let source = format!("{}:{}", source, criterion_text);
            let reason = criterion
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned);
            let has_evidence = criterion
                .get("decision")
                .and_then(|d| d.get("evidenceRefs"))
                .and_then(serde_json::Value::as_array)
                .is_some_and(|refs| !refs.is_empty());
            let goal = b.push(
                "goal",
                "G",
                &source,
                criterion_text,
                status(criterion),
                reason,
                (!has_evidence).then_some(true),
                Some(criterion.clone()),
            );
            b.edges.push(GsnEdge {
                kind: "supported_by".to_owned(),
                from: strategy.clone(),
                to: goal.clone(),
            });
            if let Some(decision) = criterion.get("decision") {
                for reference in decision
                    .get("evidenceRefs")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                {
                    let evidence = b.push(
                        "solution",
                        "Sn",
                        reference,
                        reference,
                        "supported",
                        None,
                        None,
                        Some(serde_json::json!({"evidenceRef": reference, "decision": decision})),
                    );
                    if let Some(node) = b.nodes.iter_mut().find(|node| node.id == evidence) {
                        node.evidence = Some(serde_json::json!({"references": [reference]}));
                    }
                    b.edges.push(GsnEdge {
                        kind: "supported_by".to_owned(),
                        from: goal.clone(),
                        to: evidence,
                    });
                }
            }
        }
    }
    for assumption in value
        .get("assumptions")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = str_field(assumption, "id")?;
        let node = b.push(
            "assumption",
            "A",
            id,
            str_field(assumption, "statement")?,
            status(assumption),
            assumption
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            None,
            Some(assumption.clone()),
        );
        b.edges.push(GsnEdge {
            kind: "in_context_of".to_owned(),
            from: root.clone(),
            to: node,
        });
    }
    for participant in value
        .get("participants")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = str_field(participant, "id")?;
        let node = b.push(
            "context",
            "C",
            id,
            format!(
                "{} ({})",
                str_field(participant, "role")?,
                str_field(participant, "authority")?
            ),
            "supported",
            None,
            None,
            Some(participant.clone()),
        );
        b.edges.push(GsnEdge {
            kind: "in_context_of".to_owned(),
            from: root.clone(),
            to: node,
        });
    }
    for challenge in value
        .get("challenges")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = str_field(challenge, "id")?;
        let target = str_field(challenge, "target")?;
        let open = challenge.get("status").and_then(serde_json::Value::as_str) == Some("open");
        let kind = if open {
            "counter_claim"
        } else {
            "justification"
        };
        let prefix = if open { "CC" } else { "J" };
        let node = b.push(
            kind,
            prefix,
            id,
            str_field(challenge, "statement")?,
            if open { "open" } else { "supported" },
            challenge
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
            None,
            Some(challenge.clone()),
        );
        let target_id = b
            .ids
            .get(target)
            .cloned()
            .unwrap_or_else(|| target.to_owned());
        b.edges.push(GsnEdge {
            kind: if open { "challenges" } else { "in_context_of" }.to_owned(),
            from: if open {
                node.clone()
            } else {
                target_id.clone()
            },
            to: if open { target_id } else { node },
        });
    }
    let unmapped = value
        .get("relationships")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|relationship| {
            let source = relationship.get("target")?.as_str()?;
            Some(Unmapped {
                source: source.to_owned(),
                reason: "authored relationship has no declared GSN edge mapping".to_owned(),
            })
        })
        .chain(unmapped_extra)
        .collect();
    let model = GsnCase {
        format: "quoin.gsn-case/v1".to_owned(),
        nodes: b.nodes,
        edges: b.edges,
        roots: vec![root],
        unreachable: Vec::new(),
        unmapped,
    };
    validate(&model)?;
    Ok(model)
}

impl Builder {
    fn push(
        &mut self,
        kind: &str,
        prefix: &str,
        source: &str,
        statement: impl Into<String>,
        status: &str,
        reason: Option<String>,
        undeveloped: Option<bool>,
        source_data: Option<serde_json::Value>,
    ) -> String {
        if let Some(existing) = self.ids.get(source) {
            return existing.clone();
        }
        let count = self.counters.entry(prefix.to_owned()).or_default();
        *count += 1;
        let id = format!("{prefix}{count}");
        self.ids.insert(source.to_owned(), id.clone());
        self.nodes.push(GsnNode {
            id: id.clone(),
            kind: kind.to_owned(),
            source: source.to_owned(),
            statement: statement.into(),
            status: status.to_owned(),
            open_reason: reason,
            undeveloped,
            evidence: None,
            source_data,
        });
        id
    }
}

fn str_field<'a>(value: &'a serde_json::Value, name: &str) -> Result<&'a str, String> {
    value
        .get(name)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("authored view field {name} must be a string"))
}
fn status(value: &serde_json::Value) -> &'static str {
    if value.get("status").and_then(serde_json::Value::as_str) == Some("supported") {
        "supported"
    } else {
        "open"
    }
}

/// Validate the WF-1..8 graph constraints; errors name rules and affected ids.
pub fn validate(model: &GsnCase) -> Result<(), String> {
    let mut duplicate_ids = BTreeSet::new();
    let mut by_id = BTreeMap::new();
    for node in &model.nodes {
        if by_id.insert(node.id.as_str(), node).is_some() {
            duplicate_ids.insert(node.id.as_str());
        }
    }
    let mut errors = Vec::new();
    if !duplicate_ids.is_empty() {
        errors.push(format!(
            "WF-6 duplicate node ids: {}",
            duplicate_ids.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    for edge in &model.edges {
        let from = by_id.get(edge.from.as_str());
        let to = by_id.get(edge.to.as_str());
        if from.is_none() {
            errors.push(format!("WF-6 missing source {}", edge.from));
        };
        if to.is_none() {
            errors.push(format!("WF-6 missing target {}", edge.to));
            continue;
        }
        let (Some(from), Some(to)) = (from, to) else {
            continue;
        };
        match edge.kind.as_str() {
            "supported_by" => {
                if !matches!(from.kind.as_str(), "goal" | "strategy")
                    || !matches!(to.kind.as_str(), "goal" | "strategy" | "solution")
                {
                    errors.push(format!(
                        "WF-1 invalid supported_by endpoints {} -> {}",
                        from.id, to.id
                    ));
                }
            }
            "in_context_of" => {
                if !matches!(from.kind.as_str(), "goal" | "strategy")
                    || !matches!(to.kind.as_str(), "context" | "assumption" | "justification")
                {
                    errors.push(format!(
                        "WF-2 invalid in_context_of endpoints {} -> {}",
                        from.id, to.id
                    ));
                }
            }
            "challenges" => {
                if from.kind != "counter_claim"
                    || !matches!(to.kind.as_str(), "goal" | "strategy" | "assumption")
                {
                    errors.push(format!(
                        "WF-8 invalid challenges endpoints {} -> {}",
                        from.id, to.id
                    ));
                }
            }
            _ => errors.push(format!("unknown edge kind {}", edge.kind)),
        }
    }
    let outgoing = |id: &str, kind: &str| {
        model
            .edges
            .iter()
            .filter(|edge| edge.from == id && edge.kind == kind)
            .count()
    };
    for node in &model.nodes {
        if matches!(
            node.kind.as_str(),
            "solution" | "context" | "assumption" | "justification" | "counter_claim"
        ) && (outgoing(&node.id, "supported_by") > 0 || outgoing(&node.id, "in_context_of") > 0)
        {
            errors.push(format!("WF-3 terminal node {} has outgoing edge", node.id));
        }
        if matches!(node.kind.as_str(), "goal" | "strategy")
            && outgoing(&node.id, "supported_by") == 0
            && node.undeveloped != Some(true)
        {
            errors.push(format!(
                "WF-5 {} has no support and is not undeveloped",
                node.id
            ));
        }
        if node.undeveloped == Some(true) && outgoing(&node.id, "supported_by") > 0 {
            errors.push(format!("WF-7 {} is undeveloped but has support", node.id));
        }
        if node.kind == "counter_claim" && outgoing(&node.id, "challenges") != 1 {
            errors.push(format!(
                "WF-8 {} must have exactly one challenges edge",
                node.id
            ));
        }
    }
    let mut visiting = Vec::new();
    let mut visited = BTreeSet::new();
    for node in &model.nodes {
        if node.kind == "goal" || node.kind == "strategy" {
            cycle(
                node.id.as_str(),
                model,
                &mut visiting,
                &mut visited,
                &mut errors,
            );
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn cycle(
    id: &str,
    model: &GsnCase,
    visiting: &mut Vec<String>,
    visited: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    if visited.contains(id) {
        return;
    }
    if let Some(start) = visiting.iter().position(|candidate| candidate == id) {
        let mut cycle_ids = visiting[start..].to_vec();
        cycle_ids.push(id.to_owned());
        errors.push(format!(
            "WF-4 supported_by cycle: {}",
            cycle_ids.join(" -> ")
        ));
        return;
    }
    visiting.push(id.to_owned());
    for edge in model
        .edges
        .iter()
        .filter(|edge| edge.from == id && edge.kind == "supported_by")
    {
        cycle(edge.to.as_str(), model, visiting, visited, errors);
    }
    visiting.pop();
    visited.insert(id.to_owned());
}

/// Render a GSN model as a Mermaid flowchart.
pub fn render_diagram(model: &GsnCase) -> Result<String, String> {
    validate(model)?;
    let mut lines = vec![
        "flowchart TB".to_owned(),
        "  classDef open stroke:#b00".to_owned(),
        "  classDef undeveloped stroke-dasharray: 5 5".to_owned(),
    ];
    for node in &model.nodes {
        let suffix = match node.kind.as_str() {
            "assumption" => " A",
            "justification" => " J",
            _ => "",
        };
        let full_label = format!("{}: {}{}", node.source, node.statement, suffix);
        let truncated = full_label.chars().take(160).collect::<String>();
        let label = if full_label.chars().count() > 160 {
            format!("{truncated}…")
        } else {
            truncated
        }
        .replace('"', "#quot;")
        .replace('\n', "<br/>");
        let shape = match node.kind.as_str() {
            "goal" => ("[", "]"),
            "strategy" => ("[/", "/]"),
            "solution" => ("((", "))"),
            "context" => ("(", ")"),
            _ => ("([", "])"),
        };
        lines.push(format!(
            "  {}{}\"{}{}\"{}",
            node.id,
            shape.0,
            label,
            if node.undeveloped == Some(true) {
                " ◇"
            } else {
                ""
            },
            shape.1
        ));
        if node.status == "open" {
            lines.push(format!("  class {} open", node.id));
        }
        if node.undeveloped == Some(true) {
            lines.push(format!("  class {} undeveloped", node.id));
        }
    }
    for edge in &model.edges {
        lines.push(format!(
            "  {} {} {}",
            edge.from,
            match edge.kind.as_str() {
                "in_context_of" => "-.->",
                "challenges" => "--x",
                _ => "-->",
            },
            edge.to
        ));
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test failures should identify the failed assertion"
)]
mod tests {
    use super::{GsnCase, GsnEdge, GsnNode, validate};
    use ix_trace_rs::trace;

    fn node(id: &str, kind: &str, undeveloped: Option<bool>) -> GsnNode {
        GsnNode {
            id: id.to_owned(),
            kind: kind.to_owned(),
            source: id.to_owned(),
            statement: id.to_owned(),
            status: "open".to_owned(),
            open_reason: None,
            undeveloped,
            evidence: None,
            source_data: None,
        }
    }

    fn model(nodes: Vec<GsnNode>, edges: Vec<GsnEdge>) -> GsnCase {
        GsnCase {
            format: "quoin.gsn-case/v1".to_owned(),
            nodes,
            edges,
            roots: vec!["G1".to_owned()],
            unreachable: Vec::new(),
            unmapped: Vec::new(),
        }
    }

    /// Provenance: agent-ix/quoin#PLAT-1447, agent-ix/quoin#PLAT-1644.
    #[trace("FR-222-AC-1")]
    #[test]
    fn rejects_invalid_supported_by_endpoints_with_rule_and_ids() {
        let graph = model(
            vec![
                node("Sn1", "solution", None),
                node("G1", "goal", Some(true)),
            ],
            vec![GsnEdge {
                kind: "supported_by".to_owned(),
                from: "Sn1".to_owned(),
                to: "G1".to_owned(),
            }],
        );
        let error = validate(&graph).expect_err("solution cannot support a goal");
        assert!(error.contains("WF-1"));
        assert!(error.contains("Sn1"));
        assert!(error.contains("G1"));
    }

    /// Provenance: agent-ix/quoin#PLAT-1447.
    #[trace("FR-222-AC-4")]
    #[test]
    fn rejects_support_cycles_and_accepts_shared_children() {
        let cyclic = model(
            vec![node("G1", "goal", None), node("G2", "goal", None)],
            vec![
                GsnEdge {
                    kind: "supported_by".to_owned(),
                    from: "G1".to_owned(),
                    to: "G2".to_owned(),
                },
                GsnEdge {
                    kind: "supported_by".to_owned(),
                    from: "G2".to_owned(),
                    to: "G1".to_owned(),
                },
            ],
        );
        assert!(
            validate(&cyclic)
                .expect_err("cycle rejected")
                .contains("WF-4")
        );
        let shared = model(
            vec![
                node("G1", "goal", None),
                node("G2", "goal", None),
                node("Sn1", "solution", None),
            ],
            vec![
                GsnEdge {
                    kind: "supported_by".to_owned(),
                    from: "G1".to_owned(),
                    to: "Sn1".to_owned(),
                },
                GsnEdge {
                    kind: "supported_by".to_owned(),
                    from: "G2".to_owned(),
                    to: "Sn1".to_owned(),
                },
            ],
        );
        assert!(validate(&shared).is_ok());
    }

    /// Provenance: agent-ix/quoin#PLAT-1447.
    #[trace("FR-221-AC-3")]
    #[test]
    fn open_derived_obligation_becomes_undeveloped_goal_with_reason() {
        let case: crate::AssuranceCase = serde_json::from_value(serde_json::json!({
            "claims": [{"id":"StR-1", "kind":"goal", "statement":"Claim", "status":"open", "context":[{"id":"HZ-1","kind":"context","statement":"Hazard","status":"supported","children":[]}], "children":[
                {"id":"StR-1/strategy", "kind":"strategy", "statement":"Argument", "status":"open", "children":[
                    {"id":"FR-1-AC-1", "kind":"solution", "statement":"Requirement", "status":"open", "because":"audit reason", "children":[]}
                ]}
            ]}], "unreachable":[], "unreadable":[], "producerTrust":[]
        })).expect("case fixture deserializes");
        let model = super::from_case(&case).expect("valid case converts");
        let goal = model
            .nodes
            .iter()
            .find(|node| node.source == "FR-1-AC-1")
            .expect("obligation is present");
        assert_eq!(goal.kind, "goal");
        assert_eq!(goal.status, "open");
        assert_eq!(goal.open_reason.as_deref(), Some("audit reason"));
        assert_eq!(goal.undeveloped, Some(true));
        let context = model
            .nodes
            .iter()
            .find(|node| node.source == "HZ-1")
            .expect("case context maps to a GSN context");
        assert_eq!(context.kind, "context");
        assert!(model.edges.iter().any(|edge| edge.kind == "in_context_of"
            && edge.from == model.roots[0]
            && edge.to == context.id));
    }

    /// Provenance: agent-ix/quoin#PLAT-1447.
    #[trace("FR-221-AC-4", "FR-221-AC-10")]
    #[test]
    fn global_trust_and_independence_context_attach_to_every_root_once() {
        let case: crate::AssuranceCase = serde_json::from_value(serde_json::json!({
            "claims":[
                {"id":"StR-1","kind":"goal","statement":"one","status":"open","children":[]},
                {"id":"StR-2","kind":"goal","statement":"two","status":"open","children":[]}
            ],
            "unreachable":[],"unreadable":[],
            "producerTrust":[{"id":"PT-1","status":"recorded"}],
            "evidenceIndependence":[{"id":"EI-1","status":"recorded"}]
        }))
        .expect("case fixture deserializes");
        let converted = super::from_case(&case).expect("global context converts");
        for source in ["PT-1", "EI-1"] {
            let context = converted
                .nodes
                .iter()
                .find(|node| node.source == source)
                .expect("context");
            assert_eq!(
                converted
                    .edges
                    .iter()
                    .filter(|edge| edge.kind == "in_context_of" && edge.to == context.id)
                    .count(),
                2
            );
        }
    }

    /// Provenance: agent-ix/quoin#PLAT-1447.
    #[trace("FR-221-AC-11", "FR-224-AC-1", "FR-224-AC-2", "FR-224-AC-4")]
    #[test]
    fn tool_confidence_uses_exact_run_identity_and_keeps_missing_sources_open() {
        let case: crate::AssuranceCase = serde_json::from_value(serde_json::json!({
            "claims":[{"id":"StR-1","kind":"goal","statement":"Claim","status":"supported","children":[
                {"id":"FR-1-AC-1","kind":"solution","statement":"Evidence","status":"supported","evidence":{"suites":[{"suite":"SUITE-1","at":"2026-01-01T00:00:00Z"}],"newest":"2026-01-01T00:00:00Z"},"children":[]}
            ]}],"unreachable":[],"unreadable":[],"producerTrust":[]
        })).expect("case fixture deserializes");
        let runs = vec![serde_json::json!({
            "suite":"SUITE-1","timestamp":"2026-01-01T00:00:00Z","tool":"pkg 1.2.3"
        })];
        let report = serde_json::json!({"tools":[{
            "tool":"pkg 1.2.3",
            "packageIdentity":"pkg",
            "intendedUse":{"id":"IU-1","purpose":"test","limits":["bounded"]},
            "validationReportId":"VR-1",
            "selfTestRuns":[{"id":"RUN-1","outcome":"pass","tool":"pkg 1.2.3"}],
            "anomalies":[]
        }]});
        let converted = super::from_case_with_validation_report(&case, &runs, Some(&report))
            .expect("source facts map");
        assert!(
            converted
                .nodes
                .iter()
                .any(|node| node.source == "tool-confidence:FR-1-AC-1:pkg")
        );
        for source in ["IU-1", "VR-1", "RUN-1", "anomaly-inventory"] {
            assert!(converted.nodes.iter().any(|node| node.source == source));
        }
        assert!(
            converted
                .nodes
                .iter()
                .any(|node| node.statement == "none known")
        );
        assert!(
            converted
                .nodes
                .iter()
                .all(|node| !node.source.contains("score")
                    && !node.statement.contains("confidence score"))
        );

        let incomplete = super::from_case_with_validation_report(&case, &runs, None)
            .expect("missing report stays explicit");
        let gap = incomplete
            .nodes
            .iter()
            .find(|node| node.source == "tool-confidence-gap:FR-1-AC-1")
            .expect("open gap");
        assert_eq!(gap.status, "open");
        assert!(
            gap.open_reason
                .as_deref()
                .unwrap_or_default()
                .contains("validation-report tool entry not provided")
        );
    }

    /// Provenance: agent-ix/quoin#PLAT-1447.
    #[trace("FR-222-AC-2")]
    #[test]
    fn rejects_context_endpoints_terminal_nodes_development_and_bad_references() {
        let in_context_bad = model(
            vec![
                node("A1", "assumption", None),
                node("G1", "goal", Some(true)),
            ],
            vec![GsnEdge {
                kind: "in_context_of".to_owned(),
                from: "A1".to_owned(),
                to: "G1".to_owned(),
            }],
        );
        assert!(
            validate(&in_context_bad)
                .expect_err("WF-2 endpoints")
                .contains("WF-2")
        );

        let terminal_bad = model(
            vec![node("C1", "context", None), node("G1", "goal", Some(true))],
            vec![GsnEdge {
                kind: "supported_by".to_owned(),
                from: "C1".to_owned(),
                to: "G1".to_owned(),
            }],
        );
        let terminal_error =
            validate(&terminal_bad).expect_err("terminal nodes have no outgoing support");
        assert!(terminal_error.contains("WF-1"));
        assert!(terminal_error.contains("WF-3"));

        let undeveloped_missing = model(vec![node("G1", "goal", None)], Vec::new());
        assert!(
            validate(&undeveloped_missing)
                .expect_err("goal must be developed")
                .contains("WF-5")
        );

        let duplicate = model(
            vec![
                node("G1", "goal", Some(true)),
                node("G1", "goal", Some(true)),
            ],
            Vec::new(),
        );
        assert!(
            validate(&duplicate)
                .expect_err("ids are unique")
                .contains("WF-6")
        );

        let dangling = model(
            vec![node("G1", "goal", None)],
            vec![GsnEdge {
                kind: "supported_by".to_owned(),
                from: "G1".to_owned(),
                to: "missing".to_owned(),
            }],
        );
        assert!(
            validate(&dangling)
                .expect_err("endpoints exist")
                .contains("WF-6")
        );

        let undeveloped_with_support = model(
            vec![
                node("G1", "goal", Some(true)),
                node("G2", "goal", Some(true)),
            ],
            vec![GsnEdge {
                kind: "supported_by".to_owned(),
                from: "G1".to_owned(),
                to: "G2".to_owned(),
            }],
        );
        assert!(
            validate(&undeveloped_with_support)
                .expect_err("undeveloped goal has no children")
                .contains("WF-7")
        );

        let counterclaim_without_edge = model(
            vec![
                node("CC1", "counter_claim", None),
                node("G1", "goal", Some(true)),
            ],
            Vec::new(),
        );
        assert!(
            validate(&counterclaim_without_edge)
                .expect_err("counterclaim targets one argument node")
                .contains("WF-8")
        );
        let counterclaim_multiple_edges = model(
            vec![
                node("CC1", "counter_claim", None),
                node("G1", "goal", Some(true)),
                node("S1", "strategy", Some(true)),
            ],
            vec![
                GsnEdge {
                    kind: "challenges".to_owned(),
                    from: "CC1".to_owned(),
                    to: "G1".to_owned(),
                },
                GsnEdge {
                    kind: "challenges".to_owned(),
                    from: "CC1".to_owned(),
                    to: "S1".to_owned(),
                },
            ],
        );
        assert!(
            validate(&counterclaim_multiple_edges)
                .expect_err("counterclaim has one target")
                .contains("WF-8")
        );
        let counterclaim_bad_target = model(
            vec![
                node("CC1", "counter_claim", None),
                node("Sn1", "solution", None),
            ],
            vec![GsnEdge {
                kind: "challenges".to_owned(),
                from: "CC1".to_owned(),
                to: "Sn1".to_owned(),
            }],
        );
        assert!(
            validate(&counterclaim_bad_target)
                .expect_err("counterclaim targets argument nodes only")
                .contains("WF-8")
        );

        let invalid_case: crate::AssuranceCase = serde_json::from_value(serde_json::json!({
            "claims":[{"id":"G1","kind":"goal","statement":"Claim","status":"open","context":[{"id":"C1","kind":"context","statement":"Context","status":"open","children":[{"id":"G2","kind":"goal","statement":"Nested","status":"open","children":[]}]}],"children":[]}],
            "unreachable":[],"unreadable":[],"producerTrust":[]
        })).expect("case with invalid terminal node deserializes");
        assert!(
            super::from_case(&invalid_case)
                .expect_err("terminal context cannot have children")
                .contains("terminal GSN node C1")
        );
    }

    /// Provenance: agent-ix/quoin#PLAT-1447.
    #[trace("FR-221-AC-6", "FR-223-AC-3", "FR-223-AC-4")]
    #[test]
    fn open_authored_challenge_is_only_a_counter_claim() {
        let view = serde_json::json!({
            "topClaim":{"id":"TC-1","statement":"Claim","status":"open","reasons":["challenge open"]},
            "reasoning":[],
            "assumptions":[
                {"id":"AS-1","statement":"First assumption","owner":"ops","status":"supported","declaredStatus":"accepted","reviewBy":"2030-01-01T00:00:00Z"},
                {"id":"AS-2","statement":"Second assumption","owner":"safety","status":"open","declaredStatus":"proposed","reviewBy":"2030-01-01T00:00:00Z","reason":"not accepted"}
            ],
            "participants":[{"id":"P-1","role":"reviewer","authority":"approve","independence":"implementation"}],
            "challenges":[
                {"id":"CH-1","target":"TC-1","statement":"Objection","owner":"auditor","status":"open","declaredStatus":"open","resolutionRefs":[],"reason":"unresolved"},
                {"id":"CH-2","target":"TC-1","statement":"Resolved objection","owner":"owner","status":"resolved","declaredStatus":"accepted","resolutionRefs":["EV-1"],"expiresAt":"2030-01-01T00:00:00Z"}
            ],
            "relationships":[]
        });
        let model =
            super::from_authored(&view).expect("open challenge maps to a valid counter-claim");
        let challenge = model
            .nodes
            .iter()
            .find(|node| node.source == "CH-1")
            .expect("counter-claim is present");
        assert_eq!(challenge.kind, "counter_claim");
        assert_eq!(challenge.open_reason.as_deref(), Some("unresolved"));
        assert_eq!(
            model
                .edges
                .iter()
                .filter(|edge| edge.kind == "challenges"
                    && edge.from == challenge.id
                    && edge.to == model.roots[0])
                .count(),
            1
        );
        assert!(
            !model
                .nodes
                .iter()
                .any(|node| node.source == "CH-1" && node.kind == "context")
        );
        assert_eq!(
            model
                .nodes
                .iter()
                .filter(|node| node.kind == "assumption")
                .count(),
            2
        );
        let resolved = model
            .nodes
            .iter()
            .find(|node| node.source == "CH-2")
            .expect("resolved challenge is retained");
        assert_eq!(resolved.kind, "justification");
        assert_eq!(
            resolved
                .source_data
                .as_ref()
                .and_then(|data| data.get("owner"))
                .and_then(serde_json::Value::as_str),
            Some("owner")
        );
        assert!(
            model
                .nodes
                .iter()
                .any(|node| node.source == "P-1" && node.kind == "context")
        );
        assert!(
            model
                .edges
                .iter()
                .any(|edge| edge.kind == "in_context_of" && edge.to == resolved.id)
        );
    }

    /// Provenance: agent-ix/quoin#PLAT-1447.
    #[trace("FR-221-AC-2", "FR-221-AC-5", "FR-221-AC-10")]
    #[test]
    fn authored_supports_and_top_claim_evidence_keep_authored_links() {
        let view = serde_json::json!({
            "topClaim": {
                "id": "TC-1", "statement": "Claim", "status": "open",
                "reasons": ["evidence://missing does not resolve"],
                "evidenceRefs": [
                    {"reference":"evidence://ok", "state":"resolved"},
                    {"reference":"evidence://missing", "reason":"not found"}
                ]
            },
            "reasoning": [
                {"id":"R-1", "supports":"TC-1", "statement":"root argument", "status":"open", "criteria":[]},
                {"id":"R-2", "supports":"R-1", "statement":"nested argument", "status":"supported", "criteria":[
                    {"criterion":"evidence criterion", "status":"supported", "decision":{"evidenceRefs":["evidence://ok"]}}
                ]}
            ],
            "assumptions":[], "participants":[], "challenges":[], "relationships":[]
        });
        let result = super::from_authored(&view).expect("authored support graph is valid");
        let root = &result.roots[0];
        let strategy_one = result
            .nodes
            .iter()
            .find(|node| node.source == "R-1")
            .expect("R-1");
        let strategy_two = result
            .nodes
            .iter()
            .find(|node| node.source == "R-2")
            .expect("R-2");
        assert!(result.edges.iter().any(|edge| edge.kind == "supported_by"
            && edge.from == *root
            && edge.to == strategy_one.id));
        assert!(result.edges.iter().any(|edge| edge.kind == "supported_by"
            && edge.from == strategy_one.id
            && edge.to == strategy_two.id));
        let evidence = result
            .nodes
            .iter()
            .filter(|node| node.source == "evidence://ok")
            .collect::<Vec<_>>();
        assert_eq!(evidence.len(), 1, "shared source evidence is one GSN node");
        assert!(
            result
                .edges
                .iter()
                .filter(|edge| edge.to == evidence[0].id)
                .count()
                >= 2
        );
        let missing = result
            .nodes
            .iter()
            .find(|node| node.source == "evidence://missing")
            .expect("missing solution reference");
        assert_eq!(missing.status, "open");
        assert_eq!(missing.open_reason.as_deref(), Some("not found"));
        assert_eq!(
            result
                .nodes
                .iter()
                .find(|node| node.id == *root)
                .expect("root")
                .status,
            "open"
        );
    }

    /// Provenance: agent-ix/quoin#PLAT-1645.
    #[trace("FR-221-AC-10")]
    #[test]
    fn diagram_escapes_quotes_newlines_and_truncates_code_points() {
        let text = format!("{}é", "x".repeat(170));
        let mut goal = node("G1", "goal", Some(true));
        goal.source = "FR-1".to_owned();
        goal.statement = format!("{text}\n\"quoted\"");
        let diagram = super::render_diagram(&model(vec![goal], Vec::new())).expect("valid diagram");
        assert!(diagram.starts_with("flowchart TB\n"));
        assert!(diagram.contains("…"));
        assert!(diagram.contains("class G1 open"));
        assert!(diagram.contains("classDef open stroke:#b00"));
        assert!(diagram.contains("classDef undeveloped stroke-dasharray: 5 5"));
        assert!(diagram.contains("class G1 undeveloped"));
        let mut escaped_goal = node("G1", "goal", Some(true));
        escaped_goal.statement = "first\n\"quoted\"".to_owned();
        let escaped =
            super::render_diagram(&model(vec![escaped_goal], Vec::new())).expect("valid diagram");
        assert!(escaped.contains("#quot;"));
        assert!(escaped.contains("<br/>"));
    }

    /// Provenance: agent-ix/quoin#PLAT-1645.
    #[trace("FR-221-AC-6", "FR-221-AC-7")]
    #[test]
    fn diagram_renders_challenges_as_objections_and_labels_a_j_nodes() {
        let mut assumption = node("A1", "assumption", None);
        assumption.source = "ASM-1".to_owned();
        assumption.statement = "assumed".to_owned();
        let mut counterclaim = node("CC1", "counter_claim", None);
        counterclaim.source = "CH-1".to_owned();
        let graph = model(
            vec![node("G1", "goal", Some(true)), assumption, counterclaim],
            vec![
                GsnEdge {
                    kind: "in_context_of".to_owned(),
                    from: "G1".to_owned(),
                    to: "A1".to_owned(),
                },
                GsnEdge {
                    kind: "challenges".to_owned(),
                    from: "CC1".to_owned(),
                    to: "G1".to_owned(),
                },
            ],
        );
        let diagram = super::render_diagram(&graph).expect("valid diagram");
        assert!(diagram.contains("ASM-1: assumed A"));
        assert!(diagram.contains("CC1 --x G1"));
        assert!(!diagram.contains("CC1 --> G1"));
    }
}
